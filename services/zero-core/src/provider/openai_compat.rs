//! OpenAI-Compatible Providers
//!
//! Many AI providers offer APIs compatible with OpenAI's Chat Completions format.
//! This module provides base functionality for these providers.
//!
//! Supported providers:
//! - **Ollama**: Local LLM runner
//! - **Groq**: Fast inference with Groq hardware
//! - **Mistral**: Mistral AI models
//! - **Together**: Together AI inference
//! - **Perplexity**: Search-augmented AI
//! - **DeepSeek**: DeepSeek AI models

use std::pin::Pin;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, warn};

use super::types::{
    ChatRequest, ChatResponse, ContentDelta, ContentPart, ImageSource, Message, MessageContent,
    MessageDelta, MessageRole, ProviderConfig, ProviderError, ProviderErrorKind, StopReason,
    StreamEvent, ToolChoice, Usage,
};
use super::Provider;

// ============================================================================
// Provider Endpoints
// ============================================================================

/// Ollama default endpoint
pub const OLLAMA_API_URL: &str = "http://localhost:11434";

/// Groq API endpoint
pub const GROQ_API_URL: &str = "https://api.groq.com/openai";

/// Mistral API endpoint
pub const MISTRAL_API_URL: &str = "https://api.mistral.ai";

/// Together AI endpoint
pub const TOGETHER_API_URL: &str = "https://api.together.xyz";

/// Perplexity API endpoint
pub const PERPLEXITY_API_URL: &str = "https://api.perplexity.ai";

/// DeepSeek API endpoint
pub const DEEPSEEK_API_URL: &str = "https://api.deepseek.com";

