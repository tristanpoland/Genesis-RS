//! Run kit addon scripts (`genesis do` / `genesis addon`).

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::{EnvName, HookType};
use genesis_env::Environment;
use genesis_kit::{DevKit, Kit};
use std::collections::HashMap;
use std::process::Command;

/// Run an addon script from the environment's kit.
///
/// Corresponds to Perl's `Genesis::Commands::Env::addon()`.
/// Aliases: do, apply, run, addon.
pub async fn execute(
    env_name: &str,
    script: &str,
    args: &[String],
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} addon '{}' for {}", "Running".green().bold(), script.cyan(), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;
    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;

    // Look for addon script in hooks/addon* or hooks/<script>
    let hooks_dir = kit.path().join("hooks");
    let addon_file = find_addon_script(&hooks_dir, script)?;

    let vault_prefix = env.vault_prefix();
    let deployment = env.deployment_name();

    let mut env_vars = HashMap::new();
    env_vars.insert("GENESIS_ENV_NAME".to_string(), env.name.to_string());
    env_vars.insert("GENESIS_ROOT".to_string(), env_dir.to_string_lossy().to_string());
    env_vars.insert("GENESIS_KIT_NAME".to_string(), env.kit.name.clone());
    env_vars.insert("GENESIS_KIT_VERSION".to_string(), env.kit.version.to_string());
    env_vars.insert("GENESIS_KIT_PATH".to_string(), kit.path().to_string_lossy().to_string());
    env_vars.insert("GENESIS_VAULT_PREFIX".to_string(), vault_prefix);
    env_vars.insert("GENESIS_DEPLOYMENT".to_string(), deployment);
    env_vars.insert("GENESIS_ADDON_SCRIPT".to_string(), script.to_string());

    // Set BOSH env vars if available
    if let Ok(bosh_env) = std::env::var("BOSH_ENVIRONMENT") {
        env_vars.insert("BOSH_ENVIRONMENT".to_string(), bosh_env);
    }
    if let Ok(vault_addr) = std::env::var("VAULT_ADDR") {
        env_vars.insert("VAULT_ADDR".to_string(), vault_addr);
    }

    // Pass args as GENESIS_ADDON_ARGS and individual ARGn vars
    let args_joined = args.join(" ");
    env_vars.insert("GENESIS_ADDON_ARGS".to_string(), args_joined);
    for (i, arg) in args.iter().enumerate() {
        env_vars.insert(format!("GENESIS_ADDON_ARG{}", i + 1), arg.clone());
    }

    let mut cmd = Command::new("bash");
    cmd.arg(&addon_file);
    cmd.args(args);

    for (k, v) in &env_vars {
        cmd.env(k, v);
    }

    // Inherit existing env vars
    let exit = cmd.status().context("Failed to execute addon script")?;

    if !exit.success() {
        bail!("Addon '{}' exited with status {}", script, exit);
    }

    Ok(())
}

/// Locate an addon script by name in the hooks directory.
fn find_addon_script(hooks_dir: &std::path::Path, script: &str) -> Result<std::path::PathBuf> {
    if !hooks_dir.exists() {
        bail!("No hooks directory found in kit");
    }

    // Try direct name matches first: hooks/addon-<script>, hooks/<script>
    let candidates = vec![
        hooks_dir.join(format!("addon-{}", script)),
        hooks_dir.join(format!("addon-{}.sh", script)),
        hooks_dir.join(script),
        hooks_dir.join(format!("{}.sh", script)),
        hooks_dir.join("addon"),
        hooks_dir.join("addon.sh"),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_file() {
            return Ok(candidate.clone());
        }
    }

    // List available addons to help the user
    let available = list_addons(hooks_dir);
    if available.is_empty() {
        bail!("No addon scripts found in kit (looked in {:?})", hooks_dir);
    } else {
        bail!(
            "Addon '{}' not found. Available addons: {}",
            script,
            available.join(", ")
        );
    }
}

/// List available addon script names in a hooks directory.
fn list_addons(hooks_dir: &std::path::Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(hooks_dir) else {
        return Vec::new();
    };

    let mut addons = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("addon") {
            let clean = name
                .trim_start_matches("addon-")
                .trim_end_matches(".sh")
                .trim_end_matches(".bash")
                .to_string();
            if !clean.is_empty() && clean != "addon" {
                addons.push(clean);
            }
        }
    }
    addons.sort();
    addons.dedup();
    addons
}
