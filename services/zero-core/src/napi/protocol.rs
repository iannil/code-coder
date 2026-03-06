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
    mcp_oauth::{
        AuthStatus as RustAuthStatus, OAuthConfig as RustOAuthConfig,
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
    /// OAuth configuration
    pub oauth: Option<OAuthConfig>,
    /// Whether OAuth is disabled
    pub oauth_disabled: Option<bool>,
}

/// OAuth configuration for an MCP server
#[napi(object)]
pub struct OAuthConfig {
    /// Pre-registered client ID (optional)
    pub client_id: Option<String>,
    /// Pre-registered client secret (optional)
    pub client_secret: Option<String>,
    /// OAuth scopes to request
    pub scope: Option<String>,
}

impl From<OAuthConfig> for RustOAuthConfig {
    fn from(config: OAuthConfig) -> Self {
        RustOAuthConfig {
            client_id: config.client_id,
            client_secret: config.client_secret,
            scope: config.scope,
        }
    }
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
            oauth: config.oauth.map(|o| o.into()),
            oauth_disabled: config.oauth_disabled.unwrap_or(false),
        }
    }
}

/// OAuth authentication status
#[napi(string_enum)]
pub enum AuthStatus {
    /// Not authenticated
    NotAuthenticated,
    /// Authenticated with valid tokens
    Authenticated,
    /// Token expired but can be refreshed
    Expired,
}

