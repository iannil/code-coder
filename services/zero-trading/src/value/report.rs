//! Value Analysis Report Generator.
//!
//! Generates comprehensive value analysis reports by combining all analysis modules:
//! - Financial verification (printing machine checklist)
//! - Cash flow DNA classification
//! - Evaluation power assessment
//! - Valuation coordinates (PE/PB/DY)
//! - Portfolio signals and recommendations
//!
//! # Report Types
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     价值分析报告类型                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  个股深度报告 (StockDeepDive)                                    │
//! │    └─ 单个标的的完整价值分析                                     │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  组合审视报告 (PortfolioReview)                                  │
//! │    └─ 当前持仓的信号和再平衡建议                                 │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  机会扫描报告 (OpportunityScan)                                  │
//! │    └─ 观察名单中的黄金坑评估                                     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::portfolio::{
    AllocationRecommendation, CapitalAllocator, DipAnalyzer, DipAssessment, DipChecklist,
    PortfolioPools, SignalAnalyzer, SignalAssessment, SignalContext,
};
use crate::valuation::{ValuationAnalyzer, ValuationCoordinates, ValuationInput};
use crate::value::{
    CashFlowDNA, EvaluationPowerScore, FinancialData, PrintingMachineChecklist, QualitativeInputs,
    RoeDriver, ValueAnalyzer,
};

// ============================================================================
// Report Types
// ============================================================================

/// Type of value analysis report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueReportType {
    /// Deep dive analysis of a single stock
    StockDeepDive,
    /// Portfolio review with signals and rebalancing
    PortfolioReview,
    /// Opportunity scan for watchlist
    OpportunityScan,
    /// Quick valuation check
    QuickValuation,
}

impl std::fmt::Display for ValueReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StockDeepDive => write!(f, "个股深度分析"),
            Self::PortfolioReview => write!(f, "组合审视报告"),
            Self::OpportunityScan => write!(f, "机会扫描报告"),
            Self::QuickValuation => write!(f, "快速估值检查"),
        }
    }
}

// ============================================================================
// Stock Deep Dive Report
// ============================================================================

/// Complete value analysis for a single stock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockDeepDiveReport {
    /// Stock symbol
    pub symbol: String,
    /// Company name
    pub name: String,
    /// Report type
    pub report_type: ValueReportType,

    // === Layer 1: Financial Verification ===
    /// Printing machine checklist result
    pub printing_machine: Option<PrintingMachineChecklist>,
    /// Cash flow DNA classification
    pub cash_flow_dna: Option<CashFlowDNA>,
    /// ROE driver analysis
    pub roe_driver: Option<RoeDriver>,

    // === Layer 2: Evaluation Power ===
    /// Evaluation power assessment
    pub evaluation_power: Option<EvaluationPowerScore>,

    // === Layer 3: Valuation ===
    /// Three-dimensional valuation coordinates
    pub valuation: Option<ValuationCoordinates>,

    // === Synthesis ===
    /// Overall investment verdict
    pub verdict: InvestmentVerdict,
    /// Key highlights
    pub highlights: Vec<String>,
    /// Risk factors
    pub risks: Vec<String>,
    /// Action recommendation
    pub recommendation: String,

    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
}

/// Investment verdict classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvestmentVerdict {
    /// Strong buy - excellent quality at attractive price
    StrongBuy,
    /// Buy - good quality at fair price
    Buy,
    /// Hold - decent quality, wait for better entry
    Hold,
    /// Reduce - concerns about quality or valuation
    Reduce,
    /// Avoid - significant red flags
    Avoid,
}

impl std::fmt::Display for InvestmentVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StrongBuy => write!(f, "强烈买入"),
            Self::Buy => write!(f, "买入"),
            Self::Hold => write!(f, "持有/观望"),
            Self::Reduce => write!(f, "减持"),
            Self::Avoid => write!(f, "回避"),
        }
    }
}

