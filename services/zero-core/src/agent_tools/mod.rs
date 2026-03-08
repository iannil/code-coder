//! Zero Tools - AI agent capability system.
//!
//! Provides a trait-based tool system for AI agents to interact with the world:
//! - Shell command execution (sandboxed)
//! - File read/write (path-restricted)
//! - Grep - content search with regex (high-performance ripgrep-based)
//! - Glob - pattern-based file matching
//! - Edit - file editing with fuzzy matching
//! - Memory store/recall/forget (via zero-memory)
//! - CodeCoder integration (SSE-based)
//! - Browser automation (optional)

pub mod browser;
pub mod codecoder;
pub mod edit;
pub mod file_read;
pub mod file_write;
pub mod glob;
pub mod grep;
pub mod memory_forget;
pub mod memory_recall;
pub mod memory_store;
pub mod security;
pub mod shell;
pub mod traits;

pub use traits::{Tool, ToolResult, ToolSpec};

// Re-export security types
pub use security::SecurityPolicy;

// Re-export tool implementations
pub use browser::BrowserTool;
pub use codecoder::CodeCoderTool;
pub use edit::EditTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use memory_forget::MemoryForgetTool;
pub use memory_recall::MemoryRecallTool;
pub use memory_store::MemoryStoreTool;
pub use shell::ShellTool;
