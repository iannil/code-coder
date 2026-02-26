//! Valuation System Types.
//!
//! Defines the three-dimensional valuation coordinate system (PE/PB/DY)
//! for building an "internal scoreboard" (内在记分牌) for investment decisions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// PE Analysis Types
// ============================================================================

/// Position of current PE in historical distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PeBandPosition {
    /// Above historical high (极端高估)
    AboveHistoricalHigh,
    /// In optimistic zone, 80%+ percentile (乐观区)
    OptimisticZone,
    /// In neutral zone, 20%-80% percentile (中性区)
    NeutralZone,
    /// In pessimistic zone, below 20% percentile (悲观区)
    PessimisticZone,
    /// Below historical low (极端低估)
    BelowHistoricalLow,
}

impl std::fmt::Display for PeBandPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AboveHistoricalHigh => write!(f, "极端高估"),
            Self::OptimisticZone => write!(f, "乐观区"),
            Self::NeutralZone => write!(f, "中性区"),
            Self::PessimisticZone => write!(f, "悲观区"),
            Self::BelowHistoricalLow => write!(f, "极端低估"),
        }
    }
}

/// PE-Band analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeBandAnalysis {
    /// Current TTM PE
    pub pe_ttm: f64,
    /// Forward PE (if available)
    pub pe_forward: Option<f64>,
    /// PEG ratio (PE / EPS growth rate)
    pub peg: Option<f64>,
    /// Historical median PE
    pub pe_median: f64,
    /// 20th percentile PE (pessimistic boundary)
    pub pe_20th: f64,
    /// 80th percentile PE (optimistic boundary)
    pub pe_80th: f64,
    /// Historical minimum PE
    pub pe_min: f64,
    /// Historical maximum PE
    pub pe_max: f64,
    /// Current position in PE band
    pub position: PeBandPosition,
    /// Percentile of current PE (0-100)
    pub percentile: f64,
    /// Analysis period (years)
    pub period_years: u32,
}

// ============================================================================
// PB-ROE Analysis Types
// ============================================================================

/// PB-ROE model analysis result.
///
/// The PB-ROE model suggests that fair PB = (ROE - g) / (r - g)
/// where r = required return, g = growth rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PbRoeAnalysis {
    /// Current PB ratio
    pub pb: f64,
    /// Return on Equity (%)
    pub roe: f64,
    /// Sustainable growth rate (%)
    pub growth_rate: f64,
    /// Required rate of return (%)
    pub required_return: f64,
    /// Fair value PB from PB-ROE model
    pub fair_pb: f64,
    /// Premium/discount to fair value (%)
    pub premium_discount: f64,
    /// Whether current PB is justified by ROE
    pub is_justified: bool,
    /// Analysis reasoning
    pub reasoning: String,
}

// ============================================================================
// Dividend Yield Analysis Types
// ============================================================================

/// Dividend yield analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividendYieldAnalysis {
    /// Current dividend yield (%)
    pub dividend_yield: f64,
    /// Payout ratio (%)
    pub payout_ratio: f64,
    /// Dividend growth rate over past 5 years (%)
    pub dividend_growth_5y: Option<f64>,
    /// Consecutive years of dividend increases
    pub consecutive_increase_years: Option<u32>,
    /// Dividend sustainability score (0-100)
    pub sustainability_score: f64,
    /// Yield percentile vs history (0-100)
    pub yield_percentile: f64,
    /// Is yield attractive relative to bonds?
    pub is_attractive_vs_bonds: bool,
    /// Reference risk-free rate used
    pub risk_free_rate: f64,
}

// ============================================================================
// Valuation Zone Types
// ============================================================================

/// Overall valuation zone classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValuationZone {
    /// Deep value zone - strong buy signal (深度价值区)
    DeepValue,
    /// Fair value zone - hold or accumulate (合理价值区)
    FairValue,
    /// Overvalued zone - consider trimming (高估区)
    Overvalued,
    /// Bubble zone - sell or avoid (泡沫区)
    Bubble,
}

impl std::fmt::Display for ValuationZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeepValue => write!(f, "深度价值区"),
            Self::FairValue => write!(f, "合理价值区"),
            Self::Overvalued => write!(f, "高估区"),
            Self::Bubble => write!(f, "泡沫区"),
        }
    }
}

/// Investor type that this valuation is suitable for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvestorType {
    /// Growth investor - focus on PE and PEG
    GrowthInvestor,
    /// Value investor - focus on PB and dividend yield
    ValueInvestor,
    /// Balanced investor - equal weight to all metrics
    BalancedInvestor,
    /// Income investor - focus on dividend yield
    IncomeInvestor,
}

impl std::fmt::Display for InvestorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GrowthInvestor => write!(f, "成长型投资者"),
            Self::ValueInvestor => write!(f, "价值型投资者"),
            Self::BalancedInvestor => write!(f, "平衡型投资者"),
            Self::IncomeInvestor => write!(f, "收益型投资者"),
        }
    }
}

// ============================================================================
// Comprehensive Valuation Coordinates
// ============================================================================

