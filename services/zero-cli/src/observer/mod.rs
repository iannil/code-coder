//! Observer Network Module
//!
//! Implements the core Observer Network for CodeCoder, transforming it from
//! an execution-centric to an observation-centric system.
//!
//! # Philosophy (祝融说)
//!
//! - **可能性基底 (Possibility Substrate)**: Raw observation events flow through the system
//! - **观察即收敛 (Observation as Convergence)**: Consensus forms from multiple observers
//! - **可用余量 (Available Margin)**: Mode switching preserves control flexibility
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Observer Layer (Watchers)                               │
//! │   CodeWatch │ WorldWatch │ SelfWatch │ MetaWatch                            │
//! └──────────────────────────────┬──────────────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Event Stream (Buffer + Route + Aggregate)               │
//! └──────────────────────────────┬──────────────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Consensus Layer                                         │
//! │   Attention │ PatternDetector │ AnomalyDetector │ WorldModelBuilder         │
//! └──────────────────────────────┬──────────────────────────────────────────────┘
//!                                │
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Response Layer                                          │
//! │   Gear Integration │ SSE Events │ World Model Snapshots                     │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

pub mod consensus;
pub mod network;
pub mod types;
pub mod watchers;

// Re-export primary types
pub use consensus::ConsensusSnapshot;
pub use types::{
    Anomaly, EmergentPattern, Observation, Opportunity, WorldModel,
};

// Re-export watcher types
pub use watchers::{
    WatcherManager, WatcherMetrics, WatcherStatus,
};

// Re-export from zero-hub's observer module for compatibility
