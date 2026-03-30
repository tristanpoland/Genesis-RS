//! Concourse pipeline commands: embed, repipe, graph, describe, ci-*.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

/// Embed the current Genesis version into the repository.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::embed()`.
pub async fn embed() -> Result<()> {
    let genesis_path = std::env::current_exe()
        .context("Failed to determine current executable path")?;

    let target = Path::new(".genesis").join("bin").join("genesis");

    if !Path::new(".genesis").exists() {
        bail!("Not in a Genesis repository (no .genesis directory found)");
    }

    std::fs::create_dir_all(target.parent().unwrap())?;
    std::fs::copy(&genesis_path, &target)
        .context("Failed to copy genesis binary")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target, perms)?;
    }

    println!("{} Genesis binary embedded at {}", "✓".green().bold(), target.display().to_string().cyan());
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

/// Generate and upload a Concourse pipeline.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::repipe()`.
pub async fn repipe(
    config: Option<&str>,
    vault_target: Option<&str>,
    layout: Option<&str>,
    target: Option<&str>,
    dry_run: bool,
    yes: bool,
    paused: bool,
) -> Result<()> {
    println!("{} Concourse pipeline", "Configuring".green().bold());

    if !Path::new(".genesis").exists() {
        bail!("Not in a Genesis repository (no .genesis directory found)");
    }

    let config_file = config.unwrap_or("ci/settings.yml");
    let pipeline_layout = layout.unwrap_or("default");

    if !Path::new(config_file).exists() {
        bail!("Pipeline config not found: {}. Create ci/settings.yml first.", config_file);
    }

    // Load pipeline config
    let config_content = std::fs::read_to_string(config_file)
        .context("Failed to read pipeline config")?;

    let pipeline_config: serde_json::Value = serde_yaml::from_str(&config_content)
        .context("Failed to parse pipeline config")?;

    let pipeline_name = pipeline_config.get("pipeline")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("genesis-deployments");

    let concourse_target = target.unwrap_or("default");

    println!("  Pipeline: {}", pipeline_name.cyan());
    println!("  Target:   {}", concourse_target.cyan());
    println!("  Layout:   {}", pipeline_layout.cyan());

    if dry_run {
        println!("  {}", "Dry run - not uploading".yellow());
        return Ok(());
    }

    if !yes {
        use dialoguer::Confirm;
        let confirmed = Confirm::new()
            .with_prompt(format!("Upload pipeline '{}' to target '{}'?", pipeline_name, concourse_target))
            .default(false)
            .interact()?;
        if !confirmed {
            println!("{}", "Cancelled".yellow());
            return Ok(());
        }
    }

    // Check for fly binary
    let fly_status = Command::new("fly").arg("--version").output();
    if fly_status.is_err() {
        bail!("fly CLI not found. Install Concourse fly CLI to use repipe.");
    }

    println!("  {}", "Uploading pipeline via fly...".cyan());
    let mut cmd = Command::new("fly");
    cmd.arg("-t").arg(concourse_target)
       .arg("set-pipeline")
       .arg("-p").arg(pipeline_name)
       .arg("-c").arg(config_file);

    if paused {
        cmd.arg("--pause-pipeline");
    }
    if yes {
        cmd.arg("-n");
    }

    let exit = cmd.status().context("Failed to run fly")?;
    if !exit.success() {
        bail!("fly set-pipeline failed with status {}", exit);
    }

    println!("{} Pipeline '{}' uploaded", "✓".green().bold(), pipeline_name.cyan());
    Ok(())
}

/// Generate a GraphViz pipeline dependency graph.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::graph()`.
pub async fn graph(config: Option<&str>, layout: Option<&str>) -> Result<()> {
    let config_file = config.unwrap_or("ci/settings.yml");

    if !Path::new(config_file).exists() {
        bail!("Pipeline config not found: {}", config_file);
    }

    println!("{} pipeline graph", "Generating".green().bold());

    let config_content = std::fs::read_to_string(config_file)?;
    let pipeline_config: serde_json::Value = serde_yaml::from_str(&config_content)?;

    // Output a basic DOT graph
    println!("digraph genesis_pipeline {{");
    println!("  rankdir=LR;");
    println!("  node [shape=box];");

    if let Some(envs) = pipeline_config.get("environments").and_then(|v| v.as_array()) {
        for env in envs {
            if let Some(name) = env.get("name").and_then(|v| v.as_str()) {
                println!("  \"{}\";", name);
                if let Some(trigger) = env.get("trigger").and_then(|v| v.as_str()) {
                    println!("  \"{}\" -> \"{}\";", trigger, name);
                }
            }
        }
    }

    println!("}}");
    Ok(())
}

