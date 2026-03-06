//! NAPI bindings for protocol layer (MCP Client, LSP)
//!
//! This module exposes MCP client and LSP server functionality to Node.js.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::protocol::{
    mcp_client::{
        McpClientConfig as RustMcpClientConfig, McpClientManager as RustMcpClientManager,
        McpConnectionStatus as RustMcpConnectionStatus, McpTransportType as RustMcpTransportType,
    },
    lsp::{LspServerManager as RustLspServerManager, LspServerStatus as RustLspServerStatus},
};

// ============================================================================
// MCP Client bindings
// ============================================================================

/// MCP transport type
#[napi(string_enum)]
pub enum McpTransportType {
    Stdio,
    Http,
    Sse,
}

impl From<McpTransportType> for RustMcpTransportType {
    fn from(t: McpTransportType) -> Self {
        match t {
            McpTransportType::Stdio => RustMcpTransportType::Stdio,
            McpTransportType::Http => RustMcpTransportType::Http,
            McpTransportType::Sse => RustMcpTransportType::Sse,
        }
    }
}

/// MCP client configuration
#[napi(object)]
pub struct McpClientConfig {
    /// Client name
    pub name: String,
    /// Transport type (stdio, http, sse)
    pub transport: String,
    /// Command to spawn (for stdio)
    pub command: Option<Vec<String>>,
    /// URL to connect to (for http/sse)
    pub url: Option<String>,
    /// Environment variables
    pub environment: Option<HashMap<String, String>>,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u32>,
    /// HTTP headers
    pub headers: Option<HashMap<String, String>>,
    /// Working directory (for stdio)
    pub cwd: Option<String>,
}

impl From<McpClientConfig> for RustMcpClientConfig {
    fn from(config: McpClientConfig) -> Self {
        let transport = match config.transport.as_str() {
            "http" => RustMcpTransportType::Http,
            "sse" => RustMcpTransportType::Sse,
            _ => RustMcpTransportType::Stdio,
        };

        RustMcpClientConfig {
            name: config.name,
            transport,
            command: config.command,
            url: config.url,
            environment: config.environment.unwrap_or_default(),
            timeout_ms: config.timeout_ms.unwrap_or(30000) as u64,
            headers: config.headers.unwrap_or_default(),
            cwd: config.cwd,
        }
    }
}

/// MCP connection status
#[napi(object)]
pub struct McpConnectionStatus {
    pub status: String,
    pub error: Option<String>,
}

impl From<RustMcpConnectionStatus> for McpConnectionStatus {
    fn from(status: RustMcpConnectionStatus) -> Self {
        match status {
            RustMcpConnectionStatus::Connected => McpConnectionStatus {
                status: "connected".to_string(),
                error: None,
            },
            RustMcpConnectionStatus::Disabled => McpConnectionStatus {
                status: "disabled".to_string(),
                error: None,
            },
            RustMcpConnectionStatus::Failed { error } => McpConnectionStatus {
                status: "failed".to_string(),
                error: Some(error),
            },
            RustMcpConnectionStatus::NeedsAuth => McpConnectionStatus {
                status: "needs_auth".to_string(),
                error: None,
            },
            RustMcpConnectionStatus::NeedsClientRegistration { error } => McpConnectionStatus {
                status: "needs_client_registration".to_string(),
                error: Some(error),
            },
        }
    }
}

/// MCP tool definition
#[napi(object)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tool call result
#[napi(object)]
pub struct McpToolResult {
    pub content: Vec<serde_json::Value>,
    pub is_error: bool,
}

/// Handle to an MCP client manager
#[napi]
pub struct McpClientManagerHandle {
    inner: Arc<RwLock<RustMcpClientManager>>,
}

/// Create a new MCP client manager
#[napi]
pub fn create_mcp_client_manager() -> McpClientManagerHandle {
    McpClientManagerHandle {
        inner: Arc::new(RwLock::new(RustMcpClientManager::new())),
    }
}

#[napi]
impl McpClientManagerHandle {
    /// Add a client
    #[napi]
    pub async fn add(&self, name: String, config: McpClientConfig) -> Result<McpConnectionStatus> {
        let manager = self.inner.read().await;
        let rust_config: RustMcpClientConfig = config.into();
        let status = manager
            .add(&name, rust_config)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(status.into())
    }

