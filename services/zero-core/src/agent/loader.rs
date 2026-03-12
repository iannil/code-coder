//! Agent Configuration Loader
//!
//! Loads agent definitions from YAML configuration files and Markdown prompts.
//!
//! ## Configuration Structure
//!
//! Agents are defined in two files:
//! - `~/.codecoder/agents/<name>.yaml` - Agent configuration
//! - `~/.codecoder/prompts/<name>.md` - Agent system prompt
//!
//! ## Example YAML Configuration
//!
//! ```yaml
//! name: build
//! mode: primary
//! description: "Main development agent"
//! model:
//!   provider_id: anthropic
//!   model_id: claude-opus-4-5
//!   temperature: 0.7
//!   max_tokens: 128000
//! prompt: build.md
//! permission:
//!   default: allow
//!   question: allow
//!   plan_enter: allow
//! auto_approve:
//!   enabled: true
//!   risk_threshold: medium
//! observer:
//!   can_watch: [code, self]
//!   contribute_to_consensus: true
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Prompt file not found: {0}")]
    PromptNotFound(PathBuf),
}

// ============================================================================
// Agent Configuration Types
// ============================================================================

/// Agent execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    /// Can be invoked by users directly
    #[default]
    Primary,
    /// Can only be invoked by other agents
    Subagent,
    /// Can be invoked by both users and agents
    All,
}

/// Permission action for a rule
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    /// Automatically allow
    #[default]
    Allow,
    /// Automatically deny
    Deny,
    /// Ask user for confirmation
    Ask,
}

/// Watcher types for Observer Network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatcherType {
    /// Code repository watching
    Code,
    /// External world (market, news, APIs)
    World,
    /// Self-reflection (behavior, decisions)
    #[serde(rename = "self")]
    Self_,
    /// Meta-observation (system health)
    Meta,
}

/// Risk threshold for auto-approve
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RiskThreshold {
    /// Only completely safe operations
    Safe,
    /// Low-risk operations
    #[default]
    Low,
    /// Medium-risk operations
    Medium,
    /// High-risk operations (use with caution)
    High,
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Provider identifier (e.g., "anthropic", "openai")
    pub provider_id: String,
    /// Model identifier (e.g., "claude-opus-4-5")
    pub model_id: String,
    /// Sampling temperature (0.0 - 1.0)
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Maximum output tokens
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(default)]
    pub top_p: Option<f64>,
    /// Top-k sampling
    #[serde(default)]
    pub top_k: Option<u32>,
}

/// Thinking mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ThinkingMode {
    /// Thinking disabled
    Disabled,
    /// Thinking enabled with token budget
    Enabled {
        #[serde(default = "default_thinking_budget")]
        budget_tokens: u32,
    },
}

fn default_thinking_budget() -> u32 {
    10_000
}

impl Default for ThinkingMode {
    fn default() -> Self {
        ThinkingMode::Disabled
    }
}

/// Permission rule for a specific tool or action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// The permission type (e.g., "read", "write", "bash", "external_directory")
    pub permission: String,
    /// Glob pattern to match (e.g., "*.env", "packages/**")
    pub pattern: String,
    /// Action to take
    pub action: PermissionAction,
}

/// Permission configuration using nested map structure
/// Example:
/// ```yaml
/// permission:
///   default: allow
///   question: allow
///   read:
///     "*": allow
///     "*.env": ask
///   write:
///     "packages/**": ask
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PermissionConfig {
    pub rules: HashMap<String, PermissionValue>,
}

/// Permission value - either a simple action or nested patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionValue {
    /// Simple action (allow, deny, ask)
    Simple(PermissionAction),
    /// Nested patterns with actions
    Patterns(HashMap<String, PermissionAction>),
}

impl PermissionConfig {
    /// Convert to flat list of rules
    pub fn to_rules(&self) -> Vec<PermissionRule> {
        let mut rules = Vec::new();

        for (key, value) in &self.rules {
            match value {
                PermissionValue::Simple(action) => {
                    rules.push(PermissionRule {
                        permission: key.clone(),
                        pattern: "*".to_string(),
                        action: *action,
                    });
                }
                PermissionValue::Patterns(patterns) => {
                    for (pattern, action) in patterns {
                        rules.push(PermissionRule {
                            permission: key.clone(),
                            pattern: pattern.clone(),
                            action: *action,
                        });
                    }
                }
            }
        }

        rules
    }

