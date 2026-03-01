//! Alert System for Zero CLI Daemon
//!
//! Provides lightweight alerting based on metrics thresholds.
//! Alerts are sent via zero-channels (Telegram, Discord, Slack).
//!
//! # Features
//!
//! - Rule-based alert configuration (JSON)
//! - Multiple severity levels (critical, warning, info)
//! - Alert silencing to prevent alert storms
//! - Integration with zero-channels for notifications
//!
//! # Configuration
//!
//! Alerts are configured in `~/.codecoder/alerts.json`:
//!
//! ```json
//! {
//!   "default_channel": "telegram",
//!   "rules": [
//!     {
//!       "name": "high_error_rate",
//!       "condition": "error_rate > 5",
//!       "duration_secs": 60,
//!       "severity": "critical"
//!     }
//!   ]
//! }
//! ```

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Alert check interval in seconds.
const ALERT_CHECK_INTERVAL_SECS: u64 = 10;

/// Default silence duration for repeated alerts (5 minutes).
const DEFAULT_SILENCE_DURATION_SECS: i64 = 300;

// ============================================================================
// Configuration Types
// ============================================================================

/// Alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl Severity {
    /// Get emoji for the severity level.
    pub fn emoji(&self) -> &'static str {
        match self {
            Severity::Critical => "🔴",
            Severity::Warning => "🟡",
            Severity::Info => "🔵",
        }
    }

    /// Get uppercase label.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::Warning => "WARNING",
            Severity::Info => "INFO",
        }
    }
}

/// Alert condition types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    /// Error rate exceeds threshold (percentage).
    ErrorRate,
    /// P99 latency exceeds threshold (milliseconds).
    P99Latency,
    /// P95 latency exceeds threshold (milliseconds).
    P95Latency,
    /// Service health status is not OK.
    ServiceDown,
    /// Memory usage exceeds threshold (bytes).
    MemoryUsage,
}

/// Alert rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique name for this rule.
    pub name: String,
    /// Condition type.
    pub condition: ConditionType,
    /// Threshold value for the condition.
    pub threshold: f64,
    /// Duration the condition must be true before alerting (seconds).
    #[serde(default = "default_duration")]
    pub duration_secs: u64,
    /// Alert severity.
    pub severity: Severity,
    /// Optional service filter (if empty, applies to all services).
    #[serde(default)]
    pub services: Vec<String>,
    /// Whether this rule is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_duration() -> u64 {
    60
}

fn default_enabled() -> bool {
    true
}

/// Alert configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Default channel for sending alerts.
    #[serde(default = "default_channel")]
    pub default_channel: String,
    /// Silence duration for repeated alerts (seconds).
    #[serde(default = "default_silence")]
    pub silence_duration_secs: i64,
    /// Alert rules.
    #[serde(default)]
    pub rules: Vec<AlertRule>,
    /// Zero-channels endpoint.
    #[serde(default = "default_channels_endpoint")]
    pub channels_endpoint: String,
}

fn default_channel() -> String {
    "telegram".to_string()
}

fn default_silence() -> i64 {
    DEFAULT_SILENCE_DURATION_SECS
}

fn default_channels_endpoint() -> String {
    "http://127.0.0.1:4431".to_string()
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            default_channel: default_channel(),
            silence_duration_secs: default_silence(),
            rules: vec![
                AlertRule {
                    name: "high_error_rate".to_string(),
                    condition: ConditionType::ErrorRate,
                    threshold: 5.0,
                    duration_secs: 60,
                    severity: Severity::Critical,
                    services: vec![],
                    enabled: true,
                },
                AlertRule {
                    name: "high_latency".to_string(),
                    condition: ConditionType::P99Latency,
                    threshold: 2000.0,
                    duration_secs: 30,
                    severity: Severity::Warning,
                    services: vec![],
                    enabled: true,
                },
                AlertRule {
                    name: "service_down".to_string(),
                    condition: ConditionType::ServiceDown,
                    threshold: 0.0,
                    duration_secs: 10,
                    severity: Severity::Critical,
                    services: vec![],
                    enabled: true,
                },
            ],
            channels_endpoint: default_channels_endpoint(),
        }
    }
}

