//! Show differences between manifests.

use anyhow::{Result, Context};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;
use genesis_kit::DevKit;
use genesis_manifest::ManifestBuilder;
use genesis_services::vault::VaultClient;

pub async fn execute(env1_name: &str, env2_name: &str) -> Result<()> {
    let env1_name = EnvName::new(env1_name).context("Invalid first environment name")?;
    let env2_name = EnvName::new(env2_name).context("Invalid second environment name")?;

    println!("{} manifests: {} vs {}",
        "Comparing".green().bold(),
        env1_name.to_string().cyan(),
        env2_name.to_string().cyan()
    );

    let vault_url = std::env::var("VAULT_ADDR").context("VAULT_ADDR not set")?;
    let vault_token = std::env::var("VAULT_TOKEN").context("VAULT_TOKEN not set")?;
    let vault_config = genesis_services::vault::VaultConfig {
        url: vault_url,
        token: vault_token,
        namespace: None,
        insecure: false,
    };
    let vault_client = VaultClient::new(vault_config)?;

    let env1_dir = std::path::Path::new(".").join(env1_name.to_string());
    let env1 = Environment::load(&env1_dir)?;
    let kit1_dir = env1_dir.join(".genesis").join("kits").join(&env1.kit.name);
    let kit1 = DevKit::from_directory(&kit1_dir)?;

    let manifest1 = ManifestBuilder::new(&kit1)
        .add_env_files(env1.yaml_files())
        .add_features(env1.features.clone())
        .with_vault_prefix(env1.vault_prefix())
        .generate_entombed(&vault_client)
        .await?;

    let env2_dir = std::path::Path::new(".").join(env2_name.to_string());
    let env2 = Environment::load(&env2_dir)?;
    let kit2_dir = env2_dir.join(".genesis").join("kits").join(&env2.kit.name);
    let kit2 = DevKit::from_directory(&kit2_dir)?;

    let manifest2 = ManifestBuilder::new(&kit2)
        .add_env_files(env2.yaml_files())
        .add_features(env2.features.clone())
        .with_vault_prefix(env2.vault_prefix())
        .generate_entombed(&vault_client)
        .await?;

    let temp_dir = tempfile::tempdir()?;
    let file1 = temp_dir.path().join("manifest1.yml");
    let file2 = temp_dir.path().join("manifest2.yml");

    std::fs::write(&file1, &manifest1.content)?;
    std::fs::write(&file2, &manifest2.content)?;

    let diff_cmd = if cfg!(windows) { "fc" } else { "diff" };

    let output = std::process::Command::new(diff_cmd)
        .arg(&file1)
        .arg(&file2)
        .output()?;

    println!("\n{}", String::from_utf8_lossy(&output.stdout));

    Ok(())
}
