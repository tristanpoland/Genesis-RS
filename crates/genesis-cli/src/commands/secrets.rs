//! Secret management commands.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;
use genesis_kit::DevKit;
use genesis_secrets::plan::SecretPlan;
use genesis_services::vault::VaultClient;
use dialoguer::Confirm;

pub async fn add(env_name: &str, force: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} secrets for: {}", "Generating".green().bold(), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;

    let vault_url = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: vault_token,
        namespace: None,
        insecure: false,
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    let plan = SecretPlan::from_kit(&kit, &env.features, &vault_prefix)?;

    println!("  Found {} secrets to generate", plan.secrets.len());

    if plan.secrets.is_empty() {
        println!("{} No secrets to generate", "✓".green().bold());
        return Ok(());
    }

    plan.generate(&vault_client, &vault_prefix).await?;

    println!("{} Generated {} secrets", "✓".green().bold(), plan.secrets.len());

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
        token: vault_token,
        namespace: None,
        insecure: false,
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    vault_client.delete_tree(&vault_prefix).await?;

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
        token: vault_token,
        namespace: None,
        insecure: false,
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    let mut plan = SecretPlan::from_kit(&kit, &env.features, &vault_prefix)?;

    if let Some(paths) = paths {
        plan.secrets.retain(|s| {
            paths.iter().any(|p| s.path().contains(p))
        });
    }

    println!("  Rotating {} secrets", plan.secrets.len());

    plan.rotate(&vault_client, &vault_prefix).await?;

    println!("{} Rotated {} secrets", "✓".green().bold(), plan.secrets.len());

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
        token: vault_token,
        namespace: None,
        insecure: false,
    };
    let vault_client = VaultClient::new(vault_config)?;

    let vault_prefix = env.vault_prefix();

    let plan = SecretPlan::from_kit(&kit, &env.features, &vault_prefix)?;

    let validation = plan.validate(&vault_client, &vault_prefix).await?;

    println!("\nSecret Status:");
    println!("  Total secrets: {}", plan.secrets.len());
    println!("  Valid: {}", validation.valid.len().to_string().green());
    println!("  Missing: {}", validation.missing.len().to_string().red());
    println!("  Invalid: {}", validation.invalid.len().to_string().yellow());

    if !validation.missing.is_empty() {
        println!("\nMissing secrets:");
        for path in &validation.missing {
            println!("  {} {}", "✗".red(), path);
        }
    }

    if !validation.invalid.is_empty() {
        println!("\nInvalid secrets:");
        for path in &validation.invalid {
            println!("  {} {}", "!".yellow(), path);
        }
    }

    if validation.is_valid() {
        println!("\n{} All secrets are valid", "✓".green().bold());
    } else {
        bail!("Some secrets are missing or invalid");
    }

    Ok(())
}
