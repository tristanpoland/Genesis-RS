//! Manifest provider trait and implementations.

use super::types::*;
use super::spruce::Spruce;
use super::cache::ManifestCache;
use super::transform::ManifestTransformer;
use genesis_types::{GenesisError, Result, EnvName};
use genesis_kit::{Kit, Blueprint};
use genesis_services::vault::VaultClient;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tracing::{debug, info};

/// Trait for manifest providers.
#[async_trait]
pub trait ManifestProvider: Send + Sync {
    /// Generate unevaluated manifest from kit and environment.
    async fn generate_unevaluated(
        &self,
        kit: &dyn Kit,
        env_files: &[PathBuf],
        features: &[String],
    ) -> Result<UnevaluatedManifest>;

    /// Evaluate manifest to resolve Spruce operators.
    async fn evaluate(
        &self,
        unevaluated: &UnevaluatedManifest,
    ) -> Result<PartialManifest>;

    /// Redact secrets from manifest.
    async fn redact(
        &self,
        manifest: &PartialManifest,
        secret_paths: &[String],
    ) -> Result<RedactedManifest>;

    /// Vaultify manifest (replace secrets with Vault paths).
    async fn vaultify(
        &self,
        manifest: &PartialManifest,
        vault_prefix: &str,
        secret_paths: &[String],
    ) -> Result<VaultifiedManifest>;

    /// Entomb manifest (store secrets in Vault and finalize).
    async fn entomb(
        &self,
        manifest: &PartialManifest,
        vault_client: &VaultClient,
        vault_prefix: &str,
    ) -> Result<EntombedManifest>;
}

/// Standard manifest provider implementation.
pub struct StandardManifestProvider {
    spruce: Spruce,
    cache: Option<ManifestCache>,
    transformer: ManifestTransformer,
}

impl StandardManifestProvider {
    /// Create new standard manifest provider.
    pub fn new() -> Self {
        Self {
            spruce: Spruce::new(),
            cache: None,
            transformer: ManifestTransformer::new(),
        }
    }

    /// Create with custom Spruce instance.
    pub fn with_spruce(mut self, spruce: Spruce) -> Self {
        self.spruce = spruce;
        self
    }

    /// Enable caching with specified cache directory.
    pub fn with_cache(mut self, cache_dir: impl AsRef<Path>) -> Self {
        self.cache = Some(ManifestCache::new(cache_dir));
        self
    }

    /// Create with custom cache.
    pub fn with_cache_instance(mut self, cache: ManifestCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Build metadata for manifest.
    fn build_metadata(
        &self,
        env_name: EnvName,
        kit: &dyn Kit,
        features: &[String],
        source_files: Vec<PathBuf>,
    ) -> ManifestMetadata {
        let mut metadata = ManifestMetadata::new(
            env_name,
            kit.name(),
            kit.version().to_string(),
            features.to_vec(),
        );

        for file in source_files {
            metadata.add_source_file(file);
        }

        metadata
    }

    /// Merge all manifest source files.
    fn merge_sources(&self, files: &[PathBuf]) -> Result<String> {
        if files.is_empty() {
            return Err(GenesisError::Manifest("No source files to merge".to_string()));
        }

        info!("Merging {} manifest files", files.len());
        for (i, file) in files.iter().enumerate() {
            debug!("  [{}] {:?}", i + 1, file);
        }

        self.spruce.merge(files)
    }
}

impl Default for StandardManifestProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ManifestProvider for StandardManifestProvider {
    async fn generate_unevaluated(
        &self,
        kit: &dyn Kit,
        env_files: &[PathBuf],
        features: &[String],
    ) -> Result<UnevaluatedManifest> {
        let blueprint = kit.blueprint(features)?;

        let mut all_files = Vec::new();
        all_files.extend(blueprint.base.clone());
        all_files.extend(env_files.iter().cloned());
        all_files.extend(blueprint.features.clone());
        all_files.extend(blueprint.subkits.clone());

        for file in &all_files {
            if !file.exists() {
                return Err(GenesisError::Manifest(format!(
                    "Source file not found: {:?}",
                    file
                )));
            }
        }

        let content = self.merge_sources(&all_files)?;

        let env_name = if let Some(env_file) = env_files.first() {
            EnvName::from_path(env_file)?
        } else {
            return Err(GenesisError::Manifest("No environment files provided".to_string()));
        };

        let metadata = self.build_metadata(
            env_name,
            kit,
            features,
            all_files,
        );

        Ok(UnevaluatedManifest::new(content, metadata))
    }

    async fn evaluate(
        &self,
        unevaluated: &UnevaluatedManifest,
    ) -> Result<PartialManifest> {
        use std::io::Write;

        info!("Evaluating manifest for {}", unevaluated.env_name());

        let temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| GenesisError::Manifest(format!("Failed to create temp file: {}", e)))?;

        temp_file.as_file()
            .write_all(unevaluated.content.as_bytes())
            .map_err(|e| GenesisError::Manifest(format!("Failed to write temp file: {}", e)))?;

        let evaluated = self.spruce.eval(temp_file.path())?;

        let pending_secrets = self.spruce.extract_vault_paths(&evaluated)?;

        Ok(PartialManifest::new(
            evaluated,
            unevaluated.metadata.clone(),
            pending_secrets,
        ))
    }

    async fn redact(
        &self,
        manifest: &PartialManifest,
        secret_paths: &[String],
    ) -> Result<RedactedManifest> {
        info!("Redacting {} secrets from manifest", secret_paths.len());

        let redacted_content = self.transformer.redact(&manifest.content, secret_paths)?;

        Ok(RedactedManifest::new(
            redacted_content,
            manifest.metadata.clone(),
            secret_paths.to_vec(),
        ))
    }

