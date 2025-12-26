//! # Genesis Types
//!
//! Core types, traits, and enums shared across all Genesis crates.
//!
//! This crate provides the fundamental building blocks for the Genesis BOSH
//! deployment tool, including:
//!
//! - Type-safe wrappers for environment names, versions, and identifiers
//! - Common enums for log levels, hook types, manifest types, and secret types
//! - Core traits for providers, stores, and validators
//! - Error types and result aliases
//!
//! ## Example
//!
//! ```
//! use genesis_types::{EnvName, SemVer, HookType};
//!
//! // Create a validated environment name
//! let env = EnvName::new("us-west-prod").unwrap();
//! assert_eq!(env.as_str(), "us-west-prod");
//!
//! // Parse a semantic version
//! let version = SemVer::parse("1.2.3").unwrap();
//! assert_eq!(version.major, 1);
//! assert_eq!(version.minor, 2);
//! assert_eq!(version.patch, 3);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod errors;
pub mod identifiers;
pub mod enums;
pub mod traits;
pub mod config;

// Re-export common types for convenience
pub use errors::{GenesisError, Result};
pub use identifiers::{EnvName, KitId, SemVer};
pub use enums::{LogLevel, HookType, ManifestType, SecretType};
pub use traits::{KitProvider, VaultStore, Secret, ManifestProvider};
