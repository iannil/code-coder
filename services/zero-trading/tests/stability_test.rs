//! Stability tests for long-running operation scenarios.
//!
//! These tests verify that the system can run stably over extended periods
//! without memory leaks, deadlocks, or resource exhaustion.
//!
//! Note: Some tests are marked as #[ignore] because they take a long time
//! to run. Execute them with: `cargo test --test stability_test -- --ignored`

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use tokio::sync::RwLock;

use zero_trading::data::{
    DataCapabilities, DataProvider, DataProviderRouter, ProviderError, RouterConfig,
    Candle, DataCache, Timeframe,
};

// ============================================================================
// Test Utilities
// ============================================================================

/// Provider that tracks memory-related metrics
struct InstrumentedProvider {
    name: &'static str,
    priority: u8,
    call_count: AtomicU64,
    bytes_returned: AtomicU64,
}

impl InstrumentedProvider {
    fn new(name: &'static str, priority: u8) -> Self {
        Self {
            name,
            priority,
            call_count: AtomicU64::new(0),
            bytes_returned: AtomicU64::new(0),
        }
    }

    fn stats(&self) -> (u64, u64) {
        (
            self.call_count.load(Ordering::Relaxed),
            self.bytes_returned.load(Ordering::Relaxed),
        )
    }
}

