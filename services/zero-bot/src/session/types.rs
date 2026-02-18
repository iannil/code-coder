//! Session types and configuration.

use serde::{Deserialize, Serialize};

/// Message role in a conversation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant (AI) response
    Assistant,
    /// System message (used for compressed summaries)
    System,
}

impl MessageRole {
    /// Convert to string representation for database storage.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
    }

    /// Parse from string representation.
    #[allow(clippy::match_same_arms)]
    pub fn parse(s: &str) -> Self {
        match s {
            "user" => Self::User,
            "assistant" => Self::Assistant,
            "system" => Self::System,
            _ => Self::User, // Default fallback
        }
    }
}

/// A single message in a conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Database row ID
    pub id: i64,
    /// Message role (user/assistant/system)
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Unix timestamp (seconds)
    pub timestamp: i64,
    /// Estimated token count (chars / 4)
    pub token_estimate: usize,
}

impl SessionMessage {
    /// Create a new session message with token estimation.
    pub fn new(id: i64, role: MessageRole, content: String, timestamp: i64) -> Self {
        let token_estimate = estimate_tokens(&content);
        Self {
            id,
            role,
            content,
            timestamp,
            token_estimate,
        }
    }
}

/// Estimate token count from text (approximate: chars / 4).
pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimation: ~4 characters per token for English/Chinese mix
    // More accurate would require tiktoken-rs, but this is sufficient for threshold checks
    text.chars().count().div_ceil(4)
}

/// Session management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Whether session management is enabled (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Model context window size in tokens (default: 128000)
    #[serde(default = "default_context_window")]
    pub context_window: usize,

    /// Threshold ratio to trigger auto-compaction (default: 0.8)
    /// When current session tokens exceed `context_window * compact_threshold`,
    /// auto-compaction is triggered.
    #[serde(default = "default_compact_threshold")]
    pub compact_threshold: f32,

    /// Number of recent messages to keep after compaction (default: 5)
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
}

fn default_enabled() -> bool {
    true
}

fn default_context_window() -> usize {
    128_000
}

fn default_compact_threshold() -> f32 {
    0.8
}

fn default_keep_recent() -> usize {
    5
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            context_window: default_context_window(),
            compact_threshold: default_compact_threshold(),
            keep_recent: default_keep_recent(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_role_roundtrip() {
        assert_eq!(MessageRole::parse(MessageRole::User.as_str()), MessageRole::User);
        assert_eq!(MessageRole::parse(MessageRole::Assistant.as_str()), MessageRole::Assistant);
        assert_eq!(MessageRole::parse(MessageRole::System.as_str()), MessageRole::System);
    }

    #[test]
    fn test_message_role_unknown_defaults_to_user() {
        assert_eq!(MessageRole::parse("unknown"), MessageRole::User);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("hello"), 2); // 5 chars -> 2 tokens
        assert_eq!(estimate_tokens("hello world"), 3); // 11 chars -> 3 tokens
    }

    #[test]
    fn test_estimate_tokens_unicode() {
        // Chinese characters: each char counts as 1 in .chars()
        let chinese = "你好世界"; // 4 chars
        assert_eq!(estimate_tokens(chinese), 1);
    }

    #[test]
    fn test_session_message_new() {
        let msg = SessionMessage::new(1, MessageRole::User, "hello world".to_string(), 1234567890);
        assert_eq!(msg.id, 1);
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "hello world");
        assert_eq!(msg.timestamp, 1234567890);
        assert_eq!(msg.token_estimate, 3);
    }

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.context_window, 128_000);
        assert!((config.compact_threshold - 0.8).abs() < f32::EPSILON);
        assert_eq!(config.keep_recent, 5);
    }
}
