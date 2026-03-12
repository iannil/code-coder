//! Google Gemini API Provider
//!
//! Implementation of the Google Generative AI API for Gemini models.
//! Supports streaming, tool use, and vision.

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

const GOOGLE_API_URL: &str = "https://generativelanguage.googleapis.com";

// ============================================================================
// Google Provider
// ============================================================================

/// Google Gemini API provider
pub struct GoogleProvider {
    client: Client,
    config: ProviderConfig,
    base_url: String,
}

impl GoogleProvider {
    /// Create a new Google provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| GOOGLE_API_URL.to_string());

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

    /// Get API URL with key
    fn api_url(&self, model: &str, stream: bool) -> String {
        let method = if stream {
            "streamGenerateContent"
        } else {
            "generateContent"
        };
        let alt = if stream { "?alt=sse&" } else { "?" };
        format!(
            "{}/v1beta/models/{}:{}{}key={}",
            self.base_url, model, method, alt, self.config.api_key
        )
    }

    /// Convert internal message format to Google format
    fn convert_messages(&self, messages: &[Message]) -> (Option<GeminiSystemInstruction>, Vec<GeminiContent>) {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        for msg in messages {
            if msg.role == MessageRole::System {
                // System message becomes systemInstruction
                let text = msg.content.as_text();
                system_instruction = Some(GeminiSystemInstruction {
                    parts: vec![GeminiPart::Text { text }],
                });
                continue;
            }

            let role = match msg.role {
                MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "model",
                MessageRole::System => continue, // Already handled
            };

            let parts = match &msg.content {
                MessageContent::Text(text) => vec![GeminiPart::Text { text: text.clone() }],
                MessageContent::Parts(parts) => {
                    parts
                        .iter()
                        .filter_map(|p| match p {
                            ContentPart::Text { text } => {
                                Some(GeminiPart::Text { text: text.clone() })
                            }
                            ContentPart::Image { image_source } => match image_source {
                                super::types::ImageSource::Base64 { media_type, data } => {
                                    Some(GeminiPart::InlineData {
                                        inline_data: InlineData {
                                            mime_type: media_type.clone(),
                                            data: data.clone(),
                                        },
                                    })
                                }
                                super::types::ImageSource::Url { url } => {
                                    Some(GeminiPart::FileData {
                                        file_data: FileData {
                                            mime_type: "image/*".to_string(),
                                            file_uri: url.clone(),
                                        },
                                    })
                                }
                            },
                            ContentPart::ToolUse { id, name, input } => {
                                Some(GeminiPart::FunctionCall {
                                    function_call: FunctionCall {
                                        name: name.clone(),
                                        args: input.clone(),
                                    },
                                })
                            }
                            ContentPart::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            } => Some(GeminiPart::FunctionResponse {
                                function_response: FunctionResponse {
                                    name: tool_use_id.clone(),
                                    response: json!({ "result": content }),
                                },
                            }),
                            ContentPart::Thinking { .. } => None,
                        })
                        .collect()
                }
            };

            contents.push(GeminiContent {
                role: role.to_string(),
                parts,
            });
        }

