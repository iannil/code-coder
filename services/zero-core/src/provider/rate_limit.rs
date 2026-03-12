//! Rate Limiting and Retry Logic
//!
//! Provides rate limiting, retry with exponential backoff, and circuit breaker
//! functionality for AI provider calls.

use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures_util::Stream;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::types::{
    ChatRequest, ChatResponse, ProviderError, ProviderErrorKind,
    StreamEvent,
};
use super::Provider;

// ============================================================================
// Constants
// ============================================================================

/// Default initial retry delay in milliseconds
const DEFAULT_INITIAL_DELAY_MS: u64 = 1000;

/// Default maximum retry delay in milliseconds
const DEFAULT_MAX_DELAY_MS: u64 = 60_000;

/// Default maximum number of retries
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default jitter factor (0.0 to 1.0)
const DEFAULT_JITTER: f64 = 0.1;

// ============================================================================
// Retry Configuration
// ============================================================================

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial delay before first retry (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Random jitter factor (0.0 to 1.0)
    pub jitter: f64,
    /// Whether to retry on rate limit errors
    pub retry_on_rate_limit: bool,
    /// Whether to retry on server errors
    pub retry_on_server_error: bool,
    /// Whether to retry on timeout
    pub retry_on_timeout: bool,
    /// Whether to retry on network errors
    pub retry_on_network: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            initial_delay_ms: DEFAULT_INITIAL_DELAY_MS,
            max_delay_ms: DEFAULT_MAX_DELAY_MS,
            backoff_multiplier: 2.0,
            jitter: DEFAULT_JITTER,
            retry_on_rate_limit: true,
            retry_on_server_error: true,
            retry_on_timeout: true,
            retry_on_network: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of retries
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay_ms: u64) -> Self {
        self.initial_delay_ms = delay_ms;
        self
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, delay_ms: u64) -> Self {
        self.max_delay_ms = delay_ms;
        self
    }

    /// Calculate delay for a given attempt
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_delay = self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        let capped_delay = base_delay.min(self.max_delay_ms as f64);

        // Add jitter
        let jitter_range = capped_delay * self.jitter;
        let jitter = rand::random::<f64>() * jitter_range * 2.0 - jitter_range;
        let final_delay = (capped_delay + jitter).max(0.0);

        Duration::from_millis(final_delay as u64)
    }

    /// Check if an error should be retried
    pub fn should_retry(&self, error: &ProviderError) -> bool {
        match error.kind {
            ProviderErrorKind::RateLimit { .. } => self.retry_on_rate_limit,
            ProviderErrorKind::Server => self.retry_on_server_error,
            ProviderErrorKind::Timeout => self.retry_on_timeout,
            ProviderErrorKind::Network => self.retry_on_network,
            _ => false,
        }
    }
}

// ============================================================================
// Rate Limiter
// ============================================================================

