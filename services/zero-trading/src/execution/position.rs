//! Position tracking for T+1 compliance.

use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// A trading position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Position ID
    pub id: String,
    /// Symbol
    pub symbol: String,
    /// Quantity held
    pub quantity: f64,
    /// Entry price
    pub entry_price: f64,
    /// Current price
    pub current_price: f64,
    /// Stop loss price
    pub stop_loss: f64,
    /// Take profit price
    pub take_profit: f64,
    /// Entry timestamp
    pub entry_time: DateTime<Utc>,
    /// Entry date (local time, for T+1 calculation)
    pub entry_date: NaiveDate,
    /// Status
    pub status: PositionStatus,
    /// Notes
    #[serde(default)]
    pub notes: String,
}

/// Position status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionStatus {
    /// Position is open
    Open,
    /// Position is closed
    Closed,
    /// Position is pending (order not filled)
    Pending,
}

impl Position {
    /// Create a new position
    pub fn new(
        symbol: &str,
        quantity: f64,
        entry_price: f64,
        stop_loss: f64,
        take_profit: f64,
    ) -> Self {
        let now = Utc::now();
        let local_date = Local::now().date_naive();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            quantity,
            entry_price,
            current_price: entry_price,
            stop_loss,
            take_profit,
            entry_time: now,
            entry_date: local_date,
            status: PositionStatus::Open,
            notes: String::new(),
        }
    }

    /// Check if position is open
    pub fn is_open(&self) -> bool {
        self.status == PositionStatus::Open
    }

    /// Check if position can be sold today (T+1 rule)
    ///
    /// A-share T+1: Cannot sell shares bought on the same day
    pub fn can_sell_today(&self) -> bool {
        let today = Local::now().date_naive();
        today > self.entry_date
    }

    /// Get days held
    pub fn days_held(&self) -> i64 {
        let today = Local::now().date_naive();
        (today - self.entry_date).num_days()
    }

    /// Update current price
    pub fn update_price(&mut self, price: f64) {
        self.current_price = price;
    }

    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self) -> f64 {
        (self.current_price - self.entry_price) * self.quantity
    }

    /// Calculate unrealized P&L percentage
    pub fn unrealized_pnl_pct(&self) -> f64 {
        ((self.current_price - self.entry_price) / self.entry_price) * 100.0
    }

    /// Get invested amount
    pub fn invested_amount(&self) -> f64 {
        self.entry_price * self.quantity
    }

    /// Get current value
    pub fn current_value(&self) -> f64 {
        self.current_price * self.quantity
    }

    /// Check if stop loss is hit
    pub fn is_stop_loss_hit(&self) -> bool {
        self.current_price <= self.stop_loss
    }

    /// Check if take profit is hit
    pub fn is_take_profit_hit(&self) -> bool {
        self.current_price >= self.take_profit
    }

    /// Close the position
    pub fn close(&mut self) {
        self.status = PositionStatus::Closed;
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "{} | 数量:{} | 入场:{:.2} | 现价:{:.2} | 盈亏:{:+.2} ({:+.1}%)",
            self.symbol,
            self.quantity,
            self.entry_price,
            self.current_price,
            self.unrealized_pnl(),
            self.unrealized_pnl_pct()
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new("000001.SZ", 100.0, 10.0, 9.5, 11.0);

        assert_eq!(pos.symbol, "000001.SZ");
        assert_eq!(pos.quantity, 100.0);
        assert!(pos.is_open());
    }

    #[test]
    fn test_unrealized_pnl() {
        let mut pos = Position::new("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        pos.update_price(11.0);

        assert!((pos.unrealized_pnl() - 100.0).abs() < 0.01);
        assert!((pos.unrealized_pnl_pct() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_stop_loss_hit() {
        let mut pos = Position::new("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        pos.update_price(9.4);

        assert!(pos.is_stop_loss_hit());
    }

    #[test]
    fn test_take_profit_hit() {
        let mut pos = Position::new("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        pos.update_price(11.5);

        assert!(pos.is_take_profit_hit());
    }

    #[test]
    fn test_t1_sell_rule() {
        let pos = Position::new("000001.SZ", 100.0, 10.0, 9.5, 11.0);

        // Same day - cannot sell
        assert!(!pos.can_sell_today());
    }

    #[test]
    fn test_invested_amount() {
        let pos = Position::new("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        assert!((pos.invested_amount() - 1000.0).abs() < 0.01);
    }
}
