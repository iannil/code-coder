//! Printing Machine Checklist Verifier.
//!
//! The "Printing Machine" (印钞机) checklist is a comprehensive framework for
//! identifying high-quality businesses with sustainable competitive advantages.
//!
//! # Philosophy
//!
//! A true "printing machine" business has these characteristics:
//! - **Evaluation power**: Sets prices rather than takes prices
//! - **Simplicity**: Business model is easy to understand
//! - **Demand stickiness**: Customers keep coming back
//! - **Supply stability**: Not at mercy of commodity prices
//! - **Market leadership**: Dominant position in niche
//!
//! Financially, it should show:
//! - High gross margins (>30%) indicating pricing power
//! - High net margins (>15%) indicating operational efficiency
//! - High FCF conversion (>80%) indicating earnings quality
//! - Low CapEx intensity (<20%) indicating capital efficiency
//! - High ROE (>15%) indicating shareholder returns
//! - Healthy cash flow DNA (Cash Cow or Growth Expansion)

use anyhow::Result;
use chrono::Utc;

use crate::value::types::{CashFlowDNA, FinancialData, PrintingMachineChecklist, RoeDriver};
use crate::value::QualitativeInputs;

use super::CashFlowAnalyzer;

/// Financial verifier for the printing machine checklist.
pub struct FinancialVerifier {
    /// Cash flow analyzer for DNA classification
    cash_flow_analyzer: CashFlowAnalyzer,
    /// Scoring configuration
    config: VerifierConfig,
}

/// Configuration for the financial verifier.
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    // Quantitative thresholds
    pub gross_margin_min: f64,
    pub net_margin_min: f64,
    pub fcf_conversion_min: f64,
    pub capex_ratio_max: f64,
    pub roe_min: f64,
    pub interest_coverage_min: f64,

    // Scoring weights (must sum to 1.0)
    pub qualitative_weight: f64,
    pub quantitative_weight: f64,

    // Pass threshold
    pub pass_threshold: f64,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            gross_margin_min: 30.0,
            net_margin_min: 15.0,
            fcf_conversion_min: 80.0,
            capex_ratio_max: 20.0,
            roe_min: 15.0,
            interest_coverage_min: 5.0,
            qualitative_weight: 0.4,
            quantitative_weight: 0.6,
            pass_threshold: 70.0,
        }
    }
}

