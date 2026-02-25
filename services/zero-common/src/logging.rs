//! Logging utilities for Zero services.
//!
//! Provides structured JSON logging with trace IDs for observability.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// Initialize logging with the given configuration.
pub fn init_logging(log_level: &str, log_format: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = tracing_subscriber::registry().with(filter);

    if log_format == "json" {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_span_events(FmtSpan::CLOSE)
            .with_current_span(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true);
        let _ = subscriber.with(fmt_layer).try_init();
    } else {
        // Default to pretty format
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_target(true)
            .with_file(false)
            .with_line_number(false);
        let _ = subscriber.with(fmt_layer).try_init();
    }

    tracing::info!(log_level = %log_level, log_format = %log_format, "Logging initialized");
}

/// Generate a new trace ID for request tracing.
pub fn generate_trace_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Generate a new span ID for step tracing.
pub fn generate_span_id() -> String {
    uuid::Uuid::new_v4().to_string()[..8].to_string()
}

// ============================================================================
// Request Context
// ============================================================================

/// Request context for distributed tracing.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique trace ID for the request chain
    pub trace_id: String,
    /// Current span ID
    pub span_id: String,
    /// Parent span ID (if any)
    pub parent_span_id: Option<String>,
    /// Service name
    pub service: String,
    /// User ID (if authenticated)
    pub user_id: Option<String>,
    /// Additional baggage (key-value pairs propagated across services)
    pub baggage: std::collections::HashMap<String, String>,
}

impl RequestContext {
    /// Create a new request context.
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            parent_span_id: None,
            service: service.into(),
            user_id: None,
            baggage: std::collections::HashMap::new(),
        }
    }

    /// Create a child span context.
    pub fn child_span(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: generate_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            service: self.service.clone(),
            user_id: self.user_id.clone(),
            baggage: self.baggage.clone(),
        }
    }

    /// Create context from HTTP headers.
    pub fn from_headers(headers: &http::HeaderMap, service: impl Into<String>) -> Self {
        let trace_id = headers
            .get("X-Trace-Id")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .unwrap_or_else(generate_trace_id);

        let parent_span_id = headers
            .get("X-Span-Id")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let user_id = headers
            .get("X-User-Id")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Self {
            trace_id,
            span_id: generate_span_id(),
            parent_span_id,
            service: service.into(),
            user_id,
            baggage: std::collections::HashMap::new(),
        }
    }

    /// Add context to HTTP headers for propagation.
    pub fn to_headers(&self, headers: &mut http::HeaderMap) {
        if let Ok(trace_id) = self.trace_id.parse() {
            headers.insert("X-Trace-Id", trace_id);
        }
        if let Ok(span_id) = self.span_id.parse() {
            headers.insert("X-Span-Id", span_id);
        }
        if let Some(ref user_id) = self.user_id {
            if let Ok(user_id) = user_id.parse() {
                headers.insert("X-User-Id", user_id);
            }
        }
    }

    /// Log an event with this context.
    pub fn log_event(&self, event_type: LifecycleEventType, payload: serde_json::Value) {
        let event = LifecycleEvent::with_context(
            &self.trace_id,
            &self.span_id,
            self.parent_span_id.clone(),
            &self.service,
            event_type,
            payload,
        );
        event.log();
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new("unknown")
    }
}

// ============================================================================
// Lifecycle Events (ODD compliance)
// ============================================================================

/// Structured log event for lifecycle tracking (as per ODD requirements).
#[derive(Debug, Clone, serde::Serialize)]
pub struct LifecycleEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    pub event_type: LifecycleEventType,
    pub service: String,
    pub payload: serde_json::Value,
}

/// Types of lifecycle events for observability.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEventType {
    FunctionStart,
    FunctionEnd,
    Branch,
    Error,
    ExternalCall,
    ExternalCallResult,
    HttpRequest,
    HttpResponse,
    DatabaseQuery,
    CacheHit,
    CacheMiss,
}

impl LifecycleEvent {
    /// Create a new lifecycle event.
    pub fn new(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        event_type: LifecycleEventType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id: None,
            event_type,
            service: "zero-channels".to_string(),
            payload,
        }
    }

    /// Create a new lifecycle event with full context.
    pub fn with_context(
        trace_id: impl Into<String>,
        span_id: impl Into<String>,
        parent_span_id: Option<String>,
        service: impl Into<String>,
        event_type: LifecycleEventType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id,
            event_type,
            service: service.into(),
            payload,
        }
    }

    /// Log this event as raw JSON to stdout.
    /// This outputs a flat JSON structure matching the TypeScript format for ODD compliance.
    pub fn log(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{json}");
        }
    }
}

