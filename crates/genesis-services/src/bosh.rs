//! BOSH director client implementation.

use genesis_types::{GenesisError, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;
use base64::{Engine as _, engine::general_purpose};

/// BOSH client configuration.
#[derive(Debug, Clone)]
pub struct BoshConfig {
    /// BOSH director URL
    pub url: String,
    /// CA certificate
    pub ca_cert: Option<String>,
    /// Client name
    pub client: String,
    /// Client secret
    pub client_secret: String,
    /// Environment name
    pub environment: String,
}

/// BOSH director client.
pub struct BoshClient {
    config: BoshConfig,
    client: Client,
    base_url: Url,
    auth_header: String,
}

impl BoshClient {
    /// Create a new BOSH client.
    pub fn new(config: BoshConfig) -> Result<Self> {
        let base_url = Url::parse(&config.url)
            .map_err(|e| GenesisError::Bosh(format!("Invalid BOSH URL: {}", e)))?;

        let auth_header = format!(
            "Basic {}",
            general_purpose::STANDARD.encode(format!("{}:{}", config.client, config.client_secret))
        );

        let mut builder = Client::builder()
            .timeout(Duration::from_secs(300));

        if let Some(ref ca_cert) = config.ca_cert {
            let cert = reqwest::Certificate::from_pem(ca_cert.as_bytes())
                .map_err(|e| GenesisError::Bosh(format!("Invalid CA cert: {}", e)))?;
            builder = builder.add_root_certificate(cert);
        }

        let client = builder.build()
            .map_err(|e| GenesisError::Bosh(format!("Failed to create HTTP client: {}", e)))?;

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
            .map_err(|e| GenesisError::Bosh(format!("Invalid path: {}", e)))?;

        let mut req = self.client.request(method, url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json");

        if let Some(body) = body {
            req = req.json(&body);
        }

        let resp = req.send().await
            .map_err(|e| GenesisError::Bosh(format!("Request failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let error_text = resp.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(GenesisError::Bosh(format!(
                "BOSH request failed ({}): {}",
                status, error_text
            )));
        }

        resp.json().await
            .map_err(|e| GenesisError::Bosh(format!("Failed to parse response: {}", e)))
    }

    /// Deploy a manifest.
    pub async fn deploy(&self, deployment_name: &str, manifest: &str) -> Result<String> {
        #[derive(Serialize)]
        struct DeployRequest {
            manifest: String,
        }

        #[derive(Deserialize)]
        struct TaskResponse {
            id: u64,
            state: String,
        }

        let body = serde_json::json!({
            "manifest": manifest,
            "context": {
                "deployment_name": deployment_name
            }
        });

        let task: TaskResponse = self.request(
            reqwest::Method::POST,
            "/deployments",
            Some(body),
        ).await?;

        self.wait_for_task(task.id).await?;
        Ok(format!("{}", task.id))
    }

    /// Wait for a task to complete.
    async fn wait_for_task(&self, task_id: u64) -> Result<()> {
        loop {
            #[derive(Deserialize)]
            struct TaskStatus {
                id: u64,
                state: String,
                result: Option<String>,
            }

            let status: TaskStatus = self.request(
                reqwest::Method::GET,
                &format!("/tasks/{}", task_id),
                None,
            ).await?;

            match status.state.as_str() {
                "done" => return Ok(()),
                "error" | "cancelled" | "timeout" => {
                    return Err(GenesisError::Bosh(format!(
                        "Task {} failed with state: {}, result: {:?}",
                        task_id, status.state, status.result
                    )));
                }
                "processing" | "queued" => {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                other => {
                    return Err(GenesisError::Bosh(format!(
                        "Unknown task state: {}",
                        other
                    )));
                }
            }
        }
    }

    /// Get deployment information.
    pub async fn deployment_info(&self, name: &str) -> Result<DeploymentInfo> {
        self.request(
            reqwest::Method::GET,
            &format!("/deployments/{}", name),
            None,
        ).await
    }

    /// List all deployments.
    pub async fn list_deployments(&self) -> Result<Vec<DeploymentSummary>> {
        self.request(
            reqwest::Method::GET,
            "/deployments",
            None,
        ).await
    }

    /// Delete a deployment.
    pub async fn delete_deployment(&self, name: &str, force: bool) -> Result<()> {
        let path = if force {
            format!("/deployments/{}?force=true", name)
        } else {
            format!("/deployments/{}", name)
        };

        #[derive(Deserialize)]
        struct TaskResponse {
            id: u64,
        }

        let task: TaskResponse = self.request(
            reqwest::Method::DELETE,
            &path,
            None,
        ).await?;

        self.wait_for_task(task.id).await
    }

    /// Run an errand.
    pub async fn run_errand(&self, deployment: &str, errand: &str, keep_alive: bool) -> Result<String> {
        let body = serde_json::json!({
            "name": errand,
            "keep_alive": keep_alive
        });

        #[derive(Deserialize)]
        struct TaskResponse {
            id: u64,
        }

        let task: TaskResponse = self.request(
            reqwest::Method::POST,
            &format!("/deployments/{}/errands/{}/runs", deployment, errand),
            Some(body),
        ).await?;

        self.wait_for_task(task.id).await?;

        let output = self.get_task_output(task.id).await?;
        Ok(output)
    }

    /// Get task output.
    async fn get_task_output(&self, task_id: u64) -> Result<String> {
        let url = self.base_url.join(&format!("/tasks/{}/output?type=result", task_id))
            .map_err(|e| GenesisError::Bosh(format!("Invalid URL: {}", e)))?;

        let resp = self.client.get(url)
            .header("Authorization", &self.auth_header)
            .send().await
            .map_err(|e| GenesisError::Bosh(format!("Failed to get task output: {}", e)))?;

        resp.text().await
            .map_err(|e| GenesisError::Bosh(format!("Failed to read task output: {}", e)))
    }

    /// Upload cloud config.
    pub async fn upload_cloud_config(&self, config: &str, name: Option<&str>) -> Result<()> {
        let body = serde_json::json!({
            "config": config,
            "name": name.unwrap_or("default")
        });

        #[derive(Deserialize)]
        struct TaskResponse {
            id: u64,
        }

        let task: TaskResponse = self.request(
            reqwest::Method::POST,
            "/cloud_configs",
            Some(body),
        ).await?;

        self.wait_for_task(task.id).await
    }

    /// Get cloud config.
    pub async fn get_cloud_config(&self, name: Option<&str>) -> Result<String> {
        let path = if let Some(name) = name {
            format!("/cloud_configs?name={}", name)
        } else {
            "/cloud_configs?latest=true".to_string()
        };

        #[derive(Deserialize)]
        struct CloudConfigResponse {
            properties: String,
        }

        let configs: Vec<CloudConfigResponse> = self.request(
            reqwest::Method::GET,
            &path,
            None,
        ).await?;

        configs.first()
            .map(|c| c.properties.clone())
            .ok_or_else(|| GenesisError::Bosh("No cloud config found".to_string()))
    }

    /// Upload runtime config.
    pub async fn upload_runtime_config(&self, config: &str, name: Option<&str>) -> Result<()> {
        let body = serde_json::json!({
            "config": config,
            "name": name.unwrap_or("default")
        });

        #[derive(Deserialize)]
        struct TaskResponse {
            id: u64,
        }

        let task: TaskResponse = self.request(
            reqwest::Method::POST,
            "/runtime_configs",
            Some(body),
        ).await?;

        self.wait_for_task(task.id).await
    }

    /// Get runtime config.
    pub async fn get_runtime_config(&self, name: Option<&str>) -> Result<String> {
        let path = if let Some(name) = name {
            format!("/runtime_configs?name={}", name)
        } else {
            "/runtime_configs?latest=true".to_string()
        };

        #[derive(Deserialize)]
        struct RuntimeConfigResponse {
            properties: String,
        }

        let configs: Vec<RuntimeConfigResponse> = self.request(
            reqwest::Method::GET,
            &path,
            None,
        ).await?;

        configs.first()
            .map(|c| c.properties.clone())
            .ok_or_else(|| GenesisError::Bosh("No runtime config found".to_string()))
    }

    /// List stemcells.
    pub async fn list_stemcells(&self) -> Result<Vec<StemcellInfo>> {
        self.request(
            reqwest::Method::GET,
            "/stemcells",
            None,
        ).await
    }

    /// Upload stemcell.
    pub async fn upload_stemcell(&self, stemcell_url: &str) -> Result<()> {
        let body = serde_json::json!({
            "location": stemcell_url
        });

        #[derive(Deserialize)]
        struct TaskResponse {
            id: u64,
        }

        let task: TaskResponse = self.request(
            reqwest::Method::POST,
            "/stemcells",
            Some(body),
        ).await?;

        self.wait_for_task(task.id).await
    }

    /// Get director info.
    pub async fn info(&self) -> Result<DirectorInfo> {
        self.request(
            reqwest::Method::GET,
            "/info",
            None,
        ).await
    }
}

/// BOSH deployment information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    /// Deployment name
    pub name: String,
    /// Releases used
    pub releases: Vec<ReleaseInfo>,
    /// Stemcells used
    pub stemcells: Vec<StemcellInfo>,
    /// Teams
    #[serde(default)]
    pub teams: Vec<String>,
}

/// BOSH deployment summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSummary {
    /// Deployment name
    pub name: String,
    /// Cloud config
    #[serde(default)]
    pub cloud_config: String,
}

/// BOSH release information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// Release name
    pub name: String,
    /// Release version
    pub version: String,
}

/// BOSH stemcell information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StemcellInfo {
    /// Stemcell name
    pub name: String,
    /// Stemcell version
    pub version: String,
    /// Operating system
    pub os: String,
    /// CPI
    #[serde(default)]
    pub cpi: String,
}

/// BOSH director information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorInfo {
    /// Director name
    pub name: String,
    /// UUID
    pub uuid: String,
    /// Version
    pub version: String,
    /// CPI
    pub cpi: String,
    /// Features
    #[serde(default)]
    pub features: HashMap<String, bool>,
    /// User authentication
    #[serde(default)]
    pub user_authentication: HashMap<String, serde_json::Value>,
}
