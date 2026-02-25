//! Strategy backtesting module.
//!
//! Provides historical testing of the PO3+SMT strategy with T+1 compliance.

mod engine;
mod metrics;
mod report;

pub use engine::{BacktestEngine, BacktestConfig};
pub use metrics::{BacktestMetrics, TradeRecord};
pub use report::BacktestReport;

// Re-export for convenience
pub use engine::BacktestResult;
