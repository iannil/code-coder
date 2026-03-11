//! WebSocket handler for real-time bidirectional communication
//!
//! This module provides a comprehensive WebSocket API for:
//! - Agent execution with streaming
//! - Tool requests/responses
//! - Session management
//! - Real-time observer events
//! - User interrupts and confirmations
//!
//! ## Protocol
//!
//! All messages are JSON with a `type` field discriminator.
//! Client → Server messages have `_request` suffix.
//! Server → Client messages have `_response` or event-specific names.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use super::super::state::AppState;

// ══════════════════════════════════════════════════════════════════════════════
// Client → Server Messages
// ══════════════════════════════════════════════════════════════════════════════

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    /// Ping for keepalive
    #[serde(rename = "ping")]
    Ping,

    /// Agent execution request
    #[serde(rename = "agent_request")]
    AgentRequest {
        /// Unique request ID (for correlation)
        id: String,
        /// Session ID
        session_id: String,
        /// Agent name (e.g., "build", "plan")
        agent: String,
        /// User message
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
    #[serde(rename = "agent_cancel")]
    AgentCancel {
        /// Request ID to cancel
        id: String,
    },

    /// Tool execution request (direct tool call)
    #[serde(rename = "tool_request")]
    ToolRequest {
        id: String,
        tool: String,
        params: Value,
    },

    /// User confirmation response (for tool confirmations)
    #[serde(rename = "confirmation_response")]
    ConfirmationResponse {
        /// Confirmation request ID
        id: String,
        /// User's decision
        approved: bool,
        /// Optional user comment
        comment: Option<String>,
    },

    /// Subscribe to session events
    #[serde(rename = "session_subscribe")]
    SessionSubscribe {
        session_id: String,
    },

    /// Unsubscribe from session events
    #[serde(rename = "session_unsubscribe")]
    SessionUnsubscribe {
        session_id: String,
    },

    /// Subscribe to observer events
    #[serde(rename = "observer_subscribe")]
    ObserverSubscribe,

    /// Unsubscribe from observer events
    #[serde(rename = "observer_unsubscribe")]
    ObserverUnsubscribe,
}

