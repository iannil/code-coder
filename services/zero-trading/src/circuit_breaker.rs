//! Circuit breaker pattern for external service resilience.
//!
//! Implements a circuit breaker that prevents cascading failures by temporarily
//! blocking requests to unhealthy services. This is particularly important for:
//! - Data provider API calls (iTick, Lixin)
//! - CodeCoder API calls (macro agent)
//! - Notification service (Telegram)
//!
//! # States
//!
//! ```text
//! ┌─────────┐     failure_threshold    ┌────────┐
//! │ CLOSED  │ ─────────────────────────│  OPEN  │
//! │(normal) │      exceeded           │(blocked)│
//! └─────────┘                         └────────┘
//!      ▲                                   │
//!      │     success                       │ reset_timeout
//!      │                                   ▼
//!      │                            ┌───────────┐
//!      └────────────────────────────│ HALF_OPEN │
//!                                   │(testing)  │
//!                                   └───────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! let breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
//!
//! if breaker.can_execute() {
//!     match make_request().await {
//!         Ok(result) => {
//!             breaker.record_success();
//!             Ok(result)
//!         }
//!         Err(e) => {
//!             breaker.record_failure();
//!             Err(e)
//!         }
//!     }
//! } else {
//!     Err(CircuitOpenError)
//! }
//! ```

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::RwLock;

// ============================================================================
// Circuit Breaker State
// ============================================================================

/// Current state of the circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests allowed
    Closed,
    /// Circuit tripped - requests blocked
    Open,
    /// Testing if service recovered - limited requests allowed
    HalfOpen,
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration to keep circuit open before testing
    pub reset_timeout: Duration,
    /// Number of successful requests needed to close circuit from half-open
    pub success_threshold: u32,
    /// Name for logging purposes
    pub name: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(30),
            success_threshold: 3,
            name: "default".to_string(),
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new configuration with a custom name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the failure threshold
    pub fn failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Set the reset timeout
    pub fn reset_timeout(mut self, timeout: Duration) -> Self {
        self.reset_timeout = timeout;
        self
    }

    /// Set the success threshold
    pub fn success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }
}

// ============================================================================
// Circuit Breaker Implementation
// ============================================================================