impl From<RustAuthStatus> for AuthStatus {
    fn from(status: RustAuthStatus) -> Self {
        match status {
            RustAuthStatus::NotAuthenticated => AuthStatus::NotAuthenticated,
            RustAuthStatus::Authenticated => AuthStatus::Authenticated,
            RustAuthStatus::Expired => AuthStatus::Expired,
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

    // ========================================================================
    // OAuth methods
    // ========================================================================

    /// Load OAuth credentials from storage
    #[napi]
    pub async fn load_oauth(&self) -> Result<()> {
        let manager = self.inner.read().await;
        manager
            .load_oauth()
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Start OAuth authentication flow for a server
    /// Returns the authorization URL that should be opened in a browser
    #[napi]
    pub async fn start_oauth(
        &self,
        server_name: String,
        server_url: String,
        redirect_uri: String,
        config: Option<OAuthConfig>,
    ) -> Result<String> {
        let manager = self.inner.read().await;
        let rust_config = config.map(|c| c.into());
        manager
            .start_oauth(&server_name, &server_url, rust_config.as_ref(), &redirect_uri)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Complete OAuth authentication with the authorization code
    #[napi]
    pub async fn finish_oauth(
        &self,
        server_name: String,
        authorization_code: String,
        state: String,
    ) -> Result<()> {
        let manager = self.inner.read().await;
        manager
            .finish_oauth(&server_name, &authorization_code, &state)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Remove OAuth credentials for a server
    #[napi]
    pub async fn remove_oauth(&self, server_name: String) -> Result<()> {
        let manager = self.inner.read().await;
        manager
            .remove_oauth(&server_name)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get OAuth authentication status for a server
    #[napi]
    pub async fn get_oauth_status(&self, server_name: String) -> Result<AuthStatus> {
        let manager = self.inner.read().await;
        let status = manager.get_oauth_status(&server_name).await;
        Ok(status.into())
    }

    /// Check if we have OAuth credentials for a server
    #[napi]
    pub async fn has_oauth_credentials(&self, server_name: String) -> Result<bool> {
        let manager = self.inner.read().await;
        Ok(manager.has_oauth_credentials(&server_name).await)
    }

    /// Cancel any pending OAuth flow for a server
    #[napi]
    pub async fn cancel_oauth(&self, server_name: String) -> Result<()> {
        let manager = self.inner.read().await;
        manager.cancel_oauth(&server_name).await;
        Ok(())
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

/// LSP location (file + range)
#[napi(object)]
pub struct LspLocation {
    /// File URI
    pub uri: String,
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start character (0-indexed)
    pub start_character: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End character (0-indexed)
    pub end_character: u32,
}

impl From<crate::protocol::lsp::LspLocation> for LspLocation {
    fn from(loc: crate::protocol::lsp::LspLocation) -> Self {
        LspLocation {
            uri: loc.uri,
            start_line: loc.start_line,
            start_character: loc.start_character,
            end_line: loc.end_line,
            end_character: loc.end_character,
        }
    }
}

/// LSP document symbol
#[napi(object)]
pub struct LspSymbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind (Function, Class, Method, etc.)
    pub kind: String,
    /// Start line
    pub start_line: u32,
    /// Start character
    pub start_character: u32,
    /// End line
    pub end_line: u32,
    /// End character
    pub end_character: u32,
}

impl From<crate::protocol::lsp::LspSymbol> for LspSymbol {
    fn from(sym: crate::protocol::lsp::LspSymbol) -> Self {
        LspSymbol {
            name: sym.name,
            kind: sym.kind,
            start_line: sym.start_line,
            start_character: sym.start_character,
            end_line: sym.end_line,
            end_character: sym.end_character,
        }
    }
}

/// LSP completion item
#[napi(object)]
pub struct LspCompletionItem {
    /// Display label
    pub label: String,
    /// Completion kind (Function, Variable, etc.)
    pub kind: Option<String>,
    /// Additional detail
    pub detail: Option<String>,
    /// Text to insert
    pub insert_text: Option<String>,
}

impl From<crate::protocol::lsp::LspCompletionItem> for LspCompletionItem {
    fn from(item: crate::protocol::lsp::LspCompletionItem) -> Self {
        LspCompletionItem {
            label: item.label,
            kind: item.kind,
            detail: item.detail,
            insert_text: item.insert_text,
        }
    }
}

/// LSP text edit
#[napi(object)]
pub struct LspTextEdit {
    /// Start line
    pub start_line: u32,
    /// Start character
    pub start_character: u32,
    /// End line
    pub end_line: u32,
    /// End character
    pub end_character: u32,
    /// New text to insert
    pub new_text: String,
}

impl From<crate::protocol::lsp::LspTextEdit> for LspTextEdit {
    fn from(edit: crate::protocol::lsp::LspTextEdit) -> Self {
        LspTextEdit {
            start_line: edit.start_line,
            start_character: edit.start_character,
            end_line: edit.end_line,
            end_character: edit.end_character,
            new_text: edit.new_text,
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

    // ========================================================================
    // Convenience methods for common LSP operations
    // ========================================================================

    /// Get hover information at a position
    #[napi]
    pub async fn hover(
        &self,
        key: String,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<Option<String>> {
        self.inner
            .hover(&key, &uri, line, character)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Go to definition
    #[napi]
    pub async fn goto_definition(
        &self,
        key: String,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspLocation>> {
        let locations = self.inner
            .goto_definition(&key, &uri, line, character)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(locations.into_iter().map(|l| l.into()).collect())
    }

    /// Go to type definition
    #[napi]
    pub async fn goto_type_definition(
        &self,
        key: String,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspLocation>> {
        let locations = self.inner
            .goto_type_definition(&key, &uri, line, character)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(locations.into_iter().map(|l| l.into()).collect())
    }

    /// Find references
    #[napi]
    pub async fn find_references(
        &self,
        key: String,
        uri: String,
        line: u32,
        character: u32,
        include_declaration: Option<bool>,
    ) -> Result<Vec<LspLocation>> {
        let locations = self.inner
            .find_references(&key, &uri, line, character, include_declaration.unwrap_or(true))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(locations.into_iter().map(|l| l.into()).collect())
    }

    /// Get document symbols
    #[napi]
    pub async fn document_symbols(
        &self,
        key: String,
        uri: String,
    ) -> Result<Vec<LspSymbol>> {
        let symbols = self.inner
            .document_symbols(&key, &uri)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(symbols.into_iter().map(|s| s.into()).collect())
    }

    /// Get completions at a position
    #[napi]
    pub async fn completion(
        &self,
        key: String,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<Vec<LspCompletionItem>> {
        let completions = self.inner
            .completion(&key, &uri, line, character)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(completions.into_iter().map(|c| c.into()).collect())
    }

    /// Format document
    #[napi]
    pub async fn format_document(
        &self,
        key: String,
        uri: String,
        tab_size: Option<u32>,
        insert_spaces: Option<bool>,
    ) -> Result<Vec<LspTextEdit>> {
        let edits = self.inner
            .format_document(&key, &uri, tab_size.unwrap_or(2), insert_spaces.unwrap_or(true))
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(edits.into_iter().map(|e| e.into()).collect())
    }

    /// Notify document opened
    #[napi]
    pub async fn did_open(
        &self,
        key: String,
        uri: String,
        language_id: String,
        version: u32,
        text: String,
    ) -> Result<()> {
        self.inner
            .did_open(&key, &uri, &language_id, version, &text)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Notify document closed
    #[napi]
    pub async fn did_close(&self, key: String, uri: String) -> Result<()> {
        self.inner
            .did_close(&key, &uri)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Notify document changed
    #[napi]
    pub async fn did_change(
        &self,
        key: String,
        uri: String,
        version: u32,
        text: String,
    ) -> Result<()> {
        self.inner
            .did_change(&key, &uri, version, &text)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Detect language ID from file extension
    #[napi]
    pub fn detect_language_id(extension: String) -> String {
        use crate::protocol::lsp::LspServerManager;
        LspServerManager::detect_language_id(&extension).to_string()
    }
}
