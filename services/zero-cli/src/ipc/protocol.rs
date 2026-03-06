//! IPC Protocol Types for zero-cli ↔ TypeScript TUI communication.
//!
//! Uses JSON-RPC 2.0 over Unix Domain Socket (macOS/Linux) or Named Pipe (Windows).
//! The protocol supports bidirectional communication:
//! - Requests (TUI → CLI): Initialize, ToolCall, GetSession, ListSessions, Compact
//! - Notifications (CLI → TUI): SessionUpdate, ToolRequest, LlmRequest, StreamToken, Error

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::session::types::SessionMessage;

// ══════════════════════════════════════════════════════════════════════════════
// JSON-RPC 2.0 Types (IPC-specific, reusing patterns from mcp/types.rs)
// ══════════════════════════════════════════════════════════════════════════════

/// JSON-RPC ID (can be string or number)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum IpcId {
    Number(i64),
    String(String),
}

impl From<i64> for IpcId {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<&str> for IpcId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for IpcId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

/// JSON-RPC 2.0 request (TUI → CLI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<IpcId>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl IpcRequest {
    pub fn new(id: impl Into<IpcId>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(id.into()),
            method: method.into(),
            params: None,
        }
    }

    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }

    /// Create a notification (no id, no response expected)
    pub fn notification(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: None,
            method: method.into(),
            params: None,
        }
    }
}

/// JSON-RPC 2.0 response (CLI → TUI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<IpcId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

impl IpcResponse {
    pub fn success(id: impl Into<IpcId>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(id.into()),
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<IpcId>, error: IpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl IpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Parse error (-32700)
    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self::new(-32700, msg)
    }

    /// Invalid request (-32600)
    pub fn invalid_request(msg: impl Into<String>) -> Self {
        Self::new(-32600, msg)
    }

    /// Method not found (-32601)
    pub fn method_not_found(method: &str) -> Self {
        Self::new(-32601, format!("Method not found: {method}"))
    }

    /// Invalid params (-32602)
    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(-32602, msg)
    }

    /// Internal error (-32603)
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::new(-32603, msg)
    }

    /// Session not found (-32001)
    pub fn session_not_found(session_id: &str) -> Self {
        Self::new(-32001, format!("Session not found: {session_id}"))
    }

    /// Tool execution error (-32002)
    pub fn tool_error(msg: impl Into<String>) -> Self {
        Self::new(-32002, msg)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// IPC Method Constants
// ══════════════════════════════════════════════════════════════════════════════

pub mod methods {
    // Request methods (TUI → CLI)
    pub const INITIALIZE: &str = "ipc/initialize";
    pub const TOOL_CALL: &str = "ipc/tool_call";
    pub const TOOL_RESULT: &str = "ipc/tool_result";
    pub const CANCEL_GENERATION: &str = "ipc/cancel_generation";
    pub const GET_SESSION: &str = "ipc/get_session";
    pub const LIST_SESSIONS: &str = "ipc/list_sessions";
    pub const COMPACT: &str = "ipc/compact";
    pub const PING: &str = "ipc/ping";
    /// Agent prompt request - initiates LLM call with tool callback support
    pub const AGENT_PROMPT: &str = "ipc/agent_prompt";

    // Notification methods (CLI → TUI)
    pub const SESSION_UPDATE: &str = "ipc/session_update";
    pub const TOOL_REQUEST: &str = "ipc/tool_request";
    pub const LLM_REQUEST: &str = "ipc/llm_request";
    pub const STREAM_TOKEN: &str = "ipc/stream_token";
    pub const ERROR: &str = "ipc/error";
    /// Agent stream event - streaming response from LLM
    pub const AGENT_STREAM: &str = "ipc/agent_stream";
}

// ══════════════════════════════════════════════════════════════════════════════
// Request Parameter Types
// ══════════════════════════════════════════════════════════════════════════════

/// Initialize request params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Optional session ID to resume (if None, creates new session)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Current working directory
    pub cwd: String,
    /// Client info
    pub client_info: ClientInfo,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Session ID (new or resumed)
    pub session_id: String,
    /// Server info
    pub server_info: ServerInfo,
    /// Available tools
    pub tools: Vec<ToolInfo>,
    /// Session history (if resuming)
    pub messages: Vec<SessionMessage>,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Tool information for client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
}

