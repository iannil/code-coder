//! Agent keywords configuration for routing.
//!
//! This module loads agent trigger keywords from a configuration file
//! (`~/.codecoder/keywords.json`) and provides functions for detecting
//! which agent should handle a given input.
//!
//! # Configuration File
//!
//! ```json
//! {
//!   "agents": {
//!     "macro": {
//!       "triggers": ["macro", "GDP", "economy"],
//!       "aliases": ["macro", "宏观"],
//!       "priority": 6
//!     }
//!   },
//!   "defaults": {
//!     "agent": "general",
//!     "cli_agent": "build",
//!     "im_agent": "autonomous"
//!   }
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use zero_common::keywords::{detect_alias, load_keywords};
//!
//! let keywords = load_keywords();
//! if let Some(agent) = detect_alias("@macro 分析PMI", &keywords) {
//!     println!("Routing to agent: {}", agent);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use super::config::config_dir;

// ============================================================================
// Types
// ============================================================================

/// Trigger rule for agent detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TriggerRule {
    /// Simple keyword string
    Simple(String),
    /// Advanced rule with type and options
    Advanced {
        #[serde(rename = "type")]
        trigger_type: TriggerType,
        value: String,
        #[serde(default = "default_priority")]
        priority: u8,
        #[serde(default)]
        description: Option<String>,
    },
}

/// Type of trigger rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TriggerType {
    /// Keyword contains match
    Keyword,
    /// Regex pattern match
    Pattern,
    /// Semantic context match
    Context,
    /// System event trigger
    Event,
}

/// Keywords configuration for a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentKeywords {
    /// Keywords that trigger this agent
    #[serde(default)]
    pub triggers: Vec<TriggerRule>,

    /// Aliases for @mention routing
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Agent priority (1-10, higher = more preferred)
    #[serde(default = "default_priority")]
    pub priority: u8,

    /// Whether this agent's keywords are active
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Default agent settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default agent when no triggers match
    #[serde(default = "default_agent")]
    pub agent: String,

    /// Default agent for CLI channel
    #[serde(default = "default_cli_agent")]
    pub cli_agent: String,

    /// Default agent for IM channels
    #[serde(default = "default_im_agent")]
    pub im_agent: String,

    /// Enable implicit keyword matching
    #[serde(default)]
    pub use_implicit_matching: bool,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            agent: default_agent(),
            cli_agent: default_cli_agent(),
            im_agent: default_im_agent(),
            use_implicit_matching: false,
        }
    }
}

/// Root keywords configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordsConfig {
    /// Configuration version
    #[serde(default = "default_version")]
    pub version: String,

    /// Agent keywords keyed by agent name
    #[serde(default)]
    pub agents: HashMap<String, AgentKeywords>,

    /// Default settings
    #[serde(default)]
    pub defaults: DefaultsConfig,
}

impl Default for KeywordsConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            agents: HashMap::new(),
            defaults: DefaultsConfig::default(),
        }
    }
}

// ============================================================================
// Default Value Functions
// ============================================================================

fn default_priority() -> u8 {
    5
}

fn default_enabled() -> bool {
    true
}

fn default_agent() -> String {
    "general".to_string()
}

fn default_cli_agent() -> String {
    "build".to_string()
}

fn default_im_agent() -> String {
    "autonomous".to_string()
}

fn default_version() -> String {
    "1.0.0".to_string()
}

// ============================================================================
// Loading Functions
// ============================================================================

/// Path to the keywords configuration file.
pub fn keywords_path() -> PathBuf {
    config_dir().join("keywords.json")
}

/// Load keywords configuration from file.
///
/// Returns Ok(config) if successful, Ok(default) if file doesn't exist.
pub fn load_keywords_from_file() -> KeywordsConfig {
    let path = keywords_path();

    if !path.exists() {
        tracing::debug!("Keywords config not found, using defaults");
        return KeywordsConfig::default();
    }

    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<KeywordsConfig>(&content) {
            Ok(config) => {
                tracing::info!(
                    path = %path.display(),
                    agents = config.agents.len(),
                    "Loaded keywords configuration"
                );
                config
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to parse keywords config, using defaults"
                );
                KeywordsConfig::default()
            }
        },
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "Failed to read keywords config, using defaults"
            );
            KeywordsConfig::default()
        }
    }
}