// ============================================================================
// Metrics Collection
// ============================================================================

/// Simple metrics collector for request tracking.
#[derive(Debug, Default)]
pub struct Metrics {
    inner: Arc<RwLock<MetricsInner>>,
}

#[derive(Debug, Default)]
struct MetricsInner {
    request_count: u64,
    error_count: u64,
    total_duration_ms: u64,
}

impl Metrics {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a request.
    pub async fn record_request(&self, duration_ms: u64, success: bool) {
        let mut inner = self.inner.write().await;
        inner.request_count += 1;
        inner.total_duration_ms += duration_ms;
        if !success {
            inner.error_count += 1;
        }
    }

    /// Get current metrics summary.
    pub async fn summary(&self) -> MetricsSummary {
        let inner = self.inner.read().await;
        MetricsSummary {
            request_count: inner.request_count,
            error_count: inner.error_count,
            avg_duration_ms: if inner.request_count > 0 {
                inner.total_duration_ms / inner.request_count
            } else {
                0
            },
        }
    }
}

/// Metrics summary for reporting.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSummary {
    pub request_count: u64,
    pub error_count: u64,
    pub avg_duration_ms: u64,
}

// ============================================================================
// Logging Macros
// ============================================================================

/// Log a function entry with context.
#[macro_export]
macro_rules! log_entry {
    ($ctx:expr, $func:expr) => {
        $ctx.log_event(
            $crate::logging::LifecycleEventType::FunctionStart,
            serde_json::json!({ "function": $func }),
        );
    };
    ($ctx:expr, $func:expr, $($key:literal : $value:expr),* $(,)?) => {
        $ctx.log_event(
            $crate::logging::LifecycleEventType::FunctionStart,
            serde_json::json!({ "function": $func, $($key: $value),* }),
        );
    };
}

/// Log a function exit with context.
#[macro_export]
macro_rules! log_exit {
    ($ctx:expr, $func:expr) => {
        $ctx.log_event(
            $crate::logging::LifecycleEventType::FunctionEnd,
            serde_json::json!({ "function": $func }),
        );
    };
    ($ctx:expr, $func:expr, $($key:literal : $value:expr),* $(,)?) => {
        $ctx.log_event(
            $crate::logging::LifecycleEventType::FunctionEnd,
            serde_json::json!({ "function": $func, $($key: $value),* }),
        );
    };
}

/// Log an error with context.
#[macro_export]
macro_rules! log_error {
    ($ctx:expr, $error:expr) => {
        $ctx.log_event(
            $crate::logging::LifecycleEventType::Error,
            serde_json::json!({ "error": $error.to_string() }),
        );
    };
    ($ctx:expr, $error:expr, $($key:literal : $value:expr),* $(,)?) => {
        $ctx.log_event(
            $crate::logging::LifecycleEventType::Error,
            serde_json::json!({ "error": $error.to_string(), $($key: $value),* }),
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_trace_id() {
        let id1 = generate_trace_id();
        let id2 = generate_trace_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID format
    }

    #[test]
    fn test_generate_span_id() {
        let id = generate_span_id();
        assert_eq!(id.len(), 8); // Short span ID
    }

    #[test]
    fn test_lifecycle_event() {
        let event = LifecycleEvent::new(
            "trace-123",
            "span-456",
            LifecycleEventType::FunctionStart,
            serde_json::json!({"function": "test"}),
        );
        assert_eq!(event.trace_id, "trace-123");
        assert_eq!(event.span_id, "span-456");
        assert_eq!(event.service, "zero-channels");
        assert!(event.parent_span_id.is_none());
    }

    #[test]
    fn test_lifecycle_event_with_context() {
        let event = LifecycleEvent::with_context(
            "trace-789",
            "span-abc",
            Some("parent-xyz".to_string()),
            "test-service",
            LifecycleEventType::FunctionEnd,
            serde_json::json!({"result": "ok"}),
        );
        assert_eq!(event.trace_id, "trace-789");
        assert_eq!(event.span_id, "span-abc");
        assert_eq!(event.parent_span_id, Some("parent-xyz".to_string()));
        assert_eq!(event.service, "test-service");
    }

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new("test-service");
        assert_eq!(ctx.service, "test-service");
        assert!(!ctx.trace_id.is_empty());
        assert!(!ctx.span_id.is_empty());
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_request_context_child_span() {
        let parent = RequestContext::new("test-service");
        let child = parent.child_span();

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let metrics = Metrics::new();
        metrics.record_request(100, true).await;
        metrics.record_request(200, false).await;

        let summary = metrics.summary().await;
        assert_eq!(summary.request_count, 2);
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.avg_duration_ms, 150);
    }
}
