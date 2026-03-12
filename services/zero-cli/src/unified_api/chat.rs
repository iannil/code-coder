//! SSE Streaming Chat Handler
//!
//! Provides HTTP SSE (Server-Sent Events) endpoint for agent chat.
//! This allows clients to use simple HTTP instead of WebSocket.
//!
//! ## Endpoint
//!
//! POST /api/v1/sessions/:id/chat
//!
//! ## Request
//!
//! ```json
//! {
//!     "message": "Hello!",
//!     "agent": "build",          // Optional, defaults to "build"
//!     "system": [],              // Optional extra system prompts
//!     "temperature": 0.7,        // Optional
//!     "max_tokens": 8192,        // Optional
//!     "model": "claude-sonnet-4-5-20250514"  // Optional
//! }
//! ```
//!
//! ## SSE Events
//!
//! - `text_delta` - Streaming text content
//! - `reasoning_delta` - Extended thinking content
//! - `tool_start` - Tool execution started
//! - `tool_result` - Tool execution completed
//! - `complete` - Chat finished
//! - `error` - Error occurred

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    Json,
};
use futures_util::{stream::Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Duration};

use super::state::{
    ContentPart, Message as LlmMessage, Role, StreamEvent, StreamRequest, ToolDef,
    UnifiedApiState,
};
use crate::session::types::MessageRole;

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// User message content
    pub message: String,
    /// Agent name (e.g., "build", "plan")
    #[serde(default = "default_agent")]
    pub agent: String,
    /// Additional system prompts
    #[serde(default)]
    pub system: Vec<String>,
    /// Temperature override
    pub temperature: Option<f64>,
    /// Max tokens
    pub max_tokens: Option<usize>,
    /// Model override
    pub model: Option<String>,
}

fn default_agent() -> String {
    "build".to_string()
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    /// Chat started
    Start { request_id: String },
    /// Text content delta
    TextDelta { content: String },
    /// Reasoning content delta
    ReasoningDelta { content: String },
    /// Tool execution started
    ToolStart {
        tool_call_id: String,
        tool: String,
        arguments: serde_json::Value,
    },
    /// Tool execution result
    ToolResult {
        tool_call_id: String,
        output: Option<String>,
        error: Option<String>,
    },
    /// Chat completed
    Complete {
        reason: String,
        usage: Option<TokenUsage>,
    },
    /// Error occurred
    Error { code: i32, message: String },
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// SSE Chat Handler
// ══════════════════════════════════════════════════════════════════════════════

/// POST /api/v1/sessions/:id/chat - SSE streaming chat
pub async fn chat_sse(
    State(state): State<Arc<UnifiedApiState>>,
    Path(session_id): Path<String>,
    Json(request): Json<ChatRequest>,
) -> impl IntoResponse {
    // Validate session exists or create implicitly
    let request_id = format!("chat-{}", uuid::Uuid::new_v4());

    // Get agent metadata
    let agent = match state.get_agent(&request.agent).await {
        Some(a) => a,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Agent not found: {}", request.agent),
                }),
            ));
        }
    };

    // Check LLM provider
    let provider = match &state.llm_provider {
        Some(p) => Arc::clone(p),
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    success: false,
                    error: "LLM provider not configured".to_string(),
                }),
            ));
        }
    };

    // Store user message in session
    if let Err(e) = state
        .sessions
        .add_message(&session_id, MessageRole::User, &request.message)
    {
        tracing::warn!("Failed to save user message: {}", e);
    }

    // Build initial context
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

    // Add current user message
    messages.push(LlmMessage::user(&request.message));

    // Build system prompts
    let mut system_prompts = vec![];
    if let Some(ref prompt) = agent.prompt {
        system_prompts.push(prompt.clone());
    }
    system_prompts.extend(request.system);

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

    // Build stream request
    let model = request
        .model
        .unwrap_or_else(|| "claude-sonnet-4-5-20250514".to_string());
    let temperature = request.temperature.or(agent.temperature);

    let stream_request = StreamRequest {
        system: system_prompts,
        messages,
        tools,
        model,
        temperature,
        max_tokens: request.max_tokens,
    };

    // Create SSE stream
    let stream = create_chat_stream(
        state,
        provider,
        stream_request,
        session_id.clone(),
        request_id.clone(),
    );

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}

