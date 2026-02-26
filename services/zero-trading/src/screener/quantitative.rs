//! Quantitative filtering module for the screener.
//!
//! Implements a multi-stage funnel filter:
//! 1. Basic filter: exclude ST, new stocks, suspended stocks
//! 2. Quality filter: ROE, gross margin, cash flow DNA
//! 3. Valuation filter: PE/PB/dividend yield thresholds

use crate::data::{StockInfo, FinancialStatementData};
use super::config::FilterConfig;
use serde::{Deserialize, Serialize};

// ============================================================================
// Filter Stage
// ============================================================================

/// Filter stage identifier for tracking where stocks are eliminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterStage {
    /// Input stage (all stocks)
    Input,
    /// Basic filter (exclude ST, new stocks, suspended)
    Basic,
    /// Quality filter (ROE, gross margin, cash flow DNA)
    Quality,
    /// Valuation filter (PE/PB/dividend yield)
    Valuation,
    /// Final selection
    Final,
}

impl std::fmt::Display for FilterStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => write!(f, "输入"),
            Self::Basic => write!(f, "基础筛选"),
            Self::Quality => write!(f, "质量筛选"),
            Self::Valuation => write!(f, "估值筛选"),
            Self::Final => write!(f, "最终结果"),
        }
    }
}

// ============================================================================
// Filter Result
// ============================================================================

/// Result of a filtering stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    /// Stage name
    pub stage: FilterStage,
    /// Number of stocks that passed this stage
    pub passed: usize,
    /// Number of stocks eliminated at this stage
    pub eliminated: usize,
    /// Elimination rate (%)
    pub elimination_rate: f64,
}

impl FilterResult {
    pub fn new(stage: FilterStage, input_count: usize, passed_count: usize) -> Self {
        let eliminated = input_count.saturating_sub(passed_count);
        let elimination_rate = if input_count > 0 {
            (eliminated as f64 / input_count as f64) * 100.0
        } else {
            0.0
        };

        Self {
            stage,
            passed: passed_count,
            eliminated,
            elimination_rate,
        }
    }
}

// ============================================================================
// Quantitative Filter
// ============================================================================

/// Quantitative filter for stock screening.
///
/// Implements a multi-stage funnel:
/// 1. Basic filter (market structure)
/// 2. Quality filter (fundamentals)
/// 3. Valuation filter (price reasonableness)
pub struct QuantitativeFilter {
    config: FilterConfig,
}

impl QuantitativeFilter {
    /// Create a new quantitative filter with the given configuration.
    pub fn new(config: FilterConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(FilterConfig::default())
    }

    // ========================================================================
    // Stage 1: Basic Filter
    // ========================================================================

    /// Apply basic filters to stock list.
    ///
    /// Filters:
    /// - Target symbols (if configured, only scan specific stocks)
    /// - Industry filters (include/exclude)
    /// - Market cap range
    /// - Exclude ST stocks (if configured)
    /// - Exclude Beijing Exchange stocks (if configured)
    /// - Exclude suspended stocks
    /// - Exclude stocks listed less than min_listing_days
    pub fn filter_basic(&self, stocks: &[StockInfo]) -> (Vec<StockInfo>, FilterResult) {
        let input_count = stocks.len();

        let passed: Vec<StockInfo> = stocks
            .iter()
            .filter(|s| self.passes_basic_filter(s))
            .cloned()
            .collect();

        let result = FilterResult::new(FilterStage::Basic, input_count, passed.len());
        (passed, result)
    }

