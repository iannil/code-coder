//! Provider trait for LLM backends.
//!
//! Defines the interface that all LLM providers must implement.

use async_trait::async_trait;

/// LLM provider trait.
///
/// Implementations handle authentication, request formatting,
/// and response parsing for specific LLM APIs.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name (e.g., "anthropic", "openai").
    fn name(&self) -> &str;

    /// Chat with the LLM using a system prompt.
    ///
    /// # Arguments
    /// - `system`: Optional system prompt
    /// - `message`: User message
    /// - `model`: Model identifier
    /// - `temperature`: Sampling temperature (0.0 - 1.0)
    ///
    /// # Returns
    /// The assistant's response text
    async fn chat_with_system(
        &self,
        system: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String>;

    /// Simple chat without system prompt.
    async fn chat(&self, message: &str, model: &str, temperature: f64) -> anyhow::Result<String> {
        self.chat_with_system(None, message, model, temperature).await
    }

    /// Check if the provider supports a specific model.
    fn supports_model(&self, model: &str) -> bool;

    /// Warm up the provider (e.g., pre-connect).
    async fn warmup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat_with_system(
            &self,
            _system: Option<&str>,
            message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            Ok(format!("Echo: {}", message))
        }

        fn supports_model(&self, _model: &str) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn mock_provider_works() {
        let provider = MockProvider;
        assert_eq!(provider.name(), "mock");
        assert!(provider.supports_model("any-model"));

        let response = provider.chat("Hello", "test", 0.7).await.unwrap();
        assert_eq!(response, "Echo: Hello");
    }

    #[tokio::test]
    async fn mock_provider_with_system() {
        let provider = MockProvider;
        let response = provider
            .chat_with_system(Some("You are helpful"), "Hi", "test", 0.5)
            .await
            .unwrap();
        assert_eq!(response, "Echo: Hi");
    }

    #[tokio::test]
    async fn warmup_default_succeeds() {
        let provider = MockProvider;
        assert!(provider.warmup().await.is_ok());
    }
}
