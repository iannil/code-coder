//! NAPI bindings for the observability module
//!
//! Exposes observability functions to Node.js via napi-rs.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

use crate::observability::{
    AgentLifecycleEvent as RustAgentLifecycleEvent,
    AgentLifecycleType as RustAgentLifecycleType,
    EventType as RustEventType,
    LlmCallEvent as RustLlmCallEvent,
    MetricsSummary as RustMetricsSummary,
    ObservabilityStore as RustObservabilityStore,
    SpanEvent as RustSpanEvent,
    SpanKind as RustSpanKind,
    ToolExecutionEvent as RustToolExecutionEvent,
    ToolStatus as RustToolStatus,
};

// ============================================================================
// Event Types
// ============================================================================

/// Event type enum
#[napi(string_enum)]
pub enum NapiEventType {
    LlmCall,
    ToolExecution,
    AgentLifecycle,
    Span,
}

impl From<RustEventType> for NapiEventType {
    fn from(t: RustEventType) -> Self {
        match t {
            RustEventType::LlmCall => NapiEventType::LlmCall,
            RustEventType::ToolExecution => NapiEventType::ToolExecution,
            RustEventType::AgentLifecycle => NapiEventType::AgentLifecycle,
            RustEventType::Span => NapiEventType::Span,
        }
    }
}

/// Tool status enum
#[napi(string_enum)]
pub enum NapiToolStatus {
    Success,
    Error,
    Cancelled,
    Timeout,
    Blocked,
}

impl From<NapiToolStatus> for RustToolStatus {
    fn from(s: NapiToolStatus) -> Self {
        match s {
            NapiToolStatus::Success => RustToolStatus::Success,
            NapiToolStatus::Error => RustToolStatus::Error,
            NapiToolStatus::Cancelled => RustToolStatus::Cancelled,
            NapiToolStatus::Timeout => RustToolStatus::Timeout,
            NapiToolStatus::Blocked => RustToolStatus::Blocked,
        }
    }
}

impl From<RustToolStatus> for NapiToolStatus {
    fn from(s: RustToolStatus) -> Self {
        match s {
            RustToolStatus::Success => NapiToolStatus::Success,
            RustToolStatus::Error => NapiToolStatus::Error,
            RustToolStatus::Cancelled => NapiToolStatus::Cancelled,
            RustToolStatus::Timeout => NapiToolStatus::Timeout,
            RustToolStatus::Blocked => NapiToolStatus::Blocked,
        }
    }
}

/// Agent lifecycle type enum
#[napi(string_enum)]
pub enum NapiAgentLifecycleType {
    Start,
    Complete,
    Error,
    Fork,
    Resume,
    Pause,
    Cancel,
}

impl From<NapiAgentLifecycleType> for RustAgentLifecycleType {
    fn from(t: NapiAgentLifecycleType) -> Self {
        match t {
            NapiAgentLifecycleType::Start => RustAgentLifecycleType::Start,
            NapiAgentLifecycleType::Complete => RustAgentLifecycleType::Complete,
            NapiAgentLifecycleType::Error => RustAgentLifecycleType::Error,
            NapiAgentLifecycleType::Fork => RustAgentLifecycleType::Fork,
            NapiAgentLifecycleType::Resume => RustAgentLifecycleType::Resume,
            NapiAgentLifecycleType::Pause => RustAgentLifecycleType::Pause,
            NapiAgentLifecycleType::Cancel => RustAgentLifecycleType::Cancel,
        }
    }
}

impl From<RustAgentLifecycleType> for NapiAgentLifecycleType {
    fn from(t: RustAgentLifecycleType) -> Self {
        match t {
            RustAgentLifecycleType::Start => NapiAgentLifecycleType::Start,
            RustAgentLifecycleType::Complete => NapiAgentLifecycleType::Complete,
            RustAgentLifecycleType::Error => NapiAgentLifecycleType::Error,
            RustAgentLifecycleType::Fork => NapiAgentLifecycleType::Fork,
            RustAgentLifecycleType::Resume => NapiAgentLifecycleType::Resume,
            RustAgentLifecycleType::Pause => NapiAgentLifecycleType::Pause,
            RustAgentLifecycleType::Cancel => NapiAgentLifecycleType::Cancel,
        }
    }
}

/// Span kind enum
#[napi(string_enum)]
pub enum NapiSpanKind {
    Internal,
    Client,
    Server,
    Producer,
    Consumer,
}

impl From<NapiSpanKind> for RustSpanKind {
    fn from(k: NapiSpanKind) -> Self {
        match k {
            NapiSpanKind::Internal => RustSpanKind::Internal,
            NapiSpanKind::Client => RustSpanKind::Client,
            NapiSpanKind::Server => RustSpanKind::Server,
            NapiSpanKind::Producer => RustSpanKind::Producer,
            NapiSpanKind::Consumer => RustSpanKind::Consumer,
        }
    }
}

