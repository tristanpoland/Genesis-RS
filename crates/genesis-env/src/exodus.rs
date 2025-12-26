//! Exodus data management for environment outputs.

use genesis_types::{GenesisError, Result, EnvName};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tracing::{debug, info};

/// Exodus data containing deployment outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExodusData {
    /// Environment name
    pub env_name: EnvName,

    /// Kit name that produced this data
    pub kit_name: String,

    /// Kit version
    pub kit_version: String,

    /// Exodus data values
    pub data: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
}

impl ExodusData {
    /// Create new exodus data.
    pub fn new(
        env_name: EnvName,
        kit_name: impl Into<String>,
        kit_version: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            env_name,
            kit_name: kit_name.into(),
            kit_version: kit_version.into(),
            data: HashMap::new(),
            created_at: now,
            modified_at: now,
        }
    }

    /// Set exodus value.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.data.insert(key.into(), value);
        self.modified_at = Utc::now();
    }

    /// Get exodus value.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Remove exodus value.
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        let result = self.data.remove(key);
        if result.is_some() {
            self.modified_at = Utc::now();
        }
        result
    }

    /// Check if key exists.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get all keys.
    pub fn keys(&self) -> Vec<&str> {
        self.data.keys().map(|s| s.as_str()).collect()
    }

    /// Get data count.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Merge with another exodus data.
    pub fn merge(&mut self, other: &ExodusData) {
        for (key, value) in &other.data {
            self.data.insert(key.clone(), value.clone());
        }
        self.modified_at = Utc::now();
    }

    /// Load from file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| GenesisError::Environment(format!("Failed to read exodus file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| GenesisError::Environment(format!("Failed to parse exodus data: {}", e)))
    }

    /// Save to file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| GenesisError::Environment(format!("Failed to serialize exodus data: {}", e)))?;

        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| GenesisError::Environment(format!("Failed to create exodus directory: {}", e)))?;
        }

        std::fs::write(path.as_ref(), content)
            .map_err(|e| GenesisError::Environment(format!("Failed to write exodus file: {}", e)))
    }
}

/// Exodus manager for handling exodus data operations.
pub struct ExodusManager {
    exodus_dir: PathBuf,
}

impl ExodusManager {
    /// Create new exodus manager.
    pub fn new(exodus_dir: impl AsRef<Path>) -> Self {
        Self {
            exodus_dir: exodus_dir.as_ref().to_path_buf(),
        }
    }

    /// Get exodus file path for an environment.
    fn exodus_path(&self, env_name: &EnvName) -> PathBuf {
        self.exodus_dir.join(format!("{}.json", env_name))
    }

    /// Load exodus data for an environment.
    pub fn load(&self, env_name: &EnvName) -> Result<Option<ExodusData>> {
        let path = self.exodus_path(env_name);

        if !path.exists() {
            debug!("No exodus data found for {}", env_name);
            return Ok(None);
        }

        let data = ExodusData::load(&path)?;
        info!("Loaded exodus data for {} with {} entries", env_name, data.len());
        Ok(Some(data))
    }

    /// Save exodus data for an environment.
    pub fn save(&self, data: &ExodusData) -> Result<()> {
        std::fs::create_dir_all(&self.exodus_dir)
            .map_err(|e| GenesisError::Environment(format!("Failed to create exodus directory: {}", e)))?;

        let path = self.exodus_path(&data.env_name);
        data.save(&path)?;

        info!("Saved exodus data for {} with {} entries", data.env_name, data.len());
        Ok(())
    }

    /// Delete exodus data for an environment.
    pub fn delete(&self, env_name: &EnvName) -> Result<()> {
        let path = self.exodus_path(env_name);

        if path.exists() {
            std::fs::remove_file(&path)
                .map_err(|e| GenesisError::Environment(format!("Failed to delete exodus file: {}", e)))?;
            info!("Deleted exodus data for {}", env_name);
        }

        Ok(())
    }