    /// Check if a permission is allowed
    pub fn check(&self, permission: &str, pattern: &str) -> PermissionAction {
        // First check specific permission
        if let Some(value) = self.rules.get(permission) {
            match value {
                PermissionValue::Simple(action) => return *action,
                PermissionValue::Patterns(patterns) => {
                    // Check for exact match first
                    if let Some(action) = patterns.get(pattern) {
                        return *action;
                    }

                    // Collect matching patterns with their specificity
                    let mut matches: Vec<(&str, PermissionAction)> = Vec::new();

                    for (pat, action) in patterns {
                        if glob_match(pat, pattern) {
                            matches.push((pat.as_str(), *action));
                        }
                    }

                    // Sort by specificity: more specific patterns first
                    // Specificity is determined by:
                    // 1. Exact match (no wildcards) > patterns with wildcards
                    // 2. Longer patterns > shorter patterns
                    // 3. Single * > **
                    matches.sort_by(|a, b| {
                        let spec_a = pattern_specificity(a.0);
                        let spec_b = pattern_specificity(b.0);
                        spec_b.cmp(&spec_a) // Higher specificity first
                    });

                    if let Some((_, action)) = matches.first() {
                        return *action;
                    }
                }
            }
        }

        // Fall back to default
        if let Some(PermissionValue::Simple(action)) = self.rules.get("default") {
            return *action;
        }

        // Default to ask
        PermissionAction::Ask
    }
}

/// Calculate pattern specificity (higher = more specific)
fn pattern_specificity(pattern: &str) -> u32 {
    if pattern == "*" {
        return 0; // Least specific
    }

    let mut specificity: u32 = 0;

    // Patterns without wildcards are most specific
    if !pattern.contains('*') {
        specificity += 1000;
    } else if pattern.contains("**") {
        // ** is less specific than single *
        specificity += 10;
    } else {
        // Single * is more specific than **
        specificity += 50;
    }

    // Longer patterns are more specific
    specificity += pattern.len() as u32;

    specificity
}

/// Simple glob matching (supports * and **)
fn glob_match(pattern: &str, path: &str) -> bool {
    // Handle exact match
    if pattern == path {
        return true;
    }

    // Handle wildcard match
    if pattern == "*" {
        return true;
    }

    // Handle ** at the start (match any prefix)
    if pattern.starts_with("**/") {
        let suffix = &pattern[3..];
        // Check if the suffix matches at any point
        if path.ends_with(suffix) {
            return true;
        }
        // Also check for nested suffix patterns
        if let Some(pos) = path.rfind('/') {
            let filename = &path[pos + 1..];
            if glob_match(suffix, filename) {
                return true;
            }
        }
        return glob_match(suffix, path);
    }

    // Handle ** at the end (match any suffix)
    if pattern.ends_with("/**") {
        let prefix = &pattern[..pattern.len() - 3];
        return path.starts_with(prefix) || path.starts_with(&format!("{}/", prefix));
    }

    // Handle ** in the middle
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1].trim_start_matches('/');
            if prefix.is_empty() {
                return path.ends_with(suffix);
            }
            if suffix.is_empty() {
                return path.starts_with(prefix);
            }
            return path.starts_with(prefix) && path.ends_with(suffix);
        }
    }

    // Handle single * (match single segment or part of name)
    if pattern.contains('*') && !pattern.contains("**") {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return path.starts_with(parts[0]) && path.ends_with(parts[1]);
        }
    }

    false
}

/// Auto-approve configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApproveConfig {
    /// Whether auto-approve is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Allowed tools for auto-approve
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Risk threshold for auto-approval
    #[serde(default)]
    pub risk_threshold: RiskThreshold,

    /// Maximum number of auto-approvals per session
    #[serde(default)]
    pub max_approvals: Option<u32>,
}

impl Default for AutoApproveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_tools: Vec::new(),
            risk_threshold: RiskThreshold::Safe,
            max_approvals: None,
        }
    }
}

/// Observer capability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverCapability {
    /// Watcher types this agent can contribute to
    #[serde(default)]
    pub can_watch: Vec<WatcherType>,

    /// Whether observations contribute to consensus
    #[serde(default = "default_true")]
    pub contribute_to_consensus: bool,

    /// Whether to report to MetaWatch
    #[serde(default = "default_true")]
    pub report_to_meta: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ObserverCapability {
    fn default() -> Self {
        Self {
            can_watch: Vec::new(),
            contribute_to_consensus: true,
            report_to_meta: true,
        }
    }
}

