//! Full Market Screener Module.
//!
//! Provides a system for scanning and filtering the entire A-share market
//! to discover investment opportunities based on fundamental criteria.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                  全市场扫描系统架构                                   │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐           │
//! │  │  周级同步   │────▶│ LocalStorage│────▶│  筛选引擎   │           │
//! │  │  Scheduler  │     │  (SQLite)   │     │  (Engine)   │           │
//! │  └─────────────┘     └──────┬──────┘     └──────┬──────┘           │
//! │                             │                   │                   │
//! │  ┌──────────────────────────┴───────────────────┴───────┐          │
//! │  │              Quantitative Filter                     │          │
//! │  │  - Basic: exclude ST, new stocks, suspended          │          │
//! │  │  - Quality: ROE, gross margin, cash flow DNA         │          │
//! │  │  - Valuation: PE/PB/DY thresholds                    │          │
//! │  └─────────────────────────────────────────────────────────────────┘
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use zero_trading::screener::{ScreenerEngine, ScreenerConfig};
//!
//! let config = ScreenerConfig::default();
//! let engine = ScreenerEngine::new(config, data_provider, local_storage);
//!
//! // Run a full scan
//! let results = engine.run_full_scan().await?;
//!
//! // Or just quick quantitative screening
//! let quick_results = engine.run_quick_scan().await?;
//! ```

pub mod config;
pub mod engine;
pub mod quantitative;
pub mod report;
pub mod scheduler;

pub use config::{ScreenerConfig, ValuationFilterConfig};
pub use engine::{ScreenerEngine, ScreenerResult, ScreenedStock};
pub use quantitative::{QuantitativeFilter, FilterStage, FilterResult};
pub use report::{ScreenerReport, ReportFormat};
pub use scheduler::ScreenerScheduler;