impl StockDeepDiveReport {
    /// Generate markdown summary.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str(&format!("# {} ({}) 价值分析报告\n\n", self.name, self.symbol));
        md.push_str(&format!("**生成时间**: {}\n\n", self.generated_at.format("%Y-%m-%d %H:%M")));
        md.push_str(&format!("**投资建议**: {}\n\n", self.verdict));

        // Highlights
        if !self.highlights.is_empty() {
            md.push_str("## 核心亮点\n\n");
            for highlight in &self.highlights {
                md.push_str(&format!("- {}\n", highlight));
            }
            md.push('\n');
        }

        // Financial Quality
        md.push_str("## 财务质量分析\n\n");
        if let Some(ref pm) = self.printing_machine {
            md.push_str(&format!("**印钞机评分**: {:.0}/100\n\n", pm.overall_score));
            md.push_str(&format!("- 定性指标: {}/5 通过\n", pm.qualitative_pass_count()));
            md.push_str(&format!("- 定量指标: {}/5 通过\n", pm.quantitative_pass_count()));
            md.push_str(&format!("- 评定: {}\n\n", if pm.is_printing_machine() { "✅ 印钞机" } else { "❌ 非印钞机" }));
        }

        if let Some(ref dna) = self.cash_flow_dna {
            md.push_str(&format!("**现金流DNA**: {}\n\n", dna));
        }

        if let Some(ref driver) = self.roe_driver {
            md.push_str(&format!("**ROE驱动**: {}\n\n", driver));
        }

        // Evaluation Power
        if let Some(ref power) = self.evaluation_power {
            md.push_str("## 评估权分析\n\n");
            md.push_str(&format!("**评估权层级**: {} (评分: {:.0})\n\n", power.tier, power.score));
            md.push_str(&format!("**主要护城河**: {}\n\n", power.moat_type));
            md.push_str(&format!("- 定价权: {:.0}\n", power.pricing_power));
            md.push_str(&format!("- 上游依赖: {:.0}\n", power.upstream_dependency));
            md.push_str(&format!("- 下游依赖: {:.0}\n\n", power.downstream_dependency));
        }

        // Valuation
        if let Some(ref val) = self.valuation {
            md.push_str("## 估值分析\n\n");
            md.push_str(&format!("**估值区间**: {} (评分: {:.0})\n\n", val.valuation_zone, val.overall_score));
            md.push_str(&format!("**安全边际**: {:.1}%\n\n", val.margin_of_safety));

            md.push_str("### PE分析\n");
            md.push_str(&format!("- 当前PE: {:.1}\n", val.pe_analysis.pe_ttm));
            md.push_str(&format!("- 历史分位: {:.0}%\n", val.pe_analysis.percentile));
            md.push_str(&format!("- 位置: {}\n\n", val.pe_analysis.position));

            md.push_str("### PB-ROE分析\n");
            md.push_str(&format!("- 当前PB: {:.2}\n", val.pb_analysis.pb));
            md.push_str(&format!("- 公允PB: {:.2}\n", val.pb_analysis.fair_pb));
            md.push_str(&format!("- {}\n\n", if val.pb_analysis.is_justified { "✅ PB合理" } else { "⚠️ PB可能偏高" }));
        }

        // Risks
        if !self.risks.is_empty() {
            md.push_str("## 风险提示\n\n");
            for risk in &self.risks {
                md.push_str(&format!("- ⚠️ {}\n", risk));
            }
            md.push('\n');
        }

        // Recommendation
        md.push_str("## 操作建议\n\n");
        md.push_str(&format!("{}\n", self.recommendation));

