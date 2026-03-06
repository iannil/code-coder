//! MCP Client implementation
//!
//! Provides a client for connecting to MCP (Model Context Protocol) servers.
//! Supports multiple transports: Stdio, HTTP, and SSE.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{atomic::AtomicU64, Arc};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};

use super::mcp::{McpResource, McpTool, McpToolResult};

/// MCP Transport type
#[derive(Debug, Clone, PartialEq)]
pub enum McpTransportType {
    /// Stdio transport (spawns a subprocess)
    Stdio,
    /// HTTP transport (uses HTTP requests)
    Http,
    /// SSE transport (Server-Sent Events)
    Sse,
}

/// MCP Connection status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum McpConnectionStatus {
    /// Successfully connected
    Connected,
    /// Connection disabled
    Disabled,
    /// Connection failed
    Failed { error: String },
    /// Needs authentication
    NeedsAuth,
    /// Needs client registration
    NeedsClientRegistration { error: String },
}

/// MCP Client configuration
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Client name
    pub name: String,
    /// Transport type
    pub transport: McpTransportType,
    /// For Stdio: command to spawn
    pub command: Option<Vec<String>>,
    /// For HTTP/SSE: URL to connect to
    pub url: Option<String>,
    /// Environment variables for stdio transport
    pub environment: HashMap<String, String>,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
    /// HTTP headers for remote transports
    pub headers: HashMap<String, String>,
    /// Working directory for stdio transport
    pub cwd: Option<String>,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            name: "mcp-client".to_string(),
            transport: McpTransportType::Stdio,
            command: None,
            url: None,
            environment: HashMap::new(),
            timeout_ms: 30000,
            headers: HashMap::new(),
            cwd: None,
        }
    }
}

/// JSON-RPC request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

/// Transport trait for MCP communication
#[async_trait]
trait McpTransport: Send + Sync {
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value>;
    async fn close(&self) -> Result<()>;
}

/// Stdio transport implementation
struct StdioTransport {
    request_id: AtomicU64,
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
    sender: mpsc::Sender<String>,
    #[allow(dead_code)]
    child: Arc<RwLock<Option<Child>>>,
}

impl StdioTransport {
    async fn new(config: &McpClientConfig) -> Result<Self> {
        let command = config
            .command
            .as_ref()
            .ok_or_else(|| anyhow!("Stdio transport requires command"))?;

        if command.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }

        let (cmd, args) = command.split_first().unwrap();

        let mut process = Command::new(cmd);
        process.args(args);
        process.stdin(Stdio::piped());
        process.stdout(Stdio::piped());
        process.stderr(Stdio::piped());

        if let Some(ref cwd) = config.cwd {
            process.current_dir(cwd);
        }

        for (key, value) in &config.environment {
            process.env(key, value);
        }

        let mut child = process
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server: {}", cmd))?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;

        let (sender, mut receiver) = mpsc::channel::<String>(100);
        let pending: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<Value>>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let pending_clone = pending.clone();

        // Spawn writer task
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = receiver.recv().await {
                if let Err(e) = writeln!(stdin, "{}", msg) {
                    error!("Failed to write to MCP stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush() {
                    error!("Failed to flush MCP stdin: {}", e);
                    break;
                }
            }
        });

        // Spawn reader task
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<JsonRpcResponse>(&line) {
                            Ok(response) => {
                                let mut pending = pending_clone.write().await;
                                if let Some(sender) = pending.remove(&response.id) {
                                    let result = if let Some(error) = response.error {
                                        Err(anyhow!(
                                            "JSON-RPC error {}: {}",
                                            error.code,
                                            error.message
                                        ))
                                    } else {
                                        Ok(response.result.unwrap_or(Value::Null))
                                    };
                                    let _ = sender.send(result);
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse MCP response: {} - line: {}", e, line);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from MCP stdout: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            request_id: AtomicU64::new(1),
            pending,
            sender,
            child: Arc::new(RwLock::new(Some(child))),
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)?;

        let (response_tx, response_rx) = oneshot::channel();
        {
            let mut pending = self.pending.write().await;
            pending.insert(id, response_tx);
        }

        self.sender
            .send(request_json)
            .await
            .map_err(|_| anyhow!("Failed to send request"))?;

        match tokio::time::timeout(Duration::from_secs(30), response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow!("Response channel closed")),
            Err(_) => {
                let mut pending = self.pending.write().await;
                pending.remove(&id);
                Err(anyhow!("Request timed out"))
            }
        }
    }

    async fn close(&self) -> Result<()> {
        let mut child = self.child.write().await;
        if let Some(mut c) = child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        Ok(())
    }
}

