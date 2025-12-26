//! RSA key secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use std::collections::HashMap;

/// RSA key secret.
#[derive(Debug, Clone)]
pub struct RsaSecret {
    path: String,
    key_size: u32,
}

impl RsaSecret {
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

impl Secret for RsaSecret {
    fn secret_type(&self) -> SecretType {
        SecretType::RSA
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        if self.key_size < 2048 {
            return Err(GenesisError::Secret("RSA key size must be at least 2048 bits".to_string()));
        }
        Ok(())
    }

    fn generate(&self) -> Result<HashMap<String, String>> {
        let rsa = Rsa::generate(self.key_size)
            .map_err(|e| GenesisError::Secret(format!("Failed to generate RSA key: {}", e)))?;

        let private_key = PKey::from_rsa(rsa)
            .map_err(|e| GenesisError::Secret(format!("Failed to create private key: {}", e)))?;

        let private_pem = private_key.private_key_to_pem_pkcs8()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode private key: {}", e)))?;

        let public_pem = private_key.public_key_to_pem()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode public key: {}", e)))?;

        let mut result = HashMap::new();
        result.insert("private".to_string(), String::from_utf8_lossy(&private_pem).to_string());
        result.insert("public".to_string(), String::from_utf8_lossy(&public_pem).to_string());

        Ok(result)
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        if !value.contains_key("private") || !value.contains_key("public") {
            return Ok(ValidationResult::Missing);
        }

        let private_pem = value.get("private").unwrap();
        match PKey::private_key_from_pem(private_pem.as_bytes()) {
            Ok(_) => Ok(ValidationResult::Ok),
            Err(e) => Ok(ValidationResult::Error(vec![
                format!("Invalid RSA private key: {}", e)
            ])),
        }
    }

    fn required_keys(&self) -> &[&str] {
        &["private", "public"]
    }
}
