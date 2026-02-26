//! Golden Pit vs Value Trap Assessment.
//!
//! Helps distinguish between genuine buying opportunities (黄金坑) and
//! value traps (价值陷阱) when stock prices decline significantly.
//!
//! # Philosophy
//!
//! Not every dip is an opportunity. The checklist helps answer:
//! "Is this a temporary setback in a great business, or the market correctly
//! pricing in permanent impairment?"
//!
//! # Golden Pit Criteria
//!
//! 1. **External driver**: Decline caused by macro/sector factors, not company-specific issues
//! 2. **Moat intact**: Competitive advantage unchanged or strengthened
//! 3. **Balance sheet healthy**: Can survive extended downturn
//! 4. **Insider buying**: Management putting money where their mouth is
//! 5. **Valuation attractive**: Significant margin of safety from intrinsic value

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Dip Assessment Types
// ============================================================================

/// Decline driver category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeclineDriver {
    /// External/macro factors (potentially good buying opportunity)
    External,
    /// Company-specific issues (higher risk)
    Internal,
    /// Mixed factors
    Mixed,
    /// Unknown/unclear
    Unknown,
}

impl std::fmt::Display for DeclineDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::External => write!(f, "外部因素"),
            Self::Internal => write!(f, "内部因素"),
            Self::Mixed => write!(f, "混合因素"),
            Self::Unknown => write!(f, "未知"),
        }
    }
}

/// Moat status assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoatStatus {
    /// Moat is intact and functioning
    Intact,
    /// Moat has been strengthened (competitors weakened)
    Strengthened,
    /// Moat is under pressure but not broken
    UnderPressure,
    /// Moat has been breached or eliminated
    Breached,
}

impl std::fmt::Display for MoatStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Intact => write!(f, "护城河完好"),
            Self::Strengthened => write!(f, "护城河加深"),
            Self::UnderPressure => write!(f, "护城河承压"),
            Self::Breached => write!(f, "护城河失效"),
        }
    }
}

/// Balance sheet health assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BalanceSheetHealth {
    /// Fortress balance sheet - can survive anything
    Fortress,
    /// Healthy - adequate resources
    Healthy,
    /// Adequate - can survive typical downturn
    Adequate,
    /// Stressed - limited financial flexibility
    Stressed,
    /// Distressed - survival at risk
    Distressed,
}

impl std::fmt::Display for BalanceSheetHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fortress => write!(f, "铜墙铁壁"),
            Self::Healthy => write!(f, "健康"),
            Self::Adequate => write!(f, "尚可"),
            Self::Stressed => write!(f, "承压"),
            Self::Distressed => write!(f, "危机"),
        }
    }
}

/// Insider activity indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InsiderActivity {
    /// Significant insider buying
    Buying,
    /// Company buyback announced or ongoing
    Buyback,
    /// No significant activity
    Neutral,
    /// Insider selling
    Selling,
}

impl std::fmt::Display for InsiderActivity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buying => write!(f, "内部人增持"),
            Self::Buyback => write!(f, "公司回购"),
            Self::Neutral => write!(f, "无显著动作"),
            Self::Selling => write!(f, "内部人减持"),
        }
    }
}

// ============================================================================
// Dip Checklist
// ============================================================================

/// Checklist for evaluating a price decline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipChecklist {
    // === Decline Analysis ===
    /// What's driving the decline?
    pub decline_driver: DeclineDriver,
    /// Specific reasons for the decline
    pub decline_reasons: Vec<String>,

    // === Moat Analysis ===
    /// Current moat status
    pub moat_status: MoatStatus,
    /// Moat analysis notes
    pub moat_notes: String,

    // === Balance Sheet Analysis ===
    /// Balance sheet health
    pub balance_sheet_health: BalanceSheetHealth,
    /// Cash position adequate for 2+ years
    pub cash_runway_adequate: bool,
    /// Debt levels manageable
    pub debt_manageable: bool,

    // === Insider Analysis ===
    /// Recent insider activity
    pub insider_activity: InsiderActivity,
    /// Insider activity details
    pub insider_notes: String,

    // === Valuation Analysis ===
    /// Is valuation attractive?
    pub valuation_attractive: bool,
    /// Current margin of safety (%)
    pub margin_of_safety: f64,
    /// Valuation notes
    pub valuation_notes: String,

    // === Additional Factors ===
    /// Industry cycle position (if cyclical)
    pub industry_cycle_position: Option<String>,
    /// Catalyst for recovery identified
    pub recovery_catalyst: Option<String>,
    /// Any red flags noted
    pub red_flags: Vec<String>,
}

