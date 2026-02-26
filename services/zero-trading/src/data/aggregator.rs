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

use super::itick::ITickAdapter;
use super::health::HealthMonitorConfig;
use super::lixin::LixinAdapter;
use super::local_storage::{LocalStorage, LocalStorageConfig};
use super::router::{DataProviderRouter, RouterConfig};
use super::{Candle, DataCache, IndexData, IndexOverview, ProviderInfo, SmtPair, Timeframe};
use super::default_tracked_symbols;
use zero_common::config::Config;

/// Market data aggregator with multi-provider support.
///
/// Manages multiple data providers (iTick, Lixin) with automatic
/// failover based on health status and priority.
pub struct MarketDataAggregator {
    /// Data provider router for failover
    router: Arc<DataProviderRouter>,
    /// Data cache (in-memory TTL cache)
    cache: Arc<DataCache>,
    /// Local storage for persistent data (SQLite)
    local_storage: Option<Arc<LocalStorage>>,
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

        // Get tracked symbols from config, or use default major indices
        let tracked_symbols = config
            .trading
            .as_ref()
            .and_then(|t| t.tracked_symbols.as_ref())
            .cloned()
            .unwrap_or_else(default_tracked_symbols);

        // Initialize local storage if enabled in config
        let local_storage = Self::create_local_storage(config);

