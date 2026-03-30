//! BOSH CLI passthrough and connectivity checks.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;
use genesis_services::bosh::BoshClient;
use std::process::Command;

/// Run BOSH commands for an environment, or check BOSH connectivity.
///
/// Maps to Perl's `Genesis::Commands::Bosh::bosh()`.
pub async fn execute(
    env_name: Option<&str>,
    connect: bool,
    status: bool,
    self_: bool,
    args: &[String],
) -> Result<()> {
    // No env, no args: connectivity check
    if env_name.is_none() && args.is_empty() {
        return connectivity_check(status).await;
    }

    // No env but has --status: connectivity check
    if env_name.is_none() && status {
        return connectivity_check(true).await;
    }

    let env_name = env_name.ok_or_else(|| anyhow::anyhow!(
        "Environment name required. Usage: genesis bosh <env> [-- <bosh-args>]"
    ))?;
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let bosh_env = std::env::var("BOSH_ENVIRONMENT")
        .unwrap_or_else(|_| format!("https://bosh.{}", env.name));
    let bosh_client_id = std::env::var("BOSH_CLIENT").ok();
    let bosh_client_secret = std::env::var("BOSH_CLIENT_SECRET").ok();
    let bosh_ca_cert = std::env::var("BOSH_CA_CERT").ok();
    let deployment = env.deployment_name();

    if connect {
        println!("{} BOSH environment for {}", "Connecting".green().bold(), env_name.to_string().cyan());
        println!("  BOSH_ENVIRONMENT={}", bosh_env);
        println!("  BOSH_DEPLOYMENT={}", deployment);
        return Ok(());
    }

    if status {
        let bosh_config = genesis_services::bosh::BoshConfig {
            url: bosh_env.clone(),
            ca_cert: bosh_ca_cert.clone(),
            client: bosh_client_id.clone(),
            client_secret: bosh_client_secret.clone(),
            environment: bosh_env.clone(),
        };
        let client = BoshClient::new(bosh_config)?;
        match client.info().await {
            Ok(info) => {
                println!("{} BOSH director: {}", "✓".green().bold(), bosh_env.cyan());
                println!("  Name: {}", info.name);
                println!("  Version: {}", info.version);
                println!("  UUID: {}", info.uuid);
                println!("  Deployment: {}", deployment);
            }
            Err(e) => bail!("Failed to connect to BOSH: {}", e),
        }
        return Ok(());
    }

    // Passthrough: invoke the `bosh` binary with director env vars set
    let director_url = if self_ {
        format!("https://bosh.{}", env.name)
    } else {
        bosh_env
    };

    let mut cmd = Command::new("bosh");
    cmd.env("BOSH_ENVIRONMENT", &director_url)
       .env("BOSH_DEPLOYMENT", &deployment);
    if let Some(ref id) = bosh_client_id {
        cmd.env("BOSH_CLIENT", id);
    }
    if let Some(ref secret) = bosh_client_secret {
        cmd.env("BOSH_CLIENT_SECRET", secret);
    }
    if let Some(ref cert) = bosh_ca_cert {
        cmd.env("BOSH_CA_CERT", cert);
    }
    cmd.args(args);

    let exit = cmd.status().context("Failed to run bosh binary (is it installed and in PATH?)")?;
    if !exit.success() {
        bail!("bosh exited with status {}", exit);
    }

    Ok(())
}

