//! Resilient provider wrapper with retry and fallback support.
//!
//! This module provides a wrapper around multiple providers that implements
//! retry logic with exponential backoff and automatic fallback to secondary
//! providers when the primary fails.

use super::{ChatRequest, ChatResponse, Provider, ProviderError};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// Configuration for resilient provider behavior.
#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    /// Maximum number of retries per provider before falling back.
    pub max_retries: u32,
    /// Base backoff delay in milliseconds (doubles with each retry).
    pub base_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds.
    pub max_backoff_ms: u64,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_backoff_ms: 100,
            max_backoff_ms: 10_000,
        }
    }
}

/// A resilient provider that wraps multiple providers with retry and fallback behavior.
///
/// When a request fails, it retries with exponential backoff. If all retries
/// are exhausted, it falls back to the next provider in the chain.
pub struct ResilientProvider {
    providers: Vec<Arc<dyn Provider>>,
    config: ResilienceConfig,
}

impl ResilientProvider {
    /// Create a new resilient provider with the given providers and configuration.
    ///
    /// The first provider is the primary, subsequent providers are fallbacks.
    pub fn new(providers: Vec<Arc<dyn Provider>>, config: ResilienceConfig) -> Self {
        Self { providers, config }
    }

    /// Create a new resilient provider with default configuration.
    pub fn with_defaults(providers: Vec<Arc<dyn Provider>>) -> Self {
        Self::new(providers, ResilienceConfig::default())
    }

    /// Create from a single provider with retry support (no fallbacks).
    pub fn single(provider: Arc<dyn Provider>, config: ResilienceConfig) -> Self {
        Self::new(vec![provider], config)
    }

    /// Calculate backoff delay for a given attempt.
    fn backoff_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self
            .config
            .base_backoff_ms
            .saturating_mul(2_u64.saturating_pow(attempt))
            .min(self.config.max_backoff_ms);
        Duration::from_millis(delay_ms)
    }
}

#[async_trait]
impl Provider for ResilientProvider {
    fn name(&self) -> &str {
        "resilient"
    }

    fn models(&self) -> Vec<&str> {
        // Aggregate models from all providers
        self.providers
            .iter()
            .flat_map(|p| p.models())
            .collect()
    }

    fn supports_model(&self, model: &str) -> bool {
        self.providers.iter().any(|p| p.supports_model(model))
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let mut all_errors = Vec::new();

        for (provider_idx, provider) in self.providers.iter().enumerate() {
            let provider_name = provider.name();

            for attempt in 0..=self.config.max_retries {
                match provider.chat(request.clone()).await {
                    Ok(response) => {
                        if attempt > 0 {
                            tracing::info!(
                                provider = provider_name,
                                attempt = attempt + 1,
                                "Provider recovered after retries"
                            );
                        }
                        return Ok(response);
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "{} attempt {}/{}: {}",
                            provider_name,
                            attempt + 1,
                            self.config.max_retries + 1,
                            e
                        );
                        all_errors.push(error_msg);

                        // Only sleep if there are more retries to attempt
                        if attempt < self.config.max_retries {
                            let delay = self.backoff_delay(attempt);
                            tracing::warn!(
                                provider = provider_name,
                                attempt = attempt + 1,
                                max_retries = self.config.max_retries,
                                delay_ms = delay.as_millis() as u64,
                                "Provider call failed, retrying"
                            );
                            tokio::time::sleep(delay).await;
                        }
                    }
                }
            }

            // Log fallback if there are more providers
            if provider_idx + 1 < self.providers.len() {
                let next_provider = self.providers[provider_idx + 1].name();
                tracing::warn!(
                    failed_provider = provider_name,
                    next_provider = next_provider,
                    "Switching to fallback provider"
                );
            }
        }

