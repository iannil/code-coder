//! Core types for the value analysis engine.
//!
//! This module defines the fundamental data structures based on the "Observer Constructionism"
//! (观察者建构论) value investing framework. The framework operates on four progressive layers:
//!
//! 1. **National Consensus** (国家共识): Safety themes in national policy
//! 2. **Evaluation Power** (评估权): Who evaluates whom in the supply chain
//! 3. **Financial Verification** (财务验证): "Printing machine" checklist
//! 4. **Trade Execution** (交易执行): "Trim weak, nurture strong" discipline

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Layer 1: National Consensus Types (国家共识)
// ============================================================================

/// Safety themes identified in national policy.
///
/// These represent the "hard consensus" (硬共识) areas where the state's commitment
/// is unambiguous and persistent across policy cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SafetyTheme {
    /// Energy security (能源安全): coal, electricity, oil & gas
    EnergySecurity,
    /// Food security (粮食安全): seed industry, agricultural technology
    FoodSecurity,
    /// Financial security (金融安全): gold, digital currency infrastructure
    FinancialSecurity,
    /// Industrial security (产业安全): new energy, semiconductors, high-end manufacturing
    IndustrySecurity,
    /// Technology security (科技安全): AI, quantum computing, aerospace
    TechnologySecurity,
    /// Military security (国防安全): defense contractors, aerospace
    MilitarySecurity,
}

impl std::fmt::Display for SafetyTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnergySecurity => write!(f, "能源安全"),
            Self::FoodSecurity => write!(f, "粮食安全"),
            Self::FinancialSecurity => write!(f, "金融安全"),
            Self::IndustrySecurity => write!(f, "产业安全"),
            Self::TechnologySecurity => write!(f, "科技安全"),
            Self::MilitarySecurity => write!(f, "国防安全"),
        }
    }
}

/// A signal from national policy consensus analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusSignal {
    /// The safety theme identified
    pub theme: SafetyTheme,
    /// Signal strength (0.0-1.0), based on policy emphasis
    pub strength: f64,
    /// Policy sources that support this signal
    pub policy_sources: Vec<PolicyReference>,
    /// Key phrases extracted from policy documents
    pub key_phrases: Vec<String>,
    /// When this signal was last updated
    pub updated_at: DateTime<Utc>,
}

/// Reference to a policy document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyReference {
    /// Document title (e.g., "2024年政府工作报告")
    pub title: String,
    /// Document date
    pub date: DateTime<Utc>,
    /// Source authority (e.g., "国务院", "发改委")
    pub authority: String,
    /// Relevant excerpt
    pub excerpt: Option<String>,
}

// ============================================================================
// Layer 2: Evaluation Power Types (评估权)
// ============================================================================

/// Evaluation power tier in the supply chain hierarchy.
///
/// The core insight: "Who is asking whom?" (谁在求谁)
/// - Primary evaluators set standards and prices
/// - Secondary evaluators have strong bargaining power
/// - Evaluated entities are price-takers with limited moat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvaluationTier {
    /// Primary evaluator (一级评估者): ecosystem dominators like Moutai, Tencent
    Primary,
    /// Secondary evaluator (二级评估者): critical nodes like CATL, hidden champions
    Secondary,
    /// Evaluated entity (被评估者): assembly, component packaging, OEM
    Evaluated,
}

impl std::fmt::Display for EvaluationTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "一级评估者"),
            Self::Secondary => write!(f, "二级评估者"),
            Self::Evaluated => write!(f, "被评估者"),
        }
    }
}

/// Type of economic moat (护城河).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoatType {
    /// Spiritual totem (精神图腾): brand power like Moutai
    SpiritualTotem,
    /// Network effect (网络效应): like WeChat, Alipay
    NetworkEffect,
    /// Resource monopoly (资源禀赋): natural resource control
    ResourceMonopoly,
    /// Technical patent (技术专利): proprietary technology
    TechnicalPatent,
    /// Process know-how (工艺诀窍): manufacturing expertise
    ProcessKnowhow,
    /// Cost advantage (成本优势): scale economics
    CostAdvantage,
    /// Switching cost (转换成本): high customer lock-in
    SwitchingCost,
    /// Regulatory barrier (牌照壁垒): government-granted monopoly
    RegulatoryBarrier,
    /// No moat (无护城河)
    None,
}

impl std::fmt::Display for MoatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpiritualTotem => write!(f, "精神图腾"),
            Self::NetworkEffect => write!(f, "网络效应"),
            Self::ResourceMonopoly => write!(f, "资源禀赋"),
            Self::TechnicalPatent => write!(f, "技术专利"),
            Self::ProcessKnowhow => write!(f, "工艺诀窍"),
            Self::CostAdvantage => write!(f, "成本优势"),
            Self::SwitchingCost => write!(f, "转换成本"),
            Self::RegulatoryBarrier => write!(f, "牌照壁垒"),
            Self::None => write!(f, "无护城河"),
        }
    }
}