        (system_instruction, contents)
    }

    /// Convert Google response to internal format
    fn convert_response(&self, resp: GeminiResponse, model: &str) -> ChatResponse {
        let candidate = resp.candidates.into_iter().next().unwrap_or_default();

        let mut content = Vec::new();

        for part in candidate.content.parts {
            match part {
                GeminiPart::Text { text } => {
                    content.push(ContentPart::Text { text });
                }
                GeminiPart::FunctionCall { function_call } => {
                    content.push(ContentPart::ToolUse {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: function_call.name,
                        input: function_call.args,
                    });
                }
                _ => {}
            }
        }

        let stop_reason = candidate.finish_reason.map(|r| match r.as_str() {
            "STOP" => StopReason::EndTurn,
            "MAX_TOKENS" => StopReason::MaxTokens,
            "SAFETY" => StopReason::ContentFilter,
            "RECITATION" => StopReason::ContentFilter,
            "TOOL_USE" => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        });

        let usage = resp.usage_metadata.map(|u| Usage {
            input_tokens: u.prompt_token_count.unwrap_or(0),
            output_tokens: u.candidates_token_count.unwrap_or(0),
            cache_creation_input_tokens: None,
            cache_read_input_tokens: u.cached_content_token_count,
        }).unwrap_or_default();

        ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            model: model.to_string(),
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
            let error_code = json
                .get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_i64())
                .unwrap_or(0);

            let error_msg = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(body);

            let kind = match error_code {
                401 | 403 => ProviderErrorKind::Authentication,
                429 => ProviderErrorKind::RateLimit {
                    retry_after_ms: None,
                },
                400 => ProviderErrorKind::InvalidRequest,
                404 => ProviderErrorKind::ModelNotFound,
                500..=599 => ProviderErrorKind::Server,
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
impl Provider for GoogleProvider {
    fn provider_id(&self) -> &str {
        "google"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let url = self.api_url(&request.model, false);

        let (system_instruction, contents) = self.convert_messages(&request.messages);

        // Build request body
        let mut body = json!({
            "contents": contents,
        });

        if let Some(sys) = system_instruction {
            body["systemInstruction"] = json!(sys);
        }

        // Generation config
        let mut generation_config = json!({});

        if let Some(max_tokens) = request.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }

        if let Some(temp) = request.temperature {
            generation_config["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            generation_config["topP"] = json!(top_p);
        }

        if let Some(top_k) = request.top_k {
            generation_config["topK"] = json!(top_k);
        }

        if let Some(stops) = &request.stop_sequences {
            generation_config["stopSequences"] = json!(stops);
        }

        body["generationConfig"] = generation_config;

        // Convert tools
        if let Some(tools) = &request.tools {
            let function_declarations: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = json!([{
                "functionDeclarations": function_declarations,
            }]);
        }

        // Tool config
        if let Some(choice) = &request.tool_choice {
            let mode = match choice {
                ToolChoice::Auto => "AUTO",
                ToolChoice::Any => "ANY",
                ToolChoice::None => "NONE",
                ToolChoice::Tool { .. } => "AUTO", // Gemini doesn't support specific tool
            };
            body["toolConfig"] = json!({
                "functionCallingConfig": {
                    "mode": mode,
                }
            });
        }

        debug!("Google request: {}", serde_json::to_string_pretty(&body).unwrap_or_default());

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

        let resp: GeminiResponse = serde_json::from_str(&body_text).map_err(|e| {
            ProviderError::new(
                ProviderErrorKind::Unknown,
                format!("Failed to parse response: {}", e),
            )
        })?;

        Ok(self.convert_response(resp, &request.model))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        let url = self.api_url(&request.model, true);

        let (system_instruction, contents) = self.convert_messages(&request.messages);

        // Build request body
        let mut body = json!({
            "contents": contents,
        });

        if let Some(sys) = system_instruction {
            body["systemInstruction"] = json!(sys);
        }

        // Generation config
        let mut generation_config = json!({});

        if let Some(max_tokens) = request.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }

        if let Some(temp) = request.temperature {
            generation_config["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            generation_config["topP"] = json!(top_p);
        }

        if let Some(top_k) = request.top_k {
            generation_config["topK"] = json!(top_k);
        }

        if let Some(stops) = &request.stop_sequences {
            generation_config["stopSequences"] = json!(stops);
        }

        body["generationConfig"] = generation_config;

        // Convert tools
        if let Some(tools) = &request.tools {
            let function_declarations: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = json!([{
                "functionDeclarations": function_declarations,
            }]);
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
                        parse_gemini_sse_events(&text, &model)
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

/// Parse Gemini SSE events
fn parse_gemini_sse_events(text: &str, model: &str) -> Vec<Result<StreamEvent, ProviderError>> {
    let mut events = Vec::new();

    for line in text.lines() {
        if line.starts_with("data: ") {
            let data = &line[6..];

            match serde_json::from_str::<GeminiStreamChunk>(data) {
                Ok(chunk) => {
                    for candidate in chunk.candidates {
                        // Handle content parts
                        for (idx, part) in candidate.content.parts.iter().enumerate() {
                            match part {
                                GeminiPart::Text { text } => {
                                    if !text.is_empty() {
                                        events.push(Ok(StreamEvent::ContentBlockDelta {
                                            index: idx,
                                            delta: ContentDelta::TextDelta { text: text.clone() },
                                        }));
                                    }
                                }
                                GeminiPart::FunctionCall { function_call } => {
                                    events.push(Ok(StreamEvent::ContentBlockDelta {
                                        index: idx,
                                        delta: ContentDelta::InputJsonDelta {
                                            partial_json: serde_json::to_string(&function_call.args)
                                                .unwrap_or_default(),
                                        },
                                    }));
                                }
                                _ => {}
                            }
                        }

                        // Handle finish reason
                        if let Some(reason) = &candidate.finish_reason {
                            let stop_reason = match reason.as_str() {
                                "STOP" => StopReason::EndTurn,
                                "MAX_TOKENS" => StopReason::MaxTokens,
                                "SAFETY" | "RECITATION" => StopReason::ContentFilter,
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

                    // Handle usage
                    if let Some(usage) = chunk.usage_metadata {
                        events.push(Ok(StreamEvent::MessageDelta {
                            delta: MessageDelta { stop_reason: None },
                            usage: Some(Usage {
                                input_tokens: usage.prompt_token_count.unwrap_or(0),
                                output_tokens: usage.candidates_token_count.unwrap_or(0),
                                cache_creation_input_tokens: None,
                                cache_read_input_tokens: usage.cached_content_token_count,
                            }),
                        }));
                    }
                }
                Err(e) => {
                    warn!("Failed to parse Gemini stream chunk: {}", e);
                }
            }
        }
    }

    // Always add message stop at the end
    if !events.is_empty() {
        events.push(Ok(StreamEvent::MessageStop));
    }

    events
}

// ============================================================================
// Google Gemini API Types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiSystemInstruction {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineData,
    },
    FileData {
        #[serde(rename = "fileData")]
        file_data: FileData,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCall,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponse,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileData {
    mime_type: String,
    file_uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct FunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiCandidateContent,
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct GeminiCandidateContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    cached_content_token_count: Option<u32>,
}

// Streaming types
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiStreamChunk {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: Option<UsageMetadata>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new("google", "test-key")
            .with_base_url("https://generativelanguage.googleapis.com");

        let provider = GoogleProvider::new(config);
        assert_eq!(provider.provider_id(), "google");
        assert!(provider.supports_streaming());
        assert!(provider.supports_tools());
        assert!(provider.supports_vision());
    }

    #[test]
    fn test_api_url() {
        let config = ProviderConfig::new("google", "test-key");
        let provider = GoogleProvider::new(config);

        let url = provider.api_url("gemini-pro", false);
        assert!(url.contains("generateContent"));
        assert!(url.contains("key=test-key"));

        let stream_url = provider.api_url("gemini-pro", true);
        assert!(stream_url.contains("streamGenerateContent"));
        assert!(stream_url.contains("alt=sse"));
    }
}
