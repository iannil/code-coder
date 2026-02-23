//! Risk control and margin alert system.
//!
//! Based on ZhuRongShuo (祝融说) philosophy concept of "可用余量" (available margin):
//! - Monitors consumption of various "margins" (quotas, budgets, capacity)
//! - Alerts when margins are being depleted too fast
//! - Provides sustainability analysis for decision-making
//!
//! Core principle: "可持续决策 > 最优决策" (Sustainable decisions > Optimal decisions)
//! The system helps maintain the ability to "play another round" rather than
//! optimizing for short-term gains.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// Types
// ============================================================================

/// Category of margin being monitored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarginCategory {
    /// Token usage quota
    TokenQuota,
    /// Financial budget
    Budget,
    /// API rate limit
    RateLimit,
    /// Storage capacity
    Storage,
    /// Compute resources (CPU/Memory)
    Compute,
    /// Time allocation
    Time,
    /// Decision points (remaining choices)
    DecisionCapacity,
    /// Custom margin type
    Custom,
}

impl std::fmt::Display for MarginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenQuota => write!(f, "Token配额"),
            Self::Budget => write!(f, "预算"),
            Self::RateLimit => write!(f, "频率限制"),
            Self::Storage => write!(f, "存储容量"),
            Self::Compute => write!(f, "计算资源"),
            Self::Time => write!(f, "时间"),
            Self::DecisionCapacity => write!(f, "决策余量"),
            Self::Custom => write!(f, "自定义"),
        }
    }
}

/// A margin being monitored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margin {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Category
    pub category: MarginCategory,
    /// Owner (user ID, team ID, or system)
    pub owner: String,
    /// Total capacity (limit)
    pub total: f64,
    /// Current used amount
    pub used: f64,
    /// Unit (e.g., "tokens", "USD", "GB")
    pub unit: String,
    /// Period for reset (daily, weekly, monthly)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_period: Option<ResetPeriod>,
    /// Last reset timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reset: Option<DateTime<Utc>>,
    /// Usage history (for trend analysis)
    #[serde(default)]
    pub history: Vec<MarginSnapshot>,
    /// Custom metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Margin {
    /// Calculate remaining margin.
    pub fn remaining(&self) -> f64 {
        (self.total - self.used).max(0.0)
    }

    /// Calculate usage percentage.
    pub fn usage_percent(&self) -> f64 {
        if self.total <= 0.0 {
            return 100.0;
        }
        (self.used / self.total) * 100.0
    }

    /// Calculate remaining percentage.
    pub fn remaining_percent(&self) -> f64 {
        100.0 - self.usage_percent()
    }

    /// Calculate burn rate (average consumption per hour over recent history).
    pub fn burn_rate(&self) -> Option<f64> {
        if self.history.len() < 2 {
            return None;
        }

        let recent = &self.history[self.history.len().saturating_sub(10)..];
        if recent.len() < 2 {
            return None;
        }

        let first = &recent[0];
        let last = &recent[recent.len() - 1];

        let time_diff = last.timestamp.signed_duration_since(first.timestamp);
        let hours = time_diff.num_minutes() as f64 / 60.0;

        if hours <= 0.0 {
            return None;
        }

        let usage_diff = last.used - first.used;
        Some(usage_diff / hours)
    }

    /// Estimate time until depletion based on burn rate.
    pub fn estimated_depletion(&self) -> Option<ChronoDuration> {
        let rate = self.burn_rate()?;
        if rate <= 0.0 {
            return None; // Not consuming, won't deplete
        }

        let remaining = self.remaining();
        let hours_remaining = remaining / rate;
        Some(ChronoDuration::hours(hours_remaining as i64))
    }
}

/// Reset period for margins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetPeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
    Never,
}

/// A snapshot of margin state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginSnapshot {
    pub timestamp: DateTime<Utc>,
    pub used: f64,
    pub remaining: f64,
}

/// Alert severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    /// Informational - no action needed
    Info = 1,
    /// Warning - attention recommended
    Warning = 2,
    /// Critical - immediate action needed
    Critical = 3,
    /// Emergency - system may become unavailable
    Emergency = 4,
}

impl AlertSeverity {
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Info => "ℹ️",
            Self::Warning => "⚠️",
            Self::Critical => "🔴",
            Self::Emergency => "🆘",
        }
    }
}

