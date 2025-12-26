//! Error types for Genesis operations.

use std::fmt;
use thiserror::Error;

/// The main error type for Genesis operations.
///
/// This enum covers all major error categories that can occur during
/// Genesis operations, from configuration errors to deployment failures.
#[derive(Error, Debug)]
pub enum GenesisError {
    /// Configuration-related error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Environment-related error
    #[error("Environment error: {0}")]
    Environment(String),

    /// Kit-related error
    #[error("Kit error: {0}")]
    Kit(String),

    /// Secret management error
    #[error("Secret error: {0}")]
    Secret(String),

    /// Vault operation error
    #[error("Vault error: {0}")]
    Vault(String),

    /// BOSH operation error
    #[error("BOSH error: {0}")]
    Bosh(String),

    /// Manifest generation/processing error
    #[error("Manifest error: {0}")]
    Manifest(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Hook execution error
    #[error("Hook execution error: {0}")]
    Hook(String),

    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Internal bug - should never happen in production
    #[error("Bug detected: {0}\n\nThis is an internal error. Please report this issue at:\nhttps://github.com/genesis-community/genesis-rs/issues")]
    Bug(String),

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

/// A specialized Result type for Genesis operations.
pub type Result<T> = std::result::Result<T, GenesisError>;

/// Helper macro to create and return a GenesisError::Bug
///
/// This should be used for conditions that should never occur
/// in normal operation and indicate a bug in Genesis itself.
///
/// # Example
///
/// ```ignore
/// if some_impossible_condition {
///     bug!("This should never happen: {:?}", condition);
/// }
/// ```
#[macro_export]
macro_rules! bug {
    ($msg:expr) => {
        return Err($crate::GenesisError::Bug($msg.to_string()))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::GenesisError::Bug(format!($fmt, $($arg)*)))
    };
}

/// Helper macro to bail out with a GenesisError
///
/// This is used for expected error conditions.
///
/// # Example
///
/// ```ignore
/// if !valid {
///     bail!("Invalid configuration: {}", reason);
/// }
/// ```
#[macro_export]
macro_rules! bail {
    ($variant:ident, $msg:expr) => {
        return Err($crate::GenesisError::$variant($msg.to_string()))
    };
    ($variant:ident, $fmt:expr, $($arg:tt)*) => {
        return Err($crate::GenesisError::$variant(format!($fmt, $($arg)*)))
    };
    ($msg:expr) => {
        return Err($crate::GenesisError::Other($msg.to_string()))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::GenesisError::Other(format!($fmt, $($arg)*)))
    };
}
