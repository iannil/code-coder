//! Session module - message storage, compaction, and prompt templates
//!
//! This module provides:
//! - **message**: Message storage and retrieval
//! - **compaction**: Context compaction and summarization
//! - **prompt**: Prompt template rendering
//! - **store**: Session persistence

pub mod compaction;
pub mod message;
pub mod prompt;
pub mod store;

// Re-export main types
pub use compaction::{CompactionResult, CompactionStrategy, Compactor};
pub use message::{Message, MessageRole, MessageStore};
pub use prompt::{PromptContext, PromptTemplate, TemplateEngine};
pub use store::{SessionStore, SessionData};
