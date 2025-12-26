//! CredHub client implementation.

use genesis_types::{GenesisError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;
use base64::{Engine as _, engine::general_purpose};

/// CredHub client configuration.
#[derive(Debug, Clone)]
pub struct CredhubConfig {
    /// CredHub URL
    pub url: String,
    /// Client name
    pub client: String,
    /// Client secret
    pub client_secret: String,
    /// CA certificate
    pub ca_cert: Option<String>,
}

/// CredHub client for managing credentials.
pub struct CredhubClient {
    config: CredhubConfig,
    client: Client,
    base_url: Url,
    auth_header: String,
}

impl CredhubClient {
    /// Create a new CredHub client.
    pub fn new(config: CredhubConfig) -> Result<Self> {
        let base_url = Url::parse(&config.url)
            .map_err(|e| GenesisError::Other(format!("Invalid CredHub URL: {}", e)))?;

        let auth_header = format!(
            "Basic {}",
            general_purpose::STANDARD.encode(format!("{}:{}", config.client, config.client_secret))
        );

        let mut builder = Client::builder()
            .timeout(Duration::from_secs(30));

        if let Some(ref ca_cert) = config.ca_cert {
            let cert = reqwest::Certificate::from_pem(ca_cert.as_bytes())
                .map_err(|e| GenesisError::Other(format!("Invalid CA cert: {}", e)))?;
            builder = builder.add_root_certificate(cert);
        }

        let client = builder.build()
            .map_err(|e| GenesisError::Other(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            base_url,
            auth_header,
        })
    }

    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let url = self.base_url.join(path)
            .map_err(|e| GenesisError::Other(format!("Invalid path: {}", e)))?;

        let mut req = self.client.request(method, url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            req = req.json(&body);
        }

        let resp = req.send().await
            .map_err(|e| GenesisError::Other(format!("CredHub request failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(GenesisError::Other(format!(
                "CredHub request failed ({}): {}",
                status, error_text
            )));
        }

        resp.json().await
            .map_err(|e| GenesisError::Other(format!("Failed to parse response: {}", e)))
    }

    /// Get a credential by name.
    pub async fn get(&self, name: &str) -> Result<CredentialValue> {
        #[derive(Deserialize)]
        struct Response {
            data: Vec<CredentialEntry>,
        }

        let response: Response = self.request(
            reqwest::Method::GET,
            &format!("/api/v1/data?name={}&current=true", name),
            None,
        ).await?;

        response.data.into_iter().next()
            .map(|e| e.value)
            .ok_or_else(|| GenesisError::Other(format!("Credential not found: {}", name)))
    }

    /// Set a credential.
    pub async fn set(&self, name: &str, cred_type: &str, value: &serde_json::Value) -> Result<()> {
        let body = serde_json::json!({
            "name": name,
            "type": cred_type,
            "value": value
        });

        let _: serde_json::Value = self.request(
            reqwest::Method::PUT,
            "/api/v1/data",
            Some(body),
        ).await?;

        Ok(())
    }

    /// Delete a credential.
    pub async fn delete(&self, name: &str) -> Result<()> {
        let _: serde_json::Value = self.request(
            reqwest::Method::DELETE,
            &format!("/api/v1/data?name={}", name),
            None,
        ).await?;

        Ok(())
    }

    /// Find credentials by path.
    pub async fn find(&self, path: &str) -> Result<Vec<String>> {
        #[derive(Deserialize)]
        struct FindResponse {
            credentials: Vec<CredentialRef>,
        }

        #[derive(Deserialize)]
        struct CredentialRef {
            name: String,
        }

        let response: FindResponse = self.request(
            reqwest::Method::GET,
            &format!("/api/v1/data?path={}", path),
            None,
        ).await?;

        Ok(response.credentials.into_iter().map(|c| c.name).collect())
    }

    /// Export all credentials under a path.
    pub async fn export(&self, path: &str) -> Result<HashMap<String, CredentialValue>> {
        let names = self.find(path).await?;
        let mut result = HashMap::new();

        for name in names {
            match self.get(&name).await {
                Ok(value) => {
                    result.insert(name, value);
                }
                Err(e) => {
                    tracing::warn!("Failed to get credential {}: {}", name, e);
                }
            }
        }

        Ok(result)
    }

    /// Set a certificate credential.
    pub async fn set_certificate(
        &self,
        name: &str,
        certificate: &str,
        private_key: &str,
        ca: Option<&str>,
    ) -> Result<()> {
        let mut value = serde_json::json!({
            "certificate": certificate,
            "private_key": private_key,
        });

        if let Some(ca) = ca {
            value["ca"] = serde_json::json!(ca);
        }

        self.set(name, "certificate", &value).await
    }

    /// Set an SSH key credential.
    pub async fn set_ssh(
        &self,
        name: &str,
        public_key: &str,
        private_key: &str,
    ) -> Result<()> {
        let value = serde_json::json!({
            "public_key": public_key,
            "private_key": private_key,
        });

        self.set(name, "ssh", &value).await
    }

    /// Set an RSA key credential.
    pub async fn set_rsa(
        &self,
        name: &str,
        public_key: &str,
        private_key: &str,
    ) -> Result<()> {
        let value = serde_json::json!({
            "public_key": public_key,
            "private_key": private_key,
        });

        self.set(name, "rsa", &value).await
    }

    /// Set a password credential.
    pub async fn set_password(&self, name: &str, password: &str) -> Result<()> {
        let value = serde_json::json!(password);
        self.set(name, "password", &value).await
    }

    /// Set a user credential.
    pub async fn set_user(
        &self,
        name: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        let value = serde_json::json!({
            "username": username,
            "password": password,
        });

        self.set(name, "user", &value).await
    }

    /// Set a value credential (simple string).
    pub async fn set_value(&self, name: &str, value: &str) -> Result<()> {
        let value = serde_json::json!(value);
        self.set(name, "value", &value).await
    }

    /// Set a JSON credential.
    pub async fn set_json(&self, name: &str, value: &serde_json::Value) -> Result<()> {
        self.set(name, "json", value).await
    }

    /// Interpolate variables in a manifest.
    pub async fn interpolate(&self, manifest: &str) -> Result<String> {
        let body = serde_json::json!(manifest);

        let response: String = self.request(
            reqwest::Method::POST,
            "/api/v1/interpolate",
            Some(body),
        ).await?;

        Ok(response)
    }
}

/// CredHub credential value (union type).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CredentialValue {
    /// Certificate credential
    Certificate {
        /// Certificate PEM
        certificate: String,
        /// Private key PEM
        private_key: String,
        /// CA certificate PEM
        #[serde(skip_serializing_if = "Option::is_none")]
        ca: Option<String>,
    },
    /// SSH key credential
    Ssh {
        /// Public key
        public_key: String,
        /// Private key
        private_key: String,
    },
    /// RSA key credential
    Rsa {
        /// Public key
        public_key: String,
        /// Private key
        private_key: String,
    },
    /// Password credential
    Password(String),
    /// User credential
    User {
        /// Username
        username: String,
        /// Password
        password: String,
    },
    /// Simple value credential
    Value(String),
    /// JSON credential
    Json(serde_json::Value),
}

#[derive(Debug, Clone, Deserialize)]
struct CredentialEntry {
    #[serde(rename = "type")]
    cred_type: String,
    value: CredentialValue,
    id: String,
    name: String,
}
