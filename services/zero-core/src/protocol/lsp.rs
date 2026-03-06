//! LSP Server Manager implementation
//!
//! Manages multiple Language Server Protocol (LSP) server instances.
//! Supports spawning and managing various language servers.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};

/// Language server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerInfo {
    /// Server identifier
    pub id: String,
    /// Supported file extensions
    pub extensions: Vec<String>,
    /// Whether this is a global server (not project-specific)
    #[serde(default)]
    pub global: bool,
}

/// LSP location (file + range)
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// LSP document symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// LSP completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// LSP text edit
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// LSP server status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LspServerStatus {
    /// Server is running
    Running,
    /// Server is starting
    Starting,
    /// Server is stopped
    Stopped,
    /// Server failed to start
    Failed { error: String },
    /// Server not found
    NotFound,
}

/// LSP message types
#[derive(Debug, Clone)]
pub enum LspMessage {
    Request {
        id: u64,
        method: String,
        params: Value,
    },
    Response {
        id: u64,
        result: Option<Value>,
        error: Option<Value>,
    },
    Notification {
        method: String,
        params: Value,
    },
}

/// LSP server handle for a running language server
pub struct LspServerHandle {
    /// Server ID
    pub id: String,
    /// Root directory
    pub root: PathBuf,
    /// Child process
    child: Option<Child>,
    /// Request sender
    request_tx: mpsc::Sender<(String, Value, oneshot::Sender<Result<Value>>)>,
    /// Request ID counter
    request_id: std::sync::atomic::AtomicU64,
}

impl LspServerHandle {
    /// Send a request to the language server
    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let _id = self
            .request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        self.request_tx
            .send((method.to_string(), params, tx))
            .await
            .map_err(|_| anyhow!("Failed to send request"))?;

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow!("Response channel closed")),
            Err(_) => Err(anyhow!("Request timed out")),
        }
    }

    /// Send a notification to the language server
    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        let (tx, _rx) = oneshot::channel();
        self.request_tx
            .send((method.to_string(), params, tx))
            .await
            .map_err(|_| anyhow!("Failed to send notification"))?;
        Ok(())
    }

    /// Stop the language server
    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for LspServerHandle {
    fn drop(&mut self) {
        if let Err(e) = self.stop() {
            warn!("Failed to stop LSP server {}: {}", self.id, e);
        }
    }
}

/// Supported language server configurations
pub mod servers {
    use super::*;

    /// Spawn configuration for a language server
    pub struct SpawnConfig {
        /// Command to execute
        pub command: String,
        /// Arguments
        pub args: Vec<String>,
        /// Environment variables
        pub env: HashMap<String, String>,
        /// Initialization options
        pub initialization_options: Option<Value>,
    }

    /// TypeScript language server (typescript-language-server)
    pub fn typescript(_root: &Path) -> Option<SpawnConfig> {
        if let Ok(tls) = which::which("typescript-language-server") {
            return Some(SpawnConfig {
                command: tls.to_string_lossy().to_string(),
                args: vec!["--stdio".to_string()],
                env: HashMap::new(),
                initialization_options: None,
            });
        }
        None
    }

    /// Rust Analyzer
    pub fn rust_analyzer(_root: &Path) -> Option<SpawnConfig> {
        if let Ok(ra) = which::which("rust-analyzer") {
            return Some(SpawnConfig {
                command: ra.to_string_lossy().to_string(),
                args: vec![],
                env: HashMap::new(),
                initialization_options: None,
            });
        }
        None
    }

    /// Gopls (Go language server)
    pub fn gopls(_root: &Path) -> Option<SpawnConfig> {
        if let Ok(gopls) = which::which("gopls") {
            return Some(SpawnConfig {
                command: gopls.to_string_lossy().to_string(),
                args: vec![],
                env: HashMap::new(),
                initialization_options: None,
            });
        }
        None
    }

    /// Pyright (Python language server)
    pub fn pyright(_root: &Path) -> Option<SpawnConfig> {
        if let Ok(pyright) = which::which("pyright-langserver") {
            return Some(SpawnConfig {
                command: pyright.to_string_lossy().to_string(),
                args: vec!["--stdio".to_string()],
                env: HashMap::new(),
                initialization_options: None,
            });
        }
        None
    }

