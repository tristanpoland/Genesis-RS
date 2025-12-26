//! Deployment orchestration and management.

use super::environment::Environment;
use super::exodus::ExodusManager;
use genesis_types::{GenesisError, Result};
use genesis_kit::Kit;
use genesis_services::{vault::VaultClient, bosh::BoshClient};
use genesis_secrets::plan::SecretPlan;
use genesis_manifest::{ManifestBuilder, EntombedManifest};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, debug};

/// Deployment status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Deployment is pending
    Pending,
    /// Deployment is in progress
    InProgress,
    /// Deployment succeeded
    Success,
    /// Deployment failed
    Failed,
    /// Deployment was cancelled
    Cancelled,
}

/// Deployment record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    /// Deployment ID
    pub id: String,

    /// Environment name
    pub env_name: String,

    /// Kit name
    pub kit_name: String,

    /// Kit version
    pub kit_version: String,

    /// Enabled features
    pub features: Vec<String>,

    /// Deployment status
    pub status: DeploymentStatus,

    /// Start timestamp
    pub started_at: DateTime<Utc>,

    /// End timestamp
    pub completed_at: Option<DateTime<Utc>>,

    /// Deployment duration in seconds
    pub duration_secs: Option<u64>,

    /// Deployer (user)
    pub deployer: Option<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// BOSH task ID
    pub bosh_task_id: Option<String>,

    /// Manifest hash
    pub manifest_hash: String,
}

impl DeploymentRecord {
    /// Create new deployment record.
    pub fn new(
        id: impl Into<String>,
        env: &Environment,
        manifest_hash: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            env_name: env.name.to_string(),
            kit_name: env.kit.name.clone(),
            kit_version: env.kit.version.to_string(),
            features: env.features.clone(),
            status: DeploymentStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            duration_secs: None,
            deployer: None,
            error: None,
            bosh_task_id: None,
            manifest_hash: manifest_hash.into(),
        }
    }

    /// Mark deployment as in progress.
    pub fn start(&mut self) {
        self.status = DeploymentStatus::InProgress;
    }

    /// Mark deployment as succeeded.
    pub fn succeed(&mut self) {
        let now = Utc::now();
        self.status = DeploymentStatus::Success;
        self.completed_at = Some(now);
        self.duration_secs = Some((now - self.started_at).num_seconds() as u64);
    }

    /// Mark deployment as failed.
    pub fn fail(&mut self, error: impl Into<String>) {
        let now = Utc::now();
        self.status = DeploymentStatus::Failed;
        self.completed_at = Some(now);
        self.duration_secs = Some((now - self.started_at).num_seconds() as u64);
        self.error = Some(error.into());
    }

    /// Mark deployment as cancelled.
    pub fn cancel(&mut self) {
        let now = Utc::now();
        self.status = DeploymentStatus::Cancelled;
        self.completed_at = Some(now);
        self.duration_secs = Some((now - self.started_at).num_seconds() as u64);
    }

    /// Check if deployment is complete.
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            DeploymentStatus::Success | DeploymentStatus::Failed | DeploymentStatus::Cancelled
        )
    }

    /// Check if deployment succeeded.
    pub fn is_success(&self) -> bool {
        self.status == DeploymentStatus::Success
    }
}

/// Deployment trait for deploying environments.
#[async_trait]
pub trait Deployer: Send + Sync {
    /// Deploy an environment.
    async fn deploy(
        &self,
        env: &mut Environment,
        kit: &dyn Kit,
        dry_run: bool,
    ) -> Result<DeploymentRecord>;

    /// Delete a deployment.
    async fn delete(&self, env: &Environment) -> Result<()>;

    /// Check deployment status.
    async fn status(&self, env: &Environment) -> Result<Option<DeploymentStatus>>;
}

/// BOSH deployer implementation.
pub struct BoshDeployer {
    bosh_client: BoshClient,
    vault_client: VaultClient,
    exodus_manager: Option<ExodusManager>,
}

impl BoshDeployer {
    /// Create new BOSH deployer.
    pub fn new(bosh_client: BoshClient, vault_client: VaultClient) -> Self {
        Self {
            bosh_client,
            vault_client,
            exodus_manager: None,
        }
    }

    /// Create with exodus manager.
    pub fn with_exodus(mut self, exodus_manager: ExodusManager) -> Self {
        self.exodus_manager = Some(exodus_manager);
        self
    }

