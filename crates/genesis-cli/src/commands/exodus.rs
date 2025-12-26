//! Exodus data management commands.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::{Environment, ExodusManager};

pub async fn export(env_name: &str, output: Option<&str>) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} exodus data for: {}", "Exporting".green().bold(), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir)?;

    let exodus_manager = ExodusManager::new(env.exodus_path());

    let output_path = output.unwrap_or(&format!("{}-exodus.json", env_name));

    exodus_manager.export(&env_name, output_path)?;

    println!("{} Exodus data exported to: {}", "✓".green().bold(), output_path.cyan());

    Ok(())
}

pub async fn import(from: &str, to: &str, keys: Option<&Vec<String>>) -> Result<()> {
    let from_env = EnvName::new(from).context("Invalid source environment name")?;
    let to_env = EnvName::new(to).context("Invalid target environment name")?;

    println!("{} exodus data from {} to {}",
        "Importing".green().bold(),
        from_env.to_string().cyan(),
        to_env.to_string().cyan()
    );

    let env_dir = std::env::current_dir()?;
    let exodus_dir = env_dir.join(".genesis").join("exodus");

    let exodus_manager = ExodusManager::new(&exodus_dir);

    exodus_manager.import(&from_env, &to_env, keys.cloned())?;

    if let Some(k) = keys {
        println!("{} Imported {} keys", "✓".green().bold(), k.len());
    } else {
        println!("{} Imported all exodus data", "✓".green().bold());
    }

    Ok(())
}
