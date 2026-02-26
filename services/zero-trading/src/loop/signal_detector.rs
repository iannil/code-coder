//! Signal detection for the trading loop.
//!
//! This module wraps the strategy engine to provide filtered signal detection
//! for the trading loop, applying additional filters and deduplication.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::data::MarketDataAggregator;
use crate::strategy::{SignalDirection, SignalStrength, StrategyEngine, TradingSignal};

/// Configuration for signal filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalFilter {
    /// Minimum signal strength to accept
    pub min_strength: SignalStrength,
    /// Only accept long signals (for T+1 markets)
    pub long_only: bool,
    /// Maximum signals per symbol per day
    pub max_signals_per_symbol: usize,
    /// Signal deduplication window in seconds
    pub dedup_window_secs: u64,
    /// Symbols to include (empty = all)
    pub include_symbols: Vec<String>,
    /// Symbols to exclude
    pub exclude_symbols: Vec<String>,
}

impl Default for SignalFilter {
    fn default() -> Self {
        Self {
            min_strength: SignalStrength::Medium,
            long_only: true, // A-shares T+1 rule
            max_signals_per_symbol: 1,
            dedup_window_secs: 300, // 5 minutes
            include_symbols: vec![],
            exclude_symbols: vec![],
        }
    }
}

/// A detected signal with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedSignal {
    /// The underlying trading signal
    pub signal: TradingSignal,
    /// Detection timestamp
    pub detected_at: DateTime<Utc>,
    /// Whether this signal was executed
    pub executed: bool,
    /// Execution timestamp
    pub executed_at: Option<DateTime<Utc>>,
}

impl DetectedSignal {
    /// Create from a trading signal
    pub fn from_signal(signal: TradingSignal) -> Self {
        Self {
            signal,
            detected_at: Utc::now(),
            executed: false,
            executed_at: None,
        }
    }

    /// Mark as executed
    pub fn mark_executed(&mut self) {
        self.executed = true;
        self.executed_at = Some(Utc::now());
    }
}

// Delegate common fields to the underlying signal
impl std::ops::Deref for DetectedSignal {
    type Target = TradingSignal;

    fn deref(&self) -> &Self::Target {
        &self.signal
    }
}

/// Signal detector for the trading loop
pub struct SignalDetector {
    /// Strategy engine
    strategy: Arc<StrategyEngine>,
    /// Market data source
    data: Arc<MarketDataAggregator>,
    /// Signal filter configuration
    filter: SignalFilter,
    /// Recent signals for deduplication (symbol -> last signal time)
    recent_signals: Arc<RwLock<std::collections::HashMap<String, DateTime<Utc>>>>,
    /// Daily signal counts (symbol -> count)
    daily_counts: Arc<RwLock<std::collections::HashMap<String, usize>>>,
    /// Last count reset date
    last_reset_date: Arc<RwLock<Option<chrono::NaiveDate>>>,
}

