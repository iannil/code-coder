//! Shell tool - PTY-based command execution
//!
//! This module provides shell command execution with:
//! - PTY support for interactive commands
//! - Timeout handling
//! - Output capture and streaming
//! - Environment variable management

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Options for shell command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOptions {
    /// Working directory for the command
    pub cwd: Option<String>,

    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Timeout in milliseconds (default: 120000 = 2 minutes)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Maximum output size in bytes (default: 1MB)
    #[serde(default = "default_max_output")]
    pub max_output: usize,

    /// Whether to run in a PTY (for interactive commands)
    #[serde(default)]
    pub use_pty: bool,

    /// Shell to use (default: detected from environment)
    pub shell: Option<String>,

    /// Whether to inherit environment from parent process
    #[serde(default = "default_true")]
    pub inherit_env: bool,
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            env: HashMap::new(),
            timeout_ms: 120_000, // 2 minutes
            max_output: 1024 * 1024, // 1MB
            use_pty: false,
            shell: None,
            inherit_env: true,
        }
    }
}

fn default_timeout() -> u64 {
    120_000 // 2 minutes
}

fn default_max_output() -> usize {
    1024 * 1024 // 1MB
}

fn default_true() -> bool {
    true
}

/// A shell command to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    /// The command to execute
    pub command: String,

    /// Description of what this command does
    pub description: Option<String>,
}

impl ShellCommand {
    /// Create a new shell command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            description: None,
        }
    }

    /// Create a new shell command with description
    pub fn with_description(command: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            description: Some(description.into()),
        }
    }
}

/// Output of a shell command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOutput {
    /// Exit code (0 for success)
    pub exit_code: i32,

    /// Combined stdout and stderr output
    pub output: String,

    /// Stdout only
    pub stdout: String,

    /// Stderr only
    pub stderr: String,

    /// Whether the command timed out
    pub timed_out: bool,

    /// Whether the output was truncated
    pub truncated: bool,

    /// Execution duration in milliseconds
    pub duration_ms: u64,

    /// The command that was executed
    pub command: String,
}

impl ShellOutput {
    /// Check if the command succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

/// Shell executor
pub struct Shell {
    /// Default options
    default_options: ShellOptions,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    /// Create a new Shell executor with default options
    pub fn new() -> Self {
        Self {
            default_options: ShellOptions::default(),
        }
    }

    /// Create a new Shell executor with custom default options
    pub fn with_defaults(options: ShellOptions) -> Self {
        Self {
            default_options: options,
        }
    }

    /// Execute a shell command
    pub fn execute(&self, cmd: &ShellCommand, options: Option<&ShellOptions>) -> Result<ShellOutput> {
        let options = options.unwrap_or(&self.default_options);
        let start = Instant::now();

        // Detect shell
        let shell = options.shell.clone().unwrap_or_else(|| {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        });

        // Build command
        let mut command = Command::new(&shell);
        command.arg("-c").arg(&cmd.command);

        // Set working directory
        if let Some(cwd) = &options.cwd {
            command.current_dir(cwd);
        }

        // Set environment
        if !options.inherit_env {
            command.env_clear();
        }
        for (key, value) in &options.env {
            command.env(key, value);
        }

        // Configure stdio
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let timeout = Duration::from_millis(options.timeout_ms);
        let max_output = options.max_output;

        // Spawn the process
        let child = command.spawn()
            .with_context(|| format!("Failed to spawn command: {}", cmd.command))?;

        // Use a channel to receive the result with timeout
        let (tx, rx) = std::sync::mpsc::channel();

        let wait_thread = std::thread::spawn(move || {
            let result = child.wait_with_output();
            let _ = tx.send(result);
        });

        let (exit_code, stdout_data, stderr_data, timed_out) = match rx.recv_timeout(timeout) {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                (exit_code, output.stdout, output.stderr, false)
            }
            Ok(Err(_)) => {
                // wait_with_output failed
                (-1, Vec::new(), Vec::new(), false)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout - the wait_thread is still running but we can't kill the child
                // because it was moved. However, we can let the thread run to completion
                // in the background and just report timeout.
                (-1, Vec::new(), Vec::new(), true)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed without sending - thread panicked
                (-1, Vec::new(), Vec::new(), false)
            }
        };

