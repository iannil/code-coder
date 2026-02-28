//! Hand manifest parsing from HAND.md files.
//!
//! HAND.md files use YAML frontmatter for configuration:
//!
//! ```markdown
//! ---
//! id: "market-sentinel"
//! name: "Market Sentinel"
//! schedule: "0 */30 * * * *"
//! agent: "macro"
//! enabled: true
//! autonomy:
//!   level: "crazy"
//!   unattended: true
//!   max_iterations: 5
//! decision:
//!   use_close: true
//!   web_search: true
//!   evolution: true
//! resources:
//!   max_tokens: 100000
//!   max_cost_usd: 5.0
//!   max_duration_sec: 600
//! memory_path: "hands/market-sentinel/{date}.md"
//! ---
//!
//! # Market Sentinel
//!
//! Description...
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use zero_common::config::config_dir;

/// Autonomy level for CLOSE decision framework thresholds.
///
/// Lower thresholds = more permissive (higher autonomy).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AutonomyLevel {
    /// Lunatic (90+) - Fully autonomous, no human intervention
    Lunatic,
    /// Insane (75-89) - Highly autonomous
    Insane,
    /// Crazy (60-74) - Significantly autonomous
    Crazy,
    /// Wild (40-59) - Partially autonomous
    Wild,
    /// Bold (20-39) - Cautiously autonomous
    Bold,
    /// Timid (<20) - Minimal autonomy
    Timid,
}

impl AutonomyLevel {
    /// Get CLOSE decision thresholds for this autonomy level.
    pub fn thresholds(&self) -> (f32, f32) {
        match self {
            AutonomyLevel::Lunatic => (5.0, 3.0),   // approval, caution
            AutonomyLevel::Insane => (5.5, 3.5),
            AutonomyLevel::Crazy => (6.0, 4.0),
            AutonomyLevel::Wild => (6.5, 4.5),
            AutonomyLevel::Bold => (7.0, 5.0),
            AutonomyLevel::Timid => (8.0, 6.0),
        }
    }

    /// Get the craziness score associated with this level.
    pub fn craziness_score(&self) -> u8 {
        match self {
            AutonomyLevel::Lunatic => 95,
            AutonomyLevel::Insane => 85,
            AutonomyLevel::Crazy => 75,
            AutonomyLevel::Wild => 60,
            AutonomyLevel::Bold => 40,
            AutonomyLevel::Timid => 15,
        }
    }

    /// Get description of this autonomy level.
    pub fn description(&self) -> &'static str {
        match self {
            AutonomyLevel::Lunatic => "完全自主 - 无需人工干预",
            AutonomyLevel::Insane => "高度自主 - 关键决策前通知",
            AutonomyLevel::Crazy => "显著自主 - 半自动执行",
            AutonomyLevel::Wild => "部分自主 - 仅执行简单任务",
            AutonomyLevel::Bold => "谨慎自主 - 仅执行已定义步骤",
            AutonomyLevel::Timid => "基本不自主 - 仅收集信息",
        }
    }
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        AutonomyLevel::Crazy
    }
}

/// Risk threshold for auto-approval.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskThreshold {
    /// Only auto-approve safe operations
    Safe,
    /// Auto-approve safe and low-risk operations
    Low,
    /// Auto-approve up to medium-risk operations
    Medium,
    /// Auto-approve up to high-risk operations (dangerous)
    High,
}

impl Default for RiskThreshold {
    fn default() -> Self {
        RiskThreshold::Medium
    }
}

impl RiskThreshold {
    /// Get the numeric value (0-3) for comparison.
    pub fn value(&self) -> u8 {
        match self {
            RiskThreshold::Safe => 0,
            RiskThreshold::Low => 1,
            RiskThreshold::Medium => 2,
            RiskThreshold::High => 3,
        }
    }
}

/// Auto-approve configuration for autonomous tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApproveConfig {
    /// Enable auto-approval of tool calls
    #[serde(default)]
    pub enabled: bool,

    /// Tools that are allowed to be auto-approved (whitelist)
    /// If empty, uses risk-based evaluation only
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// Maximum risk level for auto-approval
    /// Operations above this level always require manual approval
    #[serde(default)]
    pub risk_threshold: RiskThreshold,

    /// Timeout in milliseconds before auto-approving non-critical operations
    /// Only applies when unattended mode is enabled
    /// Set to 0 to disable timeout-based approval
    #[serde(default = "default_auto_approve_timeout_ms")]
    pub timeout_ms: u64,
}

