//! List kits and environments.

use anyhow::Result;
use colored::Colorize;
use genesis_kit::GenesisCommunityProvider;
use walkdir::WalkDir;

pub async fn kits(all: bool) -> Result<()> {
    println!("{} available kits", "Listing".green().bold());

    let provider = GenesisCommunityProvider::new(None);

    let common_kits = vec!["bosh", "cf", "concourse", "vault", "shield", "blacksmith"];

    for kit_name in common_kits {
        match provider.list_versions(kit_name).await {
            Ok(versions) => {
                if all {
                    println!("\n{}:", kit_name.cyan().bold());
                    for v in versions {
                        println!("  {}", v.to_string());
                    }
                } else if let Some(latest) = versions.first() {
                    println!("  {} (latest: {})", kit_name.cyan(), latest.to_string());
                }
            }
            Err(_) => continue,
        }
    }

    Ok(())
}

pub async fn envs(detailed: bool) -> Result<()> {
    println!("{} environments", "Listing".green().bold());

    let current_dir = std::env::current_dir()?;

    let mut found_any = false;

    for entry in WalkDir::new(&current_dir)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.join("env.yml").exists() {
            found_any = true;

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                println!("\n{}:", name.cyan().bold());

                if detailed {
                    if let Ok(env) = genesis_env::Environment::load(path) {
                        println!("  Kit: {} v{}", env.kit.name, env.kit.version);
                        if !env.features.is_empty() {
                            println!("  Features: {}", env.features.join(", "));
                        }
                        if let Some(deployed) = env.metadata.deployed_at {
                            println!("  Last deployed: {}", deployed);
                        }
                    }
                } else {
                    if let Ok(env) = genesis_env::Environment::load(path) {
                        println!("  {} v{}", env.kit.name, env.kit.version);
                    }
                }
            }
        }
    }

    if !found_any {
        println!("  {} No environments found", "!".yellow());
    }

    Ok(())
}