/// Three-dimensional valuation coordinates.
///
/// This combines PE, PB, and DY analysis into a comprehensive valuation view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationCoordinates {
    /// Symbol/ticker
    pub symbol: String,

    // === PE Dimension ===
    /// PE-Band analysis
    pub pe_analysis: PeBandAnalysis,

    // === PB Dimension ===
    /// PB-ROE analysis
    pub pb_analysis: PbRoeAnalysis,

    // === Dividend Yield Dimension ===
    /// Dividend yield analysis
    pub dy_analysis: DividendYieldAnalysis,

    // === Composite Assessment ===
    /// Overall valuation score (0-100, lower = cheaper)
    pub overall_score: f64,
    /// Best suited investor type
    pub investor_type_fit: InvestorType,
    /// Overall valuation zone
    pub valuation_zone: ValuationZone,
    /// Margin of safety (%) - positive means undervalued
    pub margin_of_safety: f64,
    /// Key highlights
    pub highlights: Vec<String>,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

impl ValuationCoordinates {
    /// Returns whether the stock is in an attractive buying zone.
    pub fn is_buy_zone(&self) -> bool {
        matches!(self.valuation_zone, ValuationZone::DeepValue | ValuationZone::FairValue)
            && self.margin_of_safety > 0.0
    }

    /// Returns whether the stock should be avoided due to high valuation.
    pub fn is_avoid_zone(&self) -> bool {
        matches!(self.valuation_zone, ValuationZone::Bubble)
    }

    /// Returns the primary valuation signal.
    pub fn primary_signal(&self) -> &str {
        match self.valuation_zone {
            ValuationZone::DeepValue => "强烈买入信号",
            ValuationZone::FairValue => "可考虑买入",
            ValuationZone::Overvalued => "谨慎/持有",
            ValuationZone::Bubble => "回避/卖出",
        }
    }
}

// ============================================================================
// Input Types
// ============================================================================

/// Historical PE data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalPe {
    pub date: DateTime<Utc>,
    pub pe: f64,
}

/// Market data input for valuation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValuationInput {
    /// Symbol/ticker
    pub symbol: String,
    /// Current stock price
    pub price: f64,
    /// Earnings per share (TTM)
    pub eps_ttm: f64,
    /// Forward EPS estimate (optional)
    pub eps_forward: Option<f64>,
    /// EPS growth rate (%)
    pub eps_growth_rate: Option<f64>,
    /// Book value per share
    pub book_value_per_share: f64,
    /// Return on Equity (%)
    pub roe: f64,
    /// Dividend per share
    pub dividend_per_share: f64,
    /// Payout ratio (%)
    pub payout_ratio: f64,
    /// Dividend growth rate 5Y (%)
    pub dividend_growth_5y: Option<f64>,
    /// Consecutive years of dividend increases
    pub consecutive_dividend_years: Option<u32>,
    /// Historical PE data for band analysis
    pub historical_pe: Vec<HistoricalPe>,
    /// Risk-free rate for comparison
    pub risk_free_rate: f64,
}

impl ValuationInput {
    /// Calculate current PE ratio.
    pub fn pe_ttm(&self) -> f64 {
        if self.eps_ttm > 0.0 {
            self.price / self.eps_ttm
        } else {
            f64::NAN
        }
    }

    /// Calculate forward PE ratio.
    pub fn pe_forward(&self) -> Option<f64> {
        self.eps_forward.map(|eps| {
            if eps > 0.0 {
                self.price / eps
            } else {
                f64::NAN
            }
        })
    }

    /// Calculate PB ratio.
    pub fn pb(&self) -> f64 {
        if self.book_value_per_share > 0.0 {
            self.price / self.book_value_per_share
        } else {
            f64::NAN
        }
    }

    /// Calculate dividend yield (%).
    pub fn dividend_yield(&self) -> f64 {
        if self.price > 0.0 {
            (self.dividend_per_share / self.price) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate PEG ratio.
    pub fn peg(&self) -> Option<f64> {
        self.eps_growth_rate.map(|growth| {
            if growth > 0.0 && self.eps_ttm > 0.0 {
                self.pe_ttm() / growth
            } else {
                f64::NAN
            }
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pe_band_position_display() {
        assert_eq!(PeBandPosition::BelowHistoricalLow.to_string(), "极端低估");
        assert_eq!(PeBandPosition::OptimisticZone.to_string(), "乐观区");
    }

    #[test]
    fn test_valuation_zone_display() {
        assert_eq!(ValuationZone::DeepValue.to_string(), "深度价值区");
        assert_eq!(ValuationZone::Bubble.to_string(), "泡沫区");
    }

    #[test]
    fn test_valuation_input_calculations() {
        let input = ValuationInput {
            symbol: "TEST".to_string(),
            price: 100.0,
            eps_ttm: 5.0,
            eps_forward: Some(6.0),
            eps_growth_rate: Some(20.0),
            book_value_per_share: 50.0,
            roe: 15.0,
            dividend_per_share: 2.0,
            payout_ratio: 40.0,
            dividend_growth_5y: Some(10.0),
            consecutive_dividend_years: Some(5),
            historical_pe: vec![],
            risk_free_rate: 3.0,
        };

        assert!((input.pe_ttm() - 20.0).abs() < 0.01);
        assert!((input.pe_forward().unwrap() - 16.67).abs() < 0.01);
        assert!((input.pb() - 2.0).abs() < 0.01);
        assert!((input.dividend_yield() - 2.0).abs() < 0.01);
        assert!((input.peg().unwrap() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_investor_type_display() {
        assert_eq!(InvestorType::GrowthInvestor.to_string(), "成长型投资者");
        assert_eq!(InvestorType::IncomeInvestor.to_string(), "收益型投资者");
    }
}
