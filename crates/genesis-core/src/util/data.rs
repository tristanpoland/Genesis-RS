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
/// Recursively merges objects, with overlay values taking precedence.
pub fn deep_merge(mut base: Value, overlay: Value) -> Value {
    match (&mut base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                if let Some(base_val) = base_map.get_mut(&key) {
                    *base_val = deep_merge(base_val.clone(), overlay_val);
                } else {
                    base_map.insert(key, overlay_val);
                }
            }
            Value::Object(base_map.clone())
        }
        (_, overlay_val) => overlay_val,
    }
}

/// Priority merge (overlay wins for each key).
/// Same as deep_merge - overlay values always take precedence.
pub fn priority_merge(base: Value, overlay: Value) -> Value {
    deep_merge(base, overlay)
}

/// Flatten a nested value into dotted paths.
pub fn flatten(value: &Value) -> Vec<(String, Value)> {
    fn flatten_recursive(value: &Value, prefix: String, result: &mut Vec<(String, Value)>) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let new_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", prefix, key)
                    };
                    flatten_recursive(val, new_prefix, result);
                }
            }
            _ => {
                result.push((prefix, value.clone()));
            }
        }
    }

    let mut result = Vec::new();
    flatten_recursive(value, String::new(), &mut result);
    result
}

/// Get value at a path in dotted notation.
pub fn get_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            Value::Array(arr) => {
                let index: usize = part.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }

    Some(current)
}
