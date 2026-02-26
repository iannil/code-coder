//! Market data aggregator for multi-timeframe analysis.
//!
//! Provides a unified interface to fetch and aggregate data across timeframes.
//! Uses DataProviderRouter for automatic failover between providers.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::ashare::AshareAdapter;
use super::health::HealthMonitorConfig;
use super::lixin::LixinAdapter;
use super::router::{DataProviderRouter, RouterConfig};
use super::{Candle, DataCache, ProviderInfo, SmtPair, Timeframe};
use zero_common::config::Config;

/// Market data aggregator with multi-provider support.
///
/// Manages multiple data providers (Ashare, Lixin) with automatic
/// failover based on health status and priority.
pub struct MarketDataAggregator {
    /// Data provider router for failover
    router: Arc<DataProviderRouter>,
    /// Data cache
    cache: Arc<DataCache>,
    /// Whether we're connected to any data source
    connected: AtomicBool,
    /// SMT pairs to track
    smt_pairs: Vec<SmtPair>,
    /// Symbols to track
    tracked_symbols: RwLock<Vec<String>>,
    /// Last update timestamp
    last_update: RwLock<Option<DateTime<Utc>>>,
}

impl MarketDataAggregator {
    /// Create a new aggregator from config
    pub fn new(config: &Config) -> Self {
        Self::with_router(Self::create_router(config), config)
    }

    /// Create with an existing router (for testing)
    pub fn with_router(router: Arc<DataProviderRouter>, config: &Config) -> Self {
        // Convert SmtPairConfig to SmtPair if configured, else use defaults
        let smt_pairs = config
            .trading
            .as_ref()
            .and_then(|t| t.smt_pairs.as_ref())
            .map(|pairs| {
                pairs
                    .iter()
                    .map(|p| SmtPair {
                        primary: p.primary.clone(),
                        reference: p.reference.clone(),
                        name: p.name.clone(),
                        description: p.description.clone(),
                    })
                    .collect()
            })
            .unwrap_or_else(super::default_smt_pairs);

        Self {
            router,
            cache: Arc::new(DataCache::with_ttl(60)),
            connected: AtomicBool::new(true), // Will be updated by health checks
            smt_pairs,
            tracked_symbols: RwLock::new(Vec::new()),
            last_update: RwLock::new(None),
        }
    }

    /// Create a DataProviderRouter from config
    fn create_router(config: &Config) -> Arc<DataProviderRouter> {
        let trading_config = config.trading.as_ref();

        // Build router config from data_sources config
        let router_config = trading_config
            .and_then(|t| t.data_sources.as_ref())
            .map(|ds| RouterConfig {
                max_retries: ds.max_retries,
                health_config: HealthMonitorConfig {
                    check_interval_secs: ds.health_check_interval_secs,
                    unhealthy_threshold: ds.unhealthy_threshold,
                    check_timeout_secs: ds.health_check_timeout_secs,
                },
                auto_health_check: true,
            })
            .unwrap_or_default();

        let router = Arc::new(DataProviderRouter::with_config(router_config));

        // We need to register providers asynchronously
        // This is handled in the async initialization

        router
    }

