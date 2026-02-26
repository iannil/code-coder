//! Cash Flow DNA Analyzer.
//!
//! Classifies companies based on their cash flow patterns using the "DNA" methodology.
//! The combination of operating, investing, and financing cash flow signs reveals
//! a company's life cycle stage and business model quality.
//!
//! # Cash Flow DNA Patterns
//!
//! | Pattern | OCF | ICF | FCF | Description |
//! |---------|-----|-----|-----|-------------|
//! | Cash Cow | + | - | - | Mature, generating excess cash |
//! | Growth Expansion | + | - | + | Growing, reinvesting + raising capital |
//! | Steady State | + | - | ≈0 | Balanced, self-sustaining |
//! | Harvesting | + | + | - | Selling assets, returning cash |
//! | Startup Blood Transfusion | - | - | + | Pre-profit, burning cash |
//! | Decline Liquidation | - | + | - | Declining, selling to survive |
//! | Ponzi Scheme | - | - | + | Dangerous: always raising, never generating |

use crate::value::types::{CashFlowDNA, FinancialData, RoeDriver};

/// Cash flow analyzer for DNA classification.
pub struct CashFlowAnalyzer {
    /// Threshold for considering a cash flow as "approximately zero"
    zero_threshold_pct: f64,
    /// Number of consecutive periods needed to confirm Ponzi pattern
    ponzi_confirmation_periods: usize,
}

impl CashFlowAnalyzer {
    /// Create a new cash flow analyzer with default settings.
    pub fn new() -> Self {
        Self {
            zero_threshold_pct: 5.0, // Within 5% of revenue is considered "near zero"
            ponzi_confirmation_periods: 3,
        }
    }

    /// Create with custom thresholds.
    pub fn with_thresholds(zero_threshold_pct: f64, ponzi_confirmation_periods: usize) -> Self {
        Self {
            zero_threshold_pct,
            ponzi_confirmation_periods,
        }
    }

    /// Classify cash flow DNA from a single period's financial data.
    pub fn classify(&self, data: &FinancialData) -> CashFlowDNA {
        let ocf = data.operating_cash_flow;
        let icf = data.investing_cash_flow;
        let fcf = data.financing_cash_flow;

        // Helper to determine sign
        let is_positive = |v: f64| v > 0.0;
        let is_negative = |v: f64| v < 0.0;
        let is_near_zero = |v: f64| {
            if data.revenue > 0.0 {
                (v.abs() / data.revenue) * 100.0 < self.zero_threshold_pct
            } else {
                v.abs() < 1000.0 // Fallback for zero revenue
            }
        };

        // Pattern matching based on cash flow signs
        match (is_positive(ocf), is_negative(icf), is_positive(fcf)) {
            // OCF+, ICF-, FCF- = Cash Cow
            (true, true, false) if is_negative(fcf) => CashFlowDNA::CashCow,

            // OCF+, ICF-, FCF≈0 = Steady State (check before GrowthExpansion)
            (true, true, _) if is_near_zero(fcf) => CashFlowDNA::SteadyState,

            // OCF+, ICF-, FCF+ = Growth Expansion
            (true, true, true) => CashFlowDNA::GrowthExpansion,

            // OCF+, ICF+, FCF- = Harvesting
            (true, false, false) if is_positive(icf) && is_negative(fcf) => CashFlowDNA::Harvesting,

            // OCF-, ICF-, FCF+ = Startup Blood Transfusion or Ponzi
            (false, true, true) => {
                // For single period, we can't confirm Ponzi - that requires multi-period analysis
                CashFlowDNA::StartupBloodTransfusion
            }

            // OCF-, ICF+, FCF- = Decline Liquidation
            (false, false, false) if is_positive(icf) => CashFlowDNA::DeclineLiquidation,

            // Other patterns
            _ => CashFlowDNA::Unknown,
        }
    }

    /// Classify with multi-period data to detect Ponzi patterns.
    ///
    /// The Ponzi pattern requires persistent negative OCF combined with
    /// positive FCF over multiple periods.
    pub fn classify_with_history(&self, data: &[FinancialData]) -> CashFlowDNA {
        if data.is_empty() {
            return CashFlowDNA::Unknown;
        }

        // Check for Ponzi pattern: persistent OCF-, FCF+ over multiple periods
        let ponzi_periods = data
            .iter()
            .filter(|d| d.operating_cash_flow < 0.0 && d.financing_cash_flow > 0.0)
            .count();

        if ponzi_periods >= self.ponzi_confirmation_periods {
            return CashFlowDNA::PonziScheme;
        }

        // Use the most recent period for classification
        self.classify(data.last().unwrap())
    }

