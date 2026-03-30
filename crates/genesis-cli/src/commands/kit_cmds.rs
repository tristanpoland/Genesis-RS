//! Kit management commands: create-kit, build-kit, decompile-kit, fetch-kit, compare-kits.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_kit::{GenesisCommunityProvider, KitProviderTrait};
use std::path::Path;

/// Create a new kit scaffold.
///
/// Corresponds to Perl's `Genesis::Commands::Kit::create_kit()`.
pub async fn create(name: &str, dev: bool, target_dir: Option<&str>) -> Result<()> {
    let dir = target_dir.unwrap_or(".");
    let kit_dir = Path::new(dir).join(name);

    if kit_dir.exists() && !dev {
        bail!("Directory already exists: {:?}", kit_dir);
    }

    println!("{} kit scaffold for '{}'", "Creating".green().bold(), name.cyan());

    std::fs::create_dir_all(&kit_dir)?;
    std::fs::create_dir_all(kit_dir.join("hooks"))?;
    std::fs::create_dir_all(kit_dir.join("manifests"))?;

    // kit.yml
    let kit_yml = format!(r#"---
name: {name}
version: 0.0.1
description: "A Genesis kit for {name}"
genesis_version_min: 3.0.0

params:
  # Define kit parameters here

features:
  # Define optional features here
"#);
    std::fs::write(kit_dir.join("kit.yml"), &kit_yml)
        .context("Failed to write kit.yml")?;

    // secrets.yml
    let secrets_yml = r#"---
# Define kit secrets here
# Example:
# ssl/ca:
#   type: x509
#   is_ca: true
#   valid_for: 10y
"#;
    std::fs::write(kit_dir.join("secrets.yml"), secrets_yml)
        .context("Failed to write secrets.yml")?;

    // hooks/new
    let new_hook = r#"#!/bin/bash
# Kit new hook - called when a new environment is created
set -eu
"#;
    std::fs::write(kit_dir.join("hooks").join("new"), new_hook)
        .context("Failed to write hooks/new")?;

    // hooks/blueprint
    let blueprint_hook = r#"#!/bin/bash
# Kit blueprint hook - returns list of YAML files to merge
set -eu
echo "manifests/base.yml"
"#;
    std::fs::write(kit_dir.join("hooks").join("blueprint"), blueprint_hook)
        .context("Failed to write hooks/blueprint")?;

    // manifests/base.yml
    let base_yml = r#"---
# Base manifest for this kit
name: (( param "What is the deployment name?" ))
"#;
    std::fs::write(kit_dir.join("manifests").join("base.yml"), base_yml)
        .context("Failed to write manifests/base.yml")?;

    // Make hook scripts executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for hook in &["new", "blueprint"] {
            let path = kit_dir.join("hooks").join(hook);
            if path.exists() {
                let mut perms = std::fs::metadata(&path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&path, perms)?;
            }
        }
    }

    println!("{} Kit scaffold created at {:?}", "✓".green().bold(), kit_dir);
    println!();
    println!("Next steps:");
    println!("  1. Edit {}", format!("{}/kit.yml", name).cyan());
    println!("  2. Add your manifests to {}", format!("{}/manifests/", name).cyan());
    println!("  3. Build with: {}", format!("genesis build-kit {}", name).cyan());

    Ok(())
}

/// Compile/build a kit into a tarball.
///
/// Corresponds to Perl's `Genesis::Commands::Kit::build_kit()`.
pub async fn build(
    kit_dir: Option<&str>,
    version: Option<&str>,
    target: Option<&str>,
    force: bool,
) -> Result<()> {
    let dir = kit_dir.unwrap_or(".");
    let kit_path = Path::new(dir);

    if !kit_path.exists() {
        bail!("Kit directory not found: {:?}", kit_path);
    }

    let kit_yml_path = kit_path.join("kit.yml");
    if !kit_yml_path.exists() {
        bail!("kit.yml not found in {:?}", kit_path);
    }

    let kit_yml: serde_json::Value = {
        let content = std::fs::read_to_string(&kit_yml_path)
            .context("Failed to read kit.yml")?;
        serde_yaml::from_str(&content).context("Failed to parse kit.yml")?
    };

    let kit_name = kit_yml.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("kit.yml missing 'name' field"))?;

    let kit_version = version
        .or_else(|| kit_yml.get("version").and_then(|v| v.as_str()))
        .unwrap_or("0.0.1");

    let output_name = format!("{}-{}.tar.gz", kit_name, kit_version);
    let output_path = Path::new(target.unwrap_or(".")).join(&output_name);

    if output_path.exists() && !force {
        bail!("Output file already exists: {:?}. Use --force to overwrite.", output_path);
    }

    println!("{} {} v{}", "Building kit".green().bold(), kit_name.cyan(), kit_version.cyan());

    // Create tarball using tar command
    let status = std::process::Command::new("tar")
        .arg("czf")
        .arg(&output_path)
        .arg("--transform")
        .arg(format!("s,^\\.,{}-{},", kit_name, kit_version))
        .arg("-C")
        .arg(kit_path.parent().unwrap_or(Path::new(".")))
        .arg(kit_path.file_name().unwrap_or(std::ffi::OsStr::new(".")))
        .status()
        .context("Failed to run tar. Is it installed?")?;

    if !status.success() {
        bail!("tar failed with status {}", status);
    }

    println!("{} Kit built: {}", "✓".green().bold(), output_path.display().to_string().cyan());
    Ok(())
}

