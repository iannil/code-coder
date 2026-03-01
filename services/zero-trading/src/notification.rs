//! Notification module for sending trading signals.
//!
//! Integrates with zero-channels to send signals to Telegram and other channels.
//!
//! # Features
//! - Enhanced signal notifications with execution recommendations
//! - Retry queue for failed notifications (persists to disk)
//! - Background retry task for resilient delivery

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use zero_common::config::Config;
use zero_common::{build_client, ClientCategory};

use crate::macro_filter::{MacroEnvironment, TradingBias};
use crate::strategy::{SignalStrength, TradingSignal};

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
    #[allow(dead_code)] // Reserved for plain text messages
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

/// A notification that failed to send and is queued for retry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedNotification {
    /// Unique ID for this notification
    pub id: String,
    /// Message content
    pub message: String,
    /// Channel type (telegram, etc.)
    pub channel_type: String,
    /// Channel ID (chat ID, etc.)
    pub channel_id: String,
    /// Original timestamp when signal was generated
    pub created_at: DateTime<Utc>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Last retry attempt timestamp
    pub last_retry_at: Option<DateTime<Utc>>,
    /// Last error message
    pub last_error: Option<String>,
    /// Signal ID if this is a trading signal notification
    pub signal_id: Option<String>,
}

impl FailedNotification {
    /// Create a new failed notification entry
    pub fn new(
        message: String,
        channel_type: String,
        channel_id: String,
        signal_id: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message,
            channel_type,
            channel_id,
            created_at: Utc::now(),
            retry_count: 0,
            last_retry_at: None,
            last_error: None,
            signal_id,
        }
    }

    /// Check if this notification should be retried
    pub fn should_retry(&self, max_retries: u32) -> bool {
        self.retry_count < max_retries
    }

    /// Check if this notification has expired (older than 1 hour)
    pub fn is_expired(&self) -> bool {
        let age = Utc::now().signed_duration_since(self.created_at);
        age.num_hours() >= 1
    }
}

/// Retry queue for failed notifications
#[derive(Debug)]
pub struct NotificationRetryQueue {
    /// Queue of failed notifications
    queue: Arc<RwLock<VecDeque<FailedNotification>>>,
    /// Path to persist queue to disk
    persist_path: Option<PathBuf>,
    /// Maximum queue size
    max_size: usize,
}

impl NotificationRetryQueue {
    /// Create a new retry queue
    pub fn new(persist_path: Option<PathBuf>, max_size: usize) -> Self {
        let queue = Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            persist_path,
            max_size,
        };

        // Try to load persisted queue
        if let Some(path) = &queue.persist_path {
            if path.exists() {
                if let Ok(data) = std::fs::read_to_string(path) {
                    if let Ok(items) = serde_json::from_str::<Vec<FailedNotification>>(&data) {
                        let mut q = queue.queue.blocking_write();
                        for item in items {
                            if !item.is_expired() {
                                q.push_back(item);
                            }
                        }
                        tracing::info!(
                            loaded = q.len(),
                            path = %path.display(),
                            "Loaded persisted notification queue"
                        );
                    }
                }
            }
        }

        queue
    }

    /// Add a failed notification to the queue
    pub async fn push(&self, notification: FailedNotification) {
        let mut queue = self.queue.write().await;

        // Remove oldest if at capacity
        if queue.len() >= self.max_size {
            queue.pop_front();
        }

        queue.push_back(notification);
        tracing::debug!(queue_size = queue.len(), "Added notification to retry queue");

        // Persist to disk
        self.persist_async(&queue).await;
    }

    /// Get the next notification to retry
    pub async fn pop(&self) -> Option<FailedNotification> {
        let mut queue = self.queue.write().await;
        let notification = queue.pop_front();

        if notification.is_some() {
            self.persist_async(&queue).await;
        }

        notification
    }

    /// Re-queue a notification after failed retry
    pub async fn requeue(&self, mut notification: FailedNotification, error: String) {
        notification.retry_count += 1;
        notification.last_retry_at = Some(Utc::now());
        notification.last_error = Some(error);

        let mut queue = self.queue.write().await;
        queue.push_back(notification);

        self.persist_async(&queue).await;
    }

    /// Get queue size
    pub async fn len(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.queue.read().await.is_empty()
    }

    /// Persist queue to disk
    async fn persist_async(&self, queue: &VecDeque<FailedNotification>) {
        if let Some(path) = &self.persist_path {
            let items: Vec<_> = queue.iter().collect();
            if let Ok(data) = serde_json::to_string(&items) {
                if let Err(e) = tokio::fs::write(path, data).await {
                    tracing::warn!(error = %e, "Failed to persist notification queue");
                }
            }
        }
    }

    /// Remove expired notifications
    pub async fn cleanup(&self) {
        let mut queue = self.queue.write().await;
        let before = queue.len();
        queue.retain(|n| !n.is_expired());
        let after = queue.len();

        if before != after {
            tracing::info!(
                removed = before - after,
                remaining = after,
                "Cleaned up expired notifications"
            );
            self.persist_async(&queue).await;
        }
    }
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
    /// Retry queue for failed notifications
    retry_queue: Arc<NotificationRetryQueue>,
    /// Maximum retries for queued notifications
    max_queue_retries: u32,
}

