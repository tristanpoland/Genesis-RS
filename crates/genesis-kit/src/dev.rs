//! Development kit implementation (directory-based).

use super::{Kit, KitMetadata, HookResult, Blueprint};
use genesis_types::{GenesisError, Result, KitId, SemVer, HookType};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Development kit (directory-based, not compiled).
pub struct DevKit {
    id: KitId,
    path: PathBuf,
    metadata: KitMetadata,
}

impl DevKit {
    /// Load a development kit from directory.
    pub fn from_directory(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        if !path.is_dir() {
            return Err(GenesisError::Kit(format!(
                "Dev kit path is not a directory: {:?}",
                path
            )));
        }

        let metadata_path = path.join("kit.yml");
        let metadata = KitMetadata::load(&metadata_path)?;
        metadata.validate()?;

        let version = SemVer::parse(&metadata.version)?;
        let id = KitId {
            name: metadata.name.clone(),
            version,
        };

        Ok(Self {
            id,
            path: path.to_path_buf(),
            metadata,
        })
    }

    fn find_hook_file(&self, hook_type: HookType) -> Option<PathBuf> {
        let hook_name = format!("{}", hook_type);
        let hooks_dir = self.path.join("hooks");

        if !hooks_dir.exists() {
            return None;
        }

        for ext in &["", ".sh", ".bash"] {
            let path = hooks_dir.join(format!("{}{}", hook_name, ext));
            if path.exists() && path.is_file() {
                return Some(path);
            }
        }

        None
    }
}

impl Kit for DevKit {
    fn id(&self) -> &KitId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.metadata.name
    }

    fn version(&self) -> &SemVer {
        &self.id.version
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }

    fn metadata(&self) -> &KitMetadata {
        &self.metadata
    }

    fn has_hook(&self, hook_type: HookType) -> bool {
        self.find_hook_file(hook_type).is_some()
    }

    fn execute_hook(
        &self,
        hook_type: HookType,
        env_vars: HashMap<String, String>,
    ) -> Result<HookResult> {
        let hook_file = self.find_hook_file(hook_type)
            .ok_or_else(|| GenesisError::Hook(format!(
                "Hook '{}' not found in dev kit {}",
                hook_type, self.id
            )))?;

        use std::process::Command;

        let mut cmd = Command::new("bash");
        cmd.arg(hook_file);

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        cmd.env("GENESIS_KIT_NAME", &self.metadata.name);
        cmd.env("GENESIS_KIT_VERSION", self.metadata.version.to_string());
        cmd.env("GENESIS_KIT_PATH", self.path.to_string_lossy().to_string());
        cmd.env("GENESIS_KIT_DEV_MODE", "true");

        let output = cmd.output()
            .map_err(|e| GenesisError::Hook(format!("Failed to execute hook: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(HookResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            success: output.status.success(),
        })
    }

    fn blueprint(&self, features: &[String]) -> Result<Blueprint> {
        self.metadata.validate_features(features)?;
        Blueprint::generate(self, features)
    }

    fn check_prereqs(&self) -> Result<bool> {
        Ok(true)
    }
}
