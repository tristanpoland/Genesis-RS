//! Display vault secret paths for an environment.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;
use genesis_kit::DevKit;
use genesis_kit::Kit;

/// Show vault secret paths used by an environment.
///
/// Corresponds to Perl's `Genesis::Commands::Info::vault_paths()`.
pub async fn execute(env_name: &str, references: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;
    let vault_prefix = env.vault_prefix();

    println!("{} vault paths for {}", "Showing".green().bold(), env_name.to_string().cyan());
    println!();
    println!("  Base prefix: {}", vault_prefix.cyan());
    println!();

    // Load kit to enumerate secret definitions
    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    if kit_dir.exists() {
        let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;
        let secrets_file = kit.path().join("secrets.yml");

        if secrets_file.exists() {
            let secrets_yaml = std::fs::read_to_string(&secrets_file)
                .context("Failed to read kit secrets.yml")?;
            let secrets_value: serde_json::Value = serde_yaml::from_str(&secrets_yaml)
                .context("Failed to parse kit secrets.yml")?;

            let secrets_def = secrets_value.get("secrets").unwrap_or(&secrets_value);

            if let Some(map) = secrets_def.as_object() {
                for (path, definition) in map {
                    let full_path = format!("{}{}", vault_prefix, path);
                    let secret_type = definition.get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");

                    if references {
                        println!("  {} ({})", full_path.cyan(), secret_type.yellow());
                        // Show the vault operator reference format
                        println!("    operator: (( vault meta.vault \"{}\" ))", path);
                    } else {
                        println!("  {}", full_path.cyan());
                    }
                }
            }
        } else {
            println!("  {} No secrets.yml found in kit", "!".yellow());
        }
    } else {
        println!("  {} Kit not found; showing only base prefix", "!".yellow());
    }

    Ok(())
}