/// Evaluation power assessment for a company.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationPowerScore {
    /// Symbol/ticker
    pub symbol: String,
    /// Evaluation power tier
    pub tier: EvaluationTier,
    /// Overall score (0-100)
    pub score: f64,
    /// Primary moat type
    pub moat_type: MoatType,
    /// Secondary moat types
    pub secondary_moats: Vec<MoatType>,
    /// Pricing power (0-100): ability to raise prices without losing customers
    pub pricing_power: f64,
    /// Upstream dependency (0-100): how much suppliers can squeeze margins
    pub upstream_dependency: f64,
    /// Downstream dependency (0-100): how much customers can squeeze margins
    pub downstream_dependency: f64,
    /// Human-readable reasoning
    pub reasoning: String,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

// ============================================================================
// Layer 3: Financial Verification Types (财务验证)
// ============================================================================

/// Cash flow DNA pattern based on the three cash flow statements.
///
/// The pattern is determined by the sign of:
/// - Operating Cash Flow (OCF)
/// - Investing Cash Flow (ICF)
/// - Financing Cash Flow (FCF)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CashFlowDNA {
    /// Cash cow (现金奶牛型): OCF+, ICF-, FCF-
    /// Mature business generating excess cash, paying down debt
    CashCow,
    /// Growth expansion (成长扩张型): OCF+, ICF-, FCF+
    /// Growing business reinvesting profits and raising capital
    GrowthExpansion,
    /// Steady state (稳态型): OCF+, ICF-, FCF≈0
    /// Mature business with balanced cash flows
    SteadyState,
    /// Harvesting (收割型): OCF+, ICF+, FCF-
    /// Selling assets and returning cash to shareholders
    Harvesting,
    /// Startup blood transfusion (初创输血型): OCF-, ICF-, FCF+
    /// Pre-profit stage, burning cash while raising capital
    StartupBloodTransfusion,
    /// Decline liquidation (衰退变卖型): OCF-, ICF+, FCF-
    /// Declining business selling assets to survive
    DeclineLiquidation,
    /// Ponzi scheme (庞氏骗局型): OCF-, ICF-, FCF+ (persistent)
    /// Dangerous pattern: constantly raising money without generating returns
    PonziScheme,
    /// Unknown pattern
    Unknown,
}

impl std::fmt::Display for CashFlowDNA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CashCow => write!(f, "现金奶牛型 (OCF+, ICF-, FCF-)"),
            Self::GrowthExpansion => write!(f, "成长扩张型 (OCF+, ICF-, FCF+)"),
            Self::SteadyState => write!(f, "稳态型 (OCF+, ICF-, FCF≈0)"),
            Self::Harvesting => write!(f, "收割型 (OCF+, ICF+, FCF-)"),
            Self::StartupBloodTransfusion => write!(f, "初创输血型 (OCF-, ICF-, FCF+)"),
            Self::DeclineLiquidation => write!(f, "衰退变卖型 (OCF-, ICF+, FCF-)"),
            Self::PonziScheme => write!(f, "庞氏骗局型 (OCF-, ICF-, FCF+)"),
            Self::Unknown => write!(f, "未知类型"),
        }
    }
}

impl CashFlowDNA {
    /// Returns whether this DNA pattern is generally healthy for long-term investment.
    pub fn is_healthy(&self) -> bool {
        matches!(
            self,
            Self::CashCow | Self::GrowthExpansion | Self::SteadyState
        )
    }

    /// Returns a risk level (0-10) for this pattern.
    pub fn risk_level(&self) -> u8 {
        match self {
            Self::CashCow => 1,
            Self::SteadyState => 2,
            Self::GrowthExpansion => 3,
            Self::Harvesting => 5,
            Self::StartupBloodTransfusion => 7,
            Self::DeclineLiquidation => 8,
            Self::PonziScheme => 10,
            Self::Unknown => 5,
        }
    }
}

/// ROE decomposition drivers (DuPont analysis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoeDriver {
    /// High net profit margin (高利润率): brand/moat-driven
    HighMargin,
    /// High asset turnover (高周转): efficiency-driven (retail)
    HighTurnover,
    /// High leverage (高杠杆): capital structure driven (banks, real estate)
    HighLeverage,
    /// Balanced (均衡型): no single dominant driver
    Balanced,
}

