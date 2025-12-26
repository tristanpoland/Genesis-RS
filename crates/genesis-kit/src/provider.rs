//! Kit provider implementations for downloading and installing kits.

use super::{Kit, CompiledKit};
use genesis_types::{GenesisError, Result, KitId, SemVer};
use genesis_services::github::GithubClient;
use std::path::{Path, PathBuf};
use async_trait::async_trait;
use tracing::{info, debug, warn};

/// Trait for kit providers that can download and install kits.
#[async_trait]
pub trait KitProvider: Send + Sync {
    /// Get the name of this provider.
    fn name(&self) -> &str;

    /// Check if this provider can provide the specified kit.
    async fn can_provide(&self, kit_name: &str) -> Result<bool>;

    /// List available versions for a kit.
    async fn list_versions(&self, kit_name: &str) -> Result<Vec<SemVer>>;

    /// Get the latest version for a kit.
    async fn latest_version(&self, kit_name: &str) -> Result<SemVer> {
        let versions = self.list_versions(kit_name).await?;
        versions.into_iter()
            .max()
            .ok_or_else(|| GenesisError::Kit(format!(
                "No versions available for kit: {}",
                kit_name
            )))
    }

    /// Download and install a specific kit version.
    async fn install_kit(
        &self,
        kit_name: &str,
        version: &SemVer,
        install_dir: impl AsRef<Path> + Send,
    ) -> Result<Box<dyn Kit>>;

    /// Download and install the latest kit version.
    async fn install_latest(
        &self,
        kit_name: &str,
        install_dir: impl AsRef<Path> + Send,
    ) -> Result<Box<dyn Kit>> {
        let version = self.latest_version(kit_name).await?;
        self.install_kit(kit_name, &version, install_dir).await
    }
}

/// GitHub-based kit provider.
pub struct GithubProvider {
    client: GithubClient,
    owner: String,
}

impl GithubProvider {
    /// Create a new GitHub provider for a specific owner/organization.
    pub fn new(owner: impl Into<String>, token: Option<String>) -> Self {
        Self {
            client: GithubClient::new(token),
            owner: owner.into(),
        }
    }

    /// Create a provider for the Genesis Community organization.
    pub fn genesis_community(token: Option<String>) -> Self {
        Self::new("genesis-community", token)
    }

    /// Get repository name for a kit.
    fn repo_name(&self, kit_name: &str) -> String {
        format!("{}-genesis-kit", kit_name)
    }

    /// Parse version from release tag.
    fn parse_version_tag(&self, tag: &str) -> Result<SemVer> {
        let version_str = tag.strip_prefix('v').unwrap_or(tag);
        SemVer::parse(version_str)
    }

    /// Get the tarball asset from a release.
    async fn get_tarball_asset(&self, kit_name: &str, version: &SemVer) -> Result<(String, String)> {
        let repo = self.repo_name(kit_name);
        let tag = format!("v{}", version);

        let release = self.client
            .get_release_by_tag(&self.owner, &repo, &tag)
            .await?;

        let tarball_name = format!("{}-{}.tar.gz", kit_name, version);

        for asset in &release.assets {
            if asset.name == tarball_name || asset.name.ends_with(".tar.gz") {
                return Ok((asset.name.clone(), asset.browser_download_url.clone()));
            }
        }

        Err(GenesisError::Kit(format!(
            "No tarball asset found for {}/{} version {}",
            self.owner, repo, version
        )))
    }
}

#[async_trait]
impl KitProvider for GithubProvider {
    fn name(&self) -> &str {
        &self.owner
    }