        md
    }

    /// Generate plain text summary for notifications.
    pub fn to_text(&self) -> String {
        let mut text = String::new();

        text.push_str(&format!("【{}价值分析】{} ({})\n\n", self.verdict, self.name, self.symbol));

        // Key metrics
        if let Some(ref pm) = self.printing_machine {
            text.push_str(&format!("印钞机评分: {:.0}/100\n", pm.overall_score));
        }
        if let Some(ref val) = self.valuation {
            text.push_str(&format!("估值区间: {}\n", val.valuation_zone));
            text.push_str(&format!("安全边际: {:.1}%\n", val.margin_of_safety));
        }
        if let Some(ref power) = self.evaluation_power {
            text.push_str(&format!("评估权: {} ({:.0}分)\n", power.tier, power.score));
        }

        text.push_str(&format!("\n{}\n", self.recommendation));

        text
    }
}

// ============================================================================
// Portfolio Review Report
// ============================================================================

/// Portfolio review report with signals and recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioReviewReport {
    /// Report type
    pub report_type: ValueReportType,
    /// Portfolio summary
    pub total_positions: usize,
    pub total_value: f64,
    pub total_pnl_pct: f64,

    /// Positions with triggered signals
    pub signal_alerts: Vec<PositionSignalAlert>,
    /// Allocation recommendations
    pub allocation: AllocationRecommendation,
    /// Overall portfolio health score (0-100)
    pub health_score: f64,
    /// Key observations
    pub observations: Vec<String>,

    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
}

/// Signal alert for a position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSignalAlert {
    pub symbol: String,
    pub name: String,
    pub signal: SignalAssessment,
}

impl PortfolioReviewReport {
    /// Generate markdown summary.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# 组合审视报告\n\n");
        md.push_str(&format!("**生成时间**: {}\n\n", self.generated_at.format("%Y-%m-%d %H:%M")));

        // Portfolio Overview
        md.push_str("## 组合概览\n\n");
        md.push_str(&format!("- 持仓数量: {}\n", self.total_positions));
        md.push_str(&format!("- 总市值: {:.0}\n", self.total_value));
        md.push_str(&format!("- 总收益: {:.1}%\n", self.total_pnl_pct));
        md.push_str(&format!("- 健康评分: {:.0}/100\n\n", self.health_score));

        // Signal Alerts
        if !self.signal_alerts.is_empty() {
            md.push_str("## 信号警报\n\n");
            for alert in &self.signal_alerts {
                let emoji = match alert.signal.level {
                    crate::portfolio::SignalLevel::Red => "🔴",
                    crate::portfolio::SignalLevel::Yellow => "🟡",
                    crate::portfolio::SignalLevel::Green => "🟢",
                };
                md.push_str(&format!("{} **{}**: {}\n", emoji, alert.symbol, alert.signal.level));
                md.push_str(&format!("   建议: {}\n\n", alert.signal.recommended_action));
            }
        }

        // Allocation Recommendations
        md.push_str("## 资金配置建议\n\n");
        md.push_str(&format!("{}\n\n", self.allocation.strategy_summary));

        if !self.allocation.trim_candidates.is_empty() {
            md.push_str("### 减仓建议\n");
            for trim in &self.allocation.trim_candidates {
                md.push_str(&format!("- {}: 减仓{}% ({})\n", trim.symbol, trim.trim_pct as u32, trim.reason));
            }
            md.push('\n');
        }

        if !self.allocation.add_candidates.is_empty() {
            md.push_str("### 加仓建议\n");
            for add in &self.allocation.add_candidates {
                md.push_str(&format!("- {}: 加仓{:.0} ({})\n", add.symbol, add.suggested_amount, add.reason));
            }
            md.push('\n');
        }

        // Observations
        if !self.observations.is_empty() {
            md.push_str("## 观察要点\n\n");
            for obs in &self.observations {
                md.push_str(&format!("- {}\n", obs));
            }
        }

        md
    }
}

// ============================================================================
// Opportunity Scan Report
// ============================================================================

/// Opportunity scan report for watchlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityScanReport {
    /// Report type
    pub report_type: ValueReportType,
    /// Scanned symbols
    pub symbols_scanned: usize,
    /// Golden pit opportunities found
    pub golden_pits: Vec<DipAssessment>,
    /// Value traps identified
    pub value_traps: Vec<DipAssessment>,
    /// Summary
    pub summary: String,

    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
}