/// Complete agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique agent name
    pub name: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Execution mode
    #[serde(default)]
    pub mode: AgentMode,

    /// Whether this is a native (built-in) agent
    #[serde(default)]
    pub native: bool,

    /// Whether this agent is hidden from user listing
    #[serde(default)]
    pub hidden: bool,

    /// Model configuration
    #[serde(default)]
    pub model: Option<ModelConfig>,

    /// Path to prompt file (relative to prompts directory)
    #[serde(default)]
    pub prompt: Option<String>,

    /// Loaded prompt content (populated at runtime)
    #[serde(skip)]
    pub prompt_content: Option<String>,

    /// Permission configuration
    #[serde(default)]
    pub permission: PermissionConfig,

    /// Auto-approve configuration
    #[serde(default)]
    pub auto_approve: AutoApproveConfig,

    /// Observer Network capability
    #[serde(default)]
    pub observer: ObserverCapability,

    /// Thinking mode configuration
    #[serde(default)]
    pub thinking: ThinkingMode,

    /// Display color for TUI
    #[serde(default)]
    pub color: Option<String>,

    /// Maximum execution steps
    #[serde(default)]
    pub max_steps: Option<u32>,

    /// Additional provider-specific options
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            mode: AgentMode::Primary,
            native: false,
            hidden: false,
            model: None,
            prompt: None,
            prompt_content: None,
            permission: PermissionConfig::default(),
            auto_approve: AutoApproveConfig::default(),
            observer: ObserverCapability::default(),
            thinking: ThinkingMode::Disabled,
            color: None,
            max_steps: None,
            options: HashMap::new(),
        }
    }
}

// ============================================================================
// Agent Loader
// ============================================================================

/// Configuration paths for agent loading
#[derive(Debug, Clone)]
pub struct LoaderPaths {
    /// Directory containing agent YAML files
    pub agents_dir: PathBuf,
    /// Directory containing prompt Markdown files
    pub prompts_dir: PathBuf,
}

impl LoaderPaths {
    /// Create paths with default locations (~/.codecoder/)
    pub fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let base = home.join(".codecoder");

        Self {
            agents_dir: base.join("agents"),
            prompts_dir: base.join("prompts"),
        }
    }

    /// Create paths with custom base directory
    pub fn with_base(base: impl AsRef<Path>) -> Self {
        let base = base.as_ref();
        Self {
            agents_dir: base.join("agents"),
            prompts_dir: base.join("prompts"),
        }
    }
}

/// Agent configuration loader
pub struct AgentLoader {
    paths: LoaderPaths,
}

impl AgentLoader {
    /// Create a new agent loader with default paths
    pub fn new() -> Self {
        Self {
            paths: LoaderPaths::default(),
        }
    }

    /// Create a new agent loader with custom paths
    pub fn with_paths(paths: LoaderPaths) -> Self {
        Self { paths }
    }

    /// Load a single agent by name
    pub async fn load(&self, name: &str) -> Result<AgentConfig, LoaderError> {
        let config_path = self.paths.agents_dir.join(format!("{}.yaml", name));

        if !config_path.exists() {
            return Err(LoaderError::NotFound(name.to_string()));
        }

        let content = fs::read_to_string(&config_path).await?;
        let mut config: AgentConfig = serde_yaml::from_str(&content)?;

        // Ensure name matches filename
        if config.name.is_empty() {
            config.name = name.to_string();
        }

        // Load prompt content if specified
        if let Some(ref prompt_file) = config.prompt {
            config.prompt_content = Some(self.load_prompt(prompt_file).await?);
        }

        Ok(config)
    }

    /// Load all agents from the agents directory
    pub async fn load_all(&self) -> Result<Vec<AgentConfig>, LoaderError> {
        let mut agents = Vec::new();

        // Ensure directory exists
        if !self.paths.agents_dir.exists() {
            tracing::warn!("Agents directory does not exist: {:?}", self.paths.agents_dir);
            return Ok(agents);
        }

        let mut entries = fs::read_dir(&self.paths.agents_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process YAML files
            if path.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();

                match self.load(name).await {
                    Ok(config) => agents.push(config),
                    Err(e) => {
                        tracing::warn!("Failed to load agent '{}': {}", name, e);
                    }
                }
            }
        }

