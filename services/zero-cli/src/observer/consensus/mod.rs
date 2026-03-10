//! Consensus Layer
//!
//! Aggregates observations from multiple watchers and forms
//! a unified understanding of the world.
//!
//! Implements the "祝融说" philosophy of "观察共识" (observation consensus).

pub mod engine;
pub mod world_model;

pub use engine::{ConsensusConfig, ConsensusEngine, ConsensusSnapshot};
