//! Manifest transformation operations.

use super::spruce::Spruce;
use super::types::{YamlContent, YamlValue, ManifestSubset, ManifestMetadata};
use genesis_types::{GenesisError, Result};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

/// Manifest transformer for applying operations to manifests.
pub struct ManifestTransformer {
    spruce: Spruce,
}

impl ManifestTransformer {
    /// Create new manifest transformer.
    pub fn new() -> Self {
        Self {
            spruce: Spruce::new(),
        }
    }

    /// Create with custom Spruce instance.
    pub fn with_spruce(spruce: Spruce) -> Self {
        Self { spruce }
    }

    /// Cherry-pick specific paths from a manifest.
    ///
    /// Extracts only the specified paths and their values from the manifest.
    pub fn cherry_pick(&self, yaml: &str, paths: &[String]) -> Result<String> {
        if paths.is_empty() {
            return Ok(yaml.to_string());
        }

        let parsed: YamlValue = serde_yaml::from_str(yaml)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))?;

        let mut result = JsonValue::Object(serde_json::Map::new());

        for path in paths {
            if let Some(value) = self.get_path(&parsed, path) {
                self.set_path(&mut result, path, value.clone())?;
            }
        }

        serde_yaml::to_string(&result)
            .map_err(|e| GenesisError::Manifest(format!("Failed to serialize YAML: {}", e)))
    }

    /// Prune specific paths from a manifest.
    ///
    /// Removes the specified paths and their values from the manifest.
    pub fn prune(&self, yaml: &str, paths: &[String]) -> Result<String> {
        if paths.is_empty() {
            return Ok(yaml.to_string());
        }

        let mut parsed: YamlValue = serde_yaml::from_str(yaml)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))?;

        for path in paths {
            self.delete_path(&mut parsed, path)?;
        }

        serde_yaml::to_string(&parsed)
            .map_err(|e| GenesisError::Manifest(format!("Failed to serialize YAML: {}", e)))
    }

    /// Fetch a specific value from a path in the manifest.
    pub fn fetch(&self, yaml: &str, path: &str) -> Result<YamlValue> {
        let parsed: YamlValue = serde_yaml::from_str(yaml)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))?;

        self.get_path(&parsed, path)
            .cloned()
            .ok_or_else(|| GenesisError::Manifest(format!("Path not found: {}", path)))
    }

    /// Redact secrets in manifest by replacing values with REDACTED.
    pub fn redact(&self, yaml: &str, secret_paths: &[String]) -> Result<String> {
        let mut parsed: YamlValue = serde_yaml::from_str(yaml)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))?;

        for path in secret_paths {
            if self.path_exists(&parsed, path) {
                let redacted = JsonValue::String("REDACTED".to_string());
                self.set_path(&mut parsed, path, redacted)?;
            }
        }

        serde_yaml::to_string(&parsed)
            .map_err(|e| GenesisError::Manifest(format!("Failed to serialize YAML: {}", e)))
    }

    /// Replace secret values with Vault path references.
    pub fn vaultify(&self, yaml: &str, vault_prefix: &str, secret_paths: &[String]) -> Result<(String, HashMap<String, String>)> {
        let mut parsed: YamlValue = serde_yaml::from_str(yaml)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))?;

        let mut vault_mappings = HashMap::new();

        for path in secret_paths {
            if self.path_exists(&parsed, path) {
                let vault_path = format!("{}/{}", vault_prefix, path.replace('.', "/"));
                let vault_ref = format!("((vault \"{}\"))", vault_path);

                vault_mappings.insert(path.clone(), vault_path);

                let vault_value = JsonValue::String(vault_ref);
                self.set_path(&mut parsed, path, vault_value)?;
            }
        }

        let vaultified = serde_yaml::to_string(&parsed)
            .map_err(|e| GenesisError::Manifest(format!("Failed to serialize YAML: {}", e)))?;

        Ok((vaultified, vault_mappings))
    }

    /// Get value at a dot-notation path.
    fn get_path(&self, value: &YamlValue, path: &str) -> Option<&YamlValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = value;

        for part in parts {
            match current {
                JsonValue::Object(map) => {
                    current = map.get(part)?;
                }
                JsonValue::Array(arr) => {
                    let index: usize = part.parse().ok()?;
                    current = arr.get(index)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Set value at a dot-notation path.
    fn set_path(&self, value: &mut YamlValue, path: &str, new_value: YamlValue) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Err(GenesisError::Manifest("Empty path".to_string()));
        }

        if parts.len() == 1 {
            if let JsonValue::Object(map) = value {
                map.insert(parts[0].to_string(), new_value);
                return Ok(());
            }
            return Err(GenesisError::Manifest("Root value is not an object".to_string()));
        }

        let mut current = value;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let JsonValue::Object(map) = current {
                    map.insert(part.to_string(), new_value);
                    return Ok(());
                }
                return Err(GenesisError::Manifest(format!("Cannot set value at path: {}", path)));
            }

            match current {
                JsonValue::Object(map) => {
                    if !map.contains_key(*part) {
                        map.insert(part.to_string(), JsonValue::Object(serde_json::Map::new()));
                    }
                    current = map.get_mut(*part).unwrap();
                }
                _ => return Err(GenesisError::Manifest(format!("Invalid path: {}", path))),
            }
        }

        Ok(())
    }

    /// Delete value at a dot-notation path.
    fn delete_path(&self, value: &mut YamlValue, path: &str) -> Result<()> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Ok(());
        }

        if parts.len() == 1 {
            if let JsonValue::Object(map) = value {
                map.remove(parts[0]);
            }
            return Ok(());
        }

        let mut current = value;
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                if let JsonValue::Object(map) = current {
                    map.remove(*part);
                }
                return Ok(());
            }

            match current {
                JsonValue::Object(map) => {
                    if let Some(next) = map.get_mut(*part) {
                        current = next;
                    } else {
                        return Ok(());
                    }
                }
                _ => return Ok(()),
            }
        }

        Ok(())
    }

    /// Check if a path exists in the value.
    fn path_exists(&self, value: &YamlValue, path: &str) -> bool {
        self.get_path(value, path).is_some()
    }

    /// Extract all paths from a YAML structure.
    pub fn extract_all_paths(&self, yaml: &str) -> Result<Vec<String>> {
        let parsed: YamlValue = serde_yaml::from_str(yaml)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse YAML: {}", e)))?;

        let mut paths = Vec::new();
        self.collect_paths(&parsed, String::new(), &mut paths);
        Ok(paths)
    }

    /// Recursively collect all paths.
    fn collect_paths(&self, value: &YamlValue, prefix: String, paths: &mut Vec<String>) {
        match value {
            JsonValue::Object(map) => {
                for (key, val) in map {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    paths.push(path.clone());
                    self.collect_paths(val, path, paths);
                }
            }
            JsonValue::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let path = format!("{}[{}]", prefix, i);
                    paths.push(path.clone());
                    self.collect_paths(val, path, paths);
                }
            }
            _ => {}
        }
    }

    /// Find all paths matching a pattern.
    pub fn find_paths(&self, yaml: &str, pattern: &str) -> Result<Vec<String>> {
        let all_paths = self.extract_all_paths(yaml)?;
        let regex = regex::Regex::new(pattern)
            .map_err(|e| GenesisError::Manifest(format!("Invalid pattern: {}", e)))?;

        Ok(all_paths.into_iter()
            .filter(|p| regex.is_match(p))
            .collect())
    }

    /// Create a subset of the manifest.
    pub fn create_subset(
        &self,
        yaml: &str,
        paths: &[String],
        metadata: ManifestMetadata,
    ) -> Result<ManifestSubset> {
        let subset_yaml = self.cherry_pick(yaml, paths)?;
        Ok(ManifestSubset::new(subset_yaml, metadata, paths.to_vec()))
    }

    /// Merge two manifests, with the second taking precedence.
    pub fn merge_two(&self, yaml1: &str, yaml2: &str) -> Result<String> {
        let mut val1: YamlValue = serde_yaml::from_str(yaml1)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse first YAML: {}", e)))?;

        let val2: YamlValue = serde_yaml::from_str(yaml2)
            .map_err(|e| GenesisError::Manifest(format!("Failed to parse second YAML: {}", e)))?;

        self.deep_merge(&mut val1, val2);

        serde_yaml::to_string(&val1)
            .map_err(|e| GenesisError::Manifest(format!("Failed to serialize merged YAML: {}", e)))
    }

    /// Deep merge two JSON values.
    fn deep_merge(&self, base: &mut YamlValue, overlay: YamlValue) {
        match (base, overlay) {
            (JsonValue::Object(base_map), JsonValue::Object(overlay_map)) => {
                for (key, overlay_val) in overlay_map {
                    if let Some(base_val) = base_map.get_mut(&key) {
                        self.deep_merge(base_val, overlay_val);
                    } else {
                        base_map.insert(key, overlay_val);
                    }
                }
            }
            (base_val, overlay_val) => {
                *base_val = overlay_val;
            }
        }
    }

    /// Extract secret paths from manifest (paths that likely contain secrets).
    pub fn extract_secret_paths(&self, yaml: &str) -> Result<Vec<String>> {
        let all_paths = self.extract_all_paths(yaml)?;

        let secret_keywords = vec![
            "password", "passwd", "secret", "key", "private", "token",
            "certificate", "cert", "ca", "ssl", "tls", "auth",
        ];

        let secret_paths: Vec<String> = all_paths
            .into_iter()
            .filter(|path| {
                let path_lower = path.to_lowercase();
                secret_keywords.iter().any(|kw| path_lower.contains(kw))
            })
            .collect();

        Ok(secret_paths)
    }
}