impl NotificationClient {
    /// Create a new notification client
    ///
    /// Priority for chat_id:
    /// 1. channels.telegram.trading_chat_id (set via /bind_trading command)
    /// 2. trading.telegram_notification.telegram_chat_id (manual config)
    pub fn new(config: &Config) -> Self {
        let trading = config.trading.as_ref();
        let notification = trading.and_then(|t| t.telegram_notification.as_ref());

        // Get chat_id from channels.telegram.trading_chat_id first (set via /bind_trading)
        // Fall back to trading.telegram_notification.telegram_chat_id
        let channel_id = config
            .channels
            .telegram
            .as_ref()
            .and_then(|t| t.trading_chat_id.clone())
            .or_else(|| notification.and_then(|n| n.telegram_chat_id.clone()))
            .unwrap_or_default();

        let (enabled, channel_type, retry_count, notify_signals, notify_orders, notify_positions) =
            match notification {
                Some(n) => (
                    n.enabled,
                    n.channel_type.clone(),
                    n.retry_count,
                    n.notify_signals,
                    n.notify_orders,
                    n.notify_positions,
                ),
                None => (
                    // Enable by default if trading_chat_id is set via /bind_trading
                    !channel_id.is_empty(),
                    "telegram".to_string(),
                    3,
                    true,
                    true,
                    true,
                ),
            };

        // Use centralized channels_endpoint from Config
        let channels_endpoint = config.channels_endpoint();

        // Use http_client factory for proper timeout configuration
        let client = build_client(&config.timeout, ClientCategory::Notification);

        // Initialize retry queue with persistence path
        let persist_path = dirs::data_local_dir()
            .map(|p| p.join("codecoder").join("notification_queue.json"));
        let retry_queue = Arc::new(NotificationRetryQueue::new(persist_path, 100));

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
            retry_queue,
            max_queue_retries: 10, // Retry up to 10 times over ~1 hour
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
        self.send_message_with_signal_id(&message, Some(signal.id.clone())).await
    }

    /// Send an enhanced trading signal notification with execution recommendations
    ///
    /// This method provides comprehensive signal notifications including:
    /// - Execution recommendation based on signal strength and macro conditions
    /// - Position sizing suggestion based on macro environment
    /// - Risk tips (T+1 rule reminder, stop loss emphasis)
    /// - Urgency indicator based on signal validity window
    pub async fn send_trading_signal_with_recommendation(
        &self,
        signal: &TradingSignal,
        macro_env: Option<&MacroEnvironment>,
    ) -> Result<()> {
        if !self.should_notify_signals() {
            tracing::debug!("Signal notifications disabled, skipping");
            return Ok(());
        }

        let recommendation = self.generate_execution_recommendation(signal, macro_env);
        let position_suggestion = self.generate_position_suggestion(signal, macro_env);
        let risk_tips = self.generate_risk_tips(signal);
        let urgency = self.generate_urgency_indicator(signal);

        let message = self.format_enhanced_signal_message(
            signal,
            macro_env,
            &recommendation,
            &position_suggestion,
            &risk_tips,
            &urgency,
        );

        self.send_message_with_signal_id(&message, Some(signal.id.clone())).await
    }

