//! Macro report generation and scheduling.
//!
//! Provides scheduled generation of weekly and monthly macro reports
//! with Telegram notification integration.

use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use super::bridge::{AgentBridge, AgentBridgeConfig};
use super::types::{MacroReport, ReportType};
use crate::notification::NotificationClient;

/// Configuration for the report generator.
#[derive(Debug, Clone)]
pub struct ReportGeneratorConfig {
    /// Enable weekly reports
    pub weekly_enabled: bool,
    /// Weekly report cron expression (default: "0 9 * * 1" = Monday 9 AM)
    pub weekly_cron: String,
    /// Enable monthly reports
    pub monthly_enabled: bool,
    /// Monthly report cron expression (default: "0 9 1 * *" = 1st day 9 AM)
    pub monthly_cron: String,
}

impl Default for ReportGeneratorConfig {
    fn default() -> Self {
        Self {
            weekly_enabled: true,
            weekly_cron: "0 9 * * 1".to_string(),
            monthly_enabled: true,
            monthly_cron: "0 9 1 * *".to_string(),
        }
    }
}

/// State for tracking report generation.
struct ReportState {
    last_weekly: Option<DateTime<Utc>>,
    last_monthly: Option<DateTime<Utc>>,
}

/// Macro report generator with scheduling support.
pub struct MacroReportGenerator {
    /// Agent bridge for report generation
    agent_bridge: AgentBridge,
    /// Notification client for sending reports
    notification: Arc<NotificationClient>,
    /// Configuration
    config: ReportGeneratorConfig,
    /// State tracking
    state: RwLock<ReportState>,
}

impl MacroReportGenerator {
    /// Create a new report generator.
    pub fn new(
        agent_config: AgentBridgeConfig,
        notification: Arc<NotificationClient>,
        config: ReportGeneratorConfig,
    ) -> Self {
        Self {
            agent_bridge: AgentBridge::new(agent_config),
            notification,
            config,
            state: RwLock::new(ReportState {
                last_weekly: None,
                last_monthly: None,
            }),
        }
    }

    /// Start the report scheduler.
    ///
    /// This spawns background tasks that check for scheduled report times
    /// and generate/send reports accordingly.
    pub async fn start(&self) -> Result<()> {
        info!(
            weekly_enabled = self.config.weekly_enabled,
            monthly_enabled = self.config.monthly_enabled,
            "Starting macro report scheduler"
        );

        // For now, we'll use a simple polling approach
        // In production, consider using a proper cron scheduler like tokio-cron-scheduler
        loop {
            self.check_and_generate_reports().await;
            // Sleep for 1 minute before checking again
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }

    /// Check if any scheduled reports need to be generated.
    async fn check_and_generate_reports(&self) {
        let now = Utc::now();

        // Check weekly report (Monday 9 AM Beijing time)
        if self.config.weekly_enabled && self.should_generate_weekly(&now).await {
            if let Err(e) = self.generate_and_send(ReportType::Weekly).await {
                error!(error = %e, "Failed to generate weekly report");
            }
        }

        // Check monthly report (1st day 9 AM Beijing time)
        if self.config.monthly_enabled && self.should_generate_monthly(&now).await {
            if let Err(e) = self.generate_and_send(ReportType::Monthly).await {
                error!(error = %e, "Failed to generate monthly report");
            }
        }
    }

    /// Check if weekly report should be generated.
    async fn should_generate_weekly(&self, now: &DateTime<Utc>) -> bool {
        use chrono::{Datelike, Timelike};

        // Convert to Beijing time (UTC+8)
        let beijing = now.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());

        // Check if it's Monday 9:00-9:59 Beijing time
        if beijing.weekday() != chrono::Weekday::Mon {
            return false;
        }
        if beijing.hour() != 9 {
            return false;
        }

        // Check if we already generated this week
        let state = self.state.read().await;
        if let Some(last) = state.last_weekly {
            // Don't generate if we already did today
            if last.date_naive() == now.date_naive() {
                return false;
            }
        }

        true
    }

