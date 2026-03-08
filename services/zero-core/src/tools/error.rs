//! Tool Error Classification and Recovery
//!
//! This module provides structured error classification for tool execution,
//! inspired by the Goose framework's error feedback loop pattern.
//!
//! # Design Principles
//!
//! 1. **Deterministic Classification**: Error types are categorized based on
//!    observable signals (exit codes, error patterns, timeouts)
//! 2. **Retryability Detection**: Each error type has a known retryability
//! 3. **Structured Context**: Errors carry rich context for LLM analysis
//!
//! # Error Categories
//!
//! - **Validation**: Input parameter errors (deterministic, not retryable)
//! - **Execution**: Tool runtime errors (may be retryable)
//! - **Permission**: Access denied errors (not retryable without escalation)
//! - **Timeout**: Time limit exceeded (often retryable with longer timeout)
//! - **Network**: Connection/network errors (usually retryable)
//! - **Resource**: Resource exhaustion (memory, disk, etc.)
//!
//! # Example
//!
//! ```rust,ignore
//! use zero_core::tools::error::{ToolError, ToolErrorType, ErrorClassifier};
//!
//! let classifier = ErrorClassifier::new();
//! let error = classifier.classify(exit_code, stderr);
//!
//! if error.retryable {
//!     // Implement exponential backoff
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Types of tool execution errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolErrorType {
    /// Input validation failed (invalid parameters)
    Validation,
    /// Runtime execution error
    Execution,
    /// Permission denied
    Permission,
    /// Operation timed out
    Timeout,
    /// Network/connection error
    Network,
    /// Resource exhausted (memory, disk, etc.)
    Resource,
    /// Unknown/uncategorized error
    Unknown,
}

impl ToolErrorType {
    /// Whether this error type is generally retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ToolErrorType::Timeout | ToolErrorType::Network | ToolErrorType::Resource
        )
    }

    /// Suggested retry delay for this error type
    pub fn suggested_delay(&self) -> Option<Duration> {
        match self {
            ToolErrorType::Timeout => Some(Duration::from_secs(5)),
            ToolErrorType::Network => Some(Duration::from_secs(2)),
            ToolErrorType::Resource => Some(Duration::from_secs(10)),
            _ => None,
        }
    }
}

/// Classified tool error with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedError {
    /// The error type
    pub error_type: ToolErrorType,
    /// Human-readable error message
    pub message: String,
    /// Original error output (stderr, etc.)
    pub raw_output: Option<String>,
    /// Exit code if available
    pub exit_code: Option<i32>,
    /// Whether this error is retryable
    pub retryable: bool,
    /// Suggested retry delay in milliseconds
    pub retry_delay_ms: Option<u64>,
    /// Specific field that caused the error (for validation errors)
    pub field: Option<String>,
    /// Specific reason for the error
    pub reason: Option<String>,
    /// Resource that was denied access (for permission errors)
    pub resource: Option<String>,
    /// Time elapsed before timeout (for timeout errors)
    pub elapsed_ms: Option<u64>,
    /// Configured time limit (for timeout errors)
    pub limit_ms: Option<u64>,
}

impl ClassifiedError {
    /// Create a validation error
    pub fn validation(field: impl Into<String>, reason: impl Into<String>) -> Self {
        let reason_str = reason.into();
        Self {
            error_type: ToolErrorType::Validation,
            message: format!("Validation failed: {}", &reason_str),
            raw_output: None,
            exit_code: None,
            retryable: false,
            retry_delay_ms: None,
            field: Some(field.into()),
            reason: Some(reason_str),
            resource: None,
            elapsed_ms: None,
            limit_ms: None,
        }
    }

    /// Create an execution error
    pub fn execution(exit_code: i32, stderr: impl Into<String>) -> Self {
        let stderr = stderr.into();
        let retryable = Self::is_execution_retryable(exit_code, &stderr);
        Self {
            error_type: ToolErrorType::Execution,
            message: format!("Execution failed with exit code {}", exit_code),
            raw_output: Some(stderr),
            exit_code: Some(exit_code),
            retryable,
            retry_delay_ms: if retryable { Some(1000) } else { None },
            field: None,
            reason: None,
            resource: None,
            elapsed_ms: None,
            limit_ms: None,
        }
    }