impl std::fmt::Display for RoeDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HighMargin => write!(f, "高利润率驱动"),
            Self::HighTurnover => write!(f, "高周转驱动"),
            Self::HighLeverage => write!(f, "高杠杆驱动"),
            Self::Balanced => write!(f, "均衡型"),
        }
    }
}

/// The "Printing Machine" checklist (印钞机清单).
///
/// This checklist verifies whether a company has the characteristics of
/// a sustainable, high-quality business.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintingMachineChecklist {
    /// Symbol/ticker
    pub symbol: String,

    // === Qualitative Indicators (定性指标) ===
    /// Does the company have evaluation power in its supply chain?
    pub has_evaluation_power: bool,
    /// Is the business model simple and understandable?
    pub is_simple_and_understandable: bool,
    /// Is there demand stickiness (recurring revenue, subscription, habit)?
    pub has_demand_stickiness: bool,
    /// Is supply stable (not subject to commodity price swings)?
    pub has_supply_stability: bool,
    /// Is the company a market leader in its niche?
    pub is_market_leader: bool,

    // === Quantitative Indicators (定量指标) ===
    /// Long-term gross margin (毛利率) - target: >30%
    pub gross_margin: f64,
    /// Long-term net margin (净利率) - target: >15%
    pub net_margin: f64,
    /// Free cash flow conversion rate (自由现金流转换率) - target: >80%
    pub fcf_conversion_rate: f64,
    /// Capital expenditure to revenue ratio (资本支出占比) - target: <20%
    pub capex_to_revenue: f64,
    /// Return on Equity (ROE) - target: >15%
    pub roe: f64,
    /// ROE driver type from DuPont analysis
    pub roe_driver: RoeDriver,
    /// Interest coverage ratio (利息保障倍数) - target: >5x
    pub interest_coverage: f64,
    /// Cash flow DNA pattern
    pub cash_flow_dna: CashFlowDNA,

    // === Scoring ===
    /// Qualitative score (0-100)
    pub qualitative_score: f64,
    /// Quantitative score (0-100)
    pub quantitative_score: f64,
    /// Overall score (0-100)
    pub overall_score: f64,
    /// Detailed reasoning
    pub reasoning: String,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

impl PrintingMachineChecklist {
    /// Returns whether this company passes the printing machine test.
    pub fn is_printing_machine(&self) -> bool {
        self.overall_score >= 70.0
    }

    /// Returns the number of qualitative criteria passed.
    pub fn qualitative_pass_count(&self) -> usize {
        let criteria = [
            self.has_evaluation_power,
            self.is_simple_and_understandable,
            self.has_demand_stickiness,
            self.has_supply_stability,
            self.is_market_leader,
        ];
        criteria.iter().filter(|&&x| x).count()
    }

    /// Returns the number of quantitative criteria passed.
    pub fn quantitative_pass_count(&self) -> usize {
        let mut count = 0;
        if self.gross_margin >= 30.0 {
            count += 1;
        }
        if self.net_margin >= 15.0 {
            count += 1;
        }
        if self.fcf_conversion_rate >= 80.0 {
            count += 1;
        }
        if self.capex_to_revenue <= 20.0 {
            count += 1;
        }
        if self.roe >= 15.0 {
            count += 1;
        }
        if self.interest_coverage >= 5.0 {
            count += 1;
        }
        if self.cash_flow_dna.is_healthy() {
            count += 1;
        }
        count
    }
}

// ============================================================================
// Financial Data Input Types
// ============================================================================

/// Raw financial data input for analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialData {
    /// Symbol/ticker
    pub symbol: String,
    /// Reporting period end date
    pub period_end: DateTime<Utc>,

    // === Income Statement ===
    /// Total revenue
    pub revenue: f64,
    /// Gross profit
    pub gross_profit: f64,
    /// Operating income
    pub operating_income: f64,
    /// Net income
    pub net_income: f64,
    /// Interest expense
    pub interest_expense: f64,

    // === Balance Sheet ===
    /// Total assets
    pub total_assets: f64,
    /// Total equity
    pub total_equity: f64,
    /// Total liabilities
    pub total_liabilities: f64,
    /// Cash and equivalents
    pub cash: f64,
    /// Total debt (short + long term)
    pub total_debt: f64,

    // === Cash Flow Statement ===
    /// Operating cash flow
    pub operating_cash_flow: f64,
    /// Investing cash flow
    pub investing_cash_flow: f64,
    /// Financing cash flow
    pub financing_cash_flow: f64,
    /// Capital expenditure (usually negative)
    pub capex: f64,
    /// Free cash flow (OCF + CapEx)
    pub free_cash_flow: f64,

    // === Historical averages (optional, for multi-year analysis) ===
    /// Average ROE over past 5 years
    pub avg_roe_5y: Option<f64>,
    /// Average gross margin over past 5 years
    pub avg_gross_margin_5y: Option<f64>,
    /// Average net margin over past 5 years
    pub avg_net_margin_5y: Option<f64>,
}

