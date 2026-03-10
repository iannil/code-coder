//! Lightweight Tracing API (emit)
//!
//! Provides a simple, ergonomic API for emitting observability events
//! inspired by Agent Lightning's emit_xxx() pattern.
//!
//! # Design Principles
//!
//! 1. **Minimal overhead**: Events are buffered and can be batched
//! 2. **Span-based**: Tool calls use span IDs for correlation
//! 3. **OpenTelemetry compatible**: Event format maps to OTLP
//! 4. **Thread-safe**: Uses interior mutability for concurrent access
//!
//! # Example
//!
//! ```rust,ignore
//! let emitter = Emitter::new(observer);
//!
//! // Tool execution tracking
//! let span_id = emitter.tool_start("grep", &json!({"pattern": "TODO"}));
//! // ... execute tool ...
//! emitter.tool_end(span_id, &json!({"matches": 42}), 150);
//!
//! // State transitions
//! emitter.state_transition("idle", "executing", Some("User initiated"));
//!
//! // Agent decisions
//! emitter.agent_decision("macro", "proceed", 0.85);
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::traits::{Observer, ObserverEvent};

/// Unique identifier for a span (tool execution, agent call, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(pub u64);

impl SpanId {
    /// Generate a new unique span ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        SpanId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "span_{:016x}", self.0)
    }
}

/// Event types for the emit API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmitEvent {
    /// Tool execution started
    ToolStart {
        span_id: SpanId,
        tool: String,
        args: Value,
        timestamp: u64,
    },
    /// Tool execution ended
    ToolEnd {
        span_id: SpanId,
        tool: String,
        result: Value,
        duration_ms: u64,
        success: bool,
        timestamp: u64,
    },
    /// State machine transition
    StateTransition {
        from: String,
        to: String,
        reason: Option<String>,
        timestamp: u64,
    },
    /// Agent decision made
    AgentDecision {
        agent: String,
        decision: String,
        confidence: f32,
        metadata: Option<Value>,
        timestamp: u64,
    },
    /// Error occurred
    Error {
        component: String,
        message: String,
        context: Option<Value>,
        timestamp: u64,
    },
    /// Custom event
    Custom {
        name: String,
        payload: Value,
        timestamp: u64,
    },
}

impl EmitEvent {
    #[allow(dead_code)]
    fn timestamp(&self) -> u64 {
        match self {
            EmitEvent::ToolStart { timestamp, .. } => *timestamp,
            EmitEvent::ToolEnd { timestamp, .. } => *timestamp,
            EmitEvent::StateTransition { timestamp, .. } => *timestamp,
            EmitEvent::AgentDecision { timestamp, .. } => *timestamp,
            EmitEvent::Error { timestamp, .. } => *timestamp,
            EmitEvent::Custom { timestamp, .. } => *timestamp,
        }
    }
}

/// Active span tracking
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ActiveSpan {
    tool: String,
    started_at: Instant,
    args: Value,
}

/// Emitter configuration
#[derive(Debug, Clone)]
pub struct EmitterConfig {
    /// Maximum number of events to buffer before flush
    pub buffer_size: usize,
    /// Whether to auto-flush on buffer full
    pub auto_flush: bool,
    /// Include stack traces in error events
    pub include_traces: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            buffer_size: 1000,
            auto_flush: true,
            include_traces: false,
        }
    }
}

/// Lightweight tracing emitter
///
/// Thread-safe event emitter that supports span-based tracing
/// and integrates with the Observer backend.
pub struct Emitter {
    observer: Arc<dyn Observer>,
    config: EmitterConfig,
    buffer: RwLock<Vec<EmitEvent>>,
    active_spans: RwLock<HashMap<SpanId, ActiveSpan>>,
}

impl Emitter {
    /// Create a new emitter with the given observer backend
    pub fn new(observer: Arc<dyn Observer>) -> Self {
        Self::with_config(observer, EmitterConfig::default())
    }

    /// Create a new emitter with custom configuration
    pub fn with_config(observer: Arc<dyn Observer>, config: EmitterConfig) -> Self {
        Self {
            observer,
            config,
            buffer: RwLock::new(Vec::new()),
            active_spans: RwLock::new(HashMap::new()),
        }
    }

    /// Get current timestamp in milliseconds since Unix epoch
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Record a tool execution start, returns span ID for correlation
    pub fn tool_start(&self, tool: &str, args: &Value) -> SpanId {
        let span_id = SpanId::new();
        let timestamp = Self::now_ms();

        // Track active span
        {
            let mut spans = self.active_spans.write().unwrap();
            spans.insert(
                span_id,
                ActiveSpan {
                    tool: tool.to_string(),
                    started_at: Instant::now(),
                    args: args.clone(),
                },
            );
        }

        let event = EmitEvent::ToolStart {
            span_id,
            tool: tool.to_string(),
            args: args.clone(),
            timestamp,
        };

        self.emit(event);
        span_id
    }