impl FinancialVerifier {
    /// Create a new financial verifier with default settings.
    pub fn new() -> Self {
        Self {
            cash_flow_analyzer: CashFlowAnalyzer::new(),
            config: VerifierConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: VerifierConfig) -> Self {
        Self {
            cash_flow_analyzer: CashFlowAnalyzer::new(),
            config,
        }
    }

    /// Analyze a company and produce the printing machine checklist.
    pub fn analyze(
        &self,
        data: &FinancialData,
        qualitative: QualitativeInputs,
    ) -> Result<PrintingMachineChecklist> {
        // Calculate quantitative metrics
        let gross_margin = data.gross_margin_pct();
        let net_margin = data.net_margin_pct();
        let roe = data.avg_roe_5y.unwrap_or_else(|| data.roe_pct());
        let fcf_conversion = data.fcf_conversion_rate();
        let capex_ratio = data.capex_to_revenue_pct();
        let interest_coverage = data.interest_coverage_ratio();

        // Classify cash flow DNA
        let cash_flow_dna = self.cash_flow_analyzer.classify(data);

        // Determine ROE driver
        let roe_driver = self.cash_flow_analyzer.determine_roe_driver(data);

        // Calculate qualitative score
        let qualitative_score = qualitative.score();

        // Calculate quantitative score
        let quantitative_score = self.calculate_quantitative_score(
            gross_margin,
            net_margin,
            roe,
            fcf_conversion,
            capex_ratio,
            interest_coverage,
            &cash_flow_dna,
        );

        // Calculate overall score
        let overall_score = self.config.qualitative_weight * qualitative_score
            + self.config.quantitative_weight * quantitative_score;

        // Generate reasoning
        let reasoning = self.generate_reasoning(
            &qualitative,
            gross_margin,
            net_margin,
            roe,
            fcf_conversion,
            capex_ratio,
            interest_coverage,
            &cash_flow_dna,
            &roe_driver,
            overall_score,
        );

        Ok(PrintingMachineChecklist {
            symbol: data.symbol.clone(),
            has_evaluation_power: qualitative.has_evaluation_power,
            is_simple_and_understandable: qualitative.is_simple_and_understandable,
            has_demand_stickiness: qualitative.has_demand_stickiness,
            has_supply_stability: qualitative.has_supply_stability,
            is_market_leader: qualitative.is_market_leader,
            gross_margin,
            net_margin,
            fcf_conversion_rate: fcf_conversion,
            capex_to_revenue: capex_ratio,
            roe,
            roe_driver,
            interest_coverage,
            cash_flow_dna,
            qualitative_score,
            quantitative_score,
            overall_score,
            reasoning,
            analyzed_at: Utc::now(),
        })
    }

    /// Calculate quantitative score (0-100).
    fn calculate_quantitative_score(
        &self,
        gross_margin: f64,
        net_margin: f64,
        roe: f64,
        fcf_conversion: f64,
        capex_ratio: f64,
        interest_coverage: f64,
        cash_flow_dna: &CashFlowDNA,
    ) -> f64 {
        let mut score: f64 = 0.0;
        let mut max_score: f64 = 0.0;

        // Gross margin (0-15 points)
        max_score += 15.0;
        if gross_margin >= self.config.gross_margin_min * 1.5 {
            score += 15.0;
        } else if gross_margin >= self.config.gross_margin_min {
            score += 12.0;
        } else if gross_margin >= self.config.gross_margin_min * 0.8 {
            score += 8.0;
        } else if gross_margin >= self.config.gross_margin_min * 0.5 {
            score += 4.0;
        }

        // Net margin (0-15 points)
        max_score += 15.0;
        if net_margin >= self.config.net_margin_min * 1.5 {
            score += 15.0;
        } else if net_margin >= self.config.net_margin_min {
            score += 12.0;
        } else if net_margin >= self.config.net_margin_min * 0.8 {
            score += 8.0;
        } else if net_margin >= self.config.net_margin_min * 0.5 {
            score += 4.0;
        }

        // ROE (0-20 points)
        max_score += 20.0;
        if roe >= self.config.roe_min * 1.5 {
            score += 20.0;
        } else if roe >= self.config.roe_min {
            score += 16.0;
        } else if roe >= self.config.roe_min * 0.8 {
            score += 10.0;
        } else if roe >= self.config.roe_min * 0.5 {
            score += 5.0;
        }

        // FCF conversion (0-15 points)
        max_score += 15.0;
        if fcf_conversion >= self.config.fcf_conversion_min {
            score += 15.0;
        } else if fcf_conversion >= self.config.fcf_conversion_min * 0.75 {
            score += 10.0;
        } else if fcf_conversion >= self.config.fcf_conversion_min * 0.5 {
            score += 5.0;
        } else if fcf_conversion < 0.0 {
            // Negative FCF conversion is a red flag
            score -= 5.0;
        }

        // CapEx ratio (0-15 points, lower is better)
        max_score += 15.0;
        if capex_ratio <= self.config.capex_ratio_max * 0.5 {
            score += 15.0;
        } else if capex_ratio <= self.config.capex_ratio_max {
            score += 12.0;
        } else if capex_ratio <= self.config.capex_ratio_max * 1.5 {
            score += 6.0;
        }

        // Interest coverage (0-10 points)
        max_score += 10.0;
        if interest_coverage >= self.config.interest_coverage_min * 2.0 {
            score += 10.0;
        } else if interest_coverage >= self.config.interest_coverage_min {
            score += 8.0;
        } else if interest_coverage >= self.config.interest_coverage_min * 0.5 {
            score += 4.0;
        }

        // Cash flow DNA (0-10 points)
        max_score += 10.0;
        match cash_flow_dna {
            CashFlowDNA::CashCow => score += 10.0,
            CashFlowDNA::SteadyState => score += 9.0,
            CashFlowDNA::GrowthExpansion => score += 7.0,
            CashFlowDNA::Harvesting => score += 5.0,
            CashFlowDNA::StartupBloodTransfusion => score += 3.0,
            CashFlowDNA::DeclineLiquidation => score += 1.0,
            CashFlowDNA::PonziScheme => score -= 10.0,
            CashFlowDNA::Unknown => score += 0.0,
        }

        // Normalize to 0-100
        ((score / max_score) * 100.0).clamp(0.0, 100.0)
    }

    /// Generate human-readable reasoning.
    #[allow(clippy::too_many_arguments)]
    fn generate_reasoning(
        &self,
        qualitative: &QualitativeInputs,
        gross_margin: f64,
        net_margin: f64,
        roe: f64,
        fcf_conversion: f64,
        capex_ratio: f64,
        interest_coverage: f64,
        cash_flow_dna: &CashFlowDNA,
        roe_driver: &RoeDriver,
        overall_score: f64,
    ) -> String {
        let mut parts = Vec::new();

        // Overall assessment
        let grade = match overall_score as u32 {
            90..=100 => "优秀",
            80..=89 => "良好",
            70..=79 => "合格",
            60..=69 => "一般",
            _ => "较差",
        };
        parts.push(format!(
            "综合评分 {:.1}，等级：{}。",
            overall_score, grade
        ));

        // Qualitative highlights
        let mut qualitative_items = Vec::new();
        if qualitative.has_evaluation_power {
            qualitative_items.push("具有定价权");
        }
        if qualitative.is_market_leader {
            qualitative_items.push("市场龙头");
        }
        if qualitative.has_demand_stickiness {
            qualitative_items.push("需求粘性强");
        }
        if !qualitative_items.is_empty() {
            parts.push(format!("定性优势：{}。", qualitative_items.join("、")));
        }

        // Financial highlights
        let mut strengths = Vec::new();
        let mut weaknesses = Vec::new();

        if gross_margin >= self.config.gross_margin_min {
            strengths.push(format!("毛利率 {:.1}%", gross_margin));
        } else {
            weaknesses.push(format!("毛利率偏低 {:.1}%", gross_margin));
        }

        if net_margin >= self.config.net_margin_min {
            strengths.push(format!("净利率 {:.1}%", net_margin));
        } else {
            weaknesses.push(format!("净利率不足 {:.1}%", net_margin));
        }

        if roe >= self.config.roe_min {
            strengths.push(format!("ROE {:.1}%（{}）", roe, roe_driver));
        } else {
            weaknesses.push(format!("ROE较低 {:.1}%", roe));
        }

        if fcf_conversion >= self.config.fcf_conversion_min {
            strengths.push(format!("现金流转换率 {:.0}%", fcf_conversion));
        } else if fcf_conversion < 50.0 {
            weaknesses.push(format!("现金流转换率偏低 {:.0}%", fcf_conversion));
        }

        if capex_ratio <= self.config.capex_ratio_max {
            strengths.push("资本支出可控".to_string());
        } else {
            weaknesses.push(format!("资本支出较高 {:.1}%", capex_ratio));
        }

        if interest_coverage >= self.config.interest_coverage_min {
            strengths.push(format!("利息保障 {:.1}倍", interest_coverage));
        } else if interest_coverage < 3.0 {
            weaknesses.push(format!("利息保障不足 {:.1}倍", interest_coverage));
        }

        // Cash flow DNA
        parts.push(format!("现金流DNA：{}。", cash_flow_dna));

        if !strengths.is_empty() {
            parts.push(format!("财务优势：{}。", strengths.join("、")));
        }
        if !weaknesses.is_empty() {
            parts.push(format!("需关注：{}。", weaknesses.join("、")));
        }

        parts.join("\n")
    }
}

impl Default for FinancialVerifier {
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