impl Default for DipChecklist {
    fn default() -> Self {
        Self {
            decline_driver: DeclineDriver::Unknown,
            decline_reasons: Vec::new(),
            moat_status: MoatStatus::Intact,
            moat_notes: String::new(),
            balance_sheet_health: BalanceSheetHealth::Adequate,
            cash_runway_adequate: true,
            debt_manageable: true,
            insider_activity: InsiderActivity::Neutral,
            insider_notes: String::new(),
            valuation_attractive: false,
            margin_of_safety: 0.0,
            valuation_notes: String::new(),
            industry_cycle_position: None,
            recovery_catalyst: None,
            red_flags: Vec::new(),
        }
    }
}

// ============================================================================
// Dip Assessment Result
// ============================================================================

/// Overall dip assessment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipAssessment {
    /// Symbol being assessed
    pub symbol: String,
    /// Is this a golden pit opportunity?
    pub is_golden_pit: bool,
    /// Assessment score (0-100)
    pub score: f64,
    /// The completed checklist
    pub checklist: DipChecklist,
    /// Recommended buying strategy
    pub recommended_strategy: PyramidStrategy,
    /// Confidence level (0-100)
    pub confidence: f64,
    /// Summary reasoning
    pub reasoning: String,
    /// Assessment timestamp
    pub assessed_at: DateTime<Utc>,
}

impl DipAssessment {
    /// Check if this is a high-conviction opportunity.
    pub fn is_high_conviction(&self) -> bool {
        self.is_golden_pit && self.confidence >= 70.0
    }

    /// Get suggested position sizing based on conviction.
    pub fn suggested_sizing(&self) -> &str {
        if self.confidence >= 80.0 && self.is_golden_pit {
            "大仓位 (8-12% of portfolio)"
        } else if self.confidence >= 60.0 && self.is_golden_pit {
            "中等仓位 (4-8% of portfolio)"
        } else if self.is_golden_pit {
            "小仓位 (2-4% of portfolio)"
        } else {
            "观望，暂不建仓"
        }
    }
}

// ============================================================================
// Pyramid Strategy
// ============================================================================

/// Pyramid buying strategy type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PyramidStrategy {
    /// Normal pyramid: reduce size as price drops (lower conviction)
    Normal,
    /// Inverted pyramid: increase size as price drops (high conviction only)
    Inverted,
    /// Equal tranches: same size at each level
    Equal,
    /// No action: don't buy the dip
    NoAction,
}

impl std::fmt::Display for PyramidStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "正金字塔（递减加仓）"),
            Self::Inverted => write!(f, "倒金字塔（递增加仓）"),
            Self::Equal => write!(f, "等额加仓"),
            Self::NoAction => write!(f, "暂不行动"),
        }
    }
}

/// A single tranche in a pyramid buying plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidTranche {
    /// Trigger price for this tranche
    pub trigger_price: f64,
    /// Percentage of total budget to allocate
    pub allocation_pct: f64,
    /// Minimum days between tranches
    pub min_interval_days: u32,
    /// Whether this tranche has been executed
    pub executed: bool,
    /// Execution timestamp if executed
    pub executed_at: Option<DateTime<Utc>>,
}

/// Complete pyramid buying plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PyramidPlan {
    /// Symbol
    pub symbol: String,
    /// Strategy type
    pub strategy: PyramidStrategy,
    /// Total budget for this plan
    pub total_budget: f64,
    /// Individual tranches
    pub tranches: Vec<PyramidTranche>,
    /// Plan creation date
    pub created_at: DateTime<Utc>,
    /// Plan expiration date
    pub expires_at: DateTime<Utc>,
}

impl PyramidPlan {
    /// Create a normal pyramid plan (decreasing tranche sizes).
    pub fn normal(symbol: &str, budget: f64, current_price: f64, target_low: f64) -> Self {
        let price_drop = current_price - target_low;
        let step = price_drop / 4.0;

        let tranches = vec![
            PyramidTranche {
                trigger_price: current_price,
                allocation_pct: 40.0,
                min_interval_days: 0,
                executed: false,
                executed_at: None,
            },
            PyramidTranche {
                trigger_price: current_price - step,
                allocation_pct: 30.0,
                min_interval_days: 7,
                executed: false,
                executed_at: None,
            },
            PyramidTranche {
                trigger_price: current_price - 2.0 * step,
                allocation_pct: 20.0,
                min_interval_days: 14,
                executed: false,
                executed_at: None,
            },
            PyramidTranche {
                trigger_price: target_low,
                allocation_pct: 10.0,
                min_interval_days: 21,
                executed: false,
                executed_at: None,
            },
        ];

        Self {
            symbol: symbol.to_string(),
            strategy: PyramidStrategy::Normal,
            total_budget: budget,
            tranches,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(90),
        }
    }

