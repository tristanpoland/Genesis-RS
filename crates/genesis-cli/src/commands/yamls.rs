//! List and view YAML files that make up an environment's manifest.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;
use genesis_kit::{DevKit, Kit};
use std::path::PathBuf;

/// List the YAML files that would be merged for an environment's manifest.
///
/// Corresponds to Perl's `Genesis::Commands::Info::yamls()`.
pub async fn execute(
    env_name: &str,
    include_kit: bool,
    view: bool,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;
    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);

    println!("{} YAML files for {}", "Listing".green().bold(), env_name.to_string().cyan());
    println!();

    let mut files: Vec<std::path::PathBuf> = Vec::new();

    // Environment YAML files (base and per-feature)
    let env_file = env_dir.join("env.yml");
    if env_file.exists() {
        files.push(env_file);
    }

    // Feature-specific YAML files
    for feature in &env.features {
        let feature_file = env_dir.join(format!("{}.yml", feature));
        if feature_file.exists() {
            files.push(feature_file);
        }
    }

    // Kit YAML files (if requested)
    if include_kit && kit_dir.exists() {
        let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;
        let blueprint = kit.blueprint(&env.features).context("Failed to get kit blueprint")?;

        for kit_file in blueprint.all_files() {
            files.push(kit_file.clone());
        }
    }

    for file in &files {
        let display = if file.is_absolute() {
            file.to_string_lossy().to_string()
        } else {
            file.strip_prefix("./").unwrap_or(file).to_string_lossy().to_string()
        };

        if view {
            println!("{}", format!("--- # {}", display).cyan().bold());
            let content = std::fs::read_to_string(file)
                .unwrap_or_else(|_| format!("# (could not read file)"));
            println!("{}", content);
        } else {
            println!("  {}", display);
        }
    }

    if files.is_empty() {
        println!("  {} No YAML files found", "!".yellow());
    }

    Ok(())
}
