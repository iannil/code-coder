//! Valuation Analyzer.
//!
//! Implements the three-dimensional valuation analysis combining PE, PB, and DY metrics.

use anyhow::Result;
use chrono::{Duration, Utc};
use std::sync::Arc;

use super::types::*;
use crate::data::LocalStorage;

/// Valuation analyzer configuration.
#[derive(Debug, Clone)]
pub struct ValuationConfig {
    /// Default required rate of return for PB-ROE model (%)
    pub default_required_return: f64,
    /// Low PE threshold (for value identification)
    pub low_pe_threshold: f64,
    /// High PE threshold (for overvaluation warning)
    pub high_pe_threshold: f64,
    /// Attractive PEG threshold (for growth at reasonable price)
    pub attractive_peg_threshold: f64,
    /// Minimum dividend yield to be considered attractive (%)
    pub min_attractive_yield: f64,
    /// Deep value zone score threshold
    pub deep_value_threshold: f64,
    /// Bubble zone score threshold
    pub bubble_threshold: f64,
}

impl Default for ValuationConfig {
    fn default() -> Self {
        Self {
            default_required_return: 10.0,
            low_pe_threshold: 15.0,
            high_pe_threshold: 30.0,
            attractive_peg_threshold: 1.0,
            min_attractive_yield: 3.0,
            deep_value_threshold: 30.0,
            bubble_threshold: 80.0,
        }
    }
}

/// Three-dimensional valuation analyzer.
pub struct ValuationAnalyzer {
    config: ValuationConfig,
    /// Local storage for caching analysis results
    local_storage: Option<Arc<LocalStorage>>,
}

impl ValuationAnalyzer {
    /// Create a new valuation analyzer with default config.
    pub fn new() -> Self {
        Self {
            config: ValuationConfig::default(),
            local_storage: None,
        }
    }

    /// Create with local storage.
    pub fn with_local_storage(local_storage: Option<Arc<LocalStorage>>) -> Self {
        Self {
            config: ValuationConfig::default(),
            local_storage,
        }
    }

    /// Create with custom config.
    pub fn with_config(config: ValuationConfig) -> Self {
        Self {
            config,
            local_storage: None,
        }
    }

    /// Create with custom config and local storage.
    pub fn with_config_and_storage(
        config: ValuationConfig,
        local_storage: Option<Arc<LocalStorage>>,
    ) -> Self {
        Self {
            config,
            local_storage,
        }
    }

    /// Perform comprehensive valuation analysis.
    pub fn analyze(&self, input: &ValuationInput) -> Result<ValuationCoordinates> {
        // Analyze each dimension
        let pe_analysis = self.analyze_pe_band(input);
        let pb_analysis = self.analyze_pb_roe(input);
        let dy_analysis = self.analyze_dividend_yield(input);

        // Calculate overall score and zone
        let overall_score = self.calculate_overall_score(&pe_analysis, &pb_analysis, &dy_analysis);
        let valuation_zone = self.determine_valuation_zone(overall_score);
        let margin_of_safety = self.calculate_margin_of_safety(&pe_analysis, &pb_analysis);
        let investor_type_fit = self.determine_investor_type(input, &pe_analysis, &dy_analysis);
        let highlights = self.generate_highlights(&pe_analysis, &pb_analysis, &dy_analysis, input);

        Ok(ValuationCoordinates {
            symbol: input.symbol.clone(),
            pe_analysis,
            pb_analysis,
            dy_analysis,
            overall_score,
            investor_type_fit,
            valuation_zone,
            margin_of_safety,
            highlights,
            analyzed_at: Utc::now(),
        })
    }

