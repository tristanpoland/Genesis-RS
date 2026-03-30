//! Repository configuration commands: secrets-provider, kit-provider.

use anyhow::{Result, Context};
use colored::Colorize;
use std::path::Path;

/// Configure the Vault/secrets provider for this repository.
///
/// Corresponds to Perl's `Genesis::Commands::Repo::secrets_provider()`.
pub async fn secrets_provider(
    target: Option<&str>,
    interactive: bool,
    clear: bool,
) -> Result<()> {
    let config_dir = Path::new(".genesis");
    if !config_dir.exists() {
        anyhow::bail!("Not in a Genesis repository (no .genesis directory found)");
    }
    let config_file = config_dir.join("config");

    // Load existing config
    let config_content = if config_file.exists() {
        std::fs::read_to_string(&config_file).context("Failed to read .genesis/config")?
    } else {
        "---\n".to_string()
    };

    let mut config: serde_json::Value = serde_yaml::from_str(&config_content)
        .unwrap_or_else(|_| serde_json::json!({}));

    if clear {
        config.as_object_mut().map(|m| m.remove("vault"));
        let updated = serde_yaml::to_string(&config).context("Failed to serialize config")?;
        std::fs::write(&config_file, updated).context("Failed to write .genesis/config")?;
        println!("{} Secrets provider configuration cleared", "✓".green().bold());
        return Ok(());
    }

    let vault_target = if let Some(t) = target {
        t.to_string()
    } else if interactive {
        // Prompt for vault target
        let current = config.get("vault").and_then(|v| v.as_str()).unwrap_or("");
        println!("Current vault target: {}", if current.is_empty() { "(none)" } else { current });
        print!("Enter vault target URL or 'safe' target name: ");
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let line = stdin.lock().lines().next()
            .ok_or_else(|| anyhow::anyhow!("No input"))??;
        line.trim().to_string()
    } else {
        // Just show current config
        let current = config.get("vault").and_then(|v| v.as_str()).unwrap_or("(not set)");
        println!("Current secrets provider: {}", current.cyan());
        return Ok(());
    };

    if let Some(obj) = config.as_object_mut() {
        obj.insert("vault".to_string(), serde_json::Value::String(vault_target.clone()));
    }

    let updated = serde_yaml::to_string(&config).context("Failed to serialize config")?;
    std::fs::write(&config_file, updated).context("Failed to write .genesis/config")?;

    println!("{} Secrets provider set to: {}", "✓".green().bold(), vault_target.cyan());
    Ok(())
}

/// Configure the kit provider for this repository.
///
/// Corresponds to Perl's `Genesis::Commands::Repo::kit_provider()`.
pub async fn kit_provider(
    provider_url: Option<&str>,
    set_default: bool,
    export_config: bool,
    verbose: bool,
) -> Result<()> {
    let config_dir = Path::new(".genesis");
    if !config_dir.exists() {
        anyhow::bail!("Not in a Genesis repository (no .genesis directory found)");
    }
    let config_file = config_dir.join("config");

    let config_content = if config_file.exists() {
        std::fs::read_to_string(&config_file).context("Failed to read .genesis/config")?
    } else {
        "---\n".to_string()
    };

    let mut config: serde_json::Value = serde_yaml::from_str(&config_content)
        .unwrap_or_else(|_| serde_json::json!({}));

    if export_config {
        let kit_provider_cfg = config.get("kit_provider").cloned()
            .unwrap_or_else(|| serde_json::json!({"type": "genesis-community"}));
        let yaml = serde_yaml::to_string(&kit_provider_cfg)?;
        print!("{}", yaml);
        return Ok(());
    }

    if let Some(url) = provider_url {
        let provider_cfg = if url.contains("github.com") || url.starts_with("https://") {
            serde_json::json!({"type": "github", "url": url})
        } else if set_default {
            serde_json::json!({"type": "genesis-community"})
        } else {
            serde_json::json!({"type": "custom", "url": url})
        };

        if let Some(obj) = config.as_object_mut() {
            obj.insert("kit_provider".to_string(), provider_cfg.clone());
        }

        let updated = serde_yaml::to_string(&config)?;
        std::fs::write(&config_file, updated)?;

        println!("{} Kit provider set to: {}", "✓".green().bold(), url.cyan());
    } else {
        // Show current config
        let current = config.get("kit_provider").cloned()
            .unwrap_or_else(|| serde_json::json!({"type": "genesis-community"}));

        if verbose {
            println!("{}", "Kit Provider Configuration:".green().bold());
            let yaml = serde_yaml::to_string(&current)?;
            print!("{}", yaml);
        } else {
            let ptype = current.get("type").and_then(|v| v.as_str()).unwrap_or("genesis-community");
            println!("Kit provider: {}", ptype.cyan());
        }
    }

    Ok(())
}
