//! Streaming LLM Provider for Phase 6.1 migration.
//!
//! Provides async streaming support for LLM API calls with tool calling.
//! Designed to work with the IPC protocol for TUI integration.

use anyhow::Result;
use async_trait::async_trait;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// Stream event types emitted during LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Start of response
    Start,
    /// Text delta (streaming text)
    TextDelta { content: String },
    /// Reasoning/thinking delta
    ReasoningDelta { content: String },
    /// Tool call started
    ToolCallStart { id: String, name: String },
    /// Tool call arguments (streamed)
    ToolCallDelta { id: String, arguments_delta: String },
    /// Tool call complete
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    /// Response finished
    Finish { reason: String, usage: Option<Usage> },
    /// Error occurred
    Error { code: i32, message: String },
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    #[serde(default)]
    pub reasoning_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<usize>,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Message content part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String },
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentPart>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentPart::Text { text: text.into() }],
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentPart::Text { text: text.into() }],
        }
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: vec![ContentPart::Text { text: text.into() }],
        }
    }

    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: vec![ContentPart::ToolResult {
                tool_use_id: tool_use_id.into(),
                content: content.into(),
            }],
        }
    }
}

/// Tool definition for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Streaming request configuration
#[derive(Debug, Clone)]
pub struct StreamRequest {
    /// System prompt(s)
    pub system: Vec<String>,
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Available tools
    pub tools: Vec<ToolDef>,
    /// Model ID
    pub model: String,
    /// Temperature (0.0 - 1.0)
    pub temperature: Option<f64>,
    /// Max output tokens
    pub max_tokens: Option<usize>,
}

/// Type alias for boxed stream
pub type EventStream = Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>;

/// Streaming provider trait for LLM backends.
#[async_trait]
pub trait StreamingProvider: Send + Sync {
    /// Provider name (e.g., "anthropic", "openai")
    fn name(&self) -> &str;

    /// Stream a chat response
    async fn stream(&self, request: StreamRequest) -> Result<EventStream>;

    /// Check if the provider supports a specific model
    fn supports_model(&self, model: &str) -> bool;
}

/// Anthropic Claude streaming provider
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".into(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl StreamingProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("claude-")
    }

    async fn stream(&self, request: StreamRequest) -> Result<EventStream> {
        use futures_util::StreamExt;

        // Build Anthropic API request
        let client = reqwest::Client::new();

        // Convert messages to Anthropic format
        let anthropic_messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "user", // Tool results go as user messages
                    Role::System => "user", // Should be filtered out
                };

                let content: Vec<serde_json::Value> = m
                    .content
                    .iter()
                    .map(|p| match p {
                        ContentPart::Text { text } => serde_json::json!({
                            "type": "text",
                            "text": text
                        }),
                        ContentPart::ToolUse { id, name, input } => serde_json::json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input
                        }),
                        ContentPart::ToolResult { tool_use_id, content } => serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content
                        }),
                    })
                    .collect();

                serde_json::json!({
                    "role": role,
                    "content": content
                })
            })
            .collect();

        // Build tools array
        let tools: Vec<serde_json::Value> = request
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.input_schema
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": anthropic_messages,
            "max_tokens": request.max_tokens.unwrap_or(8192),
            "stream": true
        });

        if !request.system.is_empty() {
            body["system"] = serde_json::json!(request.system.join("\n\n"));
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }

        let response = client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .body(serde_json::to_string(&body)?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error {}: {}", status, text));
        }

        // Parse SSE stream
        let byte_stream = response.bytes_stream();

        // Convert to event stream
        let event_stream = byte_stream
            .map(move |result| -> Result<StreamEvent> {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        parse_sse_event(&text)
                    }
                    Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            })
            .filter_map(|r| async move {
                match r {
                    Ok(event) => Some(Ok(event)),
                    Err(e) if e.to_string().contains("skip") => None,
                    Err(e) => Some(Err(e)),
                }
            });

        Ok(Box::pin(event_stream))
    }
}

/// Parse SSE event from Anthropic
fn parse_sse_event(data: &str) -> Result<StreamEvent> {
    // SSE format: "data: {...}\n\n"
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') {
            continue;
        }

        if let Some(json_str) = line.strip_prefix("data: ") {
            if json_str == "[DONE]" {
                return Ok(StreamEvent::Finish {
                    reason: "stop".into(),
                    usage: None,
                });
            }

            let event: serde_json::Value = serde_json::from_str(json_str)?;
            return parse_anthropic_event(event);
        }
    }

    // Skip if no valid data
    Err(anyhow::anyhow!("skip"))
}

