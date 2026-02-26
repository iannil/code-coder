//! Types for macro agent integration.
//!
//! Defines request/response structures for communicating with the CodeCoder API's
//! macro agent, as well as parsed analysis results.

use serde::{Deserialize, Serialize};

use crate::macro_filter::{EconomicCyclePhase, TradingBias};

/// Request to the CodeCoder API chat endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct AgentRequest {
    /// User ID for the request (required by CodeCoder API)
    pub user_id: String,
    /// Agent to use (e.g., "macro")
    pub agent: String,
    /// Message/prompt for the agent
    pub message: String,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
}

/// Response from the CodeCoder API chat endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentResponse {
    /// Response content from the agent
    pub content: String,
    /// Optional metadata
    #[serde(default)]
    pub metadata: Option<AgentMetadata>,
}

/// Metadata from agent response.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AgentMetadata {
    /// Model used
    #[serde(default)]
    pub model: Option<String>,
    /// Input tokens used
    #[serde(default)]
    pub input_tokens: Option<u32>,
    /// Output tokens used
    #[serde(default)]
    pub output_tokens: Option<u32>,
}

/// Context for macro analysis request.
#[derive(Debug, Clone, Serialize)]
pub struct MacroContext {
    /// PMI reading
    pub pmi: Option<f64>,
    /// M2 year-over-year growth
    pub m2_growth: Option<f64>,
    /// Social financing data
    pub social_financing: Option<f64>,
    /// Current risk appetite (0-100)
    pub risk_appetite: f64,
    /// Current position multiplier
    pub position_multiplier: f64,
    /// Current trading bias from rule engine
    pub trading_bias: TradingBias,
    /// Additional notes from rule engine
    pub notes: String,
}

/// Parsed analysis result from the macro agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalysis {
    /// Economic cycle phase assessment
    pub cycle_phase: EconomicCyclePhase,
    /// Recommended position size multiplier (0.0 - 1.5)
    pub position_advice: f64,
    /// Risk warnings identified
    pub risk_warnings: Vec<String>,
    /// Recommended trading bias
    pub trading_bias: TradingBias,
    /// Detailed reasoning from the agent
    pub reasoning: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
}

impl Default for AgentAnalysis {
    fn default() -> Self {
        Self {
            cycle_phase: EconomicCyclePhase::EarlyRecovery,
            position_advice: 1.0,
            risk_warnings: Vec::new(),
            trading_bias: TradingBias::Neutral,
            reasoning: String::new(),
            confidence: 0.5,
        }
    }
}

/// Trigger reasons for agent analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisTrigger {
    /// Risk appetite is extreme (< 30 or > 70)
    ExtremeRiskAppetite,
    /// Trading bias is AvoidTrading
    AvoidTradingSignal,
    /// Position multiplier suggests significant reduction (< 0.5)
    SignificantPositionReduction,
    /// PMI reading is extreme (< 48 or > 54)
    ExtremePmi,
    /// Multiple indicators showing divergence
    IndicatorDivergence,
    /// Scheduled periodic analysis
    ScheduledAnalysis,
    /// Manual trigger by user
    ManualTrigger,
}

impl std::fmt::Display for AnalysisTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisTrigger::ExtremeRiskAppetite => write!(f, "极端风险偏好"),
            AnalysisTrigger::AvoidTradingSignal => write!(f, "避免交易信号"),
            AnalysisTrigger::SignificantPositionReduction => write!(f, "大幅降仓建议"),
            AnalysisTrigger::ExtremePmi => write!(f, "PMI极端值"),
            AnalysisTrigger::IndicatorDivergence => write!(f, "指标背离"),
            AnalysisTrigger::ScheduledAnalysis => write!(f, "定期分析"),
            AnalysisTrigger::ManualTrigger => write!(f, "手动触发"),
        }
    }
}

/// Combined decision from rule engine and agent analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDecision {
    /// Source of the decision
    pub source: DecisionSource,
    /// Economic cycle phase
    pub cycle_phase: EconomicCyclePhase,
    /// Position multiplier (0.3 - 1.5)
    pub position_multiplier: f64,
    /// Trading bias
    pub trading_bias: TradingBias,
    /// Risk appetite (0-100)
    pub risk_appetite: f64,
    /// Risk warnings
    pub risk_warnings: Vec<String>,
    /// Human-readable summary
    pub summary: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
}

/// Source of macro decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionSource {
    /// Decision from rule engine only
    RuleEngine,
    /// Decision from agent analysis
    AgentAnalysis,
    /// Merged decision from both sources
    Merged,
    /// Default fallback decision
    Fallback,
}

/// Report type for scheduled reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    /// Weekly macro report
    Weekly,
    /// Monthly macro report
    Monthly,
    /// Daily morning report (pre-market, 9:00 Beijing time)
    DailyMorning,
    /// Daily afternoon report (post-market, 16:00 Beijing time)
    DailyAfternoon,
    /// Ad-hoc report
    AdHoc,
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportType::Weekly => write!(f, "周度"),
            ReportType::Monthly => write!(f, "月度"),
            ReportType::DailyMorning => write!(f, "早间"),
            ReportType::DailyAfternoon => write!(f, "午后"),
            ReportType::AdHoc => write!(f, "即时"),
        }
    }
}

/// Generated macro report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroReport {
    /// Report type
    pub report_type: ReportType,
    /// Report title
    pub title: String,
    /// Report period description
    pub period: String,
    /// Report content (markdown formatted)
    pub content: String,
    /// Key highlights
    pub highlights: Vec<String>,
    /// Generated timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_request_serialization() {
        let request = AgentRequest {
            user_id: "test-user".to_string(),
            agent: "macro".to_string(),
            message: "分析当前宏观环境".to_string(),
            stream: false,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("macro"));
        assert!(json.contains("分析当前宏观环境"));
        assert!(json.contains("test-user"));
    }

    #[test]
    fn test_analysis_trigger_display() {
        assert_eq!(AnalysisTrigger::ExtremeRiskAppetite.to_string(), "极端风险偏好");
        assert_eq!(AnalysisTrigger::ExtremePmi.to_string(), "PMI极端值");
    }

    #[test]
    fn test_macro_context_serialization() {
        let context = MacroContext {
            pmi: Some(49.5),
            m2_growth: Some(9.2),
            social_financing: Some(3.5),
            risk_appetite: 45.0,
            position_multiplier: 0.8,
            trading_bias: TradingBias::Neutral,
            notes: "测试".to_string(),
        };
        let json = serde_json::to_string(&context).unwrap();
        assert!(json.contains("49.5"));
    }

    #[test]
    fn test_agent_analysis_default() {
        let analysis = AgentAnalysis::default();
        assert!((analysis.position_advice - 1.0).abs() < 0.001);
        assert!(analysis.risk_warnings.is_empty());
    }
}
