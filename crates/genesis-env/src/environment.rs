//! Genesis environment representation and management.

use genesis_types::{GenesisError, Result, EnvName};
use genesis_kit::{Kit, KitId};
use genesis_core::config::Config;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Genesis environment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Environment name
    pub name: EnvName,

    /// Environment root directory
    pub root_dir: PathBuf,

    /// Environment type (e.g., "bosh", "k8s")
    #[serde(default = "default_env_type")]
    pub env_type: String,

    /// Kit identifier
    pub kit: KitId,

    /// Enabled features
    #[serde(default)]
    pub features: Vec<String>,

    /// Environment parameters
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,

    /// Environment metadata
    #[serde(default)]
    pub metadata: EnvironmentMetadata,

    /// Genesis configuration
    #[serde(skip)]
    pub config: Option<Config>,
}

fn default_env_type() -> String {
    "bosh".to_string()
}

/// Environment metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentMetadata {
    /// Creation timestamp
    pub created_at: Option<DateTime<Utc>>,

    /// Last modified timestamp
    pub modified_at: Option<DateTime<Utc>>,

    /// Last deployed timestamp
    pub deployed_at: Option<DateTime<Utc>>,

    /// Creator
    pub created_by: Option<String>,

    /// Last modifier
    pub modified_by: Option<String>,

    /// Deployment history count
    #[serde(default)]
    pub deployment_count: usize,

    /// Custom metadata
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

impl Environment {
    /// Create new environment.
    pub fn new(
        name: EnvName,
        root_dir: impl AsRef<Path>,
        kit: KitId,
    ) -> Self {
        Self {
            name,
            root_dir: root_dir.as_ref().to_path_buf(),
            env_type: default_env_type(),
            kit,
            features: Vec::new(),
            params: HashMap::new(),
            metadata: EnvironmentMetadata {
                created_at: Some(Utc::now()),
                ..Default::default()
            },
            config: None,
        }
    }

    /// Load environment from directory.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.is_dir() {
            return Err(GenesisError::Environment(format!(
                "Environment path is not a directory: {:?}",
                path
            )));
        }

        let env_yml = path.join("env.yml");
        if !env_yml.exists() {
            return Err(GenesisError::Environment(format!(
                "env.yml not found in {:?}",
                path
            )));
        }

        let content = std::fs::read_to_string(&env_yml)
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to read env.yml: {}",
                e
            )))?;

        let mut env: Self = serde_yaml::from_str(&content)
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to parse env.yml: {}",
                e
            )))?;

        env.root_dir = path.to_path_buf();

        Ok(env)
    }

    /// Save environment to directory.
    pub fn save(&self) -> Result<()> {
        let env_yml = self.root_dir.join("env.yml");

        let content = serde_yaml::to_string(self)
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to serialize environment: {}",
                e
            )))?;

        std::fs::write(&env_yml, content)
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to write env.yml: {}",
                e
            )))?;

        Ok(())
    }

    /// Get environment YAML files.
    pub fn yaml_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();

        let env_yml = self.root_dir.join("env.yml");
        if env_yml.exists() {
            files.push(env_yml);
        }

        let name_yml = self.root_dir.join(format!("{}.yml", self.name));
        if name_yml.exists() {
            files.push(name_yml);
        }

        files
    }

    /// Check if feature is enabled.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// Add feature.
    pub fn add_feature(&mut self, feature: impl Into<String>) {
        let feature = feature.into();
        if !self.has_feature(&feature) {
            self.features.push(feature);
        }
    }

    /// Remove feature.
    pub fn remove_feature(&mut self, feature: &str) {
        self.features.retain(|f| f != feature);
    }

    /// Set parameter.
    pub fn set_param(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.params.insert(key.into(), value);
    }

    /// Get parameter.
    pub fn get_param(&self, key: &str) -> Option<&serde_json::Value> {
        self.params.get(key)
    }

    /// Remove parameter.
    pub fn remove_param(&mut self, key: &str) -> Option<serde_json::Value> {
        self.params.remove(key)
    }

    /// Get Vault prefix for this environment.
    pub fn vault_prefix(&self) -> String {
        format!("secret/{}", self.name)
    }

    /// Get deployment name.
    pub fn deployment_name(&self) -> String {
        format!("{}-{}", self.kit.name, self.name)
    }

    /// Update modification metadata.
    pub fn touch(&mut self, user: Option<String>) {
        self.metadata.modified_at = Some(Utc::now());
        if let Some(user) = user {
            self.metadata.modified_by = Some(user);
        }
    }

    /// Record successful deployment.
    pub fn record_deployment(&mut self) {
        self.metadata.deployed_at = Some(Utc::now());
        self.metadata.deployment_count += 1;
    }

    /// Validate environment configuration.
    pub fn validate(&self) -> Result<()> {
        if self.name.as_str().is_empty() {
            return Err(GenesisError::Environment("Environment name cannot be empty".to_string()));
        }

        if self.kit.name.is_empty() {
            return Err(GenesisError::Environment("Kit name cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Get exodus data path.
    pub fn exodus_path(&self) -> PathBuf {
        self.root_dir.join(".genesis").join("exodus")
    }

    /// Get cached manifests path.
    pub fn cache_path(&self) -> PathBuf {
        self.root_dir.join(".genesis").join("cached")
    }

    /// Get deployment state path.
    pub fn state_path(&self) -> PathBuf {
        self.root_dir.join(".genesis").join("state")
    }

    /// Initialize environment directory structure.
    pub fn init_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root_dir)
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to create environment directory: {}",
                e
            )))?;

        std::fs::create_dir_all(self.exodus_path())
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to create exodus directory: {}",
                e
            )))?;

        std::fs::create_dir_all(self.cache_path())
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to create cache directory: {}",
                e
            )))?;

        std::fs::create_dir_all(self.state_path())
            .map_err(|e| GenesisError::Environment(format!(
                "Failed to create state directory: {}",
                e
            )))?;

        Ok(())
    }
}

