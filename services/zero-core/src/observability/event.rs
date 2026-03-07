//! Event types for the observability system
//!
//! Defines specialized event types for tracking LLM calls, tool executions,
//! agent lifecycle, and spans.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Event type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// LLM API call event
    LlmCall,
    /// Tool execution event
    ToolExecution,
    /// Agent lifecycle event
    AgentLifecycle,
    /// Span event (for nested tracing)
    Span,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::LlmCall => write!(f, "llm_call"),
            EventType::ToolExecution => write!(f, "tool_execution"),
            EventType::AgentLifecycle => write!(f, "agent_lifecycle"),
            EventType::Span => write!(f, "span"),
        }
    }
}

/// LLM call event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallEvent {
    /// Unique event ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Trace ID for correlation
    pub trace_id: String,
    /// Span ID for this event
    pub span_id: String,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID that made the call
    pub agent_id: Option<String>,

    /// LLM provider (anthropic, openai, google, etc.)
    pub provider: String,
    /// Model ID (claude-opus-4-5, gpt-4, etc.)
    pub model: String,
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Cache read tokens
    pub cache_read_tokens: u32,
    /// Cache write tokens
    pub cache_write_tokens: u32,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Estimated cost in USD
    pub cost_usd: f64,

    /// Whether the call succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Stop reason (end_turn, max_tokens, tool_use, etc.)
    pub stop_reason: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for LlmCallEvent {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            trace_id: Uuid::new_v4().to_string(),
            span_id: generate_span_id(),
            parent_span_id: None,
            session_id: None,
            agent_id: None,
            provider: String::new(),
            model: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            latency_ms: 0,
            cost_usd: 0.0,
            success: true,
            error: None,
            stop_reason: None,
            metadata: HashMap::new(),
        }
    }
}

/// Tool status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    /// Tool completed successfully
    Success,
    /// Tool failed with error
    Error,
    /// Tool was cancelled
    Cancelled,
    /// Tool timed out
    Timeout,
    /// Tool was blocked by permission
    Blocked,
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolStatus::Success => write!(f, "success"),
            ToolStatus::Error => write!(f, "error"),
            ToolStatus::Cancelled => write!(f, "cancelled"),
            ToolStatus::Timeout => write!(f, "timeout"),
            ToolStatus::Blocked => write!(f, "blocked"),
        }
    }
}

/// Tool execution event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionEvent {
    /// Unique event ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Trace ID for correlation
    pub trace_id: String,
    /// Span ID for this event
    pub span_id: String,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID that executed the tool
    pub agent_id: Option<String>,

    /// Tool name (Read, Write, Bash, etc.)
    pub tool_name: String,
    /// Tool call ID from LLM
    pub tool_call_id: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Tool status
    pub status: ToolStatus,
    /// Error message if failed
    pub error: Option<String>,

    /// Input size in bytes
    pub input_size_bytes: u32,
    /// Output size in bytes
    pub output_size_bytes: u32,

    /// Additional metadata (filtered input/output for debugging)
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ToolExecutionEvent {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            trace_id: Uuid::new_v4().to_string(),
            span_id: generate_span_id(),
            parent_span_id: None,
            session_id: None,
            agent_id: None,
            tool_name: String::new(),
            tool_call_id: None,
            duration_ms: 0,
            status: ToolStatus::Success,
            error: None,
            input_size_bytes: 0,
            output_size_bytes: 0,
            metadata: HashMap::new(),
        }
    }
}

/// Agent lifecycle type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLifecycleType {
    /// Agent started
    Start,
    /// Agent completed successfully
    Complete,
    /// Agent failed with error
    Error,
    /// Agent forked (spawned subagent)
    Fork,
    /// Agent resumed from checkpoint
    Resume,
    /// Agent paused
    Pause,
    /// Agent cancelled
    Cancel,
}

impl std::fmt::Display for AgentLifecycleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentLifecycleType::Start => write!(f, "start"),
            AgentLifecycleType::Complete => write!(f, "complete"),
            AgentLifecycleType::Error => write!(f, "error"),
            AgentLifecycleType::Fork => write!(f, "fork"),
            AgentLifecycleType::Resume => write!(f, "resume"),
            AgentLifecycleType::Pause => write!(f, "pause"),
            AgentLifecycleType::Cancel => write!(f, "cancel"),
        }
    }
}

/// Agent lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLifecycleEvent {
    /// Unique event ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Trace ID for correlation
    pub trace_id: String,
    /// Span ID for this event
    pub span_id: String,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,

    /// Agent ID
    pub agent_id: String,
    /// Agent type (build, plan, code-reviewer, etc.)
    pub agent_type: String,
    /// Lifecycle type
    pub lifecycle_type: AgentLifecycleType,
    /// Parent agent ID (for Fork events)
    pub parent_agent_id: Option<String>,
    /// Duration in milliseconds (for Complete/Error events)
    pub duration_ms: Option<u64>,
    /// Error message (for Error events)
    pub error: Option<String>,
    /// Turn count (for Complete events)
    pub turn_count: Option<u32>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for AgentLifecycleEvent {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            trace_id: Uuid::new_v4().to_string(),
            span_id: generate_span_id(),
            parent_span_id: None,
            session_id: None,
            agent_id: String::new(),
            agent_type: String::new(),
            lifecycle_type: AgentLifecycleType::Start,
            parent_agent_id: None,
            duration_ms: None,
            error: None,
            turn_count: None,
            metadata: HashMap::new(),
        }
    }
}

