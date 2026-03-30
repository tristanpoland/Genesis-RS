//! Diffie-Hellman parameters secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use std::collections::HashMap;

/// DH parameters secret.
#[derive(Debug, Clone)]
pub struct DhParamsSecret {
    path: String,
    key_size: u32,
}

impl DhParamsSecret {
    /// Create from definition hashmap.
    pub fn from_definition(path: String, mut def: HashMap<String, serde_json::Value>) -> Result<Self> {
        let key_size = def.remove("bits")
            .or_else(|| def.remove("key_size"))
            .and_then(|v| v.as_u64().map(|n| n as u32))
            .unwrap_or(2048);

        Ok(Self {
            path,
            key_size,
        })
    }
}

impl Secret for DhParamsSecret {
    fn secret_type(&self) -> SecretType {
        SecretType::DHParams
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        if self.key_size < 2048 {
            return Err(GenesisError::Secret("DH params size must be at least 2048 bits".to_string()));
        }
        Ok(())
    }

    fn generate(&self) -> Result<HashMap<String, String>> {
        Err(GenesisError::Secret(
            "DH parameter generation is not available in the no-OpenSSL build".to_string(),
        ))
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        if !value.contains_key("dhparam-pem") {
            return Ok(ValidationResult::Missing);
        }

        let pem = value.get("dhparam-pem").unwrap();
        if pem.contains("-----BEGIN DH PARAMETERS-----") {
            Ok(ValidationResult::Ok)
        } else {
            Ok(ValidationResult::Error(vec!["Invalid DH parameters: unsupported format".to_string()]))
        }
    }

    fn required_keys(&self) -> &[&str] {
        &["dhparam-pem"]
    }
}