    fn make_excellent_company() -> FinancialData {
        FinancialData {
            symbol: "EXCELLENT".to_string(),
            period_end: Utc::now(),
            revenue: 10000.0,
            gross_profit: 5000.0,    // 50% gross margin
            operating_income: 2500.0, // 25% operating margin
            net_income: 2000.0,      // 20% net margin
            interest_expense: 100.0,
            total_assets: 10000.0,
            total_equity: 8000.0,
            total_liabilities: 2000.0,
            cash: 2000.0,
            total_debt: 1000.0,
            operating_cash_flow: 2500.0, // OCF > Net Income
            investing_cash_flow: -500.0,
            financing_cash_flow: -1500.0, // Returning cash
            capex: -400.0,               // 4% CapEx ratio
            free_cash_flow: 2100.0,      // 105% FCF conversion
            avg_roe_5y: Some(25.0),
            avg_gross_margin_5y: Some(48.0),
            avg_net_margin_5y: Some(19.0),
        }
    }

    fn make_poor_company() -> FinancialData {
        FinancialData {
            symbol: "POOR".to_string(),
            period_end: Utc::now(),
            revenue: 10000.0,
            gross_profit: 1500.0,     // 15% gross margin
            operating_income: 300.0,  // 3% operating margin
            net_income: 100.0,        // 1% net margin
            interest_expense: 200.0,
            total_assets: 20000.0,
            total_equity: 5000.0,
            total_liabilities: 15000.0,
            cash: 500.0,
            total_debt: 10000.0,
            operating_cash_flow: -200.0,  // Negative OCF
            investing_cash_flow: -1000.0, // Heavy investment
            financing_cash_flow: 1500.0,  // Raising money
            capex: -800.0,                // 8% CapEx ratio
            free_cash_flow: -1000.0,      // Negative FCF
            avg_roe_5y: Some(2.0),
            avg_gross_margin_5y: Some(16.0),
            avg_net_margin_5y: Some(2.0),
        }
    }