    /// Generate execution recommendation based on signal and macro context
    fn generate_execution_recommendation(
        &self,
        signal: &TradingSignal,
        macro_env: Option<&MacroEnvironment>,
    ) -> String {
        let strength_factor = match signal.strength {
            SignalStrength::VeryStrong => "极强信号，高置信度入场机会",
            SignalStrength::Strong => "强信号，可考虑标准仓位入场",
            SignalStrength::Medium => "中等信号，建议轻仓试探",
            SignalStrength::Weak => "弱信号，观望为主，等待确认",
        };

        let macro_factor = macro_env
            .map(|env| match env.trading_bias {
                TradingBias::Bullish => "宏观环境看多，支持做多信号",
                TradingBias::Neutral => "宏观环境中性，按信号强度操作",
                TradingBias::Bearish => "宏观环境偏空，做多信号需谨慎",
                TradingBias::AvoidTrading => "宏观环境不利，建议暂缓操作",
            })
            .unwrap_or("宏观数据暂缺，按技术面操作");

        format!("{} | {}", strength_factor, macro_factor)
    }

    /// Generate position sizing suggestion
    fn generate_position_suggestion(
        &self,
        signal: &TradingSignal,
        macro_env: Option<&MacroEnvironment>,
    ) -> String {
        let base_pct = match signal.strength {
            SignalStrength::VeryStrong => 15.0,
            SignalStrength::Strong => 10.0,
            SignalStrength::Medium => 5.0,
            SignalStrength::Weak => 0.0,
        };

        let multiplier = macro_env
            .map(|env| env.position_multiplier)
            .unwrap_or(1.0);

        let adjusted_pct = (base_pct * multiplier).min(20.0);

        if adjusted_pct > 0.0 {
            format!(
                "建议仓位: {:.0}% (基础{:.0}% × 宏观系数{:.1})",
                adjusted_pct, base_pct, multiplier
            )
        } else {
            "建议仓位: 观望，暂不建仓".to_string()
        }
    }

    /// Generate risk tips based on signal characteristics
    fn generate_risk_tips(&self, signal: &TradingSignal) -> String {
        let mut tips = Vec::new();

        // T+1 rule reminder
        tips.push("📌 A股T+1: 今日买入明日方可卖出");

        // Stop loss emphasis
        let risk_pct = signal.risk_percent();
        if risk_pct > 5.0 {
            tips.push("⚠️ 风险较高: 止损距离超过5%，建议减小仓位");
        } else if risk_pct > 3.0 {
            tips.push("💡 正常风险: 止损距离适中，严格执行");
        } else {
            tips.push("✅ 低风险: 止损距离紧凑，风控良好");
        }

        // R:R quality
        let rr = signal.risk_reward();
        if rr >= 3.0 {
            tips.push("⭐ 优质R:R: 风险回报比≥3:1");
        } else if rr >= 2.0 {
            tips.push("📊 合理R:R: 风险回报比≥2:1");
        } else {
            tips.push("⚡ R:R偏低: 建议等待更好入场点");
        }

        tips.join("\n")
    }

    /// Generate urgency indicator based on signal validity
    fn generate_urgency_indicator(&self, signal: &TradingSignal) -> String {
        use chrono::Utc;

        let age_minutes = Utc::now()
            .signed_duration_since(signal.timestamp)
            .num_minutes();

        match age_minutes {
            0..=5 => "🔴 刚产生 - 可立即评估".to_string(),
            6..=15 => "🟡 较新鲜 - 建议尽快评估".to_string(),
            16..=30 => "🟢 有效期内 - 仍可考虑".to_string(),
            31..=60 => "⚪ 即将过期 - 需确认是否仍有效".to_string(),
            _ => "⬜ 已过期 - 建议等待新信号".to_string(),
        }
    }

    /// Format the enhanced signal message with all components
    fn format_enhanced_signal_message(
        &self,
        signal: &TradingSignal,
        macro_env: Option<&MacroEnvironment>,
        recommendation: &str,
        position_suggestion: &str,
        risk_tips: &str,
        urgency: &str,
    ) -> String {
        let base_message = signal.to_telegram_message();

        // Build macro context section if available
        let macro_section = macro_env
            .map(|env| {
                format!(
                    "\n\n🌍 *宏观环境*\n\
                    周期阶段: {:?}\n\
                    交易偏向: {:?}\n\
                    仓位系数: {:.1}x\n\
                    {}",
                    env.cycle_phase,
                    env.trading_bias,
                    env.position_multiplier,
                    if !env.notes.is_empty() {
                        format!("备注: {}", env.notes)
                    } else {
                        String::new()
                    }
                )
            })
            .unwrap_or_default();

        format!(
            "{}{}\n\n\
            ━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n\
            📋 *执行建议*\n{}\n\n\
            💰 *仓位建议*\n{}\n\n\
            ⚠️ *风险提示*\n{}\n\n\
            ⏰ *时效性* {}\n\n\
            ━━━━━━━━━━━━━━━━━━━━━━━━━━\n\
            _此信号仅供参考，不构成投资建议_",
            base_message,
            macro_section,
            recommendation,
            position_suggestion,
            risk_tips,
            urgency
        )
    }

