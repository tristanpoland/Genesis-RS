//! Delete a deployment.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::{Environment, BoshDeployer, ExodusManager};
use genesis_services::{vault::VaultClient, bosh::BoshClient};
use dialoguer::Confirm;

pub async fn execute(env_name: &str, yes: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} deployment: {}", "Deleting".red().bold(), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir)
        .context("Failed to load environment")?;

    if !yes {
        let confirmed = Confirm::new()
            .with_prompt(format!("Are you sure you want to delete deployment '{}'?", env_name))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", "Cancelled".yellow());
            return Ok(());
        }
    }

    let vault_url = std::env::var("VAULT_ADDR")
        .context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN")
        .context("VAULT_TOKEN not set")?;

    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: vault_token,
        namespace: None,
        insecure: false,
    };
    let vault_client = VaultClient::new(vault_config)?;

    let bosh_url = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;

    let bosh_config = genesis_services::bosh::BoshConfig {
        url: bosh_url,
        ca_cert: None,
        client: None,
        client_secret: None,
    };
    let bosh_client = BoshClient::new(bosh_config)?;

    let exodus_dir = env_dir.join(".genesis").join("exodus");
    let exodus_manager = ExodusManager::new(&exodus_dir);

    let deployer = BoshDeployer::new(bosh_client, vault_client)
        .with_exodus(exodus_manager);

    deployer.delete(&env).await
        .context("Failed to delete deployment")?;

    println!("{} Deployment deleted successfully", "✓".green().bold());

    Ok(())
}