        // Sort by name for consistent ordering
        agents.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(agents)
    }

    /// Load a prompt file by name
    pub async fn load_prompt(&self, name: &str) -> Result<String, LoaderError> {
        // Try with .md extension first, then as-is
        let paths = [
            self.paths.prompts_dir.join(name),
            self.paths.prompts_dir.join(format!("{}.md", name)),
        ];

        for path in paths {
            if path.exists() {
                return fs::read_to_string(&path)
                    .await
                    .map_err(LoaderError::from);
            }
        }

        Err(LoaderError::PromptNotFound(
            self.paths.prompts_dir.join(name),
        ))
    }

    /// Ensure configuration directories exist
    pub async fn ensure_dirs(&self) -> Result<(), LoaderError> {
        fs::create_dir_all(&self.paths.agents_dir).await?;
        fs::create_dir_all(&self.paths.prompts_dir).await?;
        Ok(())
    }

    /// Save an agent configuration
    pub async fn save(&self, config: &AgentConfig) -> Result<(), LoaderError> {
        self.ensure_dirs().await?;

        let config_path = self.paths.agents_dir.join(format!("{}.yaml", config.name));
        let content = serde_yaml::to_string(config)?;
        fs::write(&config_path, content).await?;

        // Save prompt if present
        if let Some(ref prompt_content) = config.prompt_content {
            if let Some(ref prompt_file) = config.prompt {
                let prompt_path = self.paths.prompts_dir.join(prompt_file);
                fs::write(&prompt_path, prompt_content).await?;
            }
        }

        Ok(())
    }
}

impl Default for AgentLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let yaml = r#"
name: test
mode: primary
description: Test agent
        "#;

        let config: AgentConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.mode, AgentMode::Primary);
        assert_eq!(config.description, Some("Test agent".to_string()));
    }

    #[test]
    fn test_parse_full_config() {
        let yaml = r#"
name: build
mode: primary
description: Main development agent
native: true
model:
  provider_id: anthropic
  model_id: claude-opus-4-5
  temperature: 0.7
  max_tokens: 128000
prompt: build.md
permission:
  default: allow
  question: allow
  read:
    "*": allow
    "*.env": ask
  write:
    "packages/**": ask
auto_approve:
  enabled: true
  allowed_tools:
    - Read
    - Glob
    - Grep
  risk_threshold: low
observer:
  can_watch:
    - code
    - self
  contribute_to_consensus: true
thinking:
  type: enabled
  budget_tokens: 20000
color: cyan
        "#;

        let config: AgentConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "build");
        assert!(config.native);
        assert!(config.model.is_some());

        let model = config.model.unwrap();
        assert_eq!(model.provider_id, "anthropic");
        assert_eq!(model.temperature, Some(0.7));

        assert!(config.auto_approve.enabled);
        assert_eq!(config.auto_approve.risk_threshold, RiskThreshold::Low);

        assert_eq!(config.observer.can_watch.len(), 2);
    }

    #[test]
    fn test_permission_check() {
        let yaml = r#"
default: allow
question: deny
read:
  "*": allow
  "*.env": ask
  "*.env.example": allow
write:
  "packages/**": ask
        "#;

        let config: PermissionConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.check("question", ""), PermissionAction::Deny);
        assert_eq!(config.check("read", "file.txt"), PermissionAction::Allow);
        assert_eq!(config.check("read", ".env"), PermissionAction::Ask);
        assert_eq!(config.check("read", ".env.example"), PermissionAction::Allow);
        assert_eq!(config.check("write", "packages/foo/bar.ts"), PermissionAction::Ask);
        assert_eq!(config.check("unknown", ""), PermissionAction::Allow);
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*.env", ".env"));
        assert!(glob_match("*.env", "test.env"));
        assert!(glob_match("packages/**", "packages/foo/bar.ts"));
        assert!(glob_match("**/*.ts", "src/foo/bar.ts"));
        assert!(!glob_match("*.env", "file.txt"));
    }

    #[test]
    fn test_thinking_mode() {
        let yaml_disabled = r#"
type: disabled
        "#;
        let mode: ThinkingMode = serde_yaml::from_str(yaml_disabled).unwrap();
        assert!(matches!(mode, ThinkingMode::Disabled));

        let yaml_enabled = r#"
type: enabled
budget_tokens: 15000
        "#;
        let mode: ThinkingMode = serde_yaml::from_str(yaml_enabled).unwrap();
        if let ThinkingMode::Enabled { budget_tokens } = mode {
            assert_eq!(budget_tokens, 15000);
        } else {
            panic!("Expected ThinkingMode::Enabled");
        }
    }
}