    /// Perform valuation analysis with caching support.
    /// Uses local storage to cache results for 1 hour.
    pub async fn analyze_cached(&self, input: &ValuationInput) -> Result<ValuationCoordinates> {
        const ANALYSIS_TYPE: &str = "valuation_coordinates";
        const CACHE_HOURS: i64 = 1; // Valuation changes with price, cache shorter

        // Try cache first
        if let Some(ref storage) = self.local_storage {
            if let Ok(Some(cached)) = storage
                .get_analysis_cache::<ValuationCoordinates>(&input.symbol, ANALYSIS_TYPE)
                .await
            {
                tracing::debug!(symbol = %input.symbol, "Using cached valuation analysis");
                return Ok(cached);
            }
        }

        // Compute analysis
        let result = self.analyze(input)?;

        // Cache result
        if let Some(ref storage) = self.local_storage {
            let expires_at = Utc::now() + Duration::hours(CACHE_HOURS);
            if let Err(e) = storage
                .save_analysis_cache(&input.symbol, ANALYSIS_TYPE, &result, Some(expires_at))
                .await
            {
                tracing::warn!(error = %e, "Failed to cache valuation analysis");
            }
        }

        Ok(result)
    }

    /// Analyze PE-Band.
    fn analyze_pe_band(&self, input: &ValuationInput) -> PeBandAnalysis {
        let pe_ttm = input.pe_ttm();
        let pe_forward = input.pe_forward();
        let peg = input.peg();

        // Extract historical PE values
        let pe_values: Vec<f64> = input
            .historical_pe
            .iter()
            .map(|h| h.pe)
            .filter(|pe| pe.is_finite() && *pe > 0.0)
            .collect();

        // Calculate statistics
        let (pe_median, pe_20th, pe_80th, pe_min, pe_max) = if pe_values.len() >= 10 {
            calculate_pe_statistics(&pe_values)
        } else {
            // Use default industry averages if insufficient history
            (20.0, 12.0, 30.0, 8.0, 50.0)
        };

        // Determine position
        let position = if pe_ttm > pe_max {
            PeBandPosition::AboveHistoricalHigh
        } else if pe_ttm > pe_80th {
            PeBandPosition::OptimisticZone
        } else if pe_ttm < pe_min {
            PeBandPosition::BelowHistoricalLow
        } else if pe_ttm < pe_20th {
            PeBandPosition::PessimisticZone
        } else {
            PeBandPosition::NeutralZone
        };

        // Calculate percentile
        let percentile = calculate_percentile(pe_ttm, &pe_values);

        let period_years = if !input.historical_pe.is_empty() {
            let first = input.historical_pe.first().unwrap().date;
            let last = input.historical_pe.last().unwrap().date;
            ((last - first).num_days() / 365) as u32
        } else {
            5
        };

        PeBandAnalysis {
            pe_ttm,
            pe_forward,
            peg,
            pe_median,
            pe_20th,
            pe_80th,
            pe_min,
            pe_max,
            position,
            percentile,
            period_years,
        }
    }

    /// Analyze PB-ROE relationship.
    fn analyze_pb_roe(&self, input: &ValuationInput) -> PbRoeAnalysis {
        let pb = input.pb();
        let roe = input.roe;

        // Estimate sustainable growth rate (ROE * retention ratio)
        let retention_ratio = 1.0 - (input.payout_ratio / 100.0);
        let growth_rate = roe * retention_ratio;

        let required_return = self.config.default_required_return;

        // Calculate fair PB using Gordon Growth Model variant
        // Fair PB = (ROE - g) / (r - g)
        let fair_pb = if required_return > growth_rate {
            (roe - growth_rate) / (required_return - growth_rate)
        } else {
            // Growth exceeds required return - use simplified model
            roe / required_return
        };

        let premium_discount = if fair_pb > 0.0 {
            ((pb - fair_pb) / fair_pb) * 100.0
        } else {
            0.0
        };

        // PB is justified if it's within reasonable range of fair value
        let is_justified = premium_discount.abs() < 30.0;

        let reasoning = generate_pb_reasoning(pb, fair_pb, roe, premium_discount);

        PbRoeAnalysis {
            pb,
            roe,
            growth_rate,
            required_return,
            fair_pb,
            premium_discount,
            is_justified,
            reasoning,
        }
    }

