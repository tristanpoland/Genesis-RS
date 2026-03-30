//! Display kit documentation (MANUAL.md).

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::EnvName;
use genesis_env::Environment;

/// Show the kit's MANUAL.md for an environment.
///
/// Corresponds to Perl's `Genesis::Commands::Info::kit_manual()`.
pub async fn execute(
    env_name: &str,
    raw: bool,
    pager: bool,
) -> Result<()> {
    let env_name = EnvName::new(env_name).context("Invalid environment name")?;
    let env_dir = std::path::Path::new(".").join(env_name.to_string());

    if !env_dir.exists() {
        bail!("Environment directory not found: {:?}", env_dir);
    }

    let env = Environment::load(&env_dir).context("Failed to load environment")?;
    let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);

    if !kit_dir.exists() {
        bail!("Kit not found at {:?}. Run 'genesis download {}' first", kit_dir, env.kit.name);
    }

    // Look for MANUAL.md, README.md, or docs/
    let candidates = vec![
        kit_dir.join("MANUAL.md"),
        kit_dir.join("README.md"),
        kit_dir.join("docs").join("MANUAL.md"),
        kit_dir.join("docs").join("README.md"),
    ];

    let manual_file = candidates.iter()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow::anyhow!(
            "No manual found for kit {} v{}. Checked: MANUAL.md, README.md, docs/",
            env.kit.name, env.kit.version
        ))?;

    let content = std::fs::read_to_string(manual_file)
        .context("Failed to read kit manual")?;

    if raw || !std::env::var("TERM").is_ok() {
        print!("{}", content);
        return Ok(());
    }

    if pager {
        // Try to use a pager: $PAGER, less, or more
        let pager_cmd = std::env::var("PAGER")
            .unwrap_or_else(|_| "less".to_string());

        use std::io::Write;
        let mut child = std::process::Command::new(&pager_cmd)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .or_else(|_| {
                // Fallback to just printing if pager fails
                Err(anyhow::anyhow!("pager unavailable"))
            });

        match child {
            Ok(ref mut c) => {
                if let Some(stdin) = c.stdin.take() {
                    let mut stdin = stdin;
                    let _ = stdin.write_all(content.as_bytes());
                }
                let _ = c.wait();
                return Ok(());
            }
            Err(_) => {}
        }
    }

    // Print the manual with a header
    println!("{} - {} v{}", "Kit Manual".cyan().bold(), env.kit.name.cyan(), env.kit.version);
    println!("{}", "=".repeat(60));
    println!();
    print!("{}", content);

    Ok(())
}
