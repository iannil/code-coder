//! Agent Management Routes
//!
//! Handles agent listing, dispatch, and prompt retrieval.
//! Agents are loaded from TypeScript prompt files and cached.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    Json,
};
use futures_util::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use crate::session::types::MessageRole;

use super::state::{
    AgentMetadata, ContentPart, Message, Role, StreamEvent, StreamRequest, ToolDef,
    UnifiedApiState,
};

// ══════════════════════════════════════════════════════════════════════════════
// Request/Response Types
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
pub struct AgentListResponse {
    pub success: bool,
    pub agents: Vec<AgentInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub description: Option<String>,
    pub mode: String,
    pub temperature: Option<f64>,
    pub color: Option<String>,
    pub hidden: bool,
}

impl From<AgentMetadata> for AgentInfo {
    fn from(m: AgentMetadata) -> Self {
        Self {
            name: m.name,
            description: m.description,
            mode: m.mode,
            temperature: m.temperature,
            color: m.color,
            hidden: m.hidden,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AgentDetailResponse {
    pub success: bool,
    pub agent: AgentInfo,
}

#[derive(Debug, Deserialize)]
pub struct DispatchAgentRequest {
    /// Session ID (required)
    pub session_id: String,
    /// Agent name (e.g., "build", "plan")
    pub agent: String,
    /// User message
    pub message: String,
    /// Additional system prompts (optional)
    #[serde(default)]
    pub system: Vec<String>,
    /// Temperature override (optional)
    pub temperature: Option<f64>,
    /// Max tokens (optional)
    pub max_tokens: Option<usize>,
    /// Model override (optional)
    pub model: Option<String>,
    /// Enable streaming (default: true)
    #[serde(default = "default_true")]
    pub stream: bool,
    /// Max tool execution iterations (default: 10)
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// Tool execution timeout in seconds (default: 30)
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout: u64,
}

fn default_max_iterations() -> usize {
    10
}

fn default_tool_timeout() -> u64 {
    30
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct DispatchAgentResponse {
    pub success: bool,
    pub request_id: String,
    pub streaming: bool,
}

#[derive(Debug, Serialize)]
pub struct AgentPromptResponse {
    pub success: bool,
    pub name: String,
    pub prompt: String,
    pub modified_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// Route Handlers
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/agents - List all available agents
pub async fn list_agents(State(state): State<Arc<UnifiedApiState>>) -> impl IntoResponse {
    let agents = state.list_agents().await;
    let visible_agents: Vec<AgentInfo> = agents
        .into_iter()
        .filter(|a| !a.hidden)
        .map(Into::into)
        .collect();

    let total = visible_agents.len();

    Json(AgentListResponse {
        success: true,
        agents: visible_agents,
        total,
    })
}

/// GET /api/v1/agents/:name - Get agent details
pub async fn get_agent(
    State(state): State<Arc<UnifiedApiState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.get_agent(&name).await {
        Some(agent) => Json(AgentDetailResponse {
            success: true,
            agent: agent.into(),
        })
        .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Agent not found: {}", name),
            }),
        )
            .into_response(),
    }
}

/// POST /api/v1/agents/dispatch - Dispatch an agent task
///
/// This is the main entry point for running agent tasks.
/// It starts an agent loop with streaming response support.
pub async fn dispatch_agent(
    State(state): State<Arc<UnifiedApiState>>,
    Json(request): Json<DispatchAgentRequest>,
) -> impl IntoResponse {
    // Validate agent exists
    let agent = match state.get_agent(&request.agent).await {
        Some(a) => a,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    success: false,
                    error: format!("Agent not found: {}", request.agent),
                }),
            )
                .into_response();
        }
    };

    // Check if provider is available
    let provider = match &state.llm_provider {
        Some(p) => Arc::clone(p),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    success: false,
                    error: "LLM provider not configured".to_string(),
                }),
            )
                .into_response();
        }
    };

    // Generate request ID
    let request_id = format!("dispatch-{}", uuid::Uuid::new_v4());

    // Build system prompts
    let mut system_prompts = vec![];
    if let Some(ref prompt) = agent.prompt {
        system_prompts.push(prompt.clone());
    }
    system_prompts.extend(request.system.clone());

    // Get session history and convert to Messages
    let session_messages = state
        .sessions
        .get_messages(&request.session_id)
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

    // Add the new user message
    messages.push(Message::user(&request.message));

    // Get tools from registry and convert to ToolDef
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

    // Get model (request override > default)
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| "claude-sonnet-4-5-20250514".to_string());

    // Get temperature (request override > agent default > 0.7)
    let temperature = request.temperature.or(agent.temperature);

    // Build stream request
    let stream_request = StreamRequest {
        system: system_prompts,
        messages,
        tools,
        model,
        temperature,
        max_tokens: request.max_tokens,
    };

    // Store the user message in session
    if let Err(e) = state
        .sessions
        .add_message(&request.session_id, MessageRole::User, &request.message)
    {
        tracing::warn!("Failed to store user message: {}", e);
    }

    // If not streaming, execute synchronously and return result
    if !request.stream {
        return execute_agent_sync(
            state,
            provider,
            stream_request,
            request_id,
            request.session_id,
            request.max_iterations,
            request.tool_timeout,
        )
        .await
        .into_response();
    }

    // For streaming, return SSE stream
    let sse = create_agent_stream_from_provider(
        state,
        provider,
        stream_request,
        request_id,
        request.session_id,
        request.max_iterations,
        request.tool_timeout,
    );

    sse.into_response()
}