// ============================================================================
// Event Objects
// ============================================================================

/// LLM call event input
#[napi(object)]
pub struct NapiLlmCallEvent {
    /// Trace ID for correlation
    pub trace_id: Option<String>,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID that made the call
    pub agent_id: Option<String>,
    /// LLM provider
    pub provider: String,
    /// Model ID
    pub model: String,
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Cache read tokens
    pub cache_read_tokens: Option<u32>,
    /// Cache write tokens
    pub cache_write_tokens: Option<u32>,
    /// Latency in milliseconds
    pub latency_ms: i64,
    /// Cost in USD
    pub cost_usd: f64,
    /// Whether the call succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Stop reason
    pub stop_reason: Option<String>,
}

impl From<NapiLlmCallEvent> for RustLlmCallEvent {
    fn from(e: NapiLlmCallEvent) -> Self {
        let mut event = RustLlmCallEvent::default();
        if let Some(trace_id) = e.trace_id {
            event.trace_id = trace_id;
        }
        event.parent_span_id = e.parent_span_id;
        event.session_id = e.session_id;
        event.agent_id = e.agent_id;
        event.provider = e.provider;
        event.model = e.model;
        event.input_tokens = e.input_tokens;
        event.output_tokens = e.output_tokens;
        event.cache_read_tokens = e.cache_read_tokens.unwrap_or(0);
        event.cache_write_tokens = e.cache_write_tokens.unwrap_or(0);
        event.latency_ms = e.latency_ms as u64;
        event.cost_usd = e.cost_usd;
        event.success = e.success;
        event.error = e.error;
        event.stop_reason = e.stop_reason;
        event
    }
}

/// Tool execution event input
#[napi(object)]
pub struct NapiToolExecutionEvent {
    /// Trace ID for correlation
    pub trace_id: Option<String>,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID that executed the tool
    pub agent_id: Option<String>,
    /// Tool name
    pub tool_name: String,
    /// Tool call ID from LLM
    pub tool_call_id: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: i64,
    /// Tool status
    pub status: NapiToolStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Input size in bytes
    pub input_size_bytes: Option<u32>,
    /// Output size in bytes
    pub output_size_bytes: Option<u32>,
}

impl From<NapiToolExecutionEvent> for RustToolExecutionEvent {
    fn from(e: NapiToolExecutionEvent) -> Self {
        let mut event = RustToolExecutionEvent::default();
        if let Some(trace_id) = e.trace_id {
            event.trace_id = trace_id;
        }
        event.parent_span_id = e.parent_span_id;
        event.session_id = e.session_id;
        event.agent_id = e.agent_id;
        event.tool_name = e.tool_name;
        event.tool_call_id = e.tool_call_id;
        event.duration_ms = e.duration_ms as u64;
        event.status = e.status.into();
        event.error = e.error;
        event.input_size_bytes = e.input_size_bytes.unwrap_or(0);
        event.output_size_bytes = e.output_size_bytes.unwrap_or(0);
        event
    }
}

/// Agent lifecycle event input
#[napi(object)]
pub struct NapiAgentLifecycleEvent {
    /// Trace ID for correlation
    pub trace_id: Option<String>,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID
    pub agent_id: String,
    /// Agent type
    pub agent_type: String,
    /// Lifecycle type
    pub lifecycle_type: NapiAgentLifecycleType,
    /// Parent agent ID (for Fork events)
    pub parent_agent_id: Option<String>,
    /// Duration in milliseconds (for Complete/Error events)
    pub duration_ms: Option<i64>,
    /// Error message (for Error events)
    pub error: Option<String>,
    /// Turn count (for Complete events)
    pub turn_count: Option<u32>,
}

impl From<NapiAgentLifecycleEvent> for RustAgentLifecycleEvent {
    fn from(e: NapiAgentLifecycleEvent) -> Self {
        let mut event = RustAgentLifecycleEvent::default();
        if let Some(trace_id) = e.trace_id {
            event.trace_id = trace_id;
        }
        event.parent_span_id = e.parent_span_id;
        event.session_id = e.session_id;
        event.agent_id = e.agent_id;
        event.agent_type = e.agent_type;
        event.lifecycle_type = e.lifecycle_type.into();
        event.parent_agent_id = e.parent_agent_id;
        event.duration_ms = e.duration_ms.map(|d| d as u64);
        event.error = e.error;
        event.turn_count = e.turn_count;
        event
    }
}

