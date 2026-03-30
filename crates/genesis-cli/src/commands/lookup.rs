//! Look up values in an environment's configuration or manifest.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;

/// Look up a key in the environment YAML or manifest.
///
/// Corresponds to Perl's `Genesis::Commands::Info::lookup()`.
/// Modes: --merged (default), --partial, --deployed, --exodus, --exodus-for <env>, --defined
pub async fn execute(
    env_name: &str,
    key: &str,
    merged: bool,
    partial: bool,
    deployed: bool,
    exodus: bool,
    exodus_for: Option<&str>,
    defined_only: bool,
    output_env: bool,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    if exodus || exodus_for.is_some() {
        // Look up in exodus data
        let env_name_str = env_name.to_string();
        let exodus_env = exodus_for.unwrap_or(&env_name_str);
        let exodus_file = env_dir.join(".genesis").join("exodus")
            .join(format!("{}.yml", exodus_env));

        if !exodus_file.exists() {
            bail!("No exodus data found for '{}'", exodus_env);
        }

        let exodus_yaml = std::fs::read_to_string(&exodus_file)
            .context("Failed to read exodus data")?;
        let exodus_data: serde_json::Value = serde_yaml::from_str(&exodus_yaml)
            .context("Failed to parse exodus data")?;

        if let Some(value) = lookup_nested(&exodus_data, key) {
            print_value(key, value, output_env)?;
        } else if defined_only {
            // defined_only: exit 1 without output if key not present
            std::process::exit(1);
        } else {
            bail!("Key '{}' not found in exodus data", key);
        }
        return Ok(());
    }

    if deployed {
        // Look up in the deployed (cached) manifest
        let manifest_file = env_dir.join(".genesis").join("cached")
            .join(format!("{}.yml", env_name));

        if !manifest_file.exists() {
            bail!("No cached manifest found for '{}'. Deploy first.", env_name);
        }

        let manifest_yaml = std::fs::read_to_string(&manifest_file)
            .context("Failed to read cached manifest")?;
        let manifest: serde_json::Value = serde_yaml::from_str(&manifest_yaml)
            .context("Failed to parse manifest")?;

        if let Some(value) = lookup_nested(&manifest, key) {
            print_value(key, value, output_env)?;
        } else if defined_only {
            std::process::exit(1);
        } else {
            bail!("Key '{}' not found in deployed manifest", key);
        }
        return Ok(());
    }

    // Default: look up in the merged environment YAML params
    if let Some(value) = env.params.get(key) {
        print_value(key, value, output_env)?;
    } else {
        // Try dot-separated key traversal in params
        let found = find_in_params(&env, key);
        if let Some(value) = found {
            print_value(key, &value, output_env)?;
        } else if defined_only {
            std::process::exit(1);
        } else {
            bail!("Key '{}' not found in environment '{}'", key, env_name);
        }
    }

    Ok(())
}

/// Recursively look up a dot-separated key in a JSON value.
fn lookup_nested<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    match parts.as_slice() {
        [head] => value.get(*head),
        [head, rest] => value.get(*head).and_then(|v| lookup_nested(v, rest)),
        _ => None,
    }
}

/// Look up a dot-separated key in the environment params.
fn find_in_params(env: &Environment, key: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    let root = parts[0];
    let rest = parts.get(1).copied();

    env.params.get(root).and_then(|v| {
        if let Some(r) = rest {
            lookup_nested(v, r).cloned()
        } else {
            Some(v.clone())
        }
    })
}

/// Print a value, optionally as a shell variable assignment.
fn print_value(key: &str, value: &serde_json::Value, output_env: bool) -> Result<()> {
    if output_env {
        let var_name = key.replace('.', "_").replace('-', "_").to_uppercase();
        match value {
            serde_json::Value::String(s) => println!("{}={}", var_name, s),
            serde_json::Value::Number(n) => println!("{}={}", var_name, n),
            serde_json::Value::Bool(b) => println!("{}={}", var_name, b),
            other => println!("{}={}", var_name, other),
        }
    } else {
        match value {
            serde_json::Value::String(s) => println!("{}", s),
            serde_json::Value::Number(n) => println!("{}", n),
            serde_json::Value::Bool(b) => println!("{}", b),
            other => {
                let yaml = serde_yaml::to_string(other).unwrap_or_else(|_| other.to_string());
                print!("{}", yaml);
            }
        }
    }
    Ok(())
}
