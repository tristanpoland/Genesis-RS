//! Secret type implementations.

pub mod x509;
pub mod ssh;
pub mod rsa;
pub mod dhparams;
pub mod random;
pub mod uuid_secret;
pub mod user_provided;
pub mod invalid;

pub use x509::X509Secret;
pub use ssh::SshSecret;
pub use rsa::RsaSecret;
pub use dhparams::DhParamsSecret;
pub use random::RandomSecret;
pub use uuid_secret::UuidSecret;
pub use user_provided::UserProvidedSecret;
pub use invalid::InvalidSecret;

use genesis_types::{GenesisError, Result, SecretType};
use genesis_types::traits::{Secret, ValidationResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Factory function to create secret from type and definition.
pub fn create_secret(
    secret_type: SecretType,
    path: String,
    definition: HashMap<String, serde_json::Value>,
) -> Result<Box<dyn Secret>> {
    match secret_type {
        SecretType::X509 => Ok(Box::new(X509Secret::from_definition(path, definition)?)),
        SecretType::SSH => Ok(Box::new(SshSecret::from_definition(path, definition)?)),
        SecretType::RSA => Ok(Box::new(RsaSecret::from_definition(path, definition)?)),
        SecretType::DHParams => Ok(Box::new(DhParamsSecret::from_definition(path, definition)?)),
        SecretType::Random => Ok(Box::new(RandomSecret::from_definition(path, definition)?)),
        SecretType::UUID => Ok(Box::new(UuidSecret::from_definition(path, definition)?)),
        SecretType::UserProvided => Ok(Box::new(UserProvidedSecret::from_definition(path, definition)?)),
        SecretType::Invalid => Ok(Box::new(InvalidSecret::new(path, definition))),
    }
}
