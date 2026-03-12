//! Provider Module
//!
//! Unified AI provider implementation supporting multiple LLM services.
//!
//! This module provides:
//! - **types**: Common types for messages, requests, responses
//! - **anthropic**: Claude API provider
//! - **openai**: GPT API provider
//! - **google**: Gemini API provider
//! - **rate_limit**: Rate limiting, retry, and circuit breaker
//! - **transform**: Message transformation utilities (legacy AI SDK format)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      ResilientProvider                           │
//! │        (retry + rate limiting + circuit breaker)                │
//! └─────────────────────────────────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                       Provider trait                             │
//! │              (chat, chat_stream, capabilities)                  │
//! └─────────────────────────────────────────────────────────────────┘
//!           │                    │                    │
//!           ▼                    ▼                    ▼
//! ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
//! │ AnthropicProvider│  │  OpenAIProvider │  │  GoogleProvider │
//! │   (Claude API)   │  │    (GPT API)    │  │  (Gemini API)   │
//! └─────────────────┘  └─────────────────┘  └─────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use zero_core::provider::{
//!     AnthropicProvider, ProviderTrait, ProviderConfig, ChatRequest, Message, MessageRole,
//!     MessageContent, ResilientProvider, RetryConfig,
//! };
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create provider with resilience
//!     let config = ProviderConfig::new("anthropic", "sk-ant-xxxxx");
//!     let provider = ResilientProvider::new(AnthropicProvider::new(config))
//!         .with_retry(RetryConfig::default().with_max_retries(3))
//!         .with_rate_limit(10.0);
//!
//!     // Create request
//!     let request = ChatRequest {
//!         model: "claude-sonnet-4-5-20250514".to_string(),
//!         messages: vec![Message {
//!             role: MessageRole::User,
//!             content: MessageContent::text("Hello, Claude!"),
//!             name: None,
//!         }],
//!         max_tokens: Some(1024),
//!         ..Default::default()
//!     };
//!
//!     // Execute chat
//!     let response = provider.chat(request).await?;
//!     println!("Response: {}", response.text());
//!
//!     Ok(())
//! }
//! ```

use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;

// Sub-modules
pub mod anthropic;
pub mod google;
pub mod openai;
pub mod openai_compat;
pub mod rate_limit;
pub mod transform;
pub mod types;

// Re-export from types (new provider types)
pub use types::{
    ChatRequest, ChatResponse, ContentDelta, ContentPart, ImageSource,
    Message, MessageContent, MessageDelta, MessageRole,
    ModelCapabilities, ModelCost, ModelInfo, ModelLimits,
    ProviderConfig, ProviderError, ProviderErrorKind,
    StopReason, StreamError, StreamEvent, StreamMessage,
    ThinkingConfig, ThinkingType, ToolChoice, ToolDefinition, Usage,
};

// Re-export from transform (legacy AI SDK format)
pub use transform::{
    apply_caching, get_sdk_key, get_temperature, get_top_k, get_top_p,
    normalize_messages, remap_provider_options,
    CacheResult, NormalizeResult,
    // Legacy types with prefix to avoid conflict
    ContentPart as TransformContentPart,
    ModelInfo as TransformModelInfo,
    ProviderMessage, ProviderMessageContent,
};

// Re-export providers
pub use anthropic::AnthropicProvider;
pub use google::GoogleProvider;
pub use openai::OpenAIProvider;

// Re-export OpenAI-compatible providers
pub use openai_compat::{
    DeepSeekProvider, GroqProvider, MistralProvider, OllamaProvider,
    OpenAICompatConfig, OpenAICompatProvider, PerplexityProvider, TogetherProvider,
    DEEPSEEK_API_URL, GROQ_API_URL, MISTRAL_API_URL, OLLAMA_API_URL,
    PERPLEXITY_API_URL, TOGETHER_API_URL,
};

// Re-export rate limiting
pub use rate_limit::{CircuitBreaker, CircuitState, RateLimiter, ResilientProvider, RetryConfig};

// ============================================================================
// Provider Trait
// ============================================================================

/// Core trait for AI providers
///
/// Implementations must provide chat completion and optionally streaming.
/// All providers should handle:
/// - Message conversion to/from provider format
/// - Error parsing and categorization
/// - Tool/function calling if supported
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider identifier (e.g., "anthropic", "openai", "google")
    fn provider_id(&self) -> &str;

    /// Execute a chat completion request
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    /// Execute a streaming chat completion request
    ///
    /// Returns a stream of events that can be processed incrementally.
    /// Default implementation returns an error for providers that don't support streaming.
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        Err(ProviderError::new(
            ProviderErrorKind::InvalidRequest,
            "Streaming not supported by this provider",
        ))
    }

    /// Check if the provider supports streaming
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Check if the provider supports tool/function calling
    fn supports_tools(&self) -> bool {
        false
    }

    /// Check if the provider supports vision/image input
    fn supports_vision(&self) -> bool {
        false
    }
}