    /// Create a permission error
    pub fn permission(resource: impl Into<String>) -> Self {
        Self {
            error_type: ToolErrorType::Permission,
            message: "Permission denied".to_string(),
            raw_output: None,
            exit_code: None,
            retryable: false,
            retry_delay_ms: None,
            field: None,
            reason: Some("Access denied".to_string()),
            resource: Some(resource.into()),
            elapsed_ms: None,
            limit_ms: None,
        }
    }

    /// Create a timeout error
    pub fn timeout(elapsed_ms: u64, limit_ms: u64) -> Self {
        Self {
            error_type: ToolErrorType::Timeout,
            message: format!(
                "Operation timed out after {}ms (limit: {}ms)",
                elapsed_ms, limit_ms
            ),
            raw_output: None,
            exit_code: None,
            retryable: true,
            retry_delay_ms: Some(5000),
            field: None,
            reason: None,
            resource: None,
            elapsed_ms: Some(elapsed_ms),
            limit_ms: Some(limit_ms),
        }
    }

    /// Create a network error
    pub fn network(reason: impl Into<String>) -> Self {
        Self {
            error_type: ToolErrorType::Network,
            message: "Network error".to_string(),
            raw_output: None,
            exit_code: None,
            retryable: true,
            retry_delay_ms: Some(2000),
            field: None,
            reason: Some(reason.into()),
            resource: None,
            elapsed_ms: None,
            limit_ms: None,
        }
    }

    /// Create a resource exhaustion error
    pub fn resource(resource: impl Into<String>, reason: impl Into<String>) -> Self {
        let reason_str = reason.into();
        Self {
            error_type: ToolErrorType::Resource,
            message: format!("Resource exhausted: {}", &reason_str),
            raw_output: None,
            exit_code: None,
            retryable: true,
            retry_delay_ms: Some(10000),
            field: None,
            reason: Some(reason_str),
            resource: Some(resource.into()),
            elapsed_ms: None,
            limit_ms: None,
        }
    }

    /// Determine if an execution error is retryable based on exit code and output
    fn is_execution_retryable(exit_code: i32, stderr: &str) -> bool {
        // Common retryable patterns
        let retryable_patterns = [
            "connection refused",
            "connection reset",
            "connection timed out",
            "temporarily unavailable",
            "resource busy",
            "too many open files",
            "try again",
            "EAGAIN",
            "EBUSY",
            "ETIMEDOUT",
        ];

        // Exit code 137 = OOM killed (retryable with less memory usage)
        // Exit code 124 = timeout (retryable)
        if exit_code == 137 || exit_code == 124 {
            return true;
        }

        let stderr_lower = stderr.to_lowercase();
        retryable_patterns
            .iter()
            .any(|p| stderr_lower.contains(p))
    }
}

/// Error classifier that analyzes tool output
#[derive(Debug, Clone, Default)]
pub struct ErrorClassifier {
    /// Custom patterns for error classification
    permission_patterns: Vec<String>,
    network_patterns: Vec<String>,
    resource_patterns: Vec<String>,
}

impl ErrorClassifier {
    /// Create a new error classifier with default patterns
    pub fn new() -> Self {
        Self {
            permission_patterns: vec![
                "permission denied".to_string(),
                "access denied".to_string(),
                "not permitted".to_string(),
                "operation not allowed".to_string(),
                "EACCES".to_string(),
                "EPERM".to_string(),
            ],
            network_patterns: vec![
                "connection refused".to_string(),
                "connection reset".to_string(),
                "connection timed out".to_string(),
                "network unreachable".to_string(),
                "host not found".to_string(),
                "dns".to_string(),
                "ECONNREFUSED".to_string(),
                "ECONNRESET".to_string(),
                "ETIMEDOUT".to_string(),
                "ENETUNREACH".to_string(),
            ],
            resource_patterns: vec![
                "out of memory".to_string(),
                "no space left".to_string(),
                "disk quota".to_string(),
                "too many open files".to_string(),
                "resource temporarily unavailable".to_string(),
                "ENOMEM".to_string(),
                "ENOSPC".to_string(),
                "EMFILE".to_string(),
                "ENFILE".to_string(),
            ],
        }
    }

