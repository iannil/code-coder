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
    /// Channel type (required by CodeCoder API)
    pub channel: String,
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
    /// Quarterly economic outlook (new)
    Quarterly,
    /// Data release interpretation (new)
    DataRelease,
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
            ReportType::Quarterly => write!(f, "季度"),
            ReportType::DataRelease => write!(f, "数据解读"),
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
// High-Frequency Data Types (Phase 2 preparation)
// ============================================================================

/// High-frequency economic indicator codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum HighFrequencyIndicator {
    // Production & Industrial
    /// 六大发电集团日均耗煤量
    PowerCoalConsumption,
    /// 高炉开工率
    BlastFurnaceRate,
    /// 全钢胎开工率
    TruckTireRate,
    /// 半钢胎开工率
    PassengerTireRate,
    /// PTA产业链负荷率
    PtaLoadRate,

    // Prices
    /// 螺纹钢价格
    RebarPrice,
    /// 水泥价格指数
    CementPriceIndex,
    /// 动力煤价格
    ThermalCoalPrice,

    // Investment
    /// 挖掘机销量
    ExcavatorSales,
    /// 石油沥青开工率
    AsphaltProductionRate,
    /// 100城土地成交面积
    LandTransaction100City,
    /// 30城商品房成交面积
    HouseSales30City,
    /// 土地溢价率
    LandPremiumRate,

    // Consumption & Logistics
    /// 城市拥堵指数
    CityTrafficIndex,
    /// 地铁客运量
    MetroPassengers,
    /// 整车货运流量指数
    TruckFreightIndex,
    /// 快递揽投量
    ExpressDeliveryVolume,
    /// 电影票房
    BoxOffice,

    // Agriculture & Food
    /// 农产品批发价格200指数
    AgriPrice200Index,
    /// 猪肉批发价
    PorkPrice,

    // Trade
    /// CCFI出口集装箱运价
    CcfiIndex,
    /// BDI干散货指数
    BdiIndex,

    // PMI
    /// 官方制造业PMI
    PmiOfficial,
    /// 财新制造业PMI
    PmiCaixin,
}

impl HighFrequencyIndicator {
    /// Get the Chinese name
    pub fn chinese_name(&self) -> &'static str {
        match self {
            Self::PowerCoalConsumption => "六大发电集团日均耗煤量",
            Self::BlastFurnaceRate => "高炉开工率",
            Self::TruckTireRate => "全钢胎开工率",
            Self::PassengerTireRate => "半钢胎开工率",
            Self::PtaLoadRate => "PTA产业链负荷率",
            Self::RebarPrice => "螺纹钢价格",
            Self::CementPriceIndex => "水泥价格指数",
            Self::ThermalCoalPrice => "动力煤价格",
            Self::ExcavatorSales => "挖掘机销量",
            Self::AsphaltProductionRate => "石油沥青开工率",
            Self::LandTransaction100City => "100城土地成交面积",
            Self::HouseSales30City => "30城商品房成交面积",
            Self::LandPremiumRate => "土地溢价率",
            Self::CityTrafficIndex => "城市拥堵指数",
            Self::MetroPassengers => "地铁客运量",
            Self::TruckFreightIndex => "整车货运流量指数",
            Self::ExpressDeliveryVolume => "快递揽投量",
            Self::BoxOffice => "电影票房",
            Self::AgriPrice200Index => "农产品批发价格200指数",
            Self::PorkPrice => "猪肉批发价",
            Self::CcfiIndex => "CCFI出口集装箱运价",
            Self::BdiIndex => "BDI干散货指数",
            Self::PmiOfficial => "官方制造业PMI",
            Self::PmiCaixin => "财新制造业PMI",
        }
    }

    /// Get the data frequency
    pub fn frequency(&self) -> DataFrequency {
        match self {
            Self::PowerCoalConsumption
            | Self::RebarPrice
            | Self::ThermalCoalPrice
            | Self::HouseSales30City
            | Self::BoxOffice
            | Self::AgriPrice200Index
            | Self::PorkPrice
            | Self::BdiIndex => DataFrequency::Daily,

            Self::BlastFurnaceRate
            | Self::TruckTireRate
            | Self::PassengerTireRate
            | Self::PtaLoadRate
            | Self::CementPriceIndex
            | Self::AsphaltProductionRate
            | Self::LandTransaction100City
            | Self::LandPremiumRate
            | Self::CityTrafficIndex
            | Self::MetroPassengers
            | Self::TruckFreightIndex
            | Self::ExpressDeliveryVolume
            | Self::CcfiIndex => DataFrequency::Weekly,

            Self::ExcavatorSales | Self::PmiOfficial | Self::PmiCaixin => DataFrequency::Monthly,
        }
    }

    /// Get what this indicator validates/predicts
    pub fn validates(&self) -> &'static str {
        match self {
            Self::PowerCoalConsumption => "工业生产动能",
            Self::BlastFurnaceRate => "钢铁/基建需求",
            Self::TruckTireRate => "货运物流景气",
            Self::PassengerTireRate => "乘用车生产",
            Self::PtaLoadRate => "纺织服装景气",
            Self::RebarPrice | Self::CementPriceIndex => "基建/房地产需求",
            Self::ThermalCoalPrice => "能源成本/工业热度",
            Self::ExcavatorSales => "工程开工",
            Self::AsphaltProductionRate => "道路基建进度",
            Self::LandTransaction100City | Self::LandPremiumRate => "房企拿地信心",
            Self::HouseSales30City => "房地产销售",
            Self::CityTrafficIndex | Self::MetroPassengers => "经济活跃度/线下消费",
            Self::TruckFreightIndex => "大宗物流",
            Self::ExpressDeliveryVolume => "电商消费",
            Self::BoxOffice => "线下娱乐消费",
            Self::AgriPrice200Index | Self::PorkPrice => "食品CPI",
            Self::CcfiIndex => "出口景气度",
            Self::BdiIndex => "全球工业需求",
            Self::PmiOfficial | Self::PmiCaixin => "经济景气度",
        }
    }
}

