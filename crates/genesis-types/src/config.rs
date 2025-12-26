//! Configuration types and structures.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Provider configuration structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ProviderConfig {
    /// GitHub provider configuration
    Github {
        /// GitHub organization or user
        org: String,
        /// Optional custom GitHub domain (for Enterprise)
        #[serde(skip_serializing_if = "Option::is_none")]
        domain: Option<String>,
        /// Optional personal access token for rate limiting
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    /// Genesis Community provider (default)
    GenesisCommunity,
    /// Custom provider with explicit URL
    Custom {
        /// Base URL for kit downloads
        url: String,
    },
}

/// Secrets provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsProviderConfig {
    /// Vault URL
    pub url: String,
    /// Whether to skip TLS verification (insecure)
    #[serde(default)]
    pub insecure: bool,
    /// Vault namespace (for enterprise Vault)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Whether to use strongbox mode
    #[serde(default = "default_strongbox")]
    pub strongbox: bool,
    /// Vault target alias
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

fn default_strongbox() -> bool {
    true
}

/// Deployment root configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRoot {
    /// Label for this deployment root
    pub label: String,
    /// Path to the deployment directory
    pub path: PathBuf,
}

/// Log configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log file path template (supports datetime substitution)
    pub path: String,
    /// Log level for this output
    pub level: crate::LogLevel,
    /// Whether to include stack traces
    #[serde(default)]
    pub stack: bool,
    /// Log format (pretty, json, compact)
    #[serde(default = "default_log_format")]
    pub format: LogFormat,
}

fn default_log_format() -> LogFormat {
    LogFormat::Pretty
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// Human-readable format with colors
    Pretty,
    /// JSON format for machine parsing
    Json,
    /// Compact single-line format
    Compact,
}