/// Execute agent synchronously (non-streaming) with full tool execution loop
async fn execute_agent_sync(
    state: Arc<UnifiedApiState>,
    provider: Arc<dyn super::state::StreamingProvider>,
    request: StreamRequest,
    request_id: String,
    session_id: String,
    max_iterations: usize,
    tool_timeout: u64,
) -> Json<DispatchAgentSyncResponse> {
    match run_agent_loop(state.clone(), provider, request, max_iterations, tool_timeout).await {
        Ok(result) => {
            // Store assistant response in session
            if !result.text.is_empty() {
                if let Err(e) =
                    state
                        .sessions
                        .add_message(&session_id, MessageRole::Assistant, &result.text)
                {
                    tracing::warn!("Failed to store assistant message: {}", e);
                }
            }

            Json(DispatchAgentSyncResponse {
                success: true,
                request_id,
                response: result.text,
                usage: Some(result.usage),
                error: None,
            })
        }
        Err(e) => Json(DispatchAgentSyncResponse {
            success: false,
            request_id,
            response: String::new(),
            usage: None,
            error: Some(e.to_string()),
        }),
    }
}

/// GET /api/v1/agents/:name/prompt - Get agent's raw prompt
pub async fn get_agent_prompt(
    State(state): State<Arc<UnifiedApiState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.get_agent(&name).await {
        Some(agent) => {
            let modified_at = agent.prompt_modified_at.map(|t| t.to_rfc3339());

            Json(AgentPromptResponse {
                success: true,
                name: agent.name,
                prompt: agent.prompt.unwrap_or_default(),
                modified_at,
            })
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                success: false,
                error: format!("Agent not found: {}", name),
            }),
        )
            .into_response(),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// SSE Streaming Support
// ══════════════════════════════════════════════════════════════════════════════

/// Agent stream event types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentStreamEvent {
    Start,
    TextDelta { content: String },
    ReasoningDelta { content: String },
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, arguments_delta: String },
    ToolCall { id: String, name: String, arguments: serde_json::Value },
    ToolResult { id: String, output: Option<String>, error: Option<String> },
    Finish { reason: String, usage: Option<TokenUsage> },
    Error { code: i32, message: String },
}

/// Synchronous dispatch response
#[derive(Debug, Serialize)]
pub struct DispatchAgentSyncResponse {
    pub success: bool,
    pub request_id: String,
    pub response: String,
    pub usage: Option<TokenUsage>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub reasoning_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<usize>,
}