impl Default for AutoApproveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_tools: Vec::new(),
            risk_threshold: RiskThreshold::Medium,
            timeout_ms: default_auto_approve_timeout_ms(),
        }
    }
}

fn default_auto_approve_timeout_ms() -> u64 {
    30000 // 30 seconds
}

/// Autonomy configuration for Hands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyConfig {
    /// Autonomy level (affects CLOSE thresholds)
    #[serde(default)]
    pub level: AutonomyLevel,

    /// Enable unattended mode (no human interaction)
    #[serde(default = "default_unattended")]
    pub unattended: bool,

    /// Maximum evolution iterations
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Auto-approve configuration for tool calls
    #[serde(default)]
    pub auto_approve: Option<AutoApproveConfig>,
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: AutonomyLevel::default(),
            unattended: true,
            max_iterations: 5,
            auto_approve: None,
        }
    }
}

fn default_unattended() -> bool {
    true
}

fn default_max_iterations() -> usize {
    5
}

/// Decision configuration using CLOSE framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionConfig {
    /// Use CLOSE decision framework for evaluation
    #[serde(default = "default_use_close")]
    pub use_close: bool,

    /// Enable web search for solutions
    #[serde(default)]
    pub web_search: bool,

    /// Enable evolution loop for problem solving
    #[serde(default)]
    pub evolution: bool,

    /// Auto-continue execution
    #[serde(default = "default_auto_continue")]
    pub auto_continue: bool,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            use_close: true,
            web_search: false,
            evolution: false,
            auto_continue: true,
        }
    }
}

fn default_use_close() -> bool {
    true
}

fn default_auto_continue() -> bool {
    true
}

/// Resource limits for Hand execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum tokens to consume
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    /// Maximum cost in USD
    #[serde(default = "default_max_cost_usd")]
    pub max_cost_usd: f64,

    /// Maximum duration in seconds
    #[serde(default = "default_max_duration_sec")]
    pub max_duration_sec: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_tokens: 100000,
            max_cost_usd: 5.0,
            max_duration_sec: 600,
        }
    }
}

fn default_max_tokens() -> usize {
    100000
}

fn default_max_cost_usd() -> f64 {
    5.0
}

fn default_max_duration_sec() -> usize {
    600
}

/// Pipeline execution mode for multi-agent hands.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PipelineMode {
    /// Execute agents sequentially, passing output from one to the next.
    /// Each agent receives the output of the previous agent as context.
    #[default]
    Sequential,

    /// Execute all agents in parallel and merge their outputs.
    /// Each agent receives the same initial context.
    Parallel,

    /// Execute agents conditionally based on previous agent's output.
    /// The next agent is selected based on the decision from CLOSE framework.
    Conditional,
}

impl PipelineMode {
    /// Get description of this pipeline mode.
    pub fn description(&self) -> &'static str {
        match self {
            PipelineMode::Sequential => "顺序执行：前一个 Agent 的输出作为下一个的输入",
            PipelineMode::Parallel => "并行执行：所有 Agent 同时执行并合并输出",
            PipelineMode::Conditional => "条件执行：根据 CLOSE 框架决策选择下一个 Agent",
        }
    }
}

/// Hand manifest parsed from HAND.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandManifest {
    /// Hand configuration from frontmatter
    #[serde(flatten)]
    pub config: HandConfig,

    /// Markdown content (description, documentation)
    pub content: String,

    /// Path to the HAND.md file
    pub path: PathBuf,

    /// Original frontmatter YAML string
    pub frontmatter: String,
}

/// Hand configuration from frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandConfig {
    /// Unique hand identifier
    pub id: String,

    /// Human-readable name
    #[serde(default = "default_name")]
    pub name: String,

    /// Semantic version
    #[serde(default = "default_version")]
    pub version: String,

    /// Cron expression (6 or 7 fields)
    pub schedule: String,

    /// Which agent to call (macro, trader, picker, etc.)
    /// Used for single-agent hands. For multi-agent pipelines, use `agents` instead.
    #[serde(default)]
    pub agent: String,

    /// List of agents for pipeline execution.
    /// When specified, `agent` field is ignored and agents are executed according to `pipeline` mode.
    #[serde(default)]
    pub agents: Option<Vec<String>>,

    /// Pipeline execution mode for multi-agent hands.
    /// Only used when `agents` is specified.
    #[serde(default)]
    pub pipeline: Option<PipelineMode>,

    /// Whether the hand is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Memory output path template (supports {date}, {id}, {name})
    #[serde(default)]
    pub memory_path: Option<String>,

    /// Additional parameters for the agent
    #[serde(default)]
    pub params: serde_json::Value,

    /// Autonomy configuration (optional, enables autonomous mode)
    #[serde(default)]
    pub autonomy: Option<AutonomyConfig>,

    /// Decision configuration (uses CLOSE when enabled)
    #[serde(default)]
    pub decision: Option<DecisionConfig>,

    /// Resource limits (optional)
    #[serde(default)]
    pub resources: Option<ResourceLimits>,

    /// Description (from content, extracted for convenience)
    #[serde(skip_serializing)]
    #[serde(default)]
    pub description: String,
}

