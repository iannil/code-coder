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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── JsonRpcRequest ───────────────────────────────────────────────────

    #[test]
    fn test_json_rpc_request_new() {
        let req = JsonRpcRequest::new(1, "initialize", None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "initialize");
        assert!(req.params.is_none());
    }

    #[test]
    fn test_json_rpc_request_with_params() {
        let params = serde_json::json!({"key": "value"});
        let req = JsonRpcRequest::new(42, "tools/call", Some(params));
        assert_eq!(req.id, 42);
        assert!(req.params.is_some());
    }

    #[test]
    fn test_json_rpc_request_serialize() {
        let req = JsonRpcRequest::new(1, "ping", None);
        let json = req.serialize();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["method"], "ping");
    }

    #[test]
    fn test_json_rpc_request_serialize_with_params() {
        let params = serde_json::json!({"name": "test"});
        let req = JsonRpcRequest::new(2, "tools/call", Some(params));
        let json = req.serialize();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["id"], 2);
        assert_eq!(parsed["params"]["name"], "test");
    }

    // ─── JsonRpcResponse ──────────────────────────────────────────────────

    #[test]
    fn test_json_rpc_response_success_deserialize() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(raw).unwrap();
        match resp {
            JsonRpcResponse::Success { id, result, .. } => {
                assert_eq!(id, 1);
                assert_eq!(result["ok"], true);
            }
            _ => panic!("Expected Success variant"),
        }
    }

    #[test]
    fn test_json_rpc_response_error_deserialize() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(raw).unwrap();
        match resp {
            JsonRpcResponse::Error { id, error, .. } => {
                assert_eq!(id, 1);
                assert_eq!(error.code, -32601);
                assert_eq!(error.message, "Method not found");
                assert!(error.data.is_none());
            }
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn test_json_rpc_error_with_data() {
        let raw = r#"{"code":-32000,"message":"Bad request","data":{"detail":"invalid param"}}"#;
        let err: JsonRpcError = serde_json::from_str(raw).unwrap();
        assert_eq!(err.code, -32000);
        assert_eq!(err.message, "Bad request");
        assert!(err.data.is_some());
        assert_eq!(err.data.unwrap()["detail"], "invalid param");
    }

    // ─── MCP Initialize ───────────────────────────────────────────────────

    #[test]
    fn test_initialize_params_serialize() {
        let params = InitializeParams {
            protocol_version: "2024-11-05".into(),
            capabilities: ClientCapabilities { tools: None },
            client_info: ClientInfo {
                name: "codecoder".into(),
                version: "0.1.0".into(),
            },
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["protocol_version"], "2024-11-05");
        assert_eq!(json["client_info"]["name"], "codecoder");
    }

    #[test]
    fn test_initialize_result_deserialize() {
        let raw = r#"{
            "protocol_version": "2024-11-05",
            "capabilities": {"tools": {}},
            "server_info": {"name": "test-server", "version": "1.0"}
        }"#;
        let result: InitializeResult = serde_json::from_str(raw).unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert_eq!(result.server_info.name, "test-server");
        assert!(result.capabilities.tools.is_some());
    }

    // ─── ListToolsResult ──────────────────────────────────────────────────

    #[test]
    fn test_list_tools_result_deserialize() {
        let raw = r#"{"tools":[{"name":"echo","description":"Echo back input","input_schema":{"type":"object"}}]}"#;
        let result: ListToolsResult = serde_json::from_str(raw).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "echo");
        assert_eq!(result.tools[0].description, "Echo back input");
    }

    // ─── CallToolResult ───────────────────────────────────────────────────

    #[test]
    fn test_call_tool_result_text_text_content() {
        let raw = r#"{"content":[{"type":"text","text":"Hello world"}],"is_error":false}"#;
        let result: CallToolResult = serde_json::from_str(raw).unwrap();
        assert!(!result.is_error);
        assert_eq!(result.text(), "Hello world");
    }

    #[test]
    fn test_call_tool_result_text_resource_content() {
        let raw = r#"{"content":[{"type":"resource","resource":{"text":"from resource"}}]}"#;
        let result: CallToolResult = serde_json::from_str(raw).unwrap();
        assert_eq!(result.text(), "from resource");
    }

    #[test]
    fn test_call_tool_result_text_mixed_content() {
        let raw = r#"{"content":[
            {"type":"text","text":"Hello "},
            {"type":"resource","resource":{"text":"world"}},
            {"type":"text","text":"!"}
        ]}"#;
        let result: CallToolResult = serde_json::from_str(raw).unwrap();
        assert_eq!(result.text(), "Hello world!");
    }

    #[test]
    fn test_call_tool_result_text_empty() {
        let raw = r#"{"content":[],"is_error":true}"#;
        let result: CallToolResult = serde_json::from_str(raw).unwrap();
        assert!(result.is_error);
        assert_eq!(result.text(), "");
    }

    // ─── ToolContent ──────────────────────────────────────────────────────

    #[test]
    fn test_tool_content_text_deserialize() {
        let raw = r#"{"type":"text","text":"output"}"#;
        let content: ToolContent = serde_json::from_str(raw).unwrap();
        match content {
            ToolContent::Text { text } => assert_eq!(text, "output"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_tool_content_resource_deserialize() {
        let raw = r#"{"type":"resource","resource":{"uri":"file:///test"}}"#;
        let content: ToolContent = serde_json::from_str(raw).unwrap();
        match content {
            ToolContent::Resource { resource } => {
                assert_eq!(resource["uri"], "file:///test");
            }
            _ => panic!("Expected Resource variant"),
        }
    }
}