/// Tool call request params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallParams {
    /// Unique call ID (for correlation)
    pub call_id: String,
    /// Tool name
    pub name: String,
    /// Tool arguments
    #[serde(default)]
    pub args: HashMap<String, Value>,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// Call ID (for correlation)
    pub call_id: String,
    /// Result content (text, image, etc.)
    pub content: Vec<ToolContent>,
    /// Whether the tool execution resulted in an error
    #[serde(default)]
    pub is_error: bool,
}

/// Tool content (matches MCP ToolContent)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

impl ToolContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
}

/// Tool result notification params (TUI → CLI, when LLM decides tool result)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultParams {
    /// Call ID (for correlation)
    pub call_id: String,
    /// Result value
    pub result: Value,
    /// Error message if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Get session params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSessionParams {
    pub session_id: String,
}

/// Session info result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    pub messages: Vec<SessionMessage>,
    pub token_count: usize,
    pub message_count: usize,
    pub created_at: i64,
    pub updated_at: i64,
}

/// List sessions result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSessionsResult {
    pub sessions: Vec<SessionSummary>,
}

/// Session summary (for list)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub message_count: usize,
    pub token_count: usize,
    pub updated_at: i64,
}

/// Compact session params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactParams {
    pub session_id: String,
}

/// Compact result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactResult {
    pub deleted_count: usize,
    pub new_token_count: usize,
}

// ══════════════════════════════════════════════════════════════════════════════
// Notification Types (CLI → TUI)
// ══════════════════════════════════════════════════════════════════════════════

/// Session update notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdateNotification {
    pub session_id: String,
    /// New message added
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<SessionMessage>,
    /// Updated token count
    pub token_count: usize,
}

/// Tool request notification (CLI → TUI, asking TUI to execute tool via LLM)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequestNotification {
    /// Unique request ID
    pub request_id: String,
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub args: Value,
}

/// LLM request notification (CLI → TUI, asking TUI to call LLM)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmRequestNotification {
    /// Unique request ID
    pub request_id: String,
    /// Messages to send to LLM
    pub messages: Vec<LlmMessage>,
    /// LLM options
    pub options: LlmOptions,
}

/// LLM message format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

/// LLM options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub stream: bool,
}

impl Default for LlmOptions {
    fn default() -> Self {
        Self {
            model: None,
            temperature: None,
            max_tokens: None,
            stream: true,
        }
    }
}

/// Stream token notification (CLI → TUI, streaming LLM response)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamTokenNotification {
    /// Request ID this token belongs to
    pub request_id: String,
    /// Token content
    pub token: String,
    /// Whether this is the final token
    #[serde(default)]
    pub done: bool,
}

/// Error notification (CLI → TUI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorNotification {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// ══════════════════════════════════════════════════════════════════════════════
// Agent Prompt Types (Phase 6.1)
// ══════════════════════════════════════════════════════════════════════════════

/// Model information for agent prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    /// Provider ID (e.g., "anthropic", "openai")
    pub provider_id: String,
    /// Model ID (e.g., "claude-opus-4-5", "gpt-4o")
    pub model_id: String,
    /// Optional API key override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Optional base URL override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Agent prompt request params
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPromptParams {
    /// Session ID
    pub session_id: String,
    /// User message
    pub message: String,
    /// Agent name (e.g., "build", "plan")
    pub agent: String,
    /// Model to use
    pub model: ModelInfo,
    /// System prompt parts
    #[serde(default)]
    pub system: Vec<String>,
    /// Conversation history (simplified)
    #[serde(default)]
    pub messages: Vec<AgentMessage>,
    /// Available tools (names)
    #[serde(default)]
    pub tools: Vec<String>,
}

/// Simplified message for agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<AgentToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Tool call in message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Agent prompt result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPromptResult {
    /// Request ID for stream correlation
    pub request_id: String,
    /// Whether streaming is enabled
    pub streaming: bool,
}

/// Agent stream event notification (CLI → TUI)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStreamNotification {
    /// Request ID for correlation
    pub request_id: String,
    /// Stream event
    pub event: AgentStreamEvent,
}

