//! Remote Security Policy
//!
//! Defines which tool operations require human approval when invoked remotely.
//! This complements the local SecurityPolicy by providing access control
//! specifically for remote (non-CLI) invocations.
//!
//! # Categories
//!
//! - **Dangerous**: Operations that can modify files, execute commands, or access network
//! - **Safe**: Read-only operations that don't modify state
//! - **Moderate**: Unknown operations that default to requiring approval

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::RwLock;

// ══════════════════════════════════════════════════════════════════════════════
// Risk Level
// ══════════════════════════════════════════════════════════════════════════════

/// Risk level for a tool operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteRiskLevel {
    /// Read-only, no state modification
    Safe,
    /// Unknown, default to approval
    Moderate,
    /// Can modify files, execute commands, or access network
    Dangerous,
}

impl RemoteRiskLevel {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            RemoteRiskLevel::Safe => "safe",
            RemoteRiskLevel::Moderate => "moderate",
            RemoteRiskLevel::Dangerous => "dangerous",
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Task Context
// ══════════════════════════════════════════════════════════════════════════════

/// Context for evaluating remote policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTaskContext {
    /// Source of the request: "cli", "remote", "api", etc.
    pub source: String,
    /// User identifier for allowlist lookup
    pub user_id: String,
    /// Optional session identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl RemoteTaskContext {
    /// Create a CLI context (non-remote)
    pub fn cli() -> Self {
        Self {
            source: "cli".to_string(),
            user_id: "local".to_string(),
            session_id: None,
        }
    }

    /// Create a remote context
    pub fn remote(user_id: impl Into<String>) -> Self {
        Self {
            source: "remote".to_string(),
            user_id: user_id.into(),
            session_id: None,
        }
    }

    /// Check if this is a remote context
    pub fn is_remote(&self) -> bool {
        self.source == "remote"
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Operation Categories
// ══════════════════════════════════════════════════════════════════════════════

/// Get the set of dangerous operations
fn dangerous_operations() -> HashSet<&'static str> {
    [
        // File mutations
        "write",
        "edit",
        "patch",
        "multiedit",
        "delete",
        "move",
        "rename",
        // Shell execution
        "bash",
        "shell",
        "exec",
        "run",
        // Git mutations
        "git_push",
        "git_commit",
        "git_reset",
        "git_checkout",
        "git_branch_delete",
        "git_force_push",
        // Network operations
        "fetch",
        "curl",
        "http",
        // MCP Chrome DevTools browser operations (mutating/navigating)
        "mcp__chrome_devtools__navigate_page",
        "mcp__chrome_devtools__click",
        "mcp__chrome_devtools__fill",
        "mcp__chrome_devtools__fill_form",
        "mcp__chrome_devtools__upload_file",
        "mcp__chrome_devtools__evaluate_script",
        "mcp__chrome_devtools__drag",
        "mcp__chrome_devtools__handle_dialog",
        "mcp__chrome_devtools__new_page",
        "mcp__chrome_devtools__close_page",
        // MCP Puppeteer browser operations (mutating/navigating)
        "mcp__puppeteer__puppeteer_navigate",
        "mcp__puppeteer__puppeteer_click",
        "mcp__puppeteer__puppeteer_fill",
        "mcp__puppeteer__puppeteer_evaluate",
        // MCP Playwright browser operations (mutating/navigating)
        "mcp__playwright__browser_navigate",
        "mcp__playwright__browser_click",
        "mcp__playwright__browser_type",
        "mcp__playwright__browser_fill_form",
        "mcp__playwright__browser_file_upload",
        "mcp__playwright__browser_evaluate",
        "mcp__playwright__browser_drag",
        "mcp__playwright__browser_handle_dialog",
        "mcp__playwright__browser_press_key",
        "mcp__playwright__browser_select_option",
        "mcp__playwright__browser_close",
        "mcp__playwright__browser_tabs",
    ]
    .into_iter()
    .collect()
}

/// Get the set of safe operations
fn safe_operations() -> HashSet<&'static str> {
    [
        // Read operations
        "read",
        "view",
        "search",
        "grep",
        "find",
        "list",
        "glob",
        // Git read operations
        "git_status",
        "git_log",
        "git_diff",
        // MCP Chrome DevTools read-only operations
        "mcp__chrome_devtools__take_snapshot",
        "mcp__chrome_devtools__take_screenshot",
        "mcp__chrome_devtools__list_console_messages",
        "mcp__chrome_devtools__list_network_requests",
        "mcp__chrome_devtools__get_network_request",
        "mcp__chrome_devtools__list_pages",
        "mcp__chrome_devtools__select_page",
        "mcp__chrome_devtools__wait_for",
        "mcp__chrome_devtools__navigate_page_history",
        "mcp__chrome_devtools__resize_page",
        "mcp__chrome_devtools__hover",
        "mcp__chrome_devtools__performance_start_trace",
        "mcp__chrome_devtools__performance_stop_trace",
        "mcp__chrome_devtools__performance_analyze_insight",
        "mcp__chrome_devtools__emulate_cpu",
        "mcp__chrome_devtools__emulate_network",
        // MCP Puppeteer read-only operations
        "mcp__puppeteer__puppeteer_screenshot",
        // MCP Playwright read-only operations
        "mcp__playwright__browser_snapshot",
        "mcp__playwright__browser_take_screenshot",
        "mcp__playwright__browser_console_messages",
        "mcp__playwright__browser_network_requests",
        "mcp__playwright__browser_wait_for",
        "mcp__playwright__browser_hover",
        "mcp__playwright__browser_resize",
        "mcp__playwright__browser_navigate_back",
        "mcp__playwright__browser_install",
        "mcp__playwright__browser_run_code",
    ]
    .into_iter()
    .collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// Remote Policy
// ══════════════════════════════════════════════════════════════════════════════

/// Remote security policy for tool access control
#[derive(Debug)]
pub struct RemotePolicy {
    /// User-specific allowlists (user_id -> allowed tools)
    allowlists: RwLock<HashMap<String, HashSet<String>>>,
    /// Path to persist allowlists
    allowlist_path: PathBuf,
    /// Cached dangerous operations set
    dangerous: HashSet<&'static str>,
    /// Cached safe operations set
    safe: HashSet<&'static str>,
}

impl Default for RemotePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl RemotePolicy {
    /// Create a new remote policy
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .map(|p| p.join("codecoder"))
            .unwrap_or_else(|| PathBuf::from(".codecoder"));

