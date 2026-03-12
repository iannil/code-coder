//! Anthropic Claude API Provider
//!
//! Direct implementation of the Anthropic Messages API for Claude models.
//! Supports streaming, tool use, extended thinking, and prompt caching.

use std::pin::Pin;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, warn};

use super::types::{
    ChatRequest, ChatResponse, ContentDelta, ContentPart, Message, MessageContent,
    MessageDelta, MessageRole, ProviderConfig, ProviderError, ProviderErrorKind,
    StopReason, StreamError, StreamEvent, StreamMessage, ThinkingType, ToolChoice,
    Usage,
};
use super::Provider;

// ============================================================================
// Constants
// ============================================================================

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_BETA: &str = "claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14";

// ============================================================================
// Anthropic Provider
// ============================================================================

/// Anthropic Claude API provider
pub struct AnthropicProvider {
    client: Client,
    config: ProviderConfig,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| ANTHROPIC_API_URL.to_string());

        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(
                config.timeout_ms.unwrap_or(120_000),
            ))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            base_url,
        }
    }

    /// Build request headers
    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-api-key",
            self.config.api_key.parse().expect("Invalid API key"),
        );
        headers.insert(
            "anthropic-version",
            ANTHROPIC_VERSION.parse().unwrap(),
        );
        headers.insert(
            "anthropic-beta",
            ANTHROPIC_BETA.parse().unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        // Add custom headers
        for (key, value) in &self.config.headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::try_from(key.as_str()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        headers
    }

    /// Convert internal message format to Anthropic format
    fn convert_message(&self, msg: &Message) -> AnthropicMessage {
        let content = match &msg.content {
            MessageContent::Text(text) => vec![AnthropicContent::Text {
                text: text.clone(),
                cache_control: None,
            }],
            MessageContent::Parts(parts) => parts
                .iter()
                .map(|p| match p {
                    ContentPart::Text { text } => AnthropicContent::Text {
                        text: text.clone(),
                        cache_control: None,
                    },
                    ContentPart::Image { image_source } => match image_source {
                        super::types::ImageSource::Base64 { media_type, data } => {
                            AnthropicContent::Image {
                                source: ImageSource::Base64 {
                                    media_type: media_type.clone(),
                                    data: data.clone(),
                                },
                            }
                        }
                        super::types::ImageSource::Url { url } => AnthropicContent::Image {
                            source: ImageSource::Url { url: url.clone() },
                        },
                    },
                    ContentPart::ToolUse { id, name, input } => AnthropicContent::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    },
                    ContentPart::ToolResult {
                        tool_use_id,
                        content,
                        is_error,
                    } => AnthropicContent::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: content.clone(),
                        is_error: *is_error,
                    },
                    ContentPart::Thinking { thinking, signature } => AnthropicContent::Thinking {
                        thinking: thinking.clone(),
                        signature: signature.clone(),
                    },
                })
                .collect(),
        };

        AnthropicMessage {
            role: match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "user".to_string(), // Tool results are sent as user
                MessageRole::System => "user".to_string(), // Should not happen
            },
            content,
        }
    }

    /// Convert Anthropic response to internal format
    fn convert_response(&self, resp: AnthropicResponse) -> ChatResponse {
        let content = resp
            .content
            .into_iter()
            .map(|c| match c {
                AnthropicContent::Text { text, .. } => ContentPart::Text { text },
                AnthropicContent::Image { .. } => {
                    // Images in response are rare, treat as text
                    ContentPart::Text {
                        text: "[image]".to_string(),
                    }
                }
                AnthropicContent::ToolUse { id, name, input } => {
                    ContentPart::ToolUse { id, name, input }
                }
                AnthropicContent::ToolResult { .. } => {
                    // Tool results should not appear in response
                    ContentPart::Text {
                        text: "[tool_result]".to_string(),
                    }
                }
                AnthropicContent::Thinking { thinking, signature } => {
                    ContentPart::Thinking { thinking, signature }
                }
            })
            .collect();

        let stop_reason = resp.stop_reason.map(|r| match r.as_str() {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            "tool_use" => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        });

        ChatResponse {
            id: resp.id,
            model: resp.model,
            content,
            stop_reason,
            usage: Usage {
                input_tokens: resp.usage.input_tokens,
                output_tokens: resp.usage.output_tokens,
                cache_creation_input_tokens: resp.usage.cache_creation_input_tokens,
                cache_read_input_tokens: resp.usage.cache_read_input_tokens,
            },
            raw: None,
        }
    }

    /// Parse error from API response
    fn parse_error(&self, status: StatusCode, body: &str) -> ProviderError {
        let raw: Option<serde_json::Value> = serde_json::from_str(body).ok();

        let (kind, message) = if let Some(ref json) = raw {
            let error_type = json
                .get("error")
                .and_then(|e| e.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");

            let error_msg = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(body);

            let kind = match error_type {
                "authentication_error" => ProviderErrorKind::Authentication,
                "rate_limit_error" => ProviderErrorKind::RateLimit {
                    retry_after_ms: None,
                },
                "invalid_request_error" => ProviderErrorKind::InvalidRequest,
                "not_found_error" => ProviderErrorKind::ModelNotFound,
                "overloaded_error" => ProviderErrorKind::RateLimit {
                    retry_after_ms: Some(5000),
                },
                _ => match status {
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                        ProviderErrorKind::Authentication
                    }
                    StatusCode::TOO_MANY_REQUESTS => ProviderErrorKind::RateLimit {
                        retry_after_ms: None,
                    },
                    StatusCode::BAD_REQUEST => ProviderErrorKind::InvalidRequest,
                    StatusCode::NOT_FOUND => ProviderErrorKind::ModelNotFound,
                    s if s.is_server_error() => ProviderErrorKind::Server,
                    _ => ProviderErrorKind::Unknown,
                },
            };

            (kind, error_msg.to_string())
        } else {
            let kind = match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    ProviderErrorKind::Authentication
                }
                StatusCode::TOO_MANY_REQUESTS => ProviderErrorKind::RateLimit {
                    retry_after_ms: None,
                },
                StatusCode::BAD_REQUEST => ProviderErrorKind::InvalidRequest,
                StatusCode::NOT_FOUND => ProviderErrorKind::ModelNotFound,
                s if s.is_server_error() => ProviderErrorKind::Server,
                _ => ProviderErrorKind::Unknown,
            };

            (kind, body.to_string())
        };

        let mut error = ProviderError::new(kind, message).with_status(status.as_u16());

        if let Some(raw) = raw {
            error = error.with_raw(raw);
        }

        error
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn provider_id(&self) -> &str {
        "anthropic"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = format!("{}/v1/messages", self.base_url);

        // Extract system message
        let system = request.system.clone().or_else(|| {
            request
                .messages
                .iter()
                .find(|m| m.role == MessageRole::System)
                .map(|m| m.content.as_text())
        });

        // Filter out system messages and convert
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| self.convert_message(m))
            .collect();

        // Build request body
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(system) = system {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(top_k) = request.top_k {
            body["top_k"] = json!(top_k);
        }

        if let Some(stops) = &request.stop_sequences {
            body["stop_sequences"] = json!(stops);
        }

        // Convert tools
        if let Some(tools) = &request.tools {
            let anthropic_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = json!(anthropic_tools);
        }

        // Convert tool choice
        if let Some(choice) = &request.tool_choice {
            body["tool_choice"] = match choice {
                ToolChoice::Auto => json!({"type": "auto"}),
                ToolChoice::Any => json!({"type": "any"}),
                ToolChoice::None => json!({"type": "none"}),
                ToolChoice::Tool { name } => json!({"type": "tool", "name": name}),
            };
        }

        // Extended thinking
        if let Some(thinking) = &request.thinking {
            if thinking.thinking_type == ThinkingType::Enabled {
                body["thinking"] = json!({
                    "type": "enabled",
                    "budget_tokens": thinking.budget_tokens.unwrap_or(10000),
                });
            }
        }

        debug!("Anthropic request: {}", serde_json::to_string_pretty(&body).unwrap_or_default());

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::new(ProviderErrorKind::Timeout, e.to_string())
                } else {
                    ProviderError::new(ProviderErrorKind::Network, e.to_string())
                }
            })?;

        let status = response.status();
        let body_text = response.text().await.map_err(|e| {
            ProviderError::new(ProviderErrorKind::Network, e.to_string())
        })?;

        if !status.is_success() {
            return Err(self.parse_error(status, &body_text));
        }

        let resp: AnthropicResponse = serde_json::from_str(&body_text).map_err(|e| {
            ProviderError::new(
                ProviderErrorKind::Unknown,
                format!("Failed to parse response: {}", e),
            )
        })?;

        Ok(self.convert_response(resp))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        let url = format!("{}/v1/messages", self.base_url);

        // Extract system message
        let system = request.system.clone().or_else(|| {
            request
                .messages
                .iter()
                .find(|m| m.role == MessageRole::System)
                .map(|m| m.content.as_text())
        });

        // Filter out system messages and convert
        let messages: Vec<AnthropicMessage> = request
            .messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| self.convert_message(m))
            .collect();

        // Build request body with streaming enabled
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(system) = system {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(top_k) = request.top_k {
            body["top_k"] = json!(top_k);
        }

        if let Some(stops) = &request.stop_sequences {
            body["stop_sequences"] = json!(stops);
        }

        // Convert tools
        if let Some(tools) = &request.tools {
            let anthropic_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = json!(anthropic_tools);
        }

        // Convert tool choice
        if let Some(choice) = &request.tool_choice {
            body["tool_choice"] = match choice {
                ToolChoice::Auto => json!({"type": "auto"}),
                ToolChoice::Any => json!({"type": "any"}),
                ToolChoice::None => json!({"type": "none"}),
                ToolChoice::Tool { name } => json!({"type": "tool", "name": name}),
            };
        }

        // Extended thinking
        if let Some(thinking) = &request.thinking {
            if thinking.thinking_type == ThinkingType::Enabled {
                body["thinking"] = json!({
                    "type": "enabled",
                    "budget_tokens": thinking.budget_tokens.unwrap_or(10000),
                });
            }
        }

        let response = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::new(ProviderErrorKind::Timeout, e.to_string())
                } else {
                    ProviderError::new(ProviderErrorKind::Network, e.to_string())
                }
            })?;

        let status = response.status();

        if !status.is_success() {
            let body_text = response.text().await.map_err(|e| {
                ProviderError::new(ProviderErrorKind::Network, e.to_string())
            })?;
            return Err(self.parse_error(status, &body_text));
        }

        // Create SSE stream
        let stream = response.bytes_stream();

        let event_stream = stream
            .map(move |result| {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        parse_sse_events(&text)
                    }
                    Err(e) => {
                        vec![Err(ProviderError::new(
                            ProviderErrorKind::Network,
                            e.to_string(),
                        ))]
                    }
                }
            })
            .flat_map(futures_util::stream::iter);

        Ok(Box::pin(event_stream))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }
}