/// Span event input
#[napi(object)]
pub struct NapiSpanEvent {
    /// Trace ID for correlation
    pub trace_id: Option<String>,
    /// Parent span ID (if nested)
    pub parent_span_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// Agent ID
    pub agent_id: Option<String>,
    /// Span name/operation
    pub name: String,
    /// Span kind
    pub kind: Option<NapiSpanKind>,
    /// Duration in milliseconds
    pub duration_ms: i64,
    /// Whether the span succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl From<NapiSpanEvent> for RustSpanEvent {
    fn from(e: NapiSpanEvent) -> Self {
        let mut event = RustSpanEvent::default();
        if let Some(trace_id) = e.trace_id {
            event.trace_id = trace_id;
        }
        event.parent_span_id = e.parent_span_id;
        event.session_id = e.session_id;
        event.agent_id = e.agent_id;
        event.name = e.name;
        event.kind = e.kind.map(Into::into).unwrap_or_default();
        event.duration_ms = e.duration_ms as u64;
        event.success = e.success;
        event.error = e.error;
        event
    }
}

// ============================================================================
// Metrics Objects
// ============================================================================

/// LLM metrics
#[napi(object)]
pub struct NapiLlmMetrics {
    pub total_calls: i64,
    pub successful_calls: i64,
    pub failed_calls: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub total_latency_ms: i64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: i64,
    pub p95_latency_ms: i64,
    pub p99_latency_ms: i64,
    pub total_cost_usd: f64,
    pub avg_cost_per_call_usd: f64,
    pub cache_hit_rate: f64,
    pub success_rate: f64,
}

/// Tool metrics
#[napi(object)]
pub struct NapiToolMetrics {
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub blocked_executions: i64,
    pub timeout_executions: i64,
    pub cancelled_executions: i64,
    pub total_duration_ms: i64,
    pub avg_duration_ms: f64,
    pub p50_duration_ms: i64,
    pub p95_duration_ms: i64,
    pub total_input_bytes: i64,
    pub total_output_bytes: i64,
    pub success_rate: f64,
}

/// Agent metrics
#[napi(object)]
pub struct NapiAgentMetrics {
    pub total_starts: i64,
    pub total_completions: i64,
    pub total_errors: i64,
    pub total_forks: i64,
    pub avg_turns: f64,
    pub avg_duration_ms: f64,
    pub completion_rate: f64,
}

/// Metrics summary
#[napi(object)]
pub struct NapiMetricsSummary {
    pub from_ts: String,
    pub to_ts: String,
    pub total_events: i64,
    pub llm: NapiLlmMetrics,
    pub tools: NapiToolMetrics,
    pub agents: NapiAgentMetrics,
}

impl From<RustMetricsSummary> for NapiMetricsSummary {
    fn from(m: RustMetricsSummary) -> Self {
        Self {
            from_ts: m.from.to_rfc3339(),
            to_ts: m.to.to_rfc3339(),
            total_events: m.total_events as i64,
            llm: NapiLlmMetrics {
                total_calls: m.llm.total_calls as i64,
                successful_calls: m.llm.successful_calls as i64,
                failed_calls: m.llm.failed_calls as i64,
                total_input_tokens: m.llm.total_input_tokens as i64,
                total_output_tokens: m.llm.total_output_tokens as i64,
                total_cache_read_tokens: m.llm.total_cache_read_tokens as i64,
                total_cache_write_tokens: m.llm.total_cache_write_tokens as i64,
                total_latency_ms: m.llm.total_latency_ms as i64,
                avg_latency_ms: m.llm.avg_latency_ms,
                p50_latency_ms: m.llm.p50_latency_ms as i64,
                p95_latency_ms: m.llm.p95_latency_ms as i64,
                p99_latency_ms: m.llm.p99_latency_ms as i64,
                total_cost_usd: m.llm.total_cost_usd,
                avg_cost_per_call_usd: m.llm.avg_cost_per_call_usd,
                cache_hit_rate: m.llm.cache_hit_rate,
                success_rate: m.llm.success_rate,
            },
            tools: NapiToolMetrics {
                total_executions: m.tools.total_executions as i64,
                successful_executions: m.tools.successful_executions as i64,
                failed_executions: m.tools.failed_executions as i64,
                blocked_executions: m.tools.blocked_executions as i64,
                timeout_executions: m.tools.timeout_executions as i64,
                cancelled_executions: m.tools.cancelled_executions as i64,
                total_duration_ms: m.tools.total_duration_ms as i64,
                avg_duration_ms: m.tools.avg_duration_ms,
                p50_duration_ms: m.tools.p50_duration_ms as i64,
                p95_duration_ms: m.tools.p95_duration_ms as i64,
                total_input_bytes: m.tools.total_input_bytes as i64,
                total_output_bytes: m.tools.total_output_bytes as i64,
                success_rate: m.tools.success_rate,
            },
            agents: NapiAgentMetrics {
                total_starts: m.agents.total_starts as i64,
                total_completions: m.agents.total_completions as i64,
                total_errors: m.agents.total_errors as i64,
                total_forks: m.agents.total_forks as i64,
                avg_turns: m.agents.avg_turns,
                avg_duration_ms: m.agents.avg_duration_ms,
                completion_rate: m.agents.completion_rate,
            },
        }
    }
}

