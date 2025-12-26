//! # Genesis Kit
//!
//! Complete kit handling system including:
//! - Kit extraction from tarballs
//! - Dev kit support
//! - Hook discovery and execution
//! - Blueprint processing
//! - Kit providers (GitHub, GenesisCommunity)

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod compiled;
pub mod dev;
pub mod provider;
pub mod hook;
pub mod metadata;
pub mod blueprint;

pub use compiled::CompiledKit;
pub use dev::DevKit;
pub use provider::{
    KitProvider as KitProviderTrait,
    GithubProvider,
    GenesisCommunityProvider,
    CustomProvider,
    ProviderFactory,
    ProviderChain,
};
pub use hook::{HookExecutor, HookResult};
pub use metadata::{KitMetadata, FeatureMetadata, ParamMetadata, ExodusMetadata, PrereqMetadata};
pub use blueprint::Blueprint;

use genesis_types::{GenesisError, Result, KitId};
use std::path::PathBuf;

/// Kit trait implemented by both Compiled and Dev kits.
pub trait Kit: Send + Sync {
    /// Get kit identifier.
    fn id(&self) -> &KitId;

    /// Get kit name.
    fn name(&self) -> &str;

    /// Get kit version.
    fn version(&self) -> &genesis_types::SemVer;

    /// Get path to extracted kit.
    fn path(&self) -> &PathBuf;

    /// Get kit metadata.
    fn metadata(&self) -> &KitMetadata;

    /// Check if kit has a specific hook.
    fn has_hook(&self, hook_type: genesis_types::HookType) -> bool;

    /// Execute a hook.
    fn execute_hook(
        &self,
        hook_type: genesis_types::HookType,
        env_vars: std::collections::HashMap<String, String>,
    ) -> Result<HookResult>;

    /// Get blueprint for features.
    fn blueprint(&self, features: &[String]) -> Result<Blueprint>;

    /// Validate kit prerequisites.
    fn check_prereqs(&self) -> Result<bool>;
}