impl AlertConfig {
    /// Load config from file or return default.
    pub fn load(path: &PathBuf) -> Self {
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save config to file.
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

// ============================================================================
// Metrics Snapshot (from services)
// ============================================================================

/// Metrics snapshot from a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub service: String,
    pub total_requests: u64,
    pub error_requests: u64,
    pub error_rate: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub active_connections: u64,
    pub memory_bytes: u64,
    pub uptime_secs: u64,
    #[serde(default)]
    pub status: Option<String>,
}

// ============================================================================
// Alert State
// ============================================================================

/// State for a single rule (tracks when condition first became true).
#[derive(Debug, Clone)]
struct RuleState {
    /// When the condition first became true.
    condition_start: Option<DateTime<Utc>>,
    /// Last time an alert was fired.
    last_alert: Option<DateTime<Utc>>,
}

impl Default for RuleState {
    fn default() -> Self {
        Self {
            condition_start: None,
            last_alert: None,
        }
    }
}

/// Alert fired event.
#[derive(Debug, Clone, Serialize)]
pub struct AlertEvent {
    pub rule_name: String,
    pub severity: Severity,
    pub service: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub threshold: f64,
}

// ============================================================================
// Alert Engine
// ============================================================================

/// Alert engine that checks rules and fires alerts.
pub struct AlertEngine {
    config: AlertConfig,
    state: HashMap<String, RuleState>,
    http_client: reqwest::Client,
}

impl AlertEngine {
    /// Create a new alert engine with the given configuration.
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            state: HashMap::new(),
            http_client: reqwest::Client::new(),
        }
    }

    /// Fetch metrics from a service.
    async fn fetch_metrics(&self, _service: &str, port: u16) -> Option<MetricsSnapshot> {
        let url = format!("http://127.0.0.1:{}/api/v1/metrics", port);
        match self.http_client.get(&url).timeout(std::time::Duration::from_secs(2)).send().await {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<MetricsSnapshot>().await.ok()
            }
            _ => None,
        }
    }

    /// Check all rules against current metrics.
    pub async fn check_rules(&mut self) -> Vec<AlertEvent> {
        let mut alerts = vec![];

        // Service port mappings
        let service_ports = [
            ("ccode-api", 4400u16),
            ("zero-gateway", 4430),
            ("zero-channels", 4431),
        ];

        for (service, port) in service_ports {
            let metrics = self.fetch_metrics(service, port).await;

            for rule in &self.config.rules {
                if !rule.enabled {
                    continue;
                }

                // Check service filter
                if !rule.services.is_empty() && !rule.services.contains(&service.to_string()) {
                    continue;
                }

                let rule_key = format!("{}:{}", rule.name, service);
                let state = self.state.entry(rule_key.clone()).or_default();

                // Evaluate condition
                let (condition_met, current_value) = match &rule.condition {
                    ConditionType::ErrorRate => {
                        if let Some(ref m) = metrics {
                            (m.error_rate > rule.threshold, m.error_rate)
                        } else {
                            (false, 0.0)
                        }
                    }
                    ConditionType::P99Latency => {
                        if let Some(ref m) = metrics {
                            (m.p99_ms > rule.threshold, m.p99_ms)
                        } else {
                            (false, 0.0)
                        }
                    }
                    ConditionType::P95Latency => {
                        if let Some(ref m) = metrics {
                            (m.p95_ms > rule.threshold, m.p95_ms)
                        } else {
                            (false, 0.0)
                        }
                    }
                    ConditionType::ServiceDown => {
                        let is_down = metrics.is_none();
                        (is_down, if is_down { 1.0 } else { 0.0 })
                    }
                    ConditionType::MemoryUsage => {
                        if let Some(ref m) = metrics {
                            (m.memory_bytes as f64 > rule.threshold, m.memory_bytes as f64)
                        } else {
                            (false, 0.0)
                        }
                    }
                };

                let now = Utc::now();

                if condition_met {
                    // Update condition start time
                    if state.condition_start.is_none() {
                        state.condition_start = Some(now);
                    }

                    // Check if duration threshold is met
                    if let Some(start) = state.condition_start {
                        let elapsed = (now - start).num_seconds() as u64;
                        if elapsed >= rule.duration_secs {
                            // Check silence period
                            let should_alert = state.last_alert
                                .map(|last| (now - last).num_seconds() >= self.config.silence_duration_secs)
                                .unwrap_or(true);

                            if should_alert {
                                let message = format_alert_message(
                                    &rule.condition,
                                    service,
                                    current_value,
                                    rule.threshold,
                                );

                                alerts.push(AlertEvent {
                                    rule_name: rule.name.clone(),
                                    severity: rule.severity,
                                    service: service.to_string(),
                                    message,
                                    timestamp: now,
                                    value: current_value,
                                    threshold: rule.threshold,
                                });

                                state.last_alert = Some(now);
                            }
                        }
                    }
                } else {
                    // Reset condition start when condition is no longer met
                    state.condition_start = None;
                }
            }
        }

        alerts
    }

    /// Send alerts via zero-channels.
    pub async fn send_alerts(&self, alerts: &[AlertEvent]) -> Result<()> {
        for alert in alerts {
            let message = format!(
                "{} {}: {}\n\nService: {}\nValue: {:.2}\nThreshold: {:.2}\nTime: {}",
                alert.severity.emoji(),
                alert.severity.label(),
                alert.rule_name,
                alert.service,
                alert.value,
                alert.threshold,
                alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            );

            // Send via zero-channels
            let url = format!("{}/api/v1/send", self.config.channels_endpoint);
            let payload = serde_json::json!({
                "channel": self.config.default_channel,
                "message": message,
            });

            match self.http_client.post(&url)
                .json(&payload)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!(
                        rule = %alert.rule_name,
                        service = %alert.service,
                        severity = ?alert.severity,
                        "Alert sent successfully"
                    );
                }
                Ok(resp) => {
                    tracing::warn!(
                        rule = %alert.rule_name,
                        status = %resp.status(),
                        "Failed to send alert"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        rule = %alert.rule_name,
                        error = %e,
                        "Failed to send alert"
                    );
                }
            }
        }

        Ok(())
    }
}

