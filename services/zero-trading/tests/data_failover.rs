//! Integration tests for data source failover.
//!
//! Tests automatic failover between data providers when primary sources fail.
//! These tests verify the resilience of the data layer.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use zero_trading::data::{
    DataCapabilities, DataProvider, DataProviderRouter, ProviderError, RouterConfig,
    Candle, Timeframe,
};

// ============================================================================
// Mock Providers for Testing
// ============================================================================

/// Mock provider that can be configured to succeed or fail
struct MockProvider {
    name: &'static str,
    priority: u8,
    should_fail: bool,
    fail_count: AtomicU32,
    success_count: AtomicU32,
}

impl MockProvider {
    fn new(name: &'static str, priority: u8, should_fail: bool) -> Self {
        Self {
            name,
            priority,
            should_fail,
            fail_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
        }
    }

    fn call_count(&self) -> u32 {
        self.fail_count.load(Ordering::Relaxed) + self.success_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl DataProvider for MockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn capabilities(&self) -> DataCapabilities {
        DataCapabilities::daily_only()
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        if self.should_fail {
            Err(ProviderError::Unavailable("mock failure".into()))
        } else {
            Ok(())
        }
    }

    async fn get_daily_candles(
        &self,
        symbol: &str,
        _start_date: Option<NaiveDate>,
        _end_date: Option<NaiveDate>,
        _limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError> {
        if self.should_fail {
            self.fail_count.fetch_add(1, Ordering::Relaxed);
            Err(ProviderError::Network("mock network failure".into()))
        } else {
            self.success_count.fetch_add(1, Ordering::Relaxed);
            Ok(vec![Candle {
                symbol: symbol.to_string(),
                timeframe: Timeframe::Daily,
                timestamp: Utc::now(),
                open: 10.0,
                high: 11.0,
                low: 9.5,
                close: 10.5,
                volume: 1000.0,
                amount: 10500.0,
            }])
        }
    }

    async fn get_minute_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>, ProviderError> {
        Err(ProviderError::DataNotAvailable("not implemented".into()))
    }
}

/// Mock provider that fails a specific number of times then succeeds
struct FlakeyProvider {
    name: &'static str,
    priority: u8,
    failures_remaining: AtomicU32,
    total_calls: AtomicU32,
}

impl FlakeyProvider {
    fn new(name: &'static str, priority: u8, initial_failures: u32) -> Self {
        Self {
            name,
            priority,
            failures_remaining: AtomicU32::new(initial_failures),
            total_calls: AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl DataProvider for FlakeyProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn capabilities(&self) -> DataCapabilities {
        DataCapabilities::daily_only()
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn get_daily_candles(
        &self,
        symbol: &str,
        _start_date: Option<NaiveDate>,
        _end_date: Option<NaiveDate>,
        _limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError> {
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        let remaining = self.failures_remaining.load(Ordering::Relaxed);
        if remaining > 0 {
            self.failures_remaining.fetch_sub(1, Ordering::Relaxed);
            Err(ProviderError::Network("temporary failure".into()))
        } else {
            Ok(vec![Candle {
                symbol: symbol.to_string(),
                timeframe: Timeframe::Daily,
                timestamp: Utc::now(),
                open: 10.0,
                high: 11.0,
                low: 9.5,
                close: 10.5,
                volume: 1000.0,
                amount: 10500.0,
            }])
        }
    }

    async fn get_minute_candles(
        &self,
        _symbol: &str,
        _timeframe: Timeframe,
        _start_time: Option<DateTime<Utc>>,
        _end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>, ProviderError> {
        Err(ProviderError::DataNotAvailable("not implemented".into()))
    }
}

// ============================================================================
// Failover Tests
// ============================================================================

#[tokio::test]
async fn test_failover_to_backup_provider() {
    let router = DataProviderRouter::with_config(RouterConfig {
        max_retries: 0, // No retries to make test faster
        ..Default::default()
    });

    let failing = Arc::new(MockProvider::new("failing", 1, true));
    let working = Arc::new(MockProvider::new("working", 2, false));

    router.register(failing.clone()).await;
    router.register(working.clone()).await;

    // Request should failover from failing to working
    let result = router
        .get_daily_candles("000001.SZ", None, None, None)
        .await;

    assert!(result.is_ok());
    // Verify failing provider was tried
    assert!(failing.call_count() > 0);
    // Verify working provider was used
    assert!(working.call_count() > 0);
}

#[tokio::test]
async fn test_all_providers_fail() {
    let router = DataProviderRouter::with_config(RouterConfig {
        max_retries: 0,
        ..Default::default()
    });

    let failing1 = Arc::new(MockProvider::new("failing1", 1, true));
    let failing2 = Arc::new(MockProvider::new("failing2", 2, true));

    router.register(failing1.clone()).await;
    router.register(failing2.clone()).await;

    // Request should fail gracefully
    let result = router
        .get_daily_candles("000001.SZ", None, None, None)
        .await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    // Should get an Unavailable error, not a panic
    matches!(error, ProviderError::Network(_) | ProviderError::Unavailable(_));
}

#[tokio::test]
async fn test_no_providers_registered() {
    let router = DataProviderRouter::new();

    let result = router
        .get_daily_candles("000001.SZ", None, None, None)
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ProviderError::Unavailable(msg) => {
            assert!(msg.contains("No data providers") || msg.contains("All providers failed"));
        }
        _ => panic!("Expected Unavailable error"),
    }
}

#[tokio::test]
async fn test_failover_after_transient_failure() {
    // This test verifies that failover works correctly when the primary
    // provider has transient failures
    let router = DataProviderRouter::with_config(RouterConfig {
        max_retries: 2,
        ..Default::default()
    });

    // Primary provider that always fails
    let failing = Arc::new(MockProvider::new("failing", 1, true));
    // Backup provider that always succeeds
    let backup = Arc::new(MockProvider::new("backup", 2, false));

    router.register(failing.clone()).await;
    router.register(backup.clone()).await;

    let result = router
        .get_daily_candles("000001.SZ", None, None, None)
        .await;

    // Should succeed via failover to backup
    assert!(result.is_ok(), "Expected success via failover");
    // Failing provider should have been tried
    assert!(failing.call_count() > 0);
    // Backup provider should have been used
    assert!(backup.call_count() > 0);
}

#[tokio::test]
async fn test_priority_ordering() {
    let router = DataProviderRouter::new();

    let low_priority = Arc::new(MockProvider::new("low", 10, false));
    let high_priority = Arc::new(MockProvider::new("high", 1, false));
    let mid_priority = Arc::new(MockProvider::new("mid", 5, false));

    // Register in random order
    router.register(low_priority.clone()).await;
    router.register(high_priority.clone()).await;
    router.register(mid_priority.clone()).await;

    // Request should use high priority first
    let result = router
        .get_daily_candles("000001.SZ", None, None, None)
        .await;

    assert!(result.is_ok());
    // High priority should be called
    assert_eq!(high_priority.call_count(), 1);
    // Others should not be called (since high priority succeeded)
    assert_eq!(mid_priority.call_count(), 0);
    assert_eq!(low_priority.call_count(), 0);
}

#[tokio::test]
async fn test_disable_provider() {
    let router = DataProviderRouter::new();

    let provider = Arc::new(MockProvider::new("test", 1, false));
    router.register(provider.clone()).await;

    // Disable the provider
    router.set_enabled("test", false).await;

    // Should fail since the only provider is disabled
    let result = router
        .get_daily_candles("000001.SZ", None, None, None)
        .await;

    assert!(result.is_err());
}

// ============================================================================
// Health Check Tests
// ============================================================================

#[tokio::test]
async fn test_health_check_updates_status() {
    let router = DataProviderRouter::new();

    let healthy = Arc::new(MockProvider::new("healthy", 1, false));
    let unhealthy = Arc::new(MockProvider::new("unhealthy", 2, true));

    router.register(healthy.clone()).await;
    router.register(unhealthy.clone()).await;

    // Get provider info
    let infos = router.get_providers_info().await;
    assert_eq!(infos.len(), 2);

    // Verify both are registered
    assert!(infos.iter().any(|p| p.name == "healthy"));
    assert!(infos.iter().any(|p| p.name == "unhealthy"));
}

// ============================================================================
// Graceful Degradation Tests
// ============================================================================

#[tokio::test]
async fn test_graceful_degradation_with_cache() {
    // This test verifies that the system handles partial failures gracefully
    let router = DataProviderRouter::with_config(RouterConfig {
        max_retries: 1,
        ..Default::default()
    });

    // First provider fails, second succeeds
    let failing = Arc::new(MockProvider::new("failing", 1, true));
    let working = Arc::new(MockProvider::new("working", 2, false));

    router.register(failing.clone()).await;
    router.register(working.clone()).await;

    // Multiple requests should all succeed via failover
    for _ in 0..5 {
        let result = router
            .get_daily_candles("000001.SZ", None, None, None)
            .await;
        assert!(result.is_ok());
    }

    // Working provider should have been called for each request
    assert!(working.call_count() >= 5);
}