    /// Analyze dividend yield.
    fn analyze_dividend_yield(&self, input: &ValuationInput) -> DividendYieldAnalysis {
        let dividend_yield = input.dividend_yield();
        let payout_ratio = input.payout_ratio;

        // Calculate sustainability score
        let sustainability_score = calculate_dividend_sustainability(
            payout_ratio,
            input.roe,
            input.dividend_growth_5y,
            input.consecutive_dividend_years,
        );

        // Compare to risk-free rate
        let is_attractive_vs_bonds = dividend_yield > input.risk_free_rate + 1.0;

        // Yield percentile (simplified - would need historical data in production)
        let yield_percentile = if dividend_yield > 5.0 {
            90.0
        } else if dividend_yield > 4.0 {
            75.0
        } else if dividend_yield > 3.0 {
            50.0
        } else if dividend_yield > 2.0 {
            30.0
        } else {
            15.0
        };

        DividendYieldAnalysis {
            dividend_yield,
            payout_ratio,
            dividend_growth_5y: input.dividend_growth_5y,
            consecutive_increase_years: input.consecutive_dividend_years,
            sustainability_score,
            yield_percentile,
            is_attractive_vs_bonds,
            risk_free_rate: input.risk_free_rate,
        }
    }

    /// Calculate overall valuation score (0-100, lower = cheaper).
    fn calculate_overall_score(
        &self,
        pe: &PeBandAnalysis,
        pb: &PbRoeAnalysis,
        dy: &DividendYieldAnalysis,
    ) -> f64 {
        // PE dimension (0-40 points)
        let pe_score = pe.percentile * 0.4;

        // PB dimension (0-35 points)
        let pb_score = if pb.premium_discount < -30.0 {
            0.0
        } else if pb.premium_discount > 50.0 {
            35.0
        } else {
            ((pb.premium_discount + 30.0) / 80.0) * 35.0
        };

        // DY dimension (0-25 points) - inverse: higher yield = lower score
        let dy_score = (100.0 - dy.yield_percentile) * 0.25;

        (pe_score + pb_score + dy_score).clamp(0.0, 100.0)
    }

    /// Determine overall valuation zone.
    fn determine_valuation_zone(&self, score: f64) -> ValuationZone {
        if score <= self.config.deep_value_threshold {
            ValuationZone::DeepValue
        } else if score <= 50.0 {
            ValuationZone::FairValue
        } else if score <= self.config.bubble_threshold {
            ValuationZone::Overvalued
        } else {
            ValuationZone::Bubble
        }
    }

    /// Calculate margin of safety.
    fn calculate_margin_of_safety(&self, pe: &PeBandAnalysis, pb: &PbRoeAnalysis) -> f64 {
        // Average of PE and PB margins
        let pe_margin = ((pe.pe_median - pe.pe_ttm) / pe.pe_median) * 100.0;
        let pb_margin = -pb.premium_discount; // Invert: negative premium = positive margin

        (pe_margin + pb_margin) / 2.0
    }

    /// Determine best-fit investor type.
    fn determine_investor_type(
        &self,
        input: &ValuationInput,
        pe: &PeBandAnalysis,
        dy: &DividendYieldAnalysis,
    ) -> InvestorType {
        let has_growth = input.eps_growth_rate.map(|g| g > 15.0).unwrap_or(false);
        let has_low_peg = pe.peg.map(|p| p < 1.5).unwrap_or(false);
        let has_high_yield = dy.dividend_yield > self.config.min_attractive_yield;
        let is_low_pb = input.pb() < 1.5;

        match (has_growth && has_low_peg, has_high_yield, is_low_pb) {
            (true, _, _) => InvestorType::GrowthInvestor,
            (_, true, _) => InvestorType::IncomeInvestor,
            (_, _, true) => InvestorType::ValueInvestor,
            _ => InvestorType::BalancedInvestor,
        }
    }