    /// Generate secrets for environment.
    async fn generate_secrets(
        &self,
        env: &Environment,
        kit: &dyn Kit,
    ) -> Result<()> {
        info!("Generating secrets for {}", env.name);

        let vault_prefix = env.vault_prefix();

        let secret_plan = SecretPlan::from_kit(
            kit,
            &env.features,
            &vault_prefix,
        )?;

        secret_plan.generate(&self.vault_client, &vault_prefix).await?;

        info!("Generated {} secrets", secret_plan.secrets.len());
        Ok(())
    }

    /// Generate manifest for environment.
    async fn generate_manifest(
        &self,
        env: &Environment,
        kit: &dyn Kit,
    ) -> Result<EntombedManifest> {
        info!("Generating manifest for {}", env.name);

        let env_files = env.yaml_files();
        let vault_prefix = env.vault_prefix();

        let manifest = ManifestBuilder::new(kit)
            .add_env_files(env_files)
            .add_features(env.features.clone())
            .with_vault_prefix(vault_prefix)
            .generate_entombed(&self.vault_client)
            .await?;

        info!("Generated manifest with {} secrets", manifest.secret_count());
        Ok(manifest)
    }

    /// Calculate manifest hash.
    fn manifest_hash(manifest: &EntombedManifest) -> String {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(manifest.content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Extract exodus data from manifest.
    fn extract_exodus(&self, manifest: &EntombedManifest) -> Result<genesis_manifest::types::YamlValue> {
        use genesis_manifest::Manifest;

        let exodus_paths = Manifest::find_paths(&manifest.content, ".*exodus.*")?;

        if exodus_paths.is_empty() {
            return Ok(serde_json::json!({}));
        }

        let exodus_yaml = Manifest::cherry_pick(&manifest.content, &exodus_paths)?;
        let exodus_value: serde_json::Value = serde_yaml::from_str(&exodus_yaml)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse exodus data: {}", e)))?;

        Ok(exodus_value)
    }

    /// Save exodus data.
    async fn save_exodus(
        &self,
        env: &Environment,
        manifest: &EntombedManifest,
    ) -> Result<()> {
        if let Some(ref exodus_manager) = self.exodus_manager {
            let exodus_value = self.extract_exodus(manifest)?;

            if let serde_json::Value::Object(map) = exodus_value {
                for (key, value) in map {
                    exodus_manager.set_value(&env.name, key, value)?;
                }
            }

            info!("Saved exodus data for {}", env.name);
        }

        Ok(())
    }
}

#[async_trait]
impl Deployer for BoshDeployer {
    async fn deploy(
        &self,
        env: &mut Environment,
        kit: &dyn Kit,
        dry_run: bool,
    ) -> Result<DeploymentRecord> {
        let deployment_id = uuid::Uuid::new_v4().to_string();

        info!("Starting deployment {} for {}", deployment_id, env.name);

        self.generate_secrets(env, kit).await?;

        let manifest = self.generate_manifest(env, kit).await?;

        let manifest_hash = Self::manifest_hash(&manifest);
        let mut record = DeploymentRecord::new(&deployment_id, env, &manifest_hash);
        record.start();

        if dry_run {
            info!("Dry run mode - skipping actual deployment");
            record.succeed();
            return Ok(record);
        }

        let deployment_name = env.deployment_name();

        match self.bosh_client.deploy(&deployment_name, &manifest.content).await {
            Ok(task_id) => {
                record.bosh_task_id = Some(task_id.clone());

                self.save_exodus(env, &manifest).await?;

                env.record_deployment();
                env.save()?;

                record.succeed();
                info!("Deployment {} succeeded", deployment_id);
            }
            Err(e) => {
                let error_msg = format!("BOSH deployment failed: {}", e);
                record.fail(&error_msg);
                info!("Deployment {} failed: {}", deployment_id, error_msg);
                return Err(e);
            }
        }

        Ok(record)
    }

    async fn delete(&self, env: &Environment) -> Result<()> {
        let deployment_name = env.deployment_name();
        info!("Deleting deployment {}", deployment_name);

        self.bosh_client.delete_deployment(&deployment_name).await?;

        if let Some(ref exodus_manager) = self.exodus_manager {
            exodus_manager.delete(&env.name)?;
        }

        info!("Deleted deployment {}", deployment_name);
        Ok(())
    }

    async fn status(&self, env: &Environment) -> Result<Option<DeploymentStatus>> {
        let deployment_name = env.deployment_name();

        match self.bosh_client.get_deployment(&deployment_name).await {
            Ok(_) => Ok(Some(DeploymentStatus::Success)),
            Err(GenesisError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

/// Deployment history manager.
pub struct DeploymentHistory {
    history_dir: PathBuf,
}

impl DeploymentHistory {
    /// Create new deployment history manager.
    pub fn new(history_dir: impl AsRef<std::path::Path>) -> Self {
        Self {
            history_dir: history_dir.as_ref().to_path_buf(),
        }
    }

    /// Record deployment.
    pub fn record(&self, deployment: &DeploymentRecord) -> Result<()> {
        std::fs::create_dir_all(&self.history_dir)
            .map_err(|e| GenesisError::Environment(format!("Failed to create history directory: {}", e)))?;

        let file_path = self.history_dir.join(format!("{}.json", deployment.id));

        let content = serde_json::to_string_pretty(deployment)
            .map_err(|e| GenesisError::Environment(format!("Failed to serialize deployment record: {}", e)))?;

        std::fs::write(&file_path, content)
            .map_err(|e| GenesisError::Environment(format!("Failed to write deployment record: {}", e)))?;

        debug!("Recorded deployment {}", deployment.id);
        Ok(())
    }

    /// Get deployment by ID.
    pub fn get(&self, id: &str) -> Result<Option<DeploymentRecord>> {
        let file_path = self.history_dir.join(format!("{}.json", id));

        if !file_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| GenesisError::Environment(format!("Failed to read deployment record: {}", e)))?;

        let record = serde_json::from_str(&content)
            .map_err(|e| GenesisError::Environment(format!("Failed to parse deployment record: {}", e)))?;

        Ok(Some(record))
    }

    /// List all deployments.
    pub fn list(&self) -> Result<Vec<DeploymentRecord>> {
        if !self.history_dir.exists() {
            return Ok(Vec::new());
        }

        let mut deployments = Vec::new();

        let entries = std::fs::read_dir(&self.history_dir)
            .map_err(|e| GenesisError::Environment(format!("Failed to read history directory: {}", e)))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(record) = serde_json::from_str(&content) {
                            deployments.push(record);
                        }
                    }
                }
            }
        }

        deployments.sort_by(|a, b| b.started_at.cmp(&a.started_at));

        Ok(deployments)
    }

    /// List deployments for a specific environment.
    pub fn list_for_env(&self, env_name: &str) -> Result<Vec<DeploymentRecord>> {
        let all = self.list()?;
        Ok(all.into_iter()
            .filter(|d| d.env_name == env_name)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_types::{EnvName, SemVer, KitId};
    use tempfile::TempDir;

    #[test]
    fn test_deployment_record() {
        let temp_dir = TempDir::new().unwrap();
        let env_name = EnvName::new("test-env").unwrap();
        let kit_id = KitId {
            name: "test-kit".to_string(),
            version: SemVer::parse("1.0.0").unwrap(),
        };

        let env = Environment::new(env_name, temp_dir.path(), kit_id);
        let mut record = DeploymentRecord::new("test-id", &env, "hash123");

        record.start();
        assert_eq!(record.status, DeploymentStatus::InProgress);

        record.succeed();
        assert_eq!(record.status, DeploymentStatus::Success);
        assert!(record.is_complete());
        assert!(record.is_success());
    }

    #[test]
    fn test_deployment_history() {
        let temp_dir = TempDir::new().unwrap();
        let history = DeploymentHistory::new(temp_dir.path());

        let env_name = EnvName::new("test-env").unwrap();
        let kit_id = KitId {
            name: "test-kit".to_string(),
            version: SemVer::parse("1.0.0").unwrap(),
        };

        let env = Environment::new(env_name, temp_dir.path(), kit_id);
        let mut record = DeploymentRecord::new("test-id", &env, "hash123");
        record.succeed();

        history.record(&record).unwrap();

        let loaded = history.get("test-id").unwrap().unwrap();
        assert_eq!(loaded.id, "test-id");
        assert_eq!(loaded.status, DeploymentStatus::Success);
    }
}