impl FinancialData {
    /// Calculate gross margin percentage.
    pub fn gross_margin_pct(&self) -> f64 {
        if self.revenue > 0.0 {
            (self.gross_profit / self.revenue) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate net margin percentage.
    pub fn net_margin_pct(&self) -> f64 {
        if self.revenue > 0.0 {
            (self.net_income / self.revenue) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate ROE percentage.
    pub fn roe_pct(&self) -> f64 {
        if self.total_equity > 0.0 {
            (self.net_income / self.total_equity) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate FCF conversion rate (FCF / Net Income).
    pub fn fcf_conversion_rate(&self) -> f64 {
        if self.net_income > 0.0 {
            (self.free_cash_flow / self.net_income) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate CapEx to revenue ratio.
    pub fn capex_to_revenue_pct(&self) -> f64 {
        if self.revenue > 0.0 {
            (self.capex.abs() / self.revenue) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate interest coverage ratio.
    pub fn interest_coverage_ratio(&self) -> f64 {
        if self.interest_expense > 0.0 {
            self.operating_income / self.interest_expense
        } else {
            f64::INFINITY
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_theme_display() {
        assert_eq!(SafetyTheme::EnergySecurity.to_string(), "能源安全");
        assert_eq!(SafetyTheme::FoodSecurity.to_string(), "粮食安全");
    }

    #[test]
    fn test_evaluation_tier_display() {
        assert_eq!(EvaluationTier::Primary.to_string(), "一级评估者");
        assert_eq!(EvaluationTier::Evaluated.to_string(), "被评估者");
    }

    #[test]
    fn test_moat_type_display() {
        assert_eq!(MoatType::SpiritualTotem.to_string(), "精神图腾");
        assert_eq!(MoatType::NetworkEffect.to_string(), "网络效应");
    }

    #[test]
    fn test_cash_flow_dna_health() {
        assert!(CashFlowDNA::CashCow.is_healthy());
        assert!(CashFlowDNA::GrowthExpansion.is_healthy());
        assert!(!CashFlowDNA::PonziScheme.is_healthy());
        assert!(!CashFlowDNA::DeclineLiquidation.is_healthy());
    }

    #[test]
    fn test_cash_flow_dna_risk() {
        assert_eq!(CashFlowDNA::CashCow.risk_level(), 1);
        assert_eq!(CashFlowDNA::PonziScheme.risk_level(), 10);
    }

    #[test]
    fn test_financial_data_calculations() {
        let data = FinancialData {
            symbol: "TEST".to_string(),
            period_end: Utc::now(),
            revenue: 1000.0,
            gross_profit: 400.0,
            operating_income: 200.0,
            net_income: 150.0,
            interest_expense: 20.0,
            total_assets: 5000.0,
            total_equity: 2000.0,
            total_liabilities: 3000.0,
            cash: 500.0,
            total_debt: 1000.0,
            operating_cash_flow: 180.0,
            investing_cash_flow: -100.0,
            financing_cash_flow: -50.0,
            capex: -80.0,
            free_cash_flow: 100.0,
            avg_roe_5y: Some(12.0),
            avg_gross_margin_5y: Some(38.0),
            avg_net_margin_5y: Some(14.0),
        };

        assert!((data.gross_margin_pct() - 40.0).abs() < 0.01);
        assert!((data.net_margin_pct() - 15.0).abs() < 0.01);
        assert!((data.roe_pct() - 7.5).abs() < 0.01);
        assert!((data.interest_coverage_ratio() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_printing_machine_pass_counts() {
        let checklist = PrintingMachineChecklist {
            symbol: "TEST".to_string(),
            has_evaluation_power: true,
            is_simple_and_understandable: true,
            has_demand_stickiness: true,
            has_supply_stability: false,
            is_market_leader: true,
            gross_margin: 35.0,
            net_margin: 18.0,
            fcf_conversion_rate: 90.0,
            capex_to_revenue: 15.0,
            roe: 20.0,
            roe_driver: RoeDriver::HighMargin,
            interest_coverage: 8.0,
            cash_flow_dna: CashFlowDNA::CashCow,
            qualitative_score: 80.0,
            quantitative_score: 85.0,
            overall_score: 82.5,
            reasoning: "Test".to_string(),
            analyzed_at: Utc::now(),
        };

        assert_eq!(checklist.qualitative_pass_count(), 4);
        assert_eq!(checklist.quantitative_pass_count(), 7);
        assert!(checklist.is_printing_machine());
    }
}