/// Parse SSE events from text
fn parse_sse_events(text: &str) -> Vec<Result<StreamEvent, ProviderError>> {
    let mut events = Vec::new();
    let mut current_event = String::new();
    let mut event_type = String::new();

    for line in text.lines() {
        if line.starts_with("event: ") {
            event_type = line[7..].to_string();
        } else if line.starts_with("data: ") {
            current_event = line[6..].to_string();
        } else if line.is_empty() && !current_event.is_empty() {
            // Process the complete event
            if let Some(event) = parse_single_event(&event_type, &current_event) {
                events.push(event);
            }
            current_event.clear();
            event_type.clear();
        }
    }

    // Handle any remaining event
    if !current_event.is_empty() {
        if let Some(event) = parse_single_event(&event_type, &current_event) {
            events.push(event);
        }
    }

    events
}

/// Parse a single SSE event
fn parse_single_event(event_type: &str, data: &str) -> Option<Result<StreamEvent, ProviderError>> {
    match event_type {
        "message_start" => {
            let parsed: Result<AnthropicStreamStart, _> = serde_json::from_str(data);
            match parsed {
                Ok(start) => Some(Ok(StreamEvent::MessageStart {
                    message: StreamMessage {
                        id: start.message.id,
                        model: start.message.model,
                        role: MessageRole::Assistant,
                        stop_reason: None,
                        usage: Some(Usage {
                            input_tokens: start.message.usage.input_tokens,
                            output_tokens: start.message.usage.output_tokens,
                            cache_creation_input_tokens: start.message.usage.cache_creation_input_tokens,
                            cache_read_input_tokens: start.message.usage.cache_read_input_tokens,
                        }),
                    },
                })),
                Err(e) => {
                    warn!("Failed to parse message_start: {}", e);
                    None
                }
            }
        }
        "content_block_start" => {
            let parsed: Result<AnthropicBlockStart, _> = serde_json::from_str(data);
            match parsed {
                Ok(block) => {
                    let content_block = match block.content_block {
                        AnthropicContent::Text { text, .. } => ContentPart::Text { text },
                        AnthropicContent::ToolUse { id, name, input } => {
                            ContentPart::ToolUse { id, name, input }
                        }
                        AnthropicContent::Thinking { thinking, signature } => {
                            ContentPart::Thinking { thinking, signature }
                        }
                        _ => return None,
                    };
                    Some(Ok(StreamEvent::ContentBlockStart {
                        index: block.index,
                        content_block,
                    }))
                }
                Err(e) => {
                    warn!("Failed to parse content_block_start: {}", e);
                    None
                }
            }
        }
        "content_block_delta" => {
            let parsed: Result<AnthropicBlockDelta, _> = serde_json::from_str(data);
            match parsed {
                Ok(delta) => {
                    let content_delta = match delta.delta {
                        AnthropicDelta::TextDelta { text } => ContentDelta::TextDelta { text },
                        AnthropicDelta::InputJsonDelta { partial_json } => {
                            ContentDelta::InputJsonDelta { partial_json }
                        }
                        AnthropicDelta::ThinkingDelta { thinking } => {
                            ContentDelta::ThinkingDelta { thinking }
                        }
                    };
                    Some(Ok(StreamEvent::ContentBlockDelta {
                        index: delta.index,
                        delta: content_delta,
                    }))
                }
                Err(e) => {
                    warn!("Failed to parse content_block_delta: {}", e);
                    None
                }
            }
        }
        "content_block_stop" => {
            let parsed: Result<AnthropicBlockStop, _> = serde_json::from_str(data);
            match parsed {
                Ok(stop) => Some(Ok(StreamEvent::ContentBlockStop { index: stop.index })),
                Err(e) => {
                    warn!("Failed to parse content_block_stop: {}", e);
                    None
                }
            }
        }
        "message_delta" => {
            let parsed: Result<AnthropicMessageDelta, _> = serde_json::from_str(data);
            match parsed {
                Ok(delta) => {
                    let stop_reason = delta.delta.stop_reason.map(|r| match r.as_str() {
                        "end_turn" => StopReason::EndTurn,
                        "max_tokens" => StopReason::MaxTokens,
                        "stop_sequence" => StopReason::StopSequence,
                        "tool_use" => StopReason::ToolUse,
                        _ => StopReason::EndTurn,
                    });
                    Some(Ok(StreamEvent::MessageDelta {
                        delta: MessageDelta { stop_reason },
                        usage: Some(Usage {
                            input_tokens: 0,
                            output_tokens: delta.usage.output_tokens,
                            cache_creation_input_tokens: None,
                            cache_read_input_tokens: None,
                        }),
                    }))
                }
                Err(e) => {
                    warn!("Failed to parse message_delta: {}", e);
                    None
                }
            }
        }
        "message_stop" => Some(Ok(StreamEvent::MessageStop)),
        "ping" => Some(Ok(StreamEvent::Ping)),
        "error" => {
            let parsed: Result<AnthropicStreamError, _> = serde_json::from_str(data);
            match parsed {
                Ok(err) => Some(Ok(StreamEvent::Error {
                    error: StreamError {
                        error_type: err.error.r#type,
                        message: err.error.message,
                    },
                })),
                Err(e) => Some(Err(ProviderError::new(
                    ProviderErrorKind::Unknown,
                    format!("Failed to parse error event: {}", e),
                ))),
            }
        }
        _ => {
            debug!("Unknown event type: {}", event_type);
            None
        }
    }
}