impl TokenUsage {
    /// Merge another usage into this one (accumulate tokens)
    pub fn merge(&mut self, other: &TokenUsage) {
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

// ══════════════════════════════════════════════════════════════════════════════
// Tool Execution Loop
// ══════════════════════════════════════════════════════════════════════════════

/// Information about a tool call from the LLM
#[derive(Debug, Clone)]
struct ToolCallInfo {
    id: String,
    name: String,
    arguments: serde_json::Value,
}

/// Result of the agent execution loop
#[derive(Debug)]
struct AgentLoopResult {
    /// Final text response
    text: String,
    /// Accumulated token usage
    usage: TokenUsage,
    /// Number of iterations performed (for debugging/logging)
    #[allow(dead_code)]
    iterations: usize,
}

/// Execute a single tool call
async fn execute_tool(
    state: &UnifiedApiState,
    tool_call: &ToolCallInfo,
    tool_timeout_secs: u64,
) -> String {
    let registry = state.tools.read().await;

    match registry.get_tool(&tool_call.name).await {
        Some(tool) => {
            // Execute with timeout
            let execute_future = tool.execute(tool_call.arguments.clone());
            let timeout_duration = Duration::from_secs(tool_timeout_secs);

            match timeout(timeout_duration, execute_future).await {
                Ok(Ok(result)) => {
                    if result.success {
                        result.output
                    } else {
                        format!("Error: {}", result.error.unwrap_or_default())
                    }
                }
                Ok(Err(e)) => format!("Tool execution failed: {}", e),
                Err(_) => format!("Tool execution timed out after {} seconds", tool_timeout_secs),
            }
        }
        None => format!("Unknown tool: {}", tool_call.name),
    }
}

/// Run the complete agent loop with tool execution
///
/// This function implements the multi-turn tool calling pattern:
/// 1. Send messages to LLM
/// 2. If LLM returns tool calls, execute them
/// 3. Add tool results to messages
/// 4. Continue until LLM returns text-only response or max iterations reached
async fn run_agent_loop(
    state: Arc<UnifiedApiState>,
    provider: Arc<dyn super::state::StreamingProvider>,
    mut request: StreamRequest,
    max_iterations: usize,
    tool_timeout_secs: u64,
) -> anyhow::Result<AgentLoopResult> {
    let mut iterations = 0;
    let mut total_usage = TokenUsage::default();

    loop {
        iterations += 1;
        if iterations > max_iterations {
            return Err(anyhow::anyhow!(
                "Max iterations ({}) exceeded - possible infinite tool loop",
                max_iterations
            ));
        }

        tracing::debug!("Agent loop iteration {}/{}", iterations, max_iterations);

        // 1. Call LLM
        let mut event_stream = provider.stream(request.clone()).await?;

        // 2. Collect response
        let mut text_content = String::new();
        let mut tool_calls: Vec<ToolCallInfo> = Vec::new();
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();

        while let Some(result) = event_stream.next().await {
            match result {
                Ok(event) => match event {
                    StreamEvent::TextDelta { content } => {
                        text_content.push_str(&content);
                    }
                    StreamEvent::ToolCallStart { id, name } => {
                        // Start accumulating a new tool call
                        current_tool_id = id;
                        current_tool_name = name;
                        current_tool_args.clear();
                    }
                    StreamEvent::ToolCallDelta { arguments_delta, .. } => {
                        current_tool_args.push_str(&arguments_delta);
                    }
                    StreamEvent::ToolCall { id, name, arguments } => {
                        // Complete tool call (some providers emit this directly)
                        tool_calls.push(ToolCallInfo { id, name, arguments });
                    }
                    StreamEvent::Finish { usage, .. } => {
                        // If we were accumulating a tool call, finalize it
                        if !current_tool_id.is_empty() && !current_tool_name.is_empty() {
                            let arguments: serde_json::Value =
                                serde_json::from_str(&current_tool_args).unwrap_or_default();
                            tool_calls.push(ToolCallInfo {
                                id: current_tool_id.clone(),
                                name: current_tool_name.clone(),
                                arguments,
                            });
                            current_tool_id.clear();
                            current_tool_name.clear();
                            current_tool_args.clear();
                        }

                        if let Some(u) = usage {
                            let token_usage = TokenUsage {
                                input_tokens: u.input_tokens,
                                output_tokens: u.output_tokens,
                                reasoning_tokens: u.reasoning_tokens,
                                cache_read_tokens: u.cache_read_tokens,
                                cache_write_tokens: u.cache_write_tokens,
                            };
                            total_usage.merge(&token_usage);
                        }
                    }
                    StreamEvent::Error { code, message } => {
                        return Err(anyhow::anyhow!("LLM error ({}): {}", code, message));
                    }
                    _ => {}
                },
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                }
            }
        }

        // 3. If no tool calls, return final result
        if tool_calls.is_empty() {
            return Ok(AgentLoopResult {
                text: text_content,
                usage: total_usage,
                iterations,
            });
        }

        tracing::debug!("Executing {} tool calls", tool_calls.len());

        // 4. Add assistant message (with tool uses)
        let mut assistant_content = Vec::new();
        if !text_content.is_empty() {
            assistant_content.push(ContentPart::Text { text: text_content });
        }
        for tc in &tool_calls {
            assistant_content.push(ContentPart::ToolUse {
                id: tc.id.clone(),
                name: tc.name.clone(),
                input: tc.arguments.clone(),
            });
        }
        request.messages.push(Message {
            role: Role::Assistant,
            content: assistant_content,
        });

        // 5. Execute tools and add results
        for tc in tool_calls {
            tracing::debug!("Executing tool: {} (id: {})", tc.name, tc.id);
            let result = execute_tool(&state, &tc, tool_timeout_secs).await;
            tracing::debug!("Tool result length: {} chars", result.len());
            request.messages.push(Message::tool_result(&tc.id, &result));
        }
    }
}

/// Create an SSE stream for agent events
pub fn create_agent_stream(
    _request_id: String,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        // Send start event
        let event = AgentStreamEvent::Start;
        let json = serde_json::to_string(&event).unwrap_or_default();
        yield Ok(Event::default().data(json).event("agent"));

        // This is a placeholder - actual implementation would receive events
        // from the agent execution loop
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send finish event
        let event = AgentStreamEvent::Finish {
            reason: "placeholder".to_string(),
            usage: None,
        };
        let json = serde_json::to_string(&event).unwrap_or_default();
        yield Ok(Event::default().data(json).event("agent"));
    };

