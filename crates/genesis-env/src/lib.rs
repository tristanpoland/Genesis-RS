//! # Genesis Environment
//!
//! Complete environment management system including:
//! - Environment configuration and metadata
//! - Exodus data management (deployment outputs)
//! - Deployment orchestration and history
//! - Feature management
//! - Environment validation

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod environment;
pub mod exodus;
pub mod deployment;

// Re-export main types
pub use environment::{Environment, EnvironmentMetadata, EnvironmentBuilder};
pub use exodus::{ExodusData, ExodusManager};
pub use deployment::{
    Deployer,
    BoshDeployer,
    DeploymentRecord,
    DeploymentStatus,
    DeploymentHistory,
};

use genesis_types::{GenesisError, Result};
use std::path::Path;

/// Environment manager facade for common operations.
pub struct EnvManager;

impl EnvManager {
    /// Create a new environment.
    pub fn create(
        name: genesis_types::EnvName,
        root_dir: impl AsRef<Path>,
        kit: genesis_types::KitId,
    ) -> Result<Environment> {
        EnvironmentBuilder::new()
            .name(name)
            .root_dir(root_dir.as_ref())
            .kit(kit)
            .build()
    }

    /// Load an existing environment.
    pub fn load(path: impl AsRef<Path>) -> Result<Environment> {
        Environment::load(path)
    }

    /// Save an environment.
    pub fn save(env: &Environment) -> Result<()> {
        env.save()
    }

    /// Deploy an environment.
    pub async fn deploy(
        env: &mut Environment,
        kit: &dyn genesis_kit::Kit,
        deployer: &dyn Deployer,
        dry_run: bool,
    ) -> Result<DeploymentRecord> {
        deployer.deploy(env, kit, dry_run).await
    }

    /// Delete a deployment.
    pub async fn delete(
        env: &Environment,
        deployer: &dyn Deployer,
    ) -> Result<()> {
        deployer.delete(env).await
    }

    /// Get deployment status.
    pub async fn status(
        env: &Environment,
        deployer: &dyn Deployer,
    ) -> Result<Option<DeploymentStatus>> {
        deployer.status(env).await
    }

    /// Load exodus data for an environment.
    pub fn load_exodus(
        env: &Environment,
        exodus_manager: &ExodusManager,
    ) -> Result<Option<ExodusData>> {
        exodus_manager.load(&env.name)
    }

    /// Save exodus data for an environment.
    pub fn save_exodus(
        data: &ExodusData,
        exodus_manager: &ExodusManager,
    ) -> Result<()> {
        exodus_manager.save(data)
    }

    /// Import exodus data from another environment.
    pub fn import_exodus(
        from: &genesis_types::EnvName,
        to: &genesis_types::EnvName,
        exodus_manager: &ExodusManager,
        keys: Option<Vec<String>>,
    ) -> Result<()> {
        exodus_manager.import(from, to, keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_types::{EnvName, SemVer, KitId};
    use tempfile::TempDir;

    #[test]
    fn test_env_manager_create() {
        let temp_dir = TempDir::new().unwrap();
        let env_name = EnvName::new("test-env").unwrap();
        let kit_id = KitId {
            name: "test-kit".to_string(),
            version: SemVer::parse("1.0.0").unwrap(),
        };

        let env = EnvManager::create(env_name.clone(), temp_dir.path(), kit_id).unwrap();

        assert_eq!(env.name, env_name);
        assert!(env.root_dir.join("env.yml").exists());
    }

    #[test]
    fn test_env_manager_load_save() {
        let temp_dir = TempDir::new().unwrap();
        let env_name = EnvName::new("test-env").unwrap();
        let kit_id = KitId {
            name: "test-kit".to_string(),
            version: SemVer::parse("1.0.0").unwrap(),
        };

        let env = EnvManager::create(env_name.clone(), temp_dir.path(), kit_id).unwrap();

        let loaded = EnvManager::load(temp_dir.path()).unwrap();
        assert_eq!(loaded.name, env.name);
        assert_eq!(loaded.kit, env.kit);
    }

    #[test]
    fn test_env_manager_exodus() {
        let temp_dir = TempDir::new().unwrap();
        let exodus_dir = temp_dir.path().join("exodus");
        let exodus_manager = ExodusManager::new(&exodus_dir);

        let env_name = EnvName::new("test-env").unwrap();
        let mut data = ExodusData::new(env_name.clone(), "test-kit", "1.0.0");
        data.set("key1", serde_json::json!("value1"));

        EnvManager::save_exodus(&data, &exodus_manager).unwrap();

        let loaded = exodus_manager.load(&env_name).unwrap().unwrap();
        assert_eq!(loaded.get("key1"), Some(&serde_json::json!("value1")));
    }
}
