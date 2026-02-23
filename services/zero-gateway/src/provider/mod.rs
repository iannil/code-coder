//! Multi-provider abstraction for LLM APIs.
//!
//! Provides a unified interface for calling different LLM providers
//! (Anthropic, OpenAI, Gemini, Ollama, OpenRouter, etc.) with consistent request/response formats.

mod anthropic;
mod compatible;
mod gemini;
mod ollama;
mod openai;
mod openrouter;
mod resilient;

pub use anthropic::AnthropicProvider;
pub use compatible::{AuthStyle, CompatibleProvider};
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use openrouter::OpenRouterProvider;
pub use resilient::{ResilienceConfig, ResilientProvider};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// Provider Trait
// ============================================================================

/// Unified interface for LLM providers.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name.
    fn name(&self) -> &str;

    /// Get available models for this provider.
    fn models(&self) -> Vec<&str>;

    /// Check if this provider supports a given model.
    fn supports_model(&self, model: &str) -> bool;

    /// Send a chat completion request.
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
}

/// Error from a provider.
#[derive(Debug, Clone)]
pub struct ProviderError {
    pub provider: String,
    pub model: String,
    pub message: String,
    pub status_code: Option<u16>,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}:{}] {}",
            self.provider, self.model, self.message
        )
    }
}

impl std::error::Error for ProviderError {}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Unified chat request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model to use
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    /// Temperature (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// System prompt (if not in messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Unified chat response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Provider name
    pub provider: String,
    /// Model used
    pub model: String,
    /// Response content
    pub content: String,
    /// Token usage
    pub usage: TokenUsage,
    /// Finish reason
    pub finish_reason: Option<String>,
    /// Response latency in milliseconds
    pub latency_ms: u64,
}

/// Token usage information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

// ============================================================================
// Provider Registry
// ============================================================================

/// Registry of available providers.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
    model_to_provider: HashMap<String, String>,
}

impl ProviderRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            model_to_provider: HashMap::new(),
        }
    }

    /// Register a provider.
    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        let name = provider.name().to_string();

        // Map models to this provider
        for model in provider.models() {
            self.model_to_provider.insert(model.to_string(), name.clone());
        }

        self.providers.insert(name, provider);
    }

    /// Get a provider by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(name).cloned()
    }

    /// Get the provider for a model.
    pub fn get_for_model(&self, model: &str) -> Option<Arc<dyn Provider>> {
        // First check direct model mapping
        if let Some(provider_name) = self.model_to_provider.get(model) {
            return self.providers.get(provider_name).cloned();
        }

        // Check if any provider supports this model
        for provider in self.providers.values() {
            if provider.supports_model(model) {
                return Some(provider.clone());
            }
        }

        None
    }

    /// List all registered providers.
    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    /// List all available models.
    pub fn list_models(&self) -> Vec<String> {
        self.model_to_provider.keys().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a registry with configured providers.
pub fn create_registry(
    anthropic_api_key: Option<&str>,
    openai_api_key: Option<&str>,
) -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();

    if let Some(key) = anthropic_api_key {
        if !key.is_empty() {
            registry.register(Arc::new(AnthropicProvider::new(key)));
        }
    }

    if let Some(key) = openai_api_key {
        if !key.is_empty() {
            registry.register(Arc::new(OpenAIProvider::new(key)));
        }
    }

    registry
}

/// Create a registry with all configured providers from API keys config.
pub fn create_full_registry(api_keys: &zero_common::config::ApiKeysConfig) -> ProviderRegistry {
    let mut registry = ProviderRegistry::new();

    // Primary providers
    if let Some(ref key) = api_keys.anthropic {
        if !key.is_empty() {
            registry.register(Arc::new(AnthropicProvider::new(key)));
        }
    }

    if let Some(ref key) = api_keys.openai {
        if !key.is_empty() {
            registry.register(Arc::new(OpenAIProvider::new(key)));
        }
    }

    if let Some(ref key) = api_keys.google {
        if !key.is_empty() {
            registry.register(Arc::new(GeminiProvider::new(Some(key))));
        }
    } else if GeminiProvider::has_any_auth() {
        // Try CLI auth for Gemini
        registry.register(Arc::new(GeminiProvider::new(None)));
    }

    if let Some(ref key) = api_keys.openrouter {
        if !key.is_empty() {
            registry.register(Arc::new(OpenRouterProvider::new(Some(key))));
        }
    }

    // Always register Ollama (no key required)
    registry.register(Arc::new(OllamaProvider::new(None)));

    // Compatible providers
    if let Some(ref key) = api_keys.groq {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::groq(Some(key))));
        }
    }

    if let Some(ref key) = api_keys.mistral {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::mistral(Some(key))));
        }
    }

    if let Some(ref key) = api_keys.xai {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::xai(Some(key))));
        }
    }

    if let Some(ref key) = api_keys.deepseek {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::deepseek(Some(key))));
        }
    }

    if let Some(ref key) = api_keys.together {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::together(Some(key))));
        }
    }

    if let Some(ref key) = api_keys.fireworks {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::fireworks(Some(key))));
        }
    }

    if let Some(ref key) = api_keys.perplexity {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::perplexity(Some(key))));
        }
    }

    if let Some(ref key) = api_keys.cohere {
        if !key.is_empty() {
            registry.register(Arc::new(CompatibleProvider::cohere(Some(key))));
        }
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry() {
        let mut registry = ProviderRegistry::new();
        assert!(registry.list_providers().is_empty());

        // Register a mock provider would go here
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "claude-sonnet-4".into(),
            messages: vec![Message {
                role: "user".into(),
                content: "Hello".into(),
            }],
            max_tokens: Some(1000),
            temperature: None,
            system: Some("You are helpful.".into()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("claude-sonnet-4"));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_chat_response_serialization() {
        let response = ChatResponse {
            provider: "anthropic".into(),
            model: "claude-sonnet-4".into(),
            content: "Hello!".into(),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            },
            finish_reason: Some("end_turn".into()),
            latency_ms: 500,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("anthropic"));
        assert!(json.contains("500"));
    }
}
