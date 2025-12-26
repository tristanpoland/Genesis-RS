//! # Genesis Services
//!
//! External service client implementations for Genesis.
//!
//! This crate provides async clients for:
//! - **Vault**: HashiCorp Vault secret storage
//! - **BOSH**: BOSH director operations
//! - **CredHub**: Cloud Foundry CredHub integration
//! - **GitHub**: GitHub API for kit downloads

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod vault;
pub mod bosh;
pub mod credhub;
pub mod github;

pub use vault::{VaultClient, VaultConfig};
pub use bosh::{BoshClient, BoshConfig};
pub use credhub::{CredhubClient, CredhubConfig};
pub use github::{GithubClient, GithubConfig};