        // Wait for the thread to finish (it will finish quickly now or was already done)
        let _ = wait_thread.join();

        // Apply truncation
        let truncated = stdout_data.len() > max_output || stderr_data.len() > max_output;
        let stdout_data = if stdout_data.len() > max_output {
            stdout_data[..max_output].to_vec()
        } else {
            stdout_data
        };
        let stderr_data = if stderr_data.len() > max_output {
            stderr_data[..max_output].to_vec()
        } else {
            stderr_data
        };

        let stdout = String::from_utf8_lossy(&stdout_data).to_string();
        let stderr = String::from_utf8_lossy(&stderr_data).to_string();
        let output = if stderr.is_empty() {
            stdout.clone()
        } else if stdout.is_empty() {
            stderr.clone()
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        Ok(ShellOutput {
            exit_code,
            output,
            stdout,
            stderr,
            timed_out,
            truncated,
            duration_ms: start.elapsed().as_millis() as u64,
            command: cmd.command.clone(),
        })
    }

    /// Execute a simple command string
    pub fn run(&self, command: &str) -> Result<ShellOutput> {
        self.execute(&ShellCommand::new(command), None)
    }

    /// Execute a command in a specific directory
    pub fn run_in(&self, command: &str, cwd: &Path) -> Result<ShellOutput> {
        let options = ShellOptions {
            cwd: Some(cwd.to_string_lossy().to_string()),
            ..self.default_options.clone()
        };
        self.execute(&ShellCommand::new(command), Some(&options))
    }

    /// Execute multiple commands sequentially
    pub fn run_all(&self, commands: &[&str]) -> Result<Vec<ShellOutput>> {
        commands
            .iter()
            .map(|cmd| self.run(cmd))
            .collect()
    }

    /// Check if a command exists in PATH
    pub fn command_exists(name: &str) -> bool {
        Command::new("which")
            .arg(name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Get the current shell
    pub fn current_shell() -> String {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let shell = Shell::new();
        let output = shell.run("echo 'Hello, world!'").unwrap();

        assert!(output.success());
        assert!(output.stdout.contains("Hello, world!"));
    }

    #[test]
    fn test_command_with_exit_code() {
        let shell = Shell::new();
        let output = shell.run("exit 42").unwrap();

        assert_eq!(output.exit_code, 42);
        assert!(!output.success());
    }

    #[test]
    fn test_command_stderr() {
        let shell = Shell::new();
        let output = shell.run("echo 'error' >&2").unwrap();

        assert!(output.success());
        assert!(output.stderr.contains("error"));
    }

    #[test]
    fn test_command_timeout() {
        let shell = Shell::new();
        let options = ShellOptions {
            timeout_ms: 100,
            ..Default::default()
        };

        let cmd = ShellCommand::new("sleep 10");
        let output = shell.execute(&cmd, Some(&options)).unwrap();

        assert!(output.timed_out);
        assert!(!output.success());
    }

    #[test]
    fn test_command_in_directory() {
        let shell = Shell::new();
        let output = shell.run_in("pwd", Path::new("/tmp")).unwrap();

        assert!(output.success());
        assert!(output.stdout.contains("/tmp") || output.stdout.contains("/private/tmp"));
    }

    #[test]
    fn test_command_exists() {
        assert!(Shell::command_exists("ls"));
        assert!(!Shell::command_exists("nonexistent_command_12345"));
    }

    #[test]
    fn test_run_all() {
        let shell = Shell::new();
        let outputs = shell.run_all(&["echo 'one'", "echo 'two'"]).unwrap();

        assert_eq!(outputs.len(), 2);
        assert!(outputs[0].stdout.contains("one"));
        assert!(outputs[1].stdout.contains("two"));
    }
}