    async fn vaultify(
        &self,
        manifest: &PartialManifest,
        vault_prefix: &str,
        secret_paths: &[String],
    ) -> Result<VaultifiedManifest> {
        info!("Vaultifying manifest with {} secrets", secret_paths.len());

        let (vaultified_content, vault_mappings) = self.transformer.vaultify(
            &manifest.content,
            vault_prefix,
            secret_paths,
        )?;

        Ok(VaultifiedManifest::new(
            vaultified_content,
            manifest.metadata.clone(),
            vault_mappings,
        ))
    }

    async fn entomb(
        &self,
        manifest: &PartialManifest,
        vault_client: &VaultClient,
        vault_prefix: &str,
    ) -> Result<EntombedManifest> {
        info!("Entombing manifest for {}", manifest.env_name());

        let mut entombed_secrets = Vec::new();

        for secret_path in &manifest.pending_secrets {
            let parts: Vec<&str> = secret_path.split(':').collect();
            if parts.len() != 2 {
                continue;
            }

            let vault_path = parts[0];
            let key = parts[1];

            let full_path = format!("{}/{}", vault_prefix, vault_path);

            match vault_client.read(&full_path).await {
                Ok(data) => {
                    if data.contains_key(key) {
                        entombed_secrets.push(secret_path.clone());
                    }
                }
                Err(_) => {
                    debug!("Secret not found in vault: {}", full_path);
                }
            }
        }

        use std::io::Write;

        let temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| GenesisError::Manifest(format!("Failed to create temp file: {}", e)))?;

        temp_file.as_file()
            .write_all(manifest.content.as_bytes())
            .map_err(|e| GenesisError::Manifest(format!("Failed to write temp file: {}", e)))?;

        let final_content = self.spruce.eval(temp_file.path())?;

        Ok(EntombedManifest::new(
            final_content,
            manifest.metadata.clone(),
            entombed_secrets,
        ))
    }
}

/// Cached manifest provider that uses caching layer.
pub struct CachedManifestProvider {
    inner: StandardManifestProvider,
    cache: ManifestCache,
}

impl CachedManifestProvider {
    /// Create new cached manifest provider.
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        let cache = ManifestCache::new(cache_dir);
        Self {
            inner: StandardManifestProvider::new(),
            cache,
        }
    }

    /// Create with custom inner provider and cache.
    pub fn with_provider_and_cache(
        provider: StandardManifestProvider,
        cache: ManifestCache,
    ) -> Self {
        Self {
            inner: provider,
            cache,
        }
    }

    /// Clear cache.
    pub fn clear_cache(&self) -> Result<()> {
        self.cache.clear()
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> Result<super::cache::CacheStats> {
        self.cache.stats()
    }
}

#[async_trait]
impl ManifestProvider for CachedManifestProvider {
    async fn generate_unevaluated(
        &self,
        kit: &dyn Kit,
        env_files: &[PathBuf],
        features: &[String],
    ) -> Result<UnevaluatedManifest> {
        self.inner.generate_unevaluated(kit, env_files, features).await
    }

    async fn evaluate(
        &self,
        unevaluated: &UnevaluatedManifest,
    ) -> Result<PartialManifest> {
        let env_name = unevaluated.env_name();

        if let Some(cached) = self.cache.get(env_name)? {
            info!("Using cached manifest for {}", env_name);
            return Ok(PartialManifest::new(
                cached.content,
                cached.metadata,
                vec![],
            ));
        }

        let partial = self.inner.evaluate(unevaluated).await?;

        if partial.is_complete() {
            self.cache.put(
                env_name,
                partial.content.clone(),
                partial.metadata.clone(),
            )?;
        }

        Ok(partial)
    }

    async fn redact(
        &self,
        manifest: &PartialManifest,
        secret_paths: &[String],
    ) -> Result<RedactedManifest> {
        self.inner.redact(manifest, secret_paths).await
    }

    async fn vaultify(
        &self,
        manifest: &PartialManifest,
        vault_prefix: &str,
        secret_paths: &[String],
    ) -> Result<VaultifiedManifest> {
        self.inner.vaultify(manifest, vault_prefix, secret_paths).await
    }

    async fn entomb(
        &self,
        manifest: &PartialManifest,
        vault_client: &VaultClient,
        vault_prefix: &str,
    ) -> Result<EntombedManifest> {
        self.inner.entomb(manifest, vault_client, vault_prefix).await
    }
}

/// Manifest provider factory.
pub struct ManifestProviderFactory;

impl ManifestProviderFactory {
    /// Create standard manifest provider.
    pub fn standard() -> Box<dyn ManifestProvider> {
        Box::new(StandardManifestProvider::new())
    }

    /// Create cached manifest provider.
    pub fn cached(cache_dir: impl AsRef<Path>) -> Box<dyn ManifestProvider> {
        Box::new(CachedManifestProvider::new(cache_dir))
    }

    /// Create custom provider with options.
    pub fn custom(spruce: Spruce, cache: Option<ManifestCache>) -> Box<dyn ManifestProvider> {
        let mut provider = StandardManifestProvider::new().with_spruce(spruce);

        if let Some(cache_instance) = cache {
            Box::new(CachedManifestProvider::with_provider_and_cache(
                provider,
                cache_instance,
            ))
        } else {
            Box::new(provider)
        }
    }
}