fn default_name() -> String {
    "Unnamed Hand".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_enabled() -> bool {
    true
}

/// Summary of a hand for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub schedule: String,
    pub agent: String,
    pub enabled: bool,
    pub description: String,
    /// Autonomy level if configured
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autonomy_level: Option<String>,
    /// Whether autonomous mode is enabled
    #[serde(default)]
    pub autonomous: bool,
}

impl HandManifest {
    /// Parse a HAND.md file.
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read HAND.md: {}", path.display()))?;

        Self::from_content(content, path)
    }

    /// Parse HAND.md content with its path.
    pub fn from_content(content: String, path: PathBuf) -> Result<Self> {
        let (frontmatter, markdown) = split_frontmatter(&content)?;

        let config: HandConfig = serde_yaml::from_str(&frontmatter)
            .with_context(|| format!("Invalid frontmatter in {}", path.display()))?;

        // Extract description from first heading or paragraph
        let description = extract_description(&markdown);

        Ok(Self {
            config: HandConfig {
                description,
                ..config
            },
            content: markdown,
            path,
            frontmatter,
        })
    }

    /// Get a summary of this hand.
    pub fn summary(&self) -> HandSummary {
        let autonomy_level = self.config.autonomy.as_ref().map(|a| format!("{:?}", a.level));
        let autonomous = self.config.autonomy.is_some();

        // For pipeline hands, show first agent or "pipeline"
        let agent = if self.config.is_pipeline() {
            format!("pipeline({})", self.config.agents.as_ref().map_or(0, |a| a.len()))
        } else {
            self.config.agent.clone()
        };

        HandSummary {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            version: self.config.version.clone(),
            schedule: self.config.schedule.clone(),
            agent,
            enabled: self.config.enabled,
            description: self.config.description.clone(),
            autonomy_level,
            autonomous,
        }
    }

    /// Check if this hand uses autonomous mode.
    pub fn is_autonomous(&self) -> bool {
        self.config.autonomy.is_some() &&
            self.config.decision.as_ref().map_or(false, |d| d.use_close)
    }
}

impl HandConfig {
    /// Check if this hand uses pipeline mode (multiple agents).
    pub fn is_pipeline(&self) -> bool {
        self.agents.as_ref().map_or(false, |a| a.len() > 1)
    }

    /// Get the list of agents to execute.
    /// Returns agents from `agents` field if set, otherwise wraps `agent` in a vec.
    pub fn get_agents(&self) -> Vec<String> {
        self.agents.clone().unwrap_or_else(|| {
            if self.agent.is_empty() {
                Vec::new()
            } else {
                vec![self.agent.clone()]
            }
        })
    }

    /// Get the pipeline mode, defaulting to Sequential.
    pub fn get_pipeline_mode(&self) -> PipelineMode {
        self.pipeline.unwrap_or_default()
    }
}

/// Split frontmatter from markdown content.
fn split_frontmatter(content: &str) -> Result<(String, String)> {
    let trimmed = content.trim_start();

    if !trimmed.starts_with("---") {
        anyhow::bail!("No frontmatter found (expected --- at start)");
    }

    let rest = &trimmed[3..]; // Skip opening ---
    let end_idx = rest.find("---")
        .ok_or_else(|| anyhow::anyhow!("Unclosed frontmatter (missing closing ---)"))?;

    let frontmatter = rest[..end_idx].trim().to_string();
    let markdown = rest[end_idx + 3..].trim_start().to_string();

    Ok((frontmatter, markdown))
}

/// Extract description from markdown content.
fn extract_description(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();

    // Find first non-empty line
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip heading markers
        if trimmed.starts_with('#') {
            return trimmed.trim_start_matches('#').trim().to_string();
        }
        return trimmed.to_string();
    }

    String::new()
}

