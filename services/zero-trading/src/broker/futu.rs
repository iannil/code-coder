//! Futu OpenAPI broker adapter.
//!
//! Connects to Futu OpenD gateway for order execution.
//!
//! # Prerequisites
//!
//! 1. Install Futu OpenD: https://www.futunn.com/download/openAPI
//! 2. Login to OpenD with your Futu account
//! 3. Configure the gateway port (default: 11111)
//!
//! # Protocol
//!
//! Futu OpenD uses a custom binary protocol over TCP:
//! - Header: 44 bytes (magic, version, proto_id, etc.)
//! - Body: Protocol Buffer encoded message
//!
//! This adapter provides a simplified interface, wrapping the low-level protocol.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use zero_common::config::Config;

use super::{AccountInfo, Broker, BrokerQuote, OrderResult};
use crate::execution::{OrderStatus, Position};

/// Futu OpenD protocol constants
const FUTU_MAGIC: u64 = 0x464554_5554; // "FUTUT" in hex
const HEADER_SIZE: usize = 44;

/// Futu broker adapter
pub struct FutuBroker {
    /// Gateway host
    host: String,
    /// Gateway port
    port: u16,
    /// Trading password (for unlock)
    trading_password: Option<String>,
    /// Enable real trading mode
    real_trading: bool,
    /// TCP connection
    connection: Arc<Mutex<Option<TcpStream>>>,
    /// Connection state
    connected: AtomicBool,
    /// Protocol sequence number
    seq_no: Arc<Mutex<u32>>,
}

impl FutuBroker {
    /// Create a new Futu broker
    pub fn new(config: &Config) -> Self {
        let (host, port, trading_password, real_trading) = config
            .trading
            .as_ref()
            .and_then(|t| t.futu.as_ref())
            .map(|f| (
                f.host.clone(),
                f.port,
                f.trading_password.clone(),
                f.real_trading,
            ))
            .unwrap_or_else(|| (
                "127.0.0.1".to_string(),
                11111,
                None,
                false,
            ));

        Self {
            host,
            port,
            trading_password,
            real_trading,
            connection: Arc::new(Mutex::new(None)),
            connected: AtomicBool::new(false),
            seq_no: Arc::new(Mutex::new(0)),
        }
    }

    /// Get next sequence number
    async fn next_seq(&self) -> u32 {
        let mut seq = self.seq_no.lock().await;
        *seq += 1;
        *seq
    }

    /// Send a request and receive response
    async fn send_request(&self, proto_id: u32, body: &[u8]) -> Result<Vec<u8>> {
        let mut conn = self.connection.lock().await;
        let stream = conn.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to Futu OpenD"))?;

        // Build header
        let seq = self.next_seq().await;
        let header = self.build_header(proto_id, body.len() as u32, seq);

        // Send request
        stream.write_all(&header).await?;
        stream.write_all(body).await?;
        stream.flush().await?;

        // Read response header
        let mut resp_header = [0u8; HEADER_SIZE];
        stream.read_exact(&mut resp_header).await?;

        // Parse response length from header
        let body_len = u32::from_le_bytes([
            resp_header[16], resp_header[17], resp_header[18], resp_header[19]
        ]) as usize;

        // Read response body
        let mut resp_body = vec![0u8; body_len];
        stream.read_exact(&mut resp_body).await?;

        Ok(resp_body)
    }

    /// Build protocol header
    fn build_header(&self, proto_id: u32, body_len: u32, seq_no: u32) -> Vec<u8> {
        let mut header = vec![0u8; HEADER_SIZE];

        // Magic (8 bytes)
        header[0..8].copy_from_slice(&FUTU_MAGIC.to_le_bytes());

        // Proto ID (4 bytes at offset 8)
        header[8..12].copy_from_slice(&proto_id.to_le_bytes());

        // Protocol version (2 bytes at offset 12)
        header[12..14].copy_from_slice(&1u16.to_le_bytes());

        // Reserved (2 bytes at offset 14)
        // Body length (4 bytes at offset 16)
        header[16..20].copy_from_slice(&body_len.to_le_bytes());

        // Sequence number (4 bytes at offset 20)
        header[20..24].copy_from_slice(&seq_no.to_le_bytes());

        // SHA1 hash and reserved fill remaining bytes

        header
    }

