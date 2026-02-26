//! Portfolio Management Module.
//!
//! This module implements the "Trim Weak, Nurture Strong" (斩弱养强) investment
//! discipline with three-tier pools and quantified decision rules.
//!
//! # Components
//!
//! - **Pools**: Three-tier structure (Core/Satellite/Watchlist)
//! - **Signals**: Red/yellow light system for sell decisions
//! - **Dip Buying**: Golden pit vs value trap assessment
//!
//! # Philosophy
//!
//! The portfolio system embodies several key behavioral finance insights:
//!
//! 1. **Position differentiation**: Core holdings get patience; satellites get discipline
//! 2. **Pre-commitment**: Define sell criteria before buying, not after
//! 3. **Capital efficiency**: Free capital from losers to fund winners
//! 4. **Dip discipline**: Not every decline is an opportunity
//!
//! # Usage
//!
//! ```ignore
//! use zero_trading::portfolio::{PoolsManager, SignalAnalyzer, DipAnalyzer};
//!
//! // Create portfolio manager
//! let manager = PoolsManager::with_cash(config, 100_000.0);
//!
//! // Analyze position signals
//! let analyzer = SignalAnalyzer::default();
//! let signals = analyzer.analyze(&position, &context);
//!
//! // Assess dip buying opportunity
//! let dip_analyzer = DipAnalyzer::default();
//! let assessment = dip_analyzer.assess("600519", checklist);
//! ```

pub mod allocation;
pub mod dip_buying;
pub mod pools;
pub mod pyramid_executor;
pub mod signals;
pub mod types;

// Re-export main types
pub use allocation::{
    AddCandidate, AllocationOrder, AllocationPriority, AllocationRecommendation, AllocationSide,
    AllocatorConfig, CapitalAllocator, TrimCandidate, TrimReason,
};

pub use dip_buying::{
    BalanceSheetHealth, DeclineDriver, DipAnalyzer, DipAnalyzerConfig, DipAssessment, DipChecklist,
    InsiderActivity, MoatStatus, PyramidPlan, PyramidStrategy, PyramidTranche,
};

pub use pools::{PoolsConfig, PoolsManager, RebalanceNeeds};

pub use pyramid_executor::{
    ExecutionResult, ExecutorStats, OrderSide, OrderStatus, OrderType, PyramidExecutor,
    PyramidExecutorConfig, PyramidOrder, PyramidPlanBuilder,
};

pub use signals::{
    BenchmarkData, DividendData, RecommendedAction, SignalAnalyzer, SignalAssessment, SignalConfig,
    SignalContext, SignalLevel, SignalTrigger, TechnicalData,
};

pub use types::{PoolTier, PortfolioPools, PortfolioSummary, Position, StopLossTriggers, WatchItem};
