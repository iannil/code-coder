//! WebSocket Handler for Unified API
//!
//! Provides real-time streaming for agent execution, tool calls, and observer events.
//! This integrates directly with `UnifiedApiState` to work with the daemon's unified API.
//!
//! ## Protocol
//!
//! All messages are JSON with a `type` field discriminator.
//!
//! ### Client → Server Messages
//! - `ping` - Keepalive
//! - `chat` - Send a chat message to an agent
//! - `cancel` - Cancel an ongoing agent execution
//! - `tool_call` - Execute a tool directly
//! - `confirmation` - Respond to a confirmation request
//! - `subscribe_observer` - Subscribe to observer events
//!
//! ### Server → Client Messages
//! - `pong` - Keepalive response
//! - `connected` - Connection established
//! - `text_delta` - Streaming text content
//! - `reasoning_delta` - Streaming reasoning content
//! - `tool_start` - Tool execution started
//! - `tool_result` - Tool execution completed
//! - `confirmation_required` - Tool needs user approval
//! - `complete` - Agent execution completed
//! - `error` - Error occurred
//! - `observer_event` - Observer network event

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::session::types::MessageRole;

use super::state::{ContentPart, Message as LlmMessage, Role, StreamEvent, StreamRequest, ToolDef, UnifiedApiState};

// ══════════════════════════════════════════════════════════════════════════════
// Client → Server Messages
// ══════════════════════════════════════════════════════════════════════════════

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Ping for keepalive
    #[serde(rename = "ping")]
    Ping,

    /// Chat message to an agent
    #[serde(rename = "chat")]
    Chat {
        /// Unique request ID for correlation
        id: String,
        /// Session ID
        session_id: String,
        /// Agent name (e.g., "build", "plan")
        #[serde(default = "default_agent")]
        agent: String,
        /// User message content
        message: String,
        /// Additional system prompts
        #[serde(default)]
        system: Vec<String>,
        /// Temperature override
        temperature: Option<f64>,
        /// Max tokens
        max_tokens: Option<usize>,
        /// Model override
        model: Option<String>,
    },

    /// Cancel an ongoing agent execution
    #[serde(rename = "cancel")]
    Cancel {
        /// Request ID to cancel
        id: String,
    },

    /// Direct tool execution
    #[serde(rename = "tool_call")]
    ToolCall {
        /// Request ID
        id: String,
        /// Tool name
        tool: String,
        /// Tool parameters
        params: Value,
    },

    /// Confirmation response (for HITL)
    #[serde(rename = "confirmation")]
    Confirmation {
        /// Confirmation request ID
        id: String,
        /// User's decision
        approved: bool,
        /// Optional user comment
        comment: Option<String>,
    },

    /// Subscribe to observer events
    #[serde(rename = "subscribe_observer")]
    SubscribeObserver,

    /// Unsubscribe from observer events
    #[serde(rename = "unsubscribe_observer")]
    UnsubscribeObserver,

    /// Subscribe to session events
    #[serde(rename = "subscribe_session")]
    SubscribeSession {
        session_id: String,
    },

    /// Unsubscribe from session events
    #[serde(rename = "unsubscribe_session")]
    UnsubscribeSession {
        session_id: String,
    },
}

fn default_agent() -> String {
    "build".to_string()
}

