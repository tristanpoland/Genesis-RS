//! Run kit hooks.

use anyhow::{Result, Context, bail};
use colored::Colorize;
use genesis_types::{EnvName, HookType};
use genesis_env::Environment;
use genesis_kit::{DevKit, HookExecutor};
use std::str::FromStr;
use std::collections::HashMap;

pub async fn execute(hook: &str, env_name: Option<&str>, args: &[String]) -> Result<()> {
    let hook_type = HookType::from_str(hook)
        .context("Invalid hook type")?;

    println!("{} hook: {}", "Running".green().bold(), hook.cyan());

    let (env_dir, kit_dir) = if let Some(name) = env_name {
        let env_name = EnvName::new(name)?;
        let env_dir = std::path::Path::new(".").join(env_name.to_string());
        let env = Environment::load(&env_dir)?;
        let kit_dir = env_dir.join(".genesis").join("kits").join(&env.kit.name);
        (Some(env_dir), kit_dir)
    } else {
        let current = std::env::current_dir()?;
        let kit_dir = current.join(".genesis").join("kits");
        (None, kit_dir)
    };

    if !kit_dir.exists() {
        bail!("Kit directory not found: {:?}", kit_dir);
    }

    let kit = DevKit::from_directory(&kit_dir)?;

    if !kit.has_hook(hook_type) {
        bail!("Hook '{}' not found in kit", hook);
    }

    let mut env_vars = HashMap::new();
    for (i, arg) in args.iter().enumerate() {
        env_vars.insert(format!("ARG{}", i), arg.clone());
    }

    let executor = HookExecutor::new()
        .with_env_map(env_vars);

    let result = executor.execute(&kit, hook_type)?;

    if result.is_success() {
        println!("{}", result.stdout);
        Ok(())
    } else {
        eprintln!("{}", result.stderr);
        bail!("Hook execution failed with code {}", result.exit_code);
    }
}
