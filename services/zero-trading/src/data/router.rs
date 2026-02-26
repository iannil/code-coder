//! Data provider router for multi-source failover.
//!
//! Routes requests to healthy providers based on priority and
//! automatically fails over to backup providers on errors.

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::health::{HealthMonitor, HealthMonitorConfig};
use super::provider::{DataCapabilities, DataProvider, ProviderError, ProviderInfo};
use super::provider::{ValuationMetrics, ValuationMetricName, StatisticsGranularity, ValuationStatisticsSet};
use super::{Candle, Timeframe};

// ============================================================================
// Router Configuration
// ============================================================================

/// Configuration for the data provider router.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Maximum retries per provider before failover
    pub max_retries: u32,
    /// Health monitor configuration
    pub health_config: HealthMonitorConfig,
    /// Enable automatic background health checks
    pub auto_health_check: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            max_retries: 2,
            health_config: HealthMonitorConfig::default(),
            auto_health_check: true,
        }
    }
}

// ============================================================================
// Provider Entry
// ============================================================================

/// A registered provider with its metadata.
struct ProviderEntry {
    /// The provider instance
    provider: Arc<dyn DataProvider>,
    /// Whether this provider is enabled
    enabled: bool,
}

// ============================================================================
// Data Provider Router
// ============================================================================