/// Data frequency for high-frequency indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFrequency {
    /// Daily data
    Daily,
    /// Weekly data
    Weekly,
    /// Monthly data
    Monthly,
}

impl std::fmt::Display for DataFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DataFrequency::Daily => "日",
            DataFrequency::Weekly => "周",
            DataFrequency::Monthly => "月",
        };
        write!(f, "{}", s)
    }
}

/// High-frequency data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighFrequencyDataPoint {
    /// Indicator type
    pub indicator: HighFrequencyIndicator,
    /// Value
    pub value: f64,
    /// Unit (e.g., "万吨", "%", "元/吨")
    pub unit: String,
    /// Data date
    pub data_date: chrono::NaiveDate,
    /// YoY change if available
    pub yoy_change: Option<f64>,
    /// MoM/WoW change if available
    pub period_change: Option<f64>,
    /// Data source
    pub source: String,
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
            channel: "test-channel".to_string(),
            agent: "macro".to_string(),
            message: "分析当前宏观环境".to_string(),
            stream: false,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("macro"));
        assert!(json.contains("分析当前宏观环境"));
        assert!(json.contains("test-user"));
        assert!(json.contains("test-channel"));
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

    #[test]
    fn test_high_frequency_indicator_names() {
        assert_eq!(
            HighFrequencyIndicator::PowerCoalConsumption.chinese_name(),
            "六大发电集团日均耗煤量"
        );
        assert_eq!(
            HighFrequencyIndicator::BlastFurnaceRate.chinese_name(),
            "高炉开工率"
        );
        assert_eq!(
            HighFrequencyIndicator::HouseSales30City.chinese_name(),
            "30城商品房成交面积"
        );
    }

    #[test]
    fn test_high_frequency_indicator_frequency() {
        assert_eq!(
            HighFrequencyIndicator::PowerCoalConsumption.frequency(),
            DataFrequency::Daily
        );
        assert_eq!(
            HighFrequencyIndicator::BlastFurnaceRate.frequency(),
            DataFrequency::Weekly
        );
        assert_eq!(
            HighFrequencyIndicator::PmiOfficial.frequency(),
            DataFrequency::Monthly
        );
    }

    #[test]
    fn test_high_frequency_indicator_validates() {
        assert_eq!(
            HighFrequencyIndicator::PowerCoalConsumption.validates(),
            "工业生产动能"
        );
        assert_eq!(
            HighFrequencyIndicator::HouseSales30City.validates(),
            "房地产销售"
        );
    }

    #[test]
    fn test_data_frequency_display() {
        assert_eq!(DataFrequency::Daily.to_string(), "日");
        assert_eq!(DataFrequency::Weekly.to_string(), "周");
        assert_eq!(DataFrequency::Monthly.to_string(), "月");
    }
}
