//! Hands autonomous agent system.
//!
//! Hands are persistent, stateful autonomous agents that:
//! - Define their behavior via HAND.md files (YAML frontmatter + Markdown)
//! - Execute on cron schedules
//! - Maintain state in SQLite
//! - Write execution logs to Markdown memory files
//!
//! # HAND.md Format
//!
//! ```markdown
//! ---
//! id: "market-sentinel"
//! name: "Market Sentinel"
//! version: "1.0.0"
//! schedule: "0 */30 * * * *"
//! agent: "macro"
//! enabled: true
//! memory_path: "hands/market-sentinel/{date}.md"
//! params:
//!   threshold: 0.7
//! ---
//!
//! # Market Sentinel
//!
//! Monitors market conditions...
//! ```

pub mod auto_approve;
pub mod autonomous_bridge;
pub mod close;
pub mod executor;
pub mod manifest;
pub mod risk;
pub mod scheduler;
pub mod state;

pub use auto_approve::{ApprovalDecision, ApprovalResult, AutoApprover, AutoApproverBuilder};
pub use autonomous_bridge::{AutonomousBridge, AutonomousConfig, AutonomousRequest, AutonomousResponse, CLOSEScoreResult, HandsContext, PreviousResult, ResourceBudget};
pub use close::{CloseCriteria, CloseDecision, CloseEvaluator};
pub use executor::HandExecutor;
pub use manifest::{
    discover_hands, hands_dir, AutonomyConfig, AutoApproveConfig, DecisionConfig, HandConfig, HandManifest,
    HandSummary, ResourceLimits, RiskThreshold,
};
pub use risk::{RiskEvaluation, RiskEvaluator, RiskLevel};
pub use scheduler::HandsScheduler;
pub use state::{ExecutionStatus, HandExecution, HandState, StateStore};
