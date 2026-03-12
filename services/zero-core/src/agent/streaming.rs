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

// ============================================================================
// OpenAI Provider
// ============================================================================

/// OpenAI GPT streaming provider
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com".into(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl StreamingProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("gpt-") || model.starts_with("o1") || model.starts_with("o3")
    }

    async fn stream(&self, request: StreamRequest) -> Result<EventStream> {
        use futures_util::StreamExt;

        let client = reqwest::Client::new();

        // Convert messages to OpenAI format
        let openai_messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                    Role::Tool => "tool",
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
                            "type": "function",
                            "id": id,
                            "function": {
                                "name": name,
                                "arguments": serde_json::to_string(input).unwrap_or_default()
                            }
                        }),
                        ContentPart::ToolResult { tool_use_id, content } => serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_use_id,
                            "content": content
                        }),
                    })
                    .collect();

                // Simplify single text content
                let content_value = if content.len() == 1 {
                    if let Some(text) = content[0].get("text") {
                        text.clone()
                    } else {
                        serde_json::json!(content)
                    }
                } else {
                    serde_json::json!(content)
                };

                serde_json::json!({
                    "role": role,
                    "content": content_value
                })
            })
            .collect();

        // Prepend system messages
        let mut all_messages = Vec::new();
        for sys in &request.system {
            all_messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }
        all_messages.extend(openai_messages);

        // Build tools array
        let tools: Vec<serde_json::Value> = request
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema
                    }
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": all_messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
            "stream_options": {"include_usage": true}
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if !tools.is_empty() {
            body["tools"] = serde_json::json!(tools);
        }

        let response = client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .body(serde_json::to_string(&body)?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI API error {}: {}", status, text));
        }

        let byte_stream = response.bytes_stream();

        let event_stream = byte_stream
            .map(move |result| -> Result<StreamEvent> {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        parse_openai_sse_event(&text)
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

/// Parse SSE event from OpenAI
fn parse_openai_sse_event(data: &str) -> Result<StreamEvent> {
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
            return parse_openai_event(event);
        }
    }

    Err(anyhow::anyhow!("skip"))
}

/// Parse OpenAI-specific event format
fn parse_openai_event(event: serde_json::Value) -> Result<StreamEvent> {
    // Check for usage in final chunk
    if let Some(usage) = event.get("usage").and_then(|u| u.as_object()) {
        return Ok(StreamEvent::Finish {
            reason: "stop".into(),
            usage: Some(Usage {
                input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as usize,
                output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as usize,
                reasoning_tokens: 0,
                cache_read_tokens: None,
                cache_write_tokens: None,
            }),
        });
    }

    let choices = event["choices"].as_array();
    if choices.is_none() || choices.unwrap().is_empty() {
        return Err(anyhow::anyhow!("skip"));
    }

    let choice = &choices.unwrap()[0];
    let delta = &choice["delta"];

    // Handle content delta
    if let Some(content) = delta["content"].as_str() {
        if !content.is_empty() {
            return Ok(StreamEvent::TextDelta {
                content: content.to_string(),
            });
        }
    }

    // Handle tool calls
    if let Some(tool_calls) = delta["tool_calls"].as_array() {
        for tc in tool_calls {
            if let Some(func) = tc.get("function") {
                // Tool call start
                if let Some(name) = func["name"].as_str() {
                    let id = tc["id"].as_str().unwrap_or("").to_string();
                    return Ok(StreamEvent::ToolCallStart {
                        id,
                        name: name.to_string(),
                    });
                }
                // Tool call delta (arguments)
                if let Some(args) = func["arguments"].as_str() {
                    return Ok(StreamEvent::ToolCallDelta {
                        id: String::new(),
                        arguments_delta: args.to_string(),
                    });
                }
            }
        }
    }

    // Handle finish reason
    if let Some(reason) = choice["finish_reason"].as_str() {
        let mapped_reason = match reason {
            "stop" => "stop",
            "length" => "max_tokens",
            "tool_calls" | "function_call" => "tool_use",
            "content_filter" => "content_filter",
            _ => "stop",
        };
        return Ok(StreamEvent::Finish {
            reason: mapped_reason.to_string(),
            usage: None,
        });
    }

    Err(anyhow::anyhow!("skip"))
}

