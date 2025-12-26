//! Update Genesis.

use anyhow::Result;
use colored::Colorize;

pub async fn execute(check: bool) -> Result<()> {
    if check {
        println!("{} for updates", "Checking".green().bold());
        println!("  Current version: {}", env!("CARGO_PKG_VERSION").cyan());
        println!("  {} Update checking not yet implemented", "!".yellow());
    } else {
        println!("{} Genesis", "Updating".green().bold());
        println!("  {} Self-update not yet implemented", "!".yellow());
        println!("  Please update manually with: cargo install genesis-cli");
    }

    Ok(())
}
