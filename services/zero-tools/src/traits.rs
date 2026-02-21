//! Core Tool trait and types.
//!
//! All tools implement the `Tool` trait, providing a uniform interface
//! for the agent executor to discover and invoke capabilities.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Result from executing a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool succeeded.
    pub success: bool,
    /// Tool output (stdout, result text, etc.).
    pub output: String,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result.
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
        }
    }

    /// Create a failed result with partial output.
    pub fn failure_with_output(output: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: output.into(),
            error: Some(error.into()),
        }
    }
}

/// Tool specification for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Tool name (must match `name()` method).
    pub name: String,
    /// Human-readable description for the LLM.
    pub description: String,
    /// JSON Schema for the tool's parameters.
    pub parameters: serde_json::Value,
}

/// Trait for agent tools.
///
/// Each tool provides:
/// - `name()`: unique identifier
/// - `description()`: what the tool does (shown to LLM)
/// - `parameters_schema()`: JSON Schema for arguments
/// - `execute()`: async function to run the tool
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name.
    fn name(&self) -> &str;

    /// Description shown to the LLM.
    fn description(&self) -> &str;

    /// JSON Schema for parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with given arguments.
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult>;

    /// Generate a ToolSpec for function calling.
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_success() {
        let result = ToolResult::success("done");
        assert!(result.success);
        assert_eq!(result.output, "done");
        assert!(result.error.is_none());
    }

    #[test]
    fn tool_result_failure() {
        let result = ToolResult::failure("something went wrong");
        assert!(!result.success);
        assert!(result.output.is_empty());
        assert_eq!(result.error.as_deref(), Some("something went wrong"));
    }

    #[test]
    fn tool_result_failure_with_output() {
        let result = ToolResult::failure_with_output("partial output", "but failed");
        assert!(!result.success);
        assert_eq!(result.output, "partial output");
        assert_eq!(result.error.as_deref(), Some("but failed"));
    }

    #[test]
    fn tool_result_serializes() {
        let result = ToolResult::success("output");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"output\":\"output\""));
    }

    #[test]
    fn tool_spec_serializes() {
        let spec = ToolSpec {
            name: "test".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&spec).unwrap();
        assert!(json.contains("\"name\":\"test\""));
    }
}