// ============================================================================
// Google Provider
// ============================================================================

/// Google Gemini streaming provider
pub struct GoogleProvider {
    api_key: String,
    base_url: String,
}

impl GoogleProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://generativelanguage.googleapis.com".into(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[async_trait]
impl StreamingProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("gemini-")
    }

    async fn stream(&self, request: StreamRequest) -> Result<EventStream> {
        use futures_util::StreamExt;

        let client = reqwest::Client::new();

        // Convert messages to Gemini format
        let mut contents: Vec<serde_json::Value> = Vec::new();

        for msg in &request.messages {
            if msg.role == Role::System {
                // System messages handled separately
                continue;
            }

            let role = match msg.role {
                Role::User | Role::Tool => "user",
                Role::Assistant => "model",
                Role::System => continue,
            };

            let parts: Vec<serde_json::Value> = msg
                .content
                .iter()
                .map(|p| match p {
                    ContentPart::Text { text } => serde_json::json!({ "text": text }),
                    ContentPart::ToolUse { name, input, .. } => serde_json::json!({
                        "functionCall": {
                            "name": name,
                            "args": input
                        }
                    }),
                    ContentPart::ToolResult { tool_use_id, content } => serde_json::json!({
                        "functionResponse": {
                            "name": tool_use_id,
                            "response": { "result": content }
                        }
                    }),
                })
                .collect();

            contents.push(serde_json::json!({
                "role": role,
                "parts": parts
            }));
        }

        // Build tools array (function declarations)
        let tools: Option<serde_json::Value> = if request.tools.is_empty() {
            None
        } else {
            let function_declarations: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema
                    })
                })
                .collect();

            Some(serde_json::json!([{
                "functionDeclarations": function_declarations
            }]))
        };

        // Build generation config
        let mut generation_config = serde_json::json!({
            "maxOutputTokens": request.max_tokens.unwrap_or(4096)
        });

        if let Some(temp) = request.temperature {
            generation_config["temperature"] = serde_json::json!(temp);
        }

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": generation_config
        });

        // Add system instruction if present
        if !request.system.is_empty() {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": request.system.join("\n\n") }]
            });
        }

        if let Some(tools) = tools {
            body["tools"] = tools;
        }

        // Gemini uses query params for streaming
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, request.model, self.api_key
        );

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&body)?)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Google API error {}: {}", status, text));
        }

        let byte_stream = response.bytes_stream();

        let event_stream = byte_stream
            .map(move |result| -> Result<StreamEvent> {
                match result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        parse_google_sse_event(&text)
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

/// Parse SSE event from Google
fn parse_google_sse_event(data: &str) -> Result<StreamEvent> {
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(':') {
            continue;
        }

        if let Some(json_str) = line.strip_prefix("data: ") {
            let event: serde_json::Value = serde_json::from_str(json_str)?;
            return parse_google_event(event);
        }
    }

    Err(anyhow::anyhow!("skip"))
}

/// Parse Google-specific event format
fn parse_google_event(event: serde_json::Value) -> Result<StreamEvent> {
    let candidates = event["candidates"].as_array();
    if candidates.is_none() || candidates.unwrap().is_empty() {
        // Check for usage metadata
        if let Some(usage) = event.get("usageMetadata").and_then(|u| u.as_object()) {
            return Ok(StreamEvent::Finish {
                reason: "stop".into(),
                usage: Some(Usage {
                    input_tokens: usage["promptTokenCount"].as_u64().unwrap_or(0) as usize,
                    output_tokens: usage["candidatesTokenCount"].as_u64().unwrap_or(0) as usize,
                    reasoning_tokens: 0,
                    cache_read_tokens: usage["cachedContentTokenCount"]
                        .as_u64()
                        .map(|v| v as usize),
                    cache_write_tokens: None,
                }),
            });
        }
        return Err(anyhow::anyhow!("skip"));
    }

    let candidate = &candidates.unwrap()[0];
    let content = &candidate["content"];
    let parts = content["parts"].as_array();

    if let Some(parts) = parts {
        for part in parts {
            // Text content
            if let Some(text) = part["text"].as_str() {
                if !text.is_empty() {
                    return Ok(StreamEvent::TextDelta {
                        content: text.to_string(),
                    });
                }
            }

            // Function call
            if let Some(fc) = part.get("functionCall") {
                let name = fc["name"].as_str().unwrap_or("").to_string();
                let args = fc.get("args").cloned().unwrap_or(serde_json::json!({}));
                return Ok(StreamEvent::ToolCall {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    arguments: args,
                });
            }
        }
    }

    // Handle finish reason
    if let Some(reason) = candidate["finishReason"].as_str() {
        let mapped_reason = match reason {
            "STOP" => "stop",
            "MAX_TOKENS" => "max_tokens",
            "SAFETY" | "RECITATION" => "content_filter",
            "TOOL_USE" => "tool_use",
            _ => "stop",
        };
        return Ok(StreamEvent::Finish {
            reason: mapped_reason.to_string(),
            usage: None,
        });
    }

    Err(anyhow::anyhow!("skip"))
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
        assert!(!provider.supports_model("gemini-pro"));
    }

    #[test]
    fn test_openai_provider_model_support() {
        let provider = OpenAIProvider::new("test-key");
        assert!(provider.supports_model("gpt-4o"));
        assert!(provider.supports_model("gpt-4-turbo"));
        assert!(provider.supports_model("o1-preview"));
        assert!(provider.supports_model("o3-mini"));
        assert!(!provider.supports_model("claude-opus-4-5"));
        assert!(!provider.supports_model("gemini-pro"));
    }

    #[test]
    fn test_google_provider_model_support() {
        let provider = GoogleProvider::new("test-key");
        assert!(provider.supports_model("gemini-pro"));
        assert!(provider.supports_model("gemini-1.5-pro"));
        assert!(provider.supports_model("gemini-2.0-flash"));
        assert!(!provider.supports_model("gpt-4o"));
        assert!(!provider.supports_model("claude-opus-4-5"));
    }

    #[test]
    fn test_openai_sse_parsing() {
        let sse_data = r#"data: {"id":"chatcmpl-123","choices":[{"index":0,"delta":{"content":"Hello"}}]}"#;
        let event = parse_openai_sse_event(sse_data).unwrap();
        match event {
            StreamEvent::TextDelta { content } => assert_eq!(content, "Hello"),
            _ => panic!("Expected TextDelta event"),
        }
    }

    #[test]
    fn test_openai_sse_done() {
        let sse_data = "data: [DONE]";
        let event = parse_openai_sse_event(sse_data).unwrap();
        match event {
            StreamEvent::Finish { reason, .. } => assert_eq!(reason, "stop"),
            _ => panic!("Expected Finish event"),
        }
    }

    #[test]
    fn test_google_sse_parsing() {
        let sse_data = r#"data: {"candidates":[{"content":{"parts":[{"text":"World"}]}}]}"#;
        let event = parse_google_sse_event(sse_data).unwrap();
        match event {
            StreamEvent::TextDelta { content } => assert_eq!(content, "World"),
            _ => panic!("Expected TextDelta event"),
        }
    }

    #[test]
    fn test_provider_base_url_override() {
        let anthropic = AnthropicProvider::new("key").with_base_url("https://custom.api");
        assert_eq!(anthropic.name(), "anthropic");

        let openai = OpenAIProvider::new("key").with_base_url("https://custom.openai");
        assert_eq!(openai.name(), "openai");

        let google = GoogleProvider::new("key").with_base_url("https://custom.google");
        assert_eq!(google.name(), "google");
    }
}
