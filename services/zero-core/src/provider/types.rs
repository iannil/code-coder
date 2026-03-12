//! Provider types and common structures
//!
//! Common types shared across all AI provider implementations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Message Types
// ============================================================================

/// Role of a message in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

/// Content part types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    Image {
        #[serde(rename = "source")]
        image_source: ImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

/// Image source for multimodal content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 {
        media_type: String,
        data: String,
    },
    Url {
        url: String,
    },
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Message content - either simple text or multi-part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

impl MessageContent {
    /// Create text content
    pub fn text(s: impl Into<String>) -> Self {
        MessageContent::Text(s.into())
    }

    /// Create multi-part content
    pub fn parts(parts: Vec<ContentPart>) -> Self {
        MessageContent::Parts(parts)
    }

    /// Get as plain text (joining all text parts)
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

// ============================================================================
// Tool Types
// ============================================================================

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tool choice specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Any,
    None,
    Tool { name: String },
}

impl Default for ToolChoice {
    fn default() -> Self {
        ToolChoice::Auto
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request parameters for chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Extended thinking configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Provider-specific options
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub provider_options: HashMap<String, serde_json::Value>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            system: None,
            max_tokens: Some(4096),
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            stream: Some(false),
            thinking: None,
            provider_options: HashMap::new(),
        }
    }
}

/// Configuration for extended thinking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: ThinkingType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

/// Type of thinking mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingType {
    Enabled,
    Disabled,
}

/// Stop reason for completion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    ContentFilter,
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReason::EndTurn => write!(f, "end_turn"),
            StopReason::MaxTokens => write!(f, "max_tokens"),
            StopReason::StopSequence => write!(f, "stop_sequence"),
            StopReason::ToolUse => write!(f, "tool_use"),
            StopReason::ContentFilter => write!(f, "content_filter"),
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub content: Vec<ContentPart>,
    pub stop_reason: Option<StopReason>,
    pub usage: Usage,
    /// Raw response for debugging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<serde_json::Value>,
}

impl ChatResponse {
    /// Get the text content from the response
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get tool use blocks from the response
    pub fn tool_uses(&self) -> Vec<(&str, &str, &serde_json::Value)> {
        self.content
            .iter()
            .filter_map(|p| match p {
                ContentPart::ToolUse { id, name, input } => Some((id.as_str(), name.as_str(), input)),
                _ => None,
            })
            .collect()
    }

    /// Get thinking blocks from the response
    pub fn thinking(&self) -> Option<&str> {
        self.content.iter().find_map(|p| match p {
            ContentPart::Thinking { thinking, .. } => Some(thinking.as_str()),
            _ => None,
        })
    }
}

// ============================================================================
// Streaming Types
// ============================================================================

/// Streaming event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Message started
    MessageStart {
        message: StreamMessage,
    },
    /// Content block started
    ContentBlockStart {
        index: usize,
        content_block: ContentPart,
    },
    /// Content block delta
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },
    /// Content block stopped
    ContentBlockStop {
        index: usize,
    },
    /// Message delta (final updates)
    MessageDelta {
        delta: MessageDelta,
        usage: Option<Usage>,
    },
    /// Message stopped
    MessageStop,
    /// Ping (keepalive)
    Ping,
    /// Error
    Error {
        error: StreamError,
    },
}

/// Partial message for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    pub id: String,
    pub model: String,
    pub role: MessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Content delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
}

/// Message delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
}

/// Stream error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// ============================================================================
// Provider Configuration
// ============================================================================

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_id: String,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub headers: HashMap<String, String>,
}

impl ProviderConfig {
    /// Create a new provider configuration
    pub fn new(provider_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            api_key: api_key.into(),
            base_url: None,
            timeout_ms: Some(120_000), // 2 minutes default
            max_retries: Some(3),
            headers: HashMap::new(),
        }
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// Set max retries
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// Model Information
// ============================================================================

/// Model capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub temperature: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub tool_call: bool,
    pub vision: bool,
    pub streaming: bool,
}

/// Model cost information (per million tokens)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// Model limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimits {
    pub context_window: u32,
    pub max_output_tokens: u32,
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            context_window: 128_000,
            max_output_tokens: 4096,
        }
    }
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    pub capabilities: ModelCapabilities,
    pub cost: ModelCost,
    pub limits: ModelLimits,
}

// ============================================================================
// Error Types
// ============================================================================

/// Provider error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderErrorKind {
    /// Authentication failed
    Authentication,
    /// Rate limit exceeded
    RateLimit {
        retry_after_ms: Option<u64>,
    },
    /// Invalid request
    InvalidRequest,
    /// Model not found
    ModelNotFound,
    /// Content filtered
    ContentFilter,
    /// Server error
    Server,
    /// Timeout
    Timeout,
    /// Network error
    Network,
    /// Unknown error
    Unknown,
}

/// Provider error
#[derive(Debug, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct ProviderError {
    pub kind: ProviderErrorKind,
    pub message: String,
    pub status_code: Option<u16>,
    pub raw: Option<serde_json::Value>,
}

impl ProviderError {
    pub fn new(kind: ProviderErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            status_code: None,
            raw: None,
        }
    }

    pub fn with_status(mut self, code: u16) -> Self {
        self.status_code = Some(code);
        self
    }

    pub fn with_raw(mut self, raw: serde_json::Value) -> Self {
        self.raw = Some(raw);
        self
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            ProviderErrorKind::RateLimit { .. }
                | ProviderErrorKind::Server
                | ProviderErrorKind::Timeout
                | ProviderErrorKind::Network
        )
    }

    /// Get retry delay if applicable
    pub fn retry_after_ms(&self) -> Option<u64> {
        match &self.kind {
            ProviderErrorKind::RateLimit { retry_after_ms } => *retry_after_ms,
            _ => None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_content_text() {
        let content = MessageContent::text("Hello, world!");
        assert_eq!(content.as_text(), "Hello, world!");
    }

    #[test]
    fn test_message_content_parts() {
        let parts = vec![
            ContentPart::Text { text: "Hello, ".to_string() },
            ContentPart::Text { text: "world!".to_string() },
        ];
        let content = MessageContent::parts(parts);
        assert_eq!(content.as_text(), "Hello, world!");
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new("anthropic", "sk-test")
            .with_base_url("https://api.anthropic.com")
            .with_timeout(60_000)
            .with_retries(5)
            .with_header("X-Custom", "value");

        assert_eq!(config.provider_id, "anthropic");
        assert_eq!(config.api_key, "sk-test");
        assert_eq!(config.base_url, Some("https://api.anthropic.com".to_string()));
        assert_eq!(config.timeout_ms, Some(60_000));
        assert_eq!(config.max_retries, Some(5));
        assert_eq!(config.headers.get("X-Custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_error_retryable() {
        let rate_limit = ProviderError::new(
            ProviderErrorKind::RateLimit { retry_after_ms: Some(1000) },
            "Rate limited",
        );
        assert!(rate_limit.is_retryable());
        assert_eq!(rate_limit.retry_after_ms(), Some(1000));

        let auth = ProviderError::new(ProviderErrorKind::Authentication, "Invalid API key");
        assert!(!auth.is_retryable());
    }
}
