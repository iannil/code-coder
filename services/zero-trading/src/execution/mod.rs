//! Execution engine for A-shares with T+1 compliance.
//!
//! Key T+1 rules:
//! - Cannot sell shares bought on the same day
//! - Must hold positions overnight
//! - Next-day decision based on opening auction

mod position;
mod order;
mod t1_risk;
mod executor;

pub use position::Position;
pub use order::{Order, OrderStatus, OrderType, OrderSide};
pub use t1_risk::{T1RiskManager, T1Decision};
pub use executor::{
    TradingExecutor, PaperExecutor, AccountInfo,
    ExecutionRequest, ExecutionResult, ExecutionSide, ExecutionStatus,
    create_executor,
};

use anyhow::Result;
use chrono::{Local, NaiveDate, NaiveTime};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use zero_common::config::Config;

use crate::strategy::TradingSignal;
use crate::data::AuctionData;

/// Execution engine configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Maximum number of open positions
    pub max_positions: usize,
    /// Maximum capital per position (percentage)
    pub max_position_pct: f64,
    /// Maximum daily capital deployment
    pub max_daily_capital_pct: f64,
    /// Default stop loss percentage
    pub default_stop_loss_pct: f64,
    /// Enable automatic execution
    pub auto_execute: bool,
    /// Enable paper trading (simulation)
    pub paper_trading: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_positions: 5,
            max_position_pct: 20.0,
            max_daily_capital_pct: 50.0,
            default_stop_loss_pct: 5.0,
            auto_execute: false,
            paper_trading: true,
        }
    }
}

/// Main execution engine
pub struct ExecutionEngine {
    /// Configuration
    config: ExecutionConfig,
    /// Current positions
    positions: HashMap<String, Position>,
    /// Pending orders
    orders: HashMap<String, Order>,
    /// T+1 risk manager
    risk_manager: T1RiskManager,
    /// Account capital
    total_capital: f64,
    /// Available capital
    available_capital: f64,
    /// Capital used today
    daily_used_capital: f64,
    /// Today's date (for tracking daily limits)
    current_date: NaiveDate,
    /// Whether connected to broker
    connected: AtomicBool,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(config: &Config) -> Self {
        let exec_config = config
            .trading
            .as_ref()
            .map(|t| ExecutionConfig {
                max_positions: t.max_positions.unwrap_or(5),
                max_position_pct: t.max_position_pct.unwrap_or(20.0),
                max_daily_capital_pct: t.max_daily_capital_pct.unwrap_or(50.0),
                default_stop_loss_pct: t.default_stop_loss_pct.unwrap_or(5.0),
                auto_execute: t.auto_execute.unwrap_or(false),
                paper_trading: t.paper_trading.unwrap_or(true),
            })
            .unwrap_or_default();

        let risk_config = t1_risk::T1RiskConfig {
            stop_loss_pct: exec_config.default_stop_loss_pct,
            take_profit_pct: exec_config.default_stop_loss_pct * 2.0, // 2:1 R:R default
            max_loss_per_trade_pct: 2.0,
        };

        Self {
            config: exec_config,
            positions: HashMap::new(),
            orders: HashMap::new(),
            risk_manager: T1RiskManager::new(risk_config),
            total_capital: 100000.0, // Default 100k
            available_capital: 100000.0,
            daily_used_capital: 0.0,
            current_date: Local::now().date_naive(),
            connected: AtomicBool::new(false),
        }
    }

    /// Check if connected to broker
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Set account capital
    pub fn set_capital(&mut self, capital: f64) {
        self.total_capital = capital;
        self.available_capital = capital - self.get_invested_capital();
    }

    /// Get invested capital
    fn get_invested_capital(&self) -> f64 {
        self.positions.values().map(|p| p.invested_amount()).sum()
    }

    /// Reset daily limits (call at market open)
    pub fn reset_daily_limits(&mut self) {
        let today = Local::now().date_naive();
        if today != self.current_date {
            self.daily_used_capital = 0.0;
            self.current_date = today;
        }
    }

    /// Check if we can open a new position
    pub fn can_open_position(&self, symbol: &str, amount: f64) -> bool {
        // Check if already have position
        if self.positions.contains_key(symbol) {
            return false;
        }

        // Check position count limit
        if self.positions.len() >= self.config.max_positions {
            return false;
        }

        // Check position size limit
        let position_limit = self.total_capital * (self.config.max_position_pct / 100.0);
        if amount > position_limit {
            return false;
        }

        // Check daily capital limit
        let daily_limit = self.total_capital * (self.config.max_daily_capital_pct / 100.0);
        if self.daily_used_capital + amount > daily_limit {
            return false;
        }

        // Check available capital
        if amount > self.available_capital {
            return false;
        }

        true
    }

    /// Execute a buy order based on signal
    pub async fn execute_buy(&mut self, signal: &TradingSignal) -> Result<Order> {
        // Calculate position size
        let risk_per_trade = self.total_capital * 0.02; // 2% risk per trade
        let risk_amount = (signal.entry_price - signal.stop_loss).abs();
        let quantity = (risk_per_trade / risk_amount).floor();
        let amount = quantity * signal.entry_price;

        // Validate
        if !self.can_open_position(&signal.symbol, amount) {
            anyhow::bail!("Cannot open position: limits exceeded");
        }

        // Create order
        let order = Order::new_buy(
            &signal.symbol,
            quantity,
            signal.entry_price,
            signal.stop_loss,
            signal.take_profit,
        );

        // In paper trading mode, immediately fill
        if self.config.paper_trading {
            return self.paper_fill_buy(&signal.symbol, quantity, signal.entry_price, signal.stop_loss, signal.take_profit).await;
        }

        // In real mode, send to broker
        self.send_order_to_broker(&order).await?;
        self.orders.insert(order.id.clone(), order.clone());

        Ok(order)
    }