/// Decompile (extract) a kit tarball to a dev/ directory.
///
/// Corresponds to Perl's `Genesis::Commands::Kit::decompile_kit()`.
pub async fn decompile(
    kit_source: Option<&str>,
    target_dir: Option<&str>,
    force: bool,
) -> Result<()> {
    let dest = target_dir.unwrap_or("dev");
    let dest_path = Path::new(dest);

    if dest_path.exists() && !force {
        bail!("Target directory already exists: {:?}. Use --force to overwrite.", dest_path);
    }

    let source = kit_source.unwrap_or(".");

    println!("{} kit to {}", "Decompiling".green().bold(), dest.cyan());

    // Determine if source is a file or directory
    let source_path = Path::new(source);
    if source_path.is_file() {
        // Extract tarball
        if dest_path.exists() {
            std::fs::remove_dir_all(dest_path)?;
        }
        std::fs::create_dir_all(dest_path)?;

        let status = std::process::Command::new("tar")
            .arg("xzf")
            .arg(source_path)
            .arg("-C")
            .arg(dest_path)
            .arg("--strip-components=1")
            .status()
            .context("Failed to run tar")?;

        if !status.success() {
            bail!("tar extraction failed with status {}", status);
        }
    } else if source_path.is_dir() {
        // Copy directory
        if dest_path.exists() {
            std::fs::remove_dir_all(dest_path)?;
        }
        copy_dir(source_path, dest_path)?;
    } else {
        bail!("Kit source not found: {}", source);
    }

    println!("{} Kit decompiled to {:?}", "✓".green().bold(), dest_path);
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

/// Download/fetch a kit from the Genesis community or GitHub.
///
/// Corresponds to Perl's `Genesis::Commands::Kit::fetch_kit()`.
pub async fn fetch(
    kit_name: &str,
    version: Option<&str>,
    output_dir: &str,
    as_dev: bool,
    force: bool,
) -> Result<()> {
    println!(
        "{} kit {} {}",
        "Fetching".green().bold(),
        kit_name.cyan(),
        version.map(|v| format!("v{}", v)).unwrap_or_else(|| "(latest)".to_string()).cyan()
    );

    let provider = GenesisCommunityProvider::new(None)?;

    // Determine version to fetch
    let semver = if let Some(v) = version {
        genesis_types::SemVer::parse(v).context("Invalid version")?
    } else {
        provider.latest_version(kit_name).await
            .context("Failed to fetch latest version")?
    };

    // Download the kit tarball
    let tarball_name = format!("{}-{}.tar.gz", kit_name, semver);
    let out_path = if as_dev {
        std::path::PathBuf::from("dev")
    } else {
        Path::new(output_dir).join(&tarball_name)
    };

    if out_path.exists() && !force {
        bail!("Output already exists: {:?}. Use --force to overwrite.", out_path);
    }

    let install_dir = Path::new(output_dir);
    provider.install_kit(kit_name, &semver, install_dir).await
        .context("Failed to download kit")?;

    if as_dev {
        // Decompile downloaded kit into dev/
        let tarball_path = Path::new(output_dir).join(&tarball_name);
        if tarball_path.exists() {
            decompile(
                Some(tarball_path.to_str().unwrap()),
                Some("dev"),
                force,
            ).await?;
        }
    }

    println!("{} Fetched {} v{}", "✓".green().bold(), kit_name.cyan(), semver.to_string().cyan());
    Ok(())
}

/// Compare two kit versions.
///
/// Corresponds to Perl's `Genesis::Commands::Kit::compare_kits()`.
pub async fn compare(
    kit_name: &str,
    version1: Option<&str>,
    version2: Option<&str>,
    show_unchanged: bool,
) -> Result<()> {
    println!("{} kit versions for {}", "Comparing".green().bold(), kit_name.cyan());

    let provider = GenesisCommunityProvider::new(None)?;
    let versions = provider.list_versions(kit_name).await
        .context("Failed to list kit versions")?;

    if versions.is_empty() {
        bail!("No versions found for kit '{}'", kit_name);
    }

    let v1 = if let Some(v) = version1 {
        genesis_types::SemVer::parse(v)?
    } else if versions.len() >= 2 {
        versions[1].clone()
    } else {
        bail!("Need at least two versions to compare")
    };

    let v2 = if let Some(v) = version2 {
        genesis_types::SemVer::parse(v)?
    } else {
        versions[0].clone()
    };

    println!("  From: {} v{}", kit_name.cyan(), v1.to_string().yellow());
    println!("  To:   {} v{}", kit_name.cyan(), v2.to_string().green());
    println!();
    println!("  {} Kit comparison requires downloaded kit files.", "Note:".yellow());
    println!("  Run `genesis fetch-kit {} --version {}` and `genesis fetch-kit {} --version {}`",
             kit_name, v1, kit_name, v2);
    println!("  then compare manually or use diff tools.");

    Ok(())
}
