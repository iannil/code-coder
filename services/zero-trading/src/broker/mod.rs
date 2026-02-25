//! Broker adapters for order execution.
//!
//! Supports multiple brokers for A-share trading:
//! - Futu OpenAPI (富途)
//! - Tiger Securities (老虎证券)
//! - Snowball Securities (雪盈证券)

mod futu;

pub use futu::FutuBroker;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::execution::{Order, OrderStatus, Position};

/// Account information from broker
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

/// Order result from broker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResult {
    /// Order ID from broker
    pub broker_order_id: String,
    /// Order status
    pub status: OrderStatus,
    /// Fill price (if filled)
    pub fill_price: Option<f64>,
    /// Fill quantity (if partially/fully filled)
    pub fill_quantity: Option<f64>,
    /// Error message (if rejected)
    pub error: Option<String>,
}

/// Quote data from broker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerQuote {
    /// Symbol
    pub symbol: String,
    /// Last price
    pub last_price: f64,
    /// Bid price
    pub bid_price: f64,
    /// Ask price
    pub ask_price: f64,
    /// Volume
    pub volume: f64,
    /// Timestamp
    pub timestamp: i64,
}

/// Broker trait for order execution
#[async_trait]
pub trait Broker: Send + Sync {
    /// Get broker name
    fn name(&self) -> &'static str;

    /// Connect to broker gateway
    async fn connect(&mut self) -> Result<()>;

    /// Disconnect from broker
    async fn disconnect(&mut self) -> Result<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Get account information
    async fn get_account_info(&self) -> Result<AccountInfo>;

    /// Get all positions
    async fn get_positions(&self) -> Result<Vec<Position>>;

    /// Get real-time quote
    async fn get_quote(&self, symbol: &str) -> Result<BrokerQuote>;

    /// Place a buy order
    async fn place_buy_order(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
    ) -> Result<OrderResult>;

    /// Place a sell order
    async fn place_sell_order(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
    ) -> Result<OrderResult>;

    /// Cancel an order
    async fn cancel_order(&self, broker_order_id: &str) -> Result<()>;

    /// Get order status
    async fn get_order_status(&self, broker_order_id: &str) -> Result<OrderResult>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_info_serialization() {
        let info = AccountInfo {
            account_id: "test123".to_string(),
            total_assets: 100000.0,
            cash: 50000.0,
            market_value: 50000.0,
            unrealized_pnl: 1000.0,
            realized_pnl_today: 500.0,
            buying_power: 50000.0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test123"));
    }

    #[test]
    fn test_order_result_serialization() {
        let result = OrderResult {
            broker_order_id: "ORD123".to_string(),
            status: OrderStatus::Filled,
            fill_price: Some(10.5),
            fill_quantity: Some(100.0),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("ORD123"));
    }
}