impl OpportunityScanReport {
    /// Generate markdown summary.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# 机会扫描报告\n\n");
        md.push_str(&format!("**生成时间**: {}\n\n", self.generated_at.format("%Y-%m-%d %H:%M")));
        md.push_str(&format!("**扫描数量**: {} 个标的\n\n", self.symbols_scanned));

        // Golden Pits
        if !self.golden_pits.is_empty() {
            md.push_str("## 🏆 黄金坑机会\n\n");
            for pit in &self.golden_pits {
                md.push_str(&format!("### {}\n", pit.symbol));
                md.push_str(&format!("- 评分: {:.0}/100\n", pit.score));
                md.push_str(&format!("- 信心度: {:.0}%\n", pit.confidence));
                md.push_str(&format!("- 策略: {}\n", pit.recommended_strategy));
                md.push_str(&format!("- 仓位建议: {}\n\n", pit.suggested_sizing()));
            }
        } else {
            md.push_str("## 黄金坑机会\n\n暂无符合条件的机会。\n\n");
        }

        // Value Traps
        if !self.value_traps.is_empty() {
            md.push_str("## ⚠️ 价值陷阱警示\n\n");
            for trap in &self.value_traps {
                md.push_str(&format!("### {} (回避)\n", trap.symbol));
                md.push_str(&format!("- 评分: {:.0}/100\n", trap.score));
                md.push_str(&format!("- 原因: {}\n\n", trap.reasoning.lines().next().unwrap_or("")));
            }
        }

        // Summary
        md.push_str("## 总结\n\n");
        md.push_str(&format!("{}\n", self.summary));

        md
    }
}

// ============================================================================
// Report Generator
// ============================================================================

/// Value analysis report generator.
pub struct ValueReportGenerator {
    value_analyzer: ValueAnalyzer,
    valuation_analyzer: ValuationAnalyzer,
    signal_analyzer: SignalAnalyzer,
    dip_analyzer: DipAnalyzer,
    capital_allocator: CapitalAllocator,
}

impl ValueReportGenerator {
    /// Create a new report generator.
    pub fn new() -> Self {
        Self {
            value_analyzer: ValueAnalyzer::new(&Default::default()),
            valuation_analyzer: ValuationAnalyzer::new(),
            signal_analyzer: SignalAnalyzer::default(),
            dip_analyzer: DipAnalyzer::default(),
            capital_allocator: CapitalAllocator::default(),
        }
    }

    /// Generate a stock deep dive report.
    pub fn generate_stock_report(
        &self,
        symbol: &str,
        name: &str,
        financial_data: &FinancialData,
        qualitative: QualitativeInputs,
        valuation_input: Option<&ValuationInput>,
        evaluation_power: Option<EvaluationPowerScore>,
    ) -> StockDeepDiveReport {
        // Financial analysis
        let printing_machine = self
            .value_analyzer
            .analyze_printing_machine(financial_data, qualitative)
            .ok();
        let cash_flow_dna = Some(self.value_analyzer.classify_cash_flow(financial_data));
        let roe_driver = Some(self.value_analyzer.analyze_roe_driver(financial_data));

        // Valuation analysis
        let valuation = valuation_input.and_then(|input| self.valuation_analyzer.analyze(input).ok());

        // Generate highlights
        let highlights = self.generate_highlights(&printing_machine, &cash_flow_dna, &valuation, &evaluation_power);

        // Generate risks
        let risks = self.generate_risks(&printing_machine, &cash_flow_dna, &valuation, &evaluation_power);

        // Determine verdict
        let verdict = self.determine_verdict(&printing_machine, &valuation, &evaluation_power);

        // Generate recommendation
        let recommendation = self.generate_recommendation(verdict, &valuation, &evaluation_power);

        StockDeepDiveReport {
            symbol: symbol.to_string(),
            name: name.to_string(),
            report_type: ValueReportType::StockDeepDive,
            printing_machine,
            cash_flow_dna,
            roe_driver,
            evaluation_power,
            valuation,
            verdict,
            highlights,
            risks,
            recommendation,
            generated_at: Utc::now(),
        }
    }

