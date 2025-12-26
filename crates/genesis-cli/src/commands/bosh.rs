//! BOSH connectivity checks.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_services::bosh::BoshClient;

pub async fn check(status: bool) -> Result<()> {
    let bosh_url = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;

    println!("{} BOSH connectivity", "Checking".green().bold());
    println!("  URL: {}", bosh_url.cyan());

    let bosh_config = genesis_services::bosh::BoshConfig {
        url: bosh_url,
        ca_cert: None,
        client: None,
        client_secret: None,
    };
    let client = BoshClient::new(bosh_config)?;

    match client.info().await {
        Ok(info) => {
            println!("{} BOSH is reachable", "✓".green().bold());
            if status {
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
