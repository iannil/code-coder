//! Trading loop module for automated trading execution.
//!
//! This module provides the core trading loop infrastructure that enables
//! second-level monitoring for real-time stop-loss/take-profit triggers
//! and strategy signal detection.
//!
//! # Architecture
//!
//! ```text
//! TradingSessionManager
//!        |
//!        v
//!   TradingLoop (tokio interval)
//!        |
//!    +---+---+
//!    |       |
//!    v       v
//! PriceMonitor  SignalDetector
//!    |               |
//!    v               v
//! Stop/Take Profit  Entry Signals
//! ```
//!
//! # Usage
//!
//! ```ignore
//! let config = LoopConfig::default();
//! let trading_loop = TradingLoop::new(config, data, strategy, execution);
//! trading_loop.run().await?;
//! ```

mod price_monitor;
mod signal_detector;
mod trading_loop;

pub use price_monitor::{PriceMonitor, PriceCheckResult, StopLossConfig, TakeProfitConfig};
pub use signal_detector::{SignalDetector, SignalFilter, DetectedSignal};
pub use trading_loop::{TradingLoop, LoopConfig, LoopState, LoopEvent};

use serde::{Deserialize, Serialize};

/// Trading mode for the loop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingMode {
    /// Paper trading (simulation)
    Paper,
    /// Live trading (real orders)
    Live,
}

impl Default for TradingMode {
    fn default() -> Self {
        Self::Paper
    }
}

/// A position being monitored by the trading loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredPosition {
    /// Position ID
    pub id: String,
    /// Symbol
    pub symbol: String,
    /// Entry price
    pub entry_price: f64,
    /// Current price
    pub current_price: f64,
    /// Quantity
    pub quantity: f64,
    /// Stop loss price
    pub stop_loss: f64,
    /// Take profit price
    pub take_profit: f64,
    /// Entry timestamp
    pub entry_time: chrono::DateTime<chrono::Utc>,
    /// Whether this is a paper position
    pub is_paper: bool,
}

impl MonitoredPosition {
    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self) -> f64 {
        (self.current_price - self.entry_price) * self.quantity
    }

    /// Calculate return percentage
    pub fn return_pct(&self) -> f64 {
        ((self.current_price - self.entry_price) / self.entry_price) * 100.0
    }

    /// Check if stop loss is triggered
    pub fn is_stop_loss_triggered(&self) -> bool {
        self.current_price <= self.stop_loss
    }

    /// Check if take profit is triggered
    pub fn is_take_profit_triggered(&self) -> bool {
        self.current_price >= self.take_profit
    }

    /// Update current price
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_monitored_position_pnl() {
        let pos = MonitoredPosition {
            id: "test-1".to_string(),
            symbol: "000001.SZ".to_string(),
            entry_price: 10.0,
            current_price: 11.0,
            quantity: 100.0,
            stop_loss: 9.5,
            take_profit: 11.0,
            entry_time: Utc::now(),
            is_paper: true,
        };

        assert!((pos.unrealized_pnl() - 100.0).abs() < 0.01);
        assert!((pos.return_pct() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_stop_loss_trigger() {
        let mut pos = MonitoredPosition {
            id: "test-1".to_string(),
            symbol: "000001.SZ".to_string(),
            entry_price: 10.0,
            current_price: 10.0,
            quantity: 100.0,
            stop_loss: 9.5,
            take_profit: 11.0,
            entry_time: Utc::now(),
            is_paper: true,
        };

        assert!(!pos.is_stop_loss_triggered());

        pos.update_price(9.5);
        assert!(pos.is_stop_loss_triggered());

        pos.update_price(9.0);
        assert!(pos.is_stop_loss_triggered());
    }

    #[test]
    fn test_take_profit_trigger() {
        let mut pos = MonitoredPosition {
            id: "test-1".to_string(),
            symbol: "000001.SZ".to_string(),
            entry_price: 10.0,
            current_price: 10.0,
            quantity: 100.0,
            stop_loss: 9.5,
            take_profit: 11.0,
            entry_time: Utc::now(),
            is_paper: true,
        };

        assert!(!pos.is_take_profit_triggered());

        pos.update_price(11.0);
        assert!(pos.is_take_profit_triggered());

        pos.update_price(12.0);
        assert!(pos.is_take_profit_triggered());
    }

    #[test]
    fn test_trading_mode_default() {
        assert_eq!(TradingMode::default(), TradingMode::Paper);
    }
}