    /// Generate a portfolio review report.
    pub fn generate_portfolio_report(
        &self,
        portfolio: &PortfolioPools,
        signal_contexts: &[(String, SignalContext)],
        available_capital: f64,
    ) -> PortfolioReviewReport {
        let summary = portfolio.summary();

        // Analyze signals for each position
        let mut signal_alerts = Vec::new();
        for position in portfolio.all_positions() {
            if let Some((_, context)) = signal_contexts.iter().find(|(s, _)| s == &position.symbol) {
                let signal = self.signal_analyzer.analyze(position, context);
                if signal.level != crate::portfolio::SignalLevel::Green {
                    signal_alerts.push(PositionSignalAlert {
                        symbol: position.symbol.clone(),
                        name: position.name.clone(),
                        signal,
                    });
                }
            }
        }

        // Generate allocation recommendation
        let allocation = self.capital_allocator.recommend_allocation(portfolio, available_capital);

        // Calculate health score
        let health_score = self.calculate_portfolio_health(portfolio, &signal_alerts);

        // Generate observations
        let observations = self.generate_portfolio_observations(portfolio, &signal_alerts, &allocation);

        PortfolioReviewReport {
            report_type: ValueReportType::PortfolioReview,
            total_positions: summary.total_positions,
            total_value: summary.total_value,
            total_pnl_pct: summary.total_pnl_pct,
            signal_alerts,
            allocation,
            health_score,
            observations,
            generated_at: Utc::now(),
        }
    }

    /// Generate an opportunity scan report.
    pub fn generate_opportunity_report(
        &self,
        checklists: Vec<(String, DipChecklist)>,
    ) -> OpportunityScanReport {
        let symbols_scanned = checklists.len();
        let mut golden_pits = Vec::new();
        let mut value_traps = Vec::new();

        for (symbol, checklist) in checklists {
            let assessment = self.dip_analyzer.assess(&symbol, checklist);

            if assessment.is_golden_pit {
                golden_pits.push(assessment);
            } else if assessment.score < 40.0 {
                value_traps.push(assessment);
            }
        }

        // Sort by score
        golden_pits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        value_traps.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());

        let summary = format!(
            "扫描 {} 个标的，发现 {} 个黄金坑机会，{} 个价值陷阱警示。{}",
            symbols_scanned,
            golden_pits.len(),
            value_traps.len(),
            if golden_pits.is_empty() {
                "建议继续观望，等待更好机会。"
            } else {
                "建议重点关注黄金坑机会。"
            }
        );