    Sse::new(stream)
}

/// Create an SSE stream from the streaming provider with tool execution loop
fn create_agent_stream_from_provider(
    state: Arc<UnifiedApiState>,
    provider: Arc<dyn super::state::StreamingProvider>,
    mut request: StreamRequest,
    request_id: String,
    session_id: String,
    max_iterations: usize,
    tool_timeout: u64,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        // Send start event with request_id
        let event = AgentStreamEvent::Start;
        let json = serde_json::to_string(&event).unwrap_or_default();
        yield Ok(Event::default().data(json).event("agent").id(request_id.clone()));

        let mut iterations = 0;
        let mut full_text = String::new();
        let mut total_usage = TokenUsage::default();

        // Main agent loop - continues until no more tool calls or max iterations
        'agent_loop: loop {
            iterations += 1;
            if iterations > max_iterations {
                let event = AgentStreamEvent::Error {
                    code: -2,
                    message: format!("Max iterations ({}) exceeded", max_iterations),
                };
                let json = serde_json::to_string(&event).unwrap_or_default();
                yield Ok(Event::default().data(json).event("agent"));
                break;
            }

            // Start the stream from provider
            match provider.stream(request.clone()).await {
                Ok(mut event_stream) => {
                    let mut iteration_text = String::new();
                    let mut tool_calls: Vec<ToolCallInfo> = Vec::new();
                    let mut current_tool_id = String::new();
                    let mut current_tool_name = String::new();
                    let mut current_tool_args = String::new();

                    while let Some(result) = event_stream.next().await {
                        match result {
                            Ok(provider_event) => {
                                // Convert provider event to our event format
                                let agent_event = match provider_event {
                                    StreamEvent::Start => {
                                        continue; // Already sent start
                                    }
                                    StreamEvent::TextDelta { ref content } => {
                                        iteration_text.push_str(content);
                                        AgentStreamEvent::TextDelta { content: content.clone() }
                                    }
                                    StreamEvent::ReasoningDelta { content } => {
                                        AgentStreamEvent::ReasoningDelta { content }
                                    }
                                    StreamEvent::ToolCallStart { id, name } => {
                                        // Start accumulating a new tool call
                                        current_tool_id = id.clone();
                                        current_tool_name = name.clone();
                                        current_tool_args.clear();
                                        AgentStreamEvent::ToolCallStart { id, name }
                                    }
                                    StreamEvent::ToolCallDelta { id, arguments_delta } => {
                                        current_tool_args.push_str(&arguments_delta);
                                        AgentStreamEvent::ToolCallDelta { id, arguments_delta }
                                    }
                                    StreamEvent::ToolCall { id, name, arguments } => {
                                        // Complete tool call
                                        tool_calls.push(ToolCallInfo {
                                            id: id.clone(),
                                            name: name.clone(),
                                            arguments: arguments.clone(),
                                        });
                                        AgentStreamEvent::ToolCall { id, name, arguments }
                                    }
                                    StreamEvent::Finish { reason, usage } => {
                                        // If we were accumulating a tool call, finalize it
                                        if !current_tool_id.is_empty() && !current_tool_name.is_empty() {
                                            let arguments: serde_json::Value =
                                                serde_json::from_str(&current_tool_args).unwrap_or_default();
                                            tool_calls.push(ToolCallInfo {
                                                id: current_tool_id.clone(),
                                                name: current_tool_name.clone(),
                                                arguments: arguments.clone(),
                                            });
                                            // Emit the accumulated tool call
                                            let tc_event = AgentStreamEvent::ToolCall {
                                                id: current_tool_id.clone(),
                                                name: current_tool_name.clone(),
                                                arguments,
                                            };
                                            let tc_json = serde_json::to_string(&tc_event).unwrap_or_default();
                                            yield Ok(Event::default().data(tc_json).event("agent"));

                                            current_tool_id.clear();
                                            current_tool_name.clear();
                                            current_tool_args.clear();
                                        }

                                        if let Some(ref u) = usage {
                                            let token_usage = TokenUsage {
                                                input_tokens: u.input_tokens,
                                                output_tokens: u.output_tokens,
                                                reasoning_tokens: u.reasoning_tokens,
                                                cache_read_tokens: u.cache_read_tokens,
                                                cache_write_tokens: u.cache_write_tokens,
                                            };
                                            total_usage.merge(&token_usage);
                                        }

                                        // Don't emit finish yet if we have tool calls to execute
                                        if !tool_calls.is_empty() {
                                            continue;
                                        }

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

                                let json = serde_json::to_string(&agent_event).unwrap_or_default();
                                yield Ok(Event::default().data(json).event("agent"));
                            }
                            Err(e) => {
                                tracing::error!("Stream error: {}", e);
                                let event = AgentStreamEvent::Error {
                                    code: -1,
                                    message: e.to_string(),
                                };
                                let json = serde_json::to_string(&event).unwrap_or_default();
                                yield Ok(Event::default().data(json).event("agent"));
                                break 'agent_loop;
                            }
                        }
                    }

                    // Accumulate text for session storage
                    full_text.push_str(&iteration_text);

                    // If no tool calls, we're done
                    if tool_calls.is_empty() {
                        // Emit final finish event with total usage
                        let event = AgentStreamEvent::Finish {
                            reason: "end_turn".to_string(),
                            usage: Some(total_usage.clone()),
                        };
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        yield Ok(Event::default().data(json).event("agent"));
                        break;
                    }

                    // Add assistant message (with tool uses)
                    let mut assistant_content = Vec::new();
                    if !iteration_text.is_empty() {
                        assistant_content.push(ContentPart::Text { text: iteration_text });
                    }
                    for tc in &tool_calls {
                        assistant_content.push(ContentPart::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: tc.arguments.clone(),
                        });
                    }
                    request.messages.push(Message {
                        role: Role::Assistant,
                        content: assistant_content,
                    });

                    // Execute tools and emit results
                    for tc in tool_calls {
                        tracing::debug!("Executing tool: {} (id: {})", tc.name, tc.id);
                        let result = execute_tool(&state, &tc, tool_timeout).await;
                        tracing::debug!("Tool result length: {} chars", result.len());

                        // Emit tool result event
                        let event = AgentStreamEvent::ToolResult {
                            id: tc.id.clone(),
                            output: Some(result.clone()),
                            error: None,
                        };
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        yield Ok(Event::default().data(json).event("agent"));

                        // Add tool result to messages for next iteration
                        request.messages.push(Message::tool_result(&tc.id, &result));
                    }

                    // Continue loop to let LLM process tool results
                }
                Err(e) => {
                    tracing::error!("Failed to start stream: {}", e);
                    let event = AgentStreamEvent::Error {
                        code: -1,
                        message: e.to_string(),
                    };
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    yield Ok(Event::default().data(json).event("agent"));
                    break;
                }
            }
        }

        // Store assistant response in session
        if !full_text.is_empty() {
            if let Err(e) = state.sessions.add_message(
                &session_id,
                MessageRole::Assistant,
                &full_text,
            ) {
                tracing::warn!("Failed to store assistant message: {}", e);
            }
        }
    };

    Sse::new(stream)
}