    /// Create an inverted pyramid plan (increasing tranche sizes) - for high conviction.
    pub fn inverted(symbol: &str, budget: f64, current_price: f64, target_low: f64) -> Self {
        let price_drop = current_price - target_low;
        let step = price_drop / 4.0;

        let tranches = vec![
            PyramidTranche {
                trigger_price: current_price,
                allocation_pct: 10.0,
                min_interval_days: 0,
                executed: false,
                executed_at: None,
            },
            PyramidTranche {
                trigger_price: current_price - step,
                allocation_pct: 20.0,
                min_interval_days: 7,
                executed: false,
                executed_at: None,
            },
            PyramidTranche {
                trigger_price: current_price - 2.0 * step,
                allocation_pct: 30.0,
                min_interval_days: 14,
                executed: false,
                executed_at: None,
            },
            PyramidTranche {
                trigger_price: target_low,
                allocation_pct: 40.0,
                min_interval_days: 21,
                executed: false,
                executed_at: None,
            },
        ];

        Self {
            symbol: symbol.to_string(),
            strategy: PyramidStrategy::Inverted,
            total_budget: budget,
            tranches,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(90),
        }
    }

    /// Get remaining budget not yet allocated.
    pub fn remaining_budget(&self) -> f64 {
        let executed_pct: f64 = self
            .tranches
            .iter()
            .filter(|t| t.executed)
            .map(|t| t.allocation_pct)
            .sum();
        self.total_budget * (1.0 - executed_pct / 100.0)
    }

    /// Check if plan is expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Get next tranche to execute (if price condition met).
    pub fn next_tranche(&self, current_price: f64) -> Option<&PyramidTranche> {
        self.tranches
            .iter()
            .filter(|t| !t.executed && current_price <= t.trigger_price)
            .next()
    }
}

// ============================================================================
// Dip Analyzer
// ============================================================================

/// Configuration for dip analysis.
#[derive(Debug, Clone)]
pub struct DipAnalyzerConfig {
    /// Minimum margin of safety to consider golden pit (%)
    pub min_margin_of_safety: f64,
    /// Golden pit score threshold
    pub golden_pit_threshold: f64,
    /// High conviction threshold
    pub high_conviction_threshold: f64,
}

impl Default for DipAnalyzerConfig {
    fn default() -> Self {
        Self {
            min_margin_of_safety: 20.0,
            golden_pit_threshold: 70.0,
            high_conviction_threshold: 80.0,
        }
    }
}

/// Analyzer for dip buying opportunities.
pub struct DipAnalyzer {
    config: DipAnalyzerConfig,
}

impl DipAnalyzer {
    /// Create a new dip analyzer.
    pub fn new(config: DipAnalyzerConfig) -> Self {
        Self { config }
    }

    /// Assess a potential dip buying opportunity.
    pub fn assess(&self, symbol: &str, checklist: DipChecklist) -> DipAssessment {
        let score = self.calculate_score(&checklist);
        let confidence = self.calculate_confidence(&checklist);
        let is_golden_pit = score >= self.config.golden_pit_threshold
            && checklist.margin_of_safety >= self.config.min_margin_of_safety
            && checklist.red_flags.is_empty();

        let recommended_strategy = self.determine_strategy(&checklist, score, confidence);
        let reasoning = self.generate_reasoning(&checklist, score, is_golden_pit);

        DipAssessment {
            symbol: symbol.to_string(),
            is_golden_pit,
            score,
            checklist,
            recommended_strategy,
            confidence,
            reasoning,
            assessed_at: Utc::now(),
        }
    }

