//! Download kits.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_kit::{ProviderFactory, GenesisCommunityProvider};
use crate::ui::progress;

pub async fn execute(kit_name: &str, version: Option<&str>, output: &str) -> Result<()> {
    println!("{} kit: {}", "Downloading".green().bold(), kit_name.cyan());

    let provider = GenesisCommunityProvider::new(None);

    let install_dir = std::path::Path::new(output).join(".genesis").join("kits");

    if let Some(v) = version {
        println!("  Version: {}", v.cyan());
    } else {
        println!("  Fetching latest version...");
        let latest = provider.latest_version(kit_name).await?;
        println!("  Latest version: {}", latest.to_string().cyan());
    }

    let spinner = progress::spinner("Downloading kit...");

    let kit_box = if let Some(v) = version {
        let version_obj = genesis_types::SemVer::parse(v)?;
        provider.install_kit(kit_name, &version_obj, &install_dir).await?
    } else {
        provider.install_latest(kit_name, &install_dir).await?
    };

    spinner.finish_and_clear();

    println!("{} Downloaded {} v{}", "✓".green().bold(), kit_box.name(), kit_box.version());

    Ok(())
}