/// Store statistics
#[napi(object)]
pub struct NapiObservabilityStoreStats {
    pub total_events: i64,
    pub llm_calls: i64,
    pub tool_executions: i64,
    pub agent_events: i64,
    pub total_cost_usd: f64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub oldest_ts: Option<String>,
    pub newest_ts: Option<String>,
}

// ============================================================================
// Store Handle
// ============================================================================

/// Handle to an ObservabilityStore
#[napi]
pub struct ObservabilityStoreHandle {
    inner: Arc<Mutex<RustObservabilityStore>>,
}

/// Open an observability store at the given path
#[napi]
pub fn open_observability_store(path: String) -> Result<ObservabilityStoreHandle> {
    let store = RustObservabilityStore::open(&path)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(ObservabilityStoreHandle {
        inner: Arc::new(Mutex::new(store)),
    })
}

/// Create an in-memory observability store (for testing)
#[napi]
pub fn create_memory_observability_store() -> Result<ObservabilityStoreHandle> {
    let store = RustObservabilityStore::in_memory()
        .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(ObservabilityStoreHandle {
        inner: Arc::new(Mutex::new(store)),
    })
}

#[napi]
impl ObservabilityStoreHandle {
    /// Emit an LLM call event
    #[napi]
    pub fn emit_llm_call(&self, event: NapiLlmCallEvent) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.emit_llm_call(event.into())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Emit a tool execution event
    #[napi]
    pub fn emit_tool_execution(&self, event: NapiToolExecutionEvent) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.emit_tool_execution(event.into())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Emit an agent lifecycle event
    #[napi]
    pub fn emit_agent_lifecycle(&self, event: NapiAgentLifecycleEvent) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.emit_agent_lifecycle(event.into())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Emit a span event
    #[napi]
    pub fn emit_span(&self, event: NapiSpanEvent) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.emit_span(event.into())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get total cost for a time period
    #[napi]
    pub fn total_cost(&self, from_ts: String, to_ts: String) -> Result<f64> {
        use chrono::{DateTime, Utc};

        let from = DateTime::parse_from_rfc3339(&from_ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid from_ts: {}", e)))?;
        let to = DateTime::parse_from_rfc3339(&to_ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid to_ts: {}", e)))?;

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.total_cost(from, to)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get total tokens for a time period
    #[napi]
    pub fn total_tokens(&self, from_ts: String, to_ts: String) -> Result<Vec<i64>> {
        use chrono::{DateTime, Utc};

        let from = DateTime::parse_from_rfc3339(&from_ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid from_ts: {}", e)))?;
        let to = DateTime::parse_from_rfc3339(&to_ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid to_ts: {}", e)))?;

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let (input, output) = store.total_tokens(from, to)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(vec![input as i64, output as i64])
    }

    /// Aggregate metrics for a time period
    #[napi]
    pub fn aggregate_metrics(&self, from_ts: String, to_ts: String) -> Result<NapiMetricsSummary> {
        use chrono::{DateTime, Utc};

        let from = DateTime::parse_from_rfc3339(&from_ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid from_ts: {}", e)))?;
        let to = DateTime::parse_from_rfc3339(&to_ts)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|e| Error::from_reason(format!("Invalid to_ts: {}", e)))?;

        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let metrics = store.aggregate_metrics(from, to)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(metrics.into())
    }

    /// Get store statistics
    #[napi]
    pub fn stats(&self) -> Result<NapiObservabilityStoreStats> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let stats = store.stats()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(NapiObservabilityStoreStats {
            total_events: stats.total_events as i64,
            llm_calls: stats.llm_calls as i64,
            tool_executions: stats.tool_executions as i64,
            agent_events: stats.agent_events as i64,
            total_cost_usd: stats.total_cost_usd,
            total_input_tokens: stats.total_input_tokens as i64,
            total_output_tokens: stats.total_output_tokens as i64,
            oldest_ts: stats.oldest_ts,
            newest_ts: stats.newest_ts,
        })
    }

    /// Clean up old events
    #[napi]
    pub fn cleanup(&self) -> Result<u32> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let deleted = store.cleanup()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(deleted as u32)
    }

    /// Compact the database
    #[napi]
    pub fn compact(&self) -> Result<()> {
        let store = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        store.compact()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Health check
    #[napi]
    pub fn health_check(&self) -> bool {
        self.inner
            .lock()
            .map(|store| store.health_check())
            .unwrap_or(false)
    }
}
