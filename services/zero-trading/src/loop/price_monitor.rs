//! Price monitoring for stop-loss and take-profit triggers.
//!
//! This module provides real-time price monitoring capabilities
//! to detect when positions hit stop-loss or take-profit levels.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::data::MarketDataAggregator;

/// Stop-loss configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossConfig {
    /// Enable trailing stop-loss
    pub trailing_enabled: bool,
    /// Trailing stop-loss percentage
    pub trailing_pct: f64,
    /// Hard stop-loss percentage (fallback)
    pub hard_stop_pct: f64,
}

impl Default for StopLossConfig {
    fn default() -> Self {
        Self {
            trailing_enabled: false,
            trailing_pct: 3.0,
            hard_stop_pct: 5.0,
        }
    }
}

/// Take-profit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeProfitConfig {
    /// Enable partial take-profit
    pub partial_enabled: bool,
    /// First target percentage (take 50% off)
    pub first_target_pct: f64,
    /// Final target percentage (close remaining)
    pub final_target_pct: f64,
}

impl Default for TakeProfitConfig {
    fn default() -> Self {
        Self {
            partial_enabled: false,
            first_target_pct: 3.0,
            final_target_pct: 5.0,
        }
    }
}

/// Result of a price check for a single position
#[derive(Debug, Clone)]
pub struct PriceCheckResult {
    /// Symbol checked
    pub symbol: String,
    /// Current price
    pub current_price: f64,
    /// Entry price (for reference)
    pub entry_price: Option<f64>,
    /// Stop loss level
    pub stop_loss: Option<f64>,
    /// Take profit level
    pub take_profit: Option<f64>,
    /// Whether stop-loss was triggered
    pub stop_loss_triggered: bool,
    /// Whether take-profit was triggered
    pub take_profit_triggered: bool,
    /// Price change percentage from entry
    pub change_pct: Option<f64>,
}

impl PriceCheckResult {
    /// Create a new price check result with only current price
    pub fn new(symbol: String, current_price: f64) -> Self {
        Self {
            symbol,
            current_price,
            entry_price: None,
            stop_loss: None,
            take_profit: None,
            stop_loss_triggered: false,
            take_profit_triggered: false,
            change_pct: None,
        }
    }

    /// Set entry price and calculate change percentage
    pub fn with_entry(mut self, entry_price: f64) -> Self {
        self.entry_price = Some(entry_price);
        self.change_pct = Some(((self.current_price - entry_price) / entry_price) * 100.0);
        self
    }

    /// Set stop-loss level and check trigger
    pub fn with_stop_loss(mut self, stop_loss: f64) -> Self {
        self.stop_loss = Some(stop_loss);
        self.stop_loss_triggered = self.current_price <= stop_loss;
        self
    }

    /// Set take-profit level and check trigger
    pub fn with_take_profit(mut self, take_profit: f64) -> Self {
        self.take_profit = Some(take_profit);
        self.take_profit_triggered = self.current_price >= take_profit;
        self
    }
}

/// Position data for price monitoring
#[derive(Debug, Clone)]
pub struct PositionToMonitor {
    /// Symbol
    pub symbol: String,
    /// Entry price
    pub entry_price: f64,
    /// Stop-loss price
    pub stop_loss: f64,
    /// Take-profit price
    pub take_profit: f64,
    /// Highest price since entry (for trailing stop)
    pub highest_price: f64,
}

/// Price monitor for tracking positions
pub struct PriceMonitor {
    /// Market data source
    data: Arc<MarketDataAggregator>,
    /// Stop-loss configuration
    stop_loss_config: StopLossConfig,
    /// Take-profit configuration
    take_profit_config: TakeProfitConfig,
    /// Position data cache (symbol -> position data)
    position_cache: HashMap<String, PositionToMonitor>,
}

impl PriceMonitor {
    /// Create a new price monitor
    pub fn new(data: Arc<MarketDataAggregator>) -> Self {
        Self {
            data,
            stop_loss_config: StopLossConfig::default(),
            take_profit_config: TakeProfitConfig::default(),
            position_cache: HashMap::new(),
        }
    }

    /// Create with custom configurations
    pub fn with_config(
        data: Arc<MarketDataAggregator>,
        stop_loss_config: StopLossConfig,
        take_profit_config: TakeProfitConfig,
    ) -> Self {
        Self {
            data,
            stop_loss_config,
            take_profit_config,
            position_cache: HashMap::new(),
        }
    }

    /// Register a position for monitoring
    pub fn register_position(&mut self, position: PositionToMonitor) {
        self.position_cache.insert(position.symbol.clone(), position);
    }

    /// Unregister a position
    pub fn unregister_position(&mut self, symbol: &str) {
        self.position_cache.remove(symbol);
    }