    async fn can_provide(&self, kit_name: &str) -> Result<bool> {
        let repo = self.repo_name(kit_name);
        match self.client.get_repository(&self.owner, &repo).await {
            Ok(_) => Ok(true),
            Err(GenesisError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn list_versions(&self, kit_name: &str) -> Result<Vec<SemVer>> {
        let repo = self.repo_name(kit_name);

        info!("Fetching releases for {}/{}", self.owner, repo);
        let releases = self.client.list_releases(&self.owner, &repo).await?;

        let mut versions = Vec::new();
        for release in releases {
            if release.draft || release.prerelease {
                debug!("Skipping draft/prerelease: {}", release.tag_name);
                continue;
            }

            match self.parse_version_tag(&release.tag_name) {
                Ok(version) => versions.push(version),
                Err(e) => {
                    warn!("Failed to parse version tag '{}': {}", release.tag_name, e);
                }
            }
        }

        versions.sort();
        versions.reverse();

        Ok(versions)
    }

    async fn install_kit(
        &self,
        kit_name: &str,
        version: &SemVer,
        install_dir: impl AsRef<Path> + Send,
    ) -> Result<Box<dyn Kit>> {
        let install_dir = install_dir.as_ref();

        info!("Installing kit {}/{} version {}", self.owner, kit_name, version);

        let (asset_name, download_url) = self.get_tarball_asset(kit_name, version).await?;

        let tarball_path = install_dir.join(&asset_name);

        if !tarball_path.exists() {
            std::fs::create_dir_all(install_dir)
                .map_err(|e| GenesisError::Kit(format!(
                    "Failed to create install directory: {}",
                    e
                )))?;

            info!("Downloading {} to {:?}", asset_name, tarball_path);
            self.client.download_asset(&download_url, &tarball_path).await?;
        } else {
            debug!("Tarball already exists at {:?}", tarball_path);
        }

        let extract_dir = install_dir.join(".extracted");
        let kit = CompiledKit::from_tarball(&tarball_path, &extract_dir)?;

        Ok(Box::new(kit))
    }
}

/// Genesis Community kit provider (default provider).
pub struct GenesisCommunityProvider {
    inner: GithubProvider,
}

impl GenesisCommunityProvider {
    /// Create a new Genesis Community provider.
    pub fn new(token: Option<String>) -> Self {
        Self {
            inner: GithubProvider::genesis_community(token),
        }
    }
}

#[async_trait]
impl KitProvider for GenesisCommunityProvider {
    fn name(&self) -> &str {
        "genesis-community"
    }

    async fn can_provide(&self, kit_name: &str) -> Result<bool> {
        self.inner.can_provide(kit_name).await
    }

    async fn list_versions(&self, kit_name: &str) -> Result<Vec<SemVer>> {
        self.inner.list_versions(kit_name).await
    }

    async fn install_kit(
        &self,
        kit_name: &str,
        version: &SemVer,
        install_dir: impl AsRef<Path> + Send,
    ) -> Result<Box<dyn Kit>> {
        self.inner.install_kit(kit_name, version, install_dir).await
    }
}

/// Custom kit provider that uses a specific GitHub repository URL.
pub struct CustomProvider {
    inner: GithubProvider,
    repo_name: String,
}

impl CustomProvider {
    /// Create a custom provider from a GitHub URL.
    ///
    /// Supported formats:
    /// - https://github.com/owner/repo
    /// - github.com/owner/repo
    /// - owner/repo
    pub fn from_url(url: impl AsRef<str>, token: Option<String>) -> Result<Self> {
        let url = url.as_ref();

        let parts: Vec<&str> = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("github.com/")
            .split('/')
            .collect();

        if parts.len() != 2 {
            return Err(GenesisError::Kit(format!(
                "Invalid GitHub URL format: {}. Expected 'owner/repo'",
                url
            )));
        }

        let owner = parts[0].to_string();
        let repo = parts[1]
            .trim_end_matches(".git")
            .trim_end_matches("-genesis-kit")
            .to_string();

        Ok(Self {
            inner: GithubProvider::new(owner, token),
            repo_name: repo,
        })
    }

    /// Get the full repository name.
    fn full_repo_name(&self) -> String {
        format!("{}-genesis-kit", self.repo_name)
    }
}

#[async_trait]
impl KitProvider for CustomProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn can_provide(&self, kit_name: &str) -> Result<bool> {
        if kit_name != self.repo_name {
            return Ok(false);
        }

        let full_repo = self.full_repo_name();
        match self.inner.client.get_repository(self.inner.name(), &full_repo).await {
            Ok(_) => Ok(true),
            Err(GenesisError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn list_versions(&self, kit_name: &str) -> Result<Vec<SemVer>> {
        if kit_name != self.repo_name {
            return Err(GenesisError::Kit(format!(
                "Custom provider for '{}' cannot provide kit '{}'",
                self.repo_name, kit_name
            )));
        }

        let full_repo = self.full_repo_name();
        let releases = self.inner.client.list_releases(self.inner.name(), &full_repo).await?;

        let mut versions = Vec::new();
        for release in releases {
            if release.draft || release.prerelease {
                continue;
            }

            if let Ok(version) = self.inner.parse_version_tag(&release.tag_name) {
                versions.push(version);
            }
        }

        versions.sort();
        versions.reverse();

        Ok(versions)
    }

    async fn install_kit(
        &self,
        kit_name: &str,
        version: &SemVer,
        install_dir: impl AsRef<Path> + Send,
    ) -> Result<Box<dyn Kit>> {
        if kit_name != self.repo_name {
            return Err(GenesisError::Kit(format!(
                "Custom provider for '{}' cannot provide kit '{}'",
                self.repo_name, kit_name
            )));
        }

        self.inner.install_kit(&self.repo_name, version, install_dir).await
    }
}

/// Provider factory for creating kit providers.
pub struct ProviderFactory {
    default_token: Option<String>,
}

impl ProviderFactory {
    /// Create a new provider factory.
    pub fn new(default_token: Option<String>) -> Self {
        Self { default_token }
    }

    /// Create the default Genesis Community provider.
    pub fn default_provider(&self) -> Box<dyn KitProvider> {
        Box::new(GenesisCommunityProvider::new(self.default_token.clone()))
    }

    /// Create a provider from a URL or organization name.
    ///
    /// If the input contains a '/', it's treated as a GitHub URL.
    /// Otherwise, it's treated as an organization name.
    pub fn from_source(&self, source: impl AsRef<str>) -> Result<Box<dyn KitProvider>> {
        let source = source.as_ref();

        if source.contains('/') {
            Ok(Box::new(CustomProvider::from_url(source, self.default_token.clone())?))
        } else {
            Ok(Box::new(GithubProvider::new(source, self.default_token.clone())))
        }
    }

    /// Create a provider chain that tries multiple providers in order.
    pub fn chain(&self, sources: Vec<String>) -> ProviderChain {
        let mut providers: Vec<Box<dyn KitProvider>> = Vec::new();

        for source in sources {
            match self.from_source(&source) {
                Ok(provider) => providers.push(provider),
                Err(e) => {
                    warn!("Failed to create provider from '{}': {}", source, e);
                }
            }
        }

        if providers.is_empty() {
            providers.push(self.default_provider());
        }

        ProviderChain { providers }
    }
}

impl Default for ProviderFactory {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Chain of kit providers that tries each in order.
pub struct ProviderChain {
    providers: Vec<Box<dyn KitProvider>>,
}

impl ProviderChain {
    /// Create a new provider chain.
    pub fn new(providers: Vec<Box<dyn KitProvider>>) -> Self {
        Self { providers }
    }

    /// Find the first provider that can provide the kit.
    pub async fn find_provider(&self, kit_name: &str) -> Result<&dyn KitProvider> {
        for provider in &self.providers {
            match provider.can_provide(kit_name).await {
                Ok(true) => {
                    info!("Provider '{}' can provide kit '{}'", provider.name(), kit_name);
                    return Ok(provider.as_ref());
                }
                Ok(false) => {
                    debug!("Provider '{}' cannot provide kit '{}'", provider.name(), kit_name);
                }
                Err(e) => {
                    warn!("Error checking provider '{}': {}", provider.name(), e);
                }
            }
        }

        Err(GenesisError::Kit(format!(
            "No provider found for kit: {}",
            kit_name
        )))
    }

    /// List all available versions across all providers.
    pub async fn list_versions(&self, kit_name: &str) -> Result<Vec<SemVer>> {
        let mut all_versions = Vec::new();

        for provider in &self.providers {
            match provider.list_versions(kit_name).await {
                Ok(versions) => {
                    all_versions.extend(versions);
                }
                Err(e) => {
                    debug!("Provider '{}' failed to list versions: {}", provider.name(), e);
                }
            }
        }

        if all_versions.is_empty() {
            return Err(GenesisError::Kit(format!(
                "No versions found for kit: {}",
                kit_name
            )));
        }

        all_versions.sort();
        all_versions.reverse();
        all_versions.dedup();

        Ok(all_versions)
    }

    /// Install a kit using the first available provider.
    pub async fn install_kit(
        &self,
        kit_name: &str,
        version: &SemVer,
        install_dir: impl AsRef<Path> + Send,
    ) -> Result<Box<dyn Kit>> {
        let provider = self.find_provider(kit_name).await?;
        provider.install_kit(kit_name, version, install_dir).await
    }

    /// Install the latest kit version using the first available provider.
    pub async fn install_latest(
        &self,
        kit_name: &str,
        install_dir: impl AsRef<Path> + Send,
    ) -> Result<Box<dyn Kit>> {
        let provider = self.find_provider(kit_name).await?;
        provider.install_latest(kit_name, install_dir).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_provider_repo_name() {
        let provider = GithubProvider::new("genesis-community", None);
        assert_eq!(provider.repo_name("bosh"), "bosh-genesis-kit");
        assert_eq!(provider.repo_name("cf"), "cf-genesis-kit");
    }

    #[test]
    fn test_version_tag_parsing() {
        let provider = GithubProvider::new("genesis-community", None);

        assert_eq!(
            provider.parse_version_tag("v1.2.3").unwrap(),
            SemVer::parse("1.2.3").unwrap()
        );

        assert_eq!(
            provider.parse_version_tag("1.2.3").unwrap(),
            SemVer::parse("1.2.3").unwrap()
        );
    }

    #[test]
    fn test_custom_provider_from_url() {
        let provider = CustomProvider::from_url("https://github.com/owner/repo", None).unwrap();
        assert_eq!(provider.repo_name, "repo");
        assert_eq!(provider.inner.owner, "owner");

        let provider = CustomProvider::from_url("github.com/owner/repo", None).unwrap();
        assert_eq!(provider.repo_name, "repo");
        assert_eq!(provider.inner.owner, "owner");

        let provider = CustomProvider::from_url("owner/repo", None).unwrap();
        assert_eq!(provider.repo_name, "repo");
        assert_eq!(provider.inner.owner, "owner");

        let provider = CustomProvider::from_url("owner/repo-genesis-kit", None).unwrap();
        assert_eq!(provider.repo_name, "repo");
        assert_eq!(provider.inner.owner, "owner");
    }

    #[test]
    fn test_custom_provider_invalid_url() {
        assert!(CustomProvider::from_url("invalid", None).is_err());
        assert!(CustomProvider::from_url("a/b/c", None).is_err());
    }

    #[test]
    fn test_provider_factory() {
        let factory = ProviderFactory::new(None);

        let provider = factory.from_source("genesis-community").unwrap();
        assert_eq!(provider.name(), "genesis-community");

        let provider = factory.from_source("owner/repo").unwrap();
        assert_eq!(provider.name(), "owner");
    }
}
