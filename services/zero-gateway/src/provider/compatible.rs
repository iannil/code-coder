//! Generic OpenAI-compatible provider for zero-gateway.
//!
//! Most LLM APIs follow the same `/v1/chat/completions` format.
//! This module provides a single implementation that works for all of them.
//!
//! Supported providers:
//! - Groq
//! - Mistral
//! - xAI (Grok)
//! - DeepSeek
//! - Together AI
//! - Fireworks AI
//! - Perplexity
//! - Cohere
//! - Venice
//! - Moonshot (Kimi)
//! - GLM (Zhipu)
//! - MiniMax
//! - Qianfan (Baidu)
//! - Any custom OpenAI-compatible endpoint

use super::{ChatRequest, ChatResponse, Provider, ProviderError, TokenUsage};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// How the provider expects the API key to be sent.
#[derive(Debug, Clone)]
pub enum AuthStyle {
    /// `Authorization: Bearer <key>`
    Bearer,
    /// `x-api-key: <key>` (used by some providers)
    XApiKey,
    /// Custom header name
    Custom(String),
}

/// A provider that speaks the OpenAI-compatible chat completions API.
pub struct CompatibleProvider {
    name: String,
    base_url: String,
    api_key: Option<String>,
    auth_style: AuthStyle,
    models: Vec<String>,
    client: Client,
}

#[derive(Debug, Serialize)]
struct CompatibleRequest {
    model: String,
    messages: Vec<CompatibleMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CompatibleMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct CompatibleResponse {
    choices: Vec<CompatibleChoice>,
    #[serde(default)]
    usage: Option<CompatibleUsage>,
}

#[derive(Debug, Deserialize)]
struct CompatibleChoice {
    message: CompatibleResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompatibleResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct CompatibleUsage {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

impl CompatibleProvider {
    /// Create a new OpenAI-compatible provider.
    pub fn new(
        name: &str,
        base_url: &str,
        api_key: Option<&str>,
        auth_style: AuthStyle,
        models: Vec<&str>,
    ) -> Self {
        Self {
            name: name.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.map(ToString::to_string),
            auth_style,
            models: models.into_iter().map(|s| s.to_string()).collect(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// Create Groq provider.
    pub fn groq(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("GROQ_API_KEY").ok());
        Self::new(
            "groq",
            "https://api.groq.com/openai",
            key.as_deref(),
            AuthStyle::Bearer,
            vec!["llama-3.1-70b-versatile", "mixtral-8x7b-32768"],
        )
    }

    /// Create Mistral provider.
    pub fn mistral(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("MISTRAL_API_KEY").ok());
        Self::new(
            "mistral",
            "https://api.mistral.ai",
            key.as_deref(),
            AuthStyle::Bearer,
            vec!["mistral-large-latest", "mistral-small-latest"],
        )
    }

    /// Create xAI (Grok) provider.
    pub fn xai(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("XAI_API_KEY").ok());
        Self::new(
            "xai",
            "https://api.x.ai",
            key.as_deref(),
            AuthStyle::Bearer,
            vec!["grok-beta", "grok-vision-beta"],
        )
    }

    /// Create DeepSeek provider.
    pub fn deepseek(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok());
        Self::new(
            "deepseek",
            "https://api.deepseek.com",
            key.as_deref(),
            AuthStyle::Bearer,
            vec!["deepseek-chat", "deepseek-coder"],
        )
    }

    /// Create Together AI provider.
    pub fn together(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("TOGETHER_API_KEY").ok());
        Self::new(
            "together",
            "https://api.together.xyz",
            key.as_deref(),
            AuthStyle::Bearer,
            vec![
                "meta-llama/Llama-3.3-70B-Instruct-Turbo",
                "mistralai/Mixtral-8x22B-Instruct-v0.1",
            ],
        )
    }

    /// Create Fireworks AI provider.
    pub fn fireworks(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("FIREWORKS_API_KEY").ok());
        Self::new(
            "fireworks",
            "https://api.fireworks.ai/inference",
            key.as_deref(),
            AuthStyle::Bearer,
            vec![
                "accounts/fireworks/models/llama-v3p1-70b-instruct",
                "accounts/fireworks/models/mixtral-8x22b-instruct",
            ],
        )
    }