    /// Record a tool execution end
    pub fn tool_end(&self, span_id: SpanId, result: &Value, duration_ms: u64) {
        self.tool_end_with_status(span_id, result, duration_ms, true)
    }

    /// Record a tool execution end with explicit success status
    pub fn tool_end_with_status(&self, span_id: SpanId, result: &Value, duration_ms: u64, success: bool) {
        let timestamp = Self::now_ms();

        // Get and remove active span
        let span = {
            let mut spans = self.active_spans.write().unwrap();
            spans.remove(&span_id)
        };

        let tool = span.map(|s| s.tool).unwrap_or_else(|| "unknown".to_string());

        let event = EmitEvent::ToolEnd {
            span_id,
            tool: tool.clone(),
            result: result.clone(),
            duration_ms,
            success,
            timestamp,
        };

        // Also record via Observer for backwards compatibility
        self.observer.record_event(&ObserverEvent::ToolCall {
            tool,
            duration: Duration::from_millis(duration_ms),
            success,
        });

        self.emit(event);
    }

    /// Record a state machine transition
    pub fn state_transition(&self, from: &str, to: &str, reason: Option<&str>) {
        let event = EmitEvent::StateTransition {
            from: from.to_string(),
            to: to.to_string(),
            reason: reason.map(String::from),
            timestamp: Self::now_ms(),
        };

        self.emit(event);
    }

    /// Record an agent decision
    pub fn agent_decision(&self, agent: &str, decision: &str, confidence: f32) {
        self.agent_decision_with_metadata(agent, decision, confidence, None)
    }

    /// Record an agent decision with additional metadata
    pub fn agent_decision_with_metadata(
        &self,
        agent: &str,
        decision: &str,
        confidence: f32,
        metadata: Option<Value>,
    ) {
        let event = EmitEvent::AgentDecision {
            agent: agent.to_string(),
            decision: decision.to_string(),
            confidence,
            metadata,
            timestamp: Self::now_ms(),
        };

        self.emit(event);
    }

    /// Record an error
    pub fn error(&self, component: &str, message: &str) {
        self.error_with_context(component, message, None)
    }

    /// Record an error with additional context
    pub fn error_with_context(&self, component: &str, message: &str, context: Option<Value>) {
        let event = EmitEvent::Error {
            component: component.to_string(),
            message: message.to_string(),
            context,
            timestamp: Self::now_ms(),
        };

        // Also record via Observer
        self.observer.record_event(&ObserverEvent::Error {
            component: component.to_string(),
            message: message.to_string(),
        });

        self.emit(event);
    }

    /// Record a custom event
    pub fn custom(&self, name: &str, payload: Value) {
        let event = EmitEvent::Custom {
            name: name.to_string(),
            payload,
            timestamp: Self::now_ms(),
        };

        self.emit(event);
    }

    /// Internal emit function
    fn emit(&self, event: EmitEvent) {
        let mut buffer = self.buffer.write().unwrap();
        buffer.push(event);

        if self.config.auto_flush && buffer.len() >= self.config.buffer_size {
            drop(buffer);
            self.flush();
        }
    }

    /// Flush buffered events
    pub fn flush(&self) {
        let events: Vec<EmitEvent> = {
            let mut buffer = self.buffer.write().unwrap();
            std::mem::take(&mut *buffer)
        };

        if events.is_empty() {
            return;
        }

        // Log events as JSON for downstream processing
        for event in &events {
            if let Ok(json) = serde_json::to_string(event) {
                tracing::info!(target: "emit", "{}", json);
            }
        }

        self.observer.flush();
    }

    /// Get the number of buffered events
    pub fn buffer_len(&self) -> usize {
        self.buffer.read().unwrap().len()
    }

    /// Get all buffered events (for testing/debugging)
    pub fn buffered_events(&self) -> Vec<EmitEvent> {
        self.buffer.read().unwrap().clone()
    }

    /// Clear the buffer without flushing
    pub fn clear(&self) {
        self.buffer.write().unwrap().clear();
    }

    /// Check if a span is still active
    pub fn is_span_active(&self, span_id: SpanId) -> bool {
        self.active_spans.read().unwrap().contains_key(&span_id)
    }

    /// Get the number of active spans
    pub fn active_span_count(&self) -> usize {
        self.active_spans.read().unwrap().len()
    }
}

// ============================================================================
// Global Emitter (for convenience)
// ============================================================================

use once_cell::sync::OnceCell;

static GLOBAL_EMITTER: OnceCell<Arc<Emitter>> = OnceCell::new();

/// Initialize the global emitter
pub fn init_global_emitter(observer: Arc<dyn Observer>) {
    let _ = GLOBAL_EMITTER.set(Arc::new(Emitter::new(observer)));
}

/// Get the global emitter (panics if not initialized)
pub fn global_emitter() -> &'static Arc<Emitter> {
    GLOBAL_EMITTER.get().expect("Global emitter not initialized")
}

