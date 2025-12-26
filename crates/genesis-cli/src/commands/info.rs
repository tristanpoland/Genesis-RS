//! Show environment information.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;

pub async fn execute(env_name: &str) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    println!("\n{}", "Environment Information".cyan().bold());
    println!("{}", "=".repeat(50));

    println!("\n{}:", "General".green().bold());
    println!("  Name: {}", env.name.to_string().cyan());
    println!("  Type: {}", env.env_type);
    println!("  Path: {:?}", env.root_dir);

    println!("\n{}:", "Kit".green().bold());
    println!("  Name: {}", env.kit.name.cyan());
    println!("  Version: {}", env.kit.version.to_string());

    if !env.features.is_empty() {
        println!("\n{}:", "Features".green().bold());
        for feature in &env.features {
            println!("  • {}", feature);
        }
    }

    if !env.params.is_empty() {
        println!("\n{}:", "Parameters".green().bold());
        for (key, value) in &env.params {
            println!("  {}: {}", key, value);
        }
    }

    println!("\n{}:", "Metadata".green().bold());
    if let Some(created) = env.metadata.created_at {
        println!("  Created: {}", created);
    }
    if let Some(deployed) = env.metadata.deployed_at {
        println!("  Last Deployed: {}", deployed);
        println!("  Deployment Count: {}", env.metadata.deployment_count);
    }

    println!("\n{}:", "Paths".green().bold());
    println!("  Exodus: {:?}", env.exodus_path());
    println!("  Cache: {:?}", env.cache_path());
    println!("  Vault Prefix: {}", env.vault_prefix());
    println!("  Deployment Name: {}", env.deployment_name());

    Ok(())
}