/// Format alert message based on condition type.
fn format_alert_message(
    condition: &ConditionType,
    service: &str,
    value: f64,
    threshold: f64,
) -> String {
    match condition {
        ConditionType::ErrorRate => {
            format!(
                "Error rate for {} is {:.1}% (threshold: {:.1}%)",
                service, value, threshold
            )
        }
        ConditionType::P99Latency => {
            format!(
                "P99 latency for {} is {:.0}ms (threshold: {:.0}ms)",
                service, value, threshold
            )
        }
        ConditionType::P95Latency => {
            format!(
                "P95 latency for {} is {:.0}ms (threshold: {:.0}ms)",
                service, value, threshold
            )
        }
        ConditionType::ServiceDown => {
            format!("Service {} is DOWN (unreachable)", service)
        }
        ConditionType::MemoryUsage => {
            let value_mb = value / 1_048_576.0;
            let threshold_mb = threshold / 1_048_576.0;
            format!(
                "Memory usage for {} is {:.0}MB (threshold: {:.0}MB)",
                service, value_mb, threshold_mb
            )
        }
    }
}

// ============================================================================
// Alert Worker
// ============================================================================

/// Run the alert worker in the background.
pub async fn run_alert_worker(config_dir: PathBuf) -> Result<()> {
    let config_path = config_dir.join("alerts.json");
    let config = AlertConfig::load(&config_path);

    // Save default config if it doesn't exist
    if !config_path.exists() {
        if let Err(e) = config.save(&config_path) {
            tracing::warn!("Failed to save default alert config: {}", e);
        }
    }

    tracing::info!(
        rules = config.rules.len(),
        channel = %config.default_channel,
        "Alert worker started"
    );

    let mut engine = AlertEngine::new(config);

    loop {
        // Check rules
        let alerts = engine.check_rules().await;

        // Send alerts
        if !alerts.is_empty() {
            if let Err(e) = engine.send_alerts(&alerts).await {
                tracing::error!(error = %e, "Failed to send alerts");
            }
        }

        // Wait for next check
        tokio::time::sleep(std::time::Duration::from_secs(ALERT_CHECK_INTERVAL_SECS)).await;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_emoji() {
        assert_eq!(Severity::Critical.emoji(), "🔴");
        assert_eq!(Severity::Warning.emoji(), "🟡");
        assert_eq!(Severity::Info.emoji(), "🔵");
    }

    #[test]
    fn test_default_config() {
        let config = AlertConfig::default();
        assert_eq!(config.default_channel, "telegram");
        assert_eq!(config.rules.len(), 3);
    }

    #[test]
    fn test_config_serialization() {
        let config = AlertConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: AlertConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.default_channel, config.default_channel);
        assert_eq!(parsed.rules.len(), config.rules.len());
    }
}
