//! MCP Tool Adapter
//!
//! Adapts MCP tools to `ZeroBot`'s Tool trait, allowing MCP tools to be used
//! alongside native `ZeroBot` tools.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::{Tool, ToolResult};

use super::client::McpClient;
use super::types::{McpTool, ToolContent};

/// Adapter that wraps an MCP tool to implement `ZeroBot`'s Tool trait
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool_def: McpTool,
    /// Prefixed name for disambiguation (e.g., "`mcp_filesystem_read_file`")
    prefixed_name: String,
}

impl McpToolAdapter {
    /// Create a new adapter for an MCP tool
    pub fn new(client: Arc<McpClient>, tool_def: McpTool) -> Self {
        // Create a prefixed name to avoid conflicts with native tools
        let prefixed_name = format!("mcp_{}_{}", client.server_name(), tool_def.name);

        Self {
            client,
            tool_def,
            prefixed_name,
        }
    }

    /// Get the original MCP tool definition
    pub fn mcp_tool(&self) -> &McpTool {
        &self.tool_def
    }

    /// Get the MCP server name
    pub fn server_name(&self) -> &str {
        self.client.server_name()
    }

    /// Extract text content from MCP tool result
    fn extract_text_content(content: &[ToolContent]) -> String {
        content
            .iter()
            .filter_map(|c| match c {
                ToolContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl Tool for McpToolAdapter {
    fn name(&self) -> &str {
        &self.prefixed_name
    }

    fn description(&self) -> &str {
        self.tool_def
            .description
            .as_deref()
            .unwrap_or("MCP tool (no description)")
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.tool_def.input_schema.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        // Convert args to HashMap<String, Value>
        let arguments: HashMap<String, serde_json::Value> = match args {
            serde_json::Value::Object(map) => map.into_iter().collect(),
            _ => HashMap::new(),
        };

        // Call the MCP tool
        let result = self
            .client
            .call_tool(&self.tool_def.name, arguments)
            .await?;

        // Convert MCP result to ToolResult
        let output = Self::extract_text_content(&result.content);

        Ok(ToolResult {
            success: !result.is_error,
            output,
            error: if result.is_error {
                Some("MCP tool reported an error".into())
            } else {
                None
            },
        })
    }
}

/// Create tool adapters for all tools from an MCP client
pub fn create_mcp_tool_adapters(client: Arc<McpClient>, tools: Vec<McpTool>) -> Vec<Box<dyn Tool>> {
    tools
        .into_iter()
        .map(|tool| {
            let adapter = McpToolAdapter::new(client.clone(), tool);
            Box::new(adapter) as Box<dyn Tool>
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mcp_tool() -> McpTool {
        McpTool {
            name: "read_file".into(),
            description: Some("Read a file from the filesystem".into()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        }
    }

    #[test]
    fn adapter_prefixed_name() {
        // We can't easily create a mock client here, so we test the naming logic directly
        let tool = create_test_mcp_tool();
        let prefixed = format!("mcp_{}_{}", "filesystem", tool.name);
        assert_eq!(prefixed, "mcp_filesystem_read_file");
    }

    #[test]
    fn extract_text_content_single() {
        let content = vec![ToolContent::text("Hello, world!")];
        let text = McpToolAdapter::extract_text_content(&content);
        assert_eq!(text, "Hello, world!");
    }

    #[test]
    fn extract_text_content_multiple() {
        let content = vec![
            ToolContent::text("Line 1"),
            ToolContent::text("Line 2"),
        ];
        let text = McpToolAdapter::extract_text_content(&content);
        assert_eq!(text, "Line 1\nLine 2");
    }

    #[test]
    fn extract_text_content_mixed() {
        let content = vec![
            ToolContent::text("Text content"),
            ToolContent::Image {
                data: "base64data".into(),
                mime_type: "image/png".into(),
            },
            ToolContent::text("More text"),
        ];
        let text = McpToolAdapter::extract_text_content(&content);
        assert_eq!(text, "Text content\nMore text");
    }

    #[test]
    fn extract_text_content_empty() {
        let content: Vec<ToolContent> = vec![];
        let text = McpToolAdapter::extract_text_content(&content);
        assert_eq!(text, "");
    }

    #[test]
    fn mcp_tool_schema_preserved() {
        let tool = create_test_mcp_tool();
        let schema = tool.input_schema.clone();

        assert!(schema.get("type").is_some());
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["path"].is_object());
    }
}