    fn passes_basic_filter(&self, stock: &StockInfo) -> bool {
        // Check target symbols (if configured, only these symbols are allowed)
        if !self.config.target_symbols.is_empty() {
            let symbol = format!("{}.{}", stock.code, stock.exchange);
            if !self.config.target_symbols.contains(&symbol)
                && !self.config.target_symbols.contains(&stock.code) {
                return false;
            }
        }

        // Check industry include filter
        if !self.config.include_industries.is_empty() {
            if let Some(ref industry) = stock.industry {
                let passes = self.config.include_industries.iter()
                    .any(|inc| industry.contains(inc));
                if !passes {
                    return false;
                }
            } else {
                // No industry data and include filter is set, exclude
                return false;
            }
        }

        // Check industry exclude filter
        if !self.config.exclude_industries.is_empty() {
            if let Some(ref industry) = stock.industry {
                let excluded = self.config.exclude_industries.iter()
                    .any(|exc| industry.contains(exc));
                if excluded {
                    return false;
                }
            }
        }

        // Check market cap range
        if let Some(min_mc) = self.config.min_market_cap {
            if let Some(mc) = stock.market_cap {
                if mc < min_mc {
                    return false;
                }
            } else {
                // No market cap data, exclude when filter is set
                return false;
            }
        }

        if let Some(max_mc) = self.config.max_market_cap {
            if let Some(mc) = stock.market_cap {
                if mc > max_mc {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check ST exclusion
        if self.config.exclude_st && stock.is_st {
            return false;
        }

        // Check BJ exclusion
        if self.config.exclude_bj && stock.exchange == "BJ" {
            return false;
        }

        // Check suspended
        if stock.is_suspended {
            return false;
        }

        // Check listing days
        if let Some(days) = stock.listing_days() {
            if days < self.config.min_listing_days as i64 {
                return false;
            }
        } else {
            // No listing date means we can't verify, exclude
            return false;
        }

        true
    }

    // ========================================================================
    // Stage 2: Quality Filter
    // ========================================================================

    /// Apply quality filters to financial data.
    ///
    /// Filters:
    /// - Minimum ROE
    /// - Minimum gross margin
    /// - Maximum debt ratio
    /// - Healthy cash flow DNA (optional)
    pub fn filter_quality(
        &self,
        financials: &[FinancialStatementData],
    ) -> (Vec<FinancialStatementData>, FilterResult) {
        let input_count = financials.len();

        let passed: Vec<FinancialStatementData> = financials
            .iter()
            .filter(|f| self.passes_quality_filter(f))
            .cloned()
            .collect();

        let result = FilterResult::new(FilterStage::Quality, input_count, passed.len());
        (passed, result)
    }

    fn passes_quality_filter(&self, data: &FinancialStatementData) -> bool {
        // Check ROE
        if let Some(roe) = data.roe {
            if roe < self.config.min_roe_3y {
                return false;
            }
        } else {
            return false; // No ROE data, skip
        }

        // Check gross margin
        if let Some(gm) = data.gross_margin {
            if gm < self.config.min_gross_margin {
                return false;
            }
        } else {
            return false; // No gross margin data, skip
        }

        // Check debt ratio
        if let Some(de) = data.debt_to_equity {
            if de > self.config.max_debt_ratio {
                return false;
            }
        }
        // Missing debt ratio is OK - we allow it

        // Check cash flow DNA
        if self.config.healthy_cash_flow_dna {
            if !self.has_healthy_cash_flow(data) {
                return false;
            }
        }

        true
    }

    fn has_healthy_cash_flow(&self, data: &FinancialStatementData) -> bool {
        match (
            data.operating_cash_flow,
            data.investing_cash_flow,
            data.financing_cash_flow,
        ) {
            (Some(ocf), Some(icf), Some(_fcf)) => {
                // Healthy patterns:
                // 1. Cash cow: OCF+, ICF-, FCF- (mature business)
                // 2. Growth: OCF+, ICF-, FCF+ (expanding)
                // 3. Steady: OCF+, ICF-, FCF~0
                // All require positive OCF and negative ICF (investing)
                ocf > 0.0 && icf < 0.0
            }
            _ => false, // Missing data is not healthy
        }
    }

    // ========================================================================
    // Stage 3: Valuation Filter
    // ========================================================================

    /// Apply valuation filters to financial data.
    ///
    /// Filters:
    /// - Maximum PE (if configured)
    /// - Maximum PB (if configured)
    /// - Minimum dividend yield
    pub fn filter_valuation(
        &self,
        financials: &[FinancialStatementData],
    ) -> (Vec<FinancialStatementData>, FilterResult) {
        let input_count = financials.len();

        let passed: Vec<FinancialStatementData> = financials
            .iter()
            .filter(|f| self.passes_valuation_filter(f))
            .cloned()
            .collect();

        let result = FilterResult::new(FilterStage::Valuation, input_count, passed.len());
        (passed, result)
    }

    fn passes_valuation_filter(&self, data: &FinancialStatementData) -> bool {
        let val_config = &self.config.valuation;

        // Check PE
        if let Some(max_pe) = val_config.max_pe_absolute {
            if let Some(pe) = data.pe_ttm {
                if pe <= 0.0 || pe > max_pe {
                    return false; // Negative PE or too high
                }
            }
            // Missing PE is allowed
        }

        // Check PB
        if let Some(max_pb) = val_config.max_pb {
            if let Some(pb) = data.pb {
                if pb <= 0.0 || pb > max_pb {
                    return false; // Negative PB or too high
                }
            }
            // Missing PB is allowed
        }

        // Check dividend yield
        if val_config.min_dividend_yield > 0.0 {
            if let Some(dy) = data.dividend_yield {
                if dy < val_config.min_dividend_yield {
                    return false;
                }
            } else {
                return false; // No dividend data, skip
            }
        }

        true
    }

    // ========================================================================
    // Combined Filter
    // ========================================================================

    /// Run all quality and valuation filters on financial data.
    ///
    /// This combines quality and valuation filters in a single pass
    /// for efficiency when we don't need separate stage results.
    pub fn filter_all(
        &self,
        financials: &[FinancialStatementData],
    ) -> Vec<FinancialStatementData> {
        financials
            .iter()
            .filter(|f| self.passes_quality_filter(f) && self.passes_valuation_filter(f))
            .cloned()
            .collect()
    }

    // ========================================================================
    // Scoring
    // ========================================================================

    /// Score a stock based on quality and valuation metrics.
    ///
    /// Returns a score from 0-100 where higher is better.
    pub fn score_stock(&self, data: &FinancialStatementData) -> f64 {
        let mut score = 0.0;
        let mut weight_sum = 0.0;

        // ROE score (weight: 25)
        if let Some(roe) = data.roe {
            let roe_score = (roe / 30.0).min(1.0) * 100.0; // 30% ROE = max score
            score += roe_score * 25.0;
            weight_sum += 25.0;
        }

        // Gross margin score (weight: 15)
        if let Some(gm) = data.gross_margin {
            let gm_score = (gm / 50.0).min(1.0) * 100.0; // 50% margin = max score
            score += gm_score * 15.0;
            weight_sum += 15.0;
        }

        // Debt ratio score (weight: 15) - lower is better
        if let Some(de) = data.debt_to_equity {
            let de_score = ((100.0 - de) / 100.0).max(0.0) * 100.0;
            score += de_score * 15.0;
            weight_sum += 15.0;
        }

        // Cash flow score (weight: 20)
        if self.has_healthy_cash_flow(data) {
            score += 100.0 * 20.0;
            weight_sum += 20.0;
        }

        // PE score (weight: 15) - lower is better
        if let Some(pe) = data.pe_ttm {
            if pe > 0.0 && pe < 50.0 {
                let pe_score = ((50.0 - pe) / 50.0) * 100.0;
                score += pe_score * 15.0;
                weight_sum += 15.0;
            }
        }

        // Dividend yield score (weight: 10)
        if let Some(dy) = data.dividend_yield {
            let dy_score = (dy / 5.0).min(1.0) * 100.0; // 5% yield = max score
            score += dy_score * 10.0;
            weight_sum += 10.0;
        }

        if weight_sum > 0.0 {
            score / weight_sum
        } else {
            0.0
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn create_test_stock(code: &str, is_st: bool, exchange: &str) -> StockInfo {
        StockInfo {
            code: code.to_string(),
            name: format!("Test {}", code),
            exchange: exchange.to_string(),
            industry: Some("Technology".to_string()),
            list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            is_suspended: false,
            is_st,
            market_cap: Some(100.0),
        }
    }

    fn create_test_financial(symbol: &str, roe: f64, gross_margin: f64) -> FinancialStatementData {
        FinancialStatementData {
            symbol: symbol.to_string(),
            period_end: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            report_type: "annual".to_string(),
            revenue: Some(1000.0),
            gross_profit: Some(300.0),
            operating_income: Some(150.0),
            net_income: Some(100.0),
            interest_expense: Some(10.0),
            total_assets: Some(5000.0),
            total_equity: Some(2000.0),
            total_liabilities: Some(3000.0),
            cash: Some(500.0),
            total_debt: Some(1000.0),
            shares_outstanding: Some(100.0),
            operating_cash_flow: Some(120.0),
            investing_cash_flow: Some(-80.0),
            financing_cash_flow: Some(-40.0),
            capex: Some(-50.0),
            roe: Some(roe),
            roa: Some(5.0),
            gross_margin: Some(gross_margin),
            net_margin: Some(10.0),
            debt_to_equity: Some(50.0),
            current_ratio: Some(1.5),
            pe_ttm: Some(15.0),
            pb: Some(2.0),
            dividend_yield: Some(3.0),
        }
    }

    #[test]
    fn test_basic_filter_excludes_st() {
        let filter = QuantitativeFilter::with_defaults();
        let stocks = vec![
            create_test_stock("000001", false, "SZ"),
            create_test_stock("000002", true, "SZ"), // ST
        ];

        let (passed, result) = filter.filter_basic(&stocks);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].code, "000001");
        assert_eq!(result.eliminated, 1);
    }

    #[test]
    fn test_basic_filter_excludes_bj() {
        let filter = QuantitativeFilter::with_defaults();
        let stocks = vec![
            create_test_stock("000001", false, "SZ"),
            create_test_stock("430001", false, "BJ"), // Beijing Exchange
        ];

        let (passed, result) = filter.filter_basic(&stocks);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].code, "000001");
        assert_eq!(result.stage, FilterStage::Basic);
    }

    #[test]
    fn test_quality_filter() {
        let filter = QuantitativeFilter::with_defaults();
        let financials = vec![
            create_test_financial("000001.SZ", 15.0, 25.0), // Passes
            create_test_financial("000002.SZ", 5.0, 25.0),  // Low ROE
            create_test_financial("000003.SZ", 15.0, 10.0), // Low gross margin
        ];

        let (passed, result) = filter.filter_quality(&financials);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].symbol, "000001.SZ");
        assert_eq!(result.eliminated, 2);
    }

    #[test]
    fn test_valuation_filter() {
        let mut config = FilterConfig::default();
        config.valuation.max_pe_absolute = Some(20.0);
        config.valuation.min_dividend_yield = 2.0;

        let filter = QuantitativeFilter::new(config);

        let financials = vec![
            create_test_financial("000001.SZ", 15.0, 25.0), // Passes (PE=15, DY=3%)
        ];

        let (passed, _result) = filter.filter_valuation(&financials);
        assert_eq!(passed.len(), 1);
    }

    #[test]
    fn test_stock_scoring() {
        let filter = QuantitativeFilter::with_defaults();
        let data = create_test_financial("000001.SZ", 20.0, 35.0);

        let score = filter.score_stock(&data);
        assert!(score > 50.0); // Should be a decent score
        assert!(score <= 100.0);
    }

    #[test]
    fn test_filter_result() {
        let result = FilterResult::new(FilterStage::Basic, 100, 80);
        assert_eq!(result.passed, 80);
        assert_eq!(result.eliminated, 20);
        assert!((result.elimination_rate - 20.0).abs() < 0.001);
    }

    // === New tests for scope filters ===

    #[test]
    fn test_target_symbols_filter() {
        let mut config = FilterConfig::default();
        config.target_symbols = vec!["000001.SZ".to_string(), "600000.SH".to_string()];

        let filter = QuantitativeFilter::new(config);
        let stocks = vec![
            create_test_stock("000001", false, "SZ"),
            create_test_stock("000002", false, "SZ"),
            create_test_stock("600000", false, "SH"),
            create_test_stock("600001", false, "SH"),
        ];

        let (passed, _) = filter.filter_basic(&stocks);
        assert_eq!(passed.len(), 2);
        assert_eq!(passed[0].code, "000001");
        assert_eq!(passed[1].code, "600000");
    }

    #[test]
    fn test_industry_include_filter() {
        let mut config = FilterConfig::default();
        config.include_industries = vec!["银行".to_string()];

        let filter = QuantitativeFilter::new(config);

        let stocks = vec![
            StockInfo {
                code: "000001".to_string(),
                name: "Ping An Bank".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("银行".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(100.0),
            },
            StockInfo {
                code: "000002".to_string(),
                name: "Tech Corp".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("科技".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(100.0),
            },
        ];

        let (passed, _) = filter.filter_basic(&stocks);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].industry, Some("银行".to_string()));
    }

    #[test]
    fn test_industry_exclude_filter() {
        let mut config = FilterConfig::default();
        config.exclude_industries = vec!["房地产".to_string()];

        let filter = QuantitativeFilter::new(config);

        let stocks = vec![
            StockInfo {
                code: "000001".to_string(),
                name: "Real Estate Inc".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("房地产".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(100.0),
            },
            StockInfo {
                code: "000002".to_string(),
                name: "Tech Corp".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("科技".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(100.0),
            },
        ];

        let (passed, _) = filter.filter_basic(&stocks);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].industry, Some("科技".to_string()));
    }

    #[test]
    fn test_market_cap_filter() {
        let mut config = FilterConfig::default();
        config.min_market_cap = Some(50.0);
        config.max_market_cap = Some(200.0);

        let filter = QuantitativeFilter::new(config);

        let stocks = vec![
            StockInfo {
                code: "000001".to_string(),
                name: "Large Cap".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("银行".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(300.0), // Too large
            },
            StockInfo {
                code: "000002".to_string(),
                name: "Medium Cap".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("科技".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(100.0), // Within range
            },
            StockInfo {
                code: "000003".to_string(),
                name: "Small Cap".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("医药".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(10.0), // Too small
            },
        ];

        let (passed, _) = filter.filter_basic(&stocks);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].code, "000002");
    }

    #[test]
    fn test_combined_scope_filters() {
        let mut config = FilterConfig::default();
        config.include_industries = vec!["银行".to_string()];
        config.min_market_cap = Some(50.0);

        let filter = QuantitativeFilter::new(config);

        let stocks = vec![
            StockInfo {
                code: "000001".to_string(),
                name: "Small Bank".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("银行".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(10.0), // Too small
            },
            StockInfo {
                code: "000002".to_string(),
                name: "Large Bank".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("银行".to_string()),
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(100.0), // Passes all filters
            },
            StockInfo {
                code: "000003".to_string(),
                name: "Tech Corp".to_string(),
                exchange: "SZ".to_string(),
                industry: Some("科技".to_string()), // Wrong industry
                list_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
                is_suspended: false,
                is_st: false,
                market_cap: Some(100.0),
            },
        ];

        let (passed, _) = filter.filter_basic(&stocks);
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].code, "000002");
    }
}
