//! Manifest types representing different states in the manifest pipeline.

use genesis_types::{GenesisError, Result, EnvName};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

/// Raw YAML content as a string.
pub type YamlContent = String;

/// Parsed YAML as a JSON value (for manipulation).
pub type YamlValue = JsonValue;

/// Manifest metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMetadata {
    /// Environment name
    pub env_name: EnvName,

    /// Kit name used
    pub kit_name: String,

    /// Kit version used
    pub kit_version: String,

    /// Features enabled
    pub features: Vec<String>,

    /// Timestamp of generation
    pub generated_at: DateTime<Utc>,

    /// Genesis version used
    pub genesis_version: String,

    /// Source files merged
    pub source_files: Vec<PathBuf>,
}

impl ManifestMetadata {
    /// Create new metadata.
    pub fn new(
        env_name: EnvName,
        kit_name: impl Into<String>,
        kit_version: impl Into<String>,
        features: Vec<String>,
    ) -> Self {
        Self {
            env_name,
            kit_name: kit_name.into(),
            kit_version: kit_version.into(),
            features,
            generated_at: Utc::now(),
            genesis_version: env!("CARGO_PKG_VERSION").to_string(),
            source_files: Vec::new(),
        }
    }

    /// Add source file.
    pub fn add_source_file(&mut self, path: impl AsRef<Path>) {
        self.source_files.push(path.as_ref().to_path_buf());
    }
}

/// Unevaluated manifest containing raw YAML with Spruce operators.
///
/// This is the initial state after merging all source files but before
/// any Spruce evaluation or secret resolution.
#[derive(Debug, Clone)]
pub struct UnevaluatedManifest {
    /// Raw YAML content with Spruce operators
    pub content: YamlContent,

    /// Manifest metadata
    pub metadata: ManifestMetadata,

    /// Whether this manifest contains Spruce operators
    pub has_operators: bool,
}

impl UnevaluatedManifest {
    /// Create new unevaluated manifest.
    pub fn new(content: YamlContent, metadata: ManifestMetadata) -> Self {
        let has_operators = Self::detect_operators(&content);
        Self {
            content,
            metadata,
            has_operators,
        }
    }

    /// Detect if content contains Spruce operators.
    fn detect_operators(content: &str) -> bool {
        content.contains("((") ||
        content.contains("((!") ||
        content.contains("(($") ||
        content.contains("((@")
    }

    /// Get environment name.
    pub fn env_name(&self) -> &EnvName {
        &self.metadata.env_name
    }

    /// Parse YAML content.
    pub fn parse(&self) -> Result<YamlValue> {
        serde_yaml::from_str(&self.content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))
    }
}

/// Partially evaluated manifest with some Spruce operators resolved.
///
/// This is an intermediate state during evaluation, where some operators
/// have been resolved but others remain (typically secret references).
#[derive(Debug, Clone)]
pub struct PartialManifest {
    /// Partially evaluated YAML content
    pub content: YamlContent,

    /// Manifest metadata
    pub metadata: ManifestMetadata,

    /// Remaining secret paths to resolve
    pub pending_secrets: Vec<String>,
}

impl PartialManifest {
    /// Create new partial manifest.
    pub fn new(content: YamlContent, metadata: ManifestMetadata, pending_secrets: Vec<String>) -> Self {
        Self {
            content,
            metadata,
            pending_secrets,
        }
    }

    /// Check if all secrets are resolved.
    pub fn is_complete(&self) -> bool {
        self.pending_secrets.is_empty()
    }

    /// Get environment name.
    pub fn env_name(&self) -> &EnvName {
        &self.metadata.env_name
    }

    /// Parse YAML content.
    pub fn parse(&self) -> Result<YamlValue> {
        serde_yaml::from_str(&self.content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))
    }
}

/// Redacted manifest with secrets replaced by REDACTED markers.
///
/// This is safe to display to users or write to logs without exposing
/// sensitive information.
#[derive(Debug, Clone)]
pub struct RedactedManifest {
    /// YAML content with secrets redacted
    pub content: YamlContent,

    /// Manifest metadata
    pub metadata: ManifestMetadata,

    /// Paths that were redacted
    pub redacted_paths: Vec<String>,
}

impl RedactedManifest {
    /// Create new redacted manifest.
    pub fn new(content: YamlContent, metadata: ManifestMetadata, redacted_paths: Vec<String>) -> Self {
        Self {
            content,
            metadata,
            redacted_paths,
        }
    }

    /// Get environment name.
    pub fn env_name(&self) -> &EnvName {
        &self.metadata.env_name
    }

    /// Get count of redacted secrets.
    pub fn redaction_count(&self) -> usize {
        self.redacted_paths.len()
    }

    /// Write to file.
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path.as_ref(), &self.content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to write manifest: {}", e)))
    }
}

/// Vaultified manifest with secret values replaced by Vault paths.
///
/// This manifest contains references to where secrets are stored in Vault
/// rather than the actual secret values.
#[derive(Debug, Clone)]
pub struct VaultifiedManifest {
    /// YAML content with Vault path references
    pub content: YamlContent,

