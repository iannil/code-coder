//! Unified Tool Registry
//!
//! Manages all tools from different sources:
//! - Native `ZeroBot` tools (shell, `file_read`, `file_write`, memory, etc.)
//! - MCP tools from external MCP servers
//! - `CodeCoder` tools (23 AI agents)

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::{BrowserConfig, CodeCoderConfig, McpConfig, VaultConfig};
use crate::mcp::{McpManager, McpToolAdapter};
use crate::memory::Memory;
use crate::security::SecurityPolicy;

use super::Tool;

/// Unified registry for all tool types
pub struct ToolRegistry {
    /// Native `ZeroBot` tools
    native_tools: Vec<Arc<dyn Tool>>,
    /// MCP tools from external servers
    mcp_tools: RwLock<Vec<Arc<McpToolAdapter>>>,
    /// MCP manager for connection lifecycle
    mcp_manager: RwLock<McpManager>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            native_tools: Vec::new(),
            mcp_tools: RwLock::new(Vec::new()),
            mcp_manager: RwLock::new(McpManager::new()),
        }
    }

    /// Create a registry with native tools
    pub fn with_native_tools(
        security: &Arc<SecurityPolicy>,
        memory: Arc<dyn Memory>,
        browser_config: &BrowserConfig,
        codecoder_config: &CodeCoderConfig,
        vault_config: &VaultConfig,
        vault_path: &std::path::Path,
    ) -> Self {
        let tools_vec = super::all_tools(
            security,
            memory,
            browser_config,
            codecoder_config,
            vault_config,
            vault_path,
        );

        // Convert to Arc<dyn Tool>
        let native_tools: Vec<Arc<dyn Tool>> = tools_vec
            .into_iter()
            .map(|t| Arc::from(t) as Arc<dyn Tool>)
            .collect();

        Self {
            native_tools,
            mcp_tools: RwLock::new(Vec::new()),
            mcp_manager: RwLock::new(McpManager::new()),
        }
    }

    /// Connect to MCP servers from config
    pub async fn connect_mcp_servers(&self, mcp_config: &McpConfig) -> Result<()> {
        let mut manager = self.mcp_manager.write().await;
        manager.connect_servers(&mcp_config.servers).await?;

        // Get all tools from connected MCP servers
        let mut mcp_tools = self.mcp_tools.write().await;
        mcp_tools.clear();

        for client in manager.clients().values() {
            let tools = client.list_tools().await;
            for tool in tools {
                let adapter = McpToolAdapter::new(client.clone(), tool);
                mcp_tools.push(Arc::new(adapter));
            }
        }

        tracing::info!("MCP tools loaded: {}", mcp_tools.len());

        Ok(())
    }

    /// Refresh MCP tools from all connected servers
    pub async fn refresh_mcp_tools(&self) -> Result<()> {
        let manager = self.mcp_manager.read().await;
        manager.refresh_all().await?;

        // Re-fetch tools
        let mut mcp_tools = self.mcp_tools.write().await;
        mcp_tools.clear();

        for client in manager.clients().values() {
            let tools = client.list_tools().await;
            for tool in tools {
                let adapter = McpToolAdapter::new(client.clone(), tool);
                mcp_tools.push(Arc::new(adapter));
            }
        }

        Ok(())
    }

    /// Get all tools (native + MCP)
    pub async fn all_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools: Vec<Arc<dyn Tool>> = self.native_tools.clone();

        let mcp_tools = self.mcp_tools.read().await;
        for tool in mcp_tools.iter() {
            tools.push(tool.clone() as Arc<dyn Tool>);
        }

        tools
    }

    /// Get a tool by name
    pub async fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        // Check native tools first
        for tool in &self.native_tools {
            if tool.name() == name {
                return Some(tool.clone());
            }
        }

        // Check MCP tools
        let mcp_tools = self.mcp_tools.read().await;
        for tool in mcp_tools.iter() {
            if tool.name() == name {
                return Some(tool.clone() as Arc<dyn Tool>);
            }
        }

        None
    }

    /// Get tool names
    pub async fn tool_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.native_tools.iter().map(|t| t.name().to_string()).collect();

        let mcp_tools = self.mcp_tools.read().await;
        for tool in mcp_tools.iter() {
            names.push(tool.name().to_string());
        }

        names
    }

    /// Get native tools only
    pub fn native_tools(&self) -> &[Arc<dyn Tool>] {
        &self.native_tools
    }

    /// Get the number of MCP tools
    pub async fn mcp_tool_count(&self) -> usize {
        self.mcp_tools.read().await.len()
    }

    /// Close all MCP connections
    pub async fn close(&self) -> Result<()> {
        let manager = self.mcp_manager.read().await;
        manager.close_all().await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_new_is_empty() {
        let registry = ToolRegistry::new();
        assert!(registry.native_tools.is_empty());
    }

    #[tokio::test]
    async fn registry_get_tool_not_found() {
        let registry = ToolRegistry::new();
        let tool = registry.get_tool("nonexistent").await;
        assert!(tool.is_none());
    }

    #[tokio::test]
    async fn registry_tool_names_empty() {
        let registry = ToolRegistry::new();
        let names = registry.tool_names().await;
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn registry_mcp_tool_count_empty() {
        let registry = ToolRegistry::new();
        let count = registry.mcp_tool_count().await;
        assert_eq!(count, 0);
    }
}
