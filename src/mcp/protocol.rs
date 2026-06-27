/// ─── MCP Protocol Types ───────────────────────────────────────────────────
///
/// Model Context Protocol (MCP) — JSON-RPC 2.0 based protocol for
/// tool/service discovery and invocation.
///
/// Spec: https://spec.modelcontextprotocol.io/

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─── JSON-RPC 2.0 ──────────────────────────────────────────────────────────

/// A JSON-RPC request.
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A JSON-RPC response.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Success {
        jsonrpc: String,
        id: u64,
        result: Value,
    },
    Error {
        jsonrpc: String,
        id: u64,
        error: JsonRpcError,
    },
}

/// A JSON-RPC error object.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        }
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ─── MCP Initialize ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientCapabilities {
    pub tools: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Response to initialize
#[derive(Debug, Clone, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

// ─── MCP Tools ─────────────────────────────────────────────────────────────

/// Request to list tools.
#[derive(Debug, Clone, Serialize)]
pub struct ListToolsParams {}

/// Response to tools/list.
#[derive(Debug, Clone, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<ToolDescription>,
}

/// A tool as described by an MCP server.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
}

/// Request to call a tool.
#[derive(Debug, Clone, Serialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: Value,
}

/// Response from tools/call.
#[derive(Debug, Clone, Deserialize)]
pub struct CallToolResult {
    #[serde(default)]
    pub content: Vec<ToolContent>,
    #[serde(default)]
    pub is_error: bool,
}

/// Content item from a tool response.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "resource")]
    Resource { resource: Value },
}

impl CallToolResult {
    /// Extract text content from tool result.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for item in &self.content {
            match item {
                ToolContent::Text { text } => out.push_str(text),
                ToolContent::Resource { resource } => {
                    if let Some(text) = resource.get("text").and_then(|v| v.as_str()) {
                        out.push_str(text);
                    }
                }
            }
        }
        out
    }
}