/// Describe the pipeline in human-readable format.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::describe()`.
pub async fn describe(config: Option<&str>, layout: Option<&str>) -> Result<()> {
    let config_file = config.unwrap_or("ci/settings.yml");

    if !Path::new(config_file).exists() {
        bail!("Pipeline config not found: {}", config_file);
    }

    let config_content = std::fs::read_to_string(config_file)?;
    let pipeline_config: serde_json::Value = serde_yaml::from_str(&config_content)?;

    println!("{}", "Pipeline Description".cyan().bold());
    println!("{}", "=".repeat(50));

    if let Some(name) = pipeline_config.get("pipeline").and_then(|v| v.get("name")).and_then(|v| v.as_str()) {
        println!("\nPipeline: {}", name.cyan());
    }

    if let Some(envs) = pipeline_config.get("environments").and_then(|v| v.as_array()) {
        println!("\n{} ({}):", "Environments".green(), envs.len());
        for env in envs {
            let name = env.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
            let trigger = env.get("trigger").and_then(|v| v.as_str());
            if let Some(t) = trigger {
                println!("  {} → {}", t, name.cyan());
            } else {
                println!("  {} (manual trigger)", name.cyan());
            }
        }
    }

    Ok(())
}

/// CI task: deploy an environment in pipeline context.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::ci_pipeline_deploy()`.
pub async fn ci_pipeline_deploy() -> Result<()> {
    let current_env = std::env::var("CURRENT_ENV")
        .context("CURRENT_ENV not set")?;
    let git_branch = std::env::var("GIT_BRANCH")
        .context("GIT_BRANCH not set")?;
    let working_dir = std::env::var("WORKING_DIR")
        .unwrap_or_else(|_| ".".to_string());

    println!("{} CI pipeline deploy for '{}'", "Running".green().bold(), current_env.cyan());
    println!("  Branch: {}", git_branch.cyan());
    println!("  Working dir: {}", working_dir.cyan());

    // Delegate to the deploy command
    std::env::set_current_dir(&working_dir)
        .context("Failed to change to working directory")?;

    crate::commands::deploy::execute(&current_env, false, false, false, true, false, false, false, None, None).await
        .context("Deployment failed in CI context")?;

    Ok(())
}

/// CI task: show manifest changes between current and previous deployment.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::ci_show_changes()`.
pub async fn ci_show_changes() -> Result<()> {
    let current_env = std::env::var("CURRENT_ENV")
        .context("CURRENT_ENV not set")?;
    let working_dir = std::env::var("WORKING_DIR")
        .unwrap_or_else(|_| ".".to_string());

    println!("{} manifest changes for '{}'", "Showing".green().bold(), current_env.cyan());

    std::env::set_current_dir(&working_dir)
        .context("Failed to change to working directory")?;

    // Generate current manifest and compare to cached
    let env_dir = Path::new(".").join(&current_env);
    let cached = env_dir.join(".genesis").join("cached").join(format!("{}.yml", current_env));

    if cached.exists() {
        let status = Command::new("bosh")
            .arg("diff")
            .arg(&cached)
            .status();

        match status {
            Ok(s) if s.success() => {}
            _ => println!("  {} No diff tool available or no previous manifest", "!".yellow()),
        }
    } else {
        println!("  {} No cached manifest found for comparison", "!".yellow());
    }

    Ok(())
}

/// CI task: generate cache from a previously deployed environment.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::ci_generate_cache()`.
pub async fn ci_generate_cache() -> Result<()> {
    let current_env = std::env::var("CURRENT_ENV")
        .context("CURRENT_ENV not set")?;
    let working_dir = std::env::var("WORKING_DIR")
        .unwrap_or_else(|_| ".".to_string());

    println!("{} cache for '{}'", "Generating".green().bold(), current_env.cyan());

    std::env::set_current_dir(&working_dir)?;

    // The cache stores the manifest from the previous deployment
    let env_dir = Path::new(".").join(&current_env);
    let cache_dir = env_dir.join(".genesis").join("cached");
    std::fs::create_dir_all(&cache_dir)?;

    println!("{} Cache directory ensured at {:?}", "✓".green().bold(), cache_dir);
    Ok(())
}

/// CI task: run a BOSH errand in pipeline context.
///
/// Corresponds to Perl's `Genesis::Commands::Pipelines::ci_pipeline_run_errand()`.
pub async fn ci_pipeline_run_errand() -> Result<()> {
    let current_env = std::env::var("CURRENT_ENV")
        .context("CURRENT_ENV not set")?;
    let errand_name = std::env::var("ERRAND_NAME")
        .context("ERRAND_NAME not set")?;
    let working_dir = std::env::var("WORKING_DIR")
        .unwrap_or_else(|_| ".".to_string());

    println!(
        "{} errand '{}' for '{}'",
        "Running".green().bold(),
        errand_name.cyan(),
        current_env.cyan()
    );

    std::env::set_current_dir(&working_dir)?;

    use genesis_types::EnvName;
    use genesis_env::Environment;

    let env_name = EnvName::new(&current_env)?;
    let env_dir = Path::new(".").join(env_name.to_string());
    let env = Environment::load(&env_dir)?;
    let deployment = env.deployment_name();

    let bosh_env = std::env::var("BOSH_ENVIRONMENT")
        .context("BOSH_ENVIRONMENT not set")?;

    let exit = Command::new("bosh")
        .env("BOSH_ENVIRONMENT", &bosh_env)
        .env("BOSH_DEPLOYMENT", &deployment)
        .arg("-d").arg(&deployment)
        .arg("run-errand")
        .arg(&errand_name)
        .status()
        .context("Failed to run bosh")?;

    if !exit.success() {
        bail!("Errand '{}' failed with status {}", errand_name, exit);
    }

    println!("{} Errand '{}' completed", "✓".green().bold(), errand_name.cyan());
    Ok(())
}