/// Environment builder for creating new environments.
pub struct EnvironmentBuilder {
    name: Option<EnvName>,
    root_dir: Option<PathBuf>,
    env_type: String,
    kit: Option<KitId>,
    features: Vec<String>,
    params: HashMap<String, serde_json::Value>,
}

impl EnvironmentBuilder {
    /// Create new environment builder.
    pub fn new() -> Self {
        Self {
            name: None,
            root_dir: None,
            env_type: default_env_type(),
            kit: None,
            features: Vec::new(),
            params: HashMap::new(),
        }
    }

    /// Set environment name.
    pub fn name(mut self, name: EnvName) -> Self {
        self.name = Some(name);
        self
    }

    /// Set root directory.
    pub fn root_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.root_dir = Some(path.into());
        self
    }

    /// Set environment type.
    pub fn env_type(mut self, env_type: impl Into<String>) -> Self {
        self.env_type = env_type.into();
        self
    }

    /// Set kit.
    pub fn kit(mut self, kit: KitId) -> Self {
        self.kit = Some(kit);
        self
    }

    /// Add feature.
    pub fn feature(mut self, feature: impl Into<String>) -> Self {
        self.features.push(feature.into());
        self
    }

    /// Add features.
    pub fn features(mut self, features: Vec<String>) -> Self {
        self.features.extend(features);
        self
    }

    /// Set parameter.
    pub fn param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    /// Build environment.
    pub fn build(self) -> Result<Environment> {
        let name = self.name.ok_or_else(|| GenesisError::Environment("Environment name not set".to_string()))?;
        let root_dir = self.root_dir.ok_or_else(|| GenesisError::Environment("Root directory not set".to_string()))?;
        let kit = self.kit.ok_or_else(|| GenesisError::Environment("Kit not set".to_string()))?;

        let mut env = Environment::new(name, root_dir, kit);
        env.env_type = self.env_type;
        env.features = self.features;
        env.params = self.params;

        env.validate()?;
        env.init_directories()?;
        env.save()?;

        Ok(env)
    }
}

impl Default for EnvironmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_types::SemVer;
    use tempfile::TempDir;

    #[test]
    fn test_environment_creation() {
        let temp_dir = TempDir::new().unwrap();
        let env_name = EnvName::new("test-env").unwrap();
        let kit_id = KitId {
            name: "test-kit".to_string(),
            version: SemVer::parse("1.0.0").unwrap(),
        };

        let env = Environment::new(env_name.clone(), temp_dir.path(), kit_id.clone());

        assert_eq!(env.name, env_name);
        assert_eq!(env.kit, kit_id);
        assert_eq!(env.env_type, "bosh");
    }

    #[test]
    fn test_environment_builder() {
        let temp_dir = TempDir::new().unwrap();
        let env_name = EnvName::new("test-env").unwrap();
        let kit_id = KitId {
            name: "test-kit".to_string(),
            version: SemVer::parse("1.0.0").unwrap(),
        };

        let env = EnvironmentBuilder::new()
            .name(env_name.clone())
            .root_dir(temp_dir.path())
            .kit(kit_id.clone())
            .feature("feature1")
            .feature("feature2")
            .param("param1", serde_json::json!("value1"))
            .build()
            .unwrap();

        assert_eq!(env.features.len(), 2);
        assert_eq!(env.params.len(), 1);
    }

    #[test]
    fn test_feature_management() {
        let temp_dir = TempDir::new().unwrap();
        let env_name = EnvName::new("test-env").unwrap();
        let kit_id = KitId {
            name: "test-kit".to_string(),
            version: SemVer::parse("1.0.0").unwrap(),
        };

        let mut env = Environment::new(env_name, temp_dir.path(), kit_id);

        env.add_feature("feature1");
        assert!(env.has_feature("feature1"));

        env.remove_feature("feature1");
        assert!(!env.has_feature("feature1"));
    }
}
