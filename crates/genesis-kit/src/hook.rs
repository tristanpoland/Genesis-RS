//! Hook execution system.

use genesis_types::{GenesisError, Result, HookType};
use std::collections::HashMap;

/// Result from hook execution.
#[derive(Debug, Clone)]
pub struct HookResult {
    /// Exit code from hook
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Whether the hook succeeded
    pub success: bool,
}

impl HookResult {
    /// Check if hook succeeded.
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get hook output (stdout).
    pub fn output(&self) -> &str {
        &self.stdout
    }

    /// Get hook errors (stderr).
    pub fn errors(&self) -> &str {
        &self.stderr
    }
}

/// Hook executor for running kit hooks.
pub struct HookExecutor {
    env_vars: HashMap<String, String>,
}

impl HookExecutor {
    /// Create new hook executor.
    pub fn new() -> Self {
        Self {
            env_vars: HashMap::new(),
        }
    }

    /// Add environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Add multiple environment variables.
    pub fn with_env_map(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Execute a hook.
    pub fn execute(
        &self,
        kit: &dyn super::Kit,
        hook_type: HookType,
    ) -> Result<HookResult> {
        kit.execute_hook(hook_type, self.env_vars.clone())
    }
}

impl Default for HookExecutor {
    fn default() -> Self {
        Self::new()
    }
}
