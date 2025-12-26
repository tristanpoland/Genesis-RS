//! Random password/string secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;
use std::collections::HashMap;

/// Random password/string secret.
#[derive(Debug, Clone)]
pub struct RandomSecret {
    path: String,
    length: Option<usize>,
    fixed_length: bool,
    format: RandomFormat,
    use_bcrypt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RandomFormat {
    Alphanumeric,
    Base64,
    Hex,
    Printable,
}

impl RandomSecret {
    /// Create from definition hashmap.
    pub fn from_definition(path: String, mut def: HashMap<String, serde_json::Value>) -> Result<Self> {
        let length = def.remove("length")
            .and_then(|v| v.as_u64().map(|n| n as usize));

        let fixed_length = def.remove("fixed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let format_str = def.remove("format")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "base64".to_string());

        let format = match format_str.as_str() {
            "base64" => RandomFormat::Base64,
            "hex" => RandomFormat::Hex,
            "alphanumeric" => RandomFormat::Alphanumeric,
            "printable" => RandomFormat::Printable,
            _ => return Err(GenesisError::Secret(format!("Invalid format: {}", format_str))),
        };

        let use_bcrypt = def.remove("bcrypt")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Self {
            path,
            length,
            fixed_length,
            format,
            use_bcrypt,
        })
    }

    fn generate_random_bytes(&self, length: usize) -> Vec<u8> {
        let mut rng = thread_rng();
        (0..length).map(|_| rng.gen()).collect()
    }

    fn generate_random_string(&self, length: usize) -> String {
        match self.format {
            RandomFormat::Alphanumeric => {
                thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(length)
                    .map(char::from)
                    .collect()
            }
            RandomFormat::Base64 => {
                let bytes = self.generate_random_bytes(length);
                base64::encode(&bytes)
            }
            RandomFormat::Hex => {
                let bytes = self.generate_random_bytes(length);
                hex::encode(&bytes)
            }
            RandomFormat::Printable => {
                let mut rng = thread_rng();
                (0..length)
                    .map(|_| {
                        let c = rng.gen_range(33..127) as u8;
                        c as char
                    })
                    .collect()
            }
        }
    }
}

impl Secret for RandomSecret {
    fn secret_type(&self) -> SecretType {
        SecretType::Random
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        if let Some(length) = self.length {
            if length == 0 {
                return Err(GenesisError::Secret("Length must be greater than 0".to_string()));
            }
        }
        Ok(())
    }

    fn generate(&self) -> Result<HashMap<String, String>> {
        let length = self.length.unwrap_or(64);
        let password = self.generate_random_string(length);

        let mut result = HashMap::new();
        result.insert("password".to_string(), password.clone());

        if self.use_bcrypt {
            let hashed = bcrypt::hash(&password, bcrypt::DEFAULT_COST)
                .map_err(|e| GenesisError::Secret(format!("Failed to bcrypt hash: {}", e)))?;
            result.insert("password_hash".to_string(), hashed);
        }

        Ok(result)
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        if !value.contains_key("password") {
            return Ok(ValidationResult::Missing);
        }

        let password = value.get("password").unwrap();

        if let Some(expected_length) = self.length {
            if self.fixed_length && password.len() != expected_length {
                return Ok(ValidationResult::Warning(vec![
                    format!("Password length is {} but expected {}", password.len(), expected_length)
                ]));
            }
        }

        Ok(ValidationResult::Ok)
    }

    fn required_keys(&self) -> &[&str] {
        &["password"]
    }
}