/// Check BOSH connectivity (no environment required).
async fn connectivity_check(show_status: bool) -> Result<()> {
    let bosh_url = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;

    println!("{} BOSH connectivity", "Checking".green().bold());
    println!("  URL: {}", bosh_url.cyan());

    let bosh_config = genesis_services::bosh::BoshConfig {
        url: bosh_url.clone(),
        ca_cert: None,
        client: None,
        client_secret: None,
        environment: bosh_url,
    };
    let client = BoshClient::new(bosh_config)?;

    match client.info().await {
        Ok(info) => {
            println!("{} BOSH is reachable", "✓".green().bold());
            if show_status {
                println!("\nBOSH Info:");
                println!("  Name: {}", info.name);
                println!("  Version: {}", info.version);
                println!("  UUID: {}", info.uuid);
            }
        }
        Err(e) => {
            println!("{} Failed to connect to BOSH: {}", "✗".red().bold(), e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Run CredHub commands for an environment.
///
/// Maps to Perl's `Genesis::Commands::Bosh::credhub()`.
pub async fn credhub(env_name: &str, raw: bool, args: &[String]) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let credhub_server = std::env::var("CREDHUB_SERVER")
        .unwrap_or_else(|_| format!("https://credhub.{}", env.name));
    let credhub_client = std::env::var("CREDHUB_CLIENT").ok();
    let credhub_secret = std::env::var("CREDHUB_SECRET").ok();
    let credhub_ca_cert = std::env::var("CREDHUB_CA_CERT").ok();
    let deployment = env.deployment_name();

    let mut cmd = Command::new("credhub");
    cmd.env("CREDHUB_SERVER", &credhub_server);
    if let Some(ref client) = credhub_client {
        cmd.env("CREDHUB_CLIENT", client);
    }
    if let Some(ref secret) = credhub_secret {
        cmd.env("CREDHUB_SECRET", secret);
    }
    if let Some(ref cert) = credhub_ca_cert {
        cmd.env("CREDHUB_CA_CERT", cert);
    }
    if !raw {
        cmd.env("CREDHUB_PREFIX", format!("/{}", deployment));
    }
    cmd.args(args);

    let exit = cmd.status().context("Failed to run credhub binary (is it installed and in PATH?)")?;
    if !exit.success() {
        bail!("credhub exited with status {}", exit);
    }

    Ok(())
}

/// Fetch BOSH deployment logs.
///
/// Maps to Perl's `Genesis::Commands::Bosh::logs()`.
pub async fn logs(
    env_name: &str,
    instance: Option<&str>,
    follow: bool,
    num: Option<u32>,
    job: Option<&str>,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let bosh_env = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;
    let deployment = env.deployment_name();

    let mut cmd = Command::new("bosh");
    cmd.env("BOSH_ENVIRONMENT", &bosh_env)
       .env("BOSH_DEPLOYMENT", &deployment)
       .arg("-d").arg(&deployment)
       .arg("logs");

    if let Some(inst) = instance {
        cmd.arg(inst);
    }
    if follow {
        cmd.arg("--follow");
    }
    if let Some(n) = num {
        cmd.arg("--num").arg(n.to_string());
    }
    if let Some(j) = job {
        cmd.arg("--job").arg(j);
    }

    let exit = cmd.status().context("Failed to run bosh binary")?;
    if !exit.success() {
        bail!("bosh logs exited with status {}", exit);
    }

    Ok(())
}

/// Run a command across all instances of a BOSH deployment.
///
/// Maps to Perl's `Genesis::Commands::Bosh::broadcast()`.
pub async fn broadcast(
    env_name: &str,
    instance_group: Option<&str>,
    command: &str,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let bosh_env = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;
    let deployment = env.deployment_name();

    println!(
        "{} '{}' across {}",
        "Broadcasting".green().bold(),
        command.cyan(),
        deployment.cyan()
    );

    let mut cmd = Command::new("bosh");
    cmd.env("BOSH_ENVIRONMENT", &bosh_env)
       .env("BOSH_DEPLOYMENT", &deployment)
       .arg("-d").arg(&deployment)
       .arg("ssh");

    if let Some(group) = instance_group {
        cmd.arg(group);
    }

    cmd.arg("-c").arg(command);

    let exit = cmd.status().context("Failed to run bosh binary")?;
    if !exit.success() {
        bail!("bosh ssh exited with status {}", exit);
    }

    Ok(())
}

/// Manage BOSH configs (cloud-config, runtime-config, cpi-config).
///
/// Maps to Perl's `Genesis::Commands::Bosh::bosh_configs()`.
pub async fn configs(
    env_name: &str,
    action: &str,
    config_type: Option<&str>,
    config_file: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let bosh_env = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;
    let deployment = env.deployment_name();
    let ctype = config_type.unwrap_or("cloud");

    let mut cmd = Command::new("bosh");
    cmd.env("BOSH_ENVIRONMENT", &bosh_env)
       .env("BOSH_DEPLOYMENT", &deployment);

    match action {
        "upload" | "update" => {
            cmd.arg("update-config")
               .arg("--type").arg(ctype)
               .arg("--name").arg(&deployment);
            if dry_run {
                cmd.arg("--dry-run");
            }
            if let Some(file) = config_file {
                cmd.arg(file);
            }
        }
        "list" => {
            cmd.arg("configs").arg("--type").arg(ctype);
        }
        "view" => {
            cmd.arg("config")
               .arg("--type").arg(ctype)
               .arg("--name").arg(&deployment);
        }
        "delete" => {
            cmd.arg("delete-config")
               .arg("--type").arg(ctype)
               .arg("--name").arg(&deployment);
        }
        other => bail!("Unknown bosh-configs action: {}", other),
    }

    let exit = cmd.status().context("Failed to run bosh binary")?;
    if !exit.success() {
        bail!("bosh configs exited with status {}", exit);
    }

    Ok(())
}
