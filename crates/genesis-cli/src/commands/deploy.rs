//! Deploy an environment.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::{Environment, BoshDeployer, ExodusManager, Deployer, DeployOptions};
use genesis_kit::DevKit;
use genesis_services::{vault::VaultClient, bosh::BoshClient};
use crate::ui::{progress, style};

pub async fn execute(
    env_name: &str,
    dry_run: bool,
    no_secrets: bool,
    force: bool,
    yes: bool,
    recreate: bool,
    fix_stemcells: bool,
    skip_drain: bool,
    canaries: Option<u32>,
    max_in_flight: Option<u32>,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} {}", style::section("Deploying"), env_name.to_string().cyan());

    if dry_run {
        println!("  {}", style::warning("Dry run mode - no actual deployment"));
    }

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let mut env = Environment::load(&env_dir)
        .context("Failed to load environment")?;

    println!("  Kit: {} v{}", env.kit.name.cyan(), env.kit.version.to_string().cyan());
    println!("  Features: {}", env.features.join(", ").cyan());

    if recreate {
        println!("  {}", style::warning("--recreate: all VMs will be recreated"));
    }
    if fix_stemcells {
        println!("  {}", style::warning("--fix: broken stemcells/jobs will be fixed"));
    }
    if skip_drain {
        println!("  {}", style::warning("--skip-drain: drain scripts will be skipped"));
    }

    // Confirmation prompt (skip if --yes or --dry-run)
    if !yes && !dry_run {
        print!("  Deploy {} to {}? [y/N] ", env_name.to_string().cyan(), env.kit.name.cyan());
        use std::io::{self, Write};
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).context("Failed to read confirmation")?;
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("  {}", style::warning("Deployment cancelled."));
            return Ok(());
        }
    }

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
        ca_cert: std::env::var("BOSH_CA_CERT").ok(),
        client: std::env::var("BOSH_CLIENT").ok(),
        client_secret: std::env::var("BOSH_CLIENT_SECRET").ok(),
        environment: bosh_url,
    };
    let bosh_client = BoshClient::new(bosh_config)?;

    let exodus_dir = env_dir.join(".genesis").join("exodus");
    let exodus_manager = ExodusManager::new(&exodus_dir);

    let deployer = BoshDeployer::new(bosh_client, vault_client)
        .with_exodus(exodus_manager);

    let options = DeployOptions {
        dry_run,
        yes,
        recreate,
        fix_stemcells,
        skip_drain,
        canaries,
        max_in_flight,
    };

    let spinner = progress::spinner("Deploying to BOSH...");

    let result = deployer.deploy(&mut env, &kit, &options).await;

    spinner.finish_and_clear();

    match result {
        Ok(record) => {
            if record.is_success() {
                println!("{}", style::success("Deployment succeeded"));
                if let Some(task_id) = record.bosh_task_id {
                    println!("  {}", style::info(&format!("BOSH task ID: {}", task_id)));
                }
                if let Some(duration) = record.duration_secs {
                    println!("  {}", style::info(&format!("Duration: {}s", duration)));
                }
            } else {
                bail!("{}", style::error(&format!("Deployment failed: {:?}", record.error)));
            }
        }
        Err(e) => {
            bail!("Deployment failed: {}", e);
        }
    }

    Ok(())
}
