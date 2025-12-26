//! Genesis CLI entry point.

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

mod cli;
mod commands;
mod ui;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let cli = Cli::parse();

    match cli.execute().await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("genesis=info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .init();
}
