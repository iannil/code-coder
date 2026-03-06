//! Protocol module - MCP, LSP, and JSON-RPC implementations
//!
//! This module provides:
//! - **mcp**: Model Context Protocol server
//! - **mcp_client**: Model Context Protocol client
//! - **lsp**: Language Server Protocol support

pub mod mcp;
pub mod mcp_client;
pub mod lsp;

// Re-export main types from mcp server
pub use mcp::{McpClient, McpResource, McpServer, McpTool, McpToolResult};

// Re-export main types from mcp client
pub use mcp_client::{
    McpClientConfig, McpClientInstance, McpClientManager, McpConnectionStatus, McpTransportType,
};

// Re-export main types from lsp
pub use lsp::{
    LspServerInfo, LspServerManager, LspServerStatus,
    LspLocation, LspSymbol, LspCompletionItem, LspTextEdit,
};