    /// List all environments with exodus data.
    pub fn list(&self) -> Result<Vec<EnvName>> {
        if !self.exodus_dir.exists() {
            return Ok(Vec::new());
        }

        let mut env_names = Vec::new();

        let entries = std::fs::read_dir(&self.exodus_dir)
            .map_err(|e| GenesisError::Environment(format!("Failed to read exodus directory: {}", e)))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(env_name) = EnvName::new(stem) {
                            env_names.push(env_name);
                        }
                    }
                }
            }
        }

        Ok(env_names)
    }

    /// Get exodus data for a specific key from an environment.
    pub fn get_value(&self, env_name: &EnvName, key: &str) -> Result<Option<serde_json::Value>> {
        if let Some(data) = self.load(env_name)? {
            Ok(data.get(key).cloned())
        } else {
            Ok(None)
        }
    }

    /// Set exodus value for an environment.
    pub fn set_value(&self, env_name: &EnvName, key: impl Into<String>, value: serde_json::Value) -> Result<()> {
        let mut data = self.load(env_name)?
            .unwrap_or_else(|| ExodusData::new(
                env_name.clone(),
                "unknown",
                "0.0.0",
            ));

        data.set(key, value);
        self.save(&data)
    }

    /// Remove exodus value from an environment.
    pub fn remove_value(&self, env_name: &EnvName, key: &str) -> Result<Option<serde_json::Value>> {
        if let Some(mut data) = self.load(env_name)? {
            let result = data.remove(key);
            self.save(&data)?;
            Ok(result)
        } else {
            Ok(None)
        }
    }

    /// Import exodus data from another environment.
    pub fn import(&self, from: &EnvName, to: &EnvName, keys: Option<Vec<String>>) -> Result<()> {
        let source_data = self.load(from)?
            .ok_or_else(|| GenesisError::Environment(format!("No exodus data found for {}", from)))?;

        let mut target_data = self.load(to)?
            .unwrap_or_else(|| ExodusData::new(
                to.clone(),
                source_data.kit_name.clone(),
                source_data.kit_version.clone(),
            ));

        if let Some(keys) = keys {
            for key in keys {
                if let Some(value) = source_data.get(&key) {
                    target_data.set(key, value.clone());
                }
            }
        } else {
            target_data.merge(&source_data);
        }

        self.save(&target_data)?;
        info!("Imported exodus data from {} to {}", from, to);

        Ok(())
    }

    /// Export exodus data to JSON file.
    pub fn export(&self, env_name: &EnvName, output_path: &Path) -> Result<()> {
        let data = self.load(env_name)?
            .ok_or_else(|| GenesisError::Environment(format!("No exodus data found for {}", env_name)))?;

        data.save(output_path)?;
        info!("Exported exodus data for {} to {:?}", env_name, output_path);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_exodus_data_creation() {
        let env_name = EnvName::new("test-env").unwrap();
        let mut data = ExodusData::new(env_name.clone(), "test-kit", "1.0.0");

        data.set("key1", serde_json::json!("value1"));
        data.set("key2", serde_json::json!(42));

        assert_eq!(data.get("key1"), Some(&serde_json::json!("value1")));
        assert_eq!(data.get("key2"), Some(&serde_json::json!(42)));
        assert_eq!(data.len(), 2);
    }

    #[test]
    fn test_exodus_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ExodusManager::new(temp_dir.path());

        let env_name = EnvName::new("test-env").unwrap();
        let mut data = ExodusData::new(env_name.clone(), "test-kit", "1.0.0");
        data.set("key1", serde_json::json!("value1"));

        manager.save(&data).unwrap();

        let loaded = manager.load(&env_name).unwrap().unwrap();
        assert_eq!(loaded.get("key1"), Some(&serde_json::json!("value1")));
    }

    #[test]
    fn test_exodus_merge() {
        let env_name = EnvName::new("test-env").unwrap();
        let mut data1 = ExodusData::new(env_name.clone(), "test-kit", "1.0.0");
        data1.set("key1", serde_json::json!("value1"));

        let mut data2 = ExodusData::new(env_name, "test-kit", "1.0.0");
        data2.set("key2", serde_json::json!("value2"));

        data1.merge(&data2);

        assert_eq!(data1.len(), 2);
        assert!(data1.contains_key("key1"));
        assert!(data1.contains_key("key2"));
    }
}