    /// Check prices for a list of symbols
    pub async fn check_prices(&self, symbols: &[String]) -> Result<Vec<PriceCheckResult>> {
        let mut results = Vec::new();

        for symbol in symbols {
            match self.check_single_price(symbol).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!(symbol = %symbol, error = %e, "Failed to check price");
                    // Continue with other symbols
                }
            }
        }

        Ok(results)
    }

    /// Check price for a single symbol
    async fn check_single_price(&self, symbol: &str) -> Result<PriceCheckResult> {
        // Get current quote
        let quote = self.data.get_latest_quote(symbol).await?;
        let current_price = quote.close;

        let mut result = PriceCheckResult::new(symbol.to_string(), current_price);

        // If we have cached position data, use it
        if let Some(position) = self.position_cache.get(symbol) {
            result = result
                .with_entry(position.entry_price)
                .with_stop_loss(position.stop_loss)
                .with_take_profit(position.take_profit);

            debug!(
                symbol = %symbol,
                current_price,
                entry_price = position.entry_price,
                stop_loss = position.stop_loss,
                take_profit = position.take_profit,
                stop_triggered = result.stop_loss_triggered,
                tp_triggered = result.take_profit_triggered,
                "Price checked"
            );
        }

        Ok(result)
    }

    /// Calculate trailing stop-loss level
    pub fn calculate_trailing_stop(&self, entry_price: f64, highest_price: f64) -> f64 {
        if !self.stop_loss_config.trailing_enabled {
            // Use hard stop
            return entry_price * (1.0 - self.stop_loss_config.hard_stop_pct / 100.0);
        }

        // Trailing stop based on highest price
        let trailing_stop = highest_price * (1.0 - self.stop_loss_config.trailing_pct / 100.0);

        // Don't let trailing stop go below hard stop
        let hard_stop = entry_price * (1.0 - self.stop_loss_config.hard_stop_pct / 100.0);

        trailing_stop.max(hard_stop)
    }

    /// Calculate partial take-profit levels
    pub fn calculate_take_profit_levels(&self, entry_price: f64) -> (f64, f64) {
        let first_target = entry_price * (1.0 + self.take_profit_config.first_target_pct / 100.0);
        let final_target = entry_price * (1.0 + self.take_profit_config.final_target_pct / 100.0);

        (first_target, final_target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_loss_config_default() {
        let config = StopLossConfig::default();
        assert!(!config.trailing_enabled);
        assert!((config.hard_stop_pct - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_take_profit_config_default() {
        let config = TakeProfitConfig::default();
        assert!(!config.partial_enabled);
        assert!((config.first_target_pct - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_price_check_result() {
        let result = PriceCheckResult::new("000001.SZ".to_string(), 10.0)
            .with_entry(9.5)
            .with_stop_loss(9.0)
            .with_take_profit(11.0);

        assert!(!result.stop_loss_triggered);
        assert!(!result.take_profit_triggered);

        // Change pct should be ~5.26%
        let change = result.change_pct.unwrap();
        assert!(change > 5.0 && change < 6.0);
    }

    #[test]
    fn test_stop_loss_triggered() {
        let result = PriceCheckResult::new("000001.SZ".to_string(), 9.0)
            .with_entry(10.0)
            .with_stop_loss(9.5);

        assert!(result.stop_loss_triggered);
    }

    #[test]
    fn test_take_profit_triggered() {
        let result = PriceCheckResult::new("000001.SZ".to_string(), 11.5)
            .with_entry(10.0)
            .with_take_profit(11.0);

        assert!(result.take_profit_triggered);
    }

    #[test]
    fn test_trailing_stop_calculation() {
        let config = zero_common::config::Config::default();
        let data = Arc::new(MarketDataAggregator::new(&config));
        let mut monitor = PriceMonitor::new(data);

        // Without trailing enabled
        let stop = monitor.calculate_trailing_stop(100.0, 110.0);
        // Should use hard stop (5%)
        assert!((stop - 95.0).abs() < 0.01);

        // With trailing enabled
        monitor.stop_loss_config.trailing_enabled = true;
        monitor.stop_loss_config.trailing_pct = 3.0;
        let stop = monitor.calculate_trailing_stop(100.0, 110.0);
        // Trailing stop: 110 * 0.97 = 106.7
        assert!((stop - 106.7).abs() < 0.01);
    }

    #[test]
    fn test_take_profit_levels() {
        let config = zero_common::config::Config::default();
        let data = Arc::new(MarketDataAggregator::new(&config));
        let monitor = PriceMonitor::new(data);

        let (first, final_tp) = monitor.calculate_take_profit_levels(100.0);

        // First target: 103.0 (3%)
        assert!((first - 103.0).abs() < 0.01);
        // Final target: 105.0 (5%)
        assert!((final_tp - 105.0).abs() < 0.01);
    }
}
