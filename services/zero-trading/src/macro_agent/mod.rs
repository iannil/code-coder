//! Macro agent integration module.
//!
//! This module provides intelligent macro-economic analysis by integrating
//! with the CodeCoder API's macro agent. It implements a hybrid approach:
//!
//! - **Fast path (Rule Engine)**: Deterministic rules for common cases
//! - **Slow path (Agent)**: LLM-powered analysis for anomalies and edge cases
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                    MacroOrchestrator                          │
//! ├──────────────────────────────────────────────────────────────┤
//! │                                                               │
//! │  ┌─────────────────┐     ┌─────────────────┐                │
//! │  │   MacroFilter   │     │   AgentBridge   │                │
//! │  │  (Rule Engine)  │     │ (CodeCoder API) │                │
//! │  └────────┬────────┘     └────────┬────────┘                │
//! │           │                       │                          │
//! │           └───────────┬───────────┘                          │
//! │                       ▼                                      │
//! │              ┌────────────────┐                              │
//! │              │  MacroDecision │                              │
//! │              └────────────────┘                              │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use zero_trading::macro_agent::{MacroOrchestrator, OrchestratorConfig};
//!
//! // Create orchestrator with rule engine and agent bridge
//! let orchestrator = MacroOrchestrator::new(
//!     rule_engine,
//!     agent_config,
//!     OrchestratorConfig::default(),
//! );
//!
//! // Get macro decision (uses fast path by default, slow path for anomalies)
//! let decision = orchestrator.evaluate().await?;
//!
//! // Adjust position size based on macro conditions
//! let adjusted_size = orchestrator.adjust_position_size(base_size).await;
//! ```
//!
//! # Anomaly Triggers
//!
//! The orchestrator will invoke the agent for deep analysis when:
//!
//! | Condition | Threshold | Description |
//! |-----------|-----------|-------------|
//! | Extreme risk appetite | < 30 or > 70 | Market sentiment at extremes |
//! | Avoid trading signal | trading_bias = AvoidTrading | Rule engine signals caution |
//! | Position reduction | multiplier < 0.5 | Significant de-risking suggested |
//! | Extreme PMI | < 48 or > 54 | Economic indicator at extremes |
//! | Indicator divergence | Multiple signals conflict | Conflicting economic signals |
//!
//! # Periodic Reports
//!
//! The `MacroReportGenerator` provides scheduled report generation:
//!
//! - **Weekly reports**: Every Monday 9:00 AM (Beijing time)
//! - **Monthly reports**: 1st day of each month 9:00 AM (Beijing time)
//!
//! Reports are automatically sent to Telegram via the notification system.

pub mod bridge;
pub mod orchestrator;
pub mod report;
pub mod types;

// Re-export main types for convenience
pub use bridge::{AgentBridge, AgentBridgeConfig};
pub use orchestrator::{MacroOrchestrator, OrchestratorConfig};
pub use report::{MacroReportGenerator, ReportGeneratorConfig};
pub use types::{
    AgentAnalysis, AnalysisTrigger, DecisionSource, MacroContext, MacroDecision, MacroReport,
    ReportType,
};

use std::sync::Arc;
use zero_common::config::Config;

use crate::macro_filter::MacroFilter;
use crate::notification::NotificationClient;

/// Create a fully configured macro orchestrator from the application config.
pub fn create_orchestrator(config: &Config, macro_filter: Arc<MacroFilter>) -> MacroOrchestrator {
    let agent_config = create_agent_config(config);
    let orchestrator_config = create_orchestrator_config(config);

    MacroOrchestrator::new(macro_filter, agent_config, orchestrator_config)
}

/// Create a fully configured report generator from the application config.
pub fn create_report_generator(
    config: &Config,
    notification: Arc<NotificationClient>,
) -> MacroReportGenerator {
    let agent_config = create_agent_config(config);
    let report_config = create_report_config(config);

    MacroReportGenerator::new(agent_config, notification, report_config)
}