/// Agent stream event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentStreamEvent {
    /// Start of response
    Start,
    /// Text delta (streaming text)
    TextDelta {
        content: String,
    },
    /// Reasoning/thinking delta (for models that support it)
    ReasoningDelta {
        content: String,
    },
    /// Tool call started
    ToolCallStart {
        id: String,
        name: String,
    },
    /// Tool call arguments (streamed)
    ToolCallDelta {
        id: String,
        arguments_delta: String,
    },
    /// Tool call complete - TUI should execute and respond
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    /// Tool result received (after TUI executes tool)
    ToolResult {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Response finished
    Finish {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<TokenUsage>,
    },
    /// Error occurred
    Error {
        code: i32,
        message: String,
    },
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

/// Create a notification message
pub fn create_notification(method: &str, params: impl Serialize) -> IpcRequest {
    IpcRequest::notification(method).with_params(serde_json::to_value(params).unwrap_or_default())
}

/// Default IPC socket path
pub fn default_socket_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".codecoder")
        .join("ipc.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_request_serialization() {
        let req = IpcRequest::new(1i64, "ipc/initialize").with_params(serde_json::json!({
            "cwd": "/home/user/project",
            "clientInfo": {
                "name": "ccode-tui",
                "version": "1.0.0"
            }
        }));

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"ipc/initialize\""));
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn test_ipc_response_success() {
        let resp = IpcResponse::success(1i64, serde_json::json!({"session_id": "abc123"}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_ipc_response_error() {
        let resp = IpcResponse::error(
            Some(1i64.into()),
            IpcError::session_not_found("unknown"),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32001"));
    }

    #[test]
    fn test_notification_has_no_id() {
        let notif = IpcRequest::notification("ipc/session_update");
        let json = serde_json::to_string(&notif).unwrap();
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_tool_content_text() {
        let content = ToolContent::text("Hello, world!");
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello, world!\""));
    }

    #[test]
    fn test_initialize_params_deserialization() {
        let json = r#"{
            "cwd": "/home/user/project",
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }"#;

        let params: InitializeParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.cwd, "/home/user/project");
        assert_eq!(params.client_info.name, "test-client");
        assert!(params.session_id.is_none());
    }

    #[test]
    fn test_llm_options_default() {
        let options = LlmOptions::default();
        assert!(options.stream);
        assert!(options.model.is_none());
        assert!(options.temperature.is_none());
    }

    #[test]
    fn test_default_socket_path() {
        let path = default_socket_path();
        assert!(path.to_string_lossy().contains(".codecoder"));
        assert!(path.to_string_lossy().contains("ipc.sock"));
    }

    #[test]
    fn test_agent_prompt_params_serialization() {
        let params = AgentPromptParams {
            session_id: "session-123".into(),
            message: "Hello, world!".into(),
            agent: "build".into(),
            model: ModelInfo {
                provider_id: "anthropic".into(),
                model_id: "claude-opus-4-5".into(),
                api_key: None,
                base_url: None,
            },
            system: vec!["You are a helpful assistant.".into()],
            messages: vec![],
            tools: vec!["read".into(), "write".into()],
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"sessionId\":\"session-123\""));
        assert!(json.contains("\"providerId\":\"anthropic\""));
        assert!(json.contains("\"modelId\":\"claude-opus-4-5\""));
    }

    #[test]
    fn test_agent_stream_event_text_delta() {
        let event = AgentStreamEvent::TextDelta {
            content: "Hello".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"text_delta\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_agent_stream_event_tool_call() {
        let event = AgentStreamEvent::ToolCall {
            id: "call-123".into(),
            name: "read".into(),
            arguments: serde_json::json!({"file_path": "/README.md"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"tool_call\""));
        assert!(json.contains("\"id\":\"call-123\""));
        assert!(json.contains("\"name\":\"read\""));
    }

    #[test]
    fn test_agent_stream_event_finish() {
        let event = AgentStreamEvent::Finish {
            reason: "stop".into(),
            usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                reasoning_tokens: 0,
                cache_read_tokens: Some(80),
                cache_write_tokens: None,
            }),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"finish\""));
        assert!(json.contains("\"reason\":\"stop\""));
        assert!(json.contains("\"inputTokens\":100"));
    }

    #[test]
    fn test_agent_stream_notification() {
        let notif = AgentStreamNotification {
            request_id: "req-456".into(),
            event: AgentStreamEvent::Start,
        };
        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"requestId\":\"req-456\""));
        assert!(json.contains("\"type\":\"start\""));
    }
}