    /// Calculate assessment score (0-100).
    fn calculate_score(&self, checklist: &DipChecklist) -> f64 {
        let mut score = 0.0;

        // Decline driver (0-20 points)
        match checklist.decline_driver {
            DeclineDriver::External => score += 20.0,
            DeclineDriver::Mixed => score += 10.0,
            DeclineDriver::Internal => score += 0.0,
            DeclineDriver::Unknown => score += 5.0,
        }

        // Moat status (0-25 points)
        match checklist.moat_status {
            MoatStatus::Strengthened => score += 25.0,
            MoatStatus::Intact => score += 20.0,
            MoatStatus::UnderPressure => score += 10.0,
            MoatStatus::Breached => score += 0.0,
        }

        // Balance sheet health (0-20 points)
        match checklist.balance_sheet_health {
            BalanceSheetHealth::Fortress => score += 20.0,
            BalanceSheetHealth::Healthy => score += 16.0,
            BalanceSheetHealth::Adequate => score += 12.0,
            BalanceSheetHealth::Stressed => score += 5.0,
            BalanceSheetHealth::Distressed => score += 0.0,
        }

        // Additional balance sheet factors
        if checklist.cash_runway_adequate {
            score += 5.0;
        }
        if checklist.debt_manageable {
            score += 5.0;
        }

        // Insider activity (0-15 points)
        match checklist.insider_activity {
            InsiderActivity::Buying => score += 15.0,
            InsiderActivity::Buyback => score += 12.0,
            InsiderActivity::Neutral => score += 5.0,
            InsiderActivity::Selling => score -= 5.0,
        }

        // Valuation (0-10 points)
        if checklist.valuation_attractive {
            score += 10.0;
        }
        if checklist.margin_of_safety >= 30.0 {
            score += 5.0;
        }

        // Bonus for recovery catalyst
        if checklist.recovery_catalyst.is_some() {
            score += 5.0;
        }

        // Penalty for red flags
        score -= (checklist.red_flags.len() as f64) * 10.0;

        score.clamp(0.0, 100.0)
    }

    /// Calculate confidence level (0-100).
    fn calculate_confidence(&self, checklist: &DipChecklist) -> f64 {
        let mut confidence = 50.0; // Base confidence

        // Higher confidence with clear decline driver
        if checklist.decline_driver == DeclineDriver::External {
            confidence += 15.0;
        }

        // Higher confidence with strong moat
        if matches!(
            checklist.moat_status,
            MoatStatus::Intact | MoatStatus::Strengthened
        ) {
            confidence += 15.0;
        }

        // Higher confidence with fortress balance sheet
        if matches!(
            checklist.balance_sheet_health,
            BalanceSheetHealth::Fortress | BalanceSheetHealth::Healthy
        ) {
            confidence += 10.0;
        }

        // Higher confidence with insider buying
        if matches!(
            checklist.insider_activity,
            InsiderActivity::Buying | InsiderActivity::Buyback
        ) {
            confidence += 10.0;
        }

        // Higher confidence with significant margin of safety
        if checklist.margin_of_safety >= 30.0 {
            confidence += 10.0;
        }

        // Lower confidence with red flags
        confidence -= (checklist.red_flags.len() as f64) * 15.0;

        confidence.clamp(0.0, 100.0)
    }

    /// Determine recommended pyramid strategy.
    fn determine_strategy(
        &self,
        checklist: &DipChecklist,
        score: f64,
        confidence: f64,
    ) -> PyramidStrategy {
        if score < self.config.golden_pit_threshold || !checklist.red_flags.is_empty() {
            return PyramidStrategy::NoAction;
        }

        if confidence >= self.config.high_conviction_threshold
            && matches!(
                checklist.moat_status,
                MoatStatus::Intact | MoatStatus::Strengthened
            )
            && matches!(
                checklist.balance_sheet_health,
                BalanceSheetHealth::Fortress | BalanceSheetHealth::Healthy
            )
        {
            PyramidStrategy::Inverted
        } else if confidence >= 60.0 {
            PyramidStrategy::Normal
        } else {
            PyramidStrategy::Equal
        }
    }

    /// Generate assessment reasoning.
    fn generate_reasoning(&self, checklist: &DipChecklist, score: f64, is_golden_pit: bool) -> String {
        let mut parts = Vec::new();

        // Overall verdict
        if is_golden_pit {
            parts.push(format!("评估分数 {:.0}，判定为黄金坑机会。", score));
        } else {
            parts.push(format!("评估分数 {:.0}，暂不符合黄金坑标准。", score));
        }

        // Decline driver
        parts.push(format!("下跌驱动：{}。", checklist.decline_driver));
        if !checklist.decline_reasons.is_empty() {
            parts.push(format!("具体原因：{}。", checklist.decline_reasons.join("、")));
        }

        // Moat status
        parts.push(format!("护城河状态：{}。", checklist.moat_status));
        if !checklist.moat_notes.is_empty() {
            parts.push(checklist.moat_notes.clone());
        }

        // Balance sheet
        parts.push(format!("资产负债表：{}。", checklist.balance_sheet_health));

        // Insider activity
        if checklist.insider_activity != InsiderActivity::Neutral {
            parts.push(format!("内部人动向：{}。", checklist.insider_activity));
        }

        // Valuation
        if checklist.valuation_attractive {
            parts.push(format!(
                "估值具有吸引力，安全边际 {:.0}%。",
                checklist.margin_of_safety
            ));
        }

        // Red flags
        if !checklist.red_flags.is_empty() {
            parts.push(format!("警示信号：{}。", checklist.red_flags.join("、")));
        }

        // Recovery catalyst
        if let Some(ref catalyst) = checklist.recovery_catalyst {
            parts.push(format!("潜在催化剂：{}。", catalyst));
        }

        parts.join("\n")
    }
}

