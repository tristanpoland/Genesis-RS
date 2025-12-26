//! Edit environment files.

use anyhow::{Result, Context, bail};
use genesis_types::EnvName;
use std::process::Command;

pub async fn execute(env_name: &str, file: Option<&str>) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;

    let env_dir = std::path::Path::new(".").join(env_name.to_string());
    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let file_path = if let Some(f) = file {
        env_dir.join(f)
    } else {
        env_dir.join("env.yml")
    };

    if !file_path.exists() {
        bail!("File not found: {:?}", file_path);
    }

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

    Command::new(&editor)
        .arg(&file_path)
        .status()
        .context(format!("Failed to open editor: {}", editor))?;

    Ok(())
}
