//! Secret definition parsing from kits and manifests.

use genesis_types::{GenesisError, Result, SecretType};
use crate::types::create_secret;
use crate::plan::SecretPlan;
use serde_json::Value;
use std::collections::HashMap;

/// Parse secrets from kit definitions.
pub struct FromKit;

impl FromKit {
    /// Parse kit secret definitions.
    pub fn parse(
        kit_secrets: &Value,
        plan: &mut SecretPlan,
    ) -> Result<()> {
        if let Some(secrets_map) = kit_secrets.as_object() {
            for (path, definition) in secrets_map {
                if let Some(def_obj) = definition.as_object() {
                    let secret_type = def_obj.get("type")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| GenesisError::Secret(format!(
                            "Missing type for secret: {}",
                            path
                        )))?;

                    let stype = Self::parse_secret_type(secret_type)?;

                    let mut def_map: HashMap<String, Value> = def_obj.iter()
                        .filter(|(k, _)| *k != "type")
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();

                    let secret = create_secret(stype, path.clone(), def_map)?;
                    plan.add_secret(secret);
                }
            }
        }

        Ok(())
    }

    fn parse_secret_type(type_str: &str) -> Result<SecretType> {
        match type_str.to_lowercase().as_str() {
            "x509" | "certificate" | "cert" => Ok(SecretType::X509),
            "ssh" => Ok(SecretType::SSH),
            "rsa" => Ok(SecretType::RSA),
            "dhparams" | "dhparam" | "dh" => Ok(SecretType::DHParams),
            "random" | "password" => Ok(SecretType::Random),
            "uuid" => Ok(SecretType::UUID),
            "user" | "user-provided" => Ok(SecretType::UserProvided),
            _ => Err(GenesisError::Secret(format!(
                "Unknown secret type: {}",
                type_str
            ))),
        }
    }
}

/// Parse secrets from manifest variable definitions.
pub struct FromManifest;

impl FromManifest {
    /// Parse manifest variable definitions.
    pub fn parse(
        manifest: &Value,
        plan: &mut SecretPlan,
    ) -> Result<()> {
        if let Some(variables) = manifest.get("variables").and_then(|v| v.as_array()) {
            for var in variables {
                if let Some(var_obj) = var.as_object() {
                    let name = var_obj.get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| GenesisError::Secret("Variable missing name".to_string()))?;

                    let var_type = var_obj.get("type")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| GenesisError::Secret(format!(
                            "Variable {} missing type",
                            name
                        )))?;

                    if let Ok(stype) = Self::parse_variable_type(var_type) {
                        let mut def_map = HashMap::new();

                        if let Some(options) = var_obj.get("options").and_then(|v| v.as_object()) {
                            for (k, v) in options {
                                def_map.insert(k.clone(), v.clone());
                            }
                        }

                        let secret = create_secret(stype, name.to_string(), def_map)?;
                        plan.add_secret(secret);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_variable_type(type_str: &str) -> Result<SecretType> {
        match type_str.to_lowercase().as_str() {
            "certificate" => Ok(SecretType::X509),
            "ssh" => Ok(SecretType::SSH),
            "rsa" => Ok(SecretType::RSA),
            "password" => Ok(SecretType::Random),
            "user" => Ok(SecretType::UserProvided),
            _ => Err(GenesisError::Secret(format!(
                "Unsupported variable type for secrets: {}",
                type_str
            ))),
        }
    }
}

/// Secret parser that combines kit and manifest sources.
pub struct SecretParser;

impl SecretParser {
    /// Parse secrets from both kit and manifest.
    pub fn parse_all(
        kit_secrets: Option<&Value>,
        manifest: Option<&Value>,
        plan: &mut SecretPlan,
    ) -> Result<()> {
        if let Some(kit_defs) = kit_secrets {
            FromKit::parse(kit_defs, plan)?;
        }

        if let Some(manifest_defs) = manifest {
            FromManifest::parse(manifest_defs, plan)?;
        }

        plan.sort_by_dependencies()?;

        Ok(())
    }
}