        OpportunityScanReport {
            report_type: ValueReportType::OpportunityScan,
            symbols_scanned,
            golden_pits,
            value_traps,
            summary,
            generated_at: Utc::now(),
        }
    }

    // --- Helper methods ---

    fn generate_highlights(
        &self,
        printing_machine: &Option<PrintingMachineChecklist>,
        cash_flow_dna: &Option<CashFlowDNA>,
        valuation: &Option<ValuationCoordinates>,
        evaluation_power: &Option<EvaluationPowerScore>,
    ) -> Vec<String> {
        let mut highlights = Vec::new();

        if let Some(ref pm) = printing_machine {
            if pm.is_printing_machine() {
                highlights.push("通过印钞机检验，财务质量优秀".to_string());
            }
            if pm.overall_score >= 80.0 {
                highlights.push(format!("印钞机评分 {:.0}，属于顶级质量", pm.overall_score));
            }
        }

        if let Some(ref dna) = cash_flow_dna {
            if dna.is_healthy() {
                highlights.push(format!("现金流DNA：{}，经营健康", dna));
            }
        }

        if let Some(ref val) = valuation {
            if val.margin_of_safety > 20.0 {
                highlights.push(format!("安全边际 {:.0}%，估值具吸引力", val.margin_of_safety));
            }
            if val.is_buy_zone() {
                highlights.push(format!("处于{}，具备买入价值", val.valuation_zone));
            }
        }

        if let Some(ref power) = evaluation_power {
            if power.score >= 70.0 {
                highlights.push(format!(
                    "评估权得分 {:.0}，具有较强产业链地位",
                    power.score
                ));
            }
            if power.moat_type != crate::value::MoatType::None {
                highlights.push(format!("具备{}护城河", power.moat_type));
            }
        }

        highlights
    }

    fn generate_risks(
        &self,
        printing_machine: &Option<PrintingMachineChecklist>,
        cash_flow_dna: &Option<CashFlowDNA>,
        valuation: &Option<ValuationCoordinates>,
        evaluation_power: &Option<EvaluationPowerScore>,
    ) -> Vec<String> {
        let mut risks = Vec::new();

        if let Some(ref pm) = printing_machine {
            if !pm.is_printing_machine() {
                risks.push("未通过印钞机检验，财务质量存疑".to_string());
            }
            if pm.overall_score < 50.0 {
                risks.push(format!("印钞机评分 {:.0}，财务质量较差", pm.overall_score));
            }
        }

        if let Some(ref dna) = cash_flow_dna {
            match dna {
                CashFlowDNA::PonziScheme => {
                    risks.push("现金流DNA疑似庞氏模式，高度警惕".to_string());
                }
                CashFlowDNA::DeclineLiquidation => {
                    risks.push("现金流DNA显示衰退变卖迹象".to_string());
                }
                CashFlowDNA::StartupBloodTransfusion => {
                    risks.push("现金流DNA显示输血模式，盈利模式未证实".to_string());
                }
                _ => {}
            }
        }

        if let Some(ref val) = valuation {
            if val.margin_of_safety < 0.0 {
                risks.push(format!(
                    "安全边际为负 ({:.0}%)，估值偏高",
                    val.margin_of_safety
                ));
            }
            if val.is_avoid_zone() {
                risks.push(format!("处于{}，估值风险较大", val.valuation_zone));
            }
        }

        if let Some(ref power) = evaluation_power {
            if power.score < 40.0 {
                risks.push("评估权较弱，在产业链中议价能力不足".to_string());
            }
            if power.moat_type == crate::value::MoatType::None {
                risks.push("未发现明显护城河".to_string());
            }
        }

        risks
    }

    fn determine_verdict(
        &self,
        printing_machine: &Option<PrintingMachineChecklist>,
        valuation: &Option<ValuationCoordinates>,
        evaluation_power: &Option<EvaluationPowerScore>,
    ) -> InvestmentVerdict {
        let mut score: f64 = 50.0;

        // Printing machine quality
        if let Some(ref pm) = printing_machine {
            if pm.is_printing_machine() {
                score += 20.0;
            } else {
                score -= 15.0;
            }
            score += (pm.overall_score - 50.0) * 0.3;
        }

        // Valuation
        if let Some(ref val) = valuation {
            if val.margin_of_safety > 30.0 {
                score += 20.0;
            } else if val.margin_of_safety > 15.0 {
                score += 10.0;
            } else if val.margin_of_safety < 0.0 {
                score -= 15.0;
            }

            if val.is_avoid_zone() {
                score -= 25.0;
            }
        }

        // Evaluation power
        if let Some(ref power) = evaluation_power {
            if power.score >= 70.0 {
                score += 15.0;
            } else if power.score < 40.0 {
                score -= 10.0;
            }
        }

        // Map score to verdict
        if score >= 80.0 {
            InvestmentVerdict::StrongBuy
        } else if score >= 65.0 {
            InvestmentVerdict::Buy
        } else if score >= 45.0 {
            InvestmentVerdict::Hold
        } else if score >= 30.0 {
            InvestmentVerdict::Reduce
        } else {
            InvestmentVerdict::Avoid
        }
    }

    fn generate_recommendation(
        &self,
        verdict: InvestmentVerdict,
        valuation: &Option<ValuationCoordinates>,
        evaluation_power: &Option<EvaluationPowerScore>,
    ) -> String {
        let mut parts = Vec::new();

        match verdict {
            InvestmentVerdict::StrongBuy => {
                parts.push("该标的具备优秀的财务质量和吸引力估值，建议积极建仓。".to_string());
            }
            InvestmentVerdict::Buy => {
                parts.push("该标的质量较好，当前估值合理，可考虑逐步建仓。".to_string());
            }
            InvestmentVerdict::Hold => {
                parts.push("该标的有一定投资价值，但当前不是最佳入场时机，建议持有观望。".to_string());
            }
            InvestmentVerdict::Reduce => {
                parts.push("该标的存在一定风险，建议降低仓位或等待更多信息。".to_string());
            }
            InvestmentVerdict::Avoid => {
                parts.push("该标的风险较大，建议暂时回避。".to_string());
            }
        }

        if let Some(ref val) = valuation {
            if val.margin_of_safety > 20.0 {
                parts.push(format!(
                    "当前安全边际约 {:.0}%，价格相对具有吸引力。",
                    val.margin_of_safety
                ));
            }
        }

        if let Some(ref power) = evaluation_power {
            if power.tier == crate::value::EvaluationTier::Primary {
                parts.push("作为一级评估者，公司在产业链中具有主导地位。".to_string());
            }
        }

        parts.join("")
    }

    fn calculate_portfolio_health(
        &self,
        portfolio: &PortfolioPools,
        signal_alerts: &[PositionSignalAlert],
    ) -> f64 {
        let mut score: f64 = 70.0;

        // Red signals are severe
        let red_count = signal_alerts
            .iter()
            .filter(|a| a.signal.level == crate::portfolio::SignalLevel::Red)
            .count();
        score -= red_count as f64 * 15.0;

        // Yellow signals are concerning
        let yellow_count = signal_alerts
            .iter()
            .filter(|a| a.signal.level == crate::portfolio::SignalLevel::Yellow)
            .count();
        score -= yellow_count as f64 * 5.0;

        // Diversification
        let summary = portfolio.summary();
        if summary.total_positions >= 5 {
            score += 10.0;
        }
        if summary.winners > summary.losers {
            score += 10.0;
        }

        // P&L health
        if summary.total_pnl_pct > 10.0 {
            score += 10.0;
        } else if summary.total_pnl_pct < -10.0 {
            score -= 10.0;
        }

        score.clamp(0.0, 100.0)
    }

    fn generate_portfolio_observations(
        &self,
        portfolio: &PortfolioPools,
        signal_alerts: &[PositionSignalAlert],
        allocation: &AllocationRecommendation,
    ) -> Vec<String> {
        let mut observations = Vec::new();
        let summary = portfolio.summary();

        // Concentration
        if summary.core_positions > 0 && summary.core_allocation_pct < 50.0 {
            observations.push("核心仓位占比偏低，考虑增加核心持仓".to_string());
        }

        // Signals
        if signal_alerts.iter().any(|a| a.signal.level == crate::portfolio::SignalLevel::Red) {
            observations.push("存在红灯信号，需立即关注".to_string());
        }

        // Allocation
        if !allocation.trim_candidates.is_empty() {
            observations.push(format!(
                "有 {} 个标的建议减仓",
                allocation.trim_candidates.len()
            ));
        }

        // Performance
        if summary.total_pnl_pct > 20.0 {
            observations.push("组合整体表现优秀".to_string());
        } else if summary.total_pnl_pct < -15.0 {
            observations.push("组合整体表现不佳，需检视持仓逻辑".to_string());
        }

        // Cash
        if summary.cash_allocation_pct > 30.0 {
            observations.push("现金占比较高，可考虑寻找投资机会".to_string());
        }

        observations
    }
}

