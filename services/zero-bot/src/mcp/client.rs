//! MCP Client
//!
//! Client for connecting to external MCP servers and using their tools.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::transport::{HttpTransport, StdioTransport, Transport};
use super::types::{
    methods, CallToolParams, CallToolResult, ClientCapabilities, ClientInfo, InitializeParams,
    InitializeResult, JsonRpcId, JsonRpcRequest, ListToolsResult, McpTool, ToolContent,
    MCP_PROTOCOL_VERSION,
};

/// MCP client for connecting to external MCP servers
pub struct McpClient {
    transport: Arc<dyn Transport>,
    server_name: String,
    tools: RwLock<Vec<McpTool>>,
    initialized: RwLock<bool>,
    next_id: AtomicI64,
}

impl McpClient {
    /// Create a new MCP client with the given transport
    pub fn new(transport: impl Transport + 'static, server_name: impl Into<String>) -> Self {
        Self {
            transport: Arc::new(transport),
            server_name: server_name.into(),
            tools: RwLock::new(Vec::new()),
            initialized: RwLock::new(false),
            next_id: AtomicI64::new(1),
        }
    }

    /// Connect to a local MCP server via stdio
    pub async fn connect_local(
        server_name: impl Into<String>,
        command: &[String],
        environment: Option<&HashMap<String, String>>,
    ) -> Result<Self> {
        let name = server_name.into();
        tracing::info!("Connecting to local MCP server: {name}");

        let transport = StdioTransport::spawn(command, environment).await?;
        let client = Self::new(transport, &name);
        client.initialize().await?;

        Ok(client)
    }

    /// Connect to a remote MCP server via HTTP
    pub async fn connect_remote(
        server_name: impl Into<String>,
        url: impl Into<String>,
        headers: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        let name = server_name.into();
        tracing::info!("Connecting to remote MCP server: {name}");

        let transport = HttpTransport::new(url, headers);
        let client = Self::new(transport, &name);
        client.initialize().await?;

        Ok(client)
    }

    /// Get the next request ID
    fn next_request_id(&self) -> JsonRpcId {
        JsonRpcId::Number(self.next_id.fetch_add(1, Ordering::SeqCst))
    }

    /// Initialize the MCP connection
    async fn initialize(&self) -> Result<InitializeResult> {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.into(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "zero-bot".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        };

        let request = JsonRpcRequest::new(self.next_request_id(), methods::INITIALIZE)
            .with_params(serde_json::to_value(&params)?);

        let response = self.transport.send(&request).await?;

        if let Some(error) = response.error {
            bail!("MCP initialize failed: {} ({})", error.message, error.code);
        }

        let result: InitializeResult = serde_json::from_value(
            response
                .result
                .context("MCP initialize returned no result")?,
        )?;

        tracing::info!(
            "MCP server initialized: {} v{} (protocol {})",
            result.server_info.name,
            result.server_info.version,
            result.protocol_version
        );

        // Send initialized notification
        let notification = JsonRpcRequest::notification(methods::INITIALIZED);
        self.transport.notify(&notification).await?;

        *self.initialized.write().await = true;

        // Fetch available tools
        self.refresh_tools().await?;

        Ok(result)
    }

    /// Refresh the list of available tools from the server
    pub async fn refresh_tools(&self) -> Result<()> {
        let request = JsonRpcRequest::new(self.next_request_id(), methods::TOOLS_LIST);

        let response = self.transport.send(&request).await?;

        if let Some(error) = response.error {
            bail!("MCP tools/list failed: {} ({})", error.message, error.code);
        }

        let result: ListToolsResult = serde_json::from_value(
            response.result.context("MCP tools/list returned no result")?,
        )?;

        tracing::info!(
            "MCP server {} has {} tools",
            self.server_name,
            result.tools.len()
        );

        *self.tools.write().await = result.tools;

        Ok(())
    }

    /// Get the list of available tools
    pub async fn list_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.clone()
    }

    /// Call a tool by name with the given arguments
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Result<CallToolResult> {
        if !*self.initialized.read().await {
            bail!("MCP client not initialized");
        }

        let params = CallToolParams {
            name: name.into(),
            arguments,
        };

        let request = JsonRpcRequest::new(self.next_request_id(), methods::TOOLS_CALL)
            .with_params(serde_json::to_value(&params)?);

        tracing::debug!("Calling MCP tool: {} on {}", name, self.server_name);

        let response = self.transport.send(&request).await?;

        if let Some(error) = response.error {
            // Return error as tool result rather than failing
            return Ok(CallToolResult {
                content: vec![ToolContent::text(format!(
                    "MCP error: {} ({})",
                    error.message, error.code
                ))],
                is_error: true,
            });
        }

        serde_json::from_value(response.result.context("MCP tools/call returned no result")?)
            .context("Failed to parse MCP tool result")
    }

    /// Get the server name
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Check if the client is initialized
    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    /// Check if the connection is alive
    pub async fn is_alive(&self) -> bool {
        self.transport.is_alive().await
    }

    /// Close the MCP connection
    pub async fn close(&self) -> Result<()> {
        tracing::info!("Closing MCP client: {}", self.server_name);
        self.transport.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::types::JsonRpcResponse;

    // Mock transport for testing
    struct MockTransport {
        responses: std::sync::Mutex<Vec<JsonRpcResponse>>,
    }

    impl MockTransport {
        fn new(responses: Vec<JsonRpcResponse>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
            }
        }
    }

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn send(&self, _request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                bail!("No more mock responses");
            }
            Ok(responses.remove(0))
        }

        async fn notify(&self, _request: &JsonRpcRequest) -> Result<()> {
            Ok(())
        }

        async fn is_alive(&self) -> bool {
            true
        }

        async fn close(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn client_server_name() {
        let transport = MockTransport::new(vec![]);
        let client = McpClient::new(transport, "test-server");
        assert_eq!(client.server_name(), "test-server");
    }

    #[tokio::test]
    async fn client_not_initialized_by_default() {
        let transport = MockTransport::new(vec![]);
        let client = McpClient::new(transport, "test-server");
        assert!(!client.is_initialized().await);
    }

    #[tokio::test]
    async fn client_call_tool_fails_when_not_initialized() {
        let transport = MockTransport::new(vec![]);
        let client = McpClient::new(transport, "test-server");

        let result = client.call_tool("test", HashMap::new()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not initialized"));
    }
}
