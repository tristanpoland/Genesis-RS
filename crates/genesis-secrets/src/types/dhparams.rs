//! Diffie-Hellman parameters secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use openssl::dh::Dh;
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
        tracing::info!("Generating DH parameters with {} bits (this may take a while)...", self.key_size);

        let dh = Dh::generate_params(self.key_size, 2)
            .map_err(|e| GenesisError::Secret(format!("Failed to generate DH params: {}", e)))?;

        let pem = dh.params_to_pem()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode DH params: {}", e)))?;

        let mut result = HashMap::new();
        result.insert("dhparam-pem".to_string(), String::from_utf8_lossy(&pem).to_string());

        Ok(result)
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        if !value.contains_key("dhparam-pem") {
            return Ok(ValidationResult::Missing);
        }

        let pem = value.get("dhparam-pem").unwrap();
        match Dh::params_from_pem(pem.as_bytes()) {
            Ok(_) => Ok(ValidationResult::Ok),
            Err(e) => Ok(ValidationResult::Error(vec![
                format!("Invalid DH parameters: {}", e)
            ])),
        }
    }

    fn required_keys(&self) -> &[&str] {
        &["dhparam-pem"]
    }
}