/// A risk alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAlert {
    /// Alert ID
    pub id: String,
    /// Margin ID that triggered the alert
    pub margin_id: String,
    /// Margin name
    pub margin_name: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert type
    pub alert_type: RiskAlertType,
    /// Alert message
    pub message: String,
    /// Current usage percentage
    pub usage_percent: f64,
    /// Remaining amount
    pub remaining: f64,
    /// Estimated depletion time (if calculable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_depletion_hours: Option<f64>,
    /// Suggested actions
    #[serde(default)]
    pub suggestions: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether alert has been acknowledged
    #[serde(default)]
    pub acknowledged: bool,
}

/// Type of risk alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskAlertType {
    /// Usage exceeds threshold
    ThresholdExceeded,
    /// Burn rate is too high
    HighBurnRate,
    /// Estimated depletion within warning period
    ImminentDepletion,
    /// Margin fully depleted
    Depleted,
    /// Recovery - margin restored
    Recovered,
}

/// Alert threshold configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    /// Usage percentage for info alert
    pub info: f64,
    /// Usage percentage for warning alert
    pub warning: f64,
    /// Usage percentage for critical alert
    pub critical: f64,
    /// Usage percentage for emergency alert
    pub emergency: f64,
}

impl Default for AlertThreshold {
    fn default() -> Self {
        Self {
            info: 60.0,
            warning: 75.0,
            critical: 90.0,
            emergency: 98.0,
        }
    }
}

// ============================================================================
// Risk Monitor
// ============================================================================

/// Configuration for the risk monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMonitorConfig {
    /// Default alert thresholds
    pub default_thresholds: AlertThreshold,
    /// Custom thresholds per category
    #[serde(default)]
    pub category_thresholds: HashMap<MarginCategory, AlertThreshold>,
    /// Hours before depletion to trigger warning
    pub depletion_warning_hours: f64,
    /// Channels endpoint for notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels_endpoint: Option<String>,
    /// Notification channel type
    #[serde(default = "default_channel_type")]
    pub notification_channel_type: String,
    /// Notification channel ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_channel_id: Option<String>,
}

fn default_channel_type() -> String {
    "feishu".to_string()
}

impl Default for RiskMonitorConfig {
    fn default() -> Self {
        Self {
            default_thresholds: AlertThreshold::default(),
            category_thresholds: HashMap::new(),
            depletion_warning_hours: 24.0,
            channels_endpoint: None,
            notification_channel_type: "feishu".to_string(),
            notification_channel_id: None,
        }
    }
}

