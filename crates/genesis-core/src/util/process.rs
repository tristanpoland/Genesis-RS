//! Process execution utilities.

use genesis_types::Result;
use std::process::{Command, Stdio};
use std::collections::HashMap;
use std::time::Duration;

/// Execute a command synchronously.
pub fn run(command: &str, args: &[&str]) -> Result<(String, i32, String)> {
    let output = Command::new(command)
        .args(args)
        .output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

/// Execute a command with environment variables.
pub fn run_with_env(
    command: &str,
    args: &[&str],
    env_vars: &HashMap<String, String>,
) -> Result<(String, i32, String)> {
    let mut cmd = Command::new(command);
    cmd.args(args);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output()?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

/// Execute a command asynchronously.
pub async fn run_async(command: &str, args: &[&str]) -> Result<(String, i32, String)> {
    let output = tokio::process::Command::new(command)
        .args(args)
        .output()
        .await?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

/// Execute a command asynchronously with environment variables.
pub async fn run_async_with_env(
    command: &str,
    args: &[&str],
    env_vars: &HashMap<String, String>,
) -> Result<(String, i32, String)> {
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output().await?;

    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

/// Redact secrets from command output.
pub fn redact_secrets(output: &str, secrets: &[&str]) -> String {
    let mut redacted = output.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            redacted = redacted.replace(secret, "***REDACTED***");
        }
    }
    redacted
}
