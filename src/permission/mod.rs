/// Title: Permissions module — defense-in-depth for tool calls
/// Path: src/permission/mod.rs

pub mod path_validator;
pub mod shell_classifier;

use self::path_validator::validate_path;
use self::shell_classifier::{classify_shell_command, ShellRisk};

/// Permission decision for a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allowed,
    Denied { reason: String },
    NeedsApproval { risk: String },
}

/// Aggregate permission engine.
/// Wraps path validation, shell classification, and configurable rules.
pub struct PermissionEngine {
    /// Deny reading/writing files outside these directories
    pub allowed_roots: Vec<String>,
    /// Deny reading/writing these specific paths
    pub blocked_paths: Vec<String>,
    /// Block shell commands matching these patterns
    pub blocked_shell_patterns: Vec<String>,
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self {
            allowed_roots: vec![
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ],
            blocked_paths: vec![
                // Never allow reading/writing these
                ".env".into(),
                ".env.local".into(),
            ],
            blocked_shell_patterns: vec![
                "rm -rf /".into(),
                "rm -rf ~".into(),
                "sudo ".into(),
                "mkfs".into(),
                "dd if=".into(),
                ":(){ :|:& };:".into(),
                "chmod 777 /".into(),
                "> /dev/sda".into(),
            ],
        }
    }
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluate a tool call and return a permission decision.
    pub fn evaluate(&self, tool_name: &str, tool_input: &str) -> PermissionDecision {
        match tool_name {
            "read_file" | "write_file" | "edit_file" | "list_directory" => {
                let path = try_extract_path(tool_input);
                if let Some(p) = &path {
                    self.check_path(p)
                } else {
                    PermissionDecision::NeedsApproval {
                        risk: "path not parseable".into(),
                    }
                }
            }
            "run_command" => {
                let cmd = tool_input.trim();
                let risk = classify_shell_command(cmd);
                match risk {
                    ShellRisk::Safe => PermissionDecision::Allowed,
                    ShellRisk::Dangerous => PermissionDecision::Denied {
                        reason: format!("blocked shell command: {cmd}"),
                    },
                    ShellRisk::Suspicious => PermissionDecision::NeedsApproval {
                        risk: format!("suspicious command: {}", &cmd[..cmd.len().min(80)]),
                    },
                }
            }
            _ => PermissionDecision::Allowed,
        }
    }

    fn check_path(&self, path: &str) -> PermissionDecision {
        // Check blocked paths
        for blocked in &self.blocked_paths {
            if path.contains(blocked) {
                return PermissionDecision::Denied {
                    reason: format!("path blocked: {path}"),
                };
            }
        }
        // Validate against allowed roots
        match validate_path(path, &self.allowed_roots) {
            Ok(()) => PermissionDecision::Allowed,
            Err(msg) => PermissionDecision::NeedsApproval { risk: msg },
        }
    }
}

/// Try to extract a path string from tool input (JSON or plain text).
fn try_extract_path(input: &str) -> Option<String> {
    // Try JSON first
    if let Some(path) = crate::tools::try_extract_json_field(input, "path") {
        return Some(path);
    }
    // Plain text: use as-is (trimmed)
    let trimmed = input.trim();
    if !trimmed.is_empty() {
        Some(trimmed.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_read_file_denied() {
        let engine = PermissionEngine::new();
        let result = engine.evaluate("read_file", "/etc/passwd");
        assert!(!matches!(result, PermissionDecision::Allowed),
            "reading /etc/passwd should not be auto-allowed");
    }

    #[test]
    fn test_evaluate_write_file_blocked_path() {
        let engine = PermissionEngine::default();
        let result = engine.evaluate("write_file", ".env");
        assert!(matches!(result, PermissionDecision::Denied { .. }),
            "writing .env should be denied");
    }

    #[test]
    fn test_evaluate_rm_ff_denied() {
        let engine = PermissionEngine::new();
        let result = engine.evaluate("run_command", "rm -rf /");
        assert!(matches!(result, PermissionDecision::Denied { .. }),
            "rm -rf / should be denied");
    }

    #[test]
    fn test_evaluate_safe_command_allowed() {
        let engine = PermissionEngine::new();
        let result = engine.evaluate("run_command", "ls -la");
        assert_eq!(result, PermissionDecision::Allowed);
    }

    #[test]
    fn test_evaluate_unknown_tool_allowed() {
        let engine = PermissionEngine::new();
        let result = engine.evaluate("search_web", "query");
        assert_eq!(result, PermissionDecision::Allowed);
    }
}