// ══════════════════════════════════════════════════════════════════════════════
// Server → Client Messages
// ══════════════════════════════════════════════════════════════════════════════

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Pong response
    #[serde(rename = "pong")]
    Pong,

    /// Connection established
    #[serde(rename = "connected")]
    Connected {
        /// Connection ID
        connection_id: String,
        /// Server version
        version: String,
    },

    /// Chat started acknowledgment
    #[serde(rename = "chat_start")]
    ChatStart {
        /// Request ID
        id: String,
    },

    /// Text content delta (streaming)
    #[serde(rename = "text_delta")]
    TextDelta {
        /// Request ID
        id: String,
        /// Text content
        content: String,
    },

    /// Reasoning content delta (extended thinking)
    #[serde(rename = "reasoning_delta")]
    ReasoningDelta {
        /// Request ID
        id: String,
        /// Reasoning content
        content: String,
    },

    /// Tool call started
    #[serde(rename = "tool_start")]
    ToolStart {
        /// Request ID
        id: String,
        /// Tool call ID
        tool_call_id: String,
        /// Tool name
        tool: String,
        /// Tool arguments
        arguments: Value,
    },

    /// Tool execution result
    #[serde(rename = "tool_result")]
    ToolResult {
        /// Request ID
        id: String,
        /// Tool call ID
        tool_call_id: String,
        /// Output (if success)
        output: Option<String>,
        /// Error (if failed)
        error: Option<String>,
    },

    /// Confirmation required (HITL)
    #[serde(rename = "confirmation_required")]
    ConfirmationRequired {
        /// Confirmation ID
        id: String,
        /// Request ID (parent)
        request_id: String,
        /// Tool name
        tool: String,
        /// Tool arguments
        arguments: Value,
        /// Risk level
        risk_level: String,
        /// Reason for confirmation
        reason: String,
    },

    /// Chat completed
    #[serde(rename = "complete")]
    Complete {
        /// Request ID
        id: String,
        /// Stop reason
        reason: String,
        /// Token usage
        usage: Option<TokenUsage>,
    },

    /// Chat cancelled
    #[serde(rename = "cancelled")]
    Cancelled {
        /// Request ID
        id: String,
    },

    /// Tool response (direct tool call)
    #[serde(rename = "tool_response")]
    ToolResponse {
        /// Request ID
        id: String,
        /// Success flag
        success: bool,
        /// Result value
        result: Option<Value>,
        /// Error message
        error: Option<String>,
    },

    /// Observer event
    #[serde(rename = "observer_event")]
    ObserverEvent {
        /// Event type
        event_type: String,
        /// Event data
        data: Value,
    },

    /// Session event
    #[serde(rename = "session_event")]
    SessionEvent {
        /// Session ID
        session_id: String,
        /// Event type
        event_type: String,
        /// Event data
        data: Value,
    },

    /// Error
    #[serde(rename = "error")]
    Error {
        /// Error code
        code: i32,
        /// Error message
        message: String,
        /// Request ID (if applicable)
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    #[serde(default)]
    pub reasoning_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<usize>,
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            cache_read_tokens: None,
            cache_write_tokens: None,
        }
    }
}

