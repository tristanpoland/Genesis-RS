//! Kit metadata parsing and validation.

use genesis_types::{GenesisError, Result, SemVer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Kit metadata from kit.yml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitMetadata {
    /// Kit name
    pub name: String,

    /// Kit version
    pub version: String,

    /// Kit author
    #[serde(default)]
    pub author: String,

    /// Kit homepage
    #[serde(default)]
    pub homepage: String,

    /// Kit description
    #[serde(default)]
    pub description: String,

    /// Minimum Genesis version required
    #[serde(default)]
    pub genesis_version_min: Option<String>,

    /// Supported infrastructure providers
    #[serde(default)]
    pub supports: Vec<String>,

    /// Available features
    #[serde(default)]
    pub features: HashMap<String, FeatureMetadata>,

    /// Feature groups
    #[serde(default)]
    pub feature_groups: HashMap<String, Vec<String>>,

    /// Required environment parameters
    #[serde(default)]
    pub params: HashMap<String, ParamMetadata>,

    /// Exodus data produced by this kit
    #[serde(default)]
    pub exodus: HashMap<String, ExodusMetadata>,

    /// Required software/versions
    #[serde(default)]
    pub prereqs: Vec<PrereqMetadata>,
}

/// Feature metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMetadata {
    /// Feature description
    #[serde(default)]
    pub description: String,

    /// Features this feature depends on
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Features incompatible with this one
    #[serde(default)]
    pub conflicts_with: Vec<String>,

    /// Whether this is a default feature
    #[serde(default)]
    pub default: bool,
}

/// Parameter metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamMetadata {
    /// Parameter description
    #[serde(default)]
    pub description: String,

    /// Whether parameter is required
    #[serde(default)]
    pub required: bool,

    /// Default value
    #[serde(default)]
    pub default: Option<serde_json::Value>,

    /// Example value
    #[serde(default)]
    pub example: Option<String>,

    /// Validation pattern
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Exodus data metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExodusMetadata {
    /// Description of exodus data
    #[serde(default)]
    pub description: String,

    /// Data type
    #[serde(default)]
    pub data_type: Option<String>,
}

/// Prerequisite metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrereqMetadata {
    /// Binary name
    pub binary: String,

    /// Minimum version
    #[serde(default)]
    pub version: Option<String>,

    /// Whether this is required
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

impl KitMetadata {
    /// Load metadata from kit.yml file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| GenesisError::Kit(format!("Failed to read kit.yml: {}", e)))?;

        serde_yaml::from_str(&content)
            .map_err(|e| GenesisError::Kit(format!("Failed to parse kit.yml: {}", e)))
    }

    /// Validate metadata.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(GenesisError::Kit("Kit name cannot be empty".to_string()));
        }

        if self.version.is_empty() {
            return Err(GenesisError::Kit("Kit version cannot be empty".to_string()));
        }

        SemVer::parse(&self.version)
            .map_err(|_| GenesisError::Kit(format!("Invalid kit version: {}", self.version)))?;

        if let Some(ref min_version) = self.genesis_version_min {
            SemVer::parse(min_version)
                .map_err(|_| GenesisError::Kit(format!(
                    "Invalid genesis_version_min: {}",
                    min_version
                )))?;
        }

        Ok(())
    }

    /// Check if a feature exists.
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.contains_key(feature)
    }

    /// Get default features.
    pub fn default_features(&self) -> Vec<String> {
        self.features.iter()
            .filter(|(_, meta)| meta.default)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Validate feature dependencies.
    pub fn validate_features(&self, features: &[String]) -> Result<()> {
        for feature in features {
            if !self.has_feature(feature) {
                return Err(GenesisError::Kit(format!(
                    "Unknown feature: {}",
                    feature
                )));
            }

            let meta = &self.features[feature];

            for dep in &meta.depends_on {
                if !features.contains(dep) {
                    return Err(GenesisError::Kit(format!(
                        "Feature '{}' requires feature '{}'",
                        feature, dep
                    )));
                }
            }

            for conflict in &meta.conflicts_with {
                if features.contains(conflict) {
                    return Err(GenesisError::Kit(format!(
                        "Feature '{}' conflicts with feature '{}'",
                        feature, conflict
                    )));
                }
            }
        }

        Ok(())
    }
}
