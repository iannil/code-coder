//! MCP (Model Context Protocol) Integration
//!
//! This module provides MCP support for `ZeroBot`, enabling:
//!
//! - **MCP Client**: Connect to external MCP servers and use their tools
//! - **MCP Server**: Expose `ZeroBot`'s tools to other MCP clients
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         ZeroBot                              │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌─────────────────┐     ┌─────────────────┐               │
//! │  │   MCP Client    │     │   MCP Server    │               │
//! │  │  (connects to   │     │ (exposes tools  │               │
//! │  │  external MCP)  │     │  via MCP)       │               │
//! │  └────────┬────────┘     └────────┬────────┘               │
//! │           │                       │                         │
//! │           ▼                       ▼                         │
//! │  ┌─────────────────────────────────────────┐               │
//! │  │           Tool Registry (unified)        │               │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐    │               │
//! │  │  │ Native  │ │  MCP    │ │CodeCoder│    │               │
//! │  │  │ Tools   │ │ Tools   │ │ Tools   │    │               │
//! │  │  └─────────┘ └─────────┘ └─────────┘    │               │
//! │  └─────────────────────────────────────────┘               │
//! │                       │                                     │
//! │                       ▼                                     │
//! │              AgentExecutor (LLM)                            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Configuration
//!
//! Add MCP servers to `~/.codecoder/config.json`:
//!
//! ```json
//! {
//!   "mcp": {
//!     "servers": {
//!       "filesystem": {
//!         "type": "local",
//!         "command": ["npx", "-y", "@anthropic/mcp-filesystem", "/path"],
//!         "enabled": true
//!       },
//!       "remote-api": {
//!         "type": "remote",
//!         "url": "https://api.example.com/mcp",
//!         "headers": { "Authorization": "Bearer xxx" },
//!         "enabled": true
//!       }
//!     },
//!     "server_enabled": true,
//!     "server_api_key": "optional-api-key"
//!   }
//! }
//! ```
//!
//! ## Usage
//!
//! ### As MCP Client
//!
//! ```rust,ignore
//! use zero_bot::mcp::{McpClient, create_mcp_tool_adapters};
//!
//! // Connect to a local MCP server
//! let client = McpClient::connect_local(
//!     "filesystem",
//!     &["npx", "-y", "@anthropic/mcp-filesystem", "/tmp"],
//!     None,
//! ).await?;
//!
//! // Get tools from the server
//! let mcp_tools = client.list_tools().await;
//!
//! // Create ZeroBot tool adapters
//! let tool_adapters = create_mcp_tool_adapters(Arc::new(client), mcp_tools);
//! ```
//!
//! ### As MCP Server
//!
//! ```rust,ignore
//! use zero_bot::mcp::McpServer;
//! use std::sync::Arc;
//!
//! // Create server with ZeroBot tools
//! let server = McpServer::new(tools);
//!
//! // Run in stdio mode (for subprocess usage)
//! server.serve_stdio().await?;
//!
//! // Or get routes for HTTP server
//! let routes = Arc::new(server).routes();
//! ```

pub mod adapter;
pub mod client;
pub mod server;
pub mod transport;
pub mod types;

pub use adapter::{create_mcp_tool_adapters, McpToolAdapter};
pub use client::McpClient;
pub use server::McpServer;

// Transport types - re-exported for public API consumers
#[allow(unused_imports)]
pub use transport::{HttpTransport, StdioTransport, Transport};

// MCP protocol types - re-exported for public API consumers
#[allow(unused_imports)]
pub use types::{
    CallToolParams, CallToolResult, InitializeParams, InitializeResult, JsonRpcError, JsonRpcId,
    JsonRpcRequest, JsonRpcResponse, ListToolsResult, McpTool, ServerCapabilities, ServerInfo,
    ToolContent, MCP_PROTOCOL_VERSION,
};

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::McpServerConfig;

/// Manager for MCP client connections
pub struct McpManager {
    clients: HashMap<String, Arc<McpClient>>,
}

impl McpManager {
    /// Create a new MCP manager
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Connect to configured MCP servers
    pub async fn connect_servers(
        &mut self,
        servers: &HashMap<String, McpServerConfig>,
    ) -> Result<()> {
        for (name, config) in servers {
            if !config.enabled() {
                tracing::debug!("MCP server {} is disabled, skipping", name);
                continue;
            }

            match self.connect_server(name, config).await {
                Ok(client) => {
                    self.clients.insert(name.clone(), Arc::new(client));
                    tracing::info!("Connected to MCP server: {}", name);
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to MCP server {}: {}", name, e);
                }
            }
        }

        Ok(())
    }

    /// Connect to a single MCP server
    async fn connect_server(&self, name: &str, config: &McpServerConfig) -> Result<McpClient> {
        match config {
            McpServerConfig::Local {
                command,
                environment,
                ..
            } => {
                McpClient::connect_local(name, command, environment.as_ref()).await
            }
            McpServerConfig::Remote { url, headers, .. } => {
                McpClient::connect_remote(name, url, headers.clone()).await
            }
        }
    }

    /// Get all connected clients
    pub fn clients(&self) -> &HashMap<String, Arc<McpClient>> {
        &self.clients
    }

    /// Get a specific client by name
    pub fn get_client(&self, name: &str) -> Option<&Arc<McpClient>> {
        self.clients.get(name)
    }

    /// Get all tools from all connected MCP servers
    pub async fn all_tools(&self) -> Vec<Box<dyn crate::tools::Tool>> {
        let mut tools = Vec::new();

        for (name, client) in &self.clients {
            let mcp_tools = client.list_tools().await;
            tracing::debug!("MCP server {} has {} tools", name, mcp_tools.len());

            let adapters = create_mcp_tool_adapters(client.clone(), mcp_tools);
            tools.extend(adapters);
        }

        tools
    }

    /// Refresh tools from all connected servers
    pub async fn refresh_all(&self) -> Result<()> {
        for (name, client) in &self.clients {
            if let Err(e) = client.refresh_tools().await {
                tracing::warn!("Failed to refresh tools from MCP server {}: {}", name, e);
            }
        }
        Ok(())
    }

    /// Close all connections
    pub async fn close_all(&self) -> Result<()> {
        for (name, client) in &self.clients {
            if let Err(e) = client.close().await {
                tracing::warn!("Failed to close MCP client {}: {}", name, e);
            }
        }
        Ok(())
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_manager_new() {
        let manager = McpManager::new();
        assert!(manager.clients.is_empty());
    }

    #[test]
    fn mcp_manager_default() {
        let manager = McpManager::default();
        assert!(manager.clients.is_empty());
    }

    #[test]
    fn mcp_manager_get_client_not_found() {
        let manager = McpManager::new();
        assert!(manager.get_client("nonexistent").is_none());
    }
}