impl TokenUsage {
    fn merge(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.reasoning_tokens += other.reasoning_tokens;
        if let Some(v) = other.cache_read_tokens {
            *self.cache_read_tokens.get_or_insert(0) += v;
        }
        if let Some(v) = other.cache_write_tokens {
            *self.cache_write_tokens.get_or_insert(0) += v;
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Connection State
// ══════════════════════════════════════════════════════════════════════════════

/// Per-connection state
struct ConnectionState {
    /// Connection ID
    id: String,
    /// Subscribed sessions
    subscribed_sessions: HashSet<String>,
    /// Observer subscription
    observer_subscribed: bool,
    /// Active agent executions (request_id -> cancel sender)
    active_agents: HashMap<String, mpsc::Sender<()>>,
}

// ══════════════════════════════════════════════════════════════════════════════
// WebSocket Handler
// ══════════════════════════════════════════════════════════════════════════════

/// Handle WebSocket upgrade request
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<UnifiedApiState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<UnifiedApiState>) {
    let (mut sender, mut receiver) = socket.split();

    // Generate connection ID
    let connection_id = format!("ws-{}", Uuid::new_v4());

    // Initialize connection state
    let mut conn_state = ConnectionState {
        id: connection_id.clone(),
        subscribed_sessions: HashSet::new(),
        observer_subscribed: false,
        active_agents: HashMap::new(),
    };

    tracing::info!(connection_id = %conn_state.id, "WebSocket connection established");

    // Send connected message
    let connected_msg = ServerMessage::Connected {
        connection_id: conn_state.id.clone(),
        version: super::VERSION.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // Create channel for server-initiated messages
    let (server_tx, mut server_rx) = mpsc::channel::<ServerMessage>(100);

    // Main message loop
    loop {
        tokio::select! {
            // Handle incoming client messages
            msg = receiver.next() => {
                let text = match msg {
                    Some(Ok(Message::Text(text))) => text,
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!(connection_id = %conn_state.id, "WebSocket closed by client");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                        continue;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(connection_id = %conn_state.id, error = %e, "WebSocket error");
                        break;
                    }
                    None => break,
                    _ => continue,
                };

                // Parse and handle message
                let response = match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        handle_client_message(
                            &state,
                            &mut conn_state,
                            client_msg,
                            server_tx.clone(),
                        ).await
                    }
                    Err(e) => {
                        Some(ServerMessage::Error {
                            code: -1,
                            message: format!("Failed to parse message: {}", e),
                            request_id: None,
                        })
                    }
                };

                if let Some(response) = response {
                    if let Ok(json) = serde_json::to_string(&response) {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }

            // Handle server-initiated messages
            Some(server_msg) = server_rx.recv() => {
                if let Ok(json) = serde_json::to_string(&server_msg) {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    // Cleanup: cancel all active agents
    for (_, cancel_tx) in conn_state.active_agents.drain() {
        let _ = cancel_tx.send(()).await;
    }

    tracing::info!(connection_id = %conn_state.id, "WebSocket connection ended");
}

/// Handle a client message
async fn handle_client_message(
    state: &Arc<UnifiedApiState>,
    conn_state: &mut ConnectionState,
    msg: ClientMessage,
    server_tx: mpsc::Sender<ServerMessage>,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Ping => Some(ServerMessage::Pong),

        ClientMessage::Chat {
            id,
            session_id,
            agent,
            message,
            system,
            temperature,
            max_tokens,
            model,
        } => {
            // Create cancellation channel
            let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
            conn_state.active_agents.insert(id.clone(), cancel_tx);

            // Spawn agent execution task
            let state_clone = Arc::clone(state);
            let server_tx_clone = server_tx.clone();
            let request_id = id.clone();

            tokio::spawn(async move {
                execute_chat(
                    state_clone,
                    server_tx_clone,
                    cancel_rx,
                    request_id,
                    session_id,
                    agent,
                    message,
                    system,
                    temperature,
                    max_tokens,
                    model,
                )
                .await;
            });

            // Send immediate start acknowledgment
            Some(ServerMessage::ChatStart { id })
        }

        ClientMessage::Cancel { id } => {
            if let Some(cancel_tx) = conn_state.active_agents.remove(&id) {
                let _ = cancel_tx.send(()).await;
                Some(ServerMessage::Cancelled { id })
            } else {
                Some(ServerMessage::Error {
                    code: -2,
                    message: format!("No active chat with id: {}", id),
                    request_id: Some(id),
                })
            }
        }

        ClientMessage::ToolCall { id, tool, params } => {
            let result = execute_tool(state, &tool, params).await;
            Some(ServerMessage::ToolResponse {
                id,
                success: result.0,
                result: result.1,
                error: result.2,
            })
        }

        ClientMessage::Confirmation { id, approved, comment } => {
            tracing::debug!(
                confirmation_id = %id,
                approved = approved,
                comment = ?comment,
                "Received confirmation response"
            );

            // Forward to confirmation registry
            let handled = zero_core::agent::handle_confirmation_response(&id, approved).await;
            if !handled {
                return Some(ServerMessage::Error {
                    code: -3,
                    message: format!("Confirmation not found or already handled: {}", id),
                    request_id: Some(id),
                });
            }

            None
        }

        ClientMessage::SubscribeObserver => {
            conn_state.observer_subscribed = true;
            tracing::debug!(connection_id = %conn_state.id, "Subscribed to observer");
            None
        }

        ClientMessage::UnsubscribeObserver => {
            conn_state.observer_subscribed = false;
            None
        }

        ClientMessage::SubscribeSession { session_id } => {
            conn_state.subscribed_sessions.insert(session_id.clone());
            tracing::debug!(
                connection_id = %conn_state.id,
                session_id = %session_id,
                "Subscribed to session"
            );
            None
        }

        ClientMessage::UnsubscribeSession { session_id } => {
            conn_state.subscribed_sessions.remove(&session_id);
            None
        }
    }
}

/// Execute chat with agent (streaming)
async fn execute_chat(
    state: Arc<UnifiedApiState>,
    server_tx: mpsc::Sender<ServerMessage>,
    mut cancel_rx: mpsc::Receiver<()>,
    request_id: String,
    session_id: String,
    agent_name: String,
    message: String,
    system: Vec<String>,
    temperature: Option<f64>,
    max_tokens: Option<usize>,
    model: Option<String>,
) {
    // Get agent metadata
    let agent = state.get_agent(&agent_name).await;
    if agent.is_none() {
        let _ = server_tx
            .send(ServerMessage::Error {
                code: -4,
                message: format!("Agent not found: {}", agent_name),
                request_id: Some(request_id),
            })
            .await;
        return;
    }
    let agent = agent.unwrap();

    // Get LLM provider
    let provider = match &state.llm_provider {
        Some(p) => Arc::clone(p),
        None => {
            let _ = server_tx
                .send(ServerMessage::Error {
                    code: -5,
                    message: "LLM provider not configured".to_string(),
                    request_id: Some(request_id),
                })
                .await;
            return;
        }
    };

    // Build system prompts
    let mut system_prompts = vec![];
    if let Some(ref prompt) = agent.prompt {
        system_prompts.push(prompt.clone());
    }
    system_prompts.extend(system);

    // Get session history
    let session_messages = state.sessions.get_messages(&session_id).unwrap_or_default();

    let mut messages: Vec<LlmMessage> = session_messages
        .iter()
        .map(|m| {
            let role = match m.role {
                MessageRole::User => Role::User,
                MessageRole::Assistant => Role::Assistant,
                MessageRole::System => Role::System,
            };
            LlmMessage {
                role,
                content: vec![ContentPart::Text {
                    text: m.content.clone(),
                }],
            }
        })
        .collect();

    // Add new user message
    messages.push(LlmMessage::user(&message));

    // Get tools
    let tools: Vec<ToolDef> = {
        let registry = state.tools.read().await;
        registry
            .native_tools()
            .iter()
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.parameters_schema(),
            })
            .collect()
    };

    // Build request
    let model_str = model.unwrap_or_else(|| "claude-sonnet-4-5-20250514".to_string());
    let temp = temperature.or(agent.temperature);

    let mut request = StreamRequest {
        system: system_prompts,
        messages,
        tools,
        model: model_str,
        temperature: temp,
        max_tokens,
    };

    // Store user message in session
    let _ = state.sessions.add_message(&session_id, MessageRole::User, &message);

    // Agent loop
    let max_iterations = 10;
    let tool_timeout = Duration::from_secs(30);
    let mut iterations = 0;
    let mut full_text = String::new();
    let mut total_usage = TokenUsage::default();

    'agent_loop: loop {
        iterations += 1;
        if iterations > max_iterations {
            let _ = server_tx
                .send(ServerMessage::Error {
                    code: -6,
                    message: format!("Max iterations ({}) exceeded", max_iterations),
                    request_id: Some(request_id.clone()),
                })
                .await;
            break;
        }

        // Start stream
        let mut event_stream = match provider.stream(request.clone()).await {
            Ok(s) => s,
            Err(e) => {
                let _ = server_tx
                    .send(ServerMessage::Error {
                        code: -7,
                        message: format!("Failed to start stream: {}", e),
                        request_id: Some(request_id.clone()),
                    })
                    .await;
                break;
            }
        };

        let mut iteration_text = String::new();
        let mut tool_calls: Vec<(String, String, Value)> = vec![];
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();

        loop {
            tokio::select! {
                // Check for cancellation
                _ = cancel_rx.recv() => {
                    let _ = server_tx
                        .send(ServerMessage::Cancelled {
                            id: request_id.clone(),
                        })
                        .await;
                    break 'agent_loop;
                }

                // Process stream events
                event = event_stream.next() => {
                    match event {
                        Some(Ok(stream_event)) => {
                            match stream_event {
                                StreamEvent::Start => {}

                                StreamEvent::TextDelta { content } => {
                                    iteration_text.push_str(&content);
                                    let _ = server_tx
                                        .send(ServerMessage::TextDelta {
                                            id: request_id.clone(),
                                            content,
                                        })
                                        .await;
                                }

                                StreamEvent::ReasoningDelta { content } => {
                                    let _ = server_tx
                                        .send(ServerMessage::ReasoningDelta {
                                            id: request_id.clone(),
                                            content,
                                        })
                                        .await;
                                }

                                StreamEvent::ToolCallStart { id, name } => {
                                    current_tool_id = id;
                                    current_tool_name = name;
                                    current_tool_args.clear();
                                }

                                StreamEvent::ToolCallDelta { arguments_delta, .. } => {
                                    current_tool_args.push_str(&arguments_delta);
                                }

                                StreamEvent::ToolCall { id, name, arguments } => {
                                    tool_calls.push((id.clone(), name.clone(), arguments.clone()));
                                    let _ = server_tx
                                        .send(ServerMessage::ToolStart {
                                            id: request_id.clone(),
                                            tool_call_id: id,
                                            tool: name,
                                            arguments,
                                        })
                                        .await;
                                }

                                StreamEvent::Finish { reason, usage } => {
                                    // Finalize accumulated tool call
                                    if !current_tool_id.is_empty() && !current_tool_name.is_empty() {
                                        let arguments: Value =
                                            serde_json::from_str(&current_tool_args).unwrap_or_default();
                                        tool_calls.push((
                                            current_tool_id.clone(),
                                            current_tool_name.clone(),
                                            arguments.clone(),
                                        ));
                                        let _ = server_tx
                                            .send(ServerMessage::ToolStart {
                                                id: request_id.clone(),
                                                tool_call_id: current_tool_id.clone(),
                                                tool: current_tool_name.clone(),
                                                arguments,
                                            })
                                            .await;
                                        current_tool_id.clear();
                                        current_tool_name.clear();
                                        current_tool_args.clear();
                                    }

                                    if let Some(u) = usage {
                                        let usage = TokenUsage {
                                            input_tokens: u.input_tokens,
                                            output_tokens: u.output_tokens,
                                            reasoning_tokens: u.reasoning_tokens,
                                            cache_read_tokens: u.cache_read_tokens,
                                            cache_write_tokens: u.cache_write_tokens,
                                        };
                                        total_usage.merge(&usage);
                                    }

                                    // Don't send finish if we have tool calls to execute
                                    if tool_calls.is_empty() {
                                        let _ = server_tx
                                            .send(ServerMessage::Complete {
                                                id: request_id.clone(),
                                                reason,
                                                usage: Some(total_usage.clone()),
                                            })
                                            .await;
                                    }
                                    break;
                                }

                                StreamEvent::Error { code, message: msg } => {
                                    let _ = server_tx
                                        .send(ServerMessage::Error {
                                            code,
                                            message: msg,
                                            request_id: Some(request_id.clone()),
                                        })
                                        .await;
                                    break 'agent_loop;
                                }
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("Stream error: {}", e);
                        }
                        None => break,
                    }
                }
            }
        }

        // Accumulate text
        full_text.push_str(&iteration_text);

        // If no tool calls, we're done
        if tool_calls.is_empty() {
            break;
        }

        // Add assistant message with tool uses
        let mut assistant_content = Vec::new();
        if !iteration_text.is_empty() {
            assistant_content.push(ContentPart::Text { text: iteration_text });
        }
        for (id, name, input) in &tool_calls {
            assistant_content.push(ContentPart::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            });
        }
        request.messages.push(LlmMessage {
            role: Role::Assistant,
            content: assistant_content,
        });

        // Execute tools
        for (tool_call_id, tool_name, arguments) in tool_calls {
            let result = {
                let registry = state.tools.read().await;
                if let Some(tool) = registry.get_tool(&tool_name).await {
                    match tokio::time::timeout(tool_timeout, tool.execute(arguments)).await {
                        Ok(Ok(r)) => {
                            if r.success {
                                (Some(r.output), None)
                            } else {
                                (None, Some(r.error.unwrap_or_default()))
                            }
                        }
                        Ok(Err(e)) => (None, Some(e.to_string())),
                        Err(_) => (None, Some("Tool execution timed out".to_string())),
                    }
                } else {
                    (None, Some(format!("Unknown tool: {}", tool_name)))
                }
            };

            // Send tool result
            let _ = server_tx
                .send(ServerMessage::ToolResult {
                    id: request_id.clone(),
                    tool_call_id: tool_call_id.clone(),
                    output: result.0.clone(),
                    error: result.1.clone(),
                })
                .await;

            // Add to messages
            let result_str = result.0.unwrap_or_else(|| {
                format!("Error: {}", result.1.unwrap_or_default())
            });
            request.messages.push(LlmMessage::tool_result(&tool_call_id, &result_str));
        }
    }

    // Store assistant response in session
    if !full_text.is_empty() {
        let _ = state.sessions.add_message(&session_id, MessageRole::Assistant, &full_text);
    }
}

/// Execute a direct tool call
async fn execute_tool(
    state: &UnifiedApiState,
    tool_name: &str,
    params: Value,
) -> (bool, Option<Value>, Option<String>) {
    let registry = state.tools.read().await;

    if let Some(tool) = registry.get_tool(tool_name).await {
        match tokio::time::timeout(
            Duration::from_secs(30),
            tool.execute(params),
        ).await {
            Ok(Ok(result)) => {
                if result.success {
                    let value = serde_json::from_str(&result.output).ok();
                    (true, value, None)
                } else {
                    (false, None, result.error)
                }
            }
            Ok(Err(e)) => (false, None, Some(e.to_string())),
            Err(_) => (false, None, Some("Tool execution timed out".to_string())),
        }
    } else {
        (false, None, Some(format!("Unknown tool: {}", tool_name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::Chat {
            id: "test-1".to_string(),
            session_id: "sess-1".to_string(),
            agent: "build".to_string(),
            message: "Hello".to_string(),
            system: vec![],
            temperature: None,
            max_tokens: None,
            model: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"chat\""));
        assert!(json.contains("\"agent\":\"build\""));
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::TextDelta {
            id: "test-1".to_string(),
            content: "Hello, world!".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"text_delta\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));
    }

    #[test]
    fn test_token_usage_merge() {
        let mut usage = TokenUsage::default();
        let other = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            reasoning_tokens: 10,
            cache_read_tokens: Some(20),
            cache_write_tokens: None,
        };
        usage.merge(&other);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.reasoning_tokens, 10);
        assert_eq!(usage.cache_read_tokens, Some(20));
    }

    #[test]
    fn test_parse_client_message() {
        let json = r#"{"type":"chat","id":"1","session_id":"s1","agent":"build","message":"hi"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Chat { id, agent, message, .. } => {
                assert_eq!(id, "1");
                assert_eq!(agent, "build");
                assert_eq!(message, "hi");
            }
            _ => panic!("Expected Chat message"),
        }
    }
}