    /// Deno language server
    pub fn deno(root: &Path) -> Option<SpawnConfig> {
        // Check for deno.json(c) to detect Deno projects
        let deno_config = root.join("deno.json").exists() || root.join("deno.jsonc").exists();
        if !deno_config {
            return None;
        }

        if let Ok(deno) = which::which("deno") {
            return Some(SpawnConfig {
                command: deno.to_string_lossy().to_string(),
                args: vec!["lsp".to_string()],
                env: HashMap::new(),
                initialization_options: None,
            });
        }
        None
    }

    /// Clangd (C/C++ language server)
    pub fn clangd(_root: &Path) -> Option<SpawnConfig> {
        if let Ok(clangd) = which::which("clangd") {
            return Some(SpawnConfig {
                command: clangd.to_string_lossy().to_string(),
                args: vec!["--background-index".to_string(), "--clang-tidy".to_string()],
                env: HashMap::new(),
                initialization_options: None,
            });
        }
        None
    }

    /// Get spawn config for a language by extension
    pub fn for_extension(extension: &str, root: &Path) -> Option<(String, SpawnConfig)> {
        match extension {
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
                // Check for Deno first
                if let Some(config) = deno(root) {
                    return Some(("deno".to_string(), config));
                }
                typescript(root).map(|c| ("typescript".to_string(), c))
            }
            "rs" => rust_analyzer(root).map(|c| ("rust-analyzer".to_string(), c)),
            "go" => gopls(root).map(|c| ("gopls".to_string(), c)),
            "py" | "pyi" => pyright(root).map(|c| ("pyright".to_string(), c)),
            "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" => clangd(root).map(|c| ("clangd".to_string(), c)),
            _ => None,
        }
    }
}

/// LSP Server Manager
pub struct LspServerManager {
    /// Running servers by ID
    servers: RwLock<HashMap<String, Arc<RwLock<LspServerHandle>>>>,
    /// Server statuses
    statuses: RwLock<HashMap<String, LspServerStatus>>,
}

