//! Manifest builder for orchestrating the manifest generation pipeline.

use super::provider::ManifestProvider;
use super::types::*;
use genesis_types::{GenesisError, Result};
use genesis_kit::Kit;
use genesis_services::vault::VaultClient;
use std::path::PathBuf;
use tracing::{info, debug};

/// Manifest builder for step-by-step manifest generation.
pub struct ManifestBuilder<'a> {
    kit: &'a dyn Kit,
    env_files: Vec<PathBuf>,
    features: Vec<String>,
    provider: Box<dyn ManifestProvider>,
    vault_prefix: Option<String>,
}

impl<'a> ManifestBuilder<'a> {
    /// Create new manifest builder.
    pub fn new(kit: &'a dyn Kit) -> Self {
        Self {
            kit,
            env_files: Vec::new(),
            features: Vec::new(),
            provider: super::provider::ManifestProviderFactory::standard(),
            vault_prefix: None,
        }
    }

    /// Add environment file.
    pub fn add_env_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.env_files.push(path.into());
        self
    }

    /// Add multiple environment files.
    pub fn add_env_files(mut self, paths: Vec<PathBuf>) -> Self {
        self.env_files.extend(paths);
        self
    }

    /// Add feature.
    pub fn add_feature(mut self, feature: impl Into<String>) -> Self {
        self.features.push(feature.into());
        self
    }

    /// Add multiple features.
    pub fn add_features(mut self, features: Vec<String>) -> Self {
        self.features.extend(features);
        self
    }

    /// Set manifest provider.
    pub fn with_provider(mut self, provider: Box<dyn ManifestProvider>) -> Self {
        self.provider = provider;
        self
    }

    /// Set Vault prefix for secret storage.
    pub fn with_vault_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.vault_prefix = Some(prefix.into());
        self
    }

    /// Generate unevaluated manifest.
    pub async fn generate_unevaluated(self) -> Result<UnevaluatedManifest> {
        if self.env_files.is_empty() {
            return Err(GenesisError::Manifest("No environment files specified".to_string()));
        }

        info!(
            "Generating unevaluated manifest with {} features",
            self.features.len()
        );

        self.provider
            .generate_unevaluated(self.kit, &self.env_files, &self.features)
            .await
    }

    /// Generate partial manifest (evaluated but not finalized).
    pub async fn generate_partial(self) -> Result<PartialManifest> {
        let unevaluated = self.generate_unevaluated().await?;

        info!("Evaluating manifest");
        self.provider.evaluate(&unevaluated).await
    }

    /// Generate redacted manifest.
    pub async fn generate_redacted(self, secret_paths: Vec<String>) -> Result<RedactedManifest> {
        let partial = self.generate_partial().await?;

        info!("Redacting {} secrets", secret_paths.len());
        self.provider.redact(&partial, &secret_paths).await
    }

    /// Generate vaultified manifest.
    pub async fn generate_vaultified(self, secret_paths: Vec<String>) -> Result<VaultifiedManifest> {
        let vault_prefix = self.vault_prefix
            .as_ref()
            .ok_or_else(|| GenesisError::Manifest("Vault prefix not set".to_string()))?;

        let partial = self.generate_partial().await?;

        info!("Vaultifying {} secrets", secret_paths.len());
        self.provider.vaultify(&partial, vault_prefix, &secret_paths).await
    }

    /// Generate entombed manifest (fully ready for deployment).
    pub async fn generate_entombed(self, vault_client: &VaultClient) -> Result<EntombedManifest> {
        let vault_prefix = self.vault_prefix
            .as_ref()
            .ok_or_else(|| GenesisError::Manifest("Vault prefix not set".to_string()))?;

        let partial = self.generate_partial().await?;

        info!("Entombing manifest");
        self.provider.entomb(&partial, vault_client, vault_prefix).await
    }
}

/// Manifest pipeline for complete manifest generation workflow.
pub struct ManifestPipeline {
    provider: Box<dyn ManifestProvider>,
}

impl ManifestPipeline {
    /// Create new manifest pipeline.
    pub fn new(provider: Box<dyn ManifestProvider>) -> Self {
        Self { provider }
    }

    /// Create with standard provider.
    pub fn standard() -> Self {
        Self {
            provider: super::provider::ManifestProviderFactory::standard(),
        }
    }

    /// Create with cached provider.
    pub fn cached(cache_dir: impl AsRef<std::path::Path>) -> Self {
        Self {
            provider: super::provider::ManifestProviderFactory::cached(cache_dir),
        }
    }

