//! Sandbox execution types.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Supported languages for sandbox execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    JavaScript,
    Shell,
    Rust,
}

impl Language {
    /// Default Docker image for this language.
    pub fn default_image(&self) -> &'static str {
        match self {
            Self::Python => "python:3.11-slim",
            Self::JavaScript => "node:20-slim",
            Self::Shell => "alpine:3.19",
            Self::Rust => "rust:1.75-slim",
        }
    }

    /// File extension for this language.
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::Shell => "sh",
            Self::Rust => "rs",
        }
    }

    /// Command to execute a script file.
    pub fn run_command(&self, filename: &str) -> Vec<String> {
        match self {
            Self::Python => vec!["python".into(), filename.into()],
            Self::JavaScript => vec!["node".into(), filename.into()],
            Self::Shell => vec!["sh".into(), filename.into()],
            Self::Rust => vec!["rustc".into(), filename.into(), "-o".into(), "/tmp/out".into()],
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Python => write!(f, "python"),
            Self::JavaScript => write!(f, "javascript"),
            Self::Shell => write!(f, "shell"),
            Self::Rust => write!(f, "rust"),
        }
    }
}

/// Configuration for sandbox execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Docker image to use (overrides language default)
    pub image: Option<String>,
    /// Memory limit in bytes (default: 256MB)
    pub memory_limit: u64,
    /// CPU quota (default: 1.0 = 100% of one core)
    pub cpu_quota: f64,
    /// Network access enabled (default: false)
    pub network_enabled: bool,
    /// Maximum execution time
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    /// Working directory inside container
    pub workdir: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: None,
            memory_limit: 256 * 1024 * 1024, // 256MB
            cpu_quota: 1.0,
            network_enabled: false,
            timeout: Duration::from_secs(60),
            workdir: "/workspace".into(),
        }
    }
}

/// Result of sandbox code execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    #[serde(with = "humantime_serde")]
    pub duration: Duration,
    /// Whether execution was killed due to timeout
    pub timed_out: bool,
}

impl ExecutionResult {
    /// Check if execution succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }
}

/// A single attempt at executing code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAttempt {
    /// The code that was executed
    pub code: String,
    /// Language used
    pub language: Language,
    /// Execution result
    pub result: ExecutionResult,
    /// AI reflection on the error (if failed)
    pub reflection: Option<String>,
    /// Timestamp of attempt
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_default_image() {
        assert_eq!(Language::Python.default_image(), "python:3.11-slim");
        assert_eq!(Language::JavaScript.default_image(), "node:20-slim");
        assert_eq!(Language::Shell.default_image(), "alpine:3.19");
        assert_eq!(Language::Rust.default_image(), "rust:1.75-slim");
    }

    #[test]
    fn language_file_extension() {
        assert_eq!(Language::Python.file_extension(), "py");
        assert_eq!(Language::JavaScript.file_extension(), "js");
        assert_eq!(Language::Shell.file_extension(), "sh");
        assert_eq!(Language::Rust.file_extension(), "rs");
    }

    #[test]
    fn language_display() {
        assert_eq!(Language::Python.to_string(), "python");
        assert_eq!(Language::JavaScript.to_string(), "javascript");
    }

    #[test]
    fn sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.memory_limit, 256 * 1024 * 1024);
        assert!((config.cpu_quota - 1.0).abs() < f64::EPSILON);
        assert!(!config.network_enabled);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn execution_result_success() {
        let result = ExecutionResult {
            exit_code: 0,
            stdout: "output".into(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
            timed_out: false,
        };
        assert!(result.success());

        let failed = ExecutionResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".into(),
            duration: Duration::from_millis(50),
            timed_out: false,
        };
        assert!(!failed.success());

        let timeout = ExecutionResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::from_secs(60),
            timed_out: true,
        };
        assert!(!timeout.success());
    }

    #[test]
    fn language_serialization() {
        let json = serde_json::to_string(&Language::Python).unwrap();
        assert_eq!(json, "\"python\"");

        let parsed: Language = serde_json::from_str("\"javascript\"").unwrap();
        assert_eq!(parsed, Language::JavaScript);
    }
}
