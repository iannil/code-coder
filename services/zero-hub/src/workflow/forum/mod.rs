//! Forum-based multi-agent collaboration.
//!
//! This module provides infrastructure for structured multi-agent discussions,
//! where agents take turns contributing to a topic in rounds.
//!
//! ## Architecture
//!
//! - **Session**: Tracks discussion state (participants, turns, rounds)
//! - **Scheduler**: Manages turn order, timeouts, and round progression
//!
//! ## Design Principle
//!
//! The Rust components handle **deterministic** aspects:
//! - Turn order management
//! - Timeout handling
//! - State persistence
//!
//! The TypeScript components handle **non-deterministic** aspects:
//! - Moderating discussions
//! - Detecting consensus
//! - Summarizing outcomes
//!
//! ## Example
//!
//! ```rust,ignore
//! use zero_hub::workflow::forum::{ForumSession, ForumScheduler};
//!
//! // Create a session
//! let mut session = ForumSession::new(
//!     "debate-1",
//!     "Should we use microservices?",
//!     vec!["architect".into(), "developer".into(), "ops".into()],
//! );
//!
//! // Start and run
//! session.start()?;
//!
//! let mut scheduler = ForumScheduler::new();
//! scheduler.start_round();
//!
//! while !scheduler.is_complete(&session) {
//!     scheduler.start_turn();
//!
//!     let action = scheduler.decide(&session);
//!     match action {
//!         ScheduleAction::WaitForSpeaker { speaker, deadline } => {
//!             // Get response from agent (via TypeScript/LLM)
//!             let response = get_agent_response(&speaker);
//!             session.record_turn(response)?;
//!             scheduler.record_response(&speaker);
//!         }
//!         ScheduleAction::Complete => break,
//!         _ => {}
//!     }
//! }
//! ```

pub mod scheduler;
pub mod session;

pub use scheduler::{ForumScheduler, ScheduleAction, SchedulerConfig, SchedulerStats, SkipTracker};
pub use session::{
    ForumSession, ForumTurn, SessionConfig, SessionError, SessionId, SessionStatus,
};