impl Default for DipAnalyzer {
    fn default() -> Self {
        Self::new(DipAnalyzerConfig::default())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_golden_pit_assessment() {
        let analyzer = DipAnalyzer::default();

        let checklist = DipChecklist {
            decline_driver: DeclineDriver::External,
            decline_reasons: vec!["宏观经济下行".to_string()],
            moat_status: MoatStatus::Intact,
            moat_notes: "品牌护城河依然强劲".to_string(),
            balance_sheet_health: BalanceSheetHealth::Fortress,
            cash_runway_adequate: true,
            debt_manageable: true,
            insider_activity: InsiderActivity::Buying,
            insider_notes: "管理层增持".to_string(),
            valuation_attractive: true,
            margin_of_safety: 35.0,
            valuation_notes: "PE处于历史低位".to_string(),
            industry_cycle_position: None,
            recovery_catalyst: Some("政策刺激预期".to_string()),
            red_flags: vec![],
        };

        let assessment = analyzer.assess("600519", checklist);

        assert!(assessment.is_golden_pit);
        assert!(assessment.score >= 70.0);
        assert!(assessment.confidence >= 60.0);
        assert_eq!(assessment.recommended_strategy, PyramidStrategy::Inverted);
    }

    #[test]
    fn test_value_trap_detection() {
        let analyzer = DipAnalyzer::default();

        let checklist = DipChecklist {
            decline_driver: DeclineDriver::Internal,
            decline_reasons: vec!["产品质量问题".to_string()],
            moat_status: MoatStatus::Breached,
            moat_notes: "竞争对手蚕食份额".to_string(),
            balance_sheet_health: BalanceSheetHealth::Stressed,
            cash_runway_adequate: false,
            debt_manageable: false,
            insider_activity: InsiderActivity::Selling,
            insider_notes: "高管持续减持".to_string(),
            valuation_attractive: true, // Looks cheap but...
            margin_of_safety: 40.0,
            valuation_notes: "PE很低但...".to_string(),
            industry_cycle_position: None,
            recovery_catalyst: None,
            red_flags: vec!["应收账款激增".to_string(), "存货周转下降".to_string()],
        };

        let assessment = analyzer.assess("TRAP", checklist);

        assert!(!assessment.is_golden_pit);
        assert!(assessment.score < 50.0);
        assert_eq!(assessment.recommended_strategy, PyramidStrategy::NoAction);
    }

    #[test]
    fn test_pyramid_plan_normal() {
        let plan = PyramidPlan::normal("TEST", 100_000.0, 100.0, 70.0);

        assert_eq!(plan.strategy, PyramidStrategy::Normal);
        assert_eq!(plan.tranches.len(), 4);
        assert!((plan.tranches[0].allocation_pct - 40.0).abs() < 0.01);
        assert!((plan.tranches[3].allocation_pct - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_pyramid_plan_inverted() {
        let plan = PyramidPlan::inverted("TEST", 100_000.0, 100.0, 70.0);

        assert_eq!(plan.strategy, PyramidStrategy::Inverted);
        assert!((plan.tranches[0].allocation_pct - 10.0).abs() < 0.01);
        assert!((plan.tranches[3].allocation_pct - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_pyramid_remaining_budget() {
        let mut plan = PyramidPlan::normal("TEST", 100_000.0, 100.0, 70.0);

        // Execute first tranche (40%)
        plan.tranches[0].executed = true;

        let remaining = plan.remaining_budget();
        assert!((remaining - 60_000.0).abs() < 0.01);
    }

    #[test]
    fn test_dip_assessment_sizing() {
        let assessment = DipAssessment {
            symbol: "TEST".to_string(),
            is_golden_pit: true,
            score: 85.0,
            checklist: DipChecklist::default(),
            recommended_strategy: PyramidStrategy::Inverted,
            confidence: 85.0,
            reasoning: "High conviction opportunity".to_string(),
            assessed_at: Utc::now(),
        };

        assert!(assessment.suggested_sizing().contains("大仓位"));
        assert!(assessment.is_high_conviction());
    }
}
