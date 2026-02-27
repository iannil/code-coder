//! Trading executor abstraction for unified order execution.
//!
//! This module provides a unified interface for executing trades
//! in paper trading (simulation) mode.
//!
//! # Architecture
//!
//! ```text
//! TradingExecutor (trait)
//!        |
//!        v
//! PaperExecutor
//! (simulation)
//! ```
//!
//! Note: Live trading via broker has been removed. The system now operates
//! as a signal generator with IM notifications for manual execution.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::{ExecutionEngine, OrderStatus, Position};
use crate::strategy::TradingSignal;

/// Account information for trading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account ID
    pub account_id: String,
    /// Total assets
    pub total_assets: f64,
    /// Available cash
    pub cash: f64,
    /// Market value of positions
    pub market_value: f64,
    /// Unrealized P&L
    pub unrealized_pnl: f64,
    /// Realized P&L today
    pub realized_pnl_today: f64,
    /// Buying power
    pub buying_power: f64,
}

/// Order execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Order side (buy or sell)
    pub side: ExecutionSide,
    /// Symbol
    pub symbol: String,
    /// Quantity
    pub quantity: f64,
    /// Price (None for market orders)
    pub price: Option<f64>,
    /// Stop loss price
    pub stop_loss: Option<f64>,
    /// Take profit price
    pub take_profit: Option<f64>,
    /// Order reason/note
    pub reason: Option<String>,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionSide {
    Buy,
    Sell,
}

/// Result of an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Order ID
    pub order_id: String,
    /// Broker order ID (if live trading)
    pub broker_order_id: Option<String>,
    /// Order status
    pub status: ExecutionStatus,
    /// Fill price (if filled)
    pub fill_price: Option<f64>,
    /// Fill quantity
    pub fill_quantity: Option<f64>,
    /// Execution timestamp
    pub timestamp: DateTime<Utc>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    /// Order submitted
    Pending,
    /// Order accepted by broker
    Accepted,
    /// Order partially filled
    PartiallyFilled,
    /// Order fully filled
    Filled,
    /// Order rejected
    Rejected,
    /// Order cancelled
    Cancelled,
    /// Order failed
    Failed,
}

impl From<OrderStatus> for ExecutionStatus {
    fn from(status: OrderStatus) -> Self {
        match status {
            OrderStatus::Pending => Self::Pending,
            OrderStatus::Submitted => Self::Accepted,
            OrderStatus::PartiallyFilled => Self::PartiallyFilled,
            OrderStatus::Filled => Self::Filled,
            OrderStatus::Cancelled => Self::Cancelled,
            OrderStatus::Rejected => Self::Rejected,
            OrderStatus::Expired => Self::Cancelled, // Treat expired as cancelled
        }
    }
}

/// Unified trading executor interface
#[async_trait]
pub trait TradingExecutor: Send + Sync {
    /// Get executor name
    fn name(&self) -> &'static str;

    /// Check if connected/ready
    fn is_ready(&self) -> bool;

    /// Execute an order
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult>;

    /// Cancel an order
    async fn cancel(&self, order_id: &str) -> Result<()>;

    /// Get current positions
    async fn get_positions(&self) -> Result<Vec<Position>>;

    /// Get account information
    async fn get_account(&self) -> Result<AccountInfo>;

    /// Execute a buy from a trading signal
    async fn execute_buy_signal(&self, signal: &TradingSignal) -> Result<ExecutionResult> {
        let request = ExecutionRequest {
            side: ExecutionSide::Buy,
            symbol: signal.symbol.clone(),
            quantity: self.calculate_position_size(signal).await?,
            price: Some(signal.entry_price),
            stop_loss: Some(signal.stop_loss),
            take_profit: Some(signal.take_profit),
            reason: Some(format!("Signal: {}", signal.id)),
        };

        self.execute(request).await
    }

    /// Execute a sell for a position
    async fn execute_sell_position(
        &self,
        symbol: &str,
        quantity: f64,
        reason: &str,
    ) -> Result<ExecutionResult> {
        let request = ExecutionRequest {
            side: ExecutionSide::Sell,
            symbol: symbol.to_string(),
            quantity,
            price: None, // Market sell
            stop_loss: None,
            take_profit: None,
            reason: Some(reason.to_string()),
        };

        self.execute(request).await
    }

    /// Calculate position size for a signal (default: 2% risk)
    async fn calculate_position_size(&self, signal: &TradingSignal) -> Result<f64> {
        let account = self.get_account().await?;
        let risk_per_trade = account.cash * 0.02; // 2% risk
        let risk_amount = (signal.entry_price - signal.stop_loss).abs();

        let quantity = (risk_per_trade / risk_amount).floor();
        Ok(quantity.max(100.0)) // Minimum 100 shares (A-share lot size)
    }
}

