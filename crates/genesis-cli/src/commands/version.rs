//! Show version information.

use anyhow::Result;
use colored::Colorize;

pub async fn execute(verbose: bool) -> Result<()> {
    println!("{} {}", "Genesis".cyan().bold(), env!("CARGO_PKG_VERSION"));

    if verbose {
        println!("\nBuild Information:");
        println!("  Version: {}", env!("CARGO_PKG_VERSION"));
        println!("  Target: {}", std::env::consts::ARCH);
        println!("  OS: {}", std::env::consts::OS);
        println!("  Rust Version: {}", env!("CARGO_PKG_RUST_VERSION"));
    }

    Ok(())
}