/// Global keywords configuration (lazy loaded).
static KEYWORDS: OnceLock<KeywordsConfig> = OnceLock::new();

/// Get the global keywords configuration.
pub fn keywords() -> &'static KeywordsConfig {
    KEYWORDS.get_or_init(load_keywords_from_file)
}

/// Load keywords configuration (for API compatibility).
pub fn load_keywords() -> &'static KeywordsConfig {
    keywords()
}

// ============================================================================
// Detection Functions
// ============================================================================

/// Detect agent from @mention alias.
///
/// Checks if the message starts with @<alias> and returns the matching agent name.
///
/// # Examples
///
/// ```rust,ignore
/// let result = detect_alias("@macro 分析PMI", keywords());
/// assert_eq!(result, Some("macro"));
///
/// let result = detect_alias("@宏观 分析PMI", keywords());
/// assert_eq!(result, Some("macro"));
/// ```
pub fn detect_alias<'a>(message: &str, config: &'a KeywordsConfig) -> Option<&'a str> {
    let text = message.trim();

    // Must start with @
    if !text.starts_with('@') {
        return None;
    }

    // Extract the mention (first word after @)
    let mention = text
        .split_whitespace()
        .next()?
        .trim_start_matches('@')
        .to_lowercase();

    // Search through all agents for matching alias
    for (agent_name, keywords) in &config.agents {
        if !keywords.enabled {
            continue;
        }

        // Check exact agent name match
        if agent_name.to_lowercase() == mention {
            return Some(agent_name.as_str());
        }

        // Check aliases
        for alias in &keywords.aliases {
            if alias.to_lowercase() == mention {
                return Some(agent_name.as_str());
            }
        }
    }

    None
}

/// Detect agent from message content using keyword triggers.
///
/// This performs implicit keyword matching (e.g., "macro" in "分析宏观经济").
/// Note: This is disabled by default to avoid misrouting.
///
/// Returns the agent name with highest priority match, or None.
pub fn detect_trigger<'a>(message: &str, config: &'a KeywordsConfig) -> Option<&'a str> {
    // Check if implicit matching is enabled
    if !config.defaults.use_implicit_matching {
        return None;
    }

    let text = message.to_lowercase();
    let mut best_match: Option<(&str, u8)> = None;

    for (agent_name, keywords) in &config.agents {
        if !keywords.enabled {
            continue;
        }

        for trigger in &keywords.triggers {
            let (matched, priority) = match trigger {
                TriggerRule::Simple(keyword) => {
                    (text.contains(&keyword.to_lowercase()), keywords.priority)
                }
                TriggerRule::Advanced {
                    trigger_type,
                    value,
                    priority,
                    ..
                } => {
                    let matched = match trigger_type {
                        TriggerType::Keyword => text.contains(&value.to_lowercase()),
                        TriggerType::Pattern => {
                            regex::Regex::new(value)
                                .map(|re| re.is_match(&text))
                                .unwrap_or(false)
                        }
                        TriggerType::Context => {
                            regex::Regex::new(value)
                                .map(|re| re.is_match(&text))
                                .unwrap_or(false)
                        }
                        TriggerType::Event => false, // Events are handled separately
                    };
                    (matched, *priority)
                }
            };

            if matched {
                match best_match {
                    Some((_, best_priority)) if priority > best_priority => {
                        best_match = Some((agent_name.as_str(), priority));
                    }
                    None => {
                        best_match = Some((agent_name.as_str(), priority));
                    }
                    _ => {}
                }
                break; // Only count first match per agent
            }
        }
    }

    best_match.map(|(name, _)| name)
}