    /// Unlock trade (required before placing orders)
    async fn unlock_trade(&self) -> Result<()> {
        let Some(ref password) = self.trading_password else {
            anyhow::bail!("Trading password not configured");
        };

        // Proto ID for Trd_UnlockTrade: 2005
        // In real implementation, this would encode the protobuf message
        tracing::info!("Unlocking trade with password");

        // Placeholder - actual implementation would send protobuf
        let body = password.as_bytes();
        let _resp = self.send_request(2005, body).await?;

        Ok(())
    }

    /// Parse position from protobuf response
    fn parse_position(&self, _data: &[u8]) -> Result<Position> {
        // Placeholder - would parse Trd_GetPositionList response
        anyhow::bail!("Position parsing not implemented")
    }

    /// Market code for A-shares
    fn market_code(&self) -> i32 {
        // Futu market codes:
        // 1 = HK, 11 = US, 21 = CN-SH, 22 = CN-SZ
        21 // Default to Shanghai
    }
}

#[async_trait]
impl Broker for FutuBroker {
    fn name(&self) -> &'static str {
        "Futu"
    }

    async fn connect(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.host, self.port);
        tracing::info!(addr = %addr, "Connecting to Futu OpenD");

        let stream = tokio::time::timeout(
            Duration::from_secs(10),
            TcpStream::connect(&addr),
        )
        .await
        .context("Connection timeout")?
        .context("Failed to connect to Futu OpenD")?;

        // Enable TCP keepalive
        stream.set_nodelay(true)?;

        {
            let mut conn = self.connection.lock().await;
            *conn = Some(stream);
        }

        self.connected.store(true, Ordering::SeqCst);
        tracing::info!("Connected to Futu OpenD");

        // Initialize connection (send InitConnect)
        // Proto ID for InitConnect: 1001
        let init_body = vec![]; // Simplified - real impl would have protobuf
        let _resp = self.send_request(1001, &init_body).await?;

        // Unlock trade if password is configured
        if self.trading_password.is_some() {
            self.unlock_trade().await?;
        }

        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        let mut conn = self.connection.lock().await;
        if let Some(stream) = conn.take() {
            drop(stream);
        }
        self.connected.store(false, Ordering::SeqCst);
        tracing::info!("Disconnected from Futu OpenD");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn get_account_info(&self) -> Result<AccountInfo> {
        if !self.is_connected() {
            anyhow::bail!("Not connected to broker");
        }

        // Proto ID for Trd_GetFunds: 2101
        let _resp = self.send_request(2101, &[]).await?;

        // Placeholder response
        Ok(AccountInfo {
            account_id: "futu_demo".to_string(),
            total_assets: 100000.0,
            cash: 50000.0,
            market_value: 50000.0,
            unrealized_pnl: 0.0,
            realized_pnl_today: 0.0,
            buying_power: 50000.0,
        })
    }

    async fn get_positions(&self) -> Result<Vec<Position>> {
        if !self.is_connected() {
            anyhow::bail!("Not connected to broker");
        }

        // Proto ID for Trd_GetPositionList: 2102
        let _resp = self.send_request(2102, &[]).await?;

        // Placeholder - return empty positions
        Ok(vec![])
    }

    async fn get_quote(&self, symbol: &str) -> Result<BrokerQuote> {
        if !self.is_connected() {
            anyhow::bail!("Not connected to broker");
        }

        // Proto ID for Qot_GetBasicQot: 3004
        let _resp = self.send_request(3004, symbol.as_bytes()).await?;

        // Placeholder response
        Ok(BrokerQuote {
            symbol: symbol.to_string(),
            last_price: 10.0,
            bid_price: 9.99,
            ask_price: 10.01,
            volume: 1000000.0,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    async fn place_buy_order(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
    ) -> Result<OrderResult> {
        if !self.is_connected() {
            anyhow::bail!("Not connected to broker");
        }

        if !self.real_trading {
            tracing::info!(
                symbol,
                quantity,
                price,
                "[PAPER] Simulated buy order"
            );
            return Ok(OrderResult {
                broker_order_id: format!("PAPER_{}", uuid::Uuid::new_v4()),
                status: OrderStatus::Filled,
                fill_price: Some(price),
                fill_quantity: Some(quantity),
                error: None,
            });
        }

        // Proto ID for Trd_PlaceOrder: 2202
        // In real implementation, encode the order details in protobuf
        tracing::info!(symbol, quantity, price, "Placing buy order");

        let body = format!("{}|{}|{}", symbol, quantity, price);
        let _resp = self.send_request(2202, body.as_bytes()).await?;

        // Placeholder response
        Ok(OrderResult {
            broker_order_id: format!("FUTU_{}", uuid::Uuid::new_v4()),
            status: OrderStatus::Pending,
            fill_price: None,
            fill_quantity: None,
            error: None,
        })
    }

    async fn place_sell_order(
        &self,
        symbol: &str,
        quantity: f64,
        price: f64,
    ) -> Result<OrderResult> {
        if !self.is_connected() {
            anyhow::bail!("Not connected to broker");
        }

        if !self.real_trading {
            tracing::info!(
                symbol,
                quantity,
                price,
                "[PAPER] Simulated sell order"
            );
            return Ok(OrderResult {
                broker_order_id: format!("PAPER_{}", uuid::Uuid::new_v4()),
                status: OrderStatus::Filled,
                fill_price: Some(price),
                fill_quantity: Some(quantity),
                error: None,
            });
        }

        // Proto ID for Trd_PlaceOrder: 2202 (with sell side)
        tracing::info!(symbol, quantity, price, "Placing sell order");

        let body = format!("SELL|{}|{}|{}", symbol, quantity, price);
        let _resp = self.send_request(2202, body.as_bytes()).await?;

        Ok(OrderResult {
            broker_order_id: format!("FUTU_{}", uuid::Uuid::new_v4()),
            status: OrderStatus::Pending,
            fill_price: None,
            fill_quantity: None,
            error: None,
        })
    }

    async fn cancel_order(&self, broker_order_id: &str) -> Result<()> {
        if !self.is_connected() {
            anyhow::bail!("Not connected to broker");
        }

        // Proto ID for Trd_ModifyOrder: 2205 (with cancel action)
        tracing::info!(broker_order_id, "Cancelling order");

        let _resp = self.send_request(2205, broker_order_id.as_bytes()).await?;
        Ok(())
    }

    async fn get_order_status(&self, broker_order_id: &str) -> Result<OrderResult> {
        if !self.is_connected() {
            anyhow::bail!("Not connected to broker");
        }

        // Proto ID for Trd_GetOrderList: 2201
        let _resp = self.send_request(2201, broker_order_id.as_bytes()).await?;

        // Placeholder
        Ok(OrderResult {
            broker_order_id: broker_order_id.to_string(),
            status: OrderStatus::Pending,
            fill_price: None,
            fill_quantity: None,
            error: None,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_futu_broker_creation() {
        let config = Config::default();
        let broker = FutuBroker::new(&config);
        assert_eq!(broker.host, "127.0.0.1");
        assert_eq!(broker.port, 11111);
        assert!(!broker.is_connected());
    }

    #[test]
    fn test_header_building() {
        let config = Config::default();
        let broker = FutuBroker::new(&config);
        let header = broker.build_header(1001, 0, 1);
        assert_eq!(header.len(), HEADER_SIZE);
    }
}
