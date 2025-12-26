//! # Genesis Manifest
//!
//! Complete manifest generation and transformation pipeline including:
//! - Manifest type system (Unevaluated, Partial, Redacted, Vaultified, Entombed, Cached)
//! - Spruce integration for YAML merging and evaluation
//! - Manifest transformations (cherry-pick, prune, redact, vaultify)
//! - Caching system for performance
//! - Manifest providers and factory
//! - Manifest builder and pipeline

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod types;
pub mod spruce;
pub mod transform;
pub mod cache;
pub mod provider;
pub mod builder;

// Re-export main types
pub use types::{
    YamlContent,
    YamlValue,
    ManifestMetadata,
    UnevaluatedManifest,
    PartialManifest,
    RedactedManifest,
    VaultifiedManifest,
    EntombedManifest,
    CachedManifest,
    ManifestSubset,
    ManifestDiff,
};

pub use spruce::Spruce;
pub use transform::ManifestTransformer;
pub use cache::{ManifestCache, CacheStats, CacheVerification};
pub use provider::{
    ManifestProvider,
    StandardManifestProvider,
    CachedManifestProvider,
    ManifestProviderFactory,
};
pub use builder::{
    ManifestBuilder,
    ManifestPipeline,
    PipelineResult,
    PartialPipelineResult,
};

use genesis_types::{GenesisError, Result};

/// Manifest generation facade for simple use cases.
pub struct Manifest;

impl Manifest {
    /// Generate a deployment-ready manifest.
    pub async fn generate_deployment(
        kit: &dyn genesis_kit::Kit,
        env_files: &[std::path::PathBuf],
        features: &[String],
        vault_client: &genesis_services::vault::VaultClient,
        vault_prefix: &str,
    ) -> Result<EntombedManifest> {
        let pipeline = ManifestPipeline::standard();
        let result = pipeline
            .execute(kit, env_files, features, vault_client, vault_prefix)
            .await?;
        Ok(result.entombed)
    }

    /// Generate a redacted manifest for display.
    pub async fn generate_redacted(
        kit: &dyn genesis_kit::Kit,
        env_files: &[std::path::PathBuf],
        features: &[String],
        secret_paths: Vec<String>,
    ) -> Result<RedactedManifest> {
        let builder = ManifestBuilder::new(kit)
            .add_env_files(env_files.to_vec())
            .add_features(features.to_vec());

        builder.generate_redacted(secret_paths).await
    }

    /// Generate a partial manifest (evaluated but not finalized).
    pub async fn generate_partial(
        kit: &dyn genesis_kit::Kit,
        env_files: &[std::path::PathBuf],
        features: &[String],
    ) -> Result<PartialManifest> {
        let builder = ManifestBuilder::new(kit)
            .add_env_files(env_files.to_vec())
            .add_features(features.to_vec());

        builder.generate_partial().await
    }

    /// Cherry-pick specific paths from a manifest YAML.
    pub fn cherry_pick(yaml: &str, paths: &[String]) -> Result<String> {
        let transformer = ManifestTransformer::new();
        transformer.cherry_pick(yaml, paths)
    }

    /// Prune specific paths from a manifest YAML.
    pub fn prune(yaml: &str, paths: &[String]) -> Result<String> {
        let transformer = ManifestTransformer::new();
        transformer.prune(yaml, paths)
    }

    /// Redact secrets in a manifest YAML.
    pub fn redact(yaml: &str, secret_paths: &[String]) -> Result<String> {
        let transformer = ManifestTransformer::new();
        transformer.redact(yaml, secret_paths)
    }

    /// Extract all paths from a manifest YAML.
    pub fn extract_paths(yaml: &str) -> Result<Vec<String>> {
        let transformer = ManifestTransformer::new();
        transformer.extract_all_paths(yaml)
    }

    /// Find paths matching a pattern in a manifest YAML.
    pub fn find_paths(yaml: &str, pattern: &str) -> Result<Vec<String>> {
        let transformer = ManifestTransformer::new();
        transformer.find_paths(yaml, pattern)
    }

    /// Merge two YAML documents.
    pub fn merge(yaml1: &str, yaml2: &str) -> Result<String> {
        let transformer = ManifestTransformer::new();
        transformer.merge_two(yaml1, yaml2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_cherry_pick() {
        let yaml = r#"
properties:
  username: admin
  password: secret
meta:
  environment: prod
"#;

        let result = Manifest::cherry_pick(yaml, &vec!["properties.username".to_string()]).unwrap();
        assert!(result.contains("username"));
        assert!(!result.contains("password"));
    }

    #[test]
    fn test_manifest_prune() {
        let yaml = r#"
properties:
  username: admin
  password: secret
meta:
  environment: prod
"#;

        let result = Manifest::prune(yaml, &vec!["meta".to_string()]).unwrap();
        assert!(result.contains("properties"));
        assert!(!result.contains("meta"));
    }

    #[test]
    fn test_manifest_redact() {
        let yaml = r#"
properties:
  username: admin
  password: secret123
"#;

        let result = Manifest::redact(yaml, &vec!["properties.password".to_string()]).unwrap();
        assert!(result.contains("REDACTED"));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_manifest_extract_paths() {
        let yaml = r#"
properties:
  username: admin
  database:
    host: localhost
    port: 5432
"#;

        let paths = Manifest::extract_paths(yaml).unwrap();
        assert!(paths.contains(&"properties".to_string()));
        assert!(paths.contains(&"properties.username".to_string()));
        assert!(paths.contains(&"properties.database".to_string()));
        assert!(paths.contains(&"properties.database.host".to_string()));
        assert!(paths.contains(&"properties.database.port".to_string()));
    }
}