/// Default hands directory.
pub fn hands_dir() -> PathBuf {
    config_dir().join("hands")
}

/// Discover all hands in the hands directory.
pub fn discover_hands() -> Result<Vec<HandManifest>> {
    let dir = hands_dir();
    let mut hands = Vec::new();

    if !dir.exists() {
        tracing::debug!(dir = %dir.display(), "Hands directory does not exist, creating");
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create hands directory: {}", dir.display()))?;
        return Ok(hands);
    }

    let entries = fs::read_dir(&dir)
        .with_context(|| format!("Failed to read hands directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Look for HAND.md files in subdirectories
        if path.is_dir() {
            let hand_md = path.join("HAND.md");
            if hand_md.exists() {
                match HandManifest::from_path(hand_md.clone()) {
                    Ok(hand) => {
                        tracing::debug!(hand_id = %hand.config.id, "Loaded hand from {}", hand_md.display());
                        hands.push(hand);
                    }
                    Err(e) => {
                        tracing::warn!(path = %hand_md.display(), error = %e, "Failed to load hand");
                    }
                }
            }
        }
        // Also support HAND.md directly in the hands root
        else if path.file_name() == Some(std::ffi::OsStr::new("HAND.md")) {
            match HandManifest::from_path(path.clone()) {
                Ok(hand) => {
                    tracing::debug!(hand_id = %hand.config.id, "Loaded hand from {}", path.display());
                    hands.push(hand);
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "Failed to load hand");
                }
            }
        }
    }

    Ok(hands)
}

/// Load a specific hand by ID.
pub fn load_hand(id: &str) -> Result<HandManifest> {
    let dir = hands_dir();

    // Try subdirectory first: ~/.codecoder/hands/<id>/HAND.md
    let hand_md = dir.join(id).join("HAND.md");
    if hand_md.exists() {
        return HandManifest::from_path(hand_md);
    }

    // Try direct file: ~/.codecoder/hands/<id>.md
    let direct_md = dir.join(format!("{id}.md"));
    if direct_md.exists() {
        return HandManifest::from_path(direct_md);
    }

    anyhow::bail!("Hand '{}' not found (checked {} and {})", id, hand_md.display(), direct_md.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_frontmatter() {
        let content = r#"---
id: "test"
name: "Test"
---

# Content here
"#;
        let (frontmatter, markdown) = split_frontmatter(content).unwrap();
        assert_eq!(frontmatter, "id: \"test\"\nname: \"Test\"");
        assert!(markdown.contains("Content here"));
    }

    #[test]
    fn test_parse_hand_config() {
        let yaml = r#"---
id: "market-sentinel"
name: "Market Sentinel"
version: "1.0.0"
schedule: "0 */30 * * * *"
agent: "macro"
enabled: true
memory_path: "hands/market-sentinel/{date}.md"
params:
  threshold: 0.7
---
"#;

        let content = format!("{yaml}\n# Description");
        let (frontmatter, _markdown) = split_frontmatter(&content).unwrap();
        let config: HandConfig = serde_yaml::from_str(&frontmatter).unwrap();

        assert_eq!(config.id, "market-sentinel");
        assert_eq!(config.name, "Market Sentinel");
        assert_eq!(config.agent, "macro");
        assert!(config.enabled);
    }

    #[test]
    fn test_extract_description() {
        let markdown = "# Market Sentinel\n\nDescription here";
        let desc = extract_description(markdown);
        assert_eq!(desc, "Market Sentinel");
    }

    #[test]
    fn test_extract_description_from_plain() {
        let markdown = "Plain description\n\nMore text";
        let desc = extract_description(markdown);
        assert_eq!(desc, "Plain description");
    }

    #[test]
    fn test_frontmatter_required() {
        let content = "# No frontmatter";
        assert!(split_frontmatter(content).is_err());
    }

    #[test]
    fn test_default_values() {
        let yaml = r#"---
id: "test"
schedule: "0 * * * * *"
agent: "echo"
---
"#;
        let (frontmatter, _markdown) = split_frontmatter(yaml).unwrap();
        let config: HandConfig = serde_yaml::from_str(&frontmatter).unwrap();
        assert_eq!(config.name, "Unnamed Hand");
        assert_eq!(config.version, "1.0.0");
        assert!(config.enabled);
        // params defaults to an empty object when not specified
        assert!(config.params.is_object() || config.params.is_null());
    }
}
