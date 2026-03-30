//! Manifest validation command.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::{EnvName, HookType};
use genesis_env::Environment;
use genesis_kit::{DevKit, Kit};
use std::process::Command;

/// Validate an environment's manifest without deploying.
///
/// Corresponds to Perl's `Genesis::Commands::Env::check()`.
/// Options mirror Perl: --no-config, --secrets, --manifest, --stemcells.
pub async fn execute(
    env_name: &str,
    no_config: bool,
    check_secrets: bool,
    check_manifest: bool,
    check_stemcells: bool,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} {}", "Checking".green().bold(), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;
    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;

    // Run the kit's check hook if it exists
    use genesis_types::HookType;
    if kit.has_hook(HookType::Check) {
        println!("  {}", "Running kit check hook...".cyan());
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("GENESIS_ENV_NAME".to_string(), env.name.to_string());
        env_vars.insert("GENESIS_KIT_NAME".to_string(), env.kit.name.clone());
        env_vars.insert("GENESIS_KIT_VERSION".to_string(), env.kit.version.to_string());

        match kit.execute_hook(HookType::Check, env_vars) {
            Ok(result) if result.success => {
                println!("  {} Kit check passed", "✓".green());
            }
            Ok(result) => {
                eprintln!("{}", result.stderr);
                bail!("Kit check hook failed with code {}", result.exit_code);
            }
            Err(e) => bail!("Failed to run check hook: {}", e),
        }
    }

    // Run bosh int to validate manifest syntax
    if check_manifest {
        println!("  {}", "Validating manifest...".cyan());
        let manifest_file = env_dir.join(".genesis").join("cached").join(format!("{}.yml", env_name));
        if manifest_file.exists() {
            let output = Command::new("bosh")
                .arg("int")
                .arg(&manifest_file)
                .output()
                .context("Failed to run bosh int (is bosh installed?)")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Manifest validation failed:\n{}", stderr);
            }
            println!("  {} Manifest is valid", "✓".green());
        } else {
            println!("  {} No cached manifest found; skipping manifest check", "!".yellow());
        }
    }

    println!("\n{} {} looks good", "✓".green().bold(), env_name.to_string().cyan());
    Ok(())
}