    /// Generate analysis highlights.
    fn generate_highlights(
        &self,
        pe: &PeBandAnalysis,
        pb: &PbRoeAnalysis,
        dy: &DividendYieldAnalysis,
        input: &ValuationInput,
    ) -> Vec<String> {
        let mut highlights = Vec::new();

        // PE highlights
        match pe.position {
            PeBandPosition::BelowHistoricalLow => {
                highlights.push(format!(
                    "PE ({:.1}) 低于历史最低值，处于极端低估状态",
                    pe.pe_ttm
                ));
            }
            PeBandPosition::PessimisticZone => {
                highlights.push(format!(
                    "PE ({:.1}) 处于悲观区，历史分位 {:.0}%",
                    pe.pe_ttm, pe.percentile
                ));
            }
            PeBandPosition::AboveHistoricalHigh => {
                highlights.push(format!(
                    "PE ({:.1}) 高于历史最高值，估值风险较大",
                    pe.pe_ttm
                ));
            }
            _ => {}
        }

        // PEG highlight
        if let Some(peg) = pe.peg {
            if peg < 1.0 {
                highlights.push(format!(
                    "PEG ({:.2}) < 1，成长性价比优秀",
                    peg
                ));
            } else if peg > 2.0 {
                highlights.push(format!(
                    "PEG ({:.2}) > 2，成长溢价较高",
                    peg
                ));
            }
        }

        // PB-ROE highlight
        if pb.premium_discount < -20.0 {
            highlights.push(format!(
                "PB ({:.2}) 相对公允价值折价 {:.0}%",
                pb.pb,
                pb.premium_discount.abs()
            ));
        } else if pb.premium_discount > 30.0 {
            highlights.push(format!(
                "PB ({:.2}) 相对公允价值溢价 {:.0}%",
                pb.pb, pb.premium_discount
            ));
        }

        // Dividend highlight
        if dy.is_attractive_vs_bonds && dy.sustainability_score > 70.0 {
            highlights.push(format!(
                "股息率 {:.1}% 超过无风险利率，可持续性评分 {:.0}",
                dy.dividend_yield, dy.sustainability_score
            ));
        }

        // ROE quality
        if input.roe > 20.0 {
            highlights.push(format!("ROE {:.1}% 展现优秀的资本回报能力", input.roe));
        }

        highlights
    }
}

impl Default for ValuationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn calculate_pe_statistics(pe_values: &[f64]) -> (f64, f64, f64, f64, f64) {
    let mut sorted: Vec<f64> = pe_values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    let median = if len % 2 == 0 {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    } else {
        sorted[len / 2]
    };

    let idx_20 = (len as f64 * 0.2).floor() as usize;
    let idx_80 = (len as f64 * 0.8).floor() as usize;

    let pe_20th = sorted.get(idx_20).copied().unwrap_or(median * 0.6);
    let pe_80th = sorted.get(idx_80).copied().unwrap_or(median * 1.5);
    let pe_min = sorted.first().copied().unwrap_or(median * 0.5);
    let pe_max = sorted.last().copied().unwrap_or(median * 2.0);

    (median, pe_20th, pe_80th, pe_min, pe_max)
}

fn calculate_percentile(value: f64, data: &[f64]) -> f64 {
    if data.is_empty() {
        return 50.0;
    }

    let count_below = data.iter().filter(|&&x| x < value).count();
    (count_below as f64 / data.len() as f64) * 100.0
}

fn calculate_dividend_sustainability(
    payout_ratio: f64,
    roe: f64,
    growth_5y: Option<f64>,
    consecutive_years: Option<u32>,
) -> f64 {
    let mut score: f64 = 50.0;

    // Payout ratio impact (0-30 points)
    if payout_ratio > 0.0 && payout_ratio <= 50.0 {
        score += 30.0;
    } else if payout_ratio <= 70.0 {
        score += 20.0;
    } else if payout_ratio <= 90.0 {
        score += 10.0;
    } else if payout_ratio > 100.0 {
        score -= 20.0; // Unsustainable
    }

    // ROE support (0-20 points)
    if roe >= 15.0 {
        score += 20.0;
    } else if roe >= 10.0 {
        score += 10.0;
    }

    // Growth track record
    if let Some(growth) = growth_5y {
        if growth > 5.0 {
            score += 10.0;
        }
    }

    // Consecutive years
    if let Some(years) = consecutive_years {
        if years >= 10 {
            score += 15.0;
        } else if years >= 5 {
            score += 10.0;
        }
    }

    score.clamp(0.0, 100.0)
}