    /// Classify an error based on exit code and stderr output
    pub fn classify(&self, exit_code: i32, stderr: &str) -> ClassifiedError {
        let stderr_lower = stderr.to_lowercase();

        // Check for permission errors
        if self
            .permission_patterns
            .iter()
            .any(|p| stderr_lower.contains(p))
        {
            return ClassifiedError {
                error_type: ToolErrorType::Permission,
                message: "Permission denied".to_string(),
                raw_output: Some(stderr.to_string()),
                exit_code: Some(exit_code),
                retryable: false,
                retry_delay_ms: None,
                field: None,
                reason: Some(Self::extract_reason(&stderr)),
                resource: Self::extract_resource(&stderr),
                elapsed_ms: None,
                limit_ms: None,
            };
        }

        // Check for network errors
        if self
            .network_patterns
            .iter()
            .any(|p| stderr_lower.contains(p))
        {
            return ClassifiedError {
                error_type: ToolErrorType::Network,
                message: "Network error".to_string(),
                raw_output: Some(stderr.to_string()),
                exit_code: Some(exit_code),
                retryable: true,
                retry_delay_ms: Some(2000),
                field: None,
                reason: Some(Self::extract_reason(&stderr)),
                resource: None,
                elapsed_ms: None,
                limit_ms: None,
            };
        }

        // Check for resource errors
        if self
            .resource_patterns
            .iter()
            .any(|p| stderr_lower.contains(p))
        {
            return ClassifiedError {
                error_type: ToolErrorType::Resource,
                message: "Resource exhausted".to_string(),
                raw_output: Some(stderr.to_string()),
                exit_code: Some(exit_code),
                retryable: true,
                retry_delay_ms: Some(10000),
                field: None,
                reason: Some(Self::extract_reason(&stderr)),
                resource: Self::extract_resource(&stderr),
                elapsed_ms: None,
                limit_ms: None,
            };
        }

        // Check for timeout (exit code 124 is standard for timeout command)
        if exit_code == 124 {
            return ClassifiedError {
                error_type: ToolErrorType::Timeout,
                message: "Operation timed out".to_string(),
                raw_output: Some(stderr.to_string()),
                exit_code: Some(exit_code),
                retryable: true,
                retry_delay_ms: Some(5000),
                field: None,
                reason: None,
                resource: None,
                elapsed_ms: None,
                limit_ms: None,
            };
        }

        // Default: execution error
        ClassifiedError::execution(exit_code, stderr)
    }

    /// Extract the first meaningful line as the reason
    fn extract_reason(stderr: &str) -> String {
        stderr
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown error".to_string())
    }

    /// Try to extract a file/resource path from stderr
    fn extract_resource(stderr: &str) -> Option<String> {
        // Common patterns: "'filename': ...", "filename: ..."
        for line in stderr.lines() {
            // Look for quoted paths
            if let Some(start) = line.find('\'') {
                if let Some(end) = line[start + 1..].find('\'') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
            // Look for paths before colons
            if let Some(colon) = line.find(':') {
                let potential_path = line[..colon].trim();
                if potential_path.starts_with('/') || potential_path.starts_with('.') {
                    return Some(potential_path.to_string());
                }
            }
        }
        None
    }
}

/// Result of a tool execution with timing and classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionResult<T> {
    /// Whether the operation succeeded
    pub success: bool,
    /// The result data if successful
    pub result: Option<T>,
    /// Classified error if failed
    pub error: Option<ClassifiedError>,
    /// Execution duration in milliseconds
    pub execution_time_ms: u64,
    /// Whether this can be retried
    pub retryable: bool,
    /// Number of retries attempted
    pub retry_count: u32,
}