        Self {
            router,
            cache: Arc::new(DataCache::with_ttl(60)),
            local_storage,
            connected: AtomicBool::new(true), // Will be updated by health checks
            smt_pairs,
            tracked_symbols: RwLock::new(tracked_symbols),
            last_update: RwLock::new(None),
        }
    }

    /// Create local storage from config
    fn create_local_storage(config: &Config) -> Option<Arc<LocalStorage>> {
        // Check if local storage is enabled in config
        let ls_config = config
            .trading
            .as_ref()
            .and_then(|t| t.local_storage.as_ref())
            .map(|ls| LocalStorageConfig {
                enabled: ls.enabled,
                db_path: ls.db_path.clone().map(std::path::PathBuf::from)
                    .unwrap_or_else(|| LocalStorageConfig::default().db_path),
                candle_retention_days: ls.candle_retention_days.unwrap_or(365),
                financial_retention_years: ls.financial_retention_years.unwrap_or(5),
                auto_sync_on_startup: ls.auto_sync_on_startup.unwrap_or(true),
            })
            .unwrap_or_default();

        if !ls_config.enabled {
            info!("Local storage is disabled in config");
            return None;
        }

        match LocalStorage::new(ls_config) {
            Ok(storage) => {
                info!("Local storage initialized successfully");
                Some(Arc::new(storage))
            }
            Err(e) => {
                warn!(error = %e, "Failed to initialize local storage, continuing without it");
                None
            }
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
                    "itick" => {
                        if let Some(api_key) = config.itick_api_key() {
                            let adapter = ITickAdapter::with_priority(api_key, entry.priority);
                            self.router.register(Arc::new(adapter)).await;
                            info!(provider = "itick", priority = entry.priority, "Registered iTick provider");
                            has_provider = true;
                        } else {
                            warn!("iTick enabled but no API key configured (set secrets.external.itick)");
                        }
                    }
                    "lixin" => {
                        if let Some(token) = config.lixin_token() {
                            let adapter = LixinAdapter::with_priority(token, entry.priority);
                            self.router.register(Arc::new(adapter)).await;
                            info!(provider = "lixin", priority = entry.priority, "Registered Lixin provider");
                            has_provider = true;
                        } else {
                            warn!("Lixin enabled but no token configured (set secrets.external.lixin)");
                        }
                    }
                    other => {
                        warn!(provider = other, "Unknown data provider");
                    }
                }
            }
        } else {
            // Default: use iTick if API key is available, otherwise warn
            if let Some(api_key) = config.itick_api_key() {
                let adapter = ITickAdapter::new(api_key);
                self.router.register(Arc::new(adapter)).await;
                info!(provider = "itick", "Registered default iTick provider");
                has_provider = true;
            } else {
                warn!(
                    "No iTick API key configured. Set secrets.external.itick in config. \
                     Get a free API key at https://itick.org"
                );
            }

            // Also register Lixin if token is available (as backup)
            if let Some(token) = config.lixin_token() {
                let adapter = LixinAdapter::new(token);
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

    /// Get the local storage instance (if enabled)
    pub fn local_storage(&self) -> Option<Arc<LocalStorage>> {
        self.local_storage.clone()
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
    ///
    /// Data fetching priority:
    /// 1. In-memory TTL cache (fastest)
    /// 2. Local SQLite storage (persistent)
    /// 3. Remote API providers (with failover)
    pub async fn get_candles(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        limit: usize,
    ) -> Result<Vec<Candle>> {
        // 1. Check in-memory cache first (fastest)
        if let Some(candles) = self.cache.get_candles(symbol, timeframe) {
            if candles.len() >= limit {
                debug!(symbol, ?timeframe, source = "cache", "Returning candles from in-memory cache");
                return Ok(candles.into_iter().rev().take(limit).rev().collect());
            }
        }

        // 2. Check local storage (persistent)
        if let Some(ref storage) = self.local_storage {
            match storage.get_candles(symbol, timeframe, None, None, Some(limit)).await {
                Ok(candles) if candles.len() >= limit => {
                    debug!(symbol, ?timeframe, source = "local_storage", "Returning candles from local storage");
                    // Update in-memory cache
                    self.cache.set_candles(symbol, timeframe, candles.clone());
                    return Ok(candles);
                }
                Ok(candles) if !candles.is_empty() => {
                    // Have some data but not enough, will fetch more from remote
                    debug!(
                        symbol, ?timeframe,
                        local_count = candles.len(),
                        needed = limit,
                        "Local storage has partial data, fetching from remote"
                    );
                }
                Err(e) => {
                    warn!(symbol, ?timeframe, error = %e, "Failed to read from local storage");
                }
                _ => {}
            }
        }

        // 3. Fetch from remote providers (with automatic failover)
        let candles = self.fetch_candles(symbol, timeframe, limit).await?;

        // Save to local storage for future use
        if let Some(ref storage) = self.local_storage {
            if let Err(e) = storage.save_candles(&candles, "remote").await {
                warn!(symbol, ?timeframe, error = %e, "Failed to save candles to local storage");
            }
        }

        // Cache the result in memory
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

    /// Get index overview data for daily macro reports.
    ///
    /// Fetches recent daily candles for the given index symbols and computes
    /// change percentage and moving averages.
    ///
    /// # Arguments
    /// * `symbols` - Index symbols to fetch (e.g., ["000300.SH", "000905.SH"])
    /// * `days` - Number of days of history to use for MA calculation
    ///
    /// # Returns
    /// An `IndexOverview` containing data for all requested indices.
    pub async fn get_index_overview(&self, symbols: &[String], days: u32) -> Result<IndexOverview> {
        let mut indices = Vec::new();

        for symbol in symbols {
            match self.get_candles(symbol, Timeframe::Daily, days as usize + 1).await {
                Ok(candles) if !candles.is_empty() => {
                    let latest = &candles[candles.len() - 1];
                    let prev = if candles.len() > 1 {
                        Some(&candles[candles.len() - 2])
                    } else {
                        None
                    };

                    // Calculate change percentage
                    let change_pct = prev
                        .map(|p| {
                            if p.close > 0.0 {
                                ((latest.close - p.close) / p.close) * 100.0
                            } else {
                                0.0
                            }
                        })
                        .unwrap_or(0.0);

                    // Calculate moving averages
                    let ma5 = if candles.len() >= 5 {
                        let sum: f64 = candles[candles.len() - 5..].iter().map(|c| c.close).sum();
                        Some(sum / 5.0)
                    } else {
                        None
                    };

                    let ma20 = if candles.len() >= 20 {
                        let sum: f64 = candles[candles.len() - 20..].iter().map(|c| c.close).sum();
                        Some(sum / 20.0)
                    } else {
                        None
                    };

                    // Get index name from symbol
                    let name = self.get_index_name(symbol);

                    indices.push(IndexData {
                        symbol: symbol.clone(),
                        name,
                        close: latest.close,
                        change_pct,
                        volume: latest.volume,
                        ma5,
                        ma20,
                    });
                }
                Ok(_) => {
                    warn!(symbol, "No candle data available for index");
                }
                Err(e) => {
                    warn!(symbol, error = %e, "Failed to fetch index data");
                }
            }
        }

        Ok(IndexOverview {
            indices,
            as_of: Utc::now(),
        })
    }

    /// Get human-readable index name from symbol
    fn get_index_name(&self, symbol: &str) -> String {
        match symbol {
            "000001.SH" => "上证指数".to_string(),
            "000300.SH" => "沪深300".to_string(),
            "000905.SH" => "中证500".to_string(),
            "000016.SH" => "上证50".to_string(),
            "000688.SH" => "科创50".to_string(),
            "399001.SZ" => "深证成指".to_string(),
            "399006.SZ" => "创业板指".to_string(),
            _ => symbol.to_string(),
        }
    }

    /// Start the background data updater
    pub async fn start_updater(&self) -> Result<()> {
        info!("Starting market data updater");

        // Check if any providers are registered before starting the update loop
        if !self.is_connected() {
            warn!(
                "No data providers registered. Market data updater will not run. \
                 Configure API keys (secrets.external.itick or secrets.external.lixin) to enable data updates."
            );
            return Ok(());
        }

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

    /// Preload historical data for all tracked symbols (24/7 operation).
    ///
    /// This method is called by the preparation task runner to keep
    /// cached data fresh, ensuring fast response during trading hours.
    ///
    /// # Returns
    /// Number of symbols successfully preloaded.
    pub async fn preload_historical_data(&self) -> Result<usize> {
        let symbols = self.tracked_symbols.read().await.clone();

        if symbols.is_empty() {
            debug!("No tracked symbols to preload");
            return Ok(0);
        }

        info!(count = symbols.len(), "Preloading historical data");

        let mut success_count = 0;

        for symbol in &symbols {
            // Preload common timeframes used in analysis
            for tf in [Timeframe::Daily, Timeframe::H4, Timeframe::H1] {
                match self.get_candles(symbol, tf, 200).await {
                    Ok(_) => {
                        debug!(
                            symbol = %symbol,
                            timeframe = ?tf,
                            "Preloaded candle data"
                        );
                    }
                    Err(e) => {
                        debug!(
                            symbol = %symbol,
                            timeframe = ?tf,
                            error = %e,
                            "Failed to preload candle data"
                        );
                    }
                }
            }
            success_count += 1;
        }

        info!(
            total = symbols.len(),
            success = success_count,
            "Historical data preload completed"
        );

        Ok(success_count)
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

        // Initialize without API keys - should have no providers
        aggregator.initialize(&config).await.unwrap();

        let providers = aggregator.get_providers_info().await;
        // Without API keys configured, no providers should be registered
        // (iTick requires API key, Lixin requires token)
        assert!(providers.is_empty());
        assert!(!aggregator.is_connected());
    }

    #[tokio::test]
    async fn test_updater_exits_without_providers() {
        let config = Config::default();
        let aggregator = MarketDataAggregator::new(&config);

        // Initialize without API keys - should have no providers
        aggregator.initialize(&config).await.unwrap();

        // start_updater should exit immediately when no providers are registered
        let result = aggregator.start_updater().await;
        assert!(result.is_ok(), "start_updater should return Ok when no providers");
        assert!(!aggregator.is_connected(), "should still be disconnected");
    }

    #[tokio::test]
    #[ignore = "requires valid iTick API key"]
    async fn test_aggregator_with_itick() {
        let api_key = std::env::var("ITICK_API_KEY").expect("ITICK_API_KEY not set");

        let mut config = Config::default();
        let mut trading_config = zero_common::config::TradingConfig::default();
        trading_config.itick_api_key = Some(api_key);
        config.trading = Some(trading_config);

        let aggregator = MarketDataAggregator::new(&config);
        aggregator.initialize(&config).await.unwrap();

        let providers = aggregator.get_providers_info().await;
        assert!(!providers.is_empty());
        assert!(providers.iter().any(|p| p.name == "itick"));
        assert!(aggregator.is_connected());
    }
}
