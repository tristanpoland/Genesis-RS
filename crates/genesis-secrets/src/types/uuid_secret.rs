//! UUID secret type implementation.

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use uuid::Uuid;
use std::collections::HashMap;

/// UUID secret.
#[derive(Debug, Clone)]
pub struct UuidSecret {
    path: String,
}

impl UuidSecret {
    /// Create from definition hashmap.
    pub fn from_definition(path: String, _def: HashMap<String, serde_json::Value>) -> Result<Self> {
        Ok(Self { path })
    }
}

impl Secret for UuidSecret {
    fn secret_type(&self) -> SecretType {
        SecretType::UUID
    }

    fn path(&self) -> &str {
        &self.path
    }

    fn validate_definition(&self) -> Result<()> {
        Ok(())
    }

    fn generate(&self) -> Result<HashMap<String, String>> {
        let uuid = Uuid::new_v4();
        let mut result = HashMap::new();
        result.insert("uuid".to_string(), uuid.to_string());
        Ok(result)
    }

    fn validate_value(&self, value: &HashMap<String, String>) -> Result<ValidationResult> {
        if !value.contains_key("uuid") {
            return Ok(ValidationResult::Missing);
        }

        let uuid_str = value.get("uuid").unwrap();
        match Uuid::parse_str(uuid_str) {
            Ok(_) => Ok(ValidationResult::Ok),
            Err(e) => Ok(ValidationResult::Error(vec![
                format!("Invalid UUID: {}", e)
            ])),
        }
    }

    fn required_keys(&self) -> &[&str] {
        &["uuid"]
    }
}