impl<T> ToolExecutionResult<T> {
    /// Create a successful result
    pub fn ok(result: T, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
            execution_time_ms,
            retryable: false,
            retry_count: 0,
        }
    }

    /// Create a failed result
    pub fn err(error: ClassifiedError, execution_time_ms: u64) -> Self {
        let retryable = error.retryable;
        Self {
            success: false,
            result: None,
            error: Some(error),
            execution_time_ms,
            retryable,
            retry_count: 0,
        }
    }

    /// Mark as a retry attempt
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_retryable() {
        assert!(!ToolErrorType::Validation.is_retryable());
        assert!(!ToolErrorType::Execution.is_retryable());
        assert!(!ToolErrorType::Permission.is_retryable());
        assert!(ToolErrorType::Timeout.is_retryable());
        assert!(ToolErrorType::Network.is_retryable());
        assert!(ToolErrorType::Resource.is_retryable());
    }

    #[test]
    fn test_validation_error() {
        let err = ClassifiedError::validation("path", "File not found");
        assert_eq!(err.error_type, ToolErrorType::Validation);
        assert!(!err.retryable);
        assert_eq!(err.field, Some("path".to_string()));
    }

    #[test]
    fn test_timeout_error() {
        let err = ClassifiedError::timeout(5000, 3000);
        assert_eq!(err.error_type, ToolErrorType::Timeout);
        assert!(err.retryable);
        assert_eq!(err.elapsed_ms, Some(5000));
        assert_eq!(err.limit_ms, Some(3000));
    }

    #[test]
    fn test_classifier_permission_error() {
        let classifier = ErrorClassifier::new();
        let err = classifier.classify(1, "cat: /etc/shadow: Permission denied");
        assert_eq!(err.error_type, ToolErrorType::Permission);
        assert!(!err.retryable);
    }

    #[test]
    fn test_classifier_network_error() {
        let classifier = ErrorClassifier::new();
        let err = classifier.classify(1, "curl: (7) Connection refused");
        assert_eq!(err.error_type, ToolErrorType::Network);
        assert!(err.retryable);
    }

    #[test]
    fn test_classifier_resource_error() {
        let classifier = ErrorClassifier::new();
        let err = classifier.classify(1, "write: No space left on device");
        assert_eq!(err.error_type, ToolErrorType::Resource);
        assert!(err.retryable);
    }

    #[test]
    fn test_classifier_timeout() {
        let classifier = ErrorClassifier::new();
        let err = classifier.classify(124, "");
        assert_eq!(err.error_type, ToolErrorType::Timeout);
        assert!(err.retryable);
    }

    #[test]
    fn test_classifier_execution_error() {
        let classifier = ErrorClassifier::new();
        let err = classifier.classify(1, "command not found: foo");
        assert_eq!(err.error_type, ToolErrorType::Execution);
        assert!(!err.retryable);
    }

    #[test]
    fn test_extract_resource() {
        let resource = ErrorClassifier::extract_resource("cat: '/etc/shadow': Permission denied");
        assert_eq!(resource, Some("/etc/shadow".to_string()));

        let resource2 = ErrorClassifier::extract_resource("/path/to/file: No such file");
        assert_eq!(resource2, Some("/path/to/file".to_string()));
    }

    #[test]
    fn test_execution_result_ok() {
        let result: ToolExecutionResult<i32> = ToolExecutionResult::ok(42, 100);
        assert!(result.success);
        assert_eq!(result.result, Some(42));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_execution_result_err() {
        let error = ClassifiedError::timeout(5000, 3000);
        let result: ToolExecutionResult<i32> = ToolExecutionResult::err(error, 5000);
        assert!(!result.success);
        assert!(result.result.is_none());
        assert!(result.retryable);
    }
}
