//! User-provided secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use std::collections::HashMap;

/// User-provided secret (manual entry).
#[derive(Debug, Clone)]
pub struct UserProvidedSecret {
    path: String,
    prompt: String,
    keys: Vec<String>,
}

impl UserProvidedSecret {
    /// Create from definition hashmap.
    pub fn from_definition(path: String, mut def: HashMap<String, serde_json::Value>) -> Result<Self> {
        let prompt = def.remove("prompt")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("Enter value for {}", path));

        let keys = def.remove("keys")
            .and_then(|v| {
                if let Some(arr) = v.as_array() {
                    Some(arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| vec!["value".to_string()]);

        Ok(Self {
            path,
            prompt,
            keys,
        })
    }
}

impl Secret for UserProvidedSecret {
    fn secret_type(&self) -> SecretType {
        SecretType::UserProvided
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        if self.keys.is_empty() {
            return Err(GenesisError::Secret("User-provided secret must have at least one key".to_string()));
        }
        Ok(())
    }

    fn generate(&self) -> Result<HashMap<String, String>> {
        Err(GenesisError::Secret(
            "User-provided secrets cannot be auto-generated - requires manual input".to_string()
        ))
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        for key in &self.keys {
            if !value.contains_key(key) {
                return Ok(ValidationResult::Missing);
            }
        }
        Ok(ValidationResult::Ok)
    }

    fn required_keys(&self) -> &[&str] {
        self.keys.iter().map(|s| s.as_str()).collect::<Vec<_>>().leak()
    }
}