/// Parse Anthropic-specific event format
fn parse_anthropic_event(event: serde_json::Value) -> Result<StreamEvent> {
    let event_type = event["type"].as_str().unwrap_or("");

    match event_type {
        "message_start" => Ok(StreamEvent::Start),

        "content_block_start" => {
            let content_block = &event["content_block"];
            let block_type = content_block["type"].as_str().unwrap_or("");

            match block_type {
                "tool_use" => {
                    let id = content_block["id"].as_str().unwrap_or("").to_string();
                    let name = content_block["name"].as_str().unwrap_or("").to_string();
                    Ok(StreamEvent::ToolCallStart { id, name })
                }
                "thinking" => Ok(StreamEvent::ReasoningDelta {
                    content: String::new(),
                }),
                _ => Err(anyhow::anyhow!("skip")),
            }
        }

        "content_block_delta" => {
            let delta = &event["delta"];
            let delta_type = delta["type"].as_str().unwrap_or("");

            match delta_type {
                "text_delta" => {
                    let text = delta["text"].as_str().unwrap_or("").to_string();
                    Ok(StreamEvent::TextDelta { content: text })
                }
                "thinking_delta" => {
                    let text = delta["thinking"].as_str().unwrap_or("").to_string();
                    Ok(StreamEvent::ReasoningDelta { content: text })
                }
                "input_json_delta" => {
                    let partial = delta["partial_json"].as_str().unwrap_or("").to_string();
                    // We need the tool_use_id from somewhere - this is a limitation
                    // In practice, we track the current tool call in the stream processor
                    Ok(StreamEvent::ToolCallDelta {
                        id: String::new(), // Will be filled in by stream processor
                        arguments_delta: partial,
                    })
                }
                _ => Err(anyhow::anyhow!("skip")),
            }
        }

        "content_block_stop" => {
            // Content block finished - for tool calls, we emit the final ToolCall event
            // This requires accumulating the tool call data
            Err(anyhow::anyhow!("skip"))
        }

        "message_delta" => {
            let delta = &event["delta"];
            let stop_reason = delta["stop_reason"].as_str();

            if let Some(reason) = stop_reason {
                let usage = event["usage"].as_object().map(|u| Usage {
                    input_tokens: u["input_tokens"].as_u64().unwrap_or(0) as usize,
                    output_tokens: u["output_tokens"].as_u64().unwrap_or(0) as usize,
                    reasoning_tokens: 0,
                    cache_read_tokens: u["cache_read_input_tokens"].as_u64().map(|v| v as usize),
                    cache_write_tokens: u["cache_creation_input_tokens"]
                        .as_u64()
                        .map(|v| v as usize),
                });

                Ok(StreamEvent::Finish {
                    reason: reason.to_string(),
                    usage,
                })
            } else {
                Err(anyhow::anyhow!("skip"))
            }
        }

        "message_stop" => Ok(StreamEvent::Finish {
            reason: "stop".into(),
            usage: None,
        }),

        "error" => {
            let error = &event["error"];
            let message = error["message"].as_str().unwrap_or("Unknown error");
            Ok(StreamEvent::Error {
                code: -1,
                message: message.to_string(),
            })
        }

        _ => Err(anyhow::anyhow!("skip")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.content.len(), 1);

        let assistant_msg = Message::assistant("Hi there!");
        assert_eq!(assistant_msg.role, Role::Assistant);
    }

    #[test]
    fn test_stream_event_serialization() {
        let event = StreamEvent::TextDelta {
            content: "Hello".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"text_delta\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_tool_call_event() {
        let event = StreamEvent::ToolCall {
            id: "call-123".into(),
            name: "read".into(),
            arguments: serde_json::json!({"path": "/README.md"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"tool_call\""));
        assert!(json.contains("\"name\":\"read\""));
    }

    #[test]
    fn test_usage_serialization() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            reasoning_tokens: 0,
            cache_read_tokens: Some(80),
            cache_write_tokens: None,
        };
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"inputTokens\":100"));
        assert!(json.contains("\"cacheReadTokens\":80"));
        assert!(!json.contains("cacheWriteTokens")); // Should be skipped
    }

    #[test]
    fn test_anthropic_provider_model_support() {
        let provider = AnthropicProvider::new("test-key");
        assert!(provider.supports_model("claude-opus-4-5"));
        assert!(provider.supports_model("claude-sonnet-4-5"));
        assert!(!provider.supports_model("gpt-4o"));
    }
}