fn generate_pb_reasoning(pb: f64, fair_pb: f64, roe: f64, premium: f64) -> String {
    let mut parts = Vec::new();

    parts.push(format!(
        "当前PB {:.2}，基于ROE {:.1}%的合理PB为 {:.2}。",
        pb, roe, fair_pb
    ));

    if premium < -20.0 {
        parts.push(format!(
            "当前估值相对合理水平折价 {:.0}%，具有安全边际。",
            premium.abs()
        ));
    } else if premium > 30.0 {
        parts.push(format!(
            "当前估值溢价 {:.0}%，需要高增长预期支撑。",
            premium
        ));
    } else {
        parts.push("估值处于合理区间。".to_string());
    }

    if pb < 1.0 && roe > 10.0 {
        parts.push("低PB配合正ROE，可能是价值陷阱或被低估的优质资产，需深入分析。".to_string());
    }

    parts.join(" ")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_input() -> ValuationInput {
        ValuationInput {
            symbol: "TEST".to_string(),
            price: 100.0,
            eps_ttm: 5.0,
            eps_forward: Some(6.0),
            eps_growth_rate: Some(20.0),
            book_value_per_share: 50.0,
            roe: 15.0,
            dividend_per_share: 2.5,
            payout_ratio: 50.0,
            dividend_growth_5y: Some(8.0),
            consecutive_dividend_years: Some(5),
            historical_pe: vec![],
            risk_free_rate: 3.0,
        }
    }

    #[test]
    fn test_analyze_basic() {
        let analyzer = ValuationAnalyzer::new();
        let input = make_test_input();

        let result = analyzer.analyze(&input).unwrap();

        assert_eq!(result.symbol, "TEST");
        assert!((result.pe_analysis.pe_ttm - 20.0).abs() < 0.01);
        assert!((result.pb_analysis.pb - 2.0).abs() < 0.01);
        assert!((result.dy_analysis.dividend_yield - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_pe_band_position() {
        let analyzer = ValuationAnalyzer::new();

        // Create input with historical PE data
        let mut input = make_test_input();
        input.historical_pe = (0..100)
            .map(|i| HistoricalPe {
                date: Utc::now(),
                pe: 15.0 + (i as f64 * 0.2), // PE range 15-35
            })
            .collect();

        let result = analyzer.analyze(&input).unwrap();

        // Current PE of 20 should be in lower half of 15-35 range
        assert!(
            matches!(
                result.pe_analysis.position,
                PeBandPosition::NeutralZone | PeBandPosition::PessimisticZone
            ),
            "Position: {:?}, Percentile: {}",
            result.pe_analysis.position,
            result.pe_analysis.percentile
        );
    }

    #[test]
    fn test_deep_value_detection() {
        let analyzer = ValuationAnalyzer::new();

        let mut input = make_test_input();
        input.price = 40.0; // Much lower price
        input.eps_ttm = 5.0; // PE = 8

        let result = analyzer.analyze(&input).unwrap();

        // Very low PE should result in low score (meaning cheap)
        assert!(
            result.overall_score < 50.0,
            "Score: {}",
            result.overall_score
        );
    }

    #[test]
    fn test_margin_of_safety_calculation() {
        let analyzer = ValuationAnalyzer::new();
        let input = make_test_input();

        let result = analyzer.analyze(&input).unwrap();

        // Margin of safety should be a reasonable number
        assert!(
            result.margin_of_safety.is_finite(),
            "MOS: {}",
            result.margin_of_safety
        );
    }

    #[test]
    fn test_investor_type_fit() {
        let analyzer = ValuationAnalyzer::new();

        // Growth stock scenario
        let mut growth_input = make_test_input();
        growth_input.eps_growth_rate = Some(25.0);
        let result = analyzer.analyze(&growth_input).unwrap();
        assert_eq!(result.investor_type_fit, InvestorType::GrowthInvestor);

        // Income stock scenario
        let mut income_input = make_test_input();
        income_input.dividend_per_share = 5.0; // 5% yield
        income_input.eps_growth_rate = Some(5.0);
        let result = analyzer.analyze(&income_input).unwrap();
        assert_eq!(result.investor_type_fit, InvestorType::IncomeInvestor);
    }
}
