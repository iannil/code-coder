//! Trace storage and analysis module
//!
//! Provides SQLite-backed storage for trace logs with:
//! - High-performance querying by trace_id, service, time range
//! - Performance profiling with percentile calculations
//! - Error aggregation and analysis
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                      TraceStore                                │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐    │
//! │  │  storage    │  │  profiler   │  │  query/aggregator   │    │
//! │  │  (SQLite)   │  │  (stats)    │  │  (filter/group)     │    │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘    │
//! └────────────────────────────────────────────────────────────────┘
//! ```

pub mod aggregator;
pub mod profiler;
pub mod query;
pub mod storage;

pub use aggregator::{
    aggregate_errors, error_rates_by_service, recent_errors, ErrorGroup, ErrorSample, ErrorSummary,
    GroupBy,
};
pub use profiler::{
    compare_periods, generate_comparison_report, generate_detailed_report, profile_traces,
    FunctionStats, PeriodComparison, PeriodSummary, ProfileResult, ServiceChange, ServiceStats,
    SlowOperation,
};
pub use query::{TraceFilter, TraceQuery};
pub use storage::{TraceEntry, TraceStore, TraceStoreConfig, TraceStoreStats};