#[async_trait]
impl DataProvider for InstrumentedProvider {
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
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError> {
        self.call_count.fetch_add(1, Ordering::Relaxed);

        let count = limit.unwrap_or(100);
        let now = Utc::now();

        let candles: Vec<Candle> = (0..count)
            .map(|i| Candle {
                symbol: symbol.to_string(),
                timeframe: Timeframe::Daily,
                timestamp: now - chrono::Duration::days(i as i64),
                open: 10.0 + (i as f64) * 0.01,
                high: 10.5 + (i as f64) * 0.01,
                low: 9.5 + (i as f64) * 0.01,
                close: 10.2 + (i as f64) * 0.01,
                volume: 1000000.0,
                amount: 10000000.0,
            })
            .collect();

        // Track approximate memory usage
        let approx_bytes = candles.len() * std::mem::size_of::<Candle>();
        self.bytes_returned
            .fetch_add(approx_bytes as u64, Ordering::Relaxed);

        Ok(candles)
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
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_router_access() {
    let router = Arc::new(DataProviderRouter::new());
    let provider = Arc::new(InstrumentedProvider::new("test", 1));

    router.register(provider.clone()).await;

    // Spawn multiple concurrent tasks
    let mut handles = vec![];
    let concurrent_requests = 100;

    for i in 0..concurrent_requests {
        let r = Arc::clone(&router);
        let handle = tokio::spawn(async move {
            let symbol = format!("TEST{:03}.SZ", i % 10);
            r.get_daily_candles(&symbol, None, None, Some(10)).await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            success_count += 1;
        }
    }

    // All requests should succeed
    assert_eq!(success_count, concurrent_requests);

    // Verify provider was called the expected number of times
    let (calls, _bytes) = provider.stats();
    assert_eq!(calls, concurrent_requests as u64);
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    let cache = Arc::new(DataCache::with_ttl(60));

    let symbols = vec!["000001.SZ", "000002.SZ", "000003.SZ", "000004.SZ", "000005.SZ"];
    let mut handles = vec![];

    // Concurrent writes
    for (i, symbol) in symbols.iter().enumerate() {
        let c = Arc::clone(&cache);
        let s = symbol.to_string();
        let handle = tokio::spawn(async move {
            let candles = vec![Candle {
                symbol: s.clone(),
                timeframe: Timeframe::Daily,
                timestamp: Utc::now(),
                open: 10.0 + i as f64,
                high: 11.0,
                low: 9.0,
                close: 10.5,
                volume: 1000.0,
                amount: 10000.0,
            }];
            c.set_candles(&s, Timeframe::Daily, candles);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Concurrent reads
    let mut read_handles = vec![];
    for symbol in &symbols {
        let c = Arc::clone(&cache);
        let s = symbol.to_string();
        let handle = tokio::spawn(async move { c.get_candles(&s, Timeframe::Daily) });
        read_handles.push(handle);
    }

    let mut cached_count = 0;
    for handle in read_handles {
        if let Ok(Some(_)) = handle.await {
            cached_count += 1;
        }
    }

    assert_eq!(cached_count, symbols.len());
}

// ============================================================================
// Memory Stability Tests
// ============================================================================

#[tokio::test]
async fn test_cache_memory_bounded() {
    // Test that cache doesn't grow unbounded
    let cache = DataCache::with_ttl(1); // 1 second TTL

    // Insert many entries
    for i in 0..1000 {
        let symbol = format!("TEST{:04}.SZ", i);
        let candles = vec![Candle {
            symbol: symbol.clone(),
            timeframe: Timeframe::Daily,
            timestamp: Utc::now(),
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 10.5,
            volume: 1000.0,
            amount: 10000.0,
        }];
        cache.set_candles(&symbol, Timeframe::Daily, candles);
    }

    // Wait for entries to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Clear expired entries
    cache.clear_expired();

    // Verify old entries are gone (by trying to read them)
    let old_entry = cache.get_candles("TEST0000.SZ", Timeframe::Daily);
    assert!(old_entry.is_none());
}

// ============================================================================
// Repeated Operation Tests
// ============================================================================

#[tokio::test]
async fn test_repeated_provider_registration() {
    let router = DataProviderRouter::new();

    // Register and unregister providers repeatedly
    for i in 0..100 {
        let name: &'static str = Box::leak(format!("provider_{}", i).into_boxed_str());
        let provider = Arc::new(InstrumentedProvider::new(name, (i % 10) as u8));

        router.register(provider).await;

        // Unregister every other one
        if i % 2 == 0 {
            router.unregister(name).await;
        }
    }

    // Verify the router is still functional
    let infos = router.get_providers_info().await;

    // Should have 50 providers remaining (the odd-numbered ones)
    assert_eq!(infos.len(), 50);
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

/// Provider that fails intermittently
struct IntermittentProvider {
    name: &'static str,
    priority: u8,
    failure_rate: f64, // 0.0 to 1.0
    call_count: AtomicU64,
}

impl IntermittentProvider {
    fn new(name: &'static str, priority: u8, failure_rate: f64) -> Self {
        Self {
            name,
            priority,
            failure_rate,
            call_count: AtomicU64::new(0),
        }
    }

    fn should_fail(&self) -> bool {
        let count = self.call_count.fetch_add(1, Ordering::Relaxed);
        // Deterministic failure pattern based on call count
        let x = (count.wrapping_mul(0x5DEECE66D).wrapping_add(0xB) % 1000) as f64 / 1000.0;
        x < self.failure_rate
    }
}

#[async_trait]
impl DataProvider for IntermittentProvider {
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
        if self.should_fail() {
            Err(ProviderError::Network("intermittent failure".into()))
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
                amount: 10000.0,
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

#[tokio::test]
async fn test_recovery_from_intermittent_failures() {
    let router = DataProviderRouter::with_config(RouterConfig {
        max_retries: 3,
        ..Default::default()
    });

    // Provider with 30% failure rate
    let flakey = Arc::new(IntermittentProvider::new("flakey", 1, 0.3));
    // Reliable backup
    let reliable = Arc::new(InstrumentedProvider::new("reliable", 2));

    router.register(flakey).await;
    router.register(reliable.clone()).await;

    let mut success_count = 0;
    let total_requests = 100;

    for _ in 0..total_requests {
        if router
            .get_daily_candles("TEST.SZ", None, None, Some(1))
            .await
            .is_ok()
        {
            success_count += 1;
        }
    }

    // With retries and failover, all requests should eventually succeed
    assert_eq!(success_count, total_requests);
}

// ============================================================================
// Long-Running Simulation Tests (Ignored by default)
// ============================================================================

#[tokio::test]
#[ignore = "long running test - run with --ignored flag"]
async fn test_simulated_trading_session() {
    // Simulate a 4-hour trading session with requests every minute
    let router = Arc::new(DataProviderRouter::with_config(RouterConfig {
        max_retries: 2,
        ..Default::default()
    }));

    let provider = Arc::new(InstrumentedProvider::new("main", 1));
    router.register(provider.clone()).await;

    let symbols = vec![
        "000001.SZ",
        "000002.SZ",
        "600000.SH",
        "600001.SH",
        "000300.SH",
    ];

    let start = Instant::now();
    let session_duration = Duration::from_secs(10); // Use 10 seconds for test

    let mut request_count = 0;
    let mut error_count = 0;

    while start.elapsed() < session_duration {
        for symbol in &symbols {
            match router.get_daily_candles(symbol, None, None, Some(50)).await {
                Ok(_) => request_count += 1,
                Err(_) => error_count += 1,
            }
        }

        // Simulate 1-minute intervals compressed to 100ms
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Log results
    let (calls, bytes) = provider.stats();
    println!(
        "Session completed: {} requests, {} errors, {} provider calls, {} bytes returned",
        request_count, error_count, calls, bytes
    );

    // Verify no errors occurred
    assert_eq!(error_count, 0);
    assert!(request_count > 0);
}

// ============================================================================
// Resource Cleanup Tests
// ============================================================================

#[tokio::test]
async fn test_provider_cleanup_on_drop() {
    // Create router in a scope so it gets dropped
    {
        let router = DataProviderRouter::new();
        let provider = Arc::new(InstrumentedProvider::new("temp", 1));
        router.register(provider).await;

        // Use the router
        let _ = router
            .get_daily_candles("TEST.SZ", None, None, Some(1))
            .await;

        // Router drops here
    }

    // If we reach here without hanging/crashing, cleanup was successful
    assert!(true);
}

// ============================================================================
// Stress Tests
// ============================================================================

#[tokio::test]
async fn test_rapid_request_burst() {
    let router = Arc::new(DataProviderRouter::new());
    let provider = Arc::new(InstrumentedProvider::new("fast", 1));
    router.register(provider.clone()).await;

    let burst_size = 500;
    let mut handles = vec![];

    let start = Instant::now();

    // Fire many requests simultaneously
    for _ in 0..burst_size {
        let r = Arc::clone(&router);
        handles.push(tokio::spawn(async move {
            r.get_daily_candles("BURST.SZ", None, None, Some(10)).await
        }));
    }

    // Wait for all
    let mut successes = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            successes += 1;
        }
    }

    let elapsed = start.elapsed();

    println!(
        "Burst test: {} requests in {:?} ({:.0} req/s)",
        burst_size,
        elapsed,
        burst_size as f64 / elapsed.as_secs_f64()
    );

    // All should succeed
    assert_eq!(successes, burst_size);
}
