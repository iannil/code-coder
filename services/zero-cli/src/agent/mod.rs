//! Agent system for ZeroBot.
//!
//! This module provides the agent loop and context management.
//! Core agent execution is imported from `zero-agent`.

pub mod loop_;

pub use loop_::run;

// Re-export agent types from zero-agent