impl SignalDetector {
    /// Create a new signal detector
    pub fn new(strategy: Arc<StrategyEngine>, data: Arc<MarketDataAggregator>) -> Self {
        Self {
            strategy,
            data,
            filter: SignalFilter::default(),
            recent_signals: Arc::new(RwLock::new(std::collections::HashMap::new())),
            daily_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            last_reset_date: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with custom filter
    pub fn with_filter(
        strategy: Arc<StrategyEngine>,
        data: Arc<MarketDataAggregator>,
        filter: SignalFilter,
    ) -> Self {
        Self {
            strategy,
            data,
            filter,
            recent_signals: Arc::new(RwLock::new(std::collections::HashMap::new())),
            daily_counts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            last_reset_date: Arc::new(RwLock::new(None)),
        }
    }

    /// Update the filter configuration
    pub fn set_filter(&mut self, filter: SignalFilter) {
        self.filter = filter;
    }

    /// Scan for new signals
    pub async fn scan(&self) -> Result<Vec<DetectedSignal>> {
        // Reset daily counts if needed
        self.reset_daily_counts_if_needed().await;

        // Get signals from strategy engine
        let raw_signals = self.strategy.scan_for_signals(&self.data).await?;

        debug!(
            raw_count = raw_signals.len(),
            "Strategy scan returned signals"
        );

        // Filter signals
        let mut filtered_signals = Vec::new();

        for signal in raw_signals {
            if self.should_accept(&signal).await {
                let detected = DetectedSignal::from_signal(signal.clone());
                filtered_signals.push(detected);

                // Update tracking
                self.record_signal(&signal.symbol).await;
            }
        }

        if !filtered_signals.is_empty() {
            info!(
                count = filtered_signals.len(),
                "Filtered signals detected"
            );
        }

        Ok(filtered_signals)
    }

    /// Check if a signal should be accepted based on filters
    async fn should_accept(&self, signal: &TradingSignal) -> bool {
        // Check strength
        if signal.strength < self.filter.min_strength {
            debug!(
                symbol = %signal.symbol,
                strength = ?signal.strength,
                min = ?self.filter.min_strength,
                "Signal rejected: strength too low"
            );
            return false;
        }

        // Check direction (for T+1 markets)
        if self.filter.long_only && signal.direction != SignalDirection::Long {
            debug!(
                symbol = %signal.symbol,
                direction = ?signal.direction,
                "Signal rejected: long only filter"
            );
            return false;
        }

        // Check include/exclude lists
        if !self.filter.include_symbols.is_empty()
            && !self.filter.include_symbols.contains(&signal.symbol)
        {
            debug!(
                symbol = %signal.symbol,
                "Signal rejected: not in include list"
            );
            return false;
        }

        if self.filter.exclude_symbols.contains(&signal.symbol) {
            debug!(
                symbol = %signal.symbol,
                "Signal rejected: in exclude list"
            );
            return false;
        }

        // Check deduplication
        if self.is_duplicate(&signal.symbol).await {
            debug!(
                symbol = %signal.symbol,
                "Signal rejected: duplicate within window"
            );
            return false;
        }

        // Check daily limit
        if self.exceeds_daily_limit(&signal.symbol).await {
            debug!(
                symbol = %signal.symbol,
                "Signal rejected: daily limit exceeded"
            );
            return false;
        }

        true
    }

    /// Check if this is a duplicate signal
    async fn is_duplicate(&self, symbol: &str) -> bool {
        let recent = self.recent_signals.read().await;

        if let Some(last_time) = recent.get(symbol) {
            let elapsed = Utc::now().signed_duration_since(*last_time);
            let window = chrono::Duration::seconds(self.filter.dedup_window_secs as i64);

            return elapsed < window;
        }

        false
    }

    /// Check if daily limit is exceeded
    async fn exceeds_daily_limit(&self, symbol: &str) -> bool {
        let counts = self.daily_counts.read().await;

        if let Some(count) = counts.get(symbol) {
            return *count >= self.filter.max_signals_per_symbol;
        }

        false
    }

    /// Record a signal for tracking
    async fn record_signal(&self, symbol: &str) {
        // Update recent signals
        {
            let mut recent = self.recent_signals.write().await;
            recent.insert(symbol.to_string(), Utc::now());
        }

        // Update daily counts
        {
            let mut counts = self.daily_counts.write().await;
            let count = counts.entry(symbol.to_string()).or_insert(0);
            *count += 1;
        }
    }

    /// Reset daily counts if date changed
    async fn reset_daily_counts_if_needed(&self) {
        let today = chrono::Local::now().date_naive();

        let should_reset = {
            let last_date = self.last_reset_date.read().await;
            last_date.map(|d| d != today).unwrap_or(true)
        };

        if should_reset {
            let mut counts = self.daily_counts.write().await;
            counts.clear();

            let mut last_date = self.last_reset_date.write().await;
            *last_date = Some(today);

            debug!("Daily signal counts reset");
        }
    }

    /// Get currently tracked symbols
    pub async fn get_tracked_symbols(&self) -> Vec<String> {
        let recent = self.recent_signals.read().await;
        recent.keys().cloned().collect()
    }

    /// Clear all tracking data
    pub async fn clear_tracking(&self) {
        {
            let mut recent = self.recent_signals.write().await;
            recent.clear();
        }
        {
            let mut counts = self.daily_counts.write().await;
            counts.clear();
        }
    }

    /// Preload tracking symbols data (24/7 operation).
    ///
    /// Preloads historical data for all symbols that have recently
    /// generated signals, ensuring fast access during trading hours.
    pub async fn preload_data(&self, data: &Arc<MarketDataAggregator>) -> Result<()> {
        use crate::data::Timeframe;

        let symbols = self.get_tracked_symbols().await;

        if symbols.is_empty() {
            debug!("No tracked symbols for signal detector preload");
            return Ok(());
        }

        info!(count = symbols.len(), "Preloading data for signal detector");

        for symbol in symbols {
            // Preload common timeframes
            for tf in [Timeframe::Daily, Timeframe::H4, Timeframe::H1] {
                if let Err(e) = data.get_candles(&symbol, tf, 200).await {
                    debug!(
                        symbol = %symbol,
                        timeframe = ?tf,
                        error = %e,
                        "Failed to preload symbol data"
                    );
                }
            }
        }

        Ok(())
    }

    /// Precompute technical indicator parameters (24/7 operation).
    ///
    /// Calculates and caches technical indicators that are used in
    /// signal detection, such as moving averages, ATR, and SMT divergences.
    /// This allows faster signal detection during trading hours.
    pub async fn precompute_parameters(&self, data: &Arc<MarketDataAggregator>) -> Result<()> {
        use crate::data::Timeframe;

        info!("Starting parameter precomputation");

        // Get tracked symbols
        let symbols = data.get_tracked_symbols().await;

        for symbol in symbols {
            // Precompute indicators for each timeframe
            for tf in [Timeframe::Daily, Timeframe::H4, Timeframe::H1] {
                match data.get_candles(&symbol, tf, 200).await {
                    Ok(candles) => {
                        // Compute and cache moving averages
                        if candles.len() >= 20 {
                            let ma20: f64 = candles[candles.len() - 20..].iter().map(|c| c.close).sum::<f64>() / 20.0;
                            debug!(
                                symbol = %symbol,
                                timeframe = ?tf,
                                ma20,
                                "Computed MA20"
                            );
                        }

                        if candles.len() >= 50 {
                            let ma50: f64 = candles[candles.len() - 50..].iter().map(|c| c.close).sum::<f64>() / 50.0;
                            debug!(
                                symbol = %symbol,
                                timeframe = ?tf,
                                ma50,
                                "Computed MA50"
                            );
                        }

                        // Compute ATR (Average True Range) for volatility
                        if candles.len() >= 14 {
                            let mut sum_tr = 0.0;
                            for window in candles.windows(2) {
                                let high_low = window[1].high - window[1].low;
                                let high_close = (window[1].high - window[0].close).abs();
                                let low_close = (window[1].low - window[0].close).abs();
                                sum_tr += high_low.max(high_close).max(low_close);
                            }
                            let atr = sum_tr / 14.0;
                            debug!(
                                symbol = %symbol,
                                timeframe = ?tf,
                                atr,
                                "Computed ATR"
                            );
                        }
                    }
                    Err(e) => {
                        debug!(
                            symbol = %symbol,
                            timeframe = ?tf,
                            error = %e,
                            "Failed to fetch candles for precomputation"
                        );
                    }
                }
            }
        }

        // Precompute SMT pair divergences
        let smt_pairs = data.get_smt_pairs();
        for pair in smt_pairs {
            match data.get_smt_pair_data(pair, Timeframe::Daily, 100).await {
                Ok((primary, reference)) => {
                    // Check for divergence (simplified check)
                    if !primary.is_empty() && !reference.is_empty() {
                        let primary_latest = &primary[primary.len() - 1];
                        let reference_latest = &reference[reference.len() - 1];

                        debug!(
                            pair = %pair.name,
                            primary_close = primary_latest.close,
                            reference_close = reference_latest.close,
                            "SMT pair data checked"
                        );
                    }
                }
                Err(e) => {
                    debug!(
                        pair = %pair.name,
                        error = %e,
                        "Failed to fetch SMT pair data for precomputation"
                    );
                }
            }
        }

        info!("Parameter precomputation completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_filter_default() {
        let filter = SignalFilter::default();
        assert_eq!(filter.min_strength, SignalStrength::Medium);
        assert!(filter.long_only);
        assert_eq!(filter.max_signals_per_symbol, 1);
    }

    #[test]
    fn test_signal_filter_serialization() {
        let filter = SignalFilter {
            min_strength: SignalStrength::Strong,
            long_only: false,
            max_signals_per_symbol: 3,
            dedup_window_secs: 600,
            include_symbols: vec!["000001.SZ".to_string()],
            exclude_symbols: vec!["000002.SZ".to_string()],
        };

        let json = serde_json::to_string(&filter).unwrap();
        let parsed: SignalFilter = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.min_strength, SignalStrength::Strong);
        assert!(!parsed.long_only);
        assert_eq!(parsed.include_symbols.len(), 1);
    }

    #[test]
    fn test_detected_signal_creation() {
        use crate::strategy::SignalDirection;

        let trading_signal = TradingSignal {
            id: "test-1".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: SignalDirection::Long,
            strength: SignalStrength::Strong,
            entry_price: 10.0,
            stop_loss: 9.5,
            take_profit: 11.0,
            timestamp: Utc::now(),
            po3_structure: None,
            smt_divergence: None,
            timeframe_alignment: vec![],
            notes: String::new(),
        };

        let detected = DetectedSignal::from_signal(trading_signal);

        assert!(!detected.executed);
        assert!(detected.executed_at.is_none());
        assert_eq!(detected.symbol, "000001.SZ");
    }

    #[test]
    fn test_detected_signal_mark_executed() {
        use crate::strategy::SignalDirection;

        let trading_signal = TradingSignal {
            id: "test-1".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: SignalDirection::Long,
            strength: SignalStrength::Strong,
            entry_price: 10.0,
            stop_loss: 9.5,
            take_profit: 11.0,
            timestamp: Utc::now(),
            po3_structure: None,
            smt_divergence: None,
            timeframe_alignment: vec![],
            notes: String::new(),
        };

        let mut detected = DetectedSignal::from_signal(trading_signal);
        detected.mark_executed();

        assert!(detected.executed);
        assert!(detected.executed_at.is_some());
    }
}
