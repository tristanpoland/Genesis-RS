//! Create a new environment.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_types::{EnvName, KitId, SemVer};
use genesis_env::EnvironmentBuilder;
use genesis_kit::{ProviderFactory, GenesisCommunityProvider};
use std::path::Path;

pub async fn execute(name: &str, kit_name: Option<&str>, kit_version: Option<&str>) -> Result<()> {
    let env_name = EnvName::new(name)
        .context("Invalid environment name")?;

    println!("{} environment: {}", "Creating".green().bold(), env_name.to_string().cyan());

    let kit_name = kit_name.unwrap_or("bosh");
    println!("  Kit: {}", kit_name.cyan());

    let provider = GenesisCommunityProvider::new(None);

    let version = if let Some(v) = kit_version {
        SemVer::parse(v).context("Invalid kit version")?
    } else {
        println!("  Fetching latest version of {}...", kit_name);
        provider.latest_version(kit_name).await
            .context("Failed to fetch latest kit version")?
    };

    println!("  Version: {}", version.to_string().cyan());

    let kit_id = KitId {
        name: kit_name.to_string(),
        version,
    };

    let env_dir = Path::new(".").join(&env_name.to_string());

    let env = EnvironmentBuilder::new()
        .name(env_name.clone())
        .root_dir(&env_dir)
        .kit(kit_id)
        .build()
        .context("Failed to create environment")?;

    println!("{} Environment created at {:?}", "✓".green().bold(), env_dir);
    println!();
    println!("Next steps:");
    println!("  1. Edit environment configuration: {}", format!("genesis edit {}", name).cyan());
    println!("  2. Add secrets: {}", format!("genesis add-secrets {}", name).cyan());
    println!("  3. Deploy: {}", format!("genesis deploy {}", name).cyan());

    Ok(())
}
