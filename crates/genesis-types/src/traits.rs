//! Core trait definitions for Genesis abstractions.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::{SemVer, ManifestType, SecretType};
use crate::errors::Result;
use crate::config::ProviderConfig;

/// Trait for kit providers (GitHub, custom, etc.).
///
/// Implementers of this trait can fetch kits from various sources.
#[async_trait]
pub trait KitProvider: Send + Sync {
    /// Fetch a specific version of a kit.
    ///
    /// Returns the path to the extracted kit directory.
    async fn fetch(&self, name: &str, version: &SemVer) -> Result<PathBuf>;

    /// List all available versions of a kit.
    async fn versions(&self, name: &str) -> Result<Vec<SemVer>>;

    /// Get the provider configuration.
    fn config(&self) -> ProviderConfig;

    /// Get the provider type name (e.g., "github", "custom").
    fn provider_type(&self) -> &'static str;
}

/// Trait for Vault secret storage backends.
///
/// Implementers provide access to secret storage systems like HashiCorp Vault.
#[async_trait]
pub trait VaultStore: Send + Sync {
    /// Read a secret from the vault.
    ///
    /// Returns a map of key-value pairs for the secret.
    async fn read(&self, path: &str) -> Result<HashMap<String, String>>;

    /// Write a secret to the vault.
    async fn write(&self, path: &str, data: &HashMap<String, String>) -> Result<()>;

    /// Check if a secret path exists.
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Delete a secret.
    async fn delete(&self, path: &str) -> Result<()>;

    /// List all paths under a prefix.
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Get the base path for this vault store.
    fn base_path(&self) -> &str;

    /// Get the vault URL.
    fn url(&self) -> &str;

    /// Get the vault name/alias.
    fn name(&self) -> &str;
}

/// Validation result for secret values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Secret is valid
    Ok,
    /// Secret is missing
    Missing,
    /// Secret has warnings (e.g., expiring soon)
    Warning(Vec<String>),
    /// Secret is invalid
    Error(Vec<String>),
}

/// Trait for secret types.
///
/// Each secret type (X509, SSH, RSA, etc.) implements this trait.
pub trait Secret: Send + Sync {
    /// Get the secret type.
    fn secret_type(&self) -> SecretType;

    /// Get the secret path (relative to base).
    fn path(&self) -> &str;

    /// Validate the secret definition.
    fn validate_definition(&self) -> Result<()>;

    /// Generate a new secret value.
    ///
    /// Returns a map of key-value pairs (e.g., "certificate", "private", "ca").
    fn generate(&self) -> Result<HashMap<String, String>>;

    /// Validate an existing secret value.
    ///
    /// Checks format, expiration, key usage, etc.
    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult>;

    /// Get the required keys for this secret type.
    ///
    /// For example, X509 certificates require "certificate", "private", and possibly "ca".
    fn required_keys(&self) -> &[&str];

    /// Check if this secret has dependencies on other secrets.
    ///
    /// For example, signed certificates depend on their CA.
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Trait for manifest providers.
///
/// Different manifest types implement this trait to provide their specific
/// transformation logic.
pub trait ManifestProvider: Send + Sync {
    /// Get the manifest type.
    fn manifest_type(&self) -> ManifestType;

    /// Generate the manifest.
    fn generate(&self) -> Result<String>;

    /// Get a cached manifest if available.
    fn cached(&self) -> Option<String>;

    /// Clear the cache for this manifest.
    fn clear_cache(&mut self);
}
