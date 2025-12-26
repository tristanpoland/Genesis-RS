//! YAML and JSON data handling utilities.

use genesis_types::{GenesisError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::fs;

/// Load YAML from string.
pub fn load_yaml(content: &str) -> Result<Value> {
    serde_yaml::from_str(content)
        .map_err(|e| GenesisError::Yaml(e))
}

/// Load YAML from file.
pub fn load_yaml_file(path: impl AsRef<Path>) -> Result<Value> {
    let content = fs::read_to_string(path)
        .map_err(|e| GenesisError::Io(e))?;
    load_yaml(&content)
}

/// Save YAML to file.
pub fn save_yaml_file(path: impl AsRef<Path>, data: &impl Serialize) -> Result<()> {
    let yaml = serde_yaml::to_string(data)?;
    fs::write(path, yaml)
        .map_err(|e| GenesisError::Io(e))?;
    Ok(())
}

/// Deep merge two YAML values (spruce-style).
pub fn deep_merge(base: Value, overlay: Value) -> Value {
    // TODO: Implement proper deep merging
    match (base, overlay) {
        (Value::Object(mut base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                base_map.insert(key, value);
            }
            Value::Object(base_map)
        }
        (_, overlay) => overlay,
    }
}

/// Priority merge (overlay wins for each key).
pub fn priority_merge(base: Value, overlay: Value) -> Value {
    // TODO: Implement priority merging
    deep_merge(base, overlay)
}

// TODO: Implement:
// - Flatten/unflatten
// - Multi-document YAML support
// - JSON handling
// - Value path lookup