impl Default for ValueReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::types::FinancialData;

    fn make_test_financial_data() -> FinancialData {
        FinancialData {
            symbol: "TEST".to_string(),
            period_end: Utc::now(),
            revenue: 10_000_000.0,
            gross_profit: 4_000_000.0,
            operating_income: 2_000_000.0,
            net_income: 1_500_000.0,
            interest_expense: 100_000.0,
            total_assets: 20_000_000.0,
            total_equity: 10_000_000.0,
            total_liabilities: 10_000_000.0,
            cash: 2_000_000.0,
            total_debt: 5_000_000.0,
            operating_cash_flow: 2_000_000.0,
            investing_cash_flow: -500_000.0,
            financing_cash_flow: -300_000.0,
            capex: -400_000.0,
            free_cash_flow: 1_600_000.0,
            avg_roe_5y: Some(15.0),
            avg_gross_margin_5y: Some(40.0),
            avg_net_margin_5y: Some(15.0),
        }
    }

    #[test]
    fn test_generate_stock_report() {
        let generator = ValueReportGenerator::new();
        let financial = make_test_financial_data();

        let report = generator.generate_stock_report(
            "TEST",
            "Test Company",
            &financial,
            QualitativeInputs::high_quality(),
            None,
            None,
        );

        assert_eq!(report.symbol, "TEST");
        assert!(report.printing_machine.is_some());
        assert!(report.cash_flow_dna.is_some());
        assert!(!report.highlights.is_empty() || !report.risks.is_empty());
    }

    #[test]
    fn test_investment_verdict_display() {
        assert_eq!(InvestmentVerdict::StrongBuy.to_string(), "强烈买入");
        assert_eq!(InvestmentVerdict::Avoid.to_string(), "回避");
    }

    #[test]
    fn test_report_to_markdown() {
        let generator = ValueReportGenerator::new();
        let financial = make_test_financial_data();

        let report = generator.generate_stock_report(
            "TEST",
            "Test Company",
            &financial,
            QualitativeInputs::high_quality(),
            None,
            None,
        );

        let md = report.to_markdown();
        assert!(md.contains("价值分析报告"));
        assert!(md.contains("TEST"));
    }

    #[test]
    fn test_opportunity_scan_report() {
        let generator = ValueReportGenerator::new();

        // Create test checklists
        let checklists = vec![
            (
                "GOOD".to_string(),
                DipChecklist {
                    decline_driver: crate::portfolio::DeclineDriver::External,
                    moat_status: crate::portfolio::MoatStatus::Intact,
                    balance_sheet_health: crate::portfolio::BalanceSheetHealth::Fortress,
                    cash_runway_adequate: true,
                    debt_manageable: true,
                    insider_activity: crate::portfolio::InsiderActivity::Buying,
                    valuation_attractive: true,
                    margin_of_safety: 30.0,
                    ..Default::default()
                },
            ),
            (
                "BAD".to_string(),
                DipChecklist {
                    decline_driver: crate::portfolio::DeclineDriver::Internal,
                    moat_status: crate::portfolio::MoatStatus::Breached,
                    balance_sheet_health: crate::portfolio::BalanceSheetHealth::Distressed,
                    cash_runway_adequate: false,
                    debt_manageable: false,
                    insider_activity: crate::portfolio::InsiderActivity::Selling,
                    valuation_attractive: false,
                    margin_of_safety: 0.0,
                    red_flags: vec!["Major issue".to_string()],
                    ..Default::default()
                },
            ),
        ];

        let report = generator.generate_opportunity_report(checklists);

        assert_eq!(report.symbols_scanned, 2);
        assert_eq!(report.golden_pits.len(), 1);
        assert!(report.value_traps.len() >= 1);
    }

    #[test]
    fn test_value_report_type_display() {
        assert_eq!(ValueReportType::StockDeepDive.to_string(), "个股深度分析");
        assert_eq!(ValueReportType::PortfolioReview.to_string(), "组合审视报告");
    }
}
