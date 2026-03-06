//! Audit module - structured audit logging
//!
//! This module provides:
//! - **log**: Append-only audit logging with SQLite backend
//!
//! Provides tamper-evident logging for autonomous mode operations.

pub mod log;

// Re-export main types
pub use log::{
    AuditEntry, AuditEntryInput, AuditEntryType, AuditFilter, AuditLog, AuditReport,
    AuditResult, RiskLevel,
};