/// Routes data requests to healthy providers with automatic failover.
///
/// The router maintains a list of providers sorted by priority and
/// automatically routes requests to the healthiest available provider.
pub struct DataProviderRouter {
    /// Registered providers sorted by priority
    providers: RwLock<Vec<ProviderEntry>>,
    /// Health monitor
    health_monitor: Arc<HealthMonitor>,
    /// Configuration
    config: RouterConfig,
    /// Handle to background health check task
    health_task: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl DataProviderRouter {
    /// Create a new router with default configuration
    pub fn new() -> Self {
        Self::with_config(RouterConfig::default())
    }

    /// Create a new router with custom configuration
    pub fn with_config(config: RouterConfig) -> Self {
        let health_monitor = Arc::new(HealthMonitor::with_config(config.health_config.clone()));

        Self {
            providers: RwLock::new(Vec::new()),
            health_monitor,
            config,
            health_task: RwLock::new(None),
        }
    }

    /// Register a provider with the router.
    ///
    /// Providers are automatically sorted by priority (lower = higher priority).
    pub async fn register(&self, provider: Arc<dyn DataProvider>) {
        let name = provider.name();
        let priority = provider.priority();

        // Register with health monitor
        self.health_monitor.register_provider(name).await;

        // Add to providers list
        let mut providers = self.providers.write().await;
        providers.push(ProviderEntry {
            provider,
            enabled: true,
        });

        // Sort by priority (lower = higher priority)
        providers.sort_by_key(|e| e.provider.priority());

        info!(provider = name, priority, "Registered data provider");
    }

    /// Unregister a provider from the router
    pub async fn unregister(&self, name: &str) {
        self.health_monitor.unregister_provider(name).await;

        let mut providers = self.providers.write().await;
        providers.retain(|e| e.provider.name() != name);

        info!(provider = name, "Unregistered data provider");
    }

    /// Enable or disable a provider
    pub async fn set_enabled(&self, name: &str, enabled: bool) {
        let mut providers = self.providers.write().await;
        if let Some(entry) = providers.iter_mut().find(|e| e.provider.name() == name) {
            entry.enabled = enabled;
            info!(provider = name, enabled, "Provider enabled status changed");
        }
    }

    /// Get information about all registered providers
    pub async fn get_providers_info(&self) -> Vec<ProviderInfo> {
        let providers = self.providers.read().await;
        let mut infos = Vec::new();

        for entry in providers.iter() {
            let health = self
                .health_monitor
                .get_health(entry.provider.name())
                .await;
            let healthy = health.as_ref().map(|h| h.healthy).unwrap_or(false);

            let mut info = ProviderInfo::from_provider(entry.provider.as_ref(), healthy);
            if let Some(h) = health {
                info.last_success = h.last_success;
                info.last_error = h.last_error;
                info.request_count = h.total_checks;
                info.error_count = h.total_checks - h.successful_checks;
            }
            infos.push(info);
        }

        infos
    }

    /// Get the health monitor for external access
    pub fn health_monitor(&self) -> Arc<HealthMonitor> {
        Arc::clone(&self.health_monitor)
    }

    /// Start the background health check task
    pub async fn start_health_checks(&self) {
        let mut handle = self.health_task.write().await;
        if handle.is_some() {
            return; // Already running
        }

        let providers: Vec<Arc<dyn DataProvider>> = {
            let providers = self.providers.read().await;
            providers
                .iter()
                .map(|e| Arc::clone(&e.provider))
                .collect()
        };

        let task = self
            .health_monitor
            .clone()
            .start_background_monitor_dyn(Arc::new(providers));

        *handle = Some(task);
        info!("Started background health checks");
    }

    /// Stop the background health check task
    pub async fn stop_health_checks(&self) {
        self.health_monitor.stop().await;

        let mut handle = self.health_task.write().await;
        if let Some(task) = handle.take() {
            task.abort();
            info!("Stopped background health checks");
        }
    }

    /// Get available providers sorted by priority, filtered by health and enabled status
    async fn get_available_providers(&self) -> Vec<Arc<dyn DataProvider>> {
        let providers = self.providers.read().await;
        let mut available = Vec::new();

        for entry in providers.iter() {
            if !entry.enabled {
                continue;
            }

            let healthy = self
                .health_monitor
                .is_healthy(entry.provider.name())
                .await;

            // Also skip providers that are currently rate limited
            let rate_limited = self
                .health_monitor
                .is_rate_limited(entry.provider.name())
                .await;

            if healthy && !rate_limited {
                available.push(Arc::clone(&entry.provider));
            }
        }

        available
    }

    /// Get providers that support a specific timeframe, sorted by priority
    ///
    /// Filters out providers that don't support the requested timeframe,
    /// avoiding unnecessary API calls that will fail.
    async fn get_capable_providers(
        &self,
        timeframe: Timeframe,
    ) -> Vec<Arc<dyn DataProvider>> {
        let providers = self.providers.read().await;
        let mut capable = Vec::new();

        for entry in providers.iter() {
            if !entry.enabled {
                continue;
            }

            let healthy = self
                .health_monitor
                .is_healthy(entry.provider.name())
                .await;

            let rate_limited = self
                .health_monitor
                .is_rate_limited(entry.provider.name())
                .await;

            let supports_timeframe = entry
                .provider
                .capabilities()
                .supports_timeframe(timeframe);

            if healthy && !rate_limited && supports_timeframe {
                capable.push(Arc::clone(&entry.provider));
            }
        }

        capable
    }

    /// Execute a request with automatic failover.
    ///
    /// Tries providers in priority order until one succeeds or all fail.
    async fn execute_with_failover<T, F, Fut>(&self, request_fn: F) -> Result<T, ProviderError>
    where
        F: Fn(Arc<dyn DataProvider>) -> Fut,
        Fut: std::future::Future<Output = Result<T, ProviderError>>,
    {
        let available = self.get_available_providers().await;

        if available.is_empty() {
            // Try unhealthy providers as last resort
            let providers = self.providers.read().await;
            let all_enabled: Vec<_> = providers
                .iter()
                .filter(|e| e.enabled)
                .map(|e| Arc::clone(&e.provider))
                .collect();

            if all_enabled.is_empty() {
                return Err(ProviderError::Unavailable(
                    "No data providers registered".into(),
                ));
            }

            warn!("No healthy providers available, trying unhealthy providers");

            for provider in all_enabled {
                let name = provider.name();
                debug!(provider = name, "Trying unhealthy provider as last resort");

                match request_fn(Arc::clone(&provider)).await {
                    Ok(result) => {
                        self.health_monitor.record_success(name).await;
                        return Ok(result);
                    }
                    Err(e) => {
                        self.health_monitor.record_failure(name, &e.to_string()).await;
                        warn!(provider = name, error = %e, "Unhealthy provider also failed");
                    }
                }
            }

            return Err(ProviderError::Unavailable(
                "All providers failed".into(),
            ));
        }

        let mut last_error = None;

        for provider in available {
            let name = provider.name();
            debug!(provider = name, "Routing request to provider");

            // Try with retries
            for attempt in 0..=self.config.max_retries {
                if attempt > 0 {
                    debug!(provider = name, attempt, "Retrying request");
                }

                match request_fn(Arc::clone(&provider)).await {
                    Ok(result) => {
                        self.health_monitor.record_success(name).await;
                        return Ok(result);
                    }
                    Err(e) => {
                        // Record rate limit events with specific cooldown time
                        if let ProviderError::RateLimited { retry_after_secs } = &e {
                            let retry_secs = retry_after_secs.unwrap_or(5);
                            self.health_monitor
                                .record_rate_limited(name, retry_secs)
                                .await;
                        } else {
                            self.health_monitor.record_failure(name, &e.to_string()).await;
                        }

                        // Check if we should failover to next provider
                        if e.should_failover() {
                            warn!(
                                provider = name,
                                error = %e,
                                "Provider error, failing over to next provider"
                            );
                            last_error = Some(e);
                            break; // Exit retry loop, try next provider
                        }

                        // For rate limiting, wait and retry
                        if let ProviderError::RateLimited { retry_after_secs } = &e {
                            if attempt < self.config.max_retries {
                                let wait_secs = retry_after_secs.unwrap_or(5);
                                debug!(provider = name, wait_secs, "Rate limited, waiting");
                                tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                                continue;
                            }
                        }

                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::Unavailable("All providers failed".into())))
    }

    /// Execute a request with automatic failover, filtering by timeframe capability.
    ///
    /// Similar to execute_with_failover, but only considers providers that
    /// support the requested timeframe. This avoids unnecessary API calls
    /// that will fail with DataNotAvailable.
    async fn execute_with_failover_with_timeframe<T, F, Fut>(
        &self,
        timeframe: Timeframe,
        request_fn: F,
    ) -> Result<T, ProviderError>
    where
        F: Fn(Arc<dyn DataProvider>) -> Fut,
        Fut: std::future::Future<Output = Result<T, ProviderError>>,
    {
        let available = self.get_capable_providers(timeframe).await;

        if available.is_empty() {
            // Check if any enabled providers exist
            let providers = self.providers.read().await;
            let any_enabled = providers.iter().any(|e| e.enabled);
            drop(providers);

            if any_enabled {
                return Err(ProviderError::DataNotAvailable(format!(
                    "No providers support timeframe '{}'",
                    timeframe
                )));
            } else {
                return Err(ProviderError::Unavailable(
                    "No data providers registered".into(),
                ));
            }
        }

        let mut last_error = None;

        for provider in available {
            let name = provider.name();
            debug!(provider = name, timeframe = %timeframe, "Routing request to provider");

            // Try with retries
            for attempt in 0..=self.config.max_retries {
                if attempt > 0 {
                    debug!(provider = name, attempt, "Retrying request");
                }

                match request_fn(Arc::clone(&provider)).await {
                    Ok(result) => {
                        self.health_monitor.record_success(name).await;
                        return Ok(result);
                    }
                    Err(e) => {
                        // Record rate limit events with specific cooldown time
                        if let ProviderError::RateLimited { retry_after_secs } = &e {
                            let retry_secs = retry_after_secs.unwrap_or(5);
                            self.health_monitor
                                .record_rate_limited(name, retry_secs)
                                .await;
                        } else {
                            self.health_monitor.record_failure(name, &e.to_string()).await;
                        }

                        // Check if we should failover to next provider
                        if e.should_failover() {
                            warn!(
                                provider = name,
                                error = %e,
                                "Provider error, failing over to next provider"
                            );
                            last_error = Some(e);
                            break; // Exit retry loop, try next provider
                        }

                        // For rate limiting, wait and retry
                        if let ProviderError::RateLimited { retry_after_secs } = &e {
                            if attempt < self.config.max_retries {
                                let wait_secs = retry_after_secs.unwrap_or(5);
                                debug!(provider = name, wait_secs, "Rate limited, waiting");
                                tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
                                continue;
                            }
                        }

                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::Unavailable("All providers failed".into())))
    }

    // ========================================================================
    // Public Data Fetching Methods
    // ========================================================================

    /// Fetch daily candles with automatic failover.
    pub async fn get_daily_candles(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let symbol = symbol.to_string();
        self.execute_with_failover(move |provider| {
            let symbol = symbol.clone();
            async move {
                provider
                    .get_daily_candles(&symbol, start_date, end_date, limit)
                    .await
            }
        })
        .await
    }

    /// Fetch minute candles with automatic failover.
    pub async fn get_minute_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let symbol = symbol.to_string();
        self.execute_with_failover_with_timeframe(timeframe, move |provider| {
            let symbol = symbol.clone();
            async move {
                provider
                    .get_minute_candles(&symbol, timeframe, start_time, end_time)
                    .await
            }
        })
        .await
    }

    /// Fetch index daily data with automatic failover.
    pub async fn get_index_daily(
        &self,
        symbol: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Candle>, ProviderError> {
        let symbol = symbol.to_string();
        self.execute_with_failover(move |provider| {
            let symbol = symbol.clone();
            async move {
                provider
                    .get_index_daily(&symbol, start_date, end_date)
                    .await
            }
        })
        .await
    }

    /// Fetch valuation metrics with automatic failover.
    pub async fn get_valuation_metrics(
        &self,
        symbol: &str,
        date: Option<NaiveDate>,
    ) -> Result<ValuationMetrics, ProviderError> {
        let symbol = symbol.to_string();
        self.execute_with_failover(move |provider| {
            let symbol = symbol.clone();
            async move {
                provider
                    .get_valuation_metrics(&symbol, date)
                    .await
            }
        })
        .await
    }

    /// Batch fetch valuation metrics with automatic failover.
    pub async fn batch_get_valuation_metrics(
        &self,
        symbols: &[String],
        date: Option<NaiveDate>,
    ) -> Result<Vec<ValuationMetrics>, ProviderError> {
        let symbols = symbols.to_vec();
        let date = date;
        self.execute_with_failover(move |provider| {
            let symbols = symbols.clone();
            async move {
                provider
                    .batch_get_valuation_metrics(&symbols, date)
                    .await
            }
        })
        .await
    }

    /// Fetch valuation statistics with automatic failover.
    pub async fn get_valuation_statistics(
        &self,
        symbol: &str,
        metrics: &[ValuationMetricName],
        granularities: &[StatisticsGranularity],
        date: Option<NaiveDate>,
    ) -> Result<ValuationStatisticsSet, ProviderError> {
        let symbol = symbol.to_string();
        let metrics = metrics.to_vec();
        let granularities = granularities.to_vec();
        self.execute_with_failover(move |provider| {
            let symbol = symbol.clone();
            let metrics = metrics.clone();
            let granularities = granularities.clone();
            async move {
                provider
                    .get_valuation_statistics(&symbol, &metrics, &granularities, date)
                    .await
            }
        })
        .await
    }

    /// Get combined capabilities of all enabled providers
    pub async fn combined_capabilities(&self) -> DataCapabilities {
        let providers = self.providers.read().await;
        let mut combined = DataCapabilities::default();

        for entry in providers.iter().filter(|e| e.enabled) {
            let caps = entry.provider.capabilities();

            // Merge timeframes
            for tf in caps.timeframes {
                if !combined.timeframes.contains(&tf) {
                    combined.timeframes.push(tf);
                }
            }

            // OR the boolean capabilities
            combined.realtime_quotes |= caps.realtime_quotes;
            combined.auction_data |= caps.auction_data;
            combined.index_data |= caps.index_data;
            combined.etf_data |= caps.etf_data;

            // Take the maximum history
            combined.max_history_days = match (combined.max_history_days, caps.max_history_days) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
        }

        combined
    }
}

impl Default for DataProviderRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Mock provider for testing
    struct MockProvider {
        name: &'static str,
        priority: u8,
        fail_count: AtomicU32,
        should_fail: bool,
    }

    impl MockProvider {
        fn new(name: &'static str, priority: u8, should_fail: bool) -> Self {
            Self {
                name,
                priority,
                fail_count: AtomicU32::new(0),
                should_fail,
            }
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
            self.fail_count.fetch_add(1, Ordering::Relaxed);

            if self.should_fail {
                Err(ProviderError::Network("mock failure".into()))
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

    #[tokio::test]
    async fn test_router_register() {
        let router = DataProviderRouter::new();

        router
            .register(Arc::new(MockProvider::new("test", 1, false)))
            .await;

        let infos = router.get_providers_info().await;
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name, "test");
        assert_eq!(infos[0].priority, 1);
    }

    #[tokio::test]
    async fn test_router_priority_ordering() {
        let router = DataProviderRouter::new();

        router
            .register(Arc::new(MockProvider::new("low", 10, false)))
            .await;
        router
            .register(Arc::new(MockProvider::new("high", 1, false)))
            .await;
        router
            .register(Arc::new(MockProvider::new("medium", 5, false)))
            .await;

        let infos = router.get_providers_info().await;
        assert_eq!(infos[0].name, "high");
        assert_eq!(infos[1].name, "medium");
        assert_eq!(infos[2].name, "low");
    }

    #[tokio::test]
    async fn test_router_failover() {
        let router = DataProviderRouter::with_config(RouterConfig {
            max_retries: 0, // No retries to speed up test
            ..Default::default()
        });

        let failing = Arc::new(MockProvider::new("failing", 1, true));
        let working = Arc::new(MockProvider::new("working", 2, false));

        router.register(failing.clone()).await;
        router.register(working.clone()).await;

        // First request should try failing provider, then failover to working
        let result = router
            .get_daily_candles("000001.SZ", None, None, None)
            .await;
        assert!(result.is_ok());

        // Verify failing provider was tried
        assert!(failing.fail_count.load(Ordering::Relaxed) > 0);
    }

    #[tokio::test]
    async fn test_router_combined_capabilities() {
        let router = DataProviderRouter::new();

        router
            .register(Arc::new(MockProvider::new("daily", 1, false)))
            .await;

        let caps = router.combined_capabilities().await;
        assert!(caps.supports_timeframe(Timeframe::Daily));
    }

    #[tokio::test]
    async fn test_router_disable_provider() {
        let router = DataProviderRouter::new();

        router
            .register(Arc::new(MockProvider::new("test", 1, false)))
            .await;

        router.set_enabled("test", false).await;

        // With no enabled providers, should fail
        let result = router
            .get_daily_candles("000001.SZ", None, None, None)
            .await;
        assert!(result.is_err());
    }

    /// Mock provider with configurable timeframe support
    struct MockProviderWithTimeframes {
        name: &'static str,
        priority: u8,
        supported_timeframes: Vec<Timeframe>,
    }

    impl MockProviderWithTimeframes {
        fn new(name: &'static str, priority: u8, supported_timeframes: Vec<Timeframe>) -> Self {
            Self {
                name,
                priority,
                supported_timeframes,
            }
        }
    }

    #[async_trait]
    impl DataProvider for MockProviderWithTimeframes {
        fn name(&self) -> &'static str {
            self.name
        }

        fn priority(&self) -> u8 {
            self.priority
        }

        fn capabilities(&self) -> DataCapabilities {
            DataCapabilities {
                timeframes: self.supported_timeframes.clone(),
                ..Default::default()
            }
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

        async fn get_minute_candles(
            &self,
            symbol: &str,
            timeframe: Timeframe,
            _start_time: Option<DateTime<Utc>>,
            _end_time: Option<DateTime<Utc>>,
        ) -> Result<Vec<Candle>, ProviderError> {
            if !self.supported_timeframes.contains(&timeframe) {
                return Err(ProviderError::DataNotAvailable(format!(
                    "Timeframe {:?} not supported",
                    timeframe
                )));
            }
            Ok(vec![Candle {
                symbol: symbol.to_string(),
                timeframe,
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

    #[tokio::test]
    async fn test_router_capability_filtering() {
        let router = DataProviderRouter::new();

        // Register a daily-only provider
        router
            .register(Arc::new(MockProviderWithTimeframes::new(
                "daily_only",
                1,
                vec![Timeframe::Daily],
            )))
            .await;

        // Requesting H4 should fail with DataNotAvailable, not try the provider
        let result = router
            .get_minute_candles("000001.SZ", Timeframe::H4, None, None)
            .await;

        assert!(result.is_err());
        match result {
            Err(ProviderError::DataNotAvailable(msg)) => {
                assert!(msg.contains("H4") || msg.contains("timeframe"));
            }
            _ => panic!("Expected DataNotAvailable error, got {:?}", result),
        }
    }

    #[tokio::test]
    async fn test_router_capability_filtering_with_fallback() {
        let router = DataProviderRouter::new();

        // Register daily-only provider
        router
            .register(Arc::new(MockProviderWithTimeframes::new(
                "daily_only",
                1,
                vec![Timeframe::Daily],
            )))
            .await;

        // Register full-capability provider
        router
            .register(Arc::new(MockProviderWithTimeframes::new(
                "full_capability",
                2, // Lower priority (higher number)
                vec![Timeframe::Daily, Timeframe::H1, Timeframe::H4, Timeframe::M5],
            )))
            .await;

        // Requesting H4 should work via the full-capability provider
        let result = router
            .get_minute_candles("000001.SZ", Timeframe::H4, None, None)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }
}