    /// Send a custom message
    pub async fn send_message(&self, message: &str) -> Result<()> {
        self.send_message_with_signal_id(message, None).await
    }

    /// Send a message with optional signal ID for retry tracking
    pub async fn send_message_with_signal_id(
        &self,
        message: &str,
        signal_id: Option<String>,
    ) -> Result<()> {
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

        // All immediate retries failed, add to persistent retry queue
        let failed = FailedNotification::new(
            message.to_string(),
            self.channel_type.clone(),
            self.channel_id.clone(),
            signal_id,
        );

        tracing::warn!(
            notification_id = %failed.id,
            "All immediate retries failed, queuing notification for background retry"
        );

        self.retry_queue.push(failed).await;

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error")))
    }

    /// Process a single notification from the retry queue
    ///
    /// Returns true if queue is empty or notification was successfully sent/expired
    pub async fn process_retry_queue(&self) -> bool {
        if !self.is_enabled() {
            return true;
        }

        let notification = match self.retry_queue.pop().await {
            Some(n) => n,
            None => return true, // Queue is empty
        };

        // Skip expired notifications
        if notification.is_expired() {
            tracing::info!(
                notification_id = %notification.id,
                signal_id = ?notification.signal_id,
                "Notification expired, removing from queue"
            );
            return false;
        }

        // Skip if max retries reached
        if !notification.should_retry(self.max_queue_retries) {
            tracing::warn!(
                notification_id = %notification.id,
                retry_count = notification.retry_count,
                "Max retries reached, dropping notification"
            );
            return false;
        }

        // Try to send
        let url = format!("{}/api/v1/send", self.channels_endpoint);
        let request = SendRequest {
            channel_type: notification.channel_type.clone(),
            channel_id: notification.channel_id.clone(),
            content: SendContent::Markdown { text: notification.message.clone() },
        };

        match self.try_send(&url, &request).await {
            Ok(()) => {
                tracing::info!(
                    notification_id = %notification.id,
                    signal_id = ?notification.signal_id,
                    retry_count = notification.retry_count,
                    "Queued notification sent successfully"
                );
                false
            }
            Err(e) => {
                // Requeue for later retry
                self.retry_queue.requeue(notification, e.to_string()).await;
                false
            }
        }
    }

    /// Start background retry task
    ///
    /// Processes the retry queue every 5 minutes
    pub async fn start_retry_task(self: Arc<Self>) {
        tracing::info!("Starting notification retry background task");

        loop {
            // Process up to 10 notifications per cycle
            for _ in 0..10 {
                if self.process_retry_queue().await {
                    break; // Queue is empty
                }
            }

            // Cleanup expired notifications
            self.retry_queue.cleanup().await;

            // Log queue status
            let queue_size = self.retry_queue.len().await;
            if queue_size > 0 {
                tracing::debug!(queue_size, "Retry queue status");
            }

            // Sleep for 5 minutes
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    }

    /// Get retry queue statistics
    pub async fn queue_stats(&self) -> (usize, bool) {
        (self.retry_queue.len().await, self.retry_queue.is_empty().await)
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
    use crate::data::Timeframe;
    use crate::macro_filter::{CompositeIndicators, EconomicCyclePhase, PolicyCycle};
    use crate::strategy::SignalDirection;
    use chrono::Utc;

    fn make_test_signal(strength: SignalStrength) -> TradingSignal {
        TradingSignal {
            id: "test-1".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: SignalDirection::Long,
            strength,
            entry_price: 10.0,
            stop_loss: 9.5,
            take_profit: 12.0,
            timestamp: Utc::now(),
            po3_structure: None,
            smt_divergence: None,
            timeframe_alignment: vec![Timeframe::Daily, Timeframe::H4],
            notes: "Test signal".to_string(),
        }
    }

    fn make_test_macro_env(bias: TradingBias, multiplier: f64) -> MacroEnvironment {
        MacroEnvironment {
            cycle_phase: EconomicCyclePhase::Expansion,
            m2_growth: Some(10.5),
            social_financing: None,
            risk_appetite: 60.0,
            pmi: Some(52.5),
            position_multiplier: multiplier,
            trading_bias: bias,
            notes: "Test macro".to_string(),
            composite_indicators: CompositeIndicators::default(),
            policy_cycle: PolicyCycle::Neutral,
        }
    }

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

    #[test]
    fn test_execution_recommendation_strong_bullish() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::Strong);
        let macro_env = make_test_macro_env(TradingBias::Bullish, 1.2);

        let recommendation = client.generate_execution_recommendation(&signal, Some(&macro_env));

        assert!(recommendation.contains("强信号"));
        assert!(recommendation.contains("宏观环境看多"));
    }

