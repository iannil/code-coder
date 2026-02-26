//! Valuation System Module.
//!
//! This module provides a three-dimensional valuation coordinate system for
//! building an "internal scoreboard" (内在记分牌) for investment decisions.
//!
//! # The Three Dimensions
//!
//! 1. **PE (Price/Earnings)**: Growth valuation perspective
//!    - PE-Band analysis: historical percentile positioning
//!    - PEG ratio: growth-adjusted valuation
//!
//! 2. **PB (Price/Book)**: Asset valuation perspective
//!    - PB-ROE model: is current PB justified by return on equity?
//!    - Helps identify value traps vs genuine bargains
//!
//! 3. **DY (Dividend Yield)**: Income valuation perspective
//!    - Yield sustainability analysis
//!    - Comparison to risk-free rate
//!
//! # Usage
//!
//! ```ignore
//! use zero_trading::valuation::{ValuationAnalyzer, ValuationInput};
//!
//! let analyzer = ValuationAnalyzer::new();
//! let input = ValuationInput {
//!     symbol: "600519".to_string(),
//!     price: 1800.0,
//!     eps_ttm: 60.0,
//!     // ... other fields
//! };
//!
//! let result = analyzer.analyze(&input)?;
//!
//! println!("Valuation Zone: {}", result.valuation_zone);
//! println!("Margin of Safety: {:.1}%", result.margin_of_safety);
//! ```

pub mod analyzer;
pub mod types;

pub use analyzer::{ValuationAnalyzer, ValuationConfig};
pub use types::{
    DividendYieldAnalysis, HistoricalPe, InvestorType, PbRoeAnalysis, PeBandAnalysis,
    PeBandPosition, ValuationCoordinates, ValuationInput, ValuationZone,
};