/// Combined agent detection: alias → trigger → default.
///
/// # Arguments
///
/// * `message` - The user message to analyze
/// * `default_agent` - Fallback agent if no match found
/// * `config` - Keywords configuration to use
///
/// # Returns
///
/// The detected agent name (never returns None, falls back to default).
pub fn detect_agent<'a>(
    message: &str,
    default_agent: &'a str,
    config: &KeywordsConfig,
) -> &'a str {
    // 1. Try @mention alias detection
    if let Some(agent) = detect_alias(message, config) {
        // SAFETY: We're returning a reference to a string that lives in config,
        // but we need to return &'a str. Since config is 'static (from KEYWORDS),
        // this is safe, but we need to handle the lifetime correctly.
        // For now, we use the leaked string approach or just return as-is.
        return Box::leak(agent.to_string().into_boxed_str());
    }

    // 2. Try implicit keyword matching (if enabled)
    if let Some(agent) = detect_trigger(message, config) {
        return Box::leak(agent.to_string().into_boxed_str());
    }

    // 3. Fallback to default
    default_agent
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> KeywordsConfig {
        let mut config = KeywordsConfig::default();

        config.agents.insert(
            "macro".to_string(),
            AgentKeywords {
                triggers: vec![
                    TriggerRule::Simple("macro".to_string()),
                    TriggerRule::Simple("GDP".to_string()),
                ],
                aliases: vec!["macro".to_string(), "宏观".to_string()],
                priority: 6,
                enabled: true,
            },
        );

        config.agents.insert(
            "trader".to_string(),
            AgentKeywords {
                triggers: vec![TriggerRule::Simple("trading".to_string())],
                aliases: vec!["trader".to_string(), "交易".to_string()],
                priority: 6,
                enabled: true,
            },
        );

        config.agents.insert(
            "plan".to_string(),
            AgentKeywords {
                triggers: vec![TriggerRule::Advanced {
                    trigger_type: TriggerType::Keyword,
                    value: "plan".to_string(),
                    priority: 10,
                    description: None,
                }],
                aliases: vec!["plan".to_string(), "planner".to_string()],
                priority: 8,
                enabled: true,
            },
        );

        config
    }

    #[test]
    fn test_detect_alias_english() {
        let config = test_config();
        assert_eq!(detect_alias("@macro 分析PMI", &config), Some("macro"));
        assert_eq!(detect_alias("@trader 技术分析", &config), Some("trader"));
        assert_eq!(detect_alias("@plan 设计架构", &config), Some("plan"));
    }

    #[test]
    fn test_detect_alias_chinese() {
        let config = test_config();
        assert_eq!(detect_alias("@宏观 分析PMI", &config), Some("macro"));
        assert_eq!(detect_alias("@交易 技术分析", &config), Some("trader"));
    }

    #[test]
    fn test_detect_alias_case_insensitive() {
        let config = test_config();
        assert_eq!(detect_alias("@MACRO 分析PMI", &config), Some("macro"));
        assert_eq!(detect_alias("@Macro 分析PMI", &config), Some("macro"));
    }

    #[test]
    fn test_detect_alias_no_match() {
        let config = test_config();
        assert_eq!(detect_alias("@unknown 什么", &config), None);
        assert_eq!(detect_alias("没有@mention", &config), None);
        assert_eq!(detect_alias("纯文本消息", &config), None);
    }

    #[test]
    fn test_detect_trigger_disabled() {
        let config = test_config();
        // Implicit matching is disabled by default
        assert_eq!(detect_trigger("分析宏观经济", &config), None);
    }

    #[test]
    fn test_detect_trigger_enabled() {
        let mut config = test_config();
        config.defaults.use_implicit_matching = true;

        assert_eq!(detect_trigger("分析macro形势", &config), Some("macro"));
        assert_eq!(detect_trigger("GDP增长数据", &config), Some("macro"));
    }

    #[test]
    fn test_detect_agent_fallback() {
        let config = test_config();
        let result = detect_agent("普通消息", "build", &config);
        assert_eq!(result, "build");
    }

    #[test]
    fn test_detect_agent_with_alias() {
        let config = test_config();
        let result = detect_agent("@macro 分析PMI", "build", &config);
        assert_eq!(result, "macro");
    }
}
