//! OpenAI provider implementation.

use super::{ChatRequest, ChatResponse, Provider, ProviderError, TokenUsage};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// OpenAI API provider.
pub struct OpenAIProvider {
    client: reqwest::Client,
    #[allow(dead_code)] // Stored for potential debugging/introspection
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_base_url(api_key, "https://api.openai.com")
    }

    /// Create with custom base URL (for Azure OpenAI or compatible APIs).
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let api_key = api_key.into();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .unwrap_or_else(|_| HeaderValue::from_static("")),
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

    /// Known OpenAI models.
    const MODELS: &'static [&'static str] = &[
        "gpt-4o",
        "gpt-4o-mini",
        "gpt-4-turbo",
        "gpt-4",
        "gpt-3.5-turbo",
        "o1",
        "o1-mini",
        "o1-preview",
    ];
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn models(&self) -> Vec<&str> {
        Self::MODELS.to_vec()
    }

    fn supports_model(&self, model: &str) -> bool {
        model.starts_with("gpt-") || model.starts_with("o1")
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();
        let url = format!("{}/v1/chat/completions", self.base_url);

        // Convert to OpenAI format
        let mut messages: Vec<OpenAIMessage> = request
            .messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        // Add system message if provided
        if let Some(system) = &request.system {
            messages.insert(
                0,
                OpenAIMessage {
                    role: "system".into(),
                    content: system.clone(),
                },
            );
        }

        let openai_request = OpenAIRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };

        let response = self
            .client
            .post(&url)
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| ProviderError {
                provider: "openai".into(),
                model: request.model.clone(),
                message: format!("Request failed: {}", e),
                status_code: None,
            })?;

        let status = response.status();
        let latency_ms = start.elapsed().as_millis() as u64;

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError {
                provider: "openai".into(),
                model: request.model,
                message: format!("API error: {}", body),
                status_code: Some(status.as_u16()),
            });
        }

        let openai_response: OpenAIResponse = response.json().await.map_err(|e| ProviderError {
            provider: "openai".into(),
            model: request.model.clone(),
            message: format!("Failed to parse response: {}", e),
            status_code: None,
        })?;

        // Extract content from first choice
        let content = openai_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let finish_reason = openai_response
            .choices
            .first()
            .and_then(|c| c.finish_reason.clone());

        Ok(ChatResponse {
            provider: "openai".into(),
            model: openai_response.model,
            content,
            usage: TokenUsage {
                input_tokens: openai_response.usage.prompt_tokens,
                output_tokens: openai_response.usage.completion_tokens,
                total_tokens: openai_response.usage.total_tokens,
            },
            finish_reason,
            latency_ms,
        })
    }
}

// ============================================================================
// OpenAI API Types
// ============================================================================

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    model: String,
    choices: Vec<Choice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
    total_tokens: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_models() {
        let provider = OpenAIProvider::new("test-key");
        assert!(provider.supports_model("gpt-4o"));
        assert!(provider.supports_model("gpt-4-turbo"));
        assert!(provider.supports_model("o1"));
        assert!(!provider.supports_model("claude-sonnet-4"));
    }

    #[test]
    fn test_openai_request_serialization() {
        let request = OpenAIRequest {
            model: "gpt-4o".into(),
            messages: vec![
                OpenAIMessage {
                    role: "system".into(),
                    content: "Be helpful".into(),
                },
                OpenAIMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                },
            ],
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4o"));
        assert!(json.contains("Be helpful"));
    }
}
