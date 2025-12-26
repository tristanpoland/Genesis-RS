//! GitHub API client implementation.

use genesis_types::{GenesisError, Result, SemVer};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

/// GitHub client configuration.
#[derive(Debug, Clone)]
pub struct GithubConfig {
    /// GitHub API base URL (for Enterprise)
    pub api_url: String,
    /// Personal access token (optional, for rate limiting)
    pub token: Option<String>,
    /// Organization or user
    pub org: String,
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.github.com".to_string(),
            token: None,
            org: "genesis-community".to_string(),
        }
    }
}

/// GitHub API client for downloading kits.
pub struct GithubClient {
    config: GithubConfig,
    client: Client,
}

impl GithubClient {
    /// Create a new GitHub client.
    pub fn new(config: GithubConfig) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("genesis-rs/3.0"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github.v3+json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| GenesisError::Other(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// List all releases for a repository.
    pub async fn list_releases(&self, repo: &str) -> Result<Vec<Release>> {
        let url = format!(
            "{}/repos/{}/{}/releases",
            self.config.api_url, self.config.org, repo
        );

        let mut req = self.client.get(&url);
        if let Some(token) = &self.config.token {
            req = req.header(header::AUTHORIZATION, format!("token {}", token));
        }

        let releases: Vec<Release> = req.send().await
            .map_err(|e| GenesisError::Other(format!("Failed to list releases: {}", e)))?
            .json().await
            .map_err(|e| GenesisError::Other(format!("Failed to parse releases: {}", e)))?;

        Ok(releases)
    }

    /// Get a specific release by tag.
    pub async fn get_release(&self, repo: &str, tag: &str) -> Result<Release> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{}",
            self.config.api_url, self.config.org, repo, tag
        );

        let mut req = self.client.get(&url);
        if let Some(token) = &self.config.token {
            req = req.header(header::AUTHORIZATION, format!("token {}", token));
        }

        let release: Release = req.send().await
            .map_err(|e| GenesisError::Other(format!("Failed to get release: {}", e)))?
            .json().await
            .map_err(|e| GenesisError::Other(format!("Failed to parse release: {}", e)))?;

        Ok(release)
    }

    /// Check if a repository exists.
    pub async fn get_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!(
            "{}/repos/{}/{}",
            self.config.api_url, owner, repo
        );

        let mut req = self.client.get(&url);
        if let Some(token) = &self.config.token {
            req = req.header(header::AUTHORIZATION, format!("token {}", token));
        }

        let response = req.send().await
            .map_err(|e| GenesisError::Other(format!("Failed to get repository: {}", e)))?;

        if response.status() == 404 {
            return Err(GenesisError::NotFound(format!("Repository not found: {}/{}", owner, repo)));
        }

        let repository: Repository = response.json().await
            .map_err(|e| GenesisError::Other(format!("Failed to parse repository: {}", e)))?;

        Ok(repository)
    }

    /// Download a release asset.
    pub async fn download_asset(&self, asset_url: &str, dest: &PathBuf) -> Result<()> {
        let mut req = self.client.get(asset_url);
        if let Some(token) = &self.config.token {
            req = req.header(header::AUTHORIZATION, format!("token {}", token));
        }

        let bytes = req.send().await
            .map_err(|e| GenesisError::Other(format!("Failed to download asset: {}", e)))?
            .bytes().await
            .map_err(|e| GenesisError::Other(format!("Failed to read asset bytes: {}", e)))?;

        std::fs::write(dest, bytes)
            .map_err(|e| GenesisError::Io(e))?;

        Ok(())
    }
}

/// GitHub repository information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Repository name
    pub name: String,
    /// Repository full name (owner/repo)
    pub full_name: String,
    /// Repository description
    pub description: Option<String>,
    /// Default branch
    pub default_branch: String,
}

/// GitHub release information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    /// Release tag name
    pub tag_name: String,
    /// Release name
    pub name: String,
    /// Release body/description
    pub body: Option<String>,
    /// Whether this is a draft
    pub draft: bool,
    /// Whether this is a pre-release
    pub prerelease: bool,
    /// Release assets
    pub assets: Vec<Asset>,
}

/// GitHub release asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Asset name
    pub name: String,
    /// Download URL
    pub browser_download_url: String,
    /// Asset size in bytes
    pub size: u64,
    /// Content type
    pub content_type: String,
}