    /// Manifest metadata
    pub metadata: ManifestMetadata,

    /// Map of manifest paths to Vault paths
    pub vault_mappings: HashMap<String, String>,
}

impl VaultifiedManifest {
    /// Create new vaultified manifest.
    pub fn new(
        content: YamlContent,
        metadata: ManifestMetadata,
        vault_mappings: HashMap<String, String>,
    ) -> Self {
        Self {
            content,
            metadata,
            vault_mappings,
        }
    }

    /// Get environment name.
    pub fn env_name(&self) -> &EnvName {
        &self.metadata.env_name
    }

    /// Get Vault path for a manifest path.
    pub fn get_vault_path(&self, manifest_path: &str) -> Option<&str> {
        self.vault_mappings.get(manifest_path).map(|s| s.as_str())
    }

    /// Get all Vault paths.
    pub fn vault_paths(&self) -> Vec<&str> {
        self.vault_mappings.values().map(|s| s.as_str()).collect()
    }
}

/// Entombed manifest with all secrets stored in Vault.
///
/// This is a fully evaluated manifest where all secrets have been generated
/// and stored in Vault. It can be deployed to BOSH.
#[derive(Debug, Clone)]
pub struct EntombedManifest {
    /// Fully evaluated YAML content
    pub content: YamlContent,

    /// Manifest metadata
    pub metadata: ManifestMetadata,

    /// Secrets that were stored in Vault
    pub entombed_secrets: Vec<String>,
}

impl EntombedManifest {
    /// Create new entombed manifest.
    pub fn new(content: YamlContent, metadata: ManifestMetadata, entombed_secrets: Vec<String>) -> Self {
        Self {
            content,
            metadata,
            entombed_secrets,
        }
    }

    /// Get environment name.
    pub fn env_name(&self) -> &EnvName {
        &self.metadata.env_name
    }

    /// Get count of entombed secrets.
    pub fn secret_count(&self) -> usize {
        self.entombed_secrets.len()
    }

    /// Parse YAML content.
    pub fn parse(&self) -> Result<YamlValue> {
        serde_yaml::from_str(&self.content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))
    }

    /// Write to file.
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        std::fs::write(path.as_ref(), &self.content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to write manifest: {}", e)))
    }

    /// Convert to BOSH deployment manifest format.
    pub fn to_deployment_yaml(&self) -> &str {
        &self.content
    }
}

/// Cached manifest stored locally for performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedManifest {
    /// Manifest content
    pub content: YamlContent,

    /// Manifest metadata
    pub metadata: ManifestMetadata,

    /// Cache timestamp
    pub cached_at: DateTime<Utc>,

    /// Content hash for validation
    pub content_hash: String,
}

impl CachedManifest {
    /// Create new cached manifest.
    pub fn new(content: YamlContent, metadata: ManifestMetadata) -> Self {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = hex::encode(hasher.finalize());

        Self {
            content,
            metadata,
            cached_at: Utc::now(),
            content_hash,
        }
    }

    /// Validate cache integrity.
    pub fn validate(&self) -> Result<bool> {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(self.content.as_bytes());
        let current_hash = hex::encode(hasher.finalize());

        Ok(current_hash == self.content_hash)
    }

    /// Check if cache is expired.
    pub fn is_expired(&self, max_age: chrono::Duration) -> bool {
        let age = Utc::now() - self.cached_at;
        age > max_age
    }

    /// Load from file.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| GenesisError::Manifest(format!("Failed to read cache: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse cache: {}", e)))
    }

    /// Save to file.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| GenesisError::Manifest(format!("Failed to serialize cache: {}", e)))?;

        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| GenesisError::Manifest(format!("Failed to create cache dir: {}", e)))?;
        }

        std::fs::write(path.as_ref(), content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to write cache: {}", e)))
    }
}

/// Manifest subset containing only specified paths.
#[derive(Debug, Clone)]
pub struct ManifestSubset {
    /// Subset YAML content
    pub content: YamlContent,

    /// Original metadata
    pub metadata: ManifestMetadata,

    /// Paths included in subset
    pub included_paths: Vec<String>,
}

impl ManifestSubset {
    /// Create new manifest subset.
    pub fn new(content: YamlContent, metadata: ManifestMetadata, included_paths: Vec<String>) -> Self {
        Self {
            content,
            metadata,
            included_paths,
        }
    }

    /// Get path count.
    pub fn path_count(&self) -> usize {
        self.included_paths.len()
    }

    /// Parse YAML content.
    pub fn parse(&self) -> Result<YamlValue> {
        serde_yaml::from_str(&self.content)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))
    }
}

/// Manifest diff representing changes between two manifests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestDiff {
    /// Added paths
    pub added: Vec<String>,

    /// Removed paths
    pub removed: Vec<String>,

    /// Modified paths with old and new values
    pub modified: HashMap<String, (JsonValue, JsonValue)>,
}

impl ManifestDiff {
    /// Create new empty diff.
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: HashMap::new(),
        }
    }

    /// Check if diff is empty.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Get total change count.
    pub fn change_count(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

impl Default for ManifestDiff {
    fn default() -> Self {
        Self::new()
    }
}