// ============================================================================
// Anthropic API Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContent {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    Image {
        source: ImageSource,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ImageSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheControl {
    #[serde(rename = "type")]
    cache_type: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: Option<u32>,
    #[serde(default)]
    cache_read_input_tokens: Option<u32>,
}

// Streaming types
#[derive(Debug, Deserialize)]
struct AnthropicStreamStart {
    message: AnthropicStreamMessage,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamMessage {
    id: String,
    model: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicBlockStart {
    index: usize,
    content_block: AnthropicContent,
}

#[derive(Debug, Deserialize)]
struct AnthropicBlockDelta {
    index: usize,
    delta: AnthropicDelta,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
}

#[derive(Debug, Deserialize)]
struct AnthropicBlockStop {
    index: usize,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageDelta {
    delta: AnthropicMessageDeltaInner,
    usage: AnthropicDeltaUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageDeltaInner {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDeltaUsage {
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamError {
    error: AnthropicErrorInner,
}

#[derive(Debug, Deserialize)]
struct AnthropicErrorInner {
    r#type: String,
    message: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_events() {
        let sse_data = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_123","model":"claude-3","usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: message_stop
data: {"type":"message_stop"}

"#;

        let events = parse_sse_events(sse_data);
        assert!(!events.is_empty());

        // Check first event is message_start
        if let Ok(StreamEvent::MessageStart { message }) = &events[0] {
            assert_eq!(message.id, "msg_123");
        } else {
            panic!("Expected MessageStart event");
        }
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new("anthropic", "sk-test")
            .with_base_url("https://api.anthropic.com");

        let provider = AnthropicProvider::new(config);
        assert_eq!(provider.provider_id(), "anthropic");
        assert!(provider.supports_streaming());
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
    }
}