/// Create AgentBridgeConfig from application config.
fn create_agent_config(config: &Config) -> AgentBridgeConfig {
    let trading = config.trading.as_ref();

    AgentBridgeConfig {
        // Use centralized codecoder_endpoint from Config
        codecoder_endpoint: config.codecoder_endpoint(),
        timeout: std::time::Duration::from_secs(
            trading
                .and_then(|t| t.macro_agent.as_ref())
                .map(|m| m.timeout_secs)
                .unwrap_or(30),
        ),
        max_retries: 2,
        retry_backoff: std::time::Duration::from_secs(1),
    }
}

/// Create OrchestratorConfig from application config.
fn create_orchestrator_config(config: &Config) -> OrchestratorConfig {
    let trading = config.trading.as_ref();
    let macro_agent = trading.and_then(|t| t.macro_agent.as_ref());

    OrchestratorConfig {
        agent_enabled: macro_agent.map(|m| m.enabled).unwrap_or(true),
        extreme_risk_low: 30.0,
        extreme_risk_high: 70.0,
        position_reduction_threshold: 0.5,
        pmi_low_threshold: 48.0,
        pmi_high_threshold: 54.0,
        min_agent_confidence: 0.6,
        agent_cache_secs: macro_agent
            .map(|m| m.cache_duration_secs)
            .unwrap_or(3600),
    }
}

/// Create ReportGeneratorConfig from application config.
fn create_report_config(config: &Config) -> ReportGeneratorConfig {
    let trading = config.trading.as_ref();
    let macro_agent = trading.and_then(|t| t.macro_agent.as_ref());

    ReportGeneratorConfig {
        weekly_enabled: macro_agent
            .map(|m| m.weekly_report_enabled)
            .unwrap_or(true),
        weekly_cron: macro_agent
            .and_then(|m| m.weekly_report_cron.clone())
            .unwrap_or_else(|| "0 9 * * 1".to_string()),
        monthly_enabled: macro_agent
            .map(|m| m.monthly_report_enabled)
            .unwrap_or(true),
        monthly_cron: macro_agent
            .and_then(|m| m.monthly_report_cron.clone())
            .unwrap_or_else(|| "0 9 1 * *".to_string()),
        daily_morning_enabled: macro_agent
            .map(|m| m.daily_morning_enabled)
            .unwrap_or(true),
        daily_morning_cron: macro_agent
            .and_then(|m| m.daily_morning_cron.clone())
            .unwrap_or_else(|| "0 9 * * *".to_string()),
        daily_afternoon_enabled: macro_agent
            .map(|m| m.daily_afternoon_enabled)
            .unwrap_or(true),
        daily_afternoon_cron: macro_agent
            .and_then(|m| m.daily_afternoon_cron.clone())
            .unwrap_or_else(|| "0 16 * * *".to_string()),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_agent_config_defaults() {
        let config = Config::default();
        let agent_config = create_agent_config(&config);

        assert_eq!(agent_config.codecoder_endpoint, config.codecoder_endpoint());
        assert_eq!(agent_config.timeout, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_create_orchestrator_config_defaults() {
        let config = Config::default();
        let orch_config = create_orchestrator_config(&config);

        assert!(orch_config.agent_enabled);
        assert!((orch_config.extreme_risk_low - 30.0).abs() < 0.001);
        assert!((orch_config.pmi_low_threshold - 48.0).abs() < 0.001);
    }

    #[test]
    fn test_create_report_config_defaults() {
        let config = Config::default();
        let report_config = create_report_config(&config);

        assert!(report_config.weekly_enabled);
        assert!(report_config.monthly_enabled);
        assert!(report_config.daily_morning_enabled);
        assert!(report_config.daily_afternoon_enabled);
        assert_eq!(report_config.weekly_cron, "0 9 * * 1");
        assert_eq!(report_config.daily_morning_cron, "0 9 * * *");
        assert_eq!(report_config.daily_afternoon_cron, "0 16 * * *");
    }

    #[test]
    fn test_create_orchestrator() {
        let config = Config::default();
        let macro_filter = Arc::new(MacroFilter::new(&config));
        let _orchestrator = create_orchestrator(&config, macro_filter);

        // Orchestrator created successfully if we reach here
    }
}