    /// Initialize the aggregator (register providers and start health checks).
    ///
    /// This must be called after `new()` to complete async initialization.
    pub async fn initialize(&self, config: &Config) -> Result<()> {
        let trading_config = config.trading.as_ref();

        // Get enabled providers from config
        let data_sources = trading_config.and_then(|t| t.data_sources.as_ref());

        let mut has_provider = false;

        if let Some(ds) = data_sources {
            for entry in &ds.sources {
                if !entry.enabled {
                    continue;
                }

                match entry.provider.as_str() {
                    "ashare" => {
                        let adapter = AshareAdapter::with_priority(entry.priority);
                        self.router.register(Arc::new(adapter)).await;
                        info!(provider = "ashare", priority = entry.priority, "Registered Ashare provider");
                        has_provider = true;
                    }
                    "lixin" => {
                        if let Some(token) = trading_config.and_then(|t| t.lixin_token.as_ref()) {
                            let adapter = LixinAdapter::with_priority(token.clone(), entry.priority);
                            self.router.register(Arc::new(adapter)).await;
                            info!(provider = "lixin", priority = entry.priority, "Registered Lixin provider");
                            has_provider = true;
                        } else {
                            warn!("Lixin enabled but no token configured");
                        }
                    }
                    other => {
                        warn!(provider = other, "Unknown data provider");
                    }
                }
            }
        } else {
            // Default: use Ashare if no explicit config
            let adapter = AshareAdapter::new();
            self.router.register(Arc::new(adapter)).await;
            info!(provider = "ashare", "Registered default Ashare provider");
            has_provider = true;

            // Also register Lixin if token is available (as backup)
            if let Some(token) = trading_config.and_then(|t| t.lixin_token.as_ref()) {
                let adapter = LixinAdapter::new(token.clone());
                self.router.register(Arc::new(adapter)).await;
                info!(provider = "lixin", "Registered Lixin provider as backup");
                has_provider = true;
            }
        }

        // Start background health checks
        if has_provider {
            self.router.start_health_checks().await;
            self.connected.store(true, Ordering::Relaxed);
        } else {
            self.connected.store(false, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Check if connected to any data source
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Get information about all registered providers
    pub async fn get_providers_info(&self) -> Vec<ProviderInfo> {
        self.router.get_providers_info().await
    }

    /// Add a symbol to track
    pub async fn add_symbol(&self, symbol: &str) {
        let mut symbols = self.tracked_symbols.write().await;
        if !symbols.contains(&symbol.to_string()) {
            symbols.push(symbol.to_string());
        }
    }

    /// Remove a symbol from tracking
    pub async fn remove_symbol(&self, symbol: &str) {
        let mut symbols = self.tracked_symbols.write().await;
        symbols.retain(|s| s != symbol);
    }

    /// Get tracked symbols
    pub async fn get_tracked_symbols(&self) -> Vec<String> {
        self.tracked_symbols.read().await.clone()
    }

    /// Get candles for a symbol and timeframe
    pub async fn get_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        // Check cache first
        if let Some(candles) = self.cache.get_candles(symbol, timeframe) {
            if candles.len() >= limit {
                return Ok(candles.into_iter().rev().take(limit).rev().collect());
            }
        }

        // Fetch from router (with automatic failover)
        let candles = self.fetch_candles(symbol, timeframe, limit).await?;

        // Cache the result
        self.cache.set_candles(symbol, timeframe, candles.clone());

        Ok(candles)
    }

    /// Fetch candles from the data provider router
    async fn fetch_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        debug!(symbol, ?timeframe, limit, "Fetching candles via router");

        match timeframe {
            Timeframe::Daily | Timeframe::Weekly => {
                // Check if this is an index symbol
                let is_index = symbol.starts_with("000") && symbol.ends_with(".SH");
                if is_index {
                    self.router
                        .get_index_daily(symbol, None, None)
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                } else {
                    self.router
                        .get_daily_candles(symbol, None, None, Some(limit))
                        .await
                        .map_err(|e| anyhow::anyhow!("{}", e))
                }
            }
            Timeframe::H4 => {
                // Aggregate from hourly candles
                let h1_candles = self
                    .router
                    .get_minute_candles(symbol, Timeframe::H1, None, None)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                Ok(aggregate_candles(&h1_candles, 4))
            }
            _ => {
                // Direct minute candles
                self.router
                    .get_minute_candles(symbol, timeframe, None, None)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))
            }
        }
    }

    /// Get multi-timeframe candles for a symbol
    pub async fn get_multi_timeframe(
        &self,
        symbol: &str,
        timeframes: &[Timeframe],
        limit: usize,
    ) -> Result<HashMap<Timeframe, Vec<Candle>>> {
        let mut result = HashMap::new();

        for tf in timeframes {
            match self.get_candles(symbol, *tf, limit).await {
                Ok(candles) => {
                    result.insert(*tf, candles);
                }
                Err(e) => {
                    warn!(symbol, timeframe = ?tf, error = %e, "Failed to fetch candles");
                }
            }
        }

        Ok(result)
    }

    /// Get SMT pair data
    pub async fn get_smt_pair_data(
        &self,
        pair: &SmtPair,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<(Vec<Candle>, Vec<Candle>)> {
        let primary = self.get_candles(&pair.primary, timeframe, limit).await?;
        let reference = self.get_candles(&pair.reference, timeframe, limit).await?;

        Ok((primary, reference))
    }

    /// Get all configured SMT pairs
    pub fn get_smt_pairs(&self) -> &[SmtPair] {
        &self.smt_pairs
    }

    /// Start the background data updater
    pub async fn start_updater(&self) -> Result<()> {
        info!("Starting market data updater");

        loop {
            // Update tracked symbols
            let symbols = self.tracked_symbols.read().await.clone();

            for symbol in &symbols {
                // Fetch common timeframes
                for tf in &[Timeframe::Daily, Timeframe::H4, Timeframe::H1] {
                    if let Err(e) = self.get_candles(symbol, *tf, 100).await {
                        warn!(symbol, timeframe = ?tf, error = %e, "Failed to update candles");
                    }
                }
            }

            // Update SMT pairs
            for pair in &self.smt_pairs {
                for tf in &[Timeframe::Daily, Timeframe::H4] {
                    if let Err(e) = self.get_smt_pair_data(pair, *tf, 50).await {
                        warn!(pair = %pair.name, timeframe = ?tf, error = %e, "Failed to update SMT data");
                    }
                }
            }

            // Update last update time
            *self.last_update.write().await = Some(Utc::now());

            // Clear expired cache entries
            self.cache.clear_expired();

            // Check provider health and update connected status
            let healthy_providers = self.router.health_monitor().healthy_providers().await;
            self.connected
                .store(!healthy_providers.is_empty(), Ordering::Relaxed);

            // Sleep for 1 minute
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    /// Get last update timestamp
    pub async fn last_update_time(&self) -> Option<DateTime<Utc>> {
        *self.last_update.read().await
    }

    /// Get the latest quote for a symbol
    pub async fn get_latest_quote(&self, symbol: &str) -> Result<Candle> {
        // Try to get the most recent daily candle
        let candles = self.get_candles(symbol, Timeframe::Daily, 1).await?;
        candles
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No quote available for {}", symbol))
    }

    /// Get the underlying router for direct access
    pub fn router(&self) -> Arc<DataProviderRouter> {
        Arc::clone(&self.router)
    }

    /// Stop background health checks
    pub async fn shutdown(&self) {
        info!("Shutting down market data aggregator");
        self.router.stop_health_checks().await;
    }
}

/// Aggregate smaller timeframe candles into larger ones
fn aggregate_candles(candles: &[Candle], ratio: usize) -> Vec<Candle> {
    if candles.is_empty() || ratio < 2 {
        return candles.to_vec();
    }

    candles
        .chunks(ratio)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| {
            let first = &chunk[0];
            let last = &chunk[chunk.len() - 1];

            Candle {
                symbol: first.symbol.clone(),
                timeframe: Timeframe::H4, // Assuming H4 for now
                timestamp: last.timestamp,
                open: first.open,
                high: chunk
                    .iter()
                    .map(|c| c.high)
                    .fold(f64::NEG_INFINITY, f64::max),
                low: chunk.iter().map(|c| c.low).fold(f64::INFINITY, f64::min),
                close: last.close,
                volume: chunk.iter().map(|c| c.volume).sum(),
                amount: chunk.iter().map(|c| c.amount).sum(),
            }
        })
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|i| Candle {
                symbol: "000001.SZ".to_string(),
                timeframe: Timeframe::H1,
                timestamp: Utc::now(),
                open: 10.0 + i as f64 * 0.1,
                high: 10.5 + i as f64 * 0.1,
                low: 9.5 + i as f64 * 0.1,
                close: 10.2 + i as f64 * 0.1,
                volume: 1000.0,
                amount: 10000.0,
            })
            .collect()
    }

    #[test]
    fn test_aggregate_candles() {
        let candles = make_test_candles(8);
        let aggregated = aggregate_candles(&candles, 4);

        assert_eq!(aggregated.len(), 2);
        assert_eq!(aggregated[0].open, 10.0);
        assert_eq!(aggregated[0].close, 10.2 + 0.3); // 4th candle's close
    }

    #[test]
    fn test_aggregate_candles_empty() {
        let candles: Vec<Candle> = vec![];
        let aggregated = aggregate_candles(&candles, 4);
        assert!(aggregated.is_empty());
    }

    #[tokio::test]
    async fn test_aggregator_creation() {
        let config = Config::default();
        let aggregator = MarketDataAggregator::new(&config);

        // Initially not connected until initialized
        // After initialization, should have at least Ashare registered
        aggregator.initialize(&config).await.unwrap();

        let providers = aggregator.get_providers_info().await;
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.name == "ashare"));
    }
}