/// Create the SSE stream for chat
fn create_chat_stream(
    state: Arc<UnifiedApiState>,
    provider: Arc<dyn super::state::StreamingProvider>,
    initial_request: StreamRequest,
    session_id: String,
    request_id: String,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        // Send start event
        yield Ok(Event::default()
            .event("start")
            .data(serde_json::to_string(&SseEvent::Start {
                request_id: request_id.clone(),
            }).unwrap_or_default()));

        let max_iterations = 10;
        let tool_timeout = Duration::from_secs(30);
        let mut iterations = 0;
        let mut full_text = String::new();
        let mut total_usage = TokenUsage::default();
        let mut request = initial_request;

        'agent_loop: loop {
            iterations += 1;
            if iterations > max_iterations {
                yield Ok(Event::default()
                    .event("error")
                    .data(serde_json::to_string(&SseEvent::Error {
                        code: -6,
                        message: format!("Max iterations ({}) exceeded", max_iterations),
                    }).unwrap_or_default()));
                break;
            }

            // Start stream
            let mut event_stream = match provider.stream(request.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    yield Ok(Event::default()
                        .event("error")
                        .data(serde_json::to_string(&SseEvent::Error {
                            code: -7,
                            message: format!("Failed to start stream: {}", e),
                        }).unwrap_or_default()));
                    break;
                }
            };

            let mut iteration_text = String::new();
            let mut tool_calls: Vec<(String, String, serde_json::Value)> = vec![];
            let mut current_tool_id = String::new();
            let mut current_tool_name = String::new();
            let mut current_tool_args = String::new();

            // Process stream events
            while let Some(result) = event_stream.next().await {
                match result {
                    Ok(stream_event) => match stream_event {
                        StreamEvent::Start => {}

                        StreamEvent::TextDelta { content } => {
                            iteration_text.push_str(&content);
                            yield Ok(Event::default()
                                .event("text_delta")
                                .data(serde_json::to_string(&SseEvent::TextDelta {
                                    content,
                                }).unwrap_or_default()));
                        }

                        StreamEvent::ReasoningDelta { content } => {
                            yield Ok(Event::default()
                                .event("reasoning_delta")
                                .data(serde_json::to_string(&SseEvent::ReasoningDelta {
                                    content,
                                }).unwrap_or_default()));
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
                            yield Ok(Event::default()
                                .event("tool_start")
                                .data(serde_json::to_string(&SseEvent::ToolStart {
                                    tool_call_id: id,
                                    tool: name,
                                    arguments,
                                }).unwrap_or_default()));
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
                                yield Ok(Event::default()
                                    .event("tool_start")
                                    .data(serde_json::to_string(&SseEvent::ToolStart {
                                        tool_call_id: current_tool_id.clone(),
                                        tool: current_tool_name.clone(),
                                        arguments,
                                    }).unwrap_or_default()));
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
                                yield Ok(Event::default()
                                    .event("complete")
                                    .data(serde_json::to_string(&SseEvent::Complete {
                                        reason,
                                        usage: Some(total_usage.clone()),
                                    }).unwrap_or_default()));
                            }
                            break;
                        }

                        StreamEvent::Error { code, message } => {
                            yield Ok(Event::default()
                                .event("error")
                                .data(serde_json::to_string(&SseEvent::Error {
                                    code,
                                    message,
                                }).unwrap_or_default()));
                            break 'agent_loop;
                        }
                    },
                    Err(e) => {
                        tracing::error!("Stream error: {}", e);
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
                yield Ok(Event::default()
                    .event("tool_result")
                    .data(serde_json::to_string(&SseEvent::ToolResult {
                        tool_call_id: tool_call_id.clone(),
                        output: result.0.clone(),
                        error: result.1.clone(),
                    }).unwrap_or_default()));

                // Add to messages
                let result_str = result.0.unwrap_or_else(|| {
                    format!("Error: {}", result.1.unwrap_or_default())
                });
                request.messages.push(LlmMessage::tool_result(&tool_call_id, &result_str));
            }
        }

        // Store assistant response in session
        if !full_text.is_empty() {
            if let Err(e) = state.sessions.add_message(&session_id, MessageRole::Assistant, &full_text) {
                tracing::warn!("Failed to save assistant message: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_serialization() {
        let event = SseEvent::TextDelta {
            content: "Hello".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"text_delta\""));
        assert!(json.contains("\"content\":\"Hello\""));
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
        assert_eq!(usage.cache_read_tokens, Some(20));
    }
}