impl Default for ManifestTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cherry_pick() {
        let transformer = ManifestTransformer::new();

        let yaml = r#"
properties:
  username: admin
  password: secret
meta:
  environment: prod
"#;

        let result = transformer.cherry_pick(yaml, &vec!["properties.username".to_string()]).unwrap();
        assert!(result.contains("username"));
        assert!(!result.contains("password"));
        assert!(!result.contains("meta"));
    }

    #[test]
    fn test_prune() {
        let transformer = ManifestTransformer::new();

        let yaml = r#"
properties:
  username: admin
  password: secret
meta:
  environment: prod
"#;

        let result = transformer.prune(yaml, &vec!["properties.password".to_string(), "meta".to_string()]).unwrap();
        assert!(result.contains("username"));
        assert!(!result.contains("password"));
        assert!(!result.contains("meta"));
    }

    #[test]
    fn test_redact() {
        let transformer = ManifestTransformer::new();

        let yaml = r#"
properties:
  username: admin
  password: secret123
"#;

        let result = transformer.redact(yaml, &vec!["properties.password".to_string()]).unwrap();
        assert!(result.contains("username"));
        assert!(result.contains("REDACTED"));
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_extract_secret_paths() {
        let transformer = ManifestTransformer::new();

        let yaml = r#"
properties:
  username: admin
  password: secret
  database_host: localhost
  ssl_certificate: cert_data
"#;

        let paths = transformer.extract_secret_paths(yaml).unwrap();
        assert!(paths.contains(&"properties.password".to_string()));
        assert!(paths.contains(&"properties.ssl_certificate".to_string()));
        assert!(!paths.contains(&"properties.database_host".to_string()));
    }
}
