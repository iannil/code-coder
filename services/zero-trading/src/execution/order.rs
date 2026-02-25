//! Order types and management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Order type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// Market order
    Market,
    /// Limit order
    Limit,
    /// Stop order
    Stop,
    /// Stop limit order
    StopLimit,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// Order is pending submission
    Pending,
    /// Order has been submitted
    Submitted,
    /// Order is partially filled
    PartiallyFilled,
    /// Order is fully filled
    Filled,
    /// Order was cancelled
    Cancelled,
    /// Order was rejected
    Rejected,
    /// Order has expired
    Expired,
}

/// A trading order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Order ID
    pub id: String,
    /// Symbol
    pub symbol: String,
    /// Order side
    pub side: OrderSide,
    /// Order type
    pub order_type: OrderType,
    /// Quantity
    pub quantity: f64,
    /// Limit price (for limit orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    /// Stop price (for stop orders)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<f64>,
    /// Stop loss price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_loss: Option<f64>,
    /// Take profit price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub take_profit: Option<f64>,
    /// Order status
    pub status: OrderStatus,
    /// Filled quantity
    pub filled_quantity: f64,
    /// Average fill price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_fill_price: Option<f64>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Reason for closing (if sell order)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<String>,
    /// Broker order ID (if submitted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broker_order_id: Option<String>,
}

impl Order {
    /// Create a new buy order
    pub fn new_buy(
        symbol: &str,
        quantity: f64,
        price: f64,
        stop_loss: f64,
        take_profit: f64,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity,
            price: Some(price),
            stop_price: None,
            stop_loss: Some(stop_loss),
            take_profit: Some(take_profit),
            status: OrderStatus::Pending,
            filled_quantity: 0.0,
            avg_fill_price: None,
            created_at: now,
            updated_at: now,
            close_reason: None,
            broker_order_id: None,
        }
    }

    /// Create a new sell order
    pub fn new_sell(symbol: &str, quantity: f64, price: f64, reason: &str) -> Self {
        let now = Utc::now();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Limit,
            quantity,
            price: Some(price),
            stop_price: None,
            stop_loss: None,
            take_profit: None,
            status: OrderStatus::Pending,
            filled_quantity: 0.0,
            avg_fill_price: None,
            created_at: now,
            updated_at: now,
            close_reason: Some(reason.to_string()),
            broker_order_id: None,
        }
    }

    /// Create a market order
    pub fn new_market(symbol: &str, side: OrderSide, quantity: f64) -> Self {
        let now = Utc::now();

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            quantity,
            price: None,
            stop_price: None,
            stop_loss: None,
            take_profit: None,
            status: OrderStatus::Pending,
            filled_quantity: 0.0,
            avg_fill_price: None,
            created_at: now,
            updated_at: now,
            close_reason: None,
            broker_order_id: None,
        }
    }

    /// Set order status
    pub fn with_status(mut self, status: OrderStatus) -> Self {
        self.status = status;
        self.updated_at = Utc::now();
        self
    }

    /// Update fill information
    pub fn fill(&mut self, filled_qty: f64, fill_price: f64) {
        self.filled_quantity += filled_qty;
        self.avg_fill_price = Some(fill_price);
        self.updated_at = Utc::now();

        if self.filled_quantity >= self.quantity {
            self.status = OrderStatus::Filled;
        } else if self.filled_quantity > 0.0 {
            self.status = OrderStatus::PartiallyFilled;
        }
    }

    /// Check if order is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected | OrderStatus::Expired
        )
    }

    /// Check if order is working (not complete)
    pub fn is_working(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Pending | OrderStatus::Submitted | OrderStatus::PartiallyFilled
        )
    }

    /// Cancel the order
    pub fn cancel(&mut self) {
        if self.is_working() {
            self.status = OrderStatus::Cancelled;
            self.updated_at = Utc::now();
        }
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        let side = match self.side {
            OrderSide::Buy => "买入",
            OrderSide::Sell => "卖出",
        };
        let status = match self.status {
            OrderStatus::Pending => "待提交",
            OrderStatus::Submitted => "已提交",
            OrderStatus::PartiallyFilled => "部分成交",
            OrderStatus::Filled => "已成交",
            OrderStatus::Cancelled => "已取消",
            OrderStatus::Rejected => "已拒绝",
            OrderStatus::Expired => "已过期",
        };

        format!(
            "{} {} | {} {} @ {:.2} | 状态:{}",
            self.symbol,
            side,
            self.quantity,
            match self.order_type {
                OrderType::Market => "市价",
                OrderType::Limit => "限价",
                OrderType::Stop => "止损",
                OrderType::StopLimit => "止损限价",
            },
            self.price.unwrap_or(0.0),
            status
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
    fn test_order_creation() {
        let order = Order::new_buy("000001.SZ", 100.0, 10.0, 9.5, 11.0);

        assert_eq!(order.symbol, "000001.SZ");
        assert_eq!(order.side, OrderSide::Buy);
        assert_eq!(order.quantity, 100.0);
        assert_eq!(order.status, OrderStatus::Pending);
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::new_buy("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        order.fill(100.0, 10.05);

        assert_eq!(order.status, OrderStatus::Filled);
        assert!((order.avg_fill_price.unwrap() - 10.05).abs() < 0.01);
    }

    #[test]
    fn test_order_partial_fill() {
        let mut order = Order::new_buy("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        order.fill(50.0, 10.0);

        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert!((order.filled_quantity - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_order_cancel() {
        let mut order = Order::new_buy("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        order.cancel();

        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_order_is_working() {
        let order = Order::new_buy("000001.SZ", 100.0, 10.0, 9.5, 11.0);
        assert!(order.is_working());

        let filled_order = order.with_status(OrderStatus::Filled);
        assert!(!filled_order.is_working());
    }
}