    /// Determine ROE driver using DuPont decomposition.
    ///
    /// ROE = Net Profit Margin × Asset Turnover × Equity Multiplier
    ///     = (Net Income / Revenue) × (Revenue / Assets) × (Assets / Equity)
    pub fn determine_roe_driver(&self, data: &FinancialData) -> RoeDriver {
        // Calculate DuPont components
        let net_margin = if data.revenue > 0.0 {
            data.net_income / data.revenue
        } else {
            0.0
        };

        let asset_turnover = if data.total_assets > 0.0 {
            data.revenue / data.total_assets
        } else {
            0.0
        };

        let equity_multiplier = if data.total_equity > 0.0 {
            data.total_assets / data.total_equity
        } else {
            1.0
        };

        // Thresholds for "high" in each dimension
        const HIGH_MARGIN_THRESHOLD: f64 = 0.15; // 15%
        const HIGH_TURNOVER_THRESHOLD: f64 = 1.0; // 1x
        const HIGH_LEVERAGE_THRESHOLD: f64 = 3.0; // 3x

        let is_high_margin = net_margin >= HIGH_MARGIN_THRESHOLD;
        let is_high_turnover = asset_turnover >= HIGH_TURNOVER_THRESHOLD;
        let is_high_leverage = equity_multiplier >= HIGH_LEVERAGE_THRESHOLD;

        // Determine dominant driver
        match (is_high_margin, is_high_turnover, is_high_leverage) {
            // Single dominant driver
            (true, false, false) => RoeDriver::HighMargin,
            (false, true, false) => RoeDriver::HighTurnover,
            (false, false, true) => RoeDriver::HighLeverage,

            // Margin is the most valuable driver, prioritize it
            (true, true, _) => RoeDriver::HighMargin,
            (true, _, true) => RoeDriver::HighMargin,

            // Turnover beats leverage
            (false, true, true) => RoeDriver::HighTurnover,

            // All low or all high = balanced
            _ => RoeDriver::Balanced,
        }
    }

    /// Calculate the quality score for cash flow (0-100).
    ///
    /// Higher scores indicate healthier cash flow patterns.
    pub fn quality_score(&self, data: &FinancialData) -> f64 {
        let mut score: f64 = 50.0; // Base score

        // OCF quality (0-30 points)
        if data.operating_cash_flow > 0.0 {
            score += 15.0;
            // Bonus for OCF > Net Income (high quality earnings)
            if data.operating_cash_flow > data.net_income {
                score += 15.0;
            } else if data.operating_cash_flow > data.net_income * 0.8 {
                score += 10.0;
            }
        } else {
            score -= 20.0;
        }

        // FCF quality (0-30 points)
        if data.free_cash_flow > 0.0 {
            score += 15.0;
            // Bonus for high FCF conversion
            let fcf_conversion = data.fcf_conversion_rate();
            if fcf_conversion >= 80.0 {
                score += 15.0;
            } else if fcf_conversion >= 60.0 {
                score += 10.0;
            } else if fcf_conversion >= 40.0 {
                score += 5.0;
            }
        } else if data.free_cash_flow < 0.0 {
            score -= 10.0;
        }

        // CapEx discipline (0-20 points)
        let capex_ratio = data.capex_to_revenue_pct();
        if capex_ratio <= 10.0 {
            score += 20.0;
        } else if capex_ratio <= 20.0 {
            score += 15.0;
        } else if capex_ratio <= 30.0 {
            score += 10.0;
        } else if capex_ratio > 50.0 {
            score -= 10.0;
        }

        // Clamp to 0-100
        score.clamp(0.0, 100.0)
    }
}

impl Default for CashFlowAnalyzer {
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
    use chrono::Utc;