// Re-export the trait with an alias for clarity
pub use Provider as ProviderTrait;

// Implement Provider for Box<dyn Provider> to allow dynamic dispatch with resilience
#[async_trait]
impl Provider for Box<dyn Provider> {
    fn provider_id(&self) -> &str {
        (**self).provider_id()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        (**self).chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        (**self).chat_stream(request).await
    }

    fn supports_streaming(&self) -> bool {
        (**self).supports_streaming()
    }

    fn supports_tools(&self) -> bool {
        (**self).supports_tools()
    }

    fn supports_vision(&self) -> bool {
        (**self).supports_vision()
    }
}

// ============================================================================
// Provider Factory
// ============================================================================

/// Create a provider instance by ID
///
/// # Arguments
/// * `provider_id` - The provider identifier ("anthropic", "openai", "google")
/// * `config` - Provider configuration including API key
///
/// # Returns
/// A boxed provider instance or an error if the provider is not supported
pub fn create_provider(
    provider_id: &str,
    config: ProviderConfig,
) -> Result<Box<dyn Provider>, ProviderError> {
    match provider_id {
        "anthropic" => Ok(Box::new(AnthropicProvider::new(config))),
        "openai" => Ok(Box::new(OpenAIProvider::new(config))),
        "google" | "gemini" => Ok(Box::new(GoogleProvider::new(config))),
        "ollama" => Ok(Box::new(OllamaProvider::new(config))),
        "groq" => Ok(Box::new(GroqProvider::new(config))),
        "mistral" => Ok(Box::new(MistralProvider::new(config))),
        "together" => Ok(Box::new(TogetherProvider::new(config))),
        "perplexity" => Ok(Box::new(PerplexityProvider::new(config))),
        "deepseek" => Ok(Box::new(DeepSeekProvider::new(config))),
        _ => Err(ProviderError::new(
            ProviderErrorKind::InvalidRequest,
            format!("Unknown provider: {}", provider_id),
        )),
    }
}

/// Create a resilient provider with default retry and rate limiting
pub fn create_resilient_provider(
    provider_id: &str,
    config: ProviderConfig,
) -> Result<ResilientProvider<Box<dyn Provider>>, ProviderError> {
    let inner = create_provider(provider_id, config)?;
    Ok(ResilientProvider::new(inner)
        .with_retry(RetryConfig::default())
        .with_rate_limit(10.0))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get recommended sampling parameters for a model
pub fn get_sampling_params(model_id: &str) -> SamplingParams {
    let id = model_id.to_lowercase();

    let temperature = if id.contains("qwen") {
        Some(0.55)
    } else if id.contains("gemini") {
        Some(1.0)
    } else if id.contains("glm-4") {
        Some(1.0)
    } else if id.contains("minimax") {
        Some(1.0)
    } else if id.contains("kimi") {
        if id.contains("thinking") {
            Some(1.0)
        } else {
            Some(0.6)
        }
    } else {
        None
    };

    let top_p = if id.contains("qwen") {
        Some(1.0)
    } else if id.contains("minimax") {
        Some(0.95)
    } else if id.contains("gemini") {
        Some(0.95)
    } else {
        None
    };

    let top_k = if id.contains("minimax") {
        if id.contains("m2.1") {
            Some(40)
        } else {
            Some(20)
        }
    } else if id.contains("gemini") {
        Some(64)
    } else {
        None
    };

    SamplingParams {
        temperature,
        top_p,
        top_k,
    }
}

/// Sampling parameters for model configuration
#[derive(Debug, Clone, Default)]
pub struct SamplingParams {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<u32>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider() {
        let config = ProviderConfig::new("anthropic", "sk-test");
        let provider = create_provider("anthropic", config);
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().provider_id(), "anthropic");
    }

    #[test]
    fn test_create_provider_unknown() {
        let config = ProviderConfig::new("unknown", "key");
        let provider = create_provider("unknown", config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_sampling_params() {
        let params = get_sampling_params("qwen-72b");
        assert_eq!(params.temperature, Some(0.55));
        assert_eq!(params.top_p, Some(1.0));

        let params = get_sampling_params("claude-3-opus");
        assert_eq!(params.temperature, None);

        let params = get_sampling_params("gemini-pro");
        assert_eq!(params.temperature, Some(1.0));
        assert_eq!(params.top_k, Some(64));
    }
}
