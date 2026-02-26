//! Session management for multi-turn conversation context.
//!
//! Provides per-chat_id session persistence with:
//! - Continuous conversation (default context preservation)
//! - `/new` command to reset session
//! - `/compact` command for manual context compression
//! - Auto-compression when context exceeds model limits

pub mod compactor;
pub mod store;
pub mod types;

