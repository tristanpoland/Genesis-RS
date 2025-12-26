//! Configuration management for Genesis.
//!
//! This module provides multi-layer configuration support with:
//! - File-based configuration
//! - Environment variable overrides
//! - Programmatic updates
//! - Schema validation
//! - Auto-save capability
//!
//! ## Configuration Layers
//!
//! Configuration values are resolved in this priority order:
//! 1. Environment variables
//! 2. Programmatically set values
//! 3. Values loaded from file
//! 4. Default values
//!
//! ## Example
//!
//! ```rust
//! use genesis_core::config::{Config, GlobalConfig};
//!
//! // Load global configuration
//! let config = GlobalConfig::load()?;
//!
//! // Get a value (with priority resolution)
//! let show_duration: bool = config.get("show_duration").unwrap_or(false);
//!
//! // Set a value programmatically
//! config.set("show_duration", true)?;
//!
//! // Save to file
//! config.save()?;
//! ```

use genesis_types::{GenesisError, Result};
use genesis_types::config::{ProviderConfig, SecretsProviderConfig, DeploymentRoot, LogConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

/// Configuration layer priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConfigLayer {
    /// Default values
    Default = 0,
    /// Values loaded from file
    Loaded = 1,
    /// Values set programmatically
    Set = 2,
    /// Values from environment variables
    Environment = 3,
}

/// Main configuration structure with multi-layer support.
///
/// This is the low-level configuration type. For specific configuration
/// types, see `GlobalConfig` and `RepoConfig`.
#[derive(Clone, Debug)]
pub struct Config {
    layers: HashMap<ConfigLayer, Value>,
    file_path: Option<PathBuf>,
    auto_save: bool,
    schema: Option<Value>,
}

impl Config {
    /// Create a new configuration from a file path.
    ///
    /// If the file doesn't exist, an empty configuration is created.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut layers = HashMap::new();

        // Load file if it exists
        if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| GenesisError::Config(format!("Failed to read config file: {}", e)))?;

            let value: Value = serde_yaml::from_str(&content)
                .map_err(|e| GenesisError::Config(format!("Failed to parse config: {}", e)))?;

            layers.insert(ConfigLayer::Loaded, value);
        }

        Ok(Self {
            layers,
            file_path: Some(path.to_path_buf()),
            auto_save: false,
            schema: None,
        })
    }

    /// Get a configuration value by key, respecting layer priority.
    ///
    /// Returns None if the key doesn't exist in any layer.
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        // Check layers in priority order (highest to lowest)
        let layers = [
            ConfigLayer::Environment,
            ConfigLayer::Set,
            ConfigLayer::Loaded,
            ConfigLayer::Default,
        ];

        for layer in &layers {
            if let Some(layer_data) = self.layers.get(layer) {
                if let Some(value) = self.get_value_at_path(layer_data, key) {
                    if let Ok(typed_value) = serde_json::from_value(value.clone()) {
                        return Some(typed_value);
                    }
                }
            }
        }

        None
    }

    /// Set a configuration value programmatically.
    pub fn set(&mut self, key: &str, value: impl Serialize) -> Result<()> {
        let value = serde_json::to_value(value)
            .map_err(|e| GenesisError::Config(format!("Failed to serialize value: {}", e)))?;

        let set_layer = self.layers.entry(ConfigLayer::Set).or_insert(Value::Object(Default::default()));

        Self::set_value_at_path_impl(set_layer, key, value)?;

        if self.auto_save {
            self.save()?;
        }

        Ok(())
    }

    /// Save configuration to file.
    pub fn save(&self) -> Result<()> {
        let path = self.file_path.as_ref().ok_or_else(|| {
            GenesisError::Config("Cannot save: no file path set".to_string())
        })?;

        // Merge all layers for saving
        let merged = self.merge_layers();

        let yaml = serde_yaml::to_string(&merged)
            .map_err(|e| GenesisError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, yaml)
            .map_err(|e| GenesisError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Get merged data from all layers.
    fn merged_data(&self) -> Value {
        let layers = [
            ConfigLayer::Default,
            ConfigLayer::Loaded,
            ConfigLayer::Set,
            ConfigLayer::Environment,
        ];

        let mut merged = Value::Object(serde_json::Map::new());

        for layer in &layers {
            if let Some(layer_data) = self.layers.get(layer) {
                merged = crate::util::data::deep_merge(merged, layer_data.clone());
            }
        }

        merged
    }

    /// Validate configuration against schema (if set).
    pub fn validate(&self) -> Result<()> {
        if let Some(schema) = &self.schema {
            let instance = self.merged_data();

            let compiled = jsonschema::JSONSchema::compile(schema)
                .map_err(|e| GenesisError::Config(format!("Invalid schema: {}", e)))?;

            let result = compiled.validate(&instance);
            if let Err(errors) = result {
                let error_msgs: Vec<String> = errors
                    .map(|e| format!("{}", e))
                    .collect();
                return Err(GenesisError::Config(format!(
                    "Validation failed: {}",
                    error_msgs.join(", ")
                )));
            }
        }
        Ok(())
    }

    /// Enable auto-save on changes.
    pub fn with_auto_save(mut self, auto_save: bool) -> Self {
        self.auto_save = auto_save;
        self
    }

    /// Set validation schema.
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.schema = Some(schema);
        self
    }

    // Helper: Get value at dotted path
    fn get_value_at_path<'a>(&self, data: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = data;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    // Helper: Set value at dotted path
    fn set_value_at_path_impl(data: &mut Value, path: &str, value: Value) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Err(GenesisError::Config("Empty path".to_string()));
        }

        // Navigate to parent, creating intermediate objects as needed
        let mut current = data;
        for part in &parts[..parts.len() - 1] {
            if !current.is_object() {
                *current = Value::Object(Default::default());
            }
            if current.get(part).is_none() {
                current.as_object_mut().unwrap().insert(part.to_string(), Value::Object(Default::default()));
            }
            current = current.get_mut(part).unwrap();
        }

        // Set final value
        if let Some(obj) = current.as_object_mut() {
            obj.insert(parts.last().unwrap().to_string(), value);
        }

        Ok(())
    }

    // Helper: Merge all layers
    fn merge_layers(&self) -> Value {
        let layers = [
            ConfigLayer::Default,
            ConfigLayer::Loaded,
            ConfigLayer::Set,
            ConfigLayer::Environment,
        ];

        let mut merged = Value::Object(Default::default());

        for layer in &layers {
            if let Some(layer_data) = self.layers.get(layer) {
                merged = super::util::data::deep_merge(merged, layer_data.clone());
            }
        }

        merged
    }
}