impl LspServerManager {
    /// Create a new LSP server manager
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            statuses: RwLock::new(HashMap::new()),
        }
    }

    /// Start a language server for a file
    pub async fn start_for_file(&self, file_path: &Path) -> Result<String> {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let root = self.find_project_root(file_path)?;

        let (server_id, config) = servers::for_extension(extension, &root)
            .ok_or_else(|| anyhow!("No language server available for extension: {}", extension))?;

        let key = format!("{}:{}", server_id, root.display());

        // Check if already running
        {
            let servers = self.servers.read().await;
            if servers.contains_key(&key) {
                return Ok(key);
            }
        }

        // Start the server
        self.spawn_server(&key, &server_id, &root, config).await?;

        Ok(key)
    }

    /// Start a specific language server
    pub async fn start(&self, server_id: &str, root: &Path) -> Result<String> {
        let config = match server_id {
            "typescript" => servers::typescript(root),
            "rust-analyzer" => servers::rust_analyzer(root),
            "gopls" => servers::gopls(root),
            "pyright" => servers::pyright(root),
            "deno" => servers::deno(root),
            "clangd" => servers::clangd(root),
            _ => None,
        }
        .ok_or_else(|| anyhow!("Unknown or unavailable language server: {}", server_id))?;

        let key = format!("{}:{}", server_id, root.display());
        self.spawn_server(&key, server_id, root, config).await?;

        Ok(key)
    }

    /// Spawn a language server
    async fn spawn_server(
        &self,
        key: &str,
        server_id: &str,
        root: &Path,
        config: servers::SpawnConfig,
    ) -> Result<()> {
        info!("Starting LSP server: {} at {}", server_id, root.display());

        // Update status to starting
        {
            let mut statuses = self.statuses.write().await;
            statuses.insert(key.to_string(), LspServerStatus::Starting);
        }

        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (k, v) in &config.env {
            command.env(k, v);
        }

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                let mut statuses = self.statuses.write().await;
                statuses.insert(
                    key.to_string(),
                    LspServerStatus::Failed {
                        error: e.to_string(),
                    },
                );
                return Err(e.into());
            }
        };

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("Failed to get stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to get stdout"))?;

        // Create channels for communication
        let (request_tx, mut request_rx) =
            mpsc::channel::<(String, Value, oneshot::Sender<Result<Value>>)>(100);

        let pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<Value>>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let pending_clone = pending_requests.clone();

        let request_id = std::sync::atomic::AtomicU64::new(1);

        // Spawn writer task
        let pending_for_writer = pending_requests.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            let mut id_counter: u64 = 1;

            while let Some((method, params, response_tx)) = request_rx.recv().await {
                let id = id_counter;
                id_counter += 1;

                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method,
                    "params": params
                });

                let content = serde_json::to_string(&request).unwrap();
                let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);

                // Store pending request
                {
                    let mut pending = pending_for_writer.write().await;
                    pending.insert(id, response_tx);
                }

                if let Err(e) = stdin.write_all(message.as_bytes()) {
                    error!("Failed to write to LSP: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush() {
                    error!("Failed to flush LSP stdin: {}", e);
                    break;
                }
            }
        });

        // Spawn reader task
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut headers = String::new();

            loop {
                headers.clear();

                // Read headers
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => return, // EOF
                        Ok(_) => {
                            if line == "\r\n" || line == "\n" {
                                break;
                            }
                            headers.push_str(&line);
                        }
                        Err(e) => {
                            error!("Failed to read LSP header: {}", e);
                            return;
                        }
                    }
                }

                // Parse Content-Length
                let content_length: usize = headers
                    .lines()
                    .find_map(|line| {
                        if line.to_lowercase().starts_with("content-length:") {
                            line.split(':').nth(1)?.trim().parse().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);

                if content_length == 0 {
                    continue;
                }

                // Read content
                let mut content = vec![0u8; content_length];
                if let Err(e) = std::io::Read::read_exact(&mut reader, &mut content) {
                    error!("Failed to read LSP content: {}", e);
                    continue;
                }

                // Parse JSON
                let content_str = String::from_utf8_lossy(&content);
                match serde_json::from_str::<Value>(&content_str) {
                    Ok(value) => {
                        // Check if it's a response
                        if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                            let mut pending = pending_clone.write().await;
                            if let Some(tx) = pending.remove(&id) {
                                if let Some(error) = value.get("error") {
                                    let _ = tx.send(Err(anyhow!("LSP error: {:?}", error)));
                                } else {
                                    let _ = tx.send(Ok(value.get("result").cloned().unwrap_or(Value::Null)));
                                }
                            }
                        }
                        // TODO: Handle notifications
                    }
                    Err(e) => {
                        debug!("Failed to parse LSP response: {}", e);
                    }
                }
            }
        });

        // Create handle
        let handle = LspServerHandle {
            id: server_id.to_string(),
            root: root.to_path_buf(),
            child: Some(child),
            request_tx,
            request_id,
        };

        // Initialize the server
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": format!("file://{}", root.display()),
            "capabilities": {
                "textDocument": {
                    "hover": { "contentFormat": ["markdown", "plaintext"] },
                    "completion": {
                        "completionItem": { "snippetSupport": true }
                    },
                    "definition": { "linkSupport": true },
                    "references": {},
                    "documentSymbol": { "hierarchicalDocumentSymbolSupport": true },
                    "codeAction": { "codeActionLiteralSupport": {} },
                    "formatting": {},
                    "rangeFormatting": {}
                },
                "workspace": {
                    "workspaceFolders": true
                }
            },
            "workspaceFolders": [
                {
                    "uri": format!("file://{}", root.display()),
                    "name": root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
                }
            ]
        });

        match handle.request("initialize", init_params).await {
            Ok(_) => {
                // Send initialized notification
                let _ = handle.notify("initialized", serde_json::json!({})).await;

                // Store handle
                let mut servers = self.servers.write().await;
                servers.insert(key.to_string(), Arc::new(RwLock::new(handle)));

                let mut statuses = self.statuses.write().await;
                statuses.insert(key.to_string(), LspServerStatus::Running);

                info!("LSP server started: {}", key);
            }
            Err(e) => {
                error!("Failed to initialize LSP server: {}", e);
                let mut statuses = self.statuses.write().await;
                statuses.insert(
                    key.to_string(),
                    LspServerStatus::Failed {
                        error: e.to_string(),
                    },
                );
                return Err(e);
            }
        }

        Ok(())
    }

    /// Send a request to a language server
    pub async fn request(&self, key: &str, method: &str, params: Value) -> Result<Value> {
        let servers = self.servers.read().await;
        let handle = servers
            .get(key)
            .ok_or_else(|| anyhow!("Server not found: {}", key))?;

        let server = handle.read().await;
        server.request(method, params).await
    }

    // ========================================================================
    // Convenience methods for common LSP operations
    // ========================================================================

    /// Get hover information at a position
    pub async fn hover(&self, key: &str, uri: &str, line: u32, character: u32) -> Result<Option<String>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        let result = self.request(key, "textDocument/hover", params).await?;

        // Extract the hover content
        if result.is_null() {
            return Ok(None);
        }

        let contents = result.get("contents");
        match contents {
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(Value::Object(obj)) => {
                // MarkupContent: { kind: "markdown" | "plaintext", value: string }
                obj.get("value")
                    .and_then(|v| v.as_str())
                    .map(|s| Some(s.to_string()))
                    .ok_or_else(|| anyhow!("Unexpected hover content format"))
            }
            Some(Value::Array(arr)) => {
                // Array of MarkedString or MarkupContent
                let content: Vec<String> = arr
                    .iter()
                    .filter_map(|v| match v {
                        Value::String(s) => Some(s.clone()),
                        Value::Object(obj) => obj.get("value")?.as_str().map(|s| s.to_string()),
                        _ => None,
                    })
                    .collect();
                Ok(Some(content.join("\n\n")))
            }
            _ => Ok(None),
        }
    }

    /// Go to definition
    pub async fn goto_definition(&self, key: &str, uri: &str, line: u32, character: u32) -> Result<Vec<LspLocation>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        let result = self.request(key, "textDocument/definition", params).await?;
        Self::parse_locations(result)
    }

    /// Go to type definition
    pub async fn goto_type_definition(&self, key: &str, uri: &str, line: u32, character: u32) -> Result<Vec<LspLocation>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        let result = self.request(key, "textDocument/typeDefinition", params).await?;
        Self::parse_locations(result)
    }

    /// Find references
    pub async fn find_references(&self, key: &str, uri: &str, line: u32, character: u32, include_declaration: bool) -> Result<Vec<LspLocation>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": include_declaration }
        });

        let result = self.request(key, "textDocument/references", params).await?;
        Self::parse_locations(result)
    }

    /// Get document symbols
    pub async fn document_symbols(&self, key: &str, uri: &str) -> Result<Vec<LspSymbol>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        let result = self.request(key, "textDocument/documentSymbol", params).await?;

        if result.is_null() {
            return Ok(Vec::new());
        }

        let symbols: Vec<LspSymbol> = result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let name = v.get("name")?.as_str()?.to_string();
                        let kind = v.get("kind")?.as_u64()? as u32;

                        // Try to get range from either range or location.range
                        let range = v
                            .get("range")
                            .or_else(|| v.get("location").and_then(|l| l.get("range")))?;

                        let start_line = range.get("start")?.get("line")?.as_u64()? as u32;
                        let start_char = range.get("start")?.get("character")?.as_u64()? as u32;
                        let end_line = range.get("end")?.get("line")?.as_u64()? as u32;
                        let end_char = range.get("end")?.get("character")?.as_u64()? as u32;

                        Some(LspSymbol {
                            name,
                            kind: Self::symbol_kind_name(kind),
                            start_line,
                            start_character: start_char,
                            end_line,
                            end_character: end_char,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(symbols)
    }

    /// Get completions at a position
    pub async fn completion(&self, key: &str, uri: &str, line: u32, character: u32) -> Result<Vec<LspCompletionItem>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        let result = self.request(key, "textDocument/completion", params).await?;

        if result.is_null() {
            return Ok(Vec::new());
        }

        // Result can be CompletionList or CompletionItem[]
        let items = result
            .get("items")
            .or(Some(&result))
            .and_then(|v| v.as_array());

        let completions: Vec<LspCompletionItem> = items
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let label = v.get("label")?.as_str()?.to_string();
                        let kind = v.get("kind").and_then(|k| k.as_u64()).map(|k| k as u32);
                        let detail = v.get("detail").and_then(|d| d.as_str()).map(|s| s.to_string());
                        let insert_text = v.get("insertText").and_then(|t| t.as_str()).map(|s| s.to_string());

                        Some(LspCompletionItem {
                            label,
                            kind: kind.map(Self::completion_kind_name),
                            detail,
                            insert_text,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(completions)
    }

    /// Format document
    pub async fn format_document(&self, key: &str, uri: &str, tab_size: u32, insert_spaces: bool) -> Result<Vec<LspTextEdit>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "options": {
                "tabSize": tab_size,
                "insertSpaces": insert_spaces
            }
        });

        let result = self.request(key, "textDocument/formatting", params).await?;
        Self::parse_text_edits(result)
    }

    /// Format document range
    pub async fn format_range(
        &self,
        key: &str,
        uri: &str,
        start_line: u32,
        start_character: u32,
        end_line: u32,
        end_character: u32,
        tab_size: u32,
        insert_spaces: bool,
    ) -> Result<Vec<LspTextEdit>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": start_line, "character": start_character },
                "end": { "line": end_line, "character": end_character }
            },
            "options": {
                "tabSize": tab_size,
                "insertSpaces": insert_spaces
            }
        });

        let result = self.request(key, "textDocument/rangeFormatting", params).await?;
        Self::parse_text_edits(result)
    }

    /// Open a text document (notify the server)
    pub async fn did_open(&self, key: &str, uri: &str, language_id: &str, version: u32, text: &str) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": version,
                "text": text
            }
        });

        self.notify(key, "textDocument/didOpen", params).await
    }

    /// Close a text document (notify the server)
    pub async fn did_close(&self, key: &str, uri: &str) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        self.notify(key, "textDocument/didClose", params).await
    }

    /// Notify document change
    pub async fn did_change(&self, key: &str, uri: &str, version: u32, text: &str) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "version": version
            },
            "contentChanges": [{ "text": text }]
        });

        self.notify(key, "textDocument/didChange", params).await
    }

    /// Send a notification to a language server
    pub async fn notify(&self, key: &str, method: &str, params: Value) -> Result<()> {
        let servers = self.servers.read().await;
        let handle = servers
            .get(key)
            .ok_or_else(|| anyhow!("Server not found: {}", key))?;

        let server = handle.read().await;
        server.notify(method, params).await
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    fn parse_locations(value: Value) -> Result<Vec<LspLocation>> {
        if value.is_null() {
            return Ok(Vec::new());
        }

        // Can be Location, Location[], or LocationLink[]
        let locations = if value.is_array() {
            value
                .as_array()
                .unwrap()
                .iter()
                .filter_map(Self::parse_single_location)
                .collect()
        } else if value.is_object() {
            Self::parse_single_location(&value)
                .map(|l| vec![l])
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(locations)
    }

    fn parse_single_location(value: &Value) -> Option<LspLocation> {
        // Handle both Location and LocationLink
        let uri = value
            .get("uri")
            .or_else(|| value.get("targetUri"))
            .and_then(|v| v.as_str())?
            .to_string();

        let range = value
            .get("range")
            .or_else(|| value.get("targetRange"))?;

        let start_line = range.get("start")?.get("line")?.as_u64()? as u32;
        let start_char = range.get("start")?.get("character")?.as_u64()? as u32;
        let end_line = range.get("end")?.get("line")?.as_u64()? as u32;
        let end_char = range.get("end")?.get("character")?.as_u64()? as u32;

        Some(LspLocation {
            uri,
            start_line,
            start_character: start_char,
            end_line,
            end_character: end_char,
        })
    }

    fn parse_text_edits(value: Value) -> Result<Vec<LspTextEdit>> {
        if value.is_null() {
            return Ok(Vec::new());
        }

        let edits = value
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let range = v.get("range")?;
                        let new_text = v.get("newText")?.as_str()?.to_string();

                        let start_line = range.get("start")?.get("line")?.as_u64()? as u32;
                        let start_char = range.get("start")?.get("character")?.as_u64()? as u32;
                        let end_line = range.get("end")?.get("line")?.as_u64()? as u32;
                        let end_char = range.get("end")?.get("character")?.as_u64()? as u32;

                        Some(LspTextEdit {
                            start_line,
                            start_character: start_char,
                            end_line,
                            end_character: end_char,
                            new_text,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(edits)
    }

    fn symbol_kind_name(kind: u32) -> String {
        match kind {
            1 => "File".to_string(),
            2 => "Module".to_string(),
            3 => "Namespace".to_string(),
            4 => "Package".to_string(),
            5 => "Class".to_string(),
            6 => "Method".to_string(),
            7 => "Property".to_string(),
            8 => "Field".to_string(),
            9 => "Constructor".to_string(),
            10 => "Enum".to_string(),
            11 => "Interface".to_string(),
            12 => "Function".to_string(),
            13 => "Variable".to_string(),
            14 => "Constant".to_string(),
            15 => "String".to_string(),
            16 => "Number".to_string(),
            17 => "Boolean".to_string(),
            18 => "Array".to_string(),
            19 => "Object".to_string(),
            20 => "Key".to_string(),
            21 => "Null".to_string(),
            22 => "EnumMember".to_string(),
            23 => "Struct".to_string(),
            24 => "Event".to_string(),
            25 => "Operator".to_string(),
            26 => "TypeParameter".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    fn completion_kind_name(kind: u32) -> String {
        match kind {
            1 => "Text".to_string(),
            2 => "Method".to_string(),
            3 => "Function".to_string(),
            4 => "Constructor".to_string(),
            5 => "Field".to_string(),
            6 => "Variable".to_string(),
            7 => "Class".to_string(),
            8 => "Interface".to_string(),
            9 => "Module".to_string(),
            10 => "Property".to_string(),
            11 => "Unit".to_string(),
            12 => "Value".to_string(),
            13 => "Enum".to_string(),
            14 => "Keyword".to_string(),
            15 => "Snippet".to_string(),
            16 => "Color".to_string(),
            17 => "File".to_string(),
            18 => "Reference".to_string(),
            19 => "Folder".to_string(),
            20 => "EnumMember".to_string(),
            21 => "Constant".to_string(),
            22 => "Struct".to_string(),
            23 => "Event".to_string(),
            24 => "Operator".to_string(),
            25 => "TypeParameter".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Stop a language server
    pub async fn stop(&self, key: &str) -> Result<()> {
        let mut servers = self.servers.write().await;
        if let Some(handle) = servers.remove(key) {
            let mut server = handle.write().await;
            server.stop()?;
        }

        let mut statuses = self.statuses.write().await;
        statuses.insert(key.to_string(), LspServerStatus::Stopped);

        info!("LSP server stopped: {}", key);
        Ok(())
    }

    /// Get status of a language server
    pub async fn status(&self, key: &str) -> LspServerStatus {
        let statuses = self.statuses.read().await;
        statuses
            .get(key)
            .cloned()
            .unwrap_or(LspServerStatus::NotFound)
    }

    /// Get all server statuses
    pub async fn all_statuses(&self) -> HashMap<String, LspServerStatus> {
        self.statuses.read().await.clone()
    }

    /// Stop all language servers
    pub async fn stop_all(&self) -> Result<()> {
        let mut servers = self.servers.write().await;
        for (key, handle) in servers.drain() {
            let mut server = handle.write().await;
            if let Err(e) = server.stop() {
                warn!("Failed to stop server {}: {}", key, e);
            }
        }

        let mut statuses = self.statuses.write().await;
        for (_, status) in statuses.iter_mut() {
            *status = LspServerStatus::Stopped;
        }

        Ok(())
    }

    /// Find project root for a file
    fn find_project_root(&self, file_path: &Path) -> Result<PathBuf> {
        let mut current = file_path.parent().unwrap_or(file_path).to_path_buf();

        let markers = [
            "package.json",
            "Cargo.toml",
            "go.mod",
            "pyproject.toml",
            "setup.py",
            ".git",
            "deno.json",
            "deno.jsonc",
            "tsconfig.json",
            "compile_commands.json",
        ];

        while current.parent().is_some() {
            for marker in &markers {
                if current.join(marker).exists() {
                    return Ok(current);
                }
            }
            if !current.pop() {
                break;
            }
        }

        // Fallback to file's directory
        Ok(file_path.parent().unwrap_or(file_path).to_path_buf())
    }
}

impl Default for LspServerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_server_status_serialization() {
        let status = LspServerStatus::Running;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("running"));

        let failed = LspServerStatus::Failed {
            error: "test".to_string(),
        };
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.contains("failed"));
    }

    #[test]
    fn test_lsp_server_manager_new() {
        let manager = LspServerManager::new();
        assert!(manager.servers.blocking_read().is_empty());
    }

    #[tokio::test]
    async fn test_lsp_server_manager_status() {
        let manager = LspServerManager::new();
        let status = manager.status("nonexistent").await;
        assert_eq!(status, LspServerStatus::NotFound);
    }

    #[test]
    fn test_find_project_root() {
        let manager = LspServerManager::new();
        let path = std::env::current_dir().unwrap().join("test.rs");
        let root = manager.find_project_root(&path);
        assert!(root.is_ok());
    }

    #[test]
    fn test_for_extension() {
        let root = PathBuf::from("/tmp");

        // These may return None if tools aren't installed
        let _ts = servers::for_extension("ts", &root);
        let _rs = servers::for_extension("rs", &root);
        let _unknown = servers::for_extension("xyz", &root);

        // Unknown extension should return None
        assert!(servers::for_extension("xyz", &root).is_none());
    }
}