/// HTTP transport implementation
struct HttpTransport {
    client: reqwest::Client,
    url: String,
    request_id: AtomicU64,
}

impl HttpTransport {
    async fn new(config: &McpClientConfig) -> Result<Self> {
        let url = config
            .url
            .as_ref()
            .ok_or_else(|| anyhow!("HTTP transport requires URL"))?;

        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in &config.headers {
            headers.insert(
                reqwest::header::HeaderName::from_bytes(key.as_bytes())?,
                reqwest::header::HeaderValue::from_str(value)?,
            );
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            url: url.clone(),
            request_id: AtomicU64::new(1),
        })
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        let response = self
            .client
            .post(&self.url)
            .json(&request)
            .send()
            .await
            .context("HTTP request failed")?;

        if response.status() == 401 {
            return Err(anyhow!("Unauthorized - authentication required"));
        }

        if !response.status().is_success() {
            return Err(anyhow!("HTTP error: {}", response.status()));
        }

        let json_response: JsonRpcResponse = response.json().await?;

        if let Some(error) = json_response.error {
            return Err(anyhow!(
                "JSON-RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        Ok(json_response.result.unwrap_or(Value::Null))
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// MCP Client for connecting to MCP servers
pub struct McpClientInstance {
    config: McpClientConfig,
    transport: Box<dyn McpTransport>,
    tools: RwLock<Vec<McpTool>>,
    #[allow(dead_code)]
    resources: RwLock<Vec<McpResource>>,
    status: RwLock<McpConnectionStatus>,
}

impl McpClientInstance {
    /// Connect to an MCP server
    pub async fn connect(config: McpClientConfig) -> Result<Self> {
        info!("Connecting to MCP server: {}", config.name);

        let transport: Box<dyn McpTransport> = match config.transport {
            McpTransportType::Stdio => Box::new(StdioTransport::new(&config).await?),
            McpTransportType::Http | McpTransportType::Sse => {
                Box::new(HttpTransport::new(&config).await?)
            }
        };

        let client = Self {
            config,
            transport,
            tools: RwLock::new(Vec::new()),
            resources: RwLock::new(Vec::new()),
            status: RwLock::new(McpConnectionStatus::Connected),
        };

        // Initialize connection
        client.initialize().await?;

        // List available tools
        client.refresh_tools().await?;

        Ok(client)
    }

    /// Initialize the MCP connection
    async fn initialize(&self) -> Result<()> {
        let params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": true },
                "resources": { "subscribe": true, "listChanged": true }
            },
            "clientInfo": {
                "name": &self.config.name,
                "version": "1.0.0"
            }
        });

        let result = self
            .transport
            .send_request("initialize", Some(params))
            .await?;

        debug!("MCP initialize response: {:?}", result);

        // Send initialized notification
        let _ = self
            .transport
            .send_request("notifications/initialized", None)
            .await;

        Ok(())
    }

    /// Refresh the list of available tools
    pub async fn refresh_tools(&self) -> Result<()> {
        let result = self
            .transport
            .send_request("tools/list", None)
            .await?;

        if let Some(tools_array) = result.get("tools").and_then(|t| t.as_array()) {
            let tools: Vec<McpTool> = tools_array
                .iter()
                .filter_map(|t| serde_json::from_value(t.clone()).ok())
                .collect();

            let mut tools_lock = self.tools.write().await;
            *tools_lock = tools;
        }

        Ok(())
    }

    /// List available tools
    pub async fn list_tools(&self) -> Vec<McpTool> {
        self.tools.read().await.clone()
    }

    /// Call a tool
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        let result = self
            .transport
            .send_request("tools/call", Some(params))
            .await?;

        serde_json::from_value(result).context("Failed to parse tool result")
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        let result = self
            .transport
            .send_request("resources/list", None)
            .await?;

        if let Some(resources_array) = result.get("resources").and_then(|r| r.as_array()) {
            let resources: Vec<McpResource> = resources_array
                .iter()
                .filter_map(|r| serde_json::from_value(r.clone()).ok())
                .collect();
            return Ok(resources);
        }

        Ok(Vec::new())
    }