// ============================================================================
// Paper Executor
// ============================================================================

/// Daily P&L tracking state
struct DailyPnlState {
    /// Realized P&L for today
    realized_pnl: f64,
    /// Date when P&L was last reset
    last_reset_date: chrono::NaiveDate,
}

impl DailyPnlState {
    fn new() -> Self {
        Self {
            realized_pnl: 0.0,
            last_reset_date: chrono::Local::now().date_naive(),
        }
    }

    /// Check if we need to reset for a new day
    fn maybe_reset(&mut self) {
        let today = chrono::Local::now().date_naive();
        if today != self.last_reset_date {
            self.realized_pnl = 0.0;
            self.last_reset_date = today;
        }
    }

    /// Add realized P&L (call when a position is closed)
    fn add_realized_pnl(&mut self, pnl: f64) {
        self.maybe_reset();
        self.realized_pnl += pnl;
    }

    /// Get today's realized P&L (resets if new day)
    fn get_realized_pnl(&mut self) -> f64 {
        self.maybe_reset();
        self.realized_pnl
    }
}

/// Paper trading executor (simulation)
pub struct PaperExecutor {
    /// Execution engine
    engine: Arc<RwLock<ExecutionEngine>>,
    /// Virtual account balance
    balance: Arc<RwLock<f64>>,
    /// Order counter
    order_counter: Arc<RwLock<u64>>,
    /// Daily P&L tracking
    daily_pnl: Arc<RwLock<DailyPnlState>>,
}

impl PaperExecutor {
    /// Create a new paper executor
    pub fn new(engine: Arc<RwLock<ExecutionEngine>>, initial_balance: f64) -> Self {
        Self {
            engine,
            balance: Arc::new(RwLock::new(initial_balance)),
            order_counter: Arc::new(RwLock::new(0)),
            daily_pnl: Arc::new(RwLock::new(DailyPnlState::new())),
        }
    }

    /// Generate a new order ID
    async fn next_order_id(&self) -> String {
        let mut counter = self.order_counter.write().await;
        *counter += 1;
        format!("PAPER-{:08}", counter)
    }
}

#[async_trait]
impl TradingExecutor for PaperExecutor {
    fn name(&self) -> &'static str {
        "paper"
    }

    fn is_ready(&self) -> bool {
        true // Always ready for paper trading
    }

    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionResult> {
        let order_id = self.next_order_id().await;

        info!(
            executor = "paper",
            order_id = %order_id,
            side = ?request.side,
            symbol = %request.symbol,
            quantity = request.quantity,
            price = ?request.price,
            "Executing paper order"
        );

        // For paper trading, orders are immediately filled
        let fill_price = request.price.unwrap_or(0.0);
        let amount = request.quantity * fill_price;

        // For sell orders, calculate realized P&L before closing position
        let realized_pnl = if request.side == ExecutionSide::Sell {
            let engine = self.engine.read().await;
            engine
                .get_positions()
                .iter()
                .find(|p| p.symbol == request.symbol)
                .map(|p| (fill_price - p.entry_price) * request.quantity)
                .unwrap_or(0.0)
        } else {
            0.0
        };

        // Update balance
        {
            let mut balance = self.balance.write().await;
            match request.side {
                ExecutionSide::Buy => {
                    if *balance < amount {
                        return Ok(ExecutionResult {
                            order_id,
                            broker_order_id: None,
                            status: ExecutionStatus::Rejected,
                            fill_price: None,
                            fill_quantity: None,
                            timestamp: Utc::now(),
                            error: Some("Insufficient balance".to_string()),
                        });
                    }
                    *balance -= amount;
                }
                ExecutionSide::Sell => {
                    *balance += amount;
                }
            }
        }

        // Update execution engine
        {
            let mut engine = self.engine.write().await;
            match request.side {
                ExecutionSide::Buy => {
                    // Create a minimal signal for the engine
                    let signal = TradingSignal {
                        id: order_id.clone(),
                        symbol: request.symbol.clone(),
                        direction: crate::strategy::SignalDirection::Long,
                        strength: crate::strategy::SignalStrength::Medium,
                        entry_price: fill_price,
                        stop_loss: request.stop_loss.unwrap_or(fill_price * 0.95),
                        take_profit: request.take_profit.unwrap_or(fill_price * 1.10),
                        timestamp: Utc::now(),
                        po3_structure: None,
                        smt_divergence: None,
                        timeframe_alignment: vec![],
                        notes: request.reason.unwrap_or_default(),
                    };
                    let _ = engine.execute_buy(&signal).await;
                }
                ExecutionSide::Sell => {
                    let _ = engine
                        .execute_sell(&request.symbol, request.reason.as_deref().unwrap_or("sell"))
                        .await;
                }
            }
        }

        // Track realized P&L for sell orders
        if request.side == ExecutionSide::Sell && realized_pnl != 0.0 {
            let mut daily_pnl = self.daily_pnl.write().await;
            daily_pnl.add_realized_pnl(realized_pnl);
            info!(
                symbol = %request.symbol,
                realized_pnl,
                "Tracked realized P&L"
            );
        }

        Ok(ExecutionResult {
            order_id,
            broker_order_id: None,
            status: ExecutionStatus::Filled,
            fill_price: Some(fill_price),
            fill_quantity: Some(request.quantity),
            timestamp: Utc::now(),
            error: None,
        })
    }

    async fn cancel(&self, order_id: &str) -> Result<()> {
        debug!(order_id = %order_id, "Paper order cancelled (no-op)");
        Ok(())
    }

    async fn get_positions(&self) -> Result<Vec<Position>> {
        let engine = self.engine.read().await;
        Ok(engine.get_positions())
    }

    async fn get_account(&self) -> Result<AccountInfo> {
        let balance = *self.balance.read().await;
        let positions = self.get_positions().await?;
        let market_value: f64 = positions.iter().map(|p| p.invested_amount()).sum();

        // Get today's realized P&L (auto-resets on new day)
        let realized_pnl_today = {
            let mut daily_pnl = self.daily_pnl.write().await;
            daily_pnl.get_realized_pnl()
        };

        Ok(AccountInfo {
            account_id: "PAPER-ACCOUNT".to_string(),
            total_assets: balance + market_value,
            cash: balance,
            market_value,
            unrealized_pnl: positions.iter().map(|p| p.unrealized_pnl()).sum(),
            realized_pnl_today,
            buying_power: balance,
        })
    }
}

