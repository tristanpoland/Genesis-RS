//! Vault connectivity checks.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_services::vault::VaultClient;

pub async fn check(status: bool) -> Result<()> {
    let vault_url = std::env::var("VAULT_ADDR")
        .context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN")
        .context("VAULT_TOKEN not set")?;

    println!("{} Vault connectivity", "Checking".green().bold());
    println!("  URL: {}", vault_url.cyan());

    let client = VaultClient::new(&vault_url, &vault_token)?;

    match client.is_initialized().await {
        Ok(initialized) => {
            println!("{} Vault is reachable", "✓".green().bold());
            if status {
                println!("\nVault Status:");
                println!("  Initialized: {}", initialized);
                if let Ok(sealed) = client.is_sealed().await {
                    println!("  Sealed: {}", sealed);
                }
            }
        }
        Err(e) => {
            println!("{} Failed to connect to Vault: {}", "✗".red().bold(), e);
            return Err(e.into());
        }
    }

    Ok(())
}
