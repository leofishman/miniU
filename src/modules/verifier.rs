use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VerificationResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_tests(command_str: &str) -> Result<VerificationResult, String> {
    // Basic parser for command string (e.g. "cargo test")
    let parts: Vec<&str> = command_str.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Command is empty".into());
    }

    let program = parts[0];
    let args = &parts[1..];

    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute '{}': {}", command_str, e))?;

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(VerificationResult {
        success,
        stdout,
        stderr,
    })
}
