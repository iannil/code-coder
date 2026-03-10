//! IPC Server for zero-cli ↔ TypeScript TUI communication.
//!
//! Implements a Unix Domain Socket server that handles JSON-RPC 2.0 messages.
//! Supports multiple concurrent client connections.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};

use super::protocol::{
    create_notification, methods, AgentPromptParams, AgentPromptResult, AgentStreamEvent,
    AgentStreamNotification, CompactParams, CompactResult, GetSessionParams, IpcError, IpcRequest,
    IpcResponse, InitializeParams, InitializeResult, ListSessionsResult, ServerInfo, SessionInfo,
    SessionSummary, TokenUsage, ToolCallParams, ToolCallResult, ToolContent, ToolInfo,
    ToolResultParams,
};
use crate::config::Config;
use crate::session::store::SessionStore;
use crate::tools::{self, Tool};
use zero_core::agent::{AnthropicProvider, StreamEvent, StreamingProvider};
use zero_core::agent::streaming::{ContentPart, Message, Role, StreamRequest, ToolDef};

// ══════════════════════════════════════════════════════════════════════════════
// Client Connection State
// ══════════════════════════════════════════════════════════════════════════════

/// Pending tool result callback
type ToolResultCallback = oneshot::Sender<ToolResultParams>;

/// State for a single connected client
struct ClientState {
    /// Client ID (for logging)
    #[allow(dead_code)]
    id: String,
    /// Current session ID
    session_id: Option<String>,
    /// Sender for outgoing messages to this client
    tx: mpsc::Sender<String>,
    /// Pending tool call results (tool_call_id -> callback)
    pending_tool_results: HashMap<String, ToolResultCallback>,
    /// Current agent loop request ID (for cancellation)
    current_request_id: Option<String>,
    /// Cancellation channel for current agent loop
    cancel_tx: Option<broadcast::Sender<()>>,
}

// ══════════════════════════════════════════════════════════════════════════════
// IPC Server
// ══════════════════════════════════════════════════════════════════════════════

/// IPC Server state
pub struct IpcServer {
    /// Configuration
    config: Config,
    /// Session store
    sessions: Arc<SessionStore>,
    /// Available tools
    tools: Arc<Vec<Arc<dyn Tool>>>,
    /// Connected clients (client_id -> state)
    clients: Arc<RwLock<HashMap<String, ClientState>>>,
    /// Broadcast channel for server-wide notifications
    broadcast_tx: broadcast::Sender<String>,
    /// Shutdown signal
    shutdown_tx: broadcast::Sender<()>,
}

