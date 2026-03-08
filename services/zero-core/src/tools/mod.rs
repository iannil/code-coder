//! Tools module - high-performance file operations and shell execution
//!
//! This module provides:
//! - **grep**: Content search with regex support (using grep-regex crate)
//! - **glob**: Pattern-based file matching (using ignore crate)
//! - **read**: File reading with mmap support for large files
//! - **write**: Atomic file writing with backup support
//! - **edit**: High-performance diff and patch operations (using similar crate)
//! - **shell**: PTY-based shell execution with sandbox support
//! - **ls**: Directory listing with ignore patterns
//! - **truncation**: Output truncation for large results
//! - **todo**: Task list management
//! - **multiedit**: Batch file editing operations
//! - **apply_patch**: Unified diff patch application
//! - **codesearch**: Semantic code search with context
//! - **webfetch**: HTTP request handling
//! - **error**: Error classification and recovery (Goose-inspired)

// Core file operations
pub mod edit;
pub mod error;
pub mod glob;
pub mod grep;
pub mod read;
pub mod shell;
pub mod shell_parser;
pub mod shell_pty;
pub mod write;

// Extended tools
pub mod apply_patch;
pub mod codesearch;
pub mod ls;
pub mod multiedit;
pub mod todo;
pub mod truncation;
pub mod webfetch;

// Common types used across tools
use serde::{Deserialize, Serialize};

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult<T> {
    /// Whether the operation succeeded
    pub success: bool,
    /// The result data if successful
    pub data: Option<T>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

impl<T> ToolResult<T> {
    /// Create a successful result
    pub fn ok(data: T, duration_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            duration_ms,
        }
    }

    /// Create a failed result
    pub fn err(error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            duration_ms,
        }
    }
}

/// File information returned by various tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Absolute path to the file
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Whether this is a symlink
    pub is_symlink: bool,
    /// Last modification time (Unix timestamp)
    pub modified: Option<i64>,
    /// File extension (without the dot)
    pub extension: Option<String>,
}

impl FileInfo {
    /// Create FileInfo from a path
    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        let metadata = path.metadata()?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        Ok(Self {
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            is_symlink: path.is_symlink(),
            modified,
            extension: path.extension().map(|e| e.to_string_lossy().to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_ok() {
        let result: ToolResult<i32> = ToolResult::ok(42, 100);
        assert!(result.success);
        assert_eq!(result.data, Some(42));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_tool_result_err() {
        let result: ToolResult<i32> = ToolResult::err("Something went wrong", 50);
        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }
}
