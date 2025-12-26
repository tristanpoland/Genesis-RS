//! Process execution utilities.

use genesis_types::Result;
use std::process::{Command, Output};

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

// TODO: Implement:
// - Environment variable management
// - Secret redaction
// - Timeout support
// - stdin/stdout/stderr handling
// - Background execution