/// Token bucket rate limiter
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum tokens in the bucket
    capacity: f64,
    /// Current tokens in the bucket
    tokens: RwLock<f64>,
    /// Token refill rate per second
    refill_rate: f64,
    /// Last time tokens were refilled
    last_refill: RwLock<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(requests_per_second: f64) -> Self {
        Self {
            capacity: requests_per_second,
            tokens: RwLock::new(requests_per_second),
            refill_rate: requests_per_second,
            last_refill: RwLock::new(Instant::now()),
        }
    }

    /// Create a rate limiter with custom capacity
    pub fn with_capacity(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: RwLock::new(capacity),
            refill_rate,
            last_refill: RwLock::new(Instant::now()),
        }
    }

    /// Acquire a token, waiting if necessary
    pub async fn acquire(&self) {
        loop {
            self.refill().await;

            let mut tokens = self.tokens.write().await;
            if *tokens >= 1.0 {
                *tokens -= 1.0;
                return;
            }
            drop(tokens);

            // Wait a bit before trying again
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Try to acquire a token without waiting
    pub async fn try_acquire(&self) -> bool {
        self.refill().await;

        let mut tokens = self.tokens.write().await;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    async fn refill(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.write().await;
        let elapsed = now.duration_since(*last_refill).as_secs_f64();

        if elapsed > 0.0 {
            let mut tokens = self.tokens.write().await;
            *tokens = (*tokens + elapsed * self.refill_rate).min(self.capacity);
            *last_refill = now;
        }
    }
}

// ============================================================================
// Circuit Breaker
// ============================================================================

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for protecting against cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: RwLock<u32>,
    success_count: RwLock<u32>,
    last_failure: RwLock<Option<Instant>>,
    /// Number of failures before opening the circuit
    failure_threshold: u32,
    /// Number of successes in half-open to close the circuit
    success_threshold: u32,
    /// Duration to keep the circuit open
    reset_timeout: Duration,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new() -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: RwLock::new(0),
            success_count: RwLock::new(0),
            last_failure: RwLock::new(None),
            failure_threshold: 5,
            success_threshold: 3,
            reset_timeout: Duration::from_secs(30),
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(failure_threshold: u32, success_threshold: u32, reset_timeout_secs: u64) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: RwLock::new(0),
            success_count: RwLock::new(0),
            last_failure: RwLock::new(None),
            failure_threshold,
            success_threshold,
            reset_timeout: Duration::from_secs(reset_timeout_secs),
        }
    }

    /// Check if the circuit allows a request
    pub async fn allow_request(&self) -> bool {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                let last_failure = self.last_failure.read().await;
                if let Some(last) = *last_failure {
                    if last.elapsed() >= self.reset_timeout {
                        drop(last_failure);
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.success_count.write().await = 0;
                        info!("Circuit breaker transitioning to half-open");
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                *self.failure_count.write().await = 0;
            }
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.success_threshold {
                    drop(success_count);
                    *self.state.write().await = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    info!("Circuit breaker closed after successful requests");
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let state = *self.state.read().await;

        match state {
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;

                if *failure_count >= self.failure_threshold {
                    drop(failure_count);
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure.write().await = Some(Instant::now());
                    warn!("Circuit breaker opened after {} failures", self.failure_threshold);
                }
            }
            CircuitState::HalfOpen => {
                *self.state.write().await = CircuitState::Open;
                *self.last_failure.write().await = Some(Instant::now());
                *self.success_count.write().await = 0;
                warn!("Circuit breaker re-opened after failure in half-open state");
            }
            CircuitState::Open => {}
        }
    }

    /// Get the current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Resilient Provider
// ============================================================================

/// A provider wrapper that adds retry, rate limiting, and circuit breaking
pub struct ResilientProvider<P: Provider> {
    inner: P,
    retry_config: RetryConfig,
    rate_limiter: Option<Arc<RateLimiter>>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl<P: Provider> ResilientProvider<P> {
    /// Create a new resilient provider
    pub fn new(provider: P) -> Self {
        Self {
            inner: provider,
            retry_config: RetryConfig::default(),
            rate_limiter: None,
            circuit_breaker: Arc::new(CircuitBreaker::new()),
        }
    }

    /// Set retry configuration
    pub fn with_retry(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Set rate limiter
    pub fn with_rate_limit(mut self, requests_per_second: f64) -> Self {
        self.rate_limiter = Some(Arc::new(RateLimiter::new(requests_per_second)));
        self
    }

    /// Set circuit breaker
    pub fn with_circuit_breaker(mut self, breaker: CircuitBreaker) -> Self {
        self.circuit_breaker = Arc::new(breaker);
        self
    }

    /// Execute with retry logic
    async fn execute_with_retry<F, T, Fut>(&self, operation: F) -> Result<T, ProviderError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ProviderError>>,
    {
        // Check circuit breaker
        if !self.circuit_breaker.allow_request().await {
            return Err(ProviderError::new(
                ProviderErrorKind::Server,
                "Circuit breaker is open",
            ));
        }

        // Apply rate limiting
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }

        let mut last_error: Option<ProviderError> = None;

        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                let delay = if let Some(ref error) = last_error {
                    // Use retry-after if provided
                    if let Some(retry_after) = error.retry_after_ms() {
                        Duration::from_millis(retry_after)
                    } else {
                        self.retry_config.delay_for_attempt(attempt - 1)
                    }
                } else {
                    self.retry_config.delay_for_attempt(attempt - 1)
                };

                debug!("Retrying after {:?} (attempt {})", delay, attempt);
                tokio::time::sleep(delay).await;

                // Re-check rate limit for retry
                if let Some(limiter) = &self.rate_limiter {
                    limiter.acquire().await;
                }
            }

            match operation().await {
                Ok(result) => {
                    self.circuit_breaker.record_success().await;
                    return Ok(result);
                }
                Err(error) => {
                    debug!("Request failed (attempt {}): {:?}", attempt, error);

                    if !self.retry_config.should_retry(&error) || attempt == self.retry_config.max_retries {
                        self.circuit_breaker.record_failure().await;
                        return Err(error);
                    }

                    last_error = Some(error);
                }
            }
        }

        // Should not reach here, but just in case
        Err(last_error.unwrap_or_else(|| {
            ProviderError::new(ProviderErrorKind::Unknown, "Max retries exceeded")
        }))
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> Provider for ResilientProvider<P> {
    fn provider_id(&self) -> &str {
        self.inner.provider_id()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.execute_with_retry(|| self.inner.chat(request.clone())).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>, ProviderError>
    {
        // For streaming, we don't retry mid-stream, only the initial connection
        // Check circuit breaker
        if !self.circuit_breaker.allow_request().await {
            return Err(ProviderError::new(
                ProviderErrorKind::Server,
                "Circuit breaker is open",
            ));
        }

        // Apply rate limiting
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }

        let result = self.inner.chat_stream(request).await;

        match &result {
            Ok(_) => self.circuit_breaker.record_success().await,
            Err(_) => self.circuit_breaker.record_failure().await,
        }

        result
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }

    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }

    fn supports_vision(&self) -> bool {
        self.inner.supports_vision()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_delay() {
        let config = RetryConfig::new()
            .with_initial_delay(1000)
            .with_max_delay(10000);

        // First retry should be around initial delay
        let delay0 = config.delay_for_attempt(0);
        assert!(delay0.as_millis() >= 800 && delay0.as_millis() <= 1200);

        // Second retry should be roughly doubled
        let delay1 = config.delay_for_attempt(1);
        assert!(delay1.as_millis() >= 1600 && delay1.as_millis() <= 2400);

        // Should not exceed max delay
        let delay_high = config.delay_for_attempt(10);
        assert!(delay_high.as_millis() <= 11000); // max + jitter
    }

    #[test]
    fn test_retry_should_retry() {
        let config = RetryConfig::new();

        // Should retry rate limit
        let rate_limit = ProviderError::new(
            ProviderErrorKind::RateLimit { retry_after_ms: None },
            "Rate limited",
        );
        assert!(config.should_retry(&rate_limit));

        // Should not retry auth errors
        let auth = ProviderError::new(ProviderErrorKind::Authentication, "Invalid key");
        assert!(!config.should_retry(&auth));
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(10.0); // 10 requests per second

        // Should be able to acquire tokens quickly
        for _ in 0..5 {
            assert!(limiter.try_acquire().await);
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::with_thresholds(3, 2, 1);

        // Circuit should start closed
        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert!(breaker.allow_request().await);

        // Record failures to open the circuit
        for _ in 0..3 {
            breaker.record_failure().await;
        }

        assert_eq!(breaker.state().await, CircuitState::Open);
        assert!(!breaker.allow_request().await);

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Circuit should be half-open now
        assert!(breaker.allow_request().await);
        assert_eq!(breaker.state().await, CircuitState::HalfOpen);

        // Record successes to close the circuit
        breaker.record_success().await;
        breaker.record_success().await;

        assert_eq!(breaker.state().await, CircuitState::Closed);
    }
}
