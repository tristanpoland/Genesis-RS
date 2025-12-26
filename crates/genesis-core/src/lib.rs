//! # Genesis Core
//!
//! Core utilities, configuration management, logging, and common functionality
//! for the Genesis BOSH deployment tool.
//!
//! This crate provides:
//!
//! - **Configuration**: Multi-layer configuration system (global, repo, environment)
//! - **Logging**: Structured logging with multiple outputs and stack traces
//! - **Terminal**: Colored output, terminal detection, formatting
//! - **Process Execution**: Safe command execution with environment management
//! - **File Operations**: YAML/JSON handling, path utilities
//! - **Time Utilities**: Formatting, duration calculations
//! - **Data Structures**: Deep merging, flattening, priority merging
//!
//! ## Example
//!
//! ```rust
//! use genesis_core::{config::Config, log::Logger};
//!
//! // Initialize logging
//! Logger::init_default()?;
//!
//! // Load configuration
//! let config = Config::load("~/.genesis/config")?;
//!
//! // Use utilities
//! let yaml = genesis_core::util::load_yaml_file("environment.yml")?;
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod config;
pub mod log;
pub mod term;
pub mod util;
pub mod state;
pub mod time;

// Re-export commonly used items
pub use config::{Config, GlobalConfig, RepoConfig};
pub use genesis_types::{GenesisError, Result};

/// Genesis application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Genesis application name
pub const APP_NAME: &str = "genesis";