/// Convenience functions for the global emitter
pub mod emit {
    use super::*;

    /// Record a tool execution start
    pub fn tool_start(tool: &str, args: &Value) -> SpanId {
        global_emitter().tool_start(tool, args)
    }

    /// Record a tool execution end
    pub fn tool_end(span_id: SpanId, result: &Value, duration_ms: u64) {
        global_emitter().tool_end(span_id, result, duration_ms)
    }

    /// Record a state transition
    pub fn state_transition(from: &str, to: &str, reason: Option<&str>) {
        global_emitter().state_transition(from, to, reason)
    }

    /// Record an agent decision
    pub fn agent_decision(agent: &str, decision: &str, confidence: f32) {
        global_emitter().agent_decision(agent, decision, confidence)
    }

    /// Record an error
    pub fn error(component: &str, message: &str) {
        global_emitter().error(component, message)
    }

    /// Record a custom event
    pub fn custom(name: &str, payload: Value) {
        global_emitter().custom(name, payload)
    }

    /// Flush all buffered events
    pub fn flush() {
        global_emitter().flush()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::NoopObserver;

    fn create_test_emitter() -> Emitter {
        Emitter::new(Arc::new(NoopObserver))
    }

    #[test]
    fn span_id_uniqueness() {
        let id1 = SpanId::new();
        let id2 = SpanId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn span_id_display() {
        let id = SpanId(0x123);
        assert!(id.to_string().starts_with("span_"));
    }

    #[test]
    fn tool_start_end_lifecycle() {
        let emitter = create_test_emitter();

        let span_id = emitter.tool_start("grep", &serde_json::json!({"pattern": "TODO"}));
        assert!(emitter.is_span_active(span_id));
        assert_eq!(emitter.active_span_count(), 1);

        emitter.tool_end(span_id, &serde_json::json!({"matches": 5}), 100);
        assert!(!emitter.is_span_active(span_id));
        assert_eq!(emitter.active_span_count(), 0);
    }

    #[test]
    fn events_buffered() {
        let config = EmitterConfig {
            buffer_size: 100,
            auto_flush: false,
            ..Default::default()
        };
        let emitter = Emitter::with_config(Arc::new(NoopObserver), config);

        emitter.state_transition("idle", "executing", Some("test"));
        emitter.agent_decision("macro", "proceed", 0.9);

        assert_eq!(emitter.buffer_len(), 2);
    }

    #[test]
    fn flush_clears_buffer() {
        let emitter = create_test_emitter();
        emitter.state_transition("a", "b", None);
        emitter.flush();
        assert_eq!(emitter.buffer_len(), 0);
    }

    #[test]
    fn error_event() {
        let emitter = create_test_emitter();
        emitter.error_with_context(
            "tool",
            "execution failed",
            Some(serde_json::json!({"code": 1})),
        );

        let events = emitter.buffered_events();
        assert_eq!(events.len(), 1);
        matches!(&events[0], EmitEvent::Error { .. });
    }

    #[test]
    fn custom_event() {
        let emitter = create_test_emitter();
        emitter.custom("user_action", serde_json::json!({"button": "submit"}));

        let events = emitter.buffered_events();
        assert_eq!(events.len(), 1);
        matches!(&events[0], EmitEvent::Custom { name, .. } if name == "user_action");
    }

    #[test]
    fn agent_decision_with_metadata() {
        let emitter = create_test_emitter();
        emitter.agent_decision_with_metadata(
            "trader",
            "buy",
            0.75,
            Some(serde_json::json!({"symbol": "AAPL", "quantity": 100})),
        );

        let events = emitter.buffered_events();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn multiple_active_spans() {
        let emitter = create_test_emitter();

        let s1 = emitter.tool_start("grep", &serde_json::json!({}));
        let s2 = emitter.tool_start("read", &serde_json::json!({}));
        let s3 = emitter.tool_start("edit", &serde_json::json!({}));

        assert_eq!(emitter.active_span_count(), 3);

        emitter.tool_end(s2, &serde_json::json!({}), 50);
        assert_eq!(emitter.active_span_count(), 2);
        assert!(emitter.is_span_active(s1));
        assert!(!emitter.is_span_active(s2));
        assert!(emitter.is_span_active(s3));
    }

    #[test]
    fn clear_buffer() {
        let emitter = create_test_emitter();
        emitter.state_transition("a", "b", None);
        emitter.state_transition("b", "c", None);
        assert_eq!(emitter.buffer_len(), 2);

        emitter.clear();
        assert_eq!(emitter.buffer_len(), 0);
    }

    #[test]
    fn event_timestamps_are_recent() {
        let emitter = create_test_emitter();
        let before = Emitter::now_ms();
        emitter.state_transition("a", "b", None);
        let after = Emitter::now_ms();

        let events = emitter.buffered_events();
        let ts = events[0].timestamp();
        assert!(ts >= before && ts <= after);
    }
}