    /// Read a resource
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        let params = serde_json::json!({
            "uri": uri
        });

        self.transport
            .send_request("resources/read", Some(params))
            .await
    }

    /// Get connection status
    pub async fn status(&self) -> McpConnectionStatus {
        self.status.read().await.clone()
    }

    /// Get server name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Close the connection
    pub async fn close(&self) -> Result<()> {
        info!("Closing MCP connection: {}", self.config.name);
        self.transport.close().await
    }
}

impl Drop for McpClientInstance {
    fn drop(&mut self) {
        debug!("Dropping MCP client: {}", self.config.name);
    }
}

/// MCP Client Manager for managing multiple MCP connections
pub struct McpClientManager {
    clients: RwLock<HashMap<String, Arc<McpClientInstance>>>,
}

impl McpClientManager {
    /// Create a new MCP client manager
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
        }
    }

    /// Add a client
    pub async fn add(&self, name: &str, config: McpClientConfig) -> Result<McpConnectionStatus> {
        match McpClientInstance::connect(config).await {
            Ok(client) => {
                let status = client.status().await;
                let mut clients = self.clients.write().await;
                clients.insert(name.to_string(), Arc::new(client));
                Ok(status)
            }
            Err(e) => {
                warn!("Failed to connect MCP client {}: {}", name, e);
                Ok(McpConnectionStatus::Failed {
                    error: e.to_string(),
                })
            }
        }
    }

    /// Get a client by name
    pub async fn get(&self, name: &str) -> Option<Arc<McpClientInstance>> {
        let clients = self.clients.read().await;
        clients.get(name).cloned()
    }

    /// Remove a client
    pub async fn remove(&self, name: &str) -> Result<()> {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.remove(name) {
            client.close().await?;
        }
        Ok(())
    }

    /// List all client statuses
    pub async fn status(&self) -> HashMap<String, McpConnectionStatus> {
        let clients = self.clients.read().await;
        let mut statuses = HashMap::new();

        for (name, client) in clients.iter() {
            statuses.insert(name.clone(), client.status().await);
        }

        statuses
    }

    /// Get all tools from all connected clients
    pub async fn all_tools(&self) -> HashMap<String, McpTool> {
        let clients = self.clients.read().await;
        let mut tools = HashMap::new();

        for (client_name, client) in clients.iter() {
            for tool in client.list_tools().await {
                let key = format!("{}_{}", client_name, tool.name);
                tools.insert(key, tool);
            }
        }

        tools
    }

    /// Close all connections
    pub async fn close_all(&self) -> Result<()> {
        let mut clients = self.clients.write().await;
        for (name, client) in clients.drain() {
            if let Err(e) = client.close().await {
                warn!("Failed to close MCP client {}: {}", name, e);
            }
        }
        Ok(())
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_transport_type() {
        assert_eq!(McpTransportType::Stdio, McpTransportType::Stdio);
        assert_ne!(McpTransportType::Stdio, McpTransportType::Http);
    }

    #[test]
    fn test_mcp_client_config_default() {
        let config = McpClientConfig::default();
        assert_eq!(config.name, "mcp-client");
        assert_eq!(config.transport, McpTransportType::Stdio);
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_mcp_connection_status_serialization() {
        let status = McpConnectionStatus::Connected;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("connected"));

        let failed = McpConnectionStatus::Failed {
            error: "test error".to_string(),
        };
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.contains("failed"));
        assert!(json.contains("test error"));
    }

    #[test]
    fn test_mcp_client_manager_new() {
        let manager = McpClientManager::new();
        assert!(manager.clients.blocking_read().is_empty());
    }

    #[tokio::test]
    async fn test_mcp_client_manager_operations() {
        let manager = McpClientManager::new();

        // Test status on empty manager
        let statuses = manager.status().await;
        assert!(statuses.is_empty());

        // Test all_tools on empty manager
        let tools = manager.all_tools().await;
        assert!(tools.is_empty());
    }

    #[test]
    fn test_json_rpc_request() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test".to_string(),
            params: Some(serde_json::json!({"key": "value"})),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("test"));
        assert!(json.contains("key"));
    }

    #[test]
    fn test_json_rpc_response_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"data":"test"}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_error_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, 1);
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
    }
}
