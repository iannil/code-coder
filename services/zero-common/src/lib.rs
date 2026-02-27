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

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod audit;
pub mod bus;
pub mod config;
pub mod config_loader;
pub mod error;
#[cfg(feature = "hitl-client")]
pub mod hitl_client;
pub mod hybrid;
pub mod logging;
pub mod security;
pub mod util;
pub mod validation;

pub use audit::{
    AuditConfig, AuditEntry, AuditEventBuilder, AuditEventType, AuditLogger, ComplianceReport,
};
pub use bus::{
    create_bus, create_bus_async, topics, AgentRequestPayload, AgentResponsePayload, BusBackend,
    BusError, BusResult, ChannelMessagePayload, Event, EventBus, EventReceiver, InMemoryBus,
    RedisBus, RedisBusConfig, SessionPayload,
};
pub use config::{
    AgentConfig, ChannelsConfig, CodeCoderConfig, Config, GatewayConfig, LlmConfig,
    LlmOllamaConfig, LlmProviderConfig, MemoryConfig, ObservabilityConfig, ToolsConfig,
    WorkflowConfig,
};
pub use error::{Error, Result};
pub use hybrid::{
    AgentResult, AnalysisTrigger, DecisionSource, HybridConfig, HybridDecision,
    HybridDecisionMaker,
};
pub use validation::{Validate, ValidationError, ValidationResult};

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::audit::{AuditConfig, AuditEventBuilder, AuditEventType, AuditLogger};
    pub use crate::bus::{create_bus, topics, BusBackend, Event, EventBus};
    pub use crate::config::{AgentConfig, Config, LlmConfig, ToolsConfig};
    pub use crate::error::{Error, Result};
    pub use crate::hybrid::{DecisionSource, HybridConfig, HybridDecisionMaker};
    pub use crate::logging::init_logging;
    pub use crate::validation::{Validate, ValidationError};
}