// ============================================================================
// OpenAI-Compatible Message Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAICompatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAICompatContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAICompatToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAICompatContent {
    Text(String),
    Parts(Vec<OpenAICompatContentPart>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenAICompatContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatToolCall {
    pub id: String,
    pub r#type: String,
    pub function: OpenAICompatFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompatFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAICompatRequest {
    pub model: String,
    pub messages: Vec<OpenAICompatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAICompatTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAICompatTool {
    pub r#type: String,
    pub function: OpenAICompatFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAICompatFunction {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenAICompatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAICompatChoice>,
    #[serde(default)]
    pub usage: Option<OpenAICompatUsage>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenAICompatChoice {
    pub index: u32,
    pub message: OpenAICompatResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenAICompatResponseMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<OpenAICompatToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAICompatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct OpenAICompatStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAICompatStreamChoice>,
    #[serde(default)]
    pub usage: Option<OpenAICompatUsage>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAICompatStreamChoice {
    pub index: u32,
    pub delta: OpenAICompatDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenAICompatDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<OpenAICompatToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAICompatToolCallDelta {
    pub index: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub function: Option<OpenAICompatFunctionDelta>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenAICompatFunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAICompatError {
    pub error: OpenAICompatErrorDetail,
}

#[derive(Debug, Deserialize)]
pub struct OpenAICompatErrorDetail {
    pub message: String,
    pub r#type: Option<String>,
    pub code: Option<String>,
}

// ============================================================================
// Base Provider Implementation
// ============================================================================

/// Configuration for OpenAI-compatible providers
#[derive(Debug, Clone)]
pub struct OpenAICompatConfig {
    /// Provider identifier
    pub provider_id: String,
    /// Base API URL
    pub base_url: String,
    /// API key
    pub api_key: String,
    /// Custom headers
    pub headers: std::collections::HashMap<String, String>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether tools are supported
    pub supports_tools: bool,
    /// Whether vision is supported
    pub supports_vision: bool,
}

impl OpenAICompatConfig {
    pub fn new(provider_id: impl Into<String>, base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            headers: std::collections::HashMap::new(),
            timeout_ms: 120_000,
            supports_tools: true,
            supports_vision: false,
        }
    }

    pub fn with_tools(mut self, supported: bool) -> Self {
        self.supports_tools = supported;
        self
    }

    pub fn with_vision(mut self, supported: bool) -> Self {
        self.supports_vision = supported;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Base provider for OpenAI-compatible APIs
pub struct OpenAICompatProvider {
    client: Client,
    config: OpenAICompatConfig,
}

impl OpenAICompatProvider {
    /// Create a new OpenAI-compatible provider
    pub fn new(config: OpenAICompatConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Get the chat completions endpoint URL
    fn chat_url(&self) -> String {
        format!("{}/v1/chat/completions", self.config.base_url)
    }

    /// Build request headers
    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        // Authorization header (most providers use Bearer token)
        if !self.config.api_key.is_empty() {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.config.api_key)
                    .parse()
                    .expect("Invalid API key"),
            );
        }

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

    /// Convert internal message to OpenAI-compatible format
    fn convert_message(&self, msg: &Message) -> OpenAICompatMessage {
        let role = match msg.role {
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
            MessageRole::Tool => "tool".to_string(),
        };

        let (content, tool_calls, tool_call_id) = match &msg.content {
            MessageContent::Text(text) => (Some(OpenAICompatContent::Text(text.clone())), None, None),
            MessageContent::Parts(parts) => {
                let mut text_parts = Vec::new();
                let mut tool_calls_vec = Vec::new();
                let mut tool_call_id_val = None;

                for part in parts {
                    match part {
                        ContentPart::Text { text } => {
                            text_parts.push(OpenAICompatContentPart::Text { text: text.clone() });
                        }
                        ContentPart::Image { image_source } => {
                            let url = match image_source {
                                ImageSource::Base64 { media_type, data } => {
                                    format!("data:{};base64,{}", media_type, data)
                                }
                                ImageSource::Url { url } => url.clone(),
                            };
                            text_parts.push(OpenAICompatContentPart::ImageUrl {
                                image_url: ImageUrl { url },
                            });
                        }
                        ContentPart::ToolUse { id, name, input } => {
                            tool_calls_vec.push(OpenAICompatToolCall {
                                id: id.clone(),
                                r#type: "function".to_string(),
                                function: OpenAICompatFunctionCall {
                                    name: name.clone(),
                                    arguments: serde_json::to_string(input).unwrap_or_default(),
                                },
                            });
                        }
                        ContentPart::ToolResult { tool_use_id, content, .. } => {
                            tool_call_id_val = Some(tool_use_id.clone());
                            text_parts.push(OpenAICompatContentPart::Text { text: content.clone() });
                        }
                        ContentPart::Thinking { .. } => {
                            // Most providers don't have native thinking support
                        }
                    }
                }

                let content = if text_parts.is_empty() {
                    None
                } else if text_parts.len() == 1 {
                    match &text_parts[0] {
                        OpenAICompatContentPart::Text { text } => Some(OpenAICompatContent::Text(text.clone())),
                        _ => Some(OpenAICompatContent::Parts(text_parts)),
                    }
                } else {
                    Some(OpenAICompatContent::Parts(text_parts))
                };

                let tool_calls = if tool_calls_vec.is_empty() { None } else { Some(tool_calls_vec) };
                (content, tool_calls, tool_call_id_val)
            }
        };

        OpenAICompatMessage {
            role,
            content,
            tool_calls,
            tool_call_id,
            name: msg.name.clone(),
        }
    }

    /// Convert response to internal format
    fn convert_response(&self, resp: OpenAICompatResponse) -> ChatResponse {
        let choice = resp.choices.into_iter().next().unwrap_or_default();

        let mut content = Vec::new();

        if let Some(c) = choice.message.content {
            content.push(ContentPart::Text { text: c });
        }

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
            cache_read_input_tokens: None,
        }).unwrap_or_else(|| Usage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        });

        ChatResponse {
            id: resp.id,
            model: resp.model,
            content,
            stop_reason,
            usage,
            raw: None,
        }
    }

    /// Build the request body
    fn build_request(&self, request: ChatRequest, stream: bool) -> OpenAICompatRequest {
        let messages: Vec<OpenAICompatMessage> = request
            .messages
            .iter()
            .map(|m| self.convert_message(m))
            .collect();

        let tools = if self.config.supports_tools {
            request.tools.map(|tools| {
                tools
                    .into_iter()
                    .map(|t| OpenAICompatTool {
                        r#type: "function".to_string(),
                        function: OpenAICompatFunction {
                            name: t.name,
                            description: Some(t.description),
                            parameters: t.input_schema,
                        },
                    })
                    .collect()
            })
        } else {
            None
        };

        let tool_choice = if self.config.supports_tools {
            request.tool_choice.map(|tc| match tc {
                ToolChoice::Auto => json!("auto"),
                ToolChoice::Any => json!("required"),
                ToolChoice::Tool { name } => json!({"type": "function", "function": {"name": name}}),
                ToolChoice::None => json!("none"),
            })
        } else {
            None
        };

        OpenAICompatRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop_sequences,
            stream: Some(stream),
            tools,
            tool_choice,
        }
    }

    /// Parse error response
    fn parse_error(&self, status: StatusCode, body: &str) -> ProviderError {
        if let Ok(err) = serde_json::from_str::<OpenAICompatError>(body) {
            let kind = match status {
                StatusCode::UNAUTHORIZED => ProviderErrorKind::Authentication,
                StatusCode::TOO_MANY_REQUESTS => ProviderErrorKind::RateLimit { retry_after_ms: None },
                StatusCode::BAD_REQUEST => ProviderErrorKind::InvalidRequest,
                StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE => ProviderErrorKind::Server,
                _ => ProviderErrorKind::Unknown,
            };
            ProviderError::new(kind, err.error.message)
        } else {
            ProviderError::new(
                ProviderErrorKind::Unknown,
                format!("HTTP {}: {}", status, body),
            )
        }
    }
}

#[async_trait]
impl Provider for OpenAICompatProvider {
    fn provider_id(&self) -> &str {
        &self.config.provider_id
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let body = self.build_request(request, false);

        debug!("Sending chat request to {}", self.chat_url());

        let response = self
            .client
            .post(&self.chat_url())
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::new(ProviderErrorKind::Network, e.to_string()))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| ProviderError::new(ProviderErrorKind::Network, e.to_string()))?;

        if !status.is_success() {
            return Err(self.parse_error(status, &text));
        }

        let resp: OpenAICompatResponse = serde_json::from_str(&text).map_err(|e| {
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
        let body = self.build_request(request.clone(), true);

        debug!("Starting stream to {}", self.chat_url());

        let response = self
            .client
            .post(&self.chat_url())
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::new(ProviderErrorKind::Network, e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .map_err(|e| ProviderError::new(ProviderErrorKind::Network, e.to_string()))?;
            return Err(self.parse_error(status, &text));
        }

        let model = request.model.clone();
        let stream = response.bytes_stream();

        let event_stream = stream
            .map(move |chunk_result| {
                chunk_result
                    .map_err(|e| ProviderError::new(ProviderErrorKind::Network, e.to_string()))
            })
            .flat_map(move |chunk_result| {
                let model = model.clone();
                futures_util::stream::iter(match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        parse_sse_events(&text, &model)
                    }
                    Err(e) => vec![Err(e)],
                })
            });

        Ok(Box::pin(event_stream))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        self.config.supports_tools
    }

    fn supports_vision(&self) -> bool {
        self.config.supports_vision
    }
}

/// Parse SSE events from a chunk of text
fn parse_sse_events(text: &str, _model: &str) -> Vec<Result<StreamEvent, ProviderError>> {
    let mut events = Vec::new();

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with(':') {
            continue;
        }

        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                events.push(Ok(StreamEvent::MessageStop));
                continue;
            }

            match serde_json::from_str::<OpenAICompatStreamChunk>(data) {
                Ok(chunk) => {
                    for choice in chunk.choices {
                        // Handle text content delta
                        if let Some(content) = choice.delta.content {
                            events.push(Ok(StreamEvent::ContentBlockDelta {
                                index: choice.index as usize,
                                delta: ContentDelta::TextDelta { text: content },
                            }));
                        }

                        // Handle tool call deltas
                        if let Some(tool_calls) = choice.delta.tool_calls {
                            for tc in tool_calls {
                                if let Some(ref func) = tc.function {
                                    if let Some(ref args) = func.arguments {
                                        events.push(Ok(StreamEvent::ContentBlockDelta {
                                            index: tc.index as usize,
                                            delta: ContentDelta::InputJsonDelta {
                                                partial_json: args.clone(),
                                            },
                                        }));
                                    }
                                }
                            }
                        }

                        // Handle finish reason
                        if let Some(ref reason) = choice.finish_reason {
                            let stop_reason = match reason.as_str() {
                                "stop" => StopReason::EndTurn,
                                "length" => StopReason::MaxTokens,
                                "tool_calls" => StopReason::ToolUse,
                                _ => StopReason::EndTurn,
                            };

                            let usage = chunk.usage.clone().map(|u| Usage {
                                input_tokens: u.prompt_tokens,
                                output_tokens: u.completion_tokens,
                                cache_creation_input_tokens: None,
                                cache_read_input_tokens: None,
                            });

                            events.push(Ok(StreamEvent::MessageDelta {
                                delta: MessageDelta {
                                    stop_reason: Some(stop_reason),
                                },
                                usage,
                            }));
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse stream chunk: {}", e);
                }
            }
        }
    }

    events
}

// ============================================================================
// Concrete Provider Implementations
// ============================================================================

/// Ollama provider for local LLM inference
pub struct OllamaProvider {
    inner: OpenAICompatProvider,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    ///
    /// # Arguments
    /// * `config` - Provider configuration (api_key can be empty for local Ollama)
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| OLLAMA_API_URL.to_string());

        let compat_config = OpenAICompatConfig::new("ollama", base_url, &config.api_key)
            .with_tools(true)
            .with_vision(true);

        Self {
            inner: OpenAICompatProvider::new(compat_config),
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn provider_id(&self) -> &str {
        "ollama"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        self.inner.chat_stream(request).await
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

/// Groq provider for fast inference
pub struct GroqProvider {
    inner: OpenAICompatProvider,
}

impl GroqProvider {
    /// Create a new Groq provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| GROQ_API_URL.to_string());

        let compat_config = OpenAICompatConfig::new("groq", base_url, &config.api_key)
            .with_tools(true)
            .with_vision(false);

        Self {
            inner: OpenAICompatProvider::new(compat_config),
        }
    }
}

#[async_trait]
impl Provider for GroqProvider {
    fn provider_id(&self) -> &str {
        "groq"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        self.inner.chat_stream(request).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        false
    }
}

/// Mistral AI provider
pub struct MistralProvider {
    inner: OpenAICompatProvider,
}

impl MistralProvider {
    /// Create a new Mistral provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| MISTRAL_API_URL.to_string());

        let compat_config = OpenAICompatConfig::new("mistral", base_url, &config.api_key)
            .with_tools(true)
            .with_vision(false);

        Self {
            inner: OpenAICompatProvider::new(compat_config),
        }
    }
}

#[async_trait]
impl Provider for MistralProvider {
    fn provider_id(&self) -> &str {
        "mistral"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        self.inner.chat_stream(request).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        false
    }
}

/// Together AI provider
pub struct TogetherProvider {
    inner: OpenAICompatProvider,
}

impl TogetherProvider {
    /// Create a new Together AI provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| TOGETHER_API_URL.to_string());

        let compat_config = OpenAICompatConfig::new("together", base_url, &config.api_key)
            .with_tools(true)
            .with_vision(true);

        Self {
            inner: OpenAICompatProvider::new(compat_config),
        }
    }
}

