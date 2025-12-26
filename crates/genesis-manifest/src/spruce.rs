//! Spruce integration for YAML merging and evaluation.

use genesis_types::{GenesisError, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Spruce command executor.
pub struct Spruce {
    binary_path: PathBuf,
    skip_eval: bool,
    prune_paths: Vec<String>,
    cherry_pick_paths: Vec<String>,
    env_vars: HashMap<String, String>,
}

impl Spruce {
    /// Create new Spruce executor with default binary path.
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("spruce"),
            skip_eval: false,
            prune_paths: Vec::new(),
            cherry_pick_paths: Vec::new(),
            env_vars: HashMap::new(),
        }
    }

    /// Set custom spruce binary path.
    pub fn with_binary(mut self, path: impl AsRef<Path>) -> Self {
        self.binary_path = path.as_ref().to_path_buf();
        self
    }

    /// Skip evaluation of Spruce operators.
    pub fn skip_eval(mut self, skip: bool) -> Self {
        self.skip_eval = skip;
        self
    }

    /// Add paths to prune from result.
    pub fn prune(mut self, paths: Vec<String>) -> Self {
        self.prune_paths = paths;
        self
    }

    /// Cherry-pick specific paths from result.
    pub fn cherry_pick(mut self, paths: Vec<String>) -> Self {
        self.cherry_pick_paths = paths;
        self
    }

    /// Add environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Check if spruce binary is available.
    pub fn check_available(&self) -> Result<bool> {
        match Command::new(&self.binary_path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) => Ok(status.success()),
            Err(_) => Ok(false),
        }
    }

    /// Get spruce version.
    pub fn version(&self) -> Result<String> {
        let output = Command::new(&self.binary_path)
            .arg("--version")
            .output()
            .map_err(|e| GenesisError::Manifest(format!("Failed to run spruce: {}", e)))?;

        if !output.status.success() {
            return Err(GenesisError::Manifest("Failed to get spruce version".to_string()));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }

    /// Merge multiple YAML files.
    pub fn merge(&self, files: &[impl AsRef<Path>]) -> Result<String> {
        if files.is_empty() {
            return Err(GenesisError::Manifest("No files to merge".to_string()));
        }

        debug!("Merging {} files with spruce", files.len());
        for (i, file) in files.iter().enumerate() {
            trace!("  [{}] {:?}", i, file.as_ref());
        }

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("merge");

        if self.skip_eval {
            cmd.arg("--skip-eval");
        }

        for path in &self.prune_paths {
            cmd.arg("--prune").arg(path);
        }

        for path in &self.cherry_pick_paths {
            cmd.arg("--cherry-pick").arg(path);
        }

        for file in files {
            cmd.arg(file.as_ref());
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let output = cmd
            .output()
            .map_err(|e| GenesisError::Manifest(format!("Failed to run spruce merge: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GenesisError::Manifest(format!(
                "Spruce merge failed:\n{}",
                stderr
            )));
        }

        let merged = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("Spruce merge produced {} bytes", merged.len());

        Ok(merged)
    }

    /// Merge YAML content from strings.
    pub fn merge_content(&self, contents: &[String]) -> Result<String> {
        use std::io::Write;

        if contents.is_empty() {
            return Err(GenesisError::Manifest("No content to merge".to_string()));
        }

        debug!("Merging {} content strings with spruce", contents.len());

        let temp_dir = tempfile::tempdir()
            .map_err(|e| GenesisError::Manifest(format!("Failed to create temp dir: {}", e)))?;

        let mut temp_files = Vec::new();
        for (i, content) in contents.iter().enumerate() {
            let temp_file = temp_dir.path().join(format!("merge-{}.yml", i));
            let mut file = std::fs::File::create(&temp_file)
                .map_err(|e| GenesisError::Manifest(format!("Failed to create temp file: {}", e)))?;

            file.write_all(content.as_bytes())
                .map_err(|e| GenesisError::Manifest(format!("Failed to write temp file: {}", e)))?;

            temp_files.push(temp_file);
        }

        self.merge(&temp_files)
    }

    /// Evaluate a single YAML file (resolve all Spruce operators).
    pub fn eval(&self, file: impl AsRef<Path>) -> Result<String> {
        debug!("Evaluating {:?} with spruce", file.as_ref());

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("merge").arg(file.as_ref());

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let output = cmd
            .output()
            .map_err(|e| GenesisError::Manifest(format!("Failed to run spruce eval: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GenesisError::Manifest(format!(
                "Spruce eval failed:\n{}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Extract specific path from YAML.
    pub fn json(&self, yaml: &str, path: &str) -> Result<String> {
        use std::io::Write;

        debug!("Extracting path '{}' from YAML", path);

        let temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| GenesisError::Manifest(format!("Failed to create temp file: {}", e)))?;

        temp_file.as_file()
            .write_all(yaml.as_bytes())
            .map_err(|e| GenesisError::Manifest(format!("Failed to write temp file: {}", e)))?;

        let output = Command::new(&self.binary_path)
            .arg("json")
            .arg(temp_file.path())
            .arg(path)
            .output()
            .map_err(|e| GenesisError::Manifest(format!("Failed to run spruce json: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GenesisError::Manifest(format!(
                "Spruce json failed:\n{}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Diff two YAML files.
    pub fn diff(&self, file1: impl AsRef<Path>, file2: impl AsRef<Path>) -> Result<String> {
        debug!("Diffing {:?} and {:?}", file1.as_ref(), file2.as_ref());

        let output = Command::new(&self.binary_path)
            .arg("diff")
            .arg(file1.as_ref())
            .arg(file2.as_ref())
            .output()
            .map_err(|e| GenesisError::Manifest(format!("Failed to run spruce diff: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Vault-ify a manifest (replace secrets with Vault paths).
    pub fn vaultify(&self, yaml: &str, vault_prefix: &str) -> Result<String> {
        use std::io::Write;

        debug!("Vaultifying YAML with prefix '{}'", vault_prefix);

        let temp_file = tempfile::NamedTempFile::new()
            .map_err(|e| GenesisError::Manifest(format!("Failed to create temp file: {}", e)))?;

        temp_file.as_file()
            .write_all(yaml.as_bytes())
            .map_err(|e| GenesisError::Manifest(format!("Failed to write temp file: {}", e)))?;

        let mut cmd = Command::new(&self.binary_path);
        cmd.arg("merge")
            .arg("--skip-eval")
            .arg(temp_file.path())
            .env("VAULT_PREFIX", vault_prefix);

        let output = cmd
            .output()
            .map_err(|e| GenesisError::Manifest(format!("Failed to run spruce vaultify: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GenesisError::Manifest(format!(
                "Spruce vaultify failed:\n{}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Extract all Vault paths from YAML.
    pub fn extract_vault_paths(&self, yaml: &str) -> Result<Vec<String>> {
        let vault_pattern = regex::Regex::new(r"\(\(vault\s+([^\)]+)\)\)")
            .map_err(|e| GenesisError::Manifest(format!("Invalid regex: {}", e)))?;

        let mut paths = Vec::new();
        for cap in vault_pattern.captures_iter(yaml) {
            if let Some(path) = cap.get(1) {
                paths.push(path.as_str().trim().to_string());
            }
        }

        Ok(paths)
    }

    /// Redact secrets in YAML (replace with REDACTED).
    pub fn redact(&self, yaml: &str, secret_paths: &[String]) -> Result<String> {
        let mut redacted = yaml.to_string();

        let secret_pattern = regex::Regex::new(r"(?m)^(\s*)([^:\s]+):\s*(.+)$")
            .map_err(|e| GenesisError::Manifest(format!("Invalid regex: {}", e)))?;

        for secret_path in secret_paths {
            let path_parts: Vec<&str> = secret_path.split('.').collect();
            if let Some(key) = path_parts.last() {
                redacted = secret_pattern.replace_all(
                    &redacted,
                    |caps: &regex::Captures| {
                        let indent = &caps[1];
                        let field_key = &caps[2];

                        if field_key == *key {
                            format!("{}{}:  REDACTED", indent, field_key)
                        } else {
                            caps[0].to_string()
                        }
                    }
                ).to_string();
            }
        }

        Ok(redacted)
    }
}

impl Default for Spruce {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vault_paths() {
        let spruce = Spruce::new();

        let yaml = r#"
properties:
  password: ((vault "secret/data/cf/admin:password"))
  cert: ((vault "secret/data/cf/ssl:certificate"))
  key: ((vault "secret/data/cf/ssl:private_key"))
"#;

        let paths = spruce.extract_vault_paths(yaml).unwrap();
        assert_eq!(paths.len(), 3);
        assert!(paths.contains(&"secret/data/cf/admin:password".to_string()));
        assert!(paths.contains(&"secret/data/cf/ssl:certificate".to_string()));
        assert!(paths.contains(&"secret/data/cf/ssl:private_key".to_string()));
    }

    #[test]
    fn test_spruce_builder() {
        let spruce = Spruce::new()
            .skip_eval(true)
            .prune(vec!["meta".to_string()])
            .cherry_pick(vec!["properties".to_string()])
            .with_env("VAULT_PREFIX", "secret/data");

        assert!(spruce.skip_eval);
        assert_eq!(spruce.prune_paths, vec!["meta"]);
        assert_eq!(spruce.cherry_pick_paths, vec!["properties"]);
        assert_eq!(spruce.env_vars.get("VAULT_PREFIX"), Some(&"secret/data".to_string()));
    }
}
