//! Invalid secret type for error handling.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use std::collections::HashMap;

/// Invalid secret (used for error reporting).
#[derive(Debug, Clone)]
pub struct InvalidSecret {
    path: String,
    errors: Vec<String>,
}

impl InvalidSecret {
    /// Create new invalid secret.
    pub fn new(path: String, _definition: HashMap<String, serde_json::Value>) -> Self {
        Self {
            path,
            errors: vec!["Invalid secret definition".to_string()],
        }
    }

    /// Create with specific errors.
    pub fn with_errors(path: String, errors: Vec<String>) -> Self {
        Self { path, errors }
    }
}

impl Secret for InvalidSecret {
    fn secret_type(&self) -> SecretType {
        SecretType::Invalid
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        Err(GenesisError::Secret(format!(
            "Invalid secret definition: {}",
            self.errors.join(", ")
        )))
    }

    fn generate(&self) -> Result<HashMap<String, String>> {
        Err(GenesisError::Secret(format!(
            "Cannot generate invalid secret: {}",
            self.errors.join(", ")
        )))
    }

    fn validate_value(&self, _value: &HashMap<String, String>) -> Result<ValidationResult> {
        Ok(ValidationResult::Error(self.errors.clone()))
    }

    fn required_keys(&self) -> &[&str] {
        &[]
    }
}