    #[test]
    fn test_excellent_company_passes() {
        let verifier = FinancialVerifier::new();
        let data = make_excellent_company();
        let qualitative = QualitativeInputs::high_quality();

        let result = verifier.analyze(&data, qualitative).unwrap();

        assert!(result.is_printing_machine(), "Score: {}", result.overall_score);
        assert!(result.overall_score >= 85.0);
        assert_eq!(result.cash_flow_dna, CashFlowDNA::CashCow);
    }

    #[test]
    fn test_poor_company_fails() {
        let verifier = FinancialVerifier::new();
        let data = make_poor_company();
        let qualitative = QualitativeInputs::unknown();

        let result = verifier.analyze(&data, qualitative).unwrap();

        assert!(!result.is_printing_machine(), "Score: {}", result.overall_score);
        assert!(result.overall_score < 50.0);
    }

    #[test]
    fn test_quantitative_score_boundaries() {
        let verifier = FinancialVerifier::new();

        // Test with borderline values
        let data = FinancialData {
            symbol: "BORDERLINE".to_string(),
            period_end: Utc::now(),
            revenue: 10000.0,
            gross_profit: 3000.0,   // 30% exactly at threshold
            operating_income: 1500.0,
            net_income: 1500.0,     // 15% exactly at threshold
            interest_expense: 300.0,
            total_assets: 10000.0,
            total_equity: 10000.0,
            total_liabilities: 0.0,
            cash: 1000.0,
            total_debt: 0.0,
            operating_cash_flow: 1600.0,
            investing_cash_flow: -400.0,
            financing_cash_flow: -1000.0,
            capex: -200.0,              // 2% CapEx - excellent
            free_cash_flow: 1400.0,     // 93% FCF conversion
            avg_roe_5y: Some(15.0),     // 15% exactly at threshold
            avg_gross_margin_5y: None,
            avg_net_margin_5y: None,
        };

        let qualitative = QualitativeInputs::default();
        let result = verifier.analyze(&data, qualitative).unwrap();

        // Should be around 70-90 with mostly borderline values (hitting thresholds)
        assert!(
            result.quantitative_score >= 55.0 && result.quantitative_score <= 95.0,
            "Score {} outside expected range",
            result.quantitative_score
        );
    }

    #[test]
    fn test_reasoning_generation() {
        let verifier = FinancialVerifier::new();
        let data = make_excellent_company();
        let qualitative = QualitativeInputs::high_quality();

        let result = verifier.analyze(&data, qualitative).unwrap();

        assert!(result.reasoning.contains("优秀"));
        assert!(result.reasoning.contains("毛利率"));
        assert!(result.reasoning.contains("现金流DNA"));
    }
}
