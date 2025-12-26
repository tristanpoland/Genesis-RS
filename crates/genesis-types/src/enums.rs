//! Common enumerations used throughout Genesis.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use crate::errors::{GenesisError, Result};

/// Log level enumeration for the logging system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    /// No logging
    None,
    /// Error messages only
    Error,
    /// Warnings and errors
    Warn,
    /// Informational messages
    Info,
    /// Debug messages
    Debug,
    /// Detailed trace messages
    Trace,
}

impl FromStr for LogLevel {
    type Err = GenesisError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "NONE" => Ok(LogLevel::None),
            "ERROR" => Ok(LogLevel::Error),
            "WARN" | "WARNING" => Ok(LogLevel::Warn),
            "INFO" => Ok(LogLevel::Info),
            "DEBUG" => Ok(LogLevel::Debug),
            "TRACE" => Ok(LogLevel::Trace),
            _ => Err(GenesisError::Validation(format!("Invalid log level: {}", s))),
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::None => write!(f, "NONE"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

/// Hook types that kits can implement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookType {
    /// New environment creation hook
    New,
    /// Feature validation hook
    Features,
    /// Blueprint generation hook (manifest file selection)
    Blueprint,
    /// Environment information display hook
    Info,
    /// Pre-deployment validation hook
    Check,
    /// Pre-deployment actions hook
    PreDeploy,
    /// Post-deployment actions hook
    PostDeploy,
    /// Environment termination hook
    Terminate,
    /// Custom addon script hook
    Addon,
    /// Cloud config generation hook
    CloudConfig,
    /// Runtime config generation hook
    RuntimeConfig,
    /// CPI config generation hook
    CpiConfig,
    /// Environment edit hook
    Edit,
    /// Interactive shell hook
    Shell,
}

impl fmt::Display for HookType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookType::New => write!(f, "new"),
            HookType::Features => write!(f, "features"),
            HookType::Blueprint => write!(f, "blueprint"),
            HookType::Info => write!(f, "info"),
            HookType::Check => write!(f, "check"),
            HookType::PreDeploy => write!(f, "pre-deploy"),
            HookType::PostDeploy => write!(f, "post-deploy"),
            HookType::Terminate => write!(f, "terminate"),
            HookType::Addon => write!(f, "addon"),
            HookType::CloudConfig => write!(f, "cloud-config"),
            HookType::RuntimeConfig => write!(f, "runtime-config"),
            HookType::CpiConfig => write!(f, "cpi-config"),
            HookType::Edit => write!(f, "edit"),
            HookType::Shell => write!(f, "shell"),
        }
    }
}

/// Manifest type variants representing different transformation stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestType {
    /// Raw YAML without operator evaluation
    Unevaluated,
    /// Just environment files without kit
    UnevaluatedEnvironment,
    /// Environment files with kit base (no full evaluation)
    PartialEnvironment,
    /// Full merge without evaluation
    Partial,
    /// Full manifest with secrets resolved from Vault
    Unredacted,
    /// Secrets replaced with vault path references
    Redacted,
    /// With CredHub variable references added
    Vaultified,
    /// Redacted + CredHub refs
    VaultifiedRedacted,
    /// Secrets embedded directly in manifest
    Entombed,
    /// Entombed + CredHub refs
    VaultifiedEntombed,
}

/// Secret type enumeration for different kinds of secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecretType {
    /// X.509 certificates (CA, signed, self-signed)
    X509,
    /// SSH key pairs
    SSH,
    /// RSA key pairs
    RSA,
    /// Diffie-Hellman parameters
    DHParams,
    /// Random passwords/strings
    Random,
    /// UUID v4
    UUID,
    /// User-provided secret
    UserProvided,
    /// Invalid secret definition
    Invalid,
}

impl fmt::Display for SecretType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecretType::X509 => write!(f, "x509"),
            SecretType::SSH => write!(f, "ssh"),
            SecretType::RSA => write!(f, "rsa"),
            SecretType::DHParams => write!(f, "dhparams"),
            SecretType::Random => write!(f, "random"),
            SecretType::UUID => write!(f, "uuid"),
            SecretType::UserProvided => write!(f, "user-provided"),
            SecretType::Invalid => write!(f, "invalid"),
        }
    }
}
