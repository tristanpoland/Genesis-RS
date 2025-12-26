//! Secret plan management and execution.

use genesis_types::{GenesisError, Result};
use genesis_types::traits::{Secret, ValidationResult, VaultStore};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Secret plan containing all secrets for an environment.
pub struct SecretPlan {
    secrets: Vec<Box<dyn Secret>>,
    store: Arc<RwLock<Box<dyn VaultStore>>>,
    base_path: String,
}

impl SecretPlan {
    /// Create a new secret plan.
    pub fn new(store: Box<dyn VaultStore>, base_path: String) -> Self {
        Self {
            secrets: Vec::new(),
            store: Arc::new(RwLock::new(store)),
            base_path,
        }
    }

    /// Add a secret to the plan.
    pub fn add_secret(&mut self, secret: Box<dyn Secret>) {
        self.secrets.push(secret);
    }

    /// Sort secrets by dependencies (topological sort).
    pub fn sort_by_dependencies(&mut self) -> Result<()> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn visit(
            secret_path: &str,
            secrets: &[Box<dyn Secret>],
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
            sorted: &mut Vec<usize>,
        ) -> Result<()> {
            if visited.contains(secret_path) {
                return Ok(());
            }

            if visiting.contains(secret_path) {
                return Err(GenesisError::Secret(format!(
                    "Circular dependency detected involving secret: {}",
                    secret_path
                )));
            }

            visiting.insert(secret_path.to_string());

            let idx = secrets.iter()
                .position(|s| s.path() == secret_path)
                .ok_or_else(|| GenesisError::Secret(format!(
                    "Secret not found: {}",
                    secret_path
                )))?;

            for dep in &secrets[idx].dependencies() {
                visit(dep, secrets, visited, visiting, sorted)?;
            }

            visiting.remove(secret_path);
            visited.insert(secret_path.to_string());
            sorted.push(idx);

            Ok(())
        }

        for i in 0..self.secrets.len() {
            let path = self.secrets[i].path().to_string();
            visit(&path, &self.secrets, &mut visited, &mut visiting, &mut sorted)?;
        }

        let mut new_secrets = Vec::new();
        for idx in sorted {
            new_secrets.push(self.secrets.swap_remove(idx));
        }
        self.secrets = new_secrets;

        Ok(())
    }

    /// Check which secrets exist.
    pub async fn check(&self) -> Result<HashMap<String, bool>> {
        let mut results = HashMap::new();
        let store = self.store.read().await;

        for secret in &self.secrets {
            let full_path = format!("{}{}", self.base_path, secret.path());
            let exists = store.exists(&full_path).await?;
            results.insert(secret.path().to_string(), exists);
        }

        Ok(results)
    }

    /// Validate all secrets.
    pub async fn validate(&self) -> Result<HashMap<String, ValidationResult>> {
        let mut results = HashMap::new();
        let store = self.store.read().await;

        for secret in &self.secrets {
            let full_path = format!("{}{}", self.base_path, secret.path());

            let validation = match store.read(&full_path).await {
                Ok(value) => secret.validate_value(&value)?,
                Err(_) => ValidationResult::Missing,
            };

            results.insert(secret.path().to_string(), validation);
        }

        Ok(results)
    }

    /// Generate missing secrets.
    pub async fn generate_missing(&self) -> Result<Vec<String>> {
        let mut generated = Vec::new();
        let store = self.store.write().await;

        for secret in &self.secrets {
            let full_path = format!("{}{}", self.base_path, secret.path());

            if !store.exists(&full_path).await? {
                tracing::info!("Generating secret: {}", secret.path());

                let value = secret.generate()?;
                store.write(&full_path, &value).await?;

                generated.push(secret.path().to_string());
            }
        }

        Ok(generated)
    }

    /// Rotate specific secrets.
    pub async fn rotate(&self, paths: &[String]) -> Result<Vec<String>> {
        let mut rotated = Vec::new();
        let store = self.store.write().await;

        for secret in &self.secrets {
            if paths.contains(&secret.path().to_string()) {
                tracing::info!("Rotating secret: {}", secret.path());

                let full_path = format!("{}{}", self.base_path, secret.path());
                let value = secret.generate()?;
                store.write(&full_path, &value).await?;

                rotated.push(secret.path().to_string());
            }
        }

        Ok(rotated)
    }

    /// Remove secrets.
    pub async fn remove(&self, paths: &[String]) -> Result<Vec<String>> {
        let mut removed = Vec::new();
        let store = self.store.write().await;

        for secret in &self.secrets {
            if paths.contains(&secret.path().to_string()) {
                tracing::info!("Removing secret: {}", secret.path());

                let full_path = format!("{}{}", self.base_path, secret.path());
                store.delete(&full_path).await?;

                removed.push(secret.path().to_string());
            }
        }

        Ok(removed)
    }

    /// Get all secret paths.
    pub fn paths(&self) -> Vec<String> {
        self.secrets.iter().map(|s| s.path().to_string()).collect()
    }

    /// Get count of secrets.
    pub fn count(&self) -> usize {
        self.secrets.len()
    }
}
