//! Zero Common - Shared types, utilities, and configuration for the Zero ecosystem.
//!
//! This crate provides:
//! - Configuration types and loading
//! - Configuration validation
//! - Error types and handling utilities
//! - Logging setup and structured logging helpers
//! - Utility functions used across Zero services
//! - Security primitives (secrets, encryption)
//! - Audit logging for compliance
//! - Event bus for inter-service communication
//! - HTTP client factory with unified timeout management

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod audit;
pub mod bus;
pub mod config;
pub mod config_loader;
pub mod error;
pub mod events;
pub mod guardrails;
#[cfg(feature = "hitl-client")]
pub mod hitl_client;
pub mod keywords;
pub mod messages;
#[cfg(feature = "http-client")]
pub mod http_client;
pub mod hybrid;
pub mod logging;
pub mod notification;
pub mod metrics;
pub mod redis;
pub mod security;
#[cfg(feature = "axum")]
pub mod tracing_middleware;
pub mod util;
pub mod validation;

/// Test utilities for Zero ecosystem (available in tests)
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use audit::{
    AuditConfig, AuditEntry, AuditEventBuilder, AuditEventType, AuditLogger, ComplianceReport,
};
pub use bus::{
    create_bus, create_bus_async, topics, AgentRequestPayload, AgentResponsePayload, BusBackend,
    BusError, BusResult, ChannelMessagePayload, Event, EventBus, EventReceiver, InMemoryBus,
    RedisBus, RedisBusConfig, SessionPayload,
};
pub use config::{
    AgentConfig, ChannelsConfig, CodeCoderConfig, Config, GatewayConfig, HandNotificationConfig,
    LlmConfig, LlmOllamaConfig, LlmProviderConfig, MemoryConfig, MonitorNotificationConfig,
    ObservabilityConfig, TimeoutConfig, ToolsConfig, WorkflowConfig,
};
pub use error::{Error, Result};
pub use guardrails::{
    Action, ActionCategory, ApprovalRequest, ApprovalStatus, Decision, Guardrails,
    GuardrailsConfig, RiskLevel,
};
pub use hybrid::{
    AgentResult, AnalysisTrigger, DecisionSource, HybridConfig, HybridDecision,
    HybridDecisionMaker,
};
pub use keywords::{
    detect_agent, detect_alias, detect_trigger, keywords, load_keywords, AgentKeywords,
    DefaultsConfig, KeywordsConfig, TriggerRule, TriggerType,
};
pub use messages::{
    messages, load_messages, t, AllMessages, ApprovalMessages, AuthMessages, AutonomousMessages,
    ContextMessages, ErrorMessages, MessagesConfig, SearchMessages, StatusMessages, TaskMessages,
};
pub use notification::NotificationSink;
pub use validation::{Validate, ValidationError, ValidationResult};

// Event sourcing types
pub use events::{
    AgentInfoData, AgentSwitchData, ConfirmationData, DebugInfoData, HeartbeatData, OutputData,
    ProgressData, SkillUseData, StreamEvent, TaskCompletedData, TaskCreatedData, TaskEvent,
    TaskFailedData, TaskStartedData, TaskState, TaskStatus, TaskUsage, ThoughtData, ToolUseData,
};

// Redis Streams types
pub use redis::{
    stream_keys, PendingMessage, RedisStreamClient, RedisStreamConfig, RedisStreamError,
    StreamMessage, StreamResult,
};

pub use metrics::{MetricsRegistry, MetricsSnapshot};

#[cfg(feature = "http-client")]
pub use http_client::{build_client, build_client_with_timeout, default_client, ClientCategory};

#[cfg(feature = "axum")]
pub use tracing_middleware::{TracingExt, TracingLayer, tracing_middleware};

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::audit::{AuditConfig, AuditEventBuilder, AuditEventType, AuditLogger};
    pub use crate::bus::{create_bus, topics, BusBackend, Event, EventBus};
    pub use crate::config::{AgentConfig, Config, LlmConfig, TimeoutConfig, ToolsConfig};
    pub use crate::error::{Error, Result};
    pub use crate::events::{StreamEvent, TaskEvent, TaskState, TaskStatus};
    pub use crate::guardrails::{Action, Decision, Guardrails, RiskLevel};
    pub use crate::hybrid::{DecisionSource, HybridConfig, HybridDecisionMaker};
    pub use crate::keywords::{detect_agent, detect_alias, keywords, KeywordsConfig};
    pub use crate::messages::{messages, t, MessagesConfig};
    #[cfg(feature = "http-client")]
    pub use crate::http_client::{build_client, ClientCategory};
    pub use crate::logging::init_logging;
    pub use crate::notification::NotificationSink;
    pub use crate::redis::{stream_keys, RedisStreamClient, RedisStreamConfig};
    pub use crate::validation::{Validate, ValidationError};
}
