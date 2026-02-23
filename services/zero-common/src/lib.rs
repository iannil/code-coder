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

#![warn(clippy::all)]
#![allow(clippy::pedantic)]

pub mod audit;
pub mod config;
pub mod error;
pub mod logging;
pub mod security;
pub mod util;
pub mod validation;

pub use audit::{
    AuditConfig, AuditEntry, AuditEventBuilder, AuditEventType, AuditLogger, ComplianceReport,
};
pub use config::{
    AgentConfig, ApiKeysConfig, ChannelsConfig, CodeCoderConfig, Config, GatewayConfig,
    MemoryConfig, ObservabilityConfig, OllamaConfig, ProvidersConfig, ReliabilityConfig,
    ToolsConfig, WorkflowConfig,
};
pub use error::{Error, Result};
pub use validation::{Validate, ValidationError, ValidationResult};

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::audit::{AuditConfig, AuditEventBuilder, AuditEventType, AuditLogger};
    pub use crate::config::{
        AgentConfig, ApiKeysConfig, Config, ProvidersConfig, ReliabilityConfig, ToolsConfig,
    };
    pub use crate::error::{Error, Result};
    pub use crate::logging::init_logging;
    pub use crate::validation::{Validate, ValidationError};
}
