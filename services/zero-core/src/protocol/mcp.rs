//! Model Context Protocol (MCP) implementation
//!
//! This module provides MCP client and server implementations compatible
//! with the MCP specification.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

impl McpTool {
    /// Create a new MCP tool
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    /// Set the input schema
    pub fn with_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    pub description: Option<String>,
    /// MIME type
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

/// MCP Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Content array
    pub content: Vec<McpContent>,
    /// Whether this is an error
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

impl McpToolResult {
    /// Create a text result
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent::Text { text: text.into() }],
            is_error: false,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent::Text { text: message.into() }],
            is_error: true,
        }
    }
}

/// MCP Content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    /// Text content
    Text { text: String },
    /// Image content
    Image { data: String, #[serde(rename = "mimeType")] mime_type: String },
    /// Resource content
    Resource { uri: String, #[serde(rename = "mimeType")] mime_type: Option<String>, text: Option<String> },
}

/// MCP Client for connecting to MCP servers
pub struct McpClient {
    /// Server name
    name: String,
    /// Available tools
    tools: Vec<McpTool>,
    /// Available resources
    resources: Vec<McpResource>,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tools: Vec::new(),
            resources: Vec::new(),
        }
    }

    /// Get the server name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get available tools
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// Get available resources
    pub fn resources(&self) -> &[McpResource] {
        &self.resources
    }

    /// Set tools (for testing or manual setup)
    pub fn set_tools(&mut self, tools: Vec<McpTool>) {
        self.tools = tools;
    }

    /// Set resources (for testing or manual setup)
    pub fn set_resources(&mut self, resources: Vec<McpResource>) {
        self.resources = resources;
    }
}

/// MCP Server for exposing tools and resources
pub struct McpServer {
    /// Server name
    name: String,
    /// Server version
    version: String,
    /// Registered tools
    tools: HashMap<String, McpTool>,
    /// Tool handlers
    handlers: HashMap<String, Box<dyn Fn(Value) -> McpToolResult + Send + Sync>>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            tools: HashMap::new(),
            handlers: HashMap::new(),
        }
    }

    /// Get the server name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the server version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Register a tool with its handler
    pub fn register_tool<F>(&mut self, tool: McpTool, handler: F)
    where
        F: Fn(Value) -> McpToolResult + Send + Sync + 'static,
    {
        let name = tool.name.clone();
        self.tools.insert(name.clone(), tool);
        self.handlers.insert(name, Box::new(handler));
    }

    /// Get all registered tools
    pub fn tools(&self) -> Vec<&McpTool> {
        self.tools.values().collect()
    }

    /// Call a tool
    pub fn call_tool(&self, name: &str, arguments: Value) -> McpToolResult {
        match self.handlers.get(name) {
            Some(handler) => handler(arguments),
            None => McpToolResult::error(format!("Unknown tool: {}", name)),
        }
    }

    /// Generate the tools/list response
    pub fn list_tools(&self) -> Value {
        serde_json::json!({
            "tools": self.tools.values().collect::<Vec<_>>()
        })
    }

    /// Generate the server info response
    pub fn server_info(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "version": self.version,
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": true },
                "resources": { "subscribe": true, "listChanged": true }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool() {
        let tool = McpTool::new("echo", "Echoes input back")
            .with_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            }));

        assert_eq!(tool.name, "echo");
        assert_eq!(tool.description, "Echoes input back");
    }

    #[test]
    fn test_mcp_server() {
        let mut server = McpServer::new("test-server", "1.0.0");

        let tool = McpTool::new("echo", "Echoes input back");
        server.register_tool(tool, |args| {
            let message = args.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("no message");
            McpToolResult::text(message)
        });

        assert_eq!(server.tools().len(), 1);

        let result = server.call_tool("echo", serde_json::json!({ "message": "hello" }));
        assert!(!result.is_error);
    }

    #[test]
    fn test_mcp_tool_result() {
        let result = McpToolResult::text("Hello, world!");
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);

        let error = McpToolResult::error("Something went wrong");
        assert!(error.is_error);
    }
}
