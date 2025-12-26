//! Compiled kit implementation (tarball-based).

use super::{Kit, KitMetadata, HookResult, Blueprint};
use genesis_types::{GenesisError, Result, KitId, SemVer, HookType};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::collections::HashMap;
use tar::Archive;
use flate2::read::GzDecoder;
use sha2::{Sha256, Digest};

/// Compiled kit (extracted from tarball).
pub struct CompiledKit {
    id: KitId,
    path: PathBuf,
    metadata: KitMetadata,
    extracted_root: PathBuf,
}

impl CompiledKit {
    /// Load a compiled kit from tarball.
    pub fn from_tarball(
        tarball_path: impl AsRef<Path>,
        extract_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        let tarball_path = tarball_path.as_ref();
        let extract_dir = extract_dir.as_ref();

        tracing::info!("Extracting kit from: {:?}", tarball_path);

        let kit_hash = Self::calculate_hash(tarball_path)?;
        let extracted_root = extract_dir.join(&kit_hash);

        if !extracted_root.exists() {
            Self::extract_tarball(tarball_path, &extracted_root)?;
        }

        let metadata_path = extracted_root.join("kit.yml");
        let metadata = KitMetadata::load(&metadata_path)?;
        metadata.validate()?;

        let version = SemVer::parse(&metadata.version)?;
        let id = KitId {
            name: metadata.name.clone(),
            version: version.clone(),
        };

        Ok(Self {
            id,
            path: tarball_path.to_path_buf(),
            metadata,
            extracted_root,
        })
    }

    fn calculate_hash(path: &Path) -> Result<String> {
        let mut file = File::open(path)
            .map_err(|e| GenesisError::Kit(format!("Failed to open tarball: {}", e)))?;

        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)
            .map_err(|e| GenesisError::Kit(format!("Failed to hash tarball: {}", e)))?;

        Ok(hex::encode(hasher.finalize()))
    }

    fn extract_tarball(tarball: &Path, dest: &Path) -> Result<()> {
        std::fs::create_dir_all(dest)
            .map_err(|e| GenesisError::Kit(format!("Failed to create extract dir: {}", e)))?;

        let file = File::open(tarball)
            .map_err(|e| GenesisError::Kit(format!("Failed to open tarball: {}", e)))?;

        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        archive.unpack(dest)
            .map_err(|e| GenesisError::Kit(format!("Failed to extract tarball: {}", e)))?;

        Ok(())
    }

    fn find_hook_file(&self, hook_type: HookType) -> Option<PathBuf> {
        let hook_name = format!("{}", hook_type);
        let hooks_dir = self.extracted_root.join("hooks");

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

impl Kit for CompiledKit {
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
        &self.extracted_root
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
                "Hook '{}' not found in kit {}",
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
        cmd.env("GENESIS_KIT_PATH", self.extracted_root.to_string_lossy().to_string());

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
        use std::process::Command;

        for prereq in &self.metadata.prereqs {
            let output = Command::new("which")
                .arg(&prereq.binary)
                .output()
                .map_err(|e| GenesisError::Kit(format!(
                    "Failed to check prerequisite {}: {}",
                    prereq.binary, e
                )))?;

            if !output.status.success() {
                if prereq.required {
                    return Err(GenesisError::Kit(format!(
                        "Required prerequisite '{}' not found",
                        prereq.binary
                    )));
                } else {
                    tracing::warn!("Optional prerequisite '{}' not found", prereq.binary);
                }
            }

            if let Some(ref min_version) = prereq.version {
                let version_output = Command::new(&prereq.binary)
                    .arg("--version")
                    .output()
                    .ok();

                if let Some(output) = version_output {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    tracing::debug!("Prerequisite {} version: {}", prereq.binary, version_str);
                }
            }
        }

        Ok(true)
    }
}