// ══════════════════════════════════════════════════════════════════════════════
// Server → Client Messages
// ══════════════════════════════════════════════════════════════════════════════

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    /// Pong response
    #[serde(rename = "pong")]
    Pong,

    /// Connection established acknowledgment
    #[serde(rename = "connected")]
    Connected {
        /// Connection ID
        connection_id: String,
        /// Server version
        version: String,
    },

    /// Agent execution started
    #[serde(rename = "agent_start")]
    AgentStart {
        /// Request ID
        id: String,
    },

    /// Agent text content (streaming)
    #[serde(rename = "agent_text")]
    AgentText {
        /// Request ID
        id: String,
        /// Text content delta
        content: String,
    },

    /// Agent reasoning/thinking (streaming)
    #[serde(rename = "agent_reasoning")]
    AgentReasoning {
        /// Request ID
        id: String,
        /// Reasoning content delta
        content: String,
    },

    /// Tool call initiated by agent
    #[serde(rename = "agent_tool_call")]
    AgentToolCall {
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
    #[serde(rename = "agent_tool_result")]
    AgentToolResult {
        /// Request ID
        id: String,
        /// Tool call ID
        tool_call_id: String,
        /// Tool output (if success)
        output: Option<String>,
        /// Error message (if failed)
        error: Option<String>,
    },

    /// Agent execution completed
    #[serde(rename = "agent_complete")]
    AgentComplete {
        /// Request ID
        id: String,
        /// Stop reason
        reason: String,
        /// Token usage
        usage: Option<TokenUsage>,
    },

    /// Agent execution error
    #[serde(rename = "agent_error")]
    AgentError {
        /// Request ID
        id: String,
        /// Error code
        code: i32,
        /// Error message
        message: String,
    },

    /// Agent execution cancelled
    #[serde(rename = "agent_cancelled")]
    AgentCancelled {
        /// Request ID
        id: String,
    },

    /// Tool execution response (for direct tool requests)
    #[serde(rename = "tool_response")]
    ToolResponse {
        id: String,
        success: bool,
        result: Option<Value>,
        error: Option<String>,
    },

    /// Confirmation request (tool needs user approval)
    #[serde(rename = "confirmation_request")]
    ConfirmationRequest {
        /// Confirmation ID
        id: String,
        /// Tool name
        tool: String,
        /// Tool arguments
        arguments: Value,
        /// Risk level
        risk_level: String,
        /// Explanation of why confirmation is needed
        reason: String,
    },

    /// Session event
    #[serde(rename = "session_event")]
    SessionEvent {
        session_id: String,
        event_type: String,
        data: Value,
    },

    /// Observer event
    #[serde(rename = "observer_event")]
    ObserverEvent {
        event_type: String,
        data: Value,
    },

    /// General error
    #[serde(rename = "error")]
    Error {
        /// Error code
        code: i32,
        /// Error message
        message: String,
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

// ══════════════════════════════════════════════════════════════════════════════
// Legacy Compatibility (for existing clients)
// ══════════════════════════════════════════════════════════════════════════════

/// Legacy WebSocket message types (kept for backwards compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Ping/pong for keepalive
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,

    /// Tool execution request
    #[serde(rename = "tool_request")]
    ToolRequest {
        id: String,
        tool: String,
        params: Value,
    },

    /// Tool execution response
    #[serde(rename = "tool_response")]
    ToolResponse {
        id: String,
        success: bool,
        result: Option<Value>,
        error: Option<String>,
    },

    /// Streaming content
    #[serde(rename = "stream")]
    Stream {
        id: String,
        content: String,
        done: bool,
    },

    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
}

// ══════════════════════════════════════════════════════════════════════════════
// Connection State
// ══════════════════════════════════════════════════════════════════════════════

/// Per-connection state
struct ConnectionState {
    /// Connection ID
    id: String,
    /// Subscribed sessions
    subscribed_sessions: std::collections::HashSet<String>,
    /// Observer subscription
    observer_subscribed: bool,
    /// Active agent executions (request_id -> cancel sender)
    active_agents: HashMap<String, mpsc::Sender<()>>,
}

// ══════════════════════════════════════════════════════════════════════════════
// WebSocket Handler
// ══════════════════════════════════════════════════════════════════════════════

/// Handle WebSocket upgrade
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Generate connection ID
    let connection_id = format!("ws-{}", uuid::Uuid::new_v4());

    // Initialize connection state
    let mut conn_state = ConnectionState {
        id: connection_id.clone(),
        subscribed_sessions: std::collections::HashSet::new(),
        observer_subscribed: false,
        active_agents: HashMap::new(),
    };

    tracing::info!(connection_id = %conn_state.id, "WebSocket connection established");

    // Send connected message
    let connected_msg = WsServerMessage::Connected {
        connection_id: conn_state.id.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    // Create channel for server-initiated messages (from agent execution, etc.)
    let (server_tx, mut server_rx) = mpsc::channel::<WsServerMessage>(100);

    // Main message loop
    loop {
        tokio::select! {
            // Handle incoming client messages
            msg = receiver.next() => {
                let msg = match msg {
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

                // Try parsing as new protocol first, fall back to legacy
                let response = if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&msg) {
                    handle_client_message(
                        &state,
                        &mut conn_state,
                        client_msg,
                        server_tx.clone(),
                    ).await
                } else if let Ok(legacy_msg) = serde_json::from_str::<WsMessage>(&msg) {
                    handle_legacy_message(&state, legacy_msg).await
                } else {
                    Some(WsServerMessage::Error {
                        code: -1,
                        message: format!("Failed to parse message: {}", &msg[..msg.len().min(100)]),
                    })
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

/// Handle a client message (new protocol)
async fn handle_client_message(
    state: &Arc<AppState>,
    conn_state: &mut ConnectionState,
    msg: WsClientMessage,
    server_tx: mpsc::Sender<WsServerMessage>,
) -> Option<WsServerMessage> {
    match msg {
        WsClientMessage::Ping => Some(WsServerMessage::Pong),

        WsClientMessage::AgentRequest {
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
            // Note: state is Arc<AppState>, we clone the Arc not the AppState
            let state_clone = Arc::clone(state);
            let server_tx = server_tx.clone();
            let request_id = id.clone();

            tokio::spawn(async move {
                execute_agent_via_ws(
                    state_clone,
                    server_tx,
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
            Some(WsServerMessage::AgentStart { id })
        }

        WsClientMessage::AgentCancel { id } => {
            if let Some(cancel_tx) = conn_state.active_agents.remove(&id) {
                let _ = cancel_tx.send(()).await;
                Some(WsServerMessage::AgentCancelled { id })
            } else {
                Some(WsServerMessage::Error {
                    code: -2,
                    message: format!("No active agent with id: {}", id),
                })
            }
        }

        WsClientMessage::ToolRequest { id, tool, params } => {
            let result = handle_tool_request(state.as_ref(), &id, &tool, params).await;
            Some(result)
        }

        WsClientMessage::ConfirmationResponse { id, approved, comment } => {
            // Handle confirmation response
            tracing::debug!(
                confirmation_id = %id,
                approved = approved,
                comment = ?comment,
                "Received confirmation response"
            );

            // Forward to confirmation registry
            let handled = zero_core::agent::handle_confirmation_response(&id, approved).await;
            if !handled {
                return Some(WsServerMessage::Error {
                    code: -3,
                    message: format!("Confirmation not found or already handled: {}", id),
                });
            }

            None // No response needed
        }

        WsClientMessage::SessionSubscribe { session_id } => {
            conn_state.subscribed_sessions.insert(session_id.clone());
            tracing::debug!(
                connection_id = %conn_state.id,
                session_id = %session_id,
                "Subscribed to session"
            );
            None
        }

        WsClientMessage::SessionUnsubscribe { session_id } => {
            conn_state.subscribed_sessions.remove(&session_id);
            None
        }

        WsClientMessage::ObserverSubscribe => {
            conn_state.observer_subscribed = true;
            None
        }

        WsClientMessage::ObserverUnsubscribe => {
            conn_state.observer_subscribed = false;
            None
        }
    }
}

/// Execute agent via WebSocket with streaming
async fn execute_agent_via_ws(
    state: Arc<AppState>,
    server_tx: mpsc::Sender<WsServerMessage>,
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
    use futures_util::StreamExt;
    use zero_core::agent::{ContentPart, Message, Role, StreamEvent, StreamRequest, ToolDef};
    use crate::session::types::MessageRole;

    // Get agent metadata
    let agent = match state.unified_api.as_ref() {
        Some(api) => api.get_agent(&agent_name).await,
        None => None,
    };

    if agent.is_none() {
        let _ = server_tx
            .send(WsServerMessage::AgentError {
                id: request_id,
                code: -4,
                message: format!("Agent not found: {}", agent_name),
            })
            .await;
        return;
    }
    let agent = agent.unwrap();

    // Get LLM provider
    let provider = match state.unified_api.as_ref().and_then(|api| api.llm_provider.clone()) {
        Some(p) => p,
        None => {
            let _ = server_tx
                .send(WsServerMessage::AgentError {
                    id: request_id,
                    code: -5,
                    message: "LLM provider not configured".to_string(),
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
    let session_messages = state
        .unified_api
        .as_ref()
        .map(|api| api.sessions.get_messages(&session_id).unwrap_or_default())
        .unwrap_or_default();

    let mut messages: Vec<Message> = session_messages
        .iter()
        .map(|m| {
            let role = match m.role {
                MessageRole::User => Role::User,
                MessageRole::Assistant => Role::Assistant,
                MessageRole::System => Role::System,
            };
            Message {
                role,
                content: vec![ContentPart::Text {
                    text: m.content.clone(),
                }],
            }
        })
        .collect();

    // Add new user message
    messages.push(Message::user(&message));

    // Get tools
    let tools: Vec<ToolDef> = if let Some(api) = state.unified_api.as_ref() {
        let registry = api.tools.read().await;
        registry
            .native_tools()
            .iter()
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.parameters_schema(),
            })
            .collect()
    } else {
        vec![]
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
    if let Some(api) = state.unified_api.as_ref() {
        let _ = api.sessions.add_message(&session_id, MessageRole::User, &message);
    }

    // Agent loop
    let max_iterations = 10;
    let tool_timeout = Duration::from_secs(30);
    let mut iterations = 0;
    let mut full_text = String::new();
    let mut total_usage = TokenUsage {
        input_tokens: 0,
        output_tokens: 0,
        reasoning_tokens: 0,
        cache_read_tokens: None,
        cache_write_tokens: None,
    };

    'agent_loop: loop {
        iterations += 1;
        if iterations > max_iterations {
            let _ = server_tx
                .send(WsServerMessage::AgentError {
                    id: request_id.clone(),
                    code: -6,
                    message: format!("Max iterations ({}) exceeded", max_iterations),
                })
                .await;
            break;
        }

        // Start stream
        let mut event_stream = match provider.stream(request.clone()).await {
            Ok(s) => s,
            Err(e) => {
                let _ = server_tx
                    .send(WsServerMessage::AgentError {
                        id: request_id.clone(),
                        code: -7,
                        message: format!("Failed to start stream: {}", e),
                    })
                    .await;
                break;
            }
        };

        let mut iteration_text = String::new();
        let mut tool_calls: Vec<(String, String, serde_json::Value)> = vec![];
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();

        loop {
            tokio::select! {
                // Check for cancellation
                _ = cancel_rx.recv() => {
                    let _ = server_tx
                        .send(WsServerMessage::AgentCancelled {
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
                                        .send(WsServerMessage::AgentText {
                                            id: request_id.clone(),
                                            content,
                                        })
                                        .await;
                                }

                                StreamEvent::ReasoningDelta { content } => {
                                    let _ = server_tx
                                        .send(WsServerMessage::AgentReasoning {
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
                                        .send(WsServerMessage::AgentToolCall {
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
                                        let arguments: serde_json::Value =
                                            serde_json::from_str(&current_tool_args).unwrap_or_default();
                                        tool_calls.push((
                                            current_tool_id.clone(),
                                            current_tool_name.clone(),
                                            arguments.clone(),
                                        ));
                                        let _ = server_tx
                                            .send(WsServerMessage::AgentToolCall {
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
                                        total_usage.input_tokens += u.input_tokens;
                                        total_usage.output_tokens += u.output_tokens;
                                        total_usage.reasoning_tokens += u.reasoning_tokens;
                                        if let Some(v) = u.cache_read_tokens {
                                            *total_usage.cache_read_tokens.get_or_insert(0) += v;
                                        }
                                        if let Some(v) = u.cache_write_tokens {
                                            *total_usage.cache_write_tokens.get_or_insert(0) += v;
                                        }
                                    }

                                    // Don't send finish if we have tool calls to execute
                                    if tool_calls.is_empty() {
                                        let _ = server_tx
                                            .send(WsServerMessage::AgentComplete {
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
                                        .send(WsServerMessage::AgentError {
                                            id: request_id.clone(),
                                            code,
                                            message: msg,
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
        request.messages.push(Message {
            role: Role::Assistant,
            content: assistant_content,
        });

        // Execute tools
        for (tool_call_id, tool_name, arguments) in tool_calls {
            let result = if let Some(api) = state.unified_api.as_ref() {
                let registry = api.tools.read().await;
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
            } else {
                (None, Some("API not initialized".to_string()))
            };

            // Send tool result
            let _ = server_tx
                .send(WsServerMessage::AgentToolResult {
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
            request.messages.push(Message::tool_result(&tool_call_id, &result_str));
        }
    }

    // Store assistant response in session
    if !full_text.is_empty() {
        if let Some(api) = state.unified_api.as_ref() {
            let _ = api.sessions.add_message(&session_id, MessageRole::Assistant, &full_text);
        }
    }
}

/// Handle a legacy message (backwards compatibility)
async fn handle_legacy_message(state: &Arc<AppState>, msg: WsMessage) -> Option<WsServerMessage> {
    match msg {
        WsMessage::Ping => Some(WsServerMessage::Pong),
        WsMessage::ToolRequest { id, tool, params } => {
            let result = handle_tool_request(state.as_ref(), &id, &tool, params).await;
            Some(result)
        }
        _ => Some(WsServerMessage::Error {
            code: -100,
            message: "Unsupported legacy message type".to_string(),
        }),
    }
}

/// Handle tool request via WebSocket
async fn handle_tool_request(
    state: &AppState,
    id: &str,
    tool: &str,
    params: Value,
) -> WsServerMessage {
    let result = match tool {
        "grep" => {
            let options: Result<zero_core::GrepOptions, _> = serde_json::from_value(params);
            match options {
                Ok(opts) => match state.grep.search(&opts).await {
                    Ok(r) => Ok(serde_json::to_value(r).ok()),
                    Err(e) => Err(e.to_string()),
                },
                Err(e) => Err(format!("Invalid params: {}", e)),
            }
        }
        "read" => {
            let file_path = params.get("file_path").and_then(|v| v.as_str());
            match file_path {
                Some(path) => match state.reader.read(std::path::Path::new(path), None) {
                    Ok(r) => Ok(serde_json::to_value(r).ok()),
                    Err(e) => Err(e.to_string()),
                },
                None => Err("Missing file_path".to_string()),
            }
        }
        _ => Err(format!("Unknown tool: {}", tool)),
    };

    match result {
        Ok(value) => WsServerMessage::ToolResponse {
            id: id.to_string(),
            success: true,
            result: value,
            error: None,
        },
        Err(error) => WsServerMessage::ToolResponse {
            id: id.to_string(),
            success: false,
            result: None,
            error: Some(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization() {
        let msg = WsClientMessage::AgentRequest {
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
        assert!(json.contains("\"type\":\"agent_request\""));
        assert!(json.contains("\"agent\":\"build\""));
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = WsServerMessage::AgentText {
            id: "test-1".to_string(),
            content: "Hello, world!".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"agent_text\""));
        assert!(json.contains("\"content\":\"Hello, world!\""));
    }

    #[test]
    fn test_token_usage_serialization() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            reasoning_tokens: 0,
            cache_read_tokens: Some(80),
            cache_write_tokens: None,
        };
        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"input_tokens\":100"));
        assert!(json.contains("\"cache_read_tokens\":80"));
        // cache_write_tokens should be skipped since it's None
        assert!(!json.contains("cache_write_tokens"));
    }

    #[test]
    fn test_legacy_message_compatibility() {
        let msg = WsMessage::ToolRequest {
            id: "legacy-1".to_string(),
            tool: "grep".to_string(),
            params: serde_json::json!({"pattern": "test"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"tool_request\""));
    }
}