    /// Paper trading: immediately fill buy order
    async fn paper_fill_buy(
        &mut self,
        symbol: &str,
        quantity: f64,
        price: f64,
        stop_loss: f64,
        take_profit: f64,
    ) -> Result<Order> {
        let order = Order::new_buy(symbol, quantity, price, stop_loss, take_profit)
            .with_status(OrderStatus::Filled);

        // Create position
        let position = Position::new(
            symbol,
            quantity,
            price,
            stop_loss,
            take_profit,
        );

        // Update capital tracking
        let amount = quantity * price;
        self.available_capital -= amount;
        self.daily_used_capital += amount;
        self.positions.insert(symbol.to_string(), position);

        tracing::info!(
            symbol,
            quantity,
            price,
            stop_loss,
            take_profit,
            "[PAPER] Position opened"
        );

        Ok(order)
    }

    /// Evaluate next-day decision for T+1 positions
    pub async fn evaluate_next_day_decisions(
        &mut self,
        auction_data: &HashMap<String, AuctionData>,
    ) -> Vec<(String, T1Decision)> {
        let mut decisions = Vec::new();

        for (symbol, position) in &self.positions {
            if !position.can_sell_today() {
                continue;
            }

            if let Some(auction) = auction_data.get(symbol) {
                let decision = self.risk_manager.next_day_decision(position, auction);
                decisions.push((symbol.clone(), decision));
            }
        }

        decisions
    }

    /// Execute sell order
    pub async fn execute_sell(&mut self, symbol: &str, reason: &str) -> Result<Order> {
        let position = self.positions.get(symbol)
            .ok_or_else(|| anyhow::anyhow!("No position for {}", symbol))?;

        if !position.can_sell_today() {
            anyhow::bail!("Cannot sell {} today (T+1 rule)", symbol);
        }

        let order = Order::new_sell(
            symbol,
            position.quantity,
            position.current_price,
            reason,
        );

        if self.config.paper_trading {
            return self.paper_fill_sell(symbol).await;
        }

        self.send_order_to_broker(&order).await?;
        self.orders.insert(order.id.clone(), order.clone());

        Ok(order)
    }

    /// Paper trading: immediately fill sell order
    async fn paper_fill_sell(&mut self, symbol: &str) -> Result<Order> {
        let position = self.positions.remove(symbol)
            .ok_or_else(|| anyhow::anyhow!("No position to sell"))?;

        let order = Order::new_sell(
            symbol,
            position.quantity,
            position.current_price,
            "paper_close",
        ).with_status(OrderStatus::Filled);

        // Update capital
        let amount = position.quantity * position.current_price;
        self.available_capital += amount;

        let pnl = position.unrealized_pnl();
        tracing::info!(
            symbol,
            quantity = position.quantity,
            price = position.current_price,
            pnl,
            "[PAPER] Position closed"
        );

        Ok(order)
    }

    /// Update position prices
    pub fn update_prices(&mut self, prices: &HashMap<String, f64>) {
        for (symbol, position) in &mut self.positions {
            if let Some(price) = prices.get(symbol) {
                position.update_price(*price);
            }
        }
    }

    /// Get all positions
    pub fn get_positions(&self) -> Vec<Position> {
        self.positions.values().cloned().collect()
    }

    /// Get all open orders
    pub fn get_orders(&self) -> Vec<Order> {
        self.orders.values().cloned().collect()
    }

    /// Placeholder: send order to broker API
    async fn send_order_to_broker(&self, order: &Order) -> Result<()> {
        // In paper trading mode, this is a no-op (orders are filled immediately)
        if self.config.paper_trading {
            return Ok(());
        }

        // For real trading, this would call the broker API
        // The broker is managed externally to avoid ownership issues
        tracing::info!(
            order_id = %order.id,
            symbol = %order.symbol,
            quantity = order.quantity,
            price = order.price,
            "Order sent to broker queue"
        );
        Ok(())
    }
}

/// Check if current time is within A-share trading hours
pub fn is_trading_hours() -> bool {
    let now = Local::now();
    let time = now.time();

    let morning_start = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
    let morning_end = NaiveTime::from_hms_opt(11, 30, 0).unwrap();
    let afternoon_start = NaiveTime::from_hms_opt(13, 0, 0).unwrap();
    let afternoon_end = NaiveTime::from_hms_opt(15, 0, 0).unwrap();

    (time >= morning_start && time <= morning_end) ||
    (time >= afternoon_start && time <= afternoon_end)
}

/// Check if current time is auction period (9:15-9:25)
pub fn is_auction_period() -> bool {
    let now = Local::now();
    let time = now.time();

    let auction_start = NaiveTime::from_hms_opt(9, 15, 0).unwrap();
    let auction_end = NaiveTime::from_hms_opt(9, 25, 0).unwrap();

    time >= auction_start && time < auction_end
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();
        assert_eq!(config.max_positions, 5);
        assert!((config.max_position_pct - 20.0).abs() < 0.01);
        assert!(config.paper_trading);
    }

    #[test]
    fn test_can_open_position() {
        let config = Config::default();
        let engine = ExecutionEngine::new(&config);

        // Should be able to open with reasonable amount
        assert!(engine.can_open_position("000001.SZ", 10000.0));

        // Should not exceed limits
        assert!(!engine.can_open_position("000001.SZ", 200000.0));
    }

    #[test]
    fn test_trading_hours() {
        // This test is time-dependent, just ensure it runs
        let _ = is_trading_hours();
        let _ = is_auction_period();
    }
}
