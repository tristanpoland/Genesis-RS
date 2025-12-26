//! Vault service client implementation.

use async_trait::async_trait;
use genesis_types::{GenesisError, Result};
use genesis_types::traits::VaultStore;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// Vault client configuration.
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Vault URL
    pub url: String,
    /// Vault token
    pub token: Option<String>,
    /// Skip TLS verification
    pub insecure: bool,
    /// Vault namespace (Enterprise)
    pub namespace: Option<String>,
    /// Use strongbox mode
    pub strongbox: bool,
    /// Mount point for secrets
    pub mount: String,
    /// Vault alias/name
    pub name: String,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            url: "https://127.0.0.1:8200".to_string(),
            token: None,
            insecure: false,
            namespace: None,
            strongbox: true,
            mount: "/secret/".to_string(),
            name: "default".to_string(),
        }
    }
}

/// Vault client for interacting with HashiCorp Vault.
#[derive(Clone)]
pub struct VaultClient {
    config: VaultConfig,
    client: Client,
    base_url: Url,
}

impl VaultClient {
    /// Create a new Vault client.
    pub fn new(config: VaultConfig) -> Result<Self> {
        let base_url = Url::parse(&config.url)
            .map_err(|e| GenesisError::Vault(format!("Invalid vault URL: {}", e)))?;

        let mut builder = Client::builder();

        if config.insecure {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder.build()
            .map_err(|e| GenesisError::Vault(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            base_url,
        })
    }

    /// Get the Vault token from environment or config.
    fn get_token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("VAULT_TOKEN") {
            return Ok(token);
        }

        self.config.token.clone()
            .ok_or_else(|| GenesisError::Vault("No vault token available".to_string()))
    }

    /// Build the full path for a secret.
    fn build_path(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}{}", self.config.mount.trim_end_matches('/'),
                if path.is_empty() { String::new() } else { format!("/{}", path) })
    }

    /// Make a request to Vault.
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let token = self.get_token()?;
        let url = self.base_url.join(path)
            .map_err(|e| GenesisError::Vault(format!("Invalid path: {}", e)))?;

        let mut req = self.client.request(method, url)
            .header("X-Vault-Token", token);

        if let Some(ns) = &self.config.namespace {
            req = req.header("X-Vault-Namespace", ns);
        }

        if let Some(body) = body {
            req = req.json(&body);
        }

        let resp = req.send().await
            .map_err(|e| GenesisError::Vault(format!("Request failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(GenesisError::Vault(format!(
                "Vault request failed ({}): {}",
                status, error_text
            )));
        }

        resp.json().await
            .map_err(|e| GenesisError::Vault(format!("Failed to parse response: {}", e)))
    }

    /// Check if Vault is initialized.
    pub async fn is_initialized(&self) -> Result<bool> {
        #[derive(Deserialize)]
        struct HealthResponse {
            initialized: bool,
        }

        let url = self.base_url.join("/v1/sys/health")
            .map_err(|e| GenesisError::Vault(format!("Invalid URL: {}", e)))?;

        let resp = self.client.get(url)
            .send()
            .await
            .map_err(|e| GenesisError::Vault(format!("Health check failed: {}", e)))?;

        let health: HealthResponse = resp.json().await
            .map_err(|e| GenesisError::Vault(format!("Failed to parse health response: {}", e)))?;

        Ok(health.initialized)
    }

    /// Check if Vault is sealed.
    pub async fn is_sealed(&self) -> Result<bool> {
        #[derive(Deserialize)]
        struct SealStatusResponse {
            sealed: bool,
        }

        let url = self.base_url.join("/v1/sys/seal-status")
            .map_err(|e| GenesisError::Vault(format!("Invalid URL: {}", e)))?;

        let resp = self.client.get(url)
            .send()
            .await
            .map_err(|e| GenesisError::Vault(format!("Seal status check failed: {}", e)))?;

        let status: SealStatusResponse = resp.json().await
            .map_err(|e| GenesisError::Vault(format!("Failed to parse seal status: {}", e)))?;

        Ok(status.sealed)
    }
}

#[async_trait]
impl VaultStore for VaultClient {
    async fn read(&self, path: &str) -> Result<HashMap<String, String>> {
        let full_path = self.build_path(path);

        #[derive(Deserialize)]
        struct Response {
            data: HashMap<String, serde_json::Value>,
        }

        let response: Response = self.request(
            reqwest::Method::GET,
            &format!("/v1/{}", full_path),
            None,
        ).await?;

        // Convert values to strings
        let mut result = HashMap::new();
        for (key, value) in response.data {
            result.insert(key, value.as_str()
                .unwrap_or_default()
                .to_string());
        }

        Ok(result)
    }

    async fn write(&self, path: &str, data: &HashMap<String, String>) -> Result<()> {
        let full_path = self.build_path(path);

        let body = serde_json::json!({ "data": data });

        let _: serde_json::Value = self.request(
            reqwest::Method::POST,
            &format!("/v1/{}", full_path),
            Some(body),
        ).await?;

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        match self.read(path).await {
            Ok(_) => Ok(true),
            Err(GenesisError::Vault(ref e)) if e.contains("404") => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.build_path(path);

        let _: serde_json::Value = self.request(
            reqwest::Method::DELETE,
            &format!("/v1/{}", full_path),
            None,
        ).await?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let full_path = self.build_path(prefix);

        #[derive(Deserialize)]
        struct ListResponse {
            data: ListData,
        }

        #[derive(Deserialize)]
        struct ListData {
            keys: Vec<String>,
        }

        let response: ListResponse = self.request(
            reqwest::Method::GET,
            &format!("/v1/{}?list=true", full_path),
            None,
        ).await?;

        Ok(response.data.keys)
    }

    fn base_path(&self) -> &str {
        &self.config.mount
    }

    fn url(&self) -> &str {
        &self.config.url
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_path() {
        let config = VaultConfig {
            mount: "/secret/".to_string(),
            ..Default::default()
        };

        let client = VaultClient::new(config).unwrap();
        assert_eq!(client.build_path("test/path"), "/secret/test/path");
        assert_eq!(client.build_path("/test/path"), "/secret/test/path");
    }
}
