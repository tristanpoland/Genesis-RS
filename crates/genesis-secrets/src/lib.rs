//! # Genesis Secrets
//!
//! Complete secret generation, validation, and management system.
//!
//! Provides implementations for all Genesis secret types:
//! - X509 certificates (CA, self-signed, signed)
//! - SSH keys
//! - RSA keys
//! - DH parameters
//! - Random passwords
//! - UUIDs
//! - User-provided secrets

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod types;
pub mod plan;
pub mod parser;
pub mod generator;
pub mod validator;

pub use types::*;
pub use plan::SecretPlan;
pub use parser::{SecretParser, FromKit, FromManifest};