impl IpcServer {
    /// Create a new IPC server
    pub fn new(
        config: Config,
        sessions: Arc<SessionStore>,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(256);
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config,
            sessions,
            tools: Arc::new(tools),
            clients: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            shutdown_tx,
        }
    }

    /// Start serving on the given socket path
    pub async fn serve(self: Arc<Self>, socket_path: &Path) -> Result<()> {
        // Remove existing socket file if present
        if socket_path.exists() {
            std::fs::remove_file(socket_path)
                .with_context(|| format!("Failed to remove existing socket: {}", socket_path.display()))?;
        }

        // Create parent directory if needed
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create socket directory: {}", parent.display()))?;
        }

        let listener = UnixListener::bind(socket_path)
            .with_context(|| format!("Failed to bind to socket: {}", socket_path.display()))?;

        info!("IPC server listening on {}", socket_path.display());

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let server = Arc::clone(&self);
                            tokio::spawn(async move {
                                if let Err(e) = server.handle_client(stream).await {
                                    error!("Client handler error: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {e}");
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("IPC server shutting down");
                    break;
                }
            }
        }

        // Clean up socket file
        let _ = std::fs::remove_file(socket_path);

        Ok(())
    }

    /// Handle a single client connection
    async fn handle_client(self: Arc<Self>, stream: UnixStream) -> Result<()> {
        let client_id = uuid::Uuid::new_v4().to_string();
        info!("Client connected: {client_id}");

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Create channel for outgoing messages
        let (tx, mut rx) = mpsc::channel::<String>(64);

        // Register client
        {
            let mut clients = self.clients.write().await;
            clients.insert(
                client_id.clone(),
                ClientState {
                    id: client_id.clone(),
                    session_id: None,
                    tx: tx.clone(),
                    pending_tool_results: HashMap::new(),
                    current_request_id: None,
                    cancel_tx: None,
                },
            );
        }

        // Subscribe to broadcast messages
        let mut broadcast_rx = self.broadcast_tx.subscribe();

        // Spawn writer task
        let writer_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Messages from request handlers
                    Some(msg) = rx.recv() => {
                        if let Err(e) = writer.write_all(msg.as_bytes()).await {
                            error!("Failed to write to client: {e}");
                            break;
                        }
                        if let Err(e) = writer.write_all(b"\n").await {
                            error!("Failed to write newline: {e}");
                            break;
                        }
                        if let Err(e) = writer.flush().await {
                            error!("Failed to flush: {e}");
                            break;
                        }
                    }
                    // Broadcast messages
                    Ok(msg) = broadcast_rx.recv() => {
                        if let Err(e) = writer.write_all(msg.as_bytes()).await {
                            error!("Failed to write broadcast: {e}");
                            break;
                        }
                        if let Err(e) = writer.write_all(b"\n").await {
                            error!("Failed to write newline: {e}");
                            break;
                        }
                        if let Err(e) = writer.flush().await {
                            error!("Failed to flush: {e}");
                            break;
                        }
                    }
                    else => break,
                }
            }
        });

        // Read and process messages
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF - client disconnected
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<IpcRequest>(trimmed) {
                        Ok(request) => {
                            let response = self.handle_request(&client_id, request).await;
                            if let Some(response) = response {
                                let json = serde_json::to_string(&response)
                                    .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Serialization error"}}"#.to_string());
                                if tx.send(json).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            let response = IpcResponse::error(
                                None,
                                IpcError::parse_error(format!("Invalid JSON-RPC: {e}")),
                            );
                            let json = serde_json::to_string(&response).unwrap();
                            if tx.send(json).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Read error: {e}");
                    break;
                }
            }
        }

        // Cleanup
        writer_handle.abort();
        {
            let mut clients = self.clients.write().await;
            clients.remove(&client_id);
        }

        info!("Client disconnected: {client_id}");
        Ok(())
    }

    /// Handle a single request, returning a response (or None for notifications)
    async fn handle_request(&self, client_id: &str, request: IpcRequest) -> Option<IpcResponse> {
        let id = request.id.clone();

        // Notifications (no id) don't get responses
        if id.is_none() && request.method != methods::TOOL_RESULT {
            debug!("Ignoring notification: {}", request.method);
            return None;
        }

        let result = match request.method.as_str() {
            methods::PING => self.handle_ping(),
            methods::INITIALIZE => self.handle_initialize(client_id, request.params).await,
            methods::GET_SESSION => self.handle_get_session(request.params).await,
            methods::LIST_SESSIONS => self.handle_list_sessions().await,
            methods::COMPACT => self.handle_compact(request.params).await,
            methods::TOOL_CALL => self.handle_tool_call(client_id, request.params).await,
            methods::TOOL_RESULT => {
                self.handle_tool_result(client_id, request.params).await;
                return None; // No response for notifications
            }
            methods::CANCEL_GENERATION => self.handle_cancel_generation(client_id).await,
            methods::AGENT_PROMPT => self.handle_agent_prompt(client_id, request.params).await,
            _ => Err(IpcError::method_not_found(&request.method)),
        };

        match result {
            Ok(value) => id.map(|id| IpcResponse::success(id, value)),
            Err(error) => Some(IpcResponse::error(id, error)),
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Request Handlers
    // ──────────────────────────────────────────────────────────────────────────

    fn handle_ping(&self) -> Result<serde_json::Value, IpcError> {
        Ok(serde_json::json!({ "pong": true }))
    }

    async fn handle_initialize(
        &self,
        client_id: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, IpcError> {
        let params: InitializeParams = params
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| IpcError::invalid_params(e.to_string()))?
            .ok_or_else(|| IpcError::invalid_params("Missing params"))?;

        // Generate or use provided session ID
        let session_id = params
            .session_id
            .unwrap_or_else(|| format!("cli:{}", uuid::Uuid::new_v4()));

        // Update client state
        {
            let mut clients = self.clients.write().await;
            if let Some(client) = clients.get_mut(client_id) {
                client.session_id = Some(session_id.clone());
            }
        }

        // Get existing messages if resuming
        let messages = self
            .sessions
            .get_messages(&session_id)
            .map_err(|e| IpcError::internal_error(e.to_string()))?;

        // Build tool info
        let tools: Vec<ToolInfo> = self
            .tools
            .iter()
            .map(|t| ToolInfo {
                name: t.name().to_string(),
                description: Some(t.description().to_string()),
                input_schema: t.parameters_schema(),
            })
            .collect();

        let result = InitializeResult {
            session_id,
            server_info: ServerInfo {
                name: "zero-cli".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            tools,
            messages,
        };

        serde_json::to_value(result).map_err(|e| IpcError::internal_error(e.to_string()))
    }

    async fn handle_get_session(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, IpcError> {
        let params: GetSessionParams = params
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| IpcError::invalid_params(e.to_string()))?
            .ok_or_else(|| IpcError::invalid_params("Missing params"))?;

        let messages = self
            .sessions
            .get_messages(&params.session_id)
            .map_err(|e| IpcError::internal_error(e.to_string()))?;

        let token_count = self
            .sessions
            .get_token_count(&params.session_id)
            .map_err(|e| IpcError::internal_error(e.to_string()))?;

        let message_count = messages.len();

        let (created_at, updated_at) = if messages.is_empty() {
            let now = chrono::Local::now().timestamp();
            (now, now)
        } else {
            (
                messages.first().map(|m| m.timestamp).unwrap_or(0),
                messages.last().map(|m| m.timestamp).unwrap_or(0),
            )
        };

        let result = SessionInfo {
            session_id: params.session_id,
            messages,
            token_count,
            message_count,
            created_at,
            updated_at,
        };

        serde_json::to_value(result).map_err(|e| IpcError::internal_error(e.to_string()))
    }

    async fn handle_list_sessions(&self) -> Result<serde_json::Value, IpcError> {
        // Note: This is a simplified implementation. A full implementation would
        // query the database for all unique session keys.
        // For now, we return the sessions of connected clients.
        let clients = self.clients.read().await;
        let mut sessions = Vec::new();

        for client in clients.values() {
            if let Some(ref session_id) = client.session_id {
                let message_count = self
                    .sessions
                    .get_message_count(session_id)
                    .unwrap_or(0);
                let token_count = self
                    .sessions
                    .get_token_count(session_id)
                    .unwrap_or(0);

                sessions.push(SessionSummary {
                    session_id: session_id.clone(),
                    message_count,
                    token_count,
                    updated_at: chrono::Local::now().timestamp(),
                });
            }
        }

        let result = ListSessionsResult { sessions };
        serde_json::to_value(result).map_err(|e| IpcError::internal_error(e.to_string()))
    }

    async fn handle_compact(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, IpcError> {
        let params: CompactParams = params
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| IpcError::invalid_params(e.to_string()))?
            .ok_or_else(|| IpcError::invalid_params("Missing params"))?;

        // For now, just clear old messages. A full implementation would use
        // the compactor with LLM summarization.
        let deleted = self
            .sessions
            .compact_session(&params.session_id, "", 5)
            .map_err(|e| IpcError::internal_error(e.to_string()))?;

        let new_token_count = self
            .sessions
            .get_token_count(&params.session_id)
            .map_err(|e| IpcError::internal_error(e.to_string()))?;

        let result = CompactResult {
            deleted_count: deleted,
            new_token_count,
        };

        serde_json::to_value(result).map_err(|e| IpcError::internal_error(e.to_string()))
    }

    async fn handle_tool_call(
        &self,
        _client_id: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, IpcError> {
        let params: ToolCallParams = params
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| IpcError::invalid_params(e.to_string()))?
            .ok_or_else(|| IpcError::invalid_params("Missing params"))?;

        // Find the tool
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == params.name)
            .ok_or_else(|| IpcError::method_not_found(&params.name))?;

        // Execute the tool
        let args_value = serde_json::to_value(&params.args)
            .map_err(|e| IpcError::invalid_params(e.to_string()))?;

        match tool.execute(args_value).await {
            Ok(result) => {
                let call_result = ToolCallResult {
                    call_id: params.call_id,
                    content: vec![ToolContent::text(&result.output)],
                    is_error: !result.success,
                };
                serde_json::to_value(call_result)
                    .map_err(|e| IpcError::internal_error(e.to_string()))
            }
            Err(e) => {
                let call_result = ToolCallResult {
                    call_id: params.call_id,
                    content: vec![ToolContent::text(e.to_string())],
                    is_error: true,
                };
                serde_json::to_value(call_result)
                    .map_err(|e| IpcError::internal_error(e.to_string()))
            }
        }
    }

    async fn handle_tool_result(&self, client_id: &str, params: Option<serde_json::Value>) {
        if let Some(params) = params {
            if let Ok(result) = serde_json::from_value::<ToolResultParams>(params) {
                debug!(
                    "Received tool result for call_id={}: {:?}",
                    result.call_id, result.result
                );

                // Find and notify the pending tool callback
                let callback = {
                    let mut clients = self.clients.write().await;
                    if let Some(client) = clients.get_mut(client_id) {
                        client.pending_tool_results.remove(&result.call_id)
                    } else {
                        None
                    }
                };

                if let Some(tx) = callback {
                    let _ = tx.send(result);
                } else {
                    warn!("No pending callback for tool result: {}", result.call_id);
                }
            }
        }
    }

    async fn handle_cancel_generation(
        &self,
        client_id: &str,
    ) -> Result<serde_json::Value, IpcError> {
        info!("Cancel generation requested by client: {client_id}");

        // Send cancel signal to active agent loop
        let cancel_tx = {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(client_id) {
                client.cancel_tx.clone()
            } else {
                None
            }
        };

        if let Some(tx) = cancel_tx {
            let _ = tx.send(());
            Ok(serde_json::json!({ "cancelled": true }))
        } else {
            Ok(serde_json::json!({ "cancelled": false, "reason": "no_active_generation" }))
        }
    }

    /// Handle agent prompt request - starts LLM streaming with tool callbacks
    async fn handle_agent_prompt(
        &self,
        client_id: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, IpcError> {
        let params: AgentPromptParams = params
            .map(serde_json::from_value)
            .transpose()
            .map_err(|e| IpcError::invalid_params(e.to_string()))?
            .ok_or_else(|| IpcError::invalid_params("Missing params"))?;

        // Generate request ID
        let request_id = format!("req-{}", uuid::Uuid::new_v4());
        info!(
            "Agent prompt request: {} (session={}, agent={}, model={})",
            request_id, params.session_id, params.agent, params.model.model_id
        );

        // Set up cancellation
        let (cancel_tx, mut cancel_rx) = broadcast::channel::<()>(1);
        {
            let mut clients = self.clients.write().await;
            if let Some(client) = clients.get_mut(client_id) {
                client.current_request_id = Some(request_id.clone());
                client.cancel_tx = Some(cancel_tx);
            }
        }

        // Get API key from config or params
        let api_key = params
            .model
            .api_key
            .clone()
            .or_else(|| self.config.api_key.clone())
            .ok_or_else(|| IpcError::invalid_params("No API key configured"))?;

        // Create provider based on provider_id
        let provider: Box<dyn StreamingProvider> = match params.model.provider_id.as_str() {
            "anthropic" => {
                let mut p = AnthropicProvider::new(api_key);
                if let Some(ref base_url) = params.model.base_url {
                    p = p.with_base_url(base_url.clone());
                }
                Box::new(p)
            }
            other => {
                return Err(IpcError::invalid_params(format!(
                    "Unsupported provider: {}",
                    other
                )));
            }
        };

        // Build tool definitions
        let tool_defs: Vec<ToolDef> = self
            .tools
            .iter()
            .filter(|t| params.tools.is_empty() || params.tools.contains(&t.name().to_string()))
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.parameters_schema(),
            })
            .collect();

        // Convert messages
        let messages: Vec<Message> = params
            .messages
            .iter()
            .map(|m| {
                let role = match m.role.as_str() {
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    "system" => Role::System,
                    "tool" => Role::Tool,
                    _ => Role::User,
                };

                let content = if let Some(tool_call_id) = &m.tool_call_id {
                    vec![ContentPart::ToolResult {
                        tool_use_id: tool_call_id.clone(),
                        content: m.content.clone(),
                    }]
                } else if let Some(tool_calls) = &m.tool_calls {
                    tool_calls
                        .iter()
                        .map(|tc| ContentPart::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        })
                        .collect()
                } else {
                    vec![ContentPart::Text {
                        text: m.content.clone(),
                    }]
                };

                Message { role, content }
            })
            .collect();

        // Add user message
        let mut all_messages = messages;
        all_messages.push(Message::user(&params.message));

        // Create stream request
        let stream_request = StreamRequest {
            system: params.system.clone(),
            messages: all_messages,
            tools: tool_defs,
            model: params.model.model_id.clone(),
            temperature: Some(0.7),
            max_tokens: Some(32000),
        };

        // Get client tx for sending events
        let client_tx = {
            let clients = self.clients.read().await;
            clients.get(client_id).map(|c| c.tx.clone())
        }
        .ok_or_else(|| IpcError::internal_error("Client disconnected"))?;

        // Clone what we need for the async task
        let req_id = request_id.clone();

        // Spawn the streaming task
        tokio::spawn(async move {
            // Helper to send stream event
            let send_event = |event: AgentStreamEvent| {
                let notification = create_notification(
                    methods::AGENT_STREAM,
                    AgentStreamNotification {
                        request_id: req_id.clone(),
                        event,
                    },
                );
                let json = serde_json::to_string(&notification).unwrap_or_default();
                client_tx.send(json)
            };

            // Send start event
            if send_event(AgentStreamEvent::Start).await.is_err() {
                return;
            }

            // Start streaming
            match provider.stream(stream_request).await {
                Ok(mut stream) => {
                    loop {
                        tokio::select! {
                            _ = cancel_rx.recv() => {
                                info!("Agent loop cancelled for request: {}", req_id);
                                let _ = send_event(AgentStreamEvent::Finish {
                                    reason: "cancelled".into(),
                                    usage: None,
                                }).await;
                                break;
                            }
                            event = stream.next() => {
                                match event {
                                    Some(Ok(e)) => {
                                        // Convert StreamEvent to AgentStreamEvent
                                        let agent_event = match e {
                                            StreamEvent::Start => AgentStreamEvent::Start,
                                            StreamEvent::TextDelta { content } => {
                                                AgentStreamEvent::TextDelta { content }
                                            }
                                            StreamEvent::ReasoningDelta { content } => {
                                                AgentStreamEvent::ReasoningDelta { content }
                                            }
                                            StreamEvent::ToolCallStart { id, name } => {
                                                AgentStreamEvent::ToolCallStart { id, name }
                                            }
                                            StreamEvent::ToolCallDelta { id, arguments_delta } => {
                                                AgentStreamEvent::ToolCallDelta { id, arguments_delta }
                                            }
                                            StreamEvent::ToolCall { id, name, arguments } => {
                                                AgentStreamEvent::ToolCall { id, name, arguments }
                                            }
                                            StreamEvent::Finish { reason, usage } => {
                                                AgentStreamEvent::Finish {
                                                    reason,
                                                    usage: usage.map(|u| TokenUsage {
                                                        input_tokens: u.input_tokens,
                                                        output_tokens: u.output_tokens,
                                                        reasoning_tokens: u.reasoning_tokens,
                                                        cache_read_tokens: u.cache_read_tokens,
                                                        cache_write_tokens: u.cache_write_tokens,
                                                    }),
                                                }
                                            }
                                            StreamEvent::Error { code, message } => {
                                                AgentStreamEvent::Error { code, message }
                                            }
                                        };

                                        if send_event(agent_event).await.is_err() {
                                            break;
                                        }
                                    }
                                    Some(Err(e)) => {
                                        error!("Stream error: {}", e);
                                        let _ = send_event(AgentStreamEvent::Error {
                                            code: -1,
                                            message: e.to_string(),
                                        }).await;
                                        break;
                                    }
                                    None => {
                                        // Stream ended
                                        let _ = send_event(AgentStreamEvent::Finish {
                                            reason: "stop".into(),
                                            usage: None,
                                        }).await;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to start stream: {}", e);
                    let _ = send_event(AgentStreamEvent::Error {
                        code: -1,
                        message: e.to_string(),
                    })
                    .await;
                }
            }
        });

        // Return immediate response with request_id
        let result = AgentPromptResult {
            request_id,
            streaming: true,
        };

        serde_json::to_value(result).map_err(|e| IpcError::internal_error(e.to_string()))
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Notification Helpers
    // ──────────────────────────────────────────────────────────────────────────

    /// Send a notification to a specific client
    pub async fn notify_client(&self, client_id: &str, notification: IpcRequest) -> Result<()> {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(client_id) {
            let json = serde_json::to_string(&notification)?;
            client.tx.send(json).await?;
        }
        Ok(())
    }

    /// Broadcast a notification to all clients
    pub fn broadcast(&self, notification: IpcRequest) -> Result<()> {
        let json = serde_json::to_string(&notification)?;
        let _ = self.broadcast_tx.send(json);
        Ok(())
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Public API
// ══════════════════════════════════════════════════════════════════════════════

/// Serve the IPC server on the given socket path
pub async fn serve(config: Config, socket_path: PathBuf) -> Result<()> {
    // Initialize session store
    let db_path = config.workspace_dir.join("sessions.db");
    let sessions = Arc::new(
        SessionStore::new(&db_path)
            .with_context(|| format!("Failed to open session store: {}", db_path.display()))?,
    );

    // Build tools
    let security = Arc::new(crate::security::SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));

    let mem: Arc<dyn crate::memory::Memory> = Arc::from(
        crate::memory::create_memory(
            &config.memory,
            &config.workspace_dir,
            config.api_key.as_deref(),
        )?,
    );

    let tools_vec = tools::all_tools(
        &security,
        mem,
        &config.browser,
        &config.codecoder,
        &config.vault,
        &config.workspace_dir,
    );

    let tools: Vec<Arc<dyn Tool>> = tools_vec
        .into_iter()
        .map(|t| Arc::from(t) as Arc<dyn Tool>)
        .collect();

    // Create and run server
    let server = Arc::new(IpcServer::new(config, sessions, tools));

    // Handle shutdown signals
    let server_clone = Arc::clone(&server);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("Received shutdown signal");
        server_clone.shutdown();
    });

    server.serve(&socket_path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_server_bind_and_cleanup() {
        let tmp = TempDir::new().unwrap();
        let socket_path = tmp.path().join("test.sock");

        // Verify socket doesn't exist
        assert!(!socket_path.exists());

        // Create a minimal config for testing
        // Note: This test is limited without full config setup
    }
}
