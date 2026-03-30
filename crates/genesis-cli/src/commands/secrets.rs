//! Secret management commands.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;
use genesis_kit::DevKit;
use genesis_secrets::plan::SecretPlan;
use genesis_types::VaultStore;
use genesis_services::vault::VaultClient;
use crate::ui::style;
use dialoguer::Confirm;

pub async fn add(env_name: &str, force: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} {}", style::section("Generating secrets for"), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;

    let vault_url = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: Some(vault_token),
        namespace: None,
        insecure: false,
        strongbox: true,
        mount: "/secret/".to_string(),
        name: "default".to_string(),
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    let mut plan = SecretPlan::new(Box::new(vault_client.clone()), vault_prefix.clone());

    // TODO: Parse secrets from kit - needs implementation
    // For now, create an empty plan
    // let secrets_file = kit.path().join("secrets.yml");
    // if secrets_file.exists() {
    //     let secrets_def = std::fs::read_to_string(secrets_file)?;
    //     let secrets_json: serde_json::Value = serde_yaml::from_str(&secrets_def)?;
    //     genesis_secrets::parser::FromKit::parse(&secrets_json, &mut plan)?;
    // }

    println!("{}", style::info(&format!("Found {} secrets to generate", plan.count())));

    if plan.count() == 0 {
        println!("{}", style::success("No secrets to generate"));
        return Ok(());
    }

    plan.generate_missing().await?;

    println!("{}", style::success(&format!("Generated {} secrets", plan.count())));

    Ok(())
}

pub async fn remove(env_name: &str, yes: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} secrets for: {}", "Removing".red().bold(), env_name.to_string().cyan());

    if !yes {
        let confirmed = Confirm::new()
            .with_prompt(format!("Are you sure you want to remove all secrets for '{}'?", env_name))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled".yellow());
            return Ok(());
        }
    }

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let vault_url = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: Some(vault_token),
        namespace: None,
        insecure: false,
        strongbox: true,
        mount: "/secret/".to_string(),
        name: "default".to_string(),
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    vault_client.delete(&vault_prefix).await?;

    println!("{} Removed all secrets from {}", "✓".green().bold(), vault_prefix.cyan());

    Ok(())
}

pub async fn rotate(env_name: &str, paths: Option<&Vec<String>>, yes: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} secrets for: {}", "Rotating".yellow().bold(), env_name.to_string().cyan());

    if !yes {
        let msg = if let Some(paths) = paths {
            format!("Rotate {} secrets for '{}'?", paths.len(), env_name)
        } else {
            format!("Rotate ALL secrets for '{}'?", env_name)
        };

        let confirmed = Confirm::new()
            .with_prompt(msg)
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled".yellow());
            return Ok(());
        }
    }

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;

    let vault_url = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: Some(vault_token),
        namespace: None,
        insecure: false,
        strongbox: true,
        mount: "/secret/".to_string(),
        name: "default".to_string(),
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    let mut plan = SecretPlan::new(Box::new(vault_client.clone()), vault_prefix.clone());

    // TODO: Parse secrets from kit - needs implementation
    // For now, create an empty plan

    let rotate_paths = if let Some(paths) = paths {
        paths.clone()
    } else {
        plan.paths()
    };

    println!("  Rotating {} secrets", rotate_paths.len());

    plan.rotate(&rotate_paths).await?;

    println!("{} Rotated {} secrets", "✓".green().bold(), rotate_paths.len());

    Ok(())
}

pub async fn check(env_name: &str) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} secrets for: {}", "Checking".cyan().bold(), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;

    let vault_url = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: Some(vault_token),
        namespace: None,
        insecure: false,
        strongbox: true,
        mount: "/secret/".to_string(),
        name: "default".to_string(),
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    let plan = SecretPlan::new(Box::new(vault_client.clone()), vault_prefix.clone());

    // TODO: Parse secrets from kit - needs implementation
    // For now, create an empty plan

    let validation_results = plan.validate().await?;

    println!("\nSecret Status:");
    println!("  Total secrets: {}", plan.count());

    use genesis_types::traits::ValidationResult as VR;
    let valid_count = validation_results.values().filter(|v| matches!(v, VR::Ok)).count();
    let missing_count = validation_results.values().filter(|v| matches!(v, VR::Missing)).count();
    let warning_count = validation_results.values().filter(|v| matches!(v, VR::Warning(_))).count();
    let error_count = validation_results.values().filter(|v| matches!(v, VR::Error(_))).count();

    println!("  Valid: {}", valid_count.to_string().green());
    println!("  Missing: {}", missing_count.to_string().red());
    if warning_count > 0 {
        println!("  Warnings: {}", warning_count.to_string().yellow());
    }
    if error_count > 0 {
        println!("  Errors: {}", error_count.to_string().red());
    }

    for (path, result) in &validation_results {
        match result {
            VR::Missing => {
                println!("  {} {} (missing)", "✗".red(), path);
            }
            VR::Warning(msgs) => {
                println!("  {} {} - {}", "!".yellow(), path, msgs.join(", "));
            }
            VR::Error(msgs) => {
                println!("  {} {} - {}", "✗".red(), path, msgs.join(", "));
            }
            VR::Ok => {}
        }
    }

    if missing_count == 0 && error_count == 0 {
        println!("\n{} All secrets are valid", "✓".green().bold());
    } else {
        bail!("Some secrets are missing or invalid");
    }

    Ok(())
}
