//! Ollama provider for zero-gateway.
//!
//! Connects to local Ollama instance for running local LLM models.

use super::{ChatRequest, ChatResponse, Provider, ProviderError, TokenUsage};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Ollama provider for local models.
pub struct OllamaProvider {
    base_url: String,
    client: Client,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
    #[serde(default)]
    prompt_eval_count: Option<i64>,
    #[serde(default)]
    eval_count: Option<i64>,
    #[serde(default)]
    done_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider.
    ///
    /// # Arguments
    /// * `base_url` - Base URL for Ollama API (defaults to http://localhost:11434)
    pub fn new(base_url: Option<&str>) -> Self {
        Self {
            base_url: base_url
                .unwrap_or("http://localhost:11434")
                .trim_end_matches('/')
                .to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300)) // Ollama runs locally, may be slow
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    /// Create from environment variable or config.
    pub fn from_config(config_url: Option<&str>) -> Self {
        let env_url = std::env::var("OLLAMA_BASE_URL").ok();
        let url = env_url.as_deref().or(config_url);
        Self::new(url)
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn models(&self) -> Vec<&str> {
        // Ollama can run any model, these are common ones
        vec![
            "llama3",
            "llama3:70b",
            "llama2",
            "mistral",
            "mixtral",
            "codellama",
            "phi3",
            "gemma",
            "qwen2",
        ]
    }

    fn supports_model(&self, _model: &str) -> bool {
        // Ollama can potentially run any model if it's installed
        true
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();

        // Convert messages to Ollama format
        let mut messages: Vec<OllamaMessage> = Vec::new();

        // Add system message if provided
        if let Some(ref system) = request.system {
            messages.push(OllamaMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add conversation messages
        for msg in &request.messages {
            messages.push(OllamaMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        let ollama_request = OllamaChatRequest {
            model: request.model.clone(),
            messages,
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature.unwrap_or(0.7),
                num_predict: request.max_tokens,
            },
        };

        let url = format!("{}/api/chat", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| ProviderError {
                provider: "ollama".into(),
                model: request.model.clone(),
                message: format!(
                    "Request failed: {}. Is Ollama running? (brew install ollama && ollama serve)",
                    e
                ),
                status_code: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ProviderError {
                provider: "ollama".into(),
                model: request.model.clone(),
                message: format!(
                    "API error ({}): {}. Is Ollama running? (brew install ollama && ollama serve)",
                    status.as_u16(),
                    error_text
                ),
                status_code: Some(status.as_u16()),
            });
        }

        let result: OllamaChatResponse = response.json().await.map_err(|e| ProviderError {
            provider: "ollama".into(),
            model: request.model.clone(),
            message: format!("Failed to parse response: {}", e),
            status_code: None,
        })?;

        let input_tokens = result.prompt_eval_count.unwrap_or(0);
        let output_tokens = result.eval_count.unwrap_or(0);

        Ok(ChatResponse {
            provider: "ollama".into(),
            model: request.model,
            content: result.message.content,
            usage: TokenUsage {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
            },
            finish_reason: result.done_reason,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_url() {
        let p = OllamaProvider::new(None);
        assert_eq!(p.base_url, "http://localhost:11434");
    }

    #[test]
    fn custom_url_trailing_slash() {
        let p = OllamaProvider::new(Some("http://192.168.1.100:11434/"));
        assert_eq!(p.base_url, "http://192.168.1.100:11434");
    }

    #[test]
    fn custom_url_no_trailing_slash() {
        let p = OllamaProvider::new(Some("http://myserver:11434"));
        assert_eq!(p.base_url, "http://myserver:11434");
    }

    #[test]
    fn provider_name_is_ollama() {
        let p = OllamaProvider::new(None);
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn supports_any_model() {
        let p = OllamaProvider::new(None);
        assert!(p.supports_model("llama3"));
        assert!(p.supports_model("custom-model"));
        assert!(p.supports_model("anything"));
    }

    #[test]
    fn request_serializes_with_system() {
        let req = OllamaChatRequest {
            model: "llama3".to_string(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: "You are ZeroBot".to_string(),
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: "hello".to_string(),
                },
            ],
            stream: false,
            options: OllamaOptions {
                temperature: 0.7,
                num_predict: None,
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"stream\":false"));
        assert!(json.contains("llama3"));
        assert!(json.contains("system"));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn response_deserializes() {
        let json = r#"{"message":{"role":"assistant","content":"Hello from Ollama!"},"prompt_eval_count":10,"eval_count":5}"#;
        let resp: OllamaChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.message.content, "Hello from Ollama!");
        assert_eq!(resp.prompt_eval_count, Some(10));
        assert_eq!(resp.eval_count, Some(5));
    }
}
