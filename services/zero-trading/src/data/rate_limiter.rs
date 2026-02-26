//! Token bucket rate limiter for API request throttling.
//!
//! Implements a simple token bucket algorithm to proactively limit
//! request rates and avoid hitting API rate limits.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::debug;

/// A token bucket rate limiter.
///
/// Allows up to `capacity` requests per `refill_interval`, with tokens
/// being refilled continuously at a rate of `capacity / refill_interval`.
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum tokens in the bucket
    capacity: u32,
    /// Current available tokens (scaled by 1000 for precision)
    tokens: AtomicU64,
    /// Tokens added per millisecond (scaled by 1000)
    refill_rate_per_ms: f64,
    /// Last refill timestamp
    last_refill: Mutex<Instant>,
    /// Name for logging
    name: String,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// # Arguments
    /// * `name` - Name for logging purposes
    /// * `requests_per_minute` - Maximum requests allowed per minute
    ///
    /// # Example
    /// ```
    /// let limiter = RateLimiter::new("itick", 300); // 300 req/min = 5 req/sec
    /// ```
    pub fn new(name: impl Into<String>, requests_per_minute: u32) -> Self {
        // Convert to requests per second for the bucket capacity
        // Use a 1-second window for smoother rate limiting
        let requests_per_second = (requests_per_minute as f64 / 60.0).ceil() as u32;
        let capacity = requests_per_second.max(1);

        // Calculate refill rate: tokens per millisecond
        let refill_rate_per_ms = requests_per_minute as f64 / 60_000.0;

        Self {
            capacity,
            tokens: AtomicU64::new((capacity as u64) * 1000), // Scale by 1000
            refill_rate_per_ms,
            last_refill: Mutex::new(Instant::now()),
            name: name.into(),
        }
    }

    /// Create a rate limiter from requests per second.
    ///
    /// # Arguments
    /// * `name` - Name for logging purposes
    /// * `requests_per_second` - Maximum requests allowed per second
    pub fn from_rps(name: impl Into<String>, requests_per_second: u32) -> Self {
        Self::new(name, requests_per_second * 60)
    }

    /// Acquire a token, waiting if necessary.
    ///
    /// Returns immediately if a token is available, otherwise waits
    /// until a token becomes available.
    pub async fn acquire(&self) {
        loop {
            if self.try_acquire() {
                return;
            }

            // Calculate wait time for next token
            let wait_ms = (1000.0 / (self.refill_rate_per_ms * 1000.0)).ceil() as u64;
            let wait_time = Duration::from_millis(wait_ms.max(10).min(1000));

            debug!(
                limiter = %self.name,
                wait_ms = wait_time.as_millis(),
                "Rate limited, waiting for token"
            );

            tokio::time::sleep(wait_time).await;
        }
    }

    /// Try to acquire a token without waiting.
    ///
    /// Returns `true` if a token was acquired, `false` otherwise.
    pub fn try_acquire(&self) -> bool {
        self.refill();

        // Try to decrement tokens (atomic CAS loop)
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current < 1000 {
                // Less than 1 full token
                return false;
            }

            let new_value = current - 1000;
            if self
                .tokens
                .compare_exchange_weak(current, new_value, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&self) {
        // Use try_lock to avoid blocking on refill
        if let Ok(mut last_refill) = self.last_refill.try_lock() {
            let now = Instant::now();
            let elapsed_ms = now.duration_since(*last_refill).as_millis() as f64;

            if elapsed_ms > 0.0 {
                let new_tokens = (elapsed_ms * self.refill_rate_per_ms * 1000.0) as u64;

                if new_tokens > 0 {
                    let max_tokens = (self.capacity as u64) * 1000;

                    // Add tokens (capped at capacity)
                    loop {
                        let current = self.tokens.load(Ordering::Relaxed);
                        let new_value = (current + new_tokens).min(max_tokens);

                        if current == new_value
                            || self
                                .tokens
                                .compare_exchange_weak(
                                    current,
                                    new_value,
                                    Ordering::Relaxed,
                                    Ordering::Relaxed,
                                )
                                .is_ok()
                        {
                            break;
                        }
                    }

                    *last_refill = now;
                }
            }
        }
    }

    /// Get current available tokens (for debugging/monitoring).
    pub fn available_tokens(&self) -> f64 {
        self.refill();
        self.tokens.load(Ordering::Relaxed) as f64 / 1000.0
    }

    /// Get the configured capacity.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
}

/// Shared rate limiter that can be cloned.
pub type SharedRateLimiter = Arc<RateLimiter>;

/// Create a shared rate limiter.
pub fn shared_limiter(name: impl Into<String>, requests_per_minute: u32) -> SharedRateLimiter {
    Arc::new(RateLimiter::new(name, requests_per_minute))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new("test", 300);
        assert_eq!(limiter.capacity, 5); // 300/60 = 5 req/sec
    }

    #[test]
    fn test_try_acquire() {
        let limiter = RateLimiter::new("test", 60); // 1 req/sec
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire()); // Should fail - no tokens left
    }

    #[tokio::test]
    async fn test_acquire_refill() {
        let limiter = RateLimiter::new("test", 6000); // 100 req/sec for fast test

        // Exhaust tokens
        for _ in 0..100 {
            if !limiter.try_acquire() {
                break;
            }
        }

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should have some tokens now
        assert!(limiter.try_acquire());
    }

    #[test]
    fn test_available_tokens() {
        let limiter = RateLimiter::new("test", 300);
        let initial = limiter.available_tokens();
        assert!(initial > 0.0);
        assert!(initial <= 5.0); // Capacity is 5
    }

    #[test]
    fn test_from_rps() {
        let limiter = RateLimiter::from_rps("test", 10);
        assert_eq!(limiter.capacity, 10);
    }
}