// ============================================================================
// Executor Factory
// ============================================================================

/// Create an executor based on mode
///
/// Note: Since broker integration has been removed, all modes now use
/// PaperExecutor. The system operates as a signal generator with IM
/// notifications for manual execution.
pub fn create_executor(
    _mode: crate::r#loop::TradingMode,
    engine: Arc<RwLock<ExecutionEngine>>,
    initial_balance: f64,
) -> Box<dyn TradingExecutor> {
    // Always use paper executor since broker integration is removed
    Box::new(PaperExecutor::new(engine, initial_balance))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_side_serialization() {
        let side = ExecutionSide::Buy;
        let json = serde_json::to_string(&side).unwrap();
        assert!(json.contains("Buy"));

        let parsed: ExecutionSide = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ExecutionSide::Buy);
    }

    #[test]
    fn test_execution_status_from_order_status() {
        assert_eq!(
            ExecutionStatus::from(OrderStatus::Pending),
            ExecutionStatus::Pending
        );
        assert_eq!(
            ExecutionStatus::from(OrderStatus::Filled),
            ExecutionStatus::Filled
        );
        assert_eq!(
            ExecutionStatus::from(OrderStatus::Rejected),
            ExecutionStatus::Rejected
        );
    }

    #[test]
    fn test_execution_request_serialization() {
        let request = ExecutionRequest {
            side: ExecutionSide::Buy,
            symbol: "000001.SZ".to_string(),
            quantity: 100.0,
            price: Some(10.5),
            stop_loss: Some(10.0),
            take_profit: Some(11.5),
            reason: Some("Test order".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: ExecutionRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.symbol, "000001.SZ");
        assert_eq!(parsed.quantity, 100.0);
    }

    #[test]
    fn test_execution_result_serialization() {
        let result = ExecutionResult {
            order_id: "TEST-001".to_string(),
            broker_order_id: Some("BROKER-001".to_string()),
            status: ExecutionStatus::Filled,
            fill_price: Some(10.5),
            fill_quantity: Some(100.0),
            timestamp: Utc::now(),
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("TEST-001"));
        assert!(json.contains("Filled"));
    }

    #[tokio::test]
    async fn test_paper_executor_creation() {
        let config = zero_common::config::Config::default();
        let engine = Arc::new(RwLock::new(ExecutionEngine::new(&config)));
        let executor = PaperExecutor::new(engine, 100_000.0);

        assert_eq!(executor.name(), "paper");
        assert!(executor.is_ready());
    }

    #[tokio::test]
    async fn test_paper_executor_account() {
        let config = zero_common::config::Config::default();
        let engine = Arc::new(RwLock::new(ExecutionEngine::new(&config)));
        let executor = PaperExecutor::new(engine, 100_000.0);

        let account = executor.get_account().await.unwrap();
        assert_eq!(account.account_id, "PAPER-ACCOUNT");
        assert!((account.cash - 100_000.0).abs() < 0.01);
    }
}
