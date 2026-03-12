//! OpenAI API Provider
//!
//! Implementation of the OpenAI Chat Completions API for GPT models.
//! Supports streaming, tool use, and function calling.

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
    StopReason, StreamEvent, ToolChoice, Usage,
};
use super::Provider;

// ============================================================================
// Constants
// ============================================================================

const OPENAI_API_URL: &str = "https://api.openai.com";

// ============================================================================
// OpenAI Provider
// ============================================================================

/// OpenAI API provider
pub struct OpenAIProvider {
    client: Client,
    config: ProviderConfig,
    base_url: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| OPENAI_API_URL.to_string());

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
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.config.api_key)
                .parse()
                .expect("Invalid API key"),
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

    /// Convert internal message format to OpenAI format
    fn convert_message(&self, msg: &Message) -> OpenAIMessage {
        let role = match msg.role {
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::Tool => "tool".to_string(),
        };

        let (content, tool_calls, tool_call_id) = match &msg.content {
            MessageContent::Text(text) => (Some(OpenAIContent::Text(text.clone())), None, None),
            MessageContent::Parts(parts) => {
                let mut text_parts = Vec::new();
                let mut tool_calls_vec = Vec::new();
                let mut tool_call_id_val = None;

                for part in parts {
                    match part {
                        ContentPart::Text { text } => {
                            text_parts.push(OpenAIContentPart::Text { text: text.clone() });
                        }
                        ContentPart::Image { image_source } => {
                            let url = match image_source {
                                super::types::ImageSource::Base64 { media_type, data } => {
                                    format!("data:{};base64,{}", media_type, data)
                                }
                                super::types::ImageSource::Url { url } => url.clone(),
                            };
                            text_parts.push(OpenAIContentPart::ImageUrl {
                                image_url: ImageUrl { url },
                            });
                        }
                        ContentPart::ToolUse { id, name, input } => {
                            tool_calls_vec.push(OpenAIToolCall {
                                id: id.clone(),
                                r#type: "function".to_string(),
                                function: OpenAIFunctionCall {
                                    name: name.clone(),
                                    arguments: serde_json::to_string(input).unwrap_or_default(),
                                },
                            });
                        }
                        ContentPart::ToolResult {
                            tool_use_id,
                            content,
                            ..
                        } => {
                            tool_call_id_val = Some(tool_use_id.clone());
                            text_parts.push(OpenAIContentPart::Text {
                                text: content.clone(),
                            });
                        }
                        ContentPart::Thinking { .. } => {
                            // OpenAI doesn't have native thinking support
                        }
                    }
                }

                let content = if text_parts.is_empty() {
                    None
                } else if text_parts.len() == 1 {
                    match &text_parts[0] {
                        OpenAIContentPart::Text { text } => Some(OpenAIContent::Text(text.clone())),
                        _ => Some(OpenAIContent::Parts(text_parts)),
                    }
                } else {
                    Some(OpenAIContent::Parts(text_parts))
                };

                let tool_calls = if tool_calls_vec.is_empty() {
                    None
                } else {
                    Some(tool_calls_vec)
                };

                (content, tool_calls, tool_call_id_val)
            }
        };

        OpenAIMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
            name: msg.name.clone(),
        }
    }

    /// Convert OpenAI response to internal format
    fn convert_response(&self, resp: OpenAIResponse) -> ChatResponse {
        let choice = resp.choices.into_iter().next().unwrap_or_default();

        let mut content = Vec::new();

        // Add text content
        if let Some(c) = choice.message.content {
            content.push(ContentPart::Text { text: c });
        }

        // Add tool calls
        if let Some(tool_calls) = choice.message.tool_calls {
            for tc in tool_calls {
                let input: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                content.push(ContentPart::ToolUse {
                    id: tc.id,
                    name: tc.function.name,
                    input,
                });
            }
        }

        let stop_reason = choice.finish_reason.map(|r| match r.as_str() {
            "stop" => StopReason::EndTurn,
            "length" => StopReason::MaxTokens,
            "tool_calls" | "function_call" => StopReason::ToolUse,
            "content_filter" => StopReason::ContentFilter,
            _ => StopReason::EndTurn,
        });

        let usage = resp.usage.map(|u| Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: u.prompt_tokens_details.as_ref().and_then(|d| d.cached_tokens),
        }).unwrap_or_default();

        ChatResponse {
            id: resp.id,
            model: resp.model,
            content,
            stop_reason,
            usage,
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
                "invalid_api_key" | "authentication_error" => ProviderErrorKind::Authentication,
                "rate_limit_exceeded" => ProviderErrorKind::RateLimit {
                    retry_after_ms: None,
                },
                "invalid_request_error" => ProviderErrorKind::InvalidRequest,
                "model_not_found" => ProviderErrorKind::ModelNotFound,
                "server_error" => ProviderErrorKind::Server,
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
impl Provider for OpenAIProvider {
    fn provider_id(&self) -> &str {
        "openai"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        // Convert messages
        let messages: Vec<OpenAIMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        // Build request body
        let mut body = json!({
            "model": request.model,
            "messages": messages,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(stops) = &request.stop_sequences {
            body["stop"] = json!(stops);
        }

        // Convert tools
        if let Some(tools) = &request.tools {
            let openai_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect();
            body["tools"] = json!(openai_tools);
        }

        // Convert tool choice
        if let Some(choice) = &request.tool_choice {
            body["tool_choice"] = match choice {
                ToolChoice::Auto => json!("auto"),
                ToolChoice::Any => json!("required"),
                ToolChoice::None => json!("none"),
                ToolChoice::Tool { name } => json!({"type": "function", "function": {"name": name}}),
            };
        }

        debug!("OpenAI request: {}", serde_json::to_string_pretty(&body).unwrap_or_default());

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

        let resp: OpenAIResponse = serde_json::from_str(&body_text).map_err(|e| {
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
        let url = format!("{}/v1/chat/completions", self.base_url);

        // Convert messages
        let messages: Vec<OpenAIMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        // Build request body with streaming enabled
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
            "stream_options": {"include_usage": true},
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(stops) = &request.stop_sequences {
            body["stop"] = json!(stops);
        }

        // Convert tools
        if let Some(tools) = &request.tools {
            let openai_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        }
                    })
                })
                .collect();
            body["tools"] = json!(openai_tools);
        }

        // Convert tool choice
        if let Some(choice) = &request.tool_choice {
            body["tool_choice"] = match choice {
                ToolChoice::Auto => json!("auto"),
                ToolChoice::Any => json!("required"),
                ToolChoice::None => json!("none"),
                ToolChoice::Tool { name } => json!({"type": "function", "function": {"name": name}}),
            };
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

        let model = request.model.clone();
        let event_stream = stream
            .map(move |result| {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        parse_openai_sse_events(&text, &model)
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

/// Parse OpenAI SSE events
fn parse_openai_sse_events(text: &str, model: &str) -> Vec<Result<StreamEvent, ProviderError>> {
    let mut events = Vec::new();

    for line in text.lines() {
        if line.starts_with("data: ") {
            let data = &line[6..];
            if data == "[DONE]" {
                events.push(Ok(StreamEvent::MessageStop));
                continue;
            }

            match serde_json::from_str::<OpenAIStreamChunk>(data) {
                Ok(chunk) => {
                    // Check for usage in final chunk
                    if let Some(usage) = chunk.usage {
                        events.push(Ok(StreamEvent::MessageDelta {
                            delta: MessageDelta { stop_reason: None },
                            usage: Some(Usage {
                                input_tokens: usage.prompt_tokens,
                                output_tokens: usage.completion_tokens,
                                cache_creation_input_tokens: None,
                                cache_read_input_tokens: None,
                            }),
                        }));
                    }

                    for choice in chunk.choices {
                        // Handle content delta
                        if let Some(content) = choice.delta.content {
                            if !content.is_empty() {
                                events.push(Ok(StreamEvent::ContentBlockDelta {
                                    index: choice.index,
                                    delta: ContentDelta::TextDelta { text: content },
                                }));
                            }
                        }

                        // Handle tool calls
                        if let Some(tool_calls) = choice.delta.tool_calls {
                            for tc in tool_calls {
                                if let Some(func) = tc.function {
                                    if let Some(args) = func.arguments {
                                        events.push(Ok(StreamEvent::ContentBlockDelta {
                                            index: tc.index.unwrap_or(0),
                                            delta: ContentDelta::InputJsonDelta {
                                                partial_json: args,
                                            },
                                        }));
                                    }
                                }
                            }
                        }

                        // Handle finish reason
                        if let Some(reason) = choice.finish_reason {
                            let stop_reason = match reason.as_str() {
                                "stop" => StopReason::EndTurn,
                                "length" => StopReason::MaxTokens,
                                "tool_calls" | "function_call" => StopReason::ToolUse,
                                "content_filter" => StopReason::ContentFilter,
                                _ => StopReason::EndTurn,
                            };
                            events.push(Ok(StreamEvent::MessageDelta {
                                delta: MessageDelta {
                                    stop_reason: Some(stop_reason),
                                },
                                usage: None,
                            }));
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse OpenAI stream chunk: {}", e);
                }
            }
        }
    }

    events
}

// ============================================================================
// OpenAI API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<OpenAIContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum OpenAIContent {
    Text(String),
    Parts(Vec<OpenAIContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAIContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    r#type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Deserialize)]
struct PromptTokensDetails {
    cached_tokens: Option<u32>,
}

// Streaming types
#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    id: String,
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    index: usize,
    delta: OpenAIStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIStreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamToolCall {
    index: Option<usize>,
    function: Option<OpenAIStreamFunction>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamFunction {
    arguments: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new("openai", "sk-test")
            .with_base_url("https://api.openai.com");

        let provider = OpenAIProvider::new(config);
        assert_eq!(provider.provider_id(), "openai");
        assert!(provider.supports_streaming());
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
    }

    #[test]
    fn test_parse_sse_events() {
        let sse_data = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]
"#;

        let events = parse_openai_sse_events(sse_data, "gpt-4");
        assert!(!events.is_empty());
    }
}