#[async_trait]
impl Provider for TogetherProvider {
    fn provider_id(&self) -> &str {
        "together"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        self.inner.chat_stream(request).await
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

/// Perplexity AI provider (search-augmented)
pub struct PerplexityProvider {
    inner: OpenAICompatProvider,
}

impl PerplexityProvider {
    /// Create a new Perplexity provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| PERPLEXITY_API_URL.to_string());

        let compat_config = OpenAICompatConfig::new("perplexity", base_url, &config.api_key)
            .with_tools(false) // Perplexity doesn't support function calling
            .with_vision(false);

        Self {
            inner: OpenAICompatProvider::new(compat_config),
        }
    }
}

#[async_trait]
impl Provider for PerplexityProvider {
    fn provider_id(&self) -> &str {
        "perplexity"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        self.inner.chat_stream(request).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        false
    }

    fn supports_vision(&self) -> bool {
        false
    }
}

/// DeepSeek AI provider
pub struct DeepSeekProvider {
    inner: OpenAICompatProvider,
}

impl DeepSeekProvider {
    /// Create a new DeepSeek provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| DEEPSEEK_API_URL.to_string());

        let compat_config = OpenAICompatConfig::new("deepseek", base_url, &config.api_key)
            .with_tools(true)
            .with_vision(false);