    /// Get connection status for all clients
    #[napi]
    pub async fn status(&self) -> Result<HashMap<String, McpConnectionStatus>> {
        let manager = self.inner.read().await;
        let statuses = manager.status().await;
        Ok(statuses.into_iter().map(|(k, v)| (k, v.into())).collect())
    }

    /// List all tools from all connected clients
    #[napi]
    pub async fn list_tools(&self) -> Result<HashMap<String, McpTool>> {
        let manager = self.inner.read().await;
        let tools = manager.all_tools().await;
        Ok(tools
            .into_iter()
            .map(|(k, t)| {
                (
                    k,
                    McpTool {
                        name: t.name,
                        description: t.description,
                        input_schema: t.input_schema,
                    },
                )
            })
            .collect())
    }

    /// Call a tool on a specific client
    #[napi]
    pub async fn call_tool(
        &self,
        client_name: String,
        tool_name: String,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult> {
        let manager = self.inner.read().await;
        let client = manager
            .get(&client_name)
            .await
            .ok_or_else(|| Error::from_reason(format!("Client not found: {}", client_name)))?;

        let result = client
            .call_tool(&tool_name, arguments)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(McpToolResult {
            content: result.content.into_iter().map(|c| serde_json::to_value(c).unwrap_or_default()).collect(),
            is_error: result.is_error,
        })
    }

    /// Remove a client
    #[napi]
    pub async fn remove(&self, name: String) -> Result<()> {
        let manager = self.inner.read().await;
        manager
            .remove(&name)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Close all connections
    #[napi]
    pub async fn close_all(&self) -> Result<()> {
        let manager = self.inner.read().await;
        manager
            .close_all()
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

// ============================================================================
// LSP Server bindings
// ============================================================================

/// LSP server status
#[napi(object)]
pub struct LspServerStatus {
    pub status: String,
    pub error: Option<String>,
}

impl From<RustLspServerStatus> for LspServerStatus {
    fn from(status: RustLspServerStatus) -> Self {
        match status {
            RustLspServerStatus::Running => LspServerStatus {
                status: "running".to_string(),
                error: None,
            },
            RustLspServerStatus::Starting => LspServerStatus {
                status: "starting".to_string(),
                error: None,
            },
            RustLspServerStatus::Stopped => LspServerStatus {
                status: "stopped".to_string(),
                error: None,
            },
            RustLspServerStatus::Failed { error } => LspServerStatus {
                status: "failed".to_string(),
                error: Some(error),
            },
            RustLspServerStatus::NotFound => LspServerStatus {
                status: "not_found".to_string(),
                error: None,
            },
        }
    }
}

/// Handle to an LSP server manager
#[napi]
pub struct LspServerManagerHandle {
    inner: Arc<RustLspServerManager>,
}

/// Create a new LSP server manager
#[napi]
pub fn create_lsp_server_manager() -> LspServerManagerHandle {
    LspServerManagerHandle {
        inner: Arc::new(RustLspServerManager::new()),
    }
}

#[napi]
impl LspServerManagerHandle {
    /// Start a language server for a file (auto-detects based on extension)
    #[napi]
    pub async fn start_for_file(&self, file_path: String) -> Result<String> {
        self.inner
            .start_for_file(std::path::Path::new(&file_path))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Start a specific language server
    #[napi]
    pub async fn start(&self, server_id: String, root: String) -> Result<String> {
        self.inner
            .start(&server_id, std::path::Path::new(&root))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Send a request to a language server
    #[napi]
    pub async fn request(
        &self,
        key: String,
        method: String,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.inner
            .request(&key, &method, params)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Stop a language server
    #[napi]
    pub async fn stop(&self, key: String) -> Result<()> {
        self.inner
            .stop(&key)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get status of a language server
    #[napi]
    pub async fn status(&self, key: String) -> Result<LspServerStatus> {
        let status = self.inner.status(&key).await;
        Ok(status.into())
    }

    /// Get all server statuses
    #[napi]
    pub async fn all_statuses(&self) -> Result<HashMap<String, LspServerStatus>> {
        let statuses = self.inner.all_statuses().await;
        Ok(statuses.into_iter().map(|(k, v)| (k, v.into())).collect())
    }

    /// Stop all language servers
    #[napi]
    pub async fn stop_all(&self) -> Result<()> {
        self.inner
            .stop_all()
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}