/// Global Genesis configuration (~/.genesis/config).
///
/// This represents user-wide settings stored in the home directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Whether to show execution duration
    #[serde(default)]
    pub show_duration: bool,

    /// Output style preference
    #[serde(default)]
    pub output_style: String,

    /// Deployment roots configuration
    #[serde(default)]
    pub deployment_roots: Vec<DeploymentRoot>,

    /// Default kit provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kit_provider: Option<ProviderConfig>,

    /// Default secrets provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets_provider: Option<SecretsProviderConfig>,

    /// Log configurations
    #[serde(default)]
    pub logs: Vec<LogConfig>,
}

impl GlobalConfig {
    /// Load global configuration from default location.
    pub fn load() -> Result<Self> {
        Self::load_from(Self::default_path())
    }

    /// Load global configuration from specific path.
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let config = Config::load(path)?;
        let global_config: GlobalConfig = serde_json::from_value(config.merged_data())
            .map_err(|e| GenesisError::Config(format!("Failed to parse global config: {}", e)))?;
        Ok(global_config)
    }

    /// Get the default path for global configuration.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".genesis")
            .join("config")
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            show_duration: false,
            output_style: "pretty".to_string(),
            deployment_roots: Vec::new(),
            kit_provider: None,
            secrets_provider: None,
            logs: Vec::new(),
        }
    }
}

/// Repository configuration (.genesis/config).
///
/// This represents settings specific to a Genesis deployment repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    /// Deployment type (e.g., "shield", "vault")
    pub deployment_type: String,

    /// Configuration version (should be 2 for Genesis v2+)
    pub version: u32,

    /// Minimum Genesis version required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_version: Option<String>,

    /// Creator Genesis version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_version: Option<String>,

    /// Manifest storage method
    #[serde(default = "default_manifest_store")]
    pub manifest_store: String,

    /// Path to kits directory (relative or absolute)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kits_path: Option<PathBuf>,

    /// Secrets provider configuration
    pub secrets_provider: SecretsProviderConfig,

    /// Kit provider configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kit_provider: Option<ProviderConfig>,
}

fn default_manifest_store() -> String {
    "exodus".to_string()
}

impl RepoConfig {
    /// Load repository configuration from .genesis/config
    pub fn load(repo_path: impl AsRef<Path>) -> Result<Self> {
        let config_path = repo_path.as_ref().join(".genesis").join("config");
        let config = Config::load(config_path)?;
        let repo_config: RepoConfig = serde_json::from_value(config.merged_data())
            .map_err(|e| GenesisError::Config(format!("Failed to parse repo config: {}", e)))?;
        Ok(repo_config)
    }

    /// Load with fallback to defaults
    pub fn load_or_default(repo_path: impl AsRef<Path>) -> Self {
        Self::load(&repo_path).unwrap_or_else(|_| Self {
            deployment_type: "bosh".to_string(),
            version: 2,
            minimum_version: None,
            creator_version: None,
            manifest_store: "exodus".to_string(),
            kits_path: None,
            secrets_provider: SecretsProviderConfig {
                url: "https://127.0.0.1:8200".to_string(),
                insecure: false,
                namespace: None,
                strongbox: true,
                alias: None,
            },
            kit_provider: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_layers() {
        let mut config = Config {
            layers: HashMap::new(),
            file_path: None,
            auto_save: false,
            schema: None,
        };

        // Set default value
        config.layers.insert(
            ConfigLayer::Default,
            serde_json::json!({"key": "default_value"}),
        );

        // Override with loaded value
        config.layers.insert(
            ConfigLayer::Loaded,
            serde_json::json!({"key": "loaded_value"}),
        );

        let value: String = config.get("key").unwrap();
        assert_eq!(value, "loaded_value");
    }
}