    /// Execute full pipeline to generate deployment-ready manifest.
    pub async fn execute(
        &self,
        kit: &dyn Kit,
        env_files: &[PathBuf],
        features: &[String],
        vault_client: &VaultClient,
        vault_prefix: &str,
    ) -> Result<PipelineResult> {
        info!("Starting manifest pipeline");

        debug!("Step 1: Generate unevaluated manifest");
        let unevaluated = self.provider
            .generate_unevaluated(kit, env_files, features)
            .await?;

        debug!("Step 2: Evaluate manifest");
        let partial = self.provider.evaluate(&unevaluated).await?;

        debug!("Step 3: Extract secret paths");
        let secret_paths = partial.pending_secrets.clone();

        debug!("Step 4: Generate redacted version");
        let redacted = self.provider.redact(&partial, &secret_paths).await?;

        debug!("Step 5: Generate vaultified version");
        let vaultified = self.provider
            .vaultify(&partial, vault_prefix, &secret_paths)
            .await?;

        debug!("Step 6: Entomb manifest");
        let entombed = self.provider
            .entomb(&partial, vault_client, vault_prefix)
            .await?;

        info!("Manifest pipeline completed successfully");

        Ok(PipelineResult {
            unevaluated,
            partial,
            redacted,
            vaultified,
            entombed,
        })
    }

    /// Execute pipeline up to partial evaluation.
    pub async fn execute_partial(
        &self,
        kit: &dyn Kit,
        env_files: &[PathBuf],
        features: &[String],
    ) -> Result<PartialPipelineResult> {
        info!("Starting partial manifest pipeline");

        let unevaluated = self.provider
            .generate_unevaluated(kit, env_files, features)
            .await?;

        let partial = self.provider.evaluate(&unevaluated).await?;

        info!("Partial pipeline completed successfully");

        Ok(PartialPipelineResult {
            unevaluated,
            partial,
        })
    }
}

/// Full pipeline execution result.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Unevaluated manifest
    pub unevaluated: UnevaluatedManifest,

    /// Partially evaluated manifest
    pub partial: PartialManifest,

    /// Redacted manifest
    pub redacted: RedactedManifest,

    /// Vaultified manifest
    pub vaultified: VaultifiedManifest,

    /// Entombed manifest (deployment-ready)
    pub entombed: EntombedManifest,
}

impl PipelineResult {
    /// Get deployment-ready manifest content.
    pub fn deployment_manifest(&self) -> &str {
        self.entombed.to_deployment_yaml()
    }

    /// Get redacted manifest for display.
    pub fn display_manifest(&self) -> &str {
        &self.redacted.content
    }

    /// Get environment name.
    pub fn env_name(&self) -> &genesis_types::EnvName {
        self.entombed.env_name()
    }

    /// Get kit information.
    pub fn kit_info(&self) -> (&str, &str) {
        (
            &self.entombed.metadata.kit_name,
            &self.entombed.metadata.kit_version,
        )
    }

    /// Get enabled features.
    pub fn features(&self) -> &[String] {
        &self.entombed.metadata.features
    }

    /// Get secret count.
    pub fn secret_count(&self) -> usize {
        self.entombed.secret_count()
    }
}

/// Partial pipeline execution result.
#[derive(Debug, Clone)]
pub struct PartialPipelineResult {
    /// Unevaluated manifest
    pub unevaluated: UnevaluatedManifest,

    /// Partially evaluated manifest
    pub partial: PartialManifest,
}

impl PartialPipelineResult {
    /// Get pending secret paths.
    pub fn pending_secrets(&self) -> &[String] {
        &self.partial.pending_secrets
    }

    /// Check if evaluation is complete.
    pub fn is_complete(&self) -> bool {
        self.partial.is_complete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_pattern() {
        use genesis_kit::DevKit;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let kit_dir = temp_dir.path().join("test-kit");
        std::fs::create_dir_all(&kit_dir).unwrap();

        let kit_yml = kit_dir.join("kit.yml");
        std::fs::write(&kit_yml, "name: test-kit\nversion: 1.0.0\n").unwrap();

        let kit = DevKit::from_directory(&kit_dir).unwrap();

        let builder = ManifestBuilder::new(&kit)
            .add_env_file(temp_dir.path().join("env1.yml"))
            .add_env_file(temp_dir.path().join("env2.yml"))
            .add_feature("feature1")
            .add_feature("feature2")
            .with_vault_prefix("secret/test");

        assert_eq!(builder.env_files.len(), 2);
        assert_eq!(builder.features.len(), 2);
        assert_eq!(builder.vault_prefix, Some("secret/test".to_string()));
    }
}