    fn make_financial_data(ocf: f64, icf: f64, fcf: f64, revenue: f64) -> FinancialData {
        FinancialData {
            symbol: "TEST".to_string(),
            period_end: Utc::now(),
            revenue,
            gross_profit: revenue * 0.4,
            operating_income: revenue * 0.2,
            net_income: revenue * 0.15,
            interest_expense: revenue * 0.02,
            total_assets: revenue * 5.0,
            total_equity: revenue * 2.0,
            total_liabilities: revenue * 3.0,
            cash: revenue * 0.5,
            total_debt: revenue * 1.0,
            operating_cash_flow: ocf,
            investing_cash_flow: icf,
            financing_cash_flow: fcf,
            capex: icf * 0.8,
            free_cash_flow: ocf + (icf * 0.8),
            avg_roe_5y: None,
            avg_gross_margin_5y: None,
            avg_net_margin_5y: None,
        }
    }

    #[test]
    fn test_classify_cash_cow() {
        let analyzer = CashFlowAnalyzer::new();
        let data = make_financial_data(100.0, -50.0, -30.0, 1000.0);
        assert_eq!(analyzer.classify(&data), CashFlowDNA::CashCow);
    }

    #[test]
    fn test_classify_growth_expansion() {
        let analyzer = CashFlowAnalyzer::new();
        let data = make_financial_data(100.0, -150.0, 80.0, 1000.0);
        assert_eq!(analyzer.classify(&data), CashFlowDNA::GrowthExpansion);
    }

    #[test]
    fn test_classify_steady_state() {
        let analyzer = CashFlowAnalyzer::new();
        // FCF near zero (within 5% of revenue) - use very small FCF
        let data = make_financial_data(100.0, -100.0, 2.0, 1000.0);
        assert_eq!(analyzer.classify(&data), CashFlowDNA::SteadyState);
    }

    #[test]
    fn test_classify_startup_blood_transfusion() {
        let analyzer = CashFlowAnalyzer::new();
        let data = make_financial_data(-50.0, -100.0, 180.0, 1000.0);
        assert_eq!(analyzer.classify(&data), CashFlowDNA::StartupBloodTransfusion);
    }

    #[test]
    fn test_classify_decline_liquidation() {
        let analyzer = CashFlowAnalyzer::new();
        let data = make_financial_data(-50.0, 100.0, -30.0, 1000.0);
        assert_eq!(analyzer.classify(&data), CashFlowDNA::DeclineLiquidation);
    }

    #[test]
    fn test_classify_ponzi_with_history() {
        let analyzer = CashFlowAnalyzer::new();
        let history: Vec<FinancialData> = (0..4)
            .map(|_| make_financial_data(-50.0, -100.0, 180.0, 1000.0))
            .collect();
        assert_eq!(
            analyzer.classify_with_history(&history),
            CashFlowDNA::PonziScheme
        );
    }

    #[test]
    fn test_roe_driver_high_margin() {
        let analyzer = CashFlowAnalyzer::new();
        let mut data = make_financial_data(100.0, -50.0, -30.0, 1000.0);
        data.net_income = 200.0; // 20% net margin
        data.total_assets = 5000.0; // 0.2x turnover
        data.total_equity = 3000.0; // 1.67x leverage
        assert_eq!(analyzer.determine_roe_driver(&data), RoeDriver::HighMargin);
    }

    #[test]
    fn test_roe_driver_high_turnover() {
        let analyzer = CashFlowAnalyzer::new();
        let mut data = make_financial_data(100.0, -50.0, -30.0, 1000.0);
        data.net_income = 50.0; // 5% net margin
        data.total_assets = 800.0; // 1.25x turnover
        data.total_equity = 500.0; // 1.6x leverage
        assert_eq!(analyzer.determine_roe_driver(&data), RoeDriver::HighTurnover);
    }

    #[test]
    fn test_quality_score_excellent() {
        let analyzer = CashFlowAnalyzer::new();
        let mut data = make_financial_data(200.0, -50.0, -100.0, 1000.0);
        data.net_income = 150.0;
        data.free_cash_flow = 150.0;
        data.capex = -50.0;
        let score = analyzer.quality_score(&data);
        assert!(score >= 80.0, "Score {} should be >= 80", score);
    }

    #[test]
    fn test_quality_score_poor() {
        let analyzer = CashFlowAnalyzer::new();
        let mut data = make_financial_data(-100.0, -500.0, 400.0, 1000.0);
        data.net_income = 50.0;
        data.free_cash_flow = -100.0;
        let score = analyzer.quality_score(&data);
        assert!(score <= 40.0, "Score {} should be <= 40", score);
    }
}