    /// Check if monthly report should be generated.
    async fn should_generate_monthly(&self, now: &DateTime<Utc>) -> bool {
        use chrono::{Datelike, Timelike};

        // Convert to Beijing time
        let beijing = now.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());

        // Check if it's 1st day 9:00-9:59 Beijing time
        if beijing.day() != 1 {
            return false;
        }
        if beijing.hour() != 9 {
            return false;
        }

        // Check if we already generated this month
        let state = self.state.read().await;
        if let Some(last) = state.last_monthly {
            let last_beijing = last.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
            // Don't generate if we already did this month
            if last_beijing.month() == beijing.month() && last_beijing.year() == beijing.year() {
                return false;
            }
        }

        true
    }

    /// Generate and send a report.
    async fn generate_and_send(&self, report_type: ReportType) -> Result<()> {
        info!(report_type = %report_type, "Generating macro report");

        // Generate report via agent
        let content = self.agent_bridge.generate_report(report_type).await?;

        // Build report structure
        let report = MacroReport {
            report_type,
            title: format!("📊 {}宏观经济简报", report_type),
            period: self.get_period_description(report_type),
            content: content.clone(),
            highlights: self.extract_highlights(&content),
            generated_at: Utc::now(),
        };

        // Send to Telegram
        let message = self.format_telegram_message(&report);
        self.notification.send_alert(&report.title, &message).await?;

        // Update state
        self.update_state(report_type).await;

        info!(report_type = %report_type, "Macro report sent successfully");
        Ok(())
    }

    /// Get period description for the report.
    fn get_period_description(&self, report_type: ReportType) -> String {
        let now = Utc::now();
        let beijing = now.with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());

        match report_type {
            ReportType::Weekly => {
                let week_start = beijing - chrono::Duration::days(7);
                format!(
                    "{} - {}",
                    week_start.format("%Y-%m-%d"),
                    beijing.format("%Y-%m-%d")
                )
            }
            ReportType::Monthly => {
                let prev_month = beijing - chrono::Duration::days(beijing.day() as i64);
                format!("{}", prev_month.format("%Y年%m月"))
            }
            ReportType::AdHoc => beijing.format("%Y-%m-%d %H:%M").to_string(),
        }
    }

    /// Extract key highlights from report content.
    fn extract_highlights(&self, content: &str) -> Vec<String> {
        let mut highlights = Vec::new();

        // Simple extraction: look for bullet points or numbered items
        for line in content.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("• "))
                && trimmed.len() > 10
                && trimmed.len() < 100
            {
                let highlight = trimmed
                    .trim_start_matches("- ")
                    .trim_start_matches("* ")
                    .trim_start_matches("• ")
                    .to_string();
                highlights.push(highlight);

                if highlights.len() >= 5 {
                    break;
                }
            }
        }

        highlights
    }

    /// Format the report for Telegram.
    fn format_telegram_message(&self, report: &MacroReport) -> String {
        let mut message = String::new();

        // Title and period
        message.push_str(&format!("*{}*\n", report.title));
        message.push_str(&format!("📅 {}\n\n", report.period));

        // Content (truncate if too long)
        let content = if report.content.len() > 3500 {
            format!("{}...\n\n_[报告已截断]_", &report.content[..3500])
        } else {
            report.content.clone()
        };
        message.push_str(&content);

        // Highlights
        if !report.highlights.is_empty() {
            message.push_str("\n\n*要点摘要:*\n");
            for highlight in &report.highlights {
                message.push_str(&format!("• {}\n", highlight));
            }
        }

        // Timestamp
        let beijing = report.generated_at
            .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap());
        message.push_str(&format!("\n\n_生成时间: {}_", beijing.format("%Y-%m-%d %H:%M")));

        message
    }

    /// Update state after generating a report.
    async fn update_state(&self, report_type: ReportType) {
        let mut state = self.state.write().await;
        let now = Utc::now();

        match report_type {
            ReportType::Weekly => state.last_weekly = Some(now),
            ReportType::Monthly => state.last_monthly = Some(now),
            ReportType::AdHoc => {}
        }
    }

    /// Generate an ad-hoc report immediately.
    pub async fn generate_adhoc_report(&self) -> Result<MacroReport> {
        info!("Generating ad-hoc macro report");

        let content = self.agent_bridge.generate_report(ReportType::AdHoc).await?;

        let report = MacroReport {
            report_type: ReportType::AdHoc,
            title: "📊 即时宏观分析".to_string(),
            period: self.get_period_description(ReportType::AdHoc),
            content: content.clone(),
            highlights: self.extract_highlights(&content),
            generated_at: Utc::now(),
        };

        Ok(report)
    }

    /// Generate and send an ad-hoc report.
    pub async fn send_adhoc_report(&self) -> Result<()> {
        let report = self.generate_adhoc_report().await?;
        let message = self.format_telegram_message(&report);
        self.notification.send_alert(&report.title, &message).await?;
        Ok(())
    }

    /// Check if the agent is available for report generation.
    pub async fn is_available(&self) -> bool {
        self.agent_bridge.health_check().await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generator_config_default() {
        let config = ReportGeneratorConfig::default();
        assert!(config.weekly_enabled);
        assert!(config.monthly_enabled);
        assert_eq!(config.weekly_cron, "0 9 * * 1");
        assert_eq!(config.monthly_cron, "0 9 1 * *");
    }

    #[test]
    fn test_report_type_display() {
        assert_eq!(ReportType::Weekly.to_string(), "周度");
        assert_eq!(ReportType::Monthly.to_string(), "月度");
        assert_eq!(ReportType::AdHoc.to_string(), "即时");
    }

    #[test]
    fn test_extract_highlights() {
        let notification = Arc::new(NotificationClient::new(&zero_common::config::Config::default()));
        let generator = MacroReportGenerator::new(
            AgentBridgeConfig::default(),
            notification,
            ReportGeneratorConfig::default(),
        );

        let content = r#"
## 本周要点

- PMI连续三个月处于荣枯线以上，显示制造业持续扩张
- M2增速回升至10.2%，流动性环境有所改善
- 社融数据超预期，信贷需求回暖

## 详细分析
这是详细分析内容...
"#;

        let highlights = generator.extract_highlights(content);
        assert_eq!(highlights.len(), 3);
        assert!(highlights[0].contains("PMI"));
    }

    #[test]
    fn test_period_description() {
        let notification = Arc::new(NotificationClient::new(&zero_common::config::Config::default()));
        let generator = MacroReportGenerator::new(
            AgentBridgeConfig::default(),
            notification,
            ReportGeneratorConfig::default(),
        );

        let weekly = generator.get_period_description(ReportType::Weekly);
        assert!(weekly.contains("-"));

        let adhoc = generator.get_period_description(ReportType::AdHoc);
        assert!(adhoc.contains(":"));
    }

    #[test]
    fn test_format_telegram_message() {
        let notification = Arc::new(NotificationClient::new(&zero_common::config::Config::default()));
        let generator = MacroReportGenerator::new(
            AgentBridgeConfig::default(),
            notification,
            ReportGeneratorConfig::default(),
        );

        let report = MacroReport {
            report_type: ReportType::Weekly,
            title: "测试报告".to_string(),
            period: "2024-01-01 - 2024-01-07".to_string(),
            content: "这是报告内容".to_string(),
            highlights: vec!["要点1".to_string(), "要点2".to_string()],
            generated_at: Utc::now(),
        };

        let message = generator.format_telegram_message(&report);
        assert!(message.contains("测试报告"));
        assert!(message.contains("2024-01"));
        assert!(message.contains("要点1"));
    }
}
