//! Observability module for CodeCoder
//!
//! Provides lightweight, zero-intrusion execution tracking inspired by Agent Lightning.
//! Builds on the trace module to add specialized event types and metrics aggregation.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                        Observability System                             │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
//! │  │   Events     │  │   Metrics    │  │    Store     │                  │
//! │  │  (emit_xxx)  │  │ (aggregate)  │  │  (SQLite)    │                  │
//! │  └──────────────┘  └──────────────┘  └──────────────┘                  │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Event Types
//!
//! - `llm_call`: LLM API calls with model, tokens, latency, cost
//! - `tool_execution`: Tool invocations with name, args, result, duration
//! - `agent_lifecycle`: Agent start, stop, error, fork events
//! - `span`: Nested spans for tracing execution flow
//!
//! # Usage
//!
//! ```rust,ignore
//! use zero_core::observability::{ObservabilityStore, Event, LlmCallEvent};
//!
//! let store = ObservabilityStore::open("~/.codecoder/observability.db")?;
//!
//! // Emit an LLM call event
//! store.emit_llm_call(LlmCallEvent {
//!     provider: "anthropic".into(),
//!     model: "claude-opus-4-5".into(),
//!     input_tokens: 1500,
//!     output_tokens: 500,
//!     latency_ms: 2500,
//!     cost_usd: 0.03,
//!     ..Default::default()
//! })?;
//!
//! // Get metrics
//! let metrics = store.aggregate_metrics()?;
//! println!("Total cost: ${:.4}", metrics.total_cost_usd);
//! ```

pub mod event;
pub mod metrics;
pub mod store;

pub use event::{
    AgentLifecycleEvent, AgentLifecycleType, Event, EventType, LlmCallEvent, SpanEvent, SpanKind,
    ToolExecutionEvent, ToolStatus,
};
pub use metrics::{AgentMetrics, MetricsAggregator, MetricsSummary, ModelMetrics, ToolMetrics};
pub use store::{ObservabilityStore, ObservabilityStoreConfig};