/// Span kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanKind {
    /// Internal operation
    Internal,
    /// Client operation (outgoing call)
    Client,
    /// Server operation (incoming call)
    Server,
    /// Producer (async message send)
    Producer,
    /// Consumer (async message receive)
    Consumer,
}

impl Default for SpanKind {
    fn default() -> Self {
        SpanKind::Internal
    }
}

impl std::fmt::Display for SpanKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanKind::Internal => write!(f, "internal"),
            SpanKind::Client => write!(f, "client"),
            SpanKind::Server => write!(f, "server"),
            SpanKind::Producer => write!(f, "producer"),
            SpanKind::Consumer => write!(f, "consumer"),
        }
    }
}

/// Span event for nested tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Unique event ID
    pub id: String,
    /// Timestamp (start time)
    pub timestamp: DateTime<Utc>,
    /// Trace ID for correlation
    pub trace_id: String,
    /// Span ID for this event
    pub span_id: String,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID
    pub agent_id: Option<String>,

    /// Span name/operation
    pub name: String,
    /// Span kind
    pub kind: SpanKind,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Whether the span succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,

    /// Additional attributes
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Default for SpanEvent {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            trace_id: Uuid::new_v4().to_string(),
            span_id: generate_span_id(),
            parent_span_id: None,
            session_id: None,
            agent_id: None,
            name: String::new(),
            kind: SpanKind::Internal,
            duration_ms: 0,
            success: true,
            error: None,
            attributes: HashMap::new(),
        }
    }
}

/// Unified event enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// LLM call event
    LlmCall(LlmCallEvent),
    /// Tool execution event
    ToolExecution(ToolExecutionEvent),
    /// Agent lifecycle event
    AgentLifecycle(AgentLifecycleEvent),
    /// Span event
    Span(SpanEvent),
}

impl Event {
    /// Get the event type
    pub fn event_type(&self) -> EventType {
        match self {
            Event::LlmCall(_) => EventType::LlmCall,
            Event::ToolExecution(_) => EventType::ToolExecution,
            Event::AgentLifecycle(_) => EventType::AgentLifecycle,
            Event::Span(_) => EventType::Span,
        }
    }

    /// Get the event ID
    pub fn id(&self) -> &str {
        match self {
            Event::LlmCall(e) => &e.id,
            Event::ToolExecution(e) => &e.id,
            Event::AgentLifecycle(e) => &e.id,
            Event::Span(e) => &e.id,
        }
    }

    /// Get the timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Event::LlmCall(e) => e.timestamp,
            Event::ToolExecution(e) => e.timestamp,
            Event::AgentLifecycle(e) => e.timestamp,
            Event::Span(e) => e.timestamp,
        }
    }

    /// Get the trace ID
    pub fn trace_id(&self) -> &str {
        match self {
            Event::LlmCall(e) => &e.trace_id,
            Event::ToolExecution(e) => &e.trace_id,
            Event::AgentLifecycle(e) => &e.trace_id,
            Event::Span(e) => &e.trace_id,
        }
    }

    /// Get the span ID
    pub fn span_id(&self) -> &str {
        match self {
            Event::LlmCall(e) => &e.span_id,
            Event::ToolExecution(e) => &e.span_id,
            Event::AgentLifecycle(e) => &e.span_id,
            Event::Span(e) => &e.span_id,
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> Option<&str> {
        match self {
            Event::LlmCall(e) => e.session_id.as_deref(),
            Event::ToolExecution(e) => e.session_id.as_deref(),
            Event::AgentLifecycle(e) => e.session_id.as_deref(),
            Event::Span(e) => e.session_id.as_deref(),
        }
    }

    /// Get the agent ID
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            Event::LlmCall(e) => e.agent_id.as_deref(),
            Event::ToolExecution(e) => e.agent_id.as_deref(),
            Event::AgentLifecycle(e) => Some(&e.agent_id),
            Event::Span(e) => e.agent_id.as_deref(),
        }
    }
}

/// Generate an 8-character hex span ID
fn generate_span_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 4] = rng.gen();
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_call_event_default() {
        let event = LlmCallEvent::default();
        assert!(!event.id.is_empty());
        assert!(!event.trace_id.is_empty());
        assert_eq!(event.span_id.len(), 8);
        assert!(event.success);
    }

    #[test]
    fn test_tool_execution_event_default() {
        let event = ToolExecutionEvent::default();
        assert!(!event.id.is_empty());
        assert_eq!(event.status, ToolStatus::Success);
    }

    #[test]
    fn test_event_enum_serialization() {
        let event = Event::LlmCall(LlmCallEvent {
            provider: "anthropic".into(),
            model: "claude-opus-4-5".into(),
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"llm_call\""));
        assert!(json.contains("\"provider\":\"anthropic\""));
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::LlmCall.to_string(), "llm_call");
        assert_eq!(EventType::ToolExecution.to_string(), "tool_execution");
        assert_eq!(EventType::AgentLifecycle.to_string(), "agent_lifecycle");
        assert_eq!(EventType::Span.to_string(), "span");
    }
}
