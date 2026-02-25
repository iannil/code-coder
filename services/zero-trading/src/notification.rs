//! Notification module for sending trading signals.
//!
//! Integrates with zero-channels to send signals to Telegram and other channels.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use zero_common::config::Config;

use crate::strategy::TradingSignal;

/// Request to send a message via zero-channels
#[derive(Debug, Serialize)]
struct SendRequest {
    channel_type: String,
    channel_id: String,
    content: SendContent,
}

/// Message content
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SendContent {
    Text { text: String },
    Markdown { text: String },
}

/// Response from zero-channels
#[derive(Debug, Deserialize)]
struct SendResponse {
    success: bool,
    #[allow(dead_code)]
    message_id: Option<String>,
    error: Option<String>,
}

/// Notification client for sending trading signals
pub struct NotificationClient {
    enabled: bool,
    channels_endpoint: String,
    channel_type: String,
    channel_id: String,
    retry_count: u32,
    notify_signals: bool,
    notify_orders: bool,
    notify_positions: bool,
    client: reqwest::Client,
}

impl NotificationClient {
    /// Create a new notification client
    pub fn new(config: &Config) -> Self {
        let trading = config.trading.as_ref();
        let notification = trading.and_then(|t| t.telegram_notification.as_ref());

        let (enabled, channels_endpoint, channel_type, channel_id, retry_count, notify_signals, notify_orders, notify_positions) =
            match notification {
                Some(n) => (
                    n.enabled,
                    n.channels_endpoint.clone(),
                    n.channel_type.clone(),
                    n.telegram_chat_id.clone().unwrap_or_default(),
                    n.retry_count,
                    n.notify_signals,
                    n.notify_orders,
                    n.notify_positions,
                ),
                None => (
                    false,
                    "http://127.0.0.1:4431".to_string(),
                    "telegram".to_string(),
                    String::new(),
                    3,
                    true,
                    true,
                    true,
                ),
            };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            enabled,
            channels_endpoint,
            channel_type,
            channel_id,
            retry_count,
            notify_signals,
            notify_orders,
            notify_positions,
            client,
        }
    }

    /// Check if notifications are enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled && !self.channel_id.is_empty()
    }

    /// Check if signal notifications are enabled
    pub fn should_notify_signals(&self) -> bool {
        self.is_enabled() && self.notify_signals
    }

    /// Check if order notifications are enabled
    pub fn should_notify_orders(&self) -> bool {
        self.is_enabled() && self.notify_orders
    }

    /// Check if position notifications are enabled
    pub fn should_notify_positions(&self) -> bool {
        self.is_enabled() && self.notify_positions
    }

    /// Send a trading signal notification
    pub async fn send_signal(&self, signal: &TradingSignal) -> Result<()> {
        if !self.should_notify_signals() {
            tracing::debug!("Signal notifications disabled, skipping");
            return Ok(());
        }

        let message = signal.to_telegram_message();
        self.send_message(&message).await
    }

    /// Send a custom message
    pub async fn send_message(&self, message: &str) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let url = format!("{}/api/v1/send", self.channels_endpoint);

        let request = SendRequest {
            channel_type: self.channel_type.clone(),
            channel_id: self.channel_id.clone(),
            content: SendContent::Markdown { text: message.to_string() },
        };

        let mut last_error = None;

        for attempt in 1..=self.retry_count {
            match self.try_send(&url, &request).await {
                Ok(()) => {
                    tracing::info!(
                        channel_type = %self.channel_type,
                        channel_id = %self.channel_id,
                        "Signal notification sent successfully"
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        attempt,
                        max_attempts = self.retry_count,
                        error = %e,
                        "Failed to send notification, retrying..."
                    );
                    last_error = Some(e);

                    if attempt < self.retry_count {
                        tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
    }

    /// Try to send a single request
    async fn try_send(&self, url: &str, request: &SendRequest) -> Result<()> {
        let response = self
            .client
            .post(url)
            .json(request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {}: {}", status, error_text);
        }

        let result: SendResponse = response.json().await?;

        if result.success {
            Ok(())
        } else {
            anyhow::bail!("Send failed: {}", result.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }

    /// Send a position update notification
    pub async fn send_position_update(&self, symbol: &str, action: &str, price: f64) -> Result<()> {
        if !self.should_notify_positions() {
            return Ok(());
        }

        let message = format!(
            "📊 *仓位更新*\n\n\
            标的: `{}`\n\
            操作: {}\n\
            价格: {:.2}",
            symbol, action, price
        );

        self.send_message(&message).await
    }

    /// Send an order notification
    pub async fn send_order_update(&self, symbol: &str, order_type: &str, price: f64, quantity: f64) -> Result<()> {
        if !self.should_notify_orders() {
            return Ok(());
        }

        let message = format!(
            "📝 *订单更新*\n\n\
            标的: `{}`\n\
            类型: {}\n\
            价格: {:.2}\n\
            数量: {:.0}",
            symbol, order_type, price, quantity
        );

        self.send_message(&message).await
    }

    /// Send an alert notification
    pub async fn send_alert(&self, title: &str, message: &str) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let formatted = format!(
            "⚠️ *{}*\n\n{}",
            title, message
        );

        self.send_message(&formatted).await
    }

    /// Send a daily summary
    pub async fn send_daily_summary(
        &self,
        signals_count: usize,
        positions_count: usize,
        daily_pnl: f64,
    ) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let pnl_emoji = if daily_pnl >= 0.0 { "📈" } else { "📉" };
        let pnl_sign = if daily_pnl >= 0.0 { "+" } else { "" };

        let message = format!(
            "📋 *每日交易摘要*\n\n\
            信号数量: {}\n\
            当前持仓: {}\n\
            日内盈亏: {} {}{:.2}%",
            signals_count, positions_count, pnl_emoji, pnl_sign, daily_pnl
        );

        self.send_message(&message).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_client_disabled_by_default() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        assert!(!client.is_enabled());
    }

    #[test]
    fn test_send_content_serialization() {
        let content = SendContent::Markdown { text: "Hello *world*".to_string() };
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("markdown"));
        assert!(json.contains("Hello *world*"));
    }

    #[test]
    fn test_send_request_serialization() {
        let request = SendRequest {
            channel_type: "telegram".to_string(),
            channel_id: "123456".to_string(),
            content: SendContent::Text { text: "Test".to_string() },
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("telegram"));
        assert!(json.contains("123456"));
    }
}