/// Risk control monitor.
pub struct RiskMonitor {
    /// HTTP client
    client: reqwest::Client,
    /// Configuration
    config: RiskMonitorConfig,
    /// Tracked margins
    margins: Arc<tokio::sync::RwLock<HashMap<String, Margin>>>,
    /// Alert history
    alerts: Arc<tokio::sync::RwLock<Vec<RiskAlert>>>,
    /// Last alert sent per margin (to avoid spam)
    last_alerts: Arc<tokio::sync::RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl RiskMonitor {
    /// Create a new risk monitor.
    pub fn new(config: RiskMonitorConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            config,
            margins: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            alerts: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            last_alerts: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Register or update a margin.
    pub async fn register_margin(&self, margin: Margin) {
        let mut margins = self.margins.write().await;
        margins.insert(margin.id.clone(), margin);
    }

    /// Update margin usage.
    pub async fn update_usage(&self, margin_id: &str, new_used: f64) -> Option<RiskAlert> {
        let mut margins = self.margins.write().await;
        let margin = margins.get_mut(margin_id)?;

        // Update usage
        margin.used = new_used;

        // Add to history
        margin.history.push(MarginSnapshot {
            timestamp: Utc::now(),
            used: margin.used,
            remaining: margin.remaining(),
        });

        // Keep only last 1000 snapshots
        if margin.history.len() > 1000 {
            margin.history.remove(0);
        }

        // Check for alerts
        let margin_clone = margin.clone();
        drop(margins);

        self.check_margin(&margin_clone).await
    }

    /// Check a margin and generate alert if needed.
    async fn check_margin(&self, margin: &Margin) -> Option<RiskAlert> {
        let threshold = self.get_threshold(margin.category);
        let usage_pct = margin.usage_percent();
        let remaining = margin.remaining();

        // Determine severity based on usage
        let (severity, alert_type) = if usage_pct >= threshold.emergency {
            (AlertSeverity::Emergency, RiskAlertType::ThresholdExceeded)
        } else if usage_pct >= threshold.critical {
            (AlertSeverity::Critical, RiskAlertType::ThresholdExceeded)
        } else if usage_pct >= threshold.warning {
            (AlertSeverity::Warning, RiskAlertType::ThresholdExceeded)
        } else if usage_pct >= threshold.info {
            // Check for imminent depletion
            if let Some(depletion) = margin.estimated_depletion() {
                if depletion.num_hours() < self.config.depletion_warning_hours as i64 {
                    (AlertSeverity::Warning, RiskAlertType::ImminentDepletion)
                } else {
                    return None; // No alert needed
                }
            } else {
                return None;
            }
        } else {
            return None; // No alert needed
        };

        // Check cooldown (don't spam alerts)
        let should_alert = {
            let last_alerts = self.last_alerts.read().await;
            if let Some(last_time) = last_alerts.get(&margin.id) {
                let cooldown = match severity {
                    AlertSeverity::Emergency => ChronoDuration::minutes(5),
                    AlertSeverity::Critical => ChronoDuration::minutes(15),
                    AlertSeverity::Warning => ChronoDuration::hours(1),
                    AlertSeverity::Info => ChronoDuration::hours(4),
                };
                Utc::now().signed_duration_since(*last_time) > cooldown
            } else {
                true
            }
        };

        if !should_alert {
            return None;
        }

        // Build suggestions based on ZhuRongShuo philosophy
        let suggestions = self.generate_suggestions(margin, severity);

        let alert = RiskAlert {
            id: uuid::Uuid::new_v4().to_string(),
            margin_id: margin.id.clone(),
            margin_name: margin.name.clone(),
            severity,
            alert_type,
            message: self.format_alert_message(margin, severity, alert_type),
            usage_percent: usage_pct,
            remaining,
            estimated_depletion_hours: margin.estimated_depletion().map(|d| d.num_hours() as f64),
            suggestions,
            timestamp: Utc::now(),
            acknowledged: false,
        };

        // Store alert
        {
            let mut alerts = self.alerts.write().await;
            alerts.push(alert.clone());
            if alerts.len() > 1000 {
                alerts.remove(0);
            }
        }

        // Update last alert time
        {
            let mut last_alerts = self.last_alerts.write().await;
            last_alerts.insert(margin.id.clone(), Utc::now());
        }

        Some(alert)
    }

    /// Get threshold for a category.
    fn get_threshold(&self, category: MarginCategory) -> &AlertThreshold {
        self.config
            .category_thresholds
            .get(&category)
            .unwrap_or(&self.config.default_thresholds)
    }

    /// Format alert message.
    fn format_alert_message(
        &self,
        margin: &Margin,
        severity: AlertSeverity,
        alert_type: RiskAlertType,
    ) -> String {
        let base_msg = match alert_type {
            RiskAlertType::ThresholdExceeded => format!(
                "{}「{}」使用率达到 {:.1}%，剩余 {:.1} {}",
                margin.category,
                margin.name,
                margin.usage_percent(),
                margin.remaining(),
                margin.unit
            ),
            RiskAlertType::HighBurnRate => format!(
                "{}「{}」消耗速率过高，预计 {:.1} 小时后耗尽",
                margin.category,
                margin.name,
                margin.estimated_depletion().map(|d| d.num_hours()).unwrap_or(0)
            ),
            RiskAlertType::ImminentDepletion => format!(
                "{}「{}」即将耗尽，预计剩余 {:.1} 小时",
                margin.category,
                margin.name,
                margin.estimated_depletion().map(|d| d.num_hours()).unwrap_or(0)
            ),
            RiskAlertType::Depleted => format!(
                "{}「{}」已完全耗尽",
                margin.category, margin.name
            ),
            RiskAlertType::Recovered => format!(
                "{}「{}」已恢复，当前剩余 {:.1}%",
                margin.category,
                margin.name,
                margin.remaining_percent()
            ),
        };

        format!("{} {}", severity.emoji(), base_msg)
    }

    /// Generate suggestions based on ZhuRongShuo philosophy.
    fn generate_suggestions(&self, margin: &Margin, severity: AlertSeverity) -> Vec<String> {
        let mut suggestions = Vec::new();

        // ZhuRongShuo core principle: maintain ability to "play another round"
        match severity {
            AlertSeverity::Emergency | AlertSeverity::Critical => {
                suggestions.push("⚡ 建议立即降低消耗速率或增加配额".to_string());
                suggestions.push("🔄 考虑暂停非关键任务以保留余量".to_string());
                suggestions.push("📊 复盘：是否存在可优化的消耗模式？".to_string());
            }
            AlertSeverity::Warning => {
                suggestions.push("📈 建议关注消耗趋势，提前规划".to_string());
                suggestions.push("🎯 评估当前任务的必要性与收益".to_string());
            }
            AlertSeverity::Info => {
                suggestions.push("📋 记录当前状态以便后续分析".to_string());
            }
        }

        // Category-specific suggestions
        match margin.category {
            MarginCategory::TokenQuota => {
                suggestions.push("💡 考虑使用更轻量的模型完成简单任务".to_string());
            }
            MarginCategory::Budget => {
                suggestions.push("💰 检查高成本操作是否有替代方案".to_string());
            }
            MarginCategory::DecisionCapacity => {
                suggestions.push("🧠 「祝融说」：保持可选择性比追求最优更重要".to_string());
            }
            _ => {}
        }

        suggestions
    }

    /// Check all registered margins.
    pub async fn check_all(&self) -> Vec<RiskAlert> {
        let margins = self.margins.read().await;
        let margin_list: Vec<Margin> = margins.values().cloned().collect();
        drop(margins);

        let mut alerts = Vec::new();
        for margin in margin_list {
            if let Some(alert) = self.check_margin(&margin).await {
                alerts.push(alert);
            }
        }
        alerts
    }

    /// Get all margins.
    pub async fn get_margins(&self) -> Vec<Margin> {
        let margins = self.margins.read().await;
        margins.values().cloned().collect()
    }

    /// Get a specific margin.
    pub async fn get_margin(&self, id: &str) -> Option<Margin> {
        let margins = self.margins.read().await;
        margins.get(id).cloned()
    }

    /// Get recent alerts.
    pub async fn get_alerts(&self, limit: usize) -> Vec<RiskAlert> {
        let alerts = self.alerts.read().await;
        alerts.iter().rev().take(limit).cloned().collect()
    }

    /// Acknowledge an alert.
    pub async fn acknowledge_alert(&self, alert_id: &str) -> bool {
        let mut alerts = self.alerts.write().await;
        for alert in alerts.iter_mut() {
            if alert.id == alert_id {
                alert.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// Send alert notification to IM.
    pub async fn send_notification(&self, alert: &RiskAlert) -> Result<()> {
        let endpoint = self
            .config
            .channels_endpoint
            .as_ref()
            .context("Channels endpoint not configured")?;

        let channel_id = self
            .config
            .notification_channel_id
            .as_ref()
            .context("Notification channel ID not configured")?;

        let mut message = format!(
            "{}\n\n📊 使用率: {:.1}%\n📉 剩余: {:.1}\n",
            alert.message, alert.usage_percent, alert.remaining
        );

        if let Some(hours) = alert.estimated_depletion_hours {
            message.push_str(&format!("⏱️ 预计耗尽: {:.1} 小时后\n", hours));
        }

        if !alert.suggestions.is_empty() {
            message.push_str("\n💡 建议:\n");
            for suggestion in &alert.suggestions {
                message.push_str(&format!("  {}\n", suggestion));
            }
        }

        let url = format!("{}/api/v1/send", endpoint);
        let body = serde_json::json!({
            "channel_type": self.config.notification_channel_type,
            "channel_id": channel_id,
            "content": {
                "type": "markdown",
                "text": message
            }
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to send notification: {} - {}", status, body);
        }

        Ok(())
    }

    /// Load margins from metering API.
    pub async fn load_from_metering(&self, metering_endpoint: &str) -> Result<()> {
        let url = format!("{}/api/v1/metering/users", metering_endpoint);

        let response: MeteringUsersResponse = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch metering data")?
            .json()
            .await
            .context("Failed to parse metering response")?;

        for user in response.users {
            let margin = Margin {
                id: format!("token-{}", user.user_id),
                name: format!("Token配额 ({})", user.name.as_deref().unwrap_or(&user.user_id)),
                category: MarginCategory::TokenQuota,
                owner: user.user_id.clone(),
                total: (user.quota.daily_input_limit + user.quota.daily_output_limit) as f64,
                used: (user.daily_usage.input_tokens + user.daily_usage.output_tokens) as f64,
                unit: "tokens".to_string(),
                reset_period: Some(ResetPeriod::Daily),
                last_reset: None,
                history: Vec::new(),
                metadata: HashMap::new(),
            };

            self.register_margin(margin).await;
        }

        Ok(())
    }
}

// ============================================================================
// API Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct MeteringUsersResponse {
    users: Vec<MeteringUser>,
}

#[derive(Debug, Deserialize)]
struct MeteringUser {
    user_id: String,
    name: Option<String>,
    daily_usage: MeteringUsage,
    quota: MeteringQuota,
}

#[derive(Debug, Deserialize)]
struct MeteringUsage {
    input_tokens: i64,
    output_tokens: i64,
}

#[derive(Debug, Deserialize)]
struct MeteringQuota {
    daily_input_limit: i64,
    daily_output_limit: i64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_margin_calculations() {
        let margin = Margin {
            id: "test".to_string(),
            name: "Test Margin".to_string(),
            category: MarginCategory::TokenQuota,
            owner: "user-1".to_string(),
            total: 1000.0,
            used: 750.0,
            unit: "tokens".to_string(),
            reset_period: Some(ResetPeriod::Daily),
            last_reset: None,
            history: Vec::new(),
            metadata: HashMap::new(),
        };

        assert_eq!(margin.remaining(), 250.0);
        assert_eq!(margin.usage_percent(), 75.0);
        assert_eq!(margin.remaining_percent(), 25.0);
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Emergency > AlertSeverity::Critical);
        assert!(AlertSeverity::Critical > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }

    #[test]
    fn test_margin_category_display() {
        assert_eq!(MarginCategory::TokenQuota.to_string(), "Token配额");
        assert_eq!(MarginCategory::Budget.to_string(), "预算");
        assert_eq!(MarginCategory::DecisionCapacity.to_string(), "决策余量");
    }

    #[test]
    fn test_default_thresholds() {
        let threshold = AlertThreshold::default();
        assert_eq!(threshold.info, 60.0);
        assert_eq!(threshold.warning, 75.0);
        assert_eq!(threshold.critical, 90.0);
        assert_eq!(threshold.emergency, 98.0);
    }

    #[test]
    fn test_reset_period_serialization() {
        let period = ResetPeriod::Daily;
        let json = serde_json::to_string(&period).unwrap();
        assert_eq!(json, "\"daily\"");
    }

    #[test]
    fn test_risk_alert_serialization() {
        let alert = RiskAlert {
            id: "test-123".to_string(),
            margin_id: "margin-1".to_string(),
            margin_name: "Token配额".to_string(),
            severity: AlertSeverity::Warning,
            alert_type: RiskAlertType::ThresholdExceeded,
            message: "使用率达到 80%".to_string(),
            usage_percent: 80.0,
            remaining: 200.0,
            estimated_depletion_hours: Some(5.0),
            suggestions: vec!["降低消耗".to_string()],
            timestamp: Utc::now(),
            acknowledged: false,
        };

        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("\"severity\":\"warning\""));
        assert!(json.contains("\"alert_type\":\"threshold_exceeded\""));
    }

    #[tokio::test]
    async fn test_risk_monitor_register_and_check() {
        let config = RiskMonitorConfig::default();
        let monitor = RiskMonitor::new(config);

        let margin = Margin {
            id: "test-margin".to_string(),
            name: "Test".to_string(),
            category: MarginCategory::TokenQuota,
            owner: "user-1".to_string(),
            total: 100.0,
            used: 80.0, // 80% - should trigger warning
            unit: "tokens".to_string(),
            reset_period: None,
            last_reset: None,
            history: Vec::new(),
            metadata: HashMap::new(),
        };

        monitor.register_margin(margin).await;

        let margins = monitor.get_margins().await;
        assert_eq!(margins.len(), 1);
        assert_eq!(margins[0].usage_percent(), 80.0);

        let alerts = monitor.check_all().await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
    }
}