        Self {
            inner: OpenAICompatProvider::new(compat_config),
        }
    }
}

#[async_trait]
impl Provider for DeepSeekProvider {
    fn provider_id(&self) -> &str {
        "deepseek"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        self.inner.chat_stream(request).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config() {
        let config = ProviderConfig::new("ollama", "");
        let provider = OllamaProvider::new(config);
        assert_eq!(provider.provider_id(), "ollama");
        assert!(provider.supports_streaming());
        assert!(provider.supports_tools());
    }

    #[test]
    fn test_groq_config() {
        let config = ProviderConfig::new("groq", "gsk_test");
        let provider = GroqProvider::new(config);
        assert_eq!(provider.provider_id(), "groq");
        assert!(provider.supports_streaming());
        assert!(provider.supports_tools());
        assert!(!provider.supports_vision());
    }

    #[test]
    fn test_sse_parsing() {
        let chunk = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let events = parse_sse_events(chunk, "gpt-4");
        assert_eq!(events.len(), 1);
        assert!(events[0].is_ok());
    }

    #[test]
    fn test_sse_done() {
        let chunk = "data: [DONE]";
        let events = parse_sse_events(chunk, "gpt-4");
        assert_eq!(events.len(), 1);
        match &events[0] {
            Ok(StreamEvent::MessageStop) => {}
            _ => panic!("Expected MessageStop event"),
        }
    }
}