        // All providers failed
        Err(ProviderError {
            provider: "resilient".to_string(),
            model: request.model,
            message: format!("All providers failed. Attempts:\n{}", all_errors.join("\n")),
            status_code: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock provider for testing
    struct MockProvider {
        name: &'static str,
        calls: Arc<AtomicUsize>,
        fail_until: usize,
        response: &'static str,
        error_msg: &'static str,
    }

    impl MockProvider {
        fn new(
            name: &'static str,
            fail_until: usize,
            response: &'static str,
            error_msg: &'static str,
        ) -> (Self, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            (
                Self {
                    name,
                    calls: Arc::clone(&calls),
                    fail_until,
                    response,
                    error_msg,
                },
                calls,
            )
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            self.name
        }

        fn models(&self) -> Vec<&str> {
            vec!["test-model"]
        }

        fn supports_model(&self, _model: &str) -> bool {
            true
        }

        async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
            let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;

            if attempt <= self.fail_until {
                return Err(ProviderError {
                    provider: self.name.to_string(),
                    model: request.model,
                    message: self.error_msg.to_string(),
                    status_code: Some(500),
                });
            }

            Ok(ChatResponse {
                provider: self.name.to_string(),
                model: request.model,
                content: self.response.to_string(),
                usage: Default::default(),
                finish_reason: Some("stop".to_string()),
                latency_ms: 100,
            })
        }
    }

    fn make_request() -> ChatRequest {
        ChatRequest {
            model: "test-model".to_string(),
            messages: vec![],
            max_tokens: None,
            temperature: None,
            system: None,
        }
    }

    #[tokio::test]
    async fn succeeds_without_retry() {
        let (provider, calls) = MockProvider::new("primary", 0, "success", "error");
        let resilient = ResilientProvider::single(
            Arc::new(provider),
            ResilienceConfig {
                max_retries: 2,
                base_backoff_ms: 1,
                max_backoff_ms: 10,
            },
        );

        let result = resilient.chat(make_request()).await.unwrap();
        assert_eq!(result.content, "success");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_then_succeeds() {
        let (provider, calls) = MockProvider::new("primary", 1, "recovered", "temporary");
        let resilient = ResilientProvider::single(
            Arc::new(provider),
            ResilienceConfig {
                max_retries: 2,
                base_backoff_ms: 1,
                max_backoff_ms: 10,
            },
        );

        let result = resilient.chat(make_request()).await.unwrap();
        assert_eq!(result.content, "recovered");
        assert_eq!(calls.load(Ordering::SeqCst), 2); // 1 fail + 1 success
    }

    #[tokio::test]
    async fn falls_back_after_retries_exhausted() {
        let (primary, primary_calls) = MockProvider::new("primary", usize::MAX, "never", "down");
        let (fallback, fallback_calls) = MockProvider::new("fallback", 0, "from_fallback", "err");

        let resilient = ResilientProvider::new(
            vec![Arc::new(primary), Arc::new(fallback)],
            ResilienceConfig {
                max_retries: 1,
                base_backoff_ms: 1,
                max_backoff_ms: 10,
            },
        );

        let result = resilient.chat(make_request()).await.unwrap();
        assert_eq!(result.content, "from_fallback");
        assert_eq!(primary_calls.load(Ordering::SeqCst), 2); // 2 attempts (initial + 1 retry)
        assert_eq!(fallback_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn returns_aggregated_error_when_all_fail() {
        let (p1, _) = MockProvider::new("provider1", usize::MAX, "never", "p1 error");
        let (p2, _) = MockProvider::new("provider2", usize::MAX, "never", "p2 error");

        let resilient = ResilientProvider::new(
            vec![Arc::new(p1), Arc::new(p2)],
            ResilienceConfig {
                max_retries: 0,
                base_backoff_ms: 1,
                max_backoff_ms: 10,
            },
        );

        let err = resilient.chat(make_request()).await.unwrap_err();
        assert!(err.message.contains("All providers failed"));
        assert!(err.message.contains("provider1 attempt 1/1"));
        assert!(err.message.contains("provider2 attempt 1/1"));
    }

    #[test]
    fn backoff_doubles_with_attempts() {
        let resilient = ResilientProvider::with_defaults(vec![]);

        // Base is 100ms
        let d0 = resilient.backoff_delay(0);
        let d1 = resilient.backoff_delay(1);
        let d2 = resilient.backoff_delay(2);

        assert_eq!(d0.as_millis(), 100);
        assert_eq!(d1.as_millis(), 200);
        assert_eq!(d2.as_millis(), 400);
    }

    #[test]
    fn backoff_caps_at_max() {
        let resilient = ResilientProvider::new(
            vec![],
            ResilienceConfig {
                max_retries: 10,
                base_backoff_ms: 100,
                max_backoff_ms: 500,
            },
        );

        // Very high attempt should cap at max
        let delay = resilient.backoff_delay(20);
        assert_eq!(delay.as_millis(), 500);
    }

    #[test]
    fn aggregates_models_from_all_providers() {
        struct ModelProvider {
            name: &'static str,
            models: Vec<&'static str>,
        }

        #[async_trait]
        impl Provider for ModelProvider {
            fn name(&self) -> &str {
                self.name
            }
            fn models(&self) -> Vec<&str> {
                self.models.clone()
            }
            fn supports_model(&self, _: &str) -> bool {
                true
            }
            async fn chat(&self, _: ChatRequest) -> Result<ChatResponse, ProviderError> {
                unimplemented!()
            }
        }

        let resilient = ResilientProvider::with_defaults(vec![
            Arc::new(ModelProvider {
                name: "p1",
                models: vec!["a", "b"],
            }),
            Arc::new(ModelProvider {
                name: "p2",
                models: vec!["c"],
            }),
        ]);

        let models = resilient.models();
        assert_eq!(models.len(), 3);
        assert!(models.contains(&"a"));
        assert!(models.contains(&"b"));
        assert!(models.contains(&"c"));
    }
}
