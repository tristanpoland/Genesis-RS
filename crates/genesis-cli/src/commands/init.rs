//! Initialize a new Genesis repository.

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub async fn execute(path: &str) -> Result<()> {
    let repo_path = Path::new(path);

    println!("{} Genesis repository at {:?}", "Initializing".green().bold(), repo_path);

    std::fs::create_dir_all(repo_path)?;
    std::fs::create_dir_all(repo_path.join(".genesis"))?;
    std::fs::create_dir_all(repo_path.join("ops"))?;

    let genesis_config = repo_path.join(".genesis").join("config");
    if !genesis_config.exists() {
        std::fs::write(&genesis_config, "---\n# Genesis configuration\n")?;
    }

    let readme = repo_path.join("README.md");
    if !readme.exists() {
        std::fs::write(&readme, "# Genesis Deployments\n\nThis repository contains Genesis deployment environments.\n")?;
    }

    let gitignore = repo_path.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, ".genesis/cached/\n.genesis/kits/\n*.swp\n*~\n")?;
    }

    println!("{} Genesis repository initialized successfully", "✓".green().bold());
    println!();
    println!("Next steps:");
    println!("  1. Create a new environment: {}", "genesis new <env-name>".cyan());
    println!("  2. Configure your environment files");
    println!("  3. Deploy: {}", "genesis deploy <env-name>".cyan());

    Ok(())
}
