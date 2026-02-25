//! Paper trading verification module.
//!
//! This module provides a complete paper trading simulation system
//! to validate signals before enabling real trading mode.
//!
//! # Usage
//!
//! ```ignore
//! let runner = PaperTradingRunner::new(&config);
//! let result = runner.run_session(Duration::from_hours(4)).await?;
//! println!("Verification result: {:?}", result.summary);
//! ```

mod runner;
mod validator;
mod report;

pub use runner::{PaperTradingRunner, PaperTradingConfig, SessionResult};
pub use validator::{SignalValidator, ValidationResult, ValidationMetrics};
pub use report::{PaperTradingReport, SessionSummary};

use serde::{Deserialize, Serialize};

/// Paper trading session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session not started
    Idle,
    /// Session running
    Running,
    /// Session paused
    Paused,
    /// Session completed
    Completed,
    /// Session failed
    Failed,
}

/// A paper trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperTrade {
    /// Trade ID
    pub id: String,
    /// Symbol traded
    pub symbol: String,
    /// Trade direction
    pub direction: TradeDirection,
    /// Entry price
    pub entry_price: f64,
    /// Exit price (if closed)
    pub exit_price: Option<f64>,
    /// Quantity
    pub quantity: f64,
    /// Entry timestamp
    pub entry_time: chrono::DateTime<chrono::Utc>,
    /// Exit timestamp (if closed)
    pub exit_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Signal that triggered this trade
    pub signal_id: String,
    /// Trade status
    pub status: PaperTradeStatus,
    /// Realized P&L (if closed)
    pub realized_pnl: Option<f64>,
}

/// Trade direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeDirection {
    /// Long position
    Long,
    /// Short (or exit for T+1)
    Short,
}

/// Paper trade status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperTradeStatus {
    /// Trade is open
    Open,
    /// Trade closed with profit
    ClosedProfit,
    /// Trade closed with loss
    ClosedLoss,
    /// Trade cancelled
    Cancelled,
}

impl PaperTrade {
    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        let multiplier = match self.direction {
            TradeDirection::Long => 1.0,
            TradeDirection::Short => -1.0,
        };
        (current_price - self.entry_price) * self.quantity * multiplier
    }

    /// Check if trade is profitable
    pub fn is_profitable(&self, current_price: f64) -> bool {
        self.unrealized_pnl(current_price) > 0.0
    }

    /// Calculate return percentage
    pub fn return_pct(&self, current_price: f64) -> f64 {
        ((current_price - self.entry_price) / self.entry_price) * 100.0
    }

    /// Close the trade
    pub fn close(&mut self, exit_price: f64) {
        self.exit_price = Some(exit_price);
        self.exit_time = Some(chrono::Utc::now());
        self.realized_pnl = Some(self.unrealized_pnl(exit_price));
        self.status = if self.realized_pnl.unwrap_or(0.0) > 0.0 {
            PaperTradeStatus::ClosedProfit
        } else {
            PaperTradeStatus::ClosedLoss
        };
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_trade_pnl() {
        let trade = PaperTrade {
            id: "test".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: TradeDirection::Long,
            entry_price: 10.0,
            exit_price: None,
            quantity: 100.0,
            entry_time: chrono::Utc::now(),
            exit_time: None,
            signal_id: "sig-1".to_string(),
            status: PaperTradeStatus::Open,
            realized_pnl: None,
        };

        // Price up 10% -> P&L = 100.0
        assert!((trade.unrealized_pnl(11.0) - 100.0).abs() < 0.01);
        assert!(trade.is_profitable(11.0));
        assert!((trade.return_pct(11.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_paper_trade_close() {
        let mut trade = PaperTrade {
            id: "test".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: TradeDirection::Long,
            entry_price: 10.0,
            exit_price: None,
            quantity: 100.0,
            entry_time: chrono::Utc::now(),
            exit_time: None,
            signal_id: "sig-1".to_string(),
            status: PaperTradeStatus::Open,
            realized_pnl: None,
        };

        trade.close(11.0);
        assert_eq!(trade.status, PaperTradeStatus::ClosedProfit);
        assert!(trade.realized_pnl.is_some());
        assert!((trade.realized_pnl.unwrap() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_session_state() {
        let state = SessionState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("Running"));
    }
}
