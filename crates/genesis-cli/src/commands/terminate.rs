//! Terminate (destroy) a deployment and optionally clean up associated resources.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_types::VaultStore;
use genesis_env::{Environment, BoshDeployer, ExodusManager, Deployer};
use genesis_services::{vault::VaultClient, bosh::BoshClient};
use crate::ui::style;
use dialoguer::Confirm;

/// Terminate an environment's BOSH deployment.
///
/// Corresponds to Perl's `Genesis::Commands::Env::terminate()`.
/// Aliases: destroy, implode, kill.
pub async fn execute(
    env_name: &str,
    yes: bool,
    dry_run: bool,
    force: bool,
    clean_secrets: bool,
    clean_all: bool,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} {}", style::section("Terminating"), env_name.to_string().red().bold());

    if dry_run {
        println!("  {}", style::warning("Dry run mode - no actual changes"));
    }

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    if !yes {
        println!();
        println!("  This will {} the following:", "permanently destroy".red().bold());
        println!("    • BOSH deployment: {}", env.deployment_name().red());
        if clean_secrets || clean_all {
            println!("    • Vault secrets at: {}", env.vault_prefix().red());
        }
        if clean_all {
            println!("    • Exodus data");
        }
        println!();

        let confirmed = Confirm::new()
            .with_prompt(format!("Terminate '{}' deployment?", env_name))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", style::warning("Cancelled"));
            return Ok(());
        }
    }

    let vault_url = std::env::var("VAULT_ADDR")
        .context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN")
        .context("VAULT_TOKEN not set")?;

    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: Some(vault_token),
        namespace: None,
        insecure: false,
        strongbox: false,
        mount: "/secret/".to_string(),
        name: "default".to_string(),
    };
    let vault_client = VaultClient::new(vault_config)?;

    let bosh_url = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;
    let bosh_config = genesis_services::bosh::BoshConfig {
        url: bosh_url.clone(),
        ca_cert: None,
        client: None,
        client_secret: None,
        environment: bosh_url,
    };
    let bosh_client = BoshClient::new(bosh_config)?;

    let exodus_dir = env_dir.join(".genesis").join("exodus");
    let exodus_manager = ExodusManager::new(&exodus_dir);

    let deployer = BoshDeployer::new(bosh_client, vault_client.clone())
        .with_exodus(exodus_manager);

    if !dry_run {
        println!("{}", style::info("Deleting BOSH deployment..."));
        deployer.delete(&env).await
            .context("Failed to delete BOSH deployment")?;
        println!("{} BOSH deployment deleted", "✓".green().bold());
    } else {
        println!("  [dry-run] Would delete BOSH deployment: {}", env.deployment_name());
    }

    // Optionally clean secrets
    if (clean_secrets || clean_all) && !dry_run {
        let vault_prefix = env.vault_prefix();
        println!("{}", style::info(&format!("Removing secrets at {}...", vault_prefix)));
        vault_client.delete(&vault_prefix).await
            .context("Failed to remove secrets")?;
        println!("{} Secrets removed", "✓".green().bold());
    }

    println!("\n{} {} has been terminated", "✓".green().bold(), env_name.to_string().cyan());
    Ok(())
}