/// Circuit breaker for protecting external service calls.
///
/// Thread-safe implementation using atomic operations for state management.
pub struct CircuitBreaker {
    /// Configuration
    config: CircuitBreakerConfig,
    /// Consecutive failure count
    failure_count: AtomicU32,
    /// Consecutive success count (for half-open state)
    success_count: AtomicU32,
    /// Time when circuit was opened (as unix timestamp millis)
    opened_at: AtomicU64,
    /// Current state (stored as u8 for atomic access)
    state: AtomicU32,
    /// Lock for state transitions
    transition_lock: RwLock<()>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            opened_at: AtomicU64::new(0),
            state: AtomicU32::new(CircuitState::Closed as u32),
            transition_lock: RwLock::new(()),
        }
    }

    /// Create a circuit breaker with default configuration and a name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self::new(CircuitBreakerConfig::with_name(name))
    }

    /// Get the current state of the circuit breaker
    pub fn state(&self) -> CircuitState {
        self.get_state_with_timeout_check()
    }

    /// Check if a request can be executed
    pub fn can_execute(&self) -> bool {
        match self.get_state_with_timeout_check() {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => true,
            CircuitState::Open => false,
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let state = self.get_state_with_timeout_check();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.config.success_threshold {
                    self.close_circuit();
                }
            }
            CircuitState::Open => {
                // Shouldn't happen - ignore
            }
        }

        tracing::trace!(
            circuit = %self.config.name,
            state = ?state,
            "Circuit breaker recorded success"
        );
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let state = self.get_state_with_timeout_check();

        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.config.failure_threshold {
                    self.open_circuit();
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open immediately opens the circuit
                self.open_circuit();
            }
            CircuitState::Open => {
                // Already open - ignore
            }
        }

        tracing::trace!(
            circuit = %self.config.name,
            state = ?state,
            failure_count = self.failure_count.load(Ordering::Relaxed),
            "Circuit breaker recorded failure"
        );
    }

    /// Get statistics about the circuit breaker
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            name: self.config.name.clone(),
            state: self.get_state_with_timeout_check(),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            failure_threshold: self.config.failure_threshold,
            success_threshold: self.config.success_threshold,
        }
    }

    /// Reset the circuit breaker to closed state
    pub fn reset(&self) {
        self.close_circuit();
        tracing::info!(circuit = %self.config.name, "Circuit breaker manually reset");
    }

    // ========================================================================
    // Private Methods
    // ========================================================================

    fn get_state_with_timeout_check(&self) -> CircuitState {
        let state_val = self.state.load(Ordering::Acquire);
        let state = match state_val {
            0 => CircuitState::Closed,
            1 => CircuitState::Open,
            2 => CircuitState::HalfOpen,
            _ => CircuitState::Closed, // Fallback
        };

        // Check if open circuit should transition to half-open
        if state == CircuitState::Open {
            let opened_at = self.opened_at.load(Ordering::Relaxed);
            if opened_at > 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let elapsed_ms = now.saturating_sub(opened_at);
                if elapsed_ms >= self.config.reset_timeout.as_millis() as u64 {
                    // Transition to half-open
                    self.try_transition_to_half_open();
                    return CircuitState::HalfOpen;
                }
            }
        }

        state
    }

    fn open_circuit(&self) {
        let _guard = self.transition_lock.try_write();

        self.state.store(CircuitState::Open as u32, Ordering::Release);
        self.success_count.store(0, Ordering::Relaxed);

        // Store current time as unix timestamp millis
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.opened_at.store(now, Ordering::Relaxed);

        tracing::warn!(
            circuit = %self.config.name,
            failure_count = self.failure_count.load(Ordering::Relaxed),
            reset_timeout_secs = self.config.reset_timeout.as_secs(),
            "Circuit breaker OPENED"
        );
    }

    fn try_transition_to_half_open(&self) {
        // Use compare_exchange to ensure atomic transition
        let _ = self.state.compare_exchange(
            CircuitState::Open as u32,
            CircuitState::HalfOpen as u32,
            Ordering::AcqRel,
            Ordering::Relaxed,
        );

        self.success_count.store(0, Ordering::Relaxed);

        tracing::info!(
            circuit = %self.config.name,
            "Circuit breaker transitioning to HALF_OPEN"
        );
    }

    fn close_circuit(&self) {
        let _guard = self.transition_lock.try_write();

        self.state.store(CircuitState::Closed as u32, Ordering::Release);
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.opened_at.store(0, Ordering::Relaxed);

        tracing::info!(circuit = %self.config.name, "Circuit breaker CLOSED");
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Statistics about a circuit breaker's current state
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    /// Name of the circuit breaker
    pub name: String,
    /// Current state
    pub state: CircuitState,
    /// Current failure count
    pub failure_count: u32,
    /// Current success count (relevant in half-open state)
    pub success_count: u32,
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit from half-open
    pub success_threshold: u32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_starts_closed() {
        let breaker = CircuitBreaker::with_name("test");
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.can_execute());
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(3);
        let breaker = CircuitBreaker::new(config);

        assert_eq!(breaker.state(), CircuitState::Closed);

        // Record failures
        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Closed);

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.can_execute());
    }

    #[test]
    fn test_success_resets_failure_count() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(3);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.failure_count.load(Ordering::Relaxed), 2);

        breaker.record_success();
        assert_eq!(breaker.failure_count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_manual_reset() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(2);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure();
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);

        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.can_execute());
    }

    #[test]
    fn test_stats() {
        let config = CircuitBreakerConfig::with_name("api")
            .failure_threshold(5)
            .success_threshold(2);
        let breaker = CircuitBreaker::new(config);

        breaker.record_failure();
        breaker.record_failure();

        let stats = breaker.stats();
        assert_eq!(stats.name, "api");
        assert_eq!(stats.state, CircuitState::Closed);
        assert_eq!(stats.failure_count, 2);
        assert_eq!(stats.failure_threshold, 5);
        assert_eq!(stats.success_threshold, 2);
    }

    #[test]
    fn test_config_builder() {
        let config = CircuitBreakerConfig::with_name("test")
            .failure_threshold(10)
            .reset_timeout(Duration::from_secs(60))
            .success_threshold(5);

        assert_eq!(config.name, "test");
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.reset_timeout, Duration::from_secs(60));
        assert_eq!(config.success_threshold, 5);
    }
}
