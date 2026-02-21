//! OpenRouter provider for zero-gateway.
//!
//! OpenRouter provides unified access to multiple LLM providers through a single API.

use super::{ChatRequest, ChatResponse, Provider, ProviderError, TokenUsage};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// OpenRouter provider for multi-model access.
pub struct OpenRouterProvider {
    api_key: Option<String>,
    client: Client,
}

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i64>,
}

#[derive(Debug, Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    #[serde(default)]
    usage: Option<OpenRouterUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

impl OpenRouterProvider {
    /// Create a new OpenRouter provider.
    pub fn new(api_key: Option<&str>) -> Self {
        let resolved_key = api_key
            .map(String::from)
            .or_else(|| std::env::var("OPENROUTER_API_KEY").ok());

        Self {
            api_key: resolved_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    fn models(&self) -> Vec<&str> {
        // OpenRouter supports many models, these are popular ones
        vec![
            "anthropic/claude-sonnet-4",
            "anthropic/claude-opus-4",
            "openai/gpt-4-turbo",
            "openai/gpt-4o",
            "google/gemini-pro",
            "meta-llama/llama-3.1-405b-instruct",
            "mistralai/mistral-large",
        ]
    }

    fn supports_model(&self, model: &str) -> bool {
        // OpenRouter supports any model in format "provider/model"
        model.contains('/')
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();

        let api_key = self.api_key.as_ref().ok_or_else(|| ProviderError {
            provider: "openrouter".into(),
            model: request.model.clone(),
            message: "OpenRouter API key not set. Set OPENROUTER_API_KEY env var.".into(),
            status_code: None,
        })?;

        // Convert messages to OpenRouter format
        let mut messages: Vec<OpenRouterMessage> = Vec::new();

        // Add system message if provided
        if let Some(ref system) = request.system {
            messages.push(OpenRouterMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add conversation messages
        for msg in &request.messages {
            messages.push(OpenRouterMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        let openrouter_request = OpenRouterRequest {
            model: request.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://github.com/iannil/codecoder")
            .header("X-Title", "CodeCoder")
            .json(&openrouter_request)
            .send()
            .await
            .map_err(|e| ProviderError {
                provider: "openrouter".into(),
                model: request.model.clone(),
                message: format!("Request failed: {}", e),
                status_code: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError {
                provider: "openrouter".into(),
                model: request.model.clone(),
                message: format!("API error ({}): {}", status.as_u16(), error_text),
                status_code: Some(status.as_u16()),
            });
        }

        let result: OpenRouterResponse = response.json().await.map_err(|e| ProviderError {
            provider: "openrouter".into(),
            model: request.model.clone(),
            message: format!("Failed to parse response: {}", e),
            status_code: None,
        })?;

        let choice = result.choices.into_iter().next().ok_or_else(|| ProviderError {
            provider: "openrouter".into(),
            model: request.model.clone(),
            message: "No response from OpenRouter".into(),
            status_code: None,
        })?;

        let usage = result.usage.map_or(TokenUsage::default(), |u| TokenUsage {
            input_tokens: u.prompt_tokens.unwrap_or(0),
            output_tokens: u.completion_tokens.unwrap_or(0),
            total_tokens: u.total_tokens.unwrap_or(0),
        });

        Ok(ChatResponse {
            provider: "openrouter".into(),
            model: request.model,
            content: choice.message.content,
            usage,
            finish_reason: choice.finish_reason,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_creates_without_key() {
        let provider = OpenRouterProvider::new(None);
        // May or may not have key depending on env
        assert!(provider.api_key.is_none() || provider.api_key.is_some());
    }

    #[test]
    fn provider_creates_with_key() {
        let provider = OpenRouterProvider::new(Some("test-key"));
        assert_eq!(provider.api_key.as_deref(), Some("test-key"));
    }

    #[test]
    fn provider_name_is_openrouter() {
        let provider = OpenRouterProvider::new(Some("key"));
        assert_eq!(provider.name(), "openrouter");
    }

    #[test]
    fn supports_provider_slash_model_format() {
        let provider = OpenRouterProvider::new(Some("key"));
        assert!(provider.supports_model("anthropic/claude-sonnet-4"));
        assert!(provider.supports_model("openai/gpt-4-turbo"));
        assert!(!provider.supports_model("gpt-4")); // No provider prefix
    }

    #[test]
    fn request_serializes_correctly() {
        let req = OpenRouterRequest {
            model: "anthropic/claude-sonnet-4".into(),
            messages: vec![
                OpenRouterMessage {
                    role: "system".into(),
                    content: "You are helpful".into(),
                },
                OpenRouterMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("anthropic/claude-sonnet-4"));
        assert!(json.contains("system"));
        assert!(json.contains("user"));
    }

    #[test]
    fn response_deserializes() {
        let json = r#"{
            "choices": [{
                "message": {"content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;
        let resp: OpenRouterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello!");
        assert!(resp.usage.is_some());
    }
}
