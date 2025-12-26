//! Deploy an environment.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::{Environment, BoshDeployer, ExodusManager};
use genesis_kit::DevKit;
use genesis_services::{vault::VaultClient, bosh::BoshClient};
use crate::ui::progress;

pub async fn execute(env_name: &str, dry_run: bool, no_secrets: bool, force: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} environment: {}", "Deploying".green().bold(), env_name.to_string().cyan());

    if dry_run {
        println!("  {} Dry run mode - no actual deployment", "→".yellow());
    }

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let mut env = Environment::load(&env_dir)
        .context("Failed to load environment")?;

    println!("  Kit: {} v{}", env.kit.name.cyan(), env.kit.version.to_string().cyan());
    println!("  Features: {}", env.features.join(", ").cyan());

    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    if !kit_dir.exists() {
        bail!("Kit not found. Run 'genesis download {}' first", env.kit.name);
    }

    let kit = DevKit::from_directory(&kit_dir)
        .context("Failed to load kit")?;

    let vault_url = std::env::var("GENESIS_VAULT_ADDR")
        .or_else(|_| std::env::var("VAULT_ADDR"))
        .context("VAULT_ADDR not set")?;

    let vault_token = std::env::var("GENESIS_VAULT_TOKEN")
        .or_else(|_| std::env::var("VAULT_TOKEN"))
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
        url: bosh_url.clone(),
        ca_cert: None,
        client: None,
        client_secret: None,
        environment: bosh_url,
    };
    let bosh_client = BoshClient::new(bosh_config)?;

    let exodus_dir = env_dir.join(".genesis").join("exodus");
    let exodus_manager = ExodusManager::new(&exodus_dir);

    let deployer = BoshDeployer::new(bosh_client, vault_client)
        .with_exodus(exodus_manager);

    let spinner = progress::spinner("Deploying to BOSH...");

    let result = deployer.deploy(&mut env, &kit, dry_run).await;

    spinner.finish_and_clear();

    match result {
        Ok(record) => {
            if record.is_success() {
                println!("{} Deployment succeeded", "✓".green().bold());
                if let Some(task_id) = record.bosh_task_id {
                    println!("  BOSH task ID: {}", task_id.cyan());
                }
                if let Some(duration) = record.duration_secs {
                    println!("  Duration: {}s", duration);
                }
            } else {
                bail!("Deployment failed: {:?}", record.error);
            }
        }
        Err(e) => {
            bail!("Deployment failed: {}", e);
        }
    }

    Ok(())
}
