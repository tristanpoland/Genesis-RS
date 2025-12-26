//! Manifest generation and display.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;
use genesis_kit::DevKit;
use genesis_manifest::ManifestBuilder;
use genesis_services::vault::VaultClient;

pub async fn execute(env_name: &str, output: Option<&str>, redacted: bool) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    println!("{} manifest for: {}", "Generating".green().bold(), env_name.to_string().cyan());

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir).context("Failed to load environment")?;

    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
    let kit = DevKit::from_directory(&kit_dir).context("Failed to load kit")?;

    let vault_url = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: vault_token,
        namespace: None,
        insecure: false,
    };
    let vault_client = VaultClient::new(vault_config)?;

    let env_files = env.yaml_files();
    let vault_prefix = env.vault_prefix();

    let builder = ManifestBuilder::new(&kit)
        .add_env_files(env_files)
        .add_features(env.features.clone())
        .with_vault_prefix(vault_prefix);

    let manifest_content = if redacted {
        println!("  {} Generating redacted manifest", "→".yellow());
        let secret_paths = vec![];
        let manifest = builder.generate_redacted(secret_paths).await?;
        manifest.content
    } else {
        let manifest = builder.generate_entombed(&vault_client).await?;
        manifest.content
    };

    if let Some(output_path) = output {
        std::fs::write(output_path, &manifest_content)?;
        println!("{} Manifest written to: {}", "✓".green().bold(), output_path.cyan());
    } else {
        println!("\n{}", "=".repeat(80).cyan());
        println!("{}", manifest_content);
        println!("{}", "=".repeat(80).cyan());
    }

    Ok(())
}
