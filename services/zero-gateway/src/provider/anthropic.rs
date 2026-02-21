//! Anthropic (Claude) provider implementation.

use super::{ChatRequest, ChatResponse, Provider, ProviderError, TokenUsage};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Anthropic API provider.
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, "https://api.anthropic.com")
    }

    /// Create with custom base URL.
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&api_key).unwrap_or_else(|_| HeaderValue::from_static("")),
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static("2023-06-01"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            api_key,
            base_url: base_url.into(),
        }
    }

    /// Known Claude models.
    const MODELS: &'static [&'static str] = &[
        "claude-opus-4",
        "claude-sonnet-4",
        "claude-3-5-sonnet-20241022",
        "claude-3-5-haiku-20241022",
        "claude-3-opus-20240229",
        "claude-3-sonnet-20240229",
        "claude-3-haiku-20240307",
    ];
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn models(&self) -> Vec<&str> {
        Self::MODELS.to_vec()
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("claude")
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();
        let url = format!("{}/v1/messages", self.base_url);

        // Convert to Anthropic format
        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|m| AnthropicMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            max_tokens: request.max_tokens.unwrap_or(4096),
            system: request.system,
            temperature: request.temperature,
        };

        let response = self
            .client
            .post(&url)
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| ProviderError {
                provider: "anthropic".into(),
                model: request.model.clone(),
                message: format!("Request failed: {}", e),
                status_code: None,
            })?;

        let status = response.status();
        let latency_ms = start.elapsed().as_millis() as u64;

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError {
                provider: "anthropic".into(),
                model: request.model,
                message: format!("API error: {}", body),
                status_code: Some(status.as_u16()),
            });
        }

        let anthropic_response: AnthropicResponse = response.json().await.map_err(|e| {
            ProviderError {
                provider: "anthropic".into(),
                model: request.model.clone(),
                message: format!("Failed to parse response: {}", e),
                status_code: None,
            }
        })?;

        // Extract text content
        let content = anthropic_response
            .content
            .iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    Some(c.text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(ChatResponse {
            provider: "anthropic".into(),
            model: anthropic_response.model,
            content,
            usage: TokenUsage {
                input_tokens: anthropic_response.usage.input_tokens,
                output_tokens: anthropic_response.usage.output_tokens,
                total_tokens: anthropic_response.usage.input_tokens
                    + anthropic_response.usage.output_tokens,
            },
            finish_reason: Some(anthropic_response.stop_reason.unwrap_or_default()),
            latency_ms,
        })
    }
}

// ============================================================================
// Anthropic API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    model: String,
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: i64,
    output_tokens: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_provider_models() {
        let provider = AnthropicProvider::new("test-key");
        assert!(provider.supports_model("claude-sonnet-4"));
        assert!(provider.supports_model("claude-3-opus-20240229"));
        assert!(!provider.supports_model("gpt-4"));
    }

    #[test]
    fn test_anthropic_request_serialization() {
        let request = AnthropicRequest {
            model: "claude-sonnet-4".into(),
            messages: vec![AnthropicMessage {
                role: "user".into(),
                content: "Hello".into(),
            }],
            max_tokens: 1000,
            system: Some("Be helpful".into()),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-sonnet-4"));
        assert!(json.contains("Be helpful"));
    }
}