        Self {
            allowlists: RwLock::new(HashMap::new()),
            allowlist_path: config_dir.join("remote-allowlists.json"),
            dangerous: dangerous_operations(),
            safe: safe_operations(),
        }
    }

    /// Create with custom path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            allowlists: RwLock::new(HashMap::new()),
            allowlist_path: path.into(),
            dangerous: dangerous_operations(),
            safe: safe_operations(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Policy Evaluation
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if an operation requires human approval
    pub fn should_require_approval(&self, tool: &str, context: &RemoteTaskContext) -> bool {
        // Non-remote contexts don't need extra approval
        if !context.is_remote() {
            return false;
        }

        let normalized = tool.to_lowercase();

        // Check user-specific allowlist
        if let Ok(allowlists) = self.allowlists.read() {
            if let Some(user_allowed) = allowlists.get(&context.user_id) {
                if user_allowed.contains(&normalized) {
                    return false;
                }
            }
        }

        // Safe operations never need approval
        if self.safe.contains(normalized.as_str()) {
            return false;
        }

        // Dangerous operations always need approval for remote calls
        if self.dangerous.contains(normalized.as_str()) {
            return true;
        }

        // MCP tools (prefixed with mcp__) need approval by default unless explicitly safe
        if normalized.starts_with("mcp__") {
            return true;
        }

        // Unknown operations default to requiring approval for remote calls
        true
    }

    /// Check if an operation is explicitly dangerous
    pub fn is_dangerous(&self, tool: &str) -> bool {
        self.dangerous.contains(tool.to_lowercase().as_str())
    }

    /// Check if an operation is explicitly safe
    pub fn is_safe(&self, tool: &str) -> bool {
        self.safe.contains(tool.to_lowercase().as_str())
    }

    /// Get the risk level for an operation
    pub fn risk_level(&self, tool: &str) -> RemoteRiskLevel {
        let normalized = tool.to_lowercase();

        if self.safe.contains(normalized.as_str()) {
            RemoteRiskLevel::Safe
        } else if self.dangerous.contains(normalized.as_str()) {
            RemoteRiskLevel::Dangerous
        } else {
            RemoteRiskLevel::Moderate
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // User Allowlist Management
    // ─────────────────────────────────────────────────────────────────────────

    /// Load user allowlists from persistent storage
    pub fn load_allowlists(&self) -> std::io::Result<()> {
        if !self.allowlist_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.allowlist_path)?;
        let data: HashMap<String, Vec<String>> = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let mut allowlists = self.allowlists.write().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        for (user_id, tools) in data {
            allowlists.insert(user_id, tools.into_iter().collect());
        }

        Ok(())
    }

    /// Save user allowlists to persistent storage
    pub fn save_allowlists(&self) -> std::io::Result<()> {
        let allowlists = self.allowlists.read().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;

        let data: HashMap<String, Vec<String>> = allowlists
            .iter()
            .map(|(user_id, tools)| {
                (user_id.clone(), tools.iter().cloned().collect())
            })
            .collect();

        // Ensure parent directory exists
        if let Some(parent) = self.allowlist_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(&self.allowlist_path, content)
    }

    /// Add an operation to a user's allowlist
    pub fn allow_for_user(&self, user_id: &str, tool: &str) -> std::io::Result<()> {
        {
            let mut allowlists = self.allowlists.write().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;

            allowlists
                .entry(user_id.to_string())
                .or_default()
                .insert(tool.to_lowercase());
        }

        self.save_allowlists()
    }

    /// Remove an operation from a user's allowlist
    pub fn revoke_for_user(&self, user_id: &str, tool: &str) -> std::io::Result<()> {
        {
            let mut allowlists = self.allowlists.write().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;

            if let Some(user_tools) = allowlists.get_mut(user_id) {
                user_tools.remove(&tool.to_lowercase());
            }
        }

        self.save_allowlists()
    }

    /// Get a user's allowlist
    pub fn get_user_allowlist(&self, user_id: &str) -> Vec<String> {
        self.allowlists
            .read()
            .ok()
            .and_then(|allowlists| allowlists.get(user_id).cloned())
            .map(|set| set.into_iter().collect())
            .unwrap_or_default()
    }

    /// Clear a user's allowlist
    pub fn clear_user_allowlist(&self, user_id: &str) -> std::io::Result<()> {
        {
            let mut allowlists = self.allowlists.write().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
            })?;

            allowlists.remove(user_id);
        }

        self.save_allowlists()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Description Generation
    // ─────────────────────────────────────────────────────────────────────────

    /// Generate a human-readable description of why approval is needed
    pub fn describe_approval_reason(&self, tool: &str, args: Option<&serde_json::Value>) -> String {
        let normalized = tool.to_lowercase();

        match normalized.as_str() {
            "write" | "edit" | "patch" => {
                let path = args
                    .and_then(|a| a.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown path");
                format!("File modification: {path}")
            }
            "bash" | "shell" | "exec" => {
                let cmd = args
                    .and_then(|a| a.get("command"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("unknown command");
                let truncated = if cmd.len() > 100 { &cmd[..100] } else { cmd };
                format!("Shell command: {truncated}")
            }
            "git_push" => "Git push operation will modify remote repository".to_string(),
            "git_commit" => "Git commit will modify repository history".to_string(),
            "delete" => {
                let path = args
                    .and_then(|a| a.get("path"))
                    .and_then(|p| p.as_str())
                    .unwrap_or("unknown path");
                format!("Delete file: {path}")
            }
            _ if normalized.starts_with("mcp__") => {
                // MCP tool generic description
                if normalized.contains("navigate") {
                    let url = args
                        .and_then(|a| a.get("url"))
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown URL");
                    format!("Navigate browser to: {url}")
                } else if normalized.contains("click") {
                    let element = args
                        .and_then(|a| {
                            a.get("element")
                                .or_else(|| a.get("ref"))
                                .or_else(|| a.get("selector"))
                        })
                        .and_then(|e| e.as_str())
                        .unwrap_or("unknown");
                    format!("Click element: {element}")
                } else if normalized.contains("fill") || normalized.contains("type") {
                    let element = args
                        .and_then(|a| {
                            a.get("element")
                                .or_else(|| a.get("ref"))
                                .or_else(|| a.get("selector"))
                        })
                        .and_then(|e| e.as_str())
                        .unwrap_or("unknown");
                    format!("Type text into: {element}")
                } else if normalized.contains("evaluate") || normalized.contains("run_code") {
                    "Execute JavaScript in browser".to_string()
                } else {
                    let action = normalized
                        .strip_prefix("mcp__chrome_devtools__")
                        .or_else(|| normalized.strip_prefix("mcp__puppeteer__puppeteer_"))
                        .or_else(|| normalized.strip_prefix("mcp__playwright__browser_"))
                        .unwrap_or(&normalized);
                    format!("Browser {action} operation")
                }
            }
            _ => format!("Operation \"{tool}\" requested"),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_policy() -> RemotePolicy {
        let temp = TempDir::new().unwrap();
        RemotePolicy::with_path(temp.path().join("allowlists.json"))
    }

    #[test]
    fn risk_level_safe_operations() {
        let policy = test_policy();
        assert_eq!(policy.risk_level("read"), RemoteRiskLevel::Safe);
        assert_eq!(policy.risk_level("grep"), RemoteRiskLevel::Safe);
    }

    #[test]
    fn risk_level_dangerous_operations() {
        let policy = test_policy();
        assert_eq!(policy.risk_level("write"), RemoteRiskLevel::Dangerous);
        assert_eq!(policy.risk_level("bash"), RemoteRiskLevel::Dangerous);
    }

    #[test]
    fn cli_context_never_requires_approval() {
        let policy = test_policy();
        let cli = RemoteTaskContext::cli();
        assert!(!policy.should_require_approval("write", &cli));
    }

    #[test]
    fn remote_dangerous_requires_approval() {
        let policy = test_policy();
        let remote = RemoteTaskContext::remote("user123");
        assert!(policy.should_require_approval("write", &remote));
    }

    #[test]
    fn allowlist_bypasses_approval() {
        let policy = test_policy();
        let remote = RemoteTaskContext::remote("user123");

        assert!(policy.should_require_approval("write", &remote));
        policy.allow_for_user("user123", "write").unwrap();
        assert!(!policy.should_require_approval("write", &remote));
    }
}
