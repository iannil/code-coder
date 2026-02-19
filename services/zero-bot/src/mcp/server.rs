//! MCP Server
//!
//! Server implementation for exposing `ZeroBot` tools via MCP protocol.
//! Supports both stdio and HTTP transport modes.

use anyhow::Result;
use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::tools::Tool;

use super::types::{
    methods, CallToolParams, CallToolResult, InitializeParams, InitializeResult, JsonRpcError,
    JsonRpcRequest, JsonRpcResponse, ListToolsResult, McpTool, ServerCapabilities, ServerInfo,
    ToolContent, ToolsCapability, MCP_PROTOCOL_VERSION,
};

/// MCP Server for exposing `ZeroBot` tools
pub struct McpServer {
    tools: Vec<Arc<dyn Tool>>,
    api_key: Option<String>,
}

impl McpServer {
    /// Create a new MCP server with the given tools
    pub fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
        Self {
            tools,
            api_key: None,
        }
    }

    /// Set an API key for authentication
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Get MCP tools from `ZeroBot` tools
    fn mcp_tools(&self) -> Vec<McpTool> {
        self.tools
            .iter()
            .map(|tool| McpTool {
                name: tool.name().to_string(),
                description: Some(tool.description().to_string()),
                input_schema: tool.parameters_schema(),
            })
            .collect()
    }

    /// Handle a JSON-RPC request
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        match request.method.as_str() {
            methods::INITIALIZE => self.handle_initialize(request).await,
            methods::INITIALIZED => {
                // Notification, no response needed
                JsonRpcResponse::success(id.unwrap_or(0i64.into()), serde_json::Value::Null)
            }
            methods::PING => JsonRpcResponse::success(id.unwrap_or(0i64.into()), serde_json::json!({})),
            methods::TOOLS_LIST => self.handle_tools_list(request).await,
            methods::TOOLS_CALL => self.handle_tools_call(request).await,
            _ => JsonRpcResponse::error(id, JsonRpcError::method_not_found(&request.method)),
        }
    }

    /// Handle initialize request
    #[allow(clippy::unused_async)] // Kept async for handler interface consistency
    async fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        // Parse params (optional, for validation)
        if let Some(params) = request.params {
            if let Err(e) = serde_json::from_value::<InitializeParams>(params) {
                tracing::warn!("MCP initialize params parse error: {e}");
            }
        }

        let result = InitializeResult {
            protocol_version: MCP_PROTOCOL_VERSION.into(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: false }),
                resources: None,
                prompts: None,
            },
            server_info: ServerInfo {
                name: "zero-bot".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        };

        match serde_json::to_value(&result) {
            Ok(value) => JsonRpcResponse::success(id.unwrap_or(0i64.into()), value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize response: {e}")),
            ),
        }
    }

    /// Handle tools/list request
    #[allow(clippy::unused_async)] // Kept async for handler interface consistency
    async fn handle_tools_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        let result = ListToolsResult {
            tools: self.mcp_tools(),
        };

        match serde_json::to_value(&result) {
            Ok(value) => JsonRpcResponse::success(id.unwrap_or(0i64.into()), value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize tools: {e}")),
            ),
        }
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();

        // Parse params
        let params: CallToolParams = match request.params {
            Some(p) => match serde_json::from_value(p) {
                Ok(params) => params,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        JsonRpcError::invalid_params(format!("Invalid tool call params: {e}")),
                    );
                }
            },
            None => {
                return JsonRpcResponse::error(
                    id,
                    JsonRpcError::invalid_params("Missing tool call params"),
                );
            }
        };

        // Find the tool
        let tool = if let Some(t) = self.tools.iter().find(|t| t.name() == params.name) { t } else {
            let result = CallToolResult {
                content: vec![ToolContent::text(format!("Tool not found: {}", params.name))],
                is_error: true,
            };
            return match serde_json::to_value(&result) {
                Ok(value) => JsonRpcResponse::success(id.unwrap_or(0i64.into()), value),
                Err(e) => JsonRpcResponse::error(
                    id,
                    JsonRpcError::internal_error(format!("Serialization error: {e}")),
                ),
            };
        };

        // Convert arguments to serde_json::Value
        let args = serde_json::to_value(&params.arguments).unwrap_or(serde_json::Value::Object(
            serde_json::Map::new(),
        ));

        // Execute the tool
        let result = match tool.execute(args).await {
            Ok(tool_result) => CallToolResult {
                content: vec![ToolContent::text(&tool_result.output)],
                is_error: !tool_result.success,
            },
            Err(e) => CallToolResult {
                content: vec![ToolContent::text(format!("Tool execution failed: {e}"))],
                is_error: true,
            },
        };

        match serde_json::to_value(&result) {
            Ok(value) => JsonRpcResponse::success(id.unwrap_or(0i64.into()), value),
            Err(e) => JsonRpcResponse::error(
                id,
                JsonRpcError::internal_error(format!("Failed to serialize result: {e}")),
            ),
        }
    }

    /// Run the MCP server in stdio mode
    pub async fn serve_stdio(&self) -> Result<()> {
        tracing::info!("Starting MCP server in stdio mode");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).await?;

            if bytes == 0 {
                tracing::info!("MCP client closed connection");
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue;
            }

            // Parse request
            let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
                Ok(req) => req,
                Err(e) => {
                    let error_response = JsonRpcResponse::error(
                        None,
                        JsonRpcError::parse_error(format!("Invalid JSON: {e}")),
                    );
                    let response_json = serde_json::to_string(&error_response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            // Skip notifications (no id)
            if request.id.is_none() {
                tracing::debug!("Received notification: {}", request.method);
                continue;
            }

            // Handle request
            let response = self.handle_request(request).await;
            let response_json = serde_json::to_string(&response)?;

            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    /// Create axum routes for HTTP MCP server
    pub fn routes(self: Arc<Self>) -> Router {
        Router::new()
            .route("/", post(handle_mcp_request))
            .with_state(self)
    }
}

/// Shared state for MCP HTTP server
type McpState = Arc<McpServer>;

/// Handle MCP HTTP request
async fn handle_mcp_request(
    State(server): State<McpState>,
    body: Bytes,
) -> impl IntoResponse {
    // Parse request
    let request: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            let error_response = JsonRpcResponse::error(
                None,
                JsonRpcError::parse_error(format!("Invalid JSON: {e}")),
            );
            return (StatusCode::OK, Json(error_response));
        }
    };

    // Handle request
    let response = server.handle_request(request).await;
    (StatusCode::OK, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolResult;
    use async_trait::async_trait;

    // Mock tool for testing
    struct MockTool {
        name: String,
        description: String,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult {
                success: true,
                output: "Mock output".into(),
                error: None,
            })
        }
    }

    fn create_test_server() -> McpServer {
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MockTool {
            name: "test_tool".into(),
            description: "A test tool".into(),
        })];
        McpServer::new(tools)
    }

    #[tokio::test]
    async fn server_handles_initialize() {
        let server = create_test_server();
        let request = JsonRpcRequest::new(1i64, methods::INITIALIZE)
            .with_params(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }));

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());

        let result: InitializeResult =
            serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.server_info.name, "zero-bot");
    }

    #[tokio::test]
    async fn server_handles_tools_list() {
        let server = create_test_server();
        let request = JsonRpcRequest::new(1i64, methods::TOOLS_LIST);

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());

        let result: ListToolsResult =
            serde_json::from_value(response.result.unwrap()).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn server_handles_tools_call() {
        let server = create_test_server();
        let request = JsonRpcRequest::new(1i64, methods::TOOLS_CALL)
            .with_params(serde_json::json!({
                "name": "test_tool",
                "arguments": {"input": "test"}
            }));

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());

        let result: CallToolResult =
            serde_json::from_value(response.result.unwrap()).unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
    }

    #[tokio::test]
    async fn server_handles_unknown_method() {
        let server = create_test_server();
        let request = JsonRpcRequest::new(1i64, "unknown/method");

        let response = server.handle_request(request).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn server_handles_tool_not_found() {
        let server = create_test_server();
        let request = JsonRpcRequest::new(1i64, methods::TOOLS_CALL)
            .with_params(serde_json::json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }));

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());

        let result: CallToolResult =
            serde_json::from_value(response.result.unwrap()).unwrap();
        assert!(result.is_error);
    }

    #[test]
    fn server_mcp_tools_conversion() {
        let server = create_test_server();
        let mcp_tools = server.mcp_tools();

        assert_eq!(mcp_tools.len(), 1);
        assert_eq!(mcp_tools[0].name, "test_tool");
        assert_eq!(mcp_tools[0].description, Some("A test tool".into()));
    }
}