    #[test]
    fn test_execution_recommendation_weak_bearish() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::Weak);
        let macro_env = make_test_macro_env(TradingBias::Bearish, 0.7);

        let recommendation = client.generate_execution_recommendation(&signal, Some(&macro_env));

        assert!(recommendation.contains("弱信号"));
        assert!(recommendation.contains("宏观环境偏空"));
    }

    #[test]
    fn test_execution_recommendation_no_macro() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::Medium);

        let recommendation = client.generate_execution_recommendation(&signal, None);

        assert!(recommendation.contains("中等信号"));
        assert!(recommendation.contains("宏观数据暂缺"));
    }

    #[test]
    fn test_position_suggestion_very_strong() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::VeryStrong);
        let macro_env = make_test_macro_env(TradingBias::Bullish, 1.2);

        let suggestion = client.generate_position_suggestion(&signal, Some(&macro_env));

        // 15% * 1.2 = 18%
        assert!(suggestion.contains("18%"));
        assert!(suggestion.contains("基础15%"));
        assert!(suggestion.contains("宏观系数1.2"));
    }

    #[test]
    fn test_position_suggestion_weak() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::Weak);
        let macro_env = make_test_macro_env(TradingBias::Neutral, 1.0);

        let suggestion = client.generate_position_suggestion(&signal, Some(&macro_env));

        assert!(suggestion.contains("观望"));
        assert!(suggestion.contains("暂不建仓"));
    }

    #[test]
    fn test_position_suggestion_capped_at_20() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::VeryStrong);
        let macro_env = make_test_macro_env(TradingBias::Bullish, 1.5);

        let suggestion = client.generate_position_suggestion(&signal, Some(&macro_env));

        // 15% * 1.5 = 22.5%, but should be capped at 20%
        assert!(suggestion.contains("20%"));
    }

    #[test]
    fn test_risk_tips_high_risk() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let mut signal = make_test_signal(SignalStrength::Strong);
        signal.entry_price = 10.0;
        signal.stop_loss = 9.0; // 10% risk

        let tips = client.generate_risk_tips(&signal);

        assert!(tips.contains("T+1"));
        assert!(tips.contains("风险较高"));
    }

    #[test]
    fn test_risk_tips_good_rr() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let mut signal = make_test_signal(SignalStrength::Strong);
        signal.entry_price = 10.0;
        signal.stop_loss = 9.7;   // 3% risk
        signal.take_profit = 11.0; // 10% reward, R:R = 3.33:1

        let tips = client.generate_risk_tips(&signal);

        assert!(tips.contains("优质R:R"));
    }

    #[test]
    fn test_urgency_indicator_fresh() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::Strong);

        let urgency = client.generate_urgency_indicator(&signal);

        assert!(urgency.contains("刚产生"));
    }

    #[test]
    fn test_enhanced_message_format() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::Strong);
        let macro_env = make_test_macro_env(TradingBias::Bullish, 1.2);

        let message = client.format_enhanced_signal_message(
            &signal,
            Some(&macro_env),
            "Test recommendation",
            "Test position",
            "Test risk tips",
            "Test urgency",
        );

        // Check all sections are present
        assert!(message.contains("000001.SZ"));  // Base signal
        assert!(message.contains("宏观环境"));    // Macro section
        assert!(message.contains("执行建议"));    // Execution section
        assert!(message.contains("仓位建议"));    // Position section
        assert!(message.contains("风险提示"));    // Risk section
        assert!(message.contains("时效性"));      // Urgency section
        assert!(message.contains("不构成投资建议")); // Disclaimer
    }

    #[test]
    fn test_enhanced_message_without_macro() {
        let config = Config::default();
        let client = NotificationClient::new(&config);
        let signal = make_test_signal(SignalStrength::Strong);

        let message = client.format_enhanced_signal_message(
            &signal,
            None,
            "Test recommendation",
            "Test position",
            "Test risk tips",
            "Test urgency",
        );

        // Macro section should not be present
        assert!(!message.contains("宏观环境"));
        // Other sections should still be present
        assert!(message.contains("执行建议"));
    }
}
