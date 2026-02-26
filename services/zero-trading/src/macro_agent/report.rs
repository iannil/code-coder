//! Macro report generation and scheduling.
//!
//! Provides scheduled generation of weekly and monthly macro reports
//! with Telegram notification integration.

use anyhow::Result;
use chrono::{DateTime, Datelike, FixedOffset, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use super::bridge::{AgentBridge, AgentBridgeConfig};
use super::types::{MacroReport, ReportType};
use crate::notification::NotificationClient;

// ============================================================================
// Timezone Constants
// ============================================================================

/// Get Beijing timezone (UTC+8).
///
/// SAFETY: 8 hours = 28800 seconds is always within valid FixedOffset range
/// (which supports -23:59:59 to +23:59:59).
fn beijing_timezone() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).expect("UTC+8 is a valid timezone offset")
}

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
    /// Enable daily morning reports (pre-market)
    pub daily_morning_enabled: bool,
    /// Daily morning report cron expression (default: "0 9 * * *" = 9 AM daily)
    pub daily_morning_cron: String,
    /// Enable daily afternoon reports (post-market)
    pub daily_afternoon_enabled: bool,
    /// Daily afternoon report cron expression (default: "0 16 * * *" = 4 PM daily)
    pub daily_afternoon_cron: String,
}

impl Default for ReportGeneratorConfig {
    fn default() -> Self {
        Self {
            weekly_enabled: true,
            weekly_cron: "0 9 * * 1".to_string(),
            monthly_enabled: true,
            monthly_cron: "0 9 1 * *".to_string(),
            daily_morning_enabled: true,
            daily_morning_cron: "0 9 * * *".to_string(),
            daily_afternoon_enabled: true,
            daily_afternoon_cron: "0 16 * * *".to_string(),
        }
    }
}

/// State for tracking report generation.
struct ReportState {
    last_weekly: Option<DateTime<Utc>>,
    last_monthly: Option<DateTime<Utc>>,
    last_daily_morning: Option<DateTime<Utc>>,
    last_daily_afternoon: Option<DateTime<Utc>>,
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
                last_daily_morning: None,
                last_daily_afternoon: None,
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
            daily_morning_enabled = self.config.daily_morning_enabled,
            daily_afternoon_enabled = self.config.daily_afternoon_enabled,
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

        // Check daily morning report (9 AM Beijing time, weekdays only)
        if self.config.daily_morning_enabled && self.should_generate_daily_morning(&now).await {
            if let Err(e) = self.generate_and_send(ReportType::DailyMorning).await {
                error!(error = %e, "Failed to generate daily morning report");
            }
        }

        // Check daily afternoon report (4 PM Beijing time, weekdays only)
        if self.config.daily_afternoon_enabled && self.should_generate_daily_afternoon(&now).await {
            if let Err(e) = self.generate_and_send(ReportType::DailyAfternoon).await {
                error!(error = %e, "Failed to generate daily afternoon report");
            }
        }
    }

    /// Check if weekly report should be generated.
    async fn should_generate_weekly(&self, now: &DateTime<Utc>) -> bool {
        use chrono::{Datelike, Timelike};

        // Convert to Beijing time (UTC+8)
        let beijing = now.with_timezone(&beijing_timezone());

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
        let beijing = now.with_timezone(&beijing_timezone());

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
            let last_beijing = last.with_timezone(&beijing_timezone());
            // Don't generate if we already did this month
            if last_beijing.month() == beijing.month() && last_beijing.year() == beijing.year() {
                return false;
            }
        }

        true
    }

    /// Check if daily morning report should be generated.
    ///
    /// Daily morning reports are generated at 9:00 AM Beijing time on weekdays.
    async fn should_generate_daily_morning(&self, now: &DateTime<Utc>) -> bool {
        use chrono::{Datelike, Timelike};

        // Convert to Beijing time (UTC+8)
        let beijing = now.with_timezone(&beijing_timezone());

        // Skip weekends (Saturday = 5, Sunday = 6 in chrono's Weekday)
        let weekday = beijing.weekday();
        if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
            return false;
        }

        // Check if it's 9:00-9:59 Beijing time
        if beijing.hour() != 9 {
            return false;
        }

        // Check if we already generated today
        let state = self.state.read().await;
        if let Some(last) = state.last_daily_morning {
            // Don't generate if we already did today
            if last.date_naive() == now.date_naive() {
                return false;
            }
        }

        true
    }

    /// Check if daily afternoon report should be generated.
    ///
    /// Daily afternoon reports are generated at 4:00 PM Beijing time on weekdays.
    async fn should_generate_daily_afternoon(&self, now: &DateTime<Utc>) -> bool {
        use chrono::{Datelike, Timelike};

        // Convert to Beijing time (UTC+8)
        let beijing = now.with_timezone(&beijing_timezone());

        // Skip weekends
        let weekday = beijing.weekday();
        if weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun {
            return false;
        }

        // Check if it's 16:00-16:59 Beijing time
        if beijing.hour() != 16 {
            return false;
        }

        // Check if we already generated today
        let state = self.state.read().await;
        if let Some(last) = state.last_daily_afternoon {
            // Don't generate if we already did today
            if last.date_naive() == now.date_naive() {
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
        let beijing = now.with_timezone(&beijing_timezone());

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
            ReportType::DailyMorning => {
                format!("{} 早间", beijing.format("%Y-%m-%d"))
            }
            ReportType::DailyAfternoon => {
                format!("{} 收盘", beijing.format("%Y-%m-%d"))
            }
            ReportType::Quarterly => {
                // Determine current quarter
                let quarter = (beijing.month() - 1) / 3 + 1;
                format!("{}年Q{}", beijing.year(), quarter)
            }
            ReportType::DataRelease => {
                format!("{} 数据发布", beijing.format("%Y-%m-%d %H:%M"))
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
            .with_timezone(&beijing_timezone());
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
            ReportType::DailyMorning => state.last_daily_morning = Some(now),
            ReportType::DailyAfternoon => state.last_daily_afternoon = Some(now),
            ReportType::Quarterly | ReportType::DataRelease | ReportType::AdHoc => {}
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
        assert!(config.daily_morning_enabled);
        assert!(config.daily_afternoon_enabled);
        assert_eq!(config.weekly_cron, "0 9 * * 1");
        assert_eq!(config.monthly_cron, "0 9 1 * *");
        assert_eq!(config.daily_morning_cron, "0 9 * * *");
        assert_eq!(config.daily_afternoon_cron, "0 16 * * *");
    }

    #[test]
    fn test_report_type_display() {
        assert_eq!(ReportType::Weekly.to_string(), "周度");
        assert_eq!(ReportType::Monthly.to_string(), "月度");
        assert_eq!(ReportType::DailyMorning.to_string(), "早间");
        assert_eq!(ReportType::DailyAfternoon.to_string(), "午后");
        assert_eq!(ReportType::Quarterly.to_string(), "季度");
        assert_eq!(ReportType::DataRelease.to_string(), "数据解读");
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

        let daily_morning = generator.get_period_description(ReportType::DailyMorning);
        assert!(daily_morning.contains("早间"));

        let daily_afternoon = generator.get_period_description(ReportType::DailyAfternoon);
        assert!(daily_afternoon.contains("收盘"));

        let quarterly = generator.get_period_description(ReportType::Quarterly);
        assert!(quarterly.contains("Q"));

        let data_release = generator.get_period_description(ReportType::DataRelease);
        assert!(data_release.contains("数据发布"));
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