    /// Create Perplexity provider.
    pub fn perplexity(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("PERPLEXITY_API_KEY").ok());
        Self::new(
            "perplexity",
            "https://api.perplexity.ai",
            key.as_deref(),
            AuthStyle::Bearer,
            vec!["llama-3.1-sonar-large-128k-online", "llama-3.1-sonar-small-128k-online"],
        )
    }

    /// Create Cohere provider.
    pub fn cohere(api_key: Option<&str>) -> Self {
        let key = api_key
            .map(String::from)
            .or_else(|| std::env::var("COHERE_API_KEY").ok());
        Self::new(
            "cohere",
            "https://api.cohere.com/compatibility",
            key.as_deref(),
            AuthStyle::Bearer,
            vec!["command-r-plus", "command-r"],
        )
    }

    /// Create a custom provider with any OpenAI-compatible endpoint.
    pub fn custom(name: &str, base_url: &str, api_key: Option<&str>) -> Self {
        Self::new(name, base_url, api_key, AuthStyle::Bearer, vec![])
    }
}

#[async_trait]
impl Provider for CompatibleProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn models(&self) -> Vec<&str> {
        self.models.iter().map(|s| s.as_str()).collect()
    }

    fn supports_model(&self, model: &str) -> bool {
        // If no models specified (custom provider), support any model
        if self.models.is_empty() {
            return true;
        }
        self.models.iter().any(|m| m == model || model.contains(&self.name))
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();

        let api_key = self.api_key.as_ref().ok_or_else(|| ProviderError {
            provider: self.name.clone(),
            model: request.model.clone(),
            message: format!("{} API key not set. Set the appropriate env var.", self.name),
            status_code: None,
        })?;

        // Convert messages
        let mut messages: Vec<CompatibleMessage> = Vec::new();

        if let Some(ref system) = request.system {
            messages.push(CompatibleMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        for msg in &request.messages {
            messages.push(CompatibleMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        let compatible_request = CompatibleRequest {
            model: request.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut req = self.client.post(&url).json(&compatible_request);

        match &self.auth_style {
            AuthStyle::Bearer => {
                req = req.header("Authorization", format!("Bearer {}", api_key));
            }
            AuthStyle::XApiKey => {
                req = req.header("x-api-key", api_key.as_str());
            }
            AuthStyle::Custom(header) => {
                req = req.header(header.as_str(), api_key.as_str());
            }
        }

        let response = req.send().await.map_err(|e| ProviderError {
            provider: self.name.clone(),
            model: request.model.clone(),
            message: format!("Request failed: {}", e),
            status_code: None,
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError {
                provider: self.name.clone(),
                model: request.model.clone(),
                message: format!("API error ({}): {}", status.as_u16(), error_text),
                status_code: Some(status.as_u16()),
            });
        }

        let result: CompatibleResponse = response.json().await.map_err(|e| ProviderError {
            provider: self.name.clone(),
            model: request.model.clone(),
            message: format!("Failed to parse response: {}", e),
            status_code: None,
        })?;

        let choice = result.choices.into_iter().next().ok_or_else(|| ProviderError {
            provider: self.name.clone(),
            model: request.model.clone(),
            message: format!("No response from {}", self.name),
            status_code: None,
        })?;

        let usage = result.usage.map_or(TokenUsage::default(), |u| TokenUsage {
            input_tokens: u.prompt_tokens.unwrap_or(0),
            output_tokens: u.completion_tokens.unwrap_or(0),
            total_tokens: u.total_tokens.unwrap_or(0),
        });

        Ok(ChatResponse {
            provider: self.name.clone(),
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
    fn groq_provider_creation() {
        let provider = CompatibleProvider::groq(Some("test-key"));
        assert_eq!(provider.name(), "groq");
        assert_eq!(provider.base_url, "https://api.groq.com/openai");
    }

    #[test]
    fn mistral_provider_creation() {
        let provider = CompatibleProvider::mistral(Some("test-key"));
        assert_eq!(provider.name(), "mistral");
    }

    #[test]
    fn deepseek_provider_creation() {
        let provider = CompatibleProvider::deepseek(Some("test-key"));
        assert_eq!(provider.name(), "deepseek");
    }

    #[test]
    fn xai_provider_creation() {
        let provider = CompatibleProvider::xai(Some("test-key"));
        assert_eq!(provider.name(), "xai");
    }

    #[test]
    fn custom_provider_creation() {
        let provider = CompatibleProvider::custom("my-llm", "https://api.example.com", Some("key"));
        assert_eq!(provider.name(), "my-llm");
        assert!(provider.supports_model("any-model")); // Custom supports any model
    }

    #[test]
    fn request_serializes_correctly() {
        let req = CompatibleRequest {
            model: "llama-3.1-70b-versatile".into(),
            messages: vec![
                CompatibleMessage {
                    role: "system".into(),
                    content: "You are helpful".into(),
                },
                CompatibleMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("llama-3.1-70b-versatile"));
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
        let resp: CompatibleResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello!");
    }

    #[test]
    fn strips_trailing_slash() {
        let provider = CompatibleProvider::custom("test", "https://api.example.com/", Some("key"));
        assert_eq!(provider.base_url, "https://api.example.com");
    }
}
