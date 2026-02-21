//! Shell command execution tool.
//!
//! Executes shell commands with security sandboxing:
//! - Command allowlist enforcement
//! - Environment variable filtering
//! - Timeout handling
//! - Rate limiting

use crate::security::SecurityPolicy;
use crate::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

/// Default command timeout in seconds.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Environment variables to always filter out.
const FILTERED_ENV_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GOOGLE_API_KEY",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_ACCESS_KEY_ID",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "SSH_AUTH_SOCK",
    "GPG_AGENT_INFO",
];

/// Shell command execution tool.
pub struct ShellTool {
    security: Arc<SecurityPolicy>,
    timeout: Duration,
    env_filter: HashMap<String, String>,
}

impl ShellTool {
    /// Create a new shell tool with the given security policy.
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self {
            security,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            env_filter: HashMap::new(),
        }
    }

    /// Set the command timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Add environment variables to pass to commands.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_filter.insert(key.into(), value.into());
        self
    }

    /// Build a filtered environment for command execution.
    fn build_env(&self) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = std::env::vars()
            .filter(|(k, _)| !FILTERED_ENV_VARS.contains(&k.as_str()))
            .collect();

        // Add user-specified overrides
        env.extend(self.env_filter.clone());

        env
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute shell commands in a sandboxed environment. Commands are validated \
        against an allowlist and rate-limited. Use for running build commands, \
        git operations, file listings, and other safe operations."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Command timeout in seconds (default: 30)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // Check autonomy level
        if !self.security.can_act() {
            return Ok(ToolResult::failure("Action blocked: autonomy is read-only"));
        }

        // Check rate limit
        if !self.security.record_action() {
            return Ok(ToolResult::failure("Action blocked: rate limit exceeded"));
        }

        // Parse command
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        // Validate command against allowlist
        if !self.security.is_command_allowed(command) {
            return Ok(ToolResult::failure(format!(
                "Command not allowed: {}",
                command.split_whitespace().next().unwrap_or("(empty)")
            )));
        }

        // Parse optional timeout
        #[allow(clippy::cast_possible_truncation)]
        let timeout = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .map(Duration::from_secs)
            .unwrap_or(self.timeout);

        // Build filtered environment
        let env = self.build_env();

        // Execute command
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .env_clear()
            .envs(env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&self.security.workspace_dir);

        let result = tokio::time::timeout(timeout, cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    let mut out = stdout.to_string();
                    if !stderr.is_empty() {
                        out.push_str("\n[stderr]\n");
                        out.push_str(&stderr);
                    }
                    Ok(ToolResult::success(out))
                } else {
                    let code = output.status.code().unwrap_or(-1);
                    Ok(ToolResult::failure_with_output(
                        stdout.to_string(),
                        format!("Command failed with exit code {}: {}", code, stderr.trim()),
                    ))
                }
            }
            Ok(Err(e)) => Ok(ToolResult::failure(format!("Failed to execute command: {e}"))),
            Err(_) => Ok(ToolResult::failure(format!(
                "Command timed out after {} seconds",
                timeout.as_secs()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tool() -> ShellTool {
        let security = Arc::new(SecurityPolicy::default());
        ShellTool::new(security)
    }

    #[test]
    fn name_and_schema() {
        let tool = test_tool();
        assert_eq!(tool.name(), "shell");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["command"].is_object());
    }

    #[tokio::test]
    async fn execute_ls() {
        let tool = test_tool();
        let result = tool.execute(json!({"command": "ls"})).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn execute_echo() {
        let tool = test_tool();
        let result = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn blocked_command() {
        let tool = test_tool();
        let result = tool
            .execute(json!({"command": "rm -rf /"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn missing_command() {
        let tool = test_tool();
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn timeout_handling() {
        let security = Arc::new(SecurityPolicy {
            allowed_commands: vec!["sleep".into()],
            ..SecurityPolicy::default()
        });
        let tool = ShellTool::new(security).with_timeout(Duration::from_millis(100));
        let result = tool
            .execute(json!({"command": "sleep 10"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn readonly_blocks_execution() {
        let security = Arc::new(SecurityPolicy {
            autonomy: crate::security::AutonomyLevel::ReadOnly,
            ..SecurityPolicy::default()
        });
        let tool = ShellTool::new(security);
        let result = tool.execute(json!({"command": "ls"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("read-only"));
    }

    #[test]
    fn env_filtering() {
        let tool = test_tool();
        let env = tool.build_env();
        assert!(!env.contains_key("OPENAI_API_KEY"));
        assert!(!env.contains_key("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn with_env_adds_vars() {
        let tool = test_tool().with_env("MY_VAR", "my_value");
        let env = tool.build_env();
        assert_eq!(env.get("MY_VAR"), Some(&"my_value".to_string()));
    }
}
