//! Common NAPI types and conventions
//!
//! This module provides documentation and helper utilities for NAPI bindings.
//! New bindings should follow these conventions for consistency.
//!
//! # Field Naming Conventions
//!
//! | Concept | Field Name | Type | Notes |
//! |---------|-----------|------|-------|
//! | Success indicator | `success` | `bool` | |
//! | Error message | `error` | `Option<String>` | |
//! | File path | `path` or `file_path` | `String` | Use `path` for single, `file_path` when ambiguous |
//! | Content | `content` | `String` | Raw text content |
//! | Line count | `lines` or `total_lines` | `u32` | |
//! | Byte count | `bytes` or `size` | `u32` or `f64` | Use `f64` for large files |
//! | Truncation | `truncated` | `bool` | |
//! | Timing | `duration_ms` | `u32` | Milliseconds |
//! | Counts | `*_count` or `total_*` | `u32` | e.g., `match_count`, `total_files` |
//! | Items returned | `returned` or `*_returned` | `u32` | |
//! | Pagination | `offset`, `limit` | `u32` | |
//!
//! # Result Type Patterns
//!
//! ## Simple Operations
//! ```rust,ignore
//! #[napi(object)]
//! pub struct MyOperationResult {
//!     pub success: bool,
//!     pub data: Option<MyData>,  // or just the data directly
//!     pub error: Option<String>,
//! }
//! ```
//!
//! ## Batch Operations
//! ```rust,ignore
//! #[napi(object)]
//! pub struct MyBatchResult {
//!     pub results: Vec<MyItem>,
//!     pub total: u32,
//!     pub returned: u32,
//!     pub truncated: bool,
//!     pub offset: u32,
//! }
//! ```
//!
//! ## File Operations
//! ```rust,ignore
//! #[napi(object)]
//! pub struct MyFileResult {
//!     pub content: String,
//!     pub path: String,
//!     pub size: f64,            // bytes as f64 for large files
//!     pub truncated: bool,
//!     pub encoding: Option<String>,
//! }
//! ```

/// Helper to create a success message for NAPI Result types
pub fn success_message(operation: &str) -> String {
    format!("{} completed successfully", operation)
}

/// Helper to create an error message for NAPI Result types
pub fn error_message(operation: &str, error: &str) -> String {
    format!("{} failed: {}", operation, error)
}

/// Calculate whether results are truncated
pub fn is_truncated(returned: usize, total: usize) -> bool {
    returned < total
}

/// Calculate pagination metadata
pub fn pagination_info(
    total: usize,
    offset: usize,
    limit: usize,
) -> (u32, u32, bool) {
    let returned = (total.saturating_sub(offset)).min(limit);
    let truncated = offset + returned < total;
    (total as u32, returned as u32, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_message() {
        assert_eq!(
            success_message("File read"),
            "File read completed successfully"
        );
    }

    #[test]
    fn test_error_message() {
        assert_eq!(
            error_message("File read", "not found"),
            "File read failed: not found"
        );
    }

    #[test]
    fn test_is_truncated() {
        assert!(is_truncated(10, 100));
        assert!(!is_truncated(100, 100));
        assert!(!is_truncated(100, 50));
    }

    #[test]
    fn test_pagination_info() {
        // Normal case: 100 total, offset 0, limit 20
        let (total, returned, truncated) = pagination_info(100, 0, 20);
        assert_eq!(total, 100);
        assert_eq!(returned, 20);
        assert!(truncated);

        // Last page: 100 total, offset 90, limit 20
        let (total, returned, truncated) = pagination_info(100, 90, 20);
        assert_eq!(total, 100);
        assert_eq!(returned, 10);
        assert!(!truncated);

        // All results: 10 total, offset 0, limit 20
        let (total, returned, truncated) = pagination_info(10, 0, 20);
        assert_eq!(total, 10);
        assert_eq!(returned, 10);
        assert!(!truncated);
    }
}
