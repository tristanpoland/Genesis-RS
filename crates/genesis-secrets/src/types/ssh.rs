//! SSH key secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SSH key secret.
#[derive(Debug, Clone)]
pub struct SshSecret {
    path: String,
    key_size: u32,
    fixed_fingerprint: bool,
}

impl SshSecret {
    /// Create from definition hashmap.
    pub fn from_definition(path: String, mut def: HashMap<String, serde_json::Value>) -> Result<Self> {
        let key_size = def.remove("bits")
            .or_else(|| def.remove("key_size"))
            .and_then(|v| v.as_u64().map(|n| n as u32))
            .unwrap_or(2048);

        let fixed_fingerprint = def.remove("fixed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Self {
            path,
            key_size,
            fixed_fingerprint,
        })
    }
}

impl Secret for SshSecret {
    fn secret_type(&self) -> SecretType {
        SecretType::SSH
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        if self.key_size < 2048 {
            return Err(GenesisError::Secret("SSH key size must be at least 2048 bits".to_string()));
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

        let public_ssh = Self::convert_to_ssh_format(&private_key)?;

        let mut result = HashMap::new();
        result.insert("private".to_string(), String::from_utf8_lossy(&private_pem).to_string());
        result.insert("public".to_string(), public_ssh);
        result.insert("fingerprint".to_string(), Self::calculate_fingerprint(&private_key)?);

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
                format!("Invalid SSH private key: {}", e)
            ])),
        }
    }

    fn required_keys(&self) -> &[&str] {
        &["private", "public"]
    }
}

impl SshSecret {
    fn convert_to_ssh_format(key: &PKey<Private>) -> Result<String> {
        let rsa = key.rsa()
            .map_err(|e| GenesisError::Secret(format!("Failed to get RSA key: {}", e)))?;

        let e = rsa.e().to_vec();
        let n = rsa.n().to_vec();

        let mut buf = Vec::new();
        Self::write_ssh_string(&mut buf, b"ssh-rsa");
        Self::write_ssh_mpint(&mut buf, &e);
        Self::write_ssh_mpint(&mut buf, &n);

        Ok(format!("ssh-rsa {} genesis-generated", base64::encode(&buf)))
    }

    fn write_ssh_string(buf: &mut Vec<u8>, data: &[u8]) {
        buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
        buf.extend_from_slice(data);
    }

    fn write_ssh_mpint(buf: &mut Vec<u8>, data: &[u8]) {
        let mut trimmed = data;
        while trimmed.len() > 1 && trimmed[0] == 0 && (trimmed[1] & 0x80) == 0 {
            trimmed = &trimmed[1..];
        }

        if (trimmed[0] & 0x80) != 0 {
            buf.extend_from_slice(&((trimmed.len() + 1) as u32).to_be_bytes());
            buf.push(0);
            buf.extend_from_slice(trimmed);
        } else {
            Self::write_ssh_string(buf, trimmed);
        }
    }

    fn calculate_fingerprint(key: &PKey<Private>) -> Result<String> {
        use sha2::{Sha256, Digest};

        let public_pem = key.public_key_to_pem()
            .map_err(|e| GenesisError::Secret(format!("Failed to encode public key: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(&public_pem);
        let hash = hasher.finalize();

        Ok(format!("SHA256:{}", base64::encode(&hash)))
    }
}
