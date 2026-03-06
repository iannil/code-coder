//! NAPI bindings for audit module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - Audit logging
//! - Audit querying
//! - Report generation

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audit::log::{
    AuditEntry as RustAuditEntry, AuditEntryInput as RustAuditEntryInput,
    AuditEntryType as RustAuditEntryType, AuditFilter as RustAuditFilter,
    AuditLog as RustAuditLog, AuditReport as RustAuditReport, AuditResult as RustAuditResult,
    AuditSummary as RustAuditSummary, RiskLevel as RustRiskLevel, TimeRange as RustTimeRange,
    TypeSummary as RustTypeSummary, ResultSummary as RustResultSummary, RiskSummary as RustRiskSummary,
};

// ============================================================================
// Enums (NAPI)
// ============================================================================

/// Audit entry type for NAPI
#[napi(string_enum)]
pub enum NapiAuditEntryType {
    Permission,
    ToolCall,
    Decision,
    StateChange,
    Checkpoint,
    Rollback,
    Error,
    SessionStart,
    SessionEnd,
}

impl From<NapiAuditEntryType> for RustAuditEntryType {
    fn from(t: NapiAuditEntryType) -> Self {
        match t {
            NapiAuditEntryType::Permission => RustAuditEntryType::Permission,
            NapiAuditEntryType::ToolCall => RustAuditEntryType::ToolCall,
            NapiAuditEntryType::Decision => RustAuditEntryType::Decision,
            NapiAuditEntryType::StateChange => RustAuditEntryType::StateChange,
            NapiAuditEntryType::Checkpoint => RustAuditEntryType::Checkpoint,
            NapiAuditEntryType::Rollback => RustAuditEntryType::Rollback,
            NapiAuditEntryType::Error => RustAuditEntryType::Error,
            NapiAuditEntryType::SessionStart => RustAuditEntryType::SessionStart,
            NapiAuditEntryType::SessionEnd => RustAuditEntryType::SessionEnd,
        }
    }
}

impl From<RustAuditEntryType> for NapiAuditEntryType {
    fn from(t: RustAuditEntryType) -> Self {
        match t {
            RustAuditEntryType::Permission => NapiAuditEntryType::Permission,
            RustAuditEntryType::ToolCall => NapiAuditEntryType::ToolCall,
            RustAuditEntryType::Decision => NapiAuditEntryType::Decision,
            RustAuditEntryType::StateChange => NapiAuditEntryType::StateChange,
            RustAuditEntryType::Checkpoint => NapiAuditEntryType::Checkpoint,
            RustAuditEntryType::Rollback => NapiAuditEntryType::Rollback,
            RustAuditEntryType::Error => NapiAuditEntryType::Error,
            RustAuditEntryType::SessionStart => NapiAuditEntryType::SessionStart,
            RustAuditEntryType::SessionEnd => NapiAuditEntryType::SessionEnd,
        }
    }
}

/// Audit result for NAPI
#[napi(string_enum)]
pub enum NapiAuditResult {
    Approved,
    Rejected,
    Error,
    Success,
    Failed,
}

impl From<NapiAuditResult> for RustAuditResult {
    fn from(r: NapiAuditResult) -> Self {
        match r {
            NapiAuditResult::Approved => RustAuditResult::Approved,
            NapiAuditResult::Rejected => RustAuditResult::Rejected,
            NapiAuditResult::Error => RustAuditResult::Error,
            NapiAuditResult::Success => RustAuditResult::Success,
            NapiAuditResult::Failed => RustAuditResult::Failed,
        }
    }
}

impl From<RustAuditResult> for NapiAuditResult {
    fn from(r: RustAuditResult) -> Self {
        match r {
            RustAuditResult::Approved => NapiAuditResult::Approved,
            RustAuditResult::Rejected => NapiAuditResult::Rejected,
            RustAuditResult::Error => NapiAuditResult::Error,
            RustAuditResult::Success => NapiAuditResult::Success,
            RustAuditResult::Failed => NapiAuditResult::Failed,
        }
    }
}

/// Risk level for NAPI
#[napi(string_enum)]
pub enum NapiRiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl From<NapiRiskLevel> for RustRiskLevel {
    fn from(r: NapiRiskLevel) -> Self {
        match r {
            NapiRiskLevel::Safe => RustRiskLevel::Safe,
            NapiRiskLevel::Low => RustRiskLevel::Low,
            NapiRiskLevel::Medium => RustRiskLevel::Medium,
            NapiRiskLevel::High => RustRiskLevel::High,
            NapiRiskLevel::Critical => RustRiskLevel::Critical,
        }
    }
}

impl From<RustRiskLevel> for NapiRiskLevel {
    fn from(r: RustRiskLevel) -> Self {
        match r {
            RustRiskLevel::Safe => NapiRiskLevel::Safe,
            RustRiskLevel::Low => NapiRiskLevel::Low,
            RustRiskLevel::Medium => NapiRiskLevel::Medium,
            RustRiskLevel::High => NapiRiskLevel::High,
            RustRiskLevel::Critical => NapiRiskLevel::Critical,
        }
    }
}

// ============================================================================
// Types (NAPI)
// ============================================================================

/// Audit entry for NAPI
#[napi(object)]
pub struct NapiAuditEntry {
    pub id: String,
    pub timestamp: i64,
    pub session_id: String,
    pub entry_type: NapiAuditEntryType,
    pub action: String,
    pub input: String, // JSON string
    pub result: NapiAuditResult,
    pub risk: Option<NapiRiskLevel>,
    pub auto_approved: Option<bool>,
    pub reason: String,
    pub metadata: String, // JSON string
}

impl From<RustAuditEntry> for NapiAuditEntry {
    fn from(e: RustAuditEntry) -> Self {
        Self {
            id: e.id,
            timestamp: e.timestamp,
            session_id: e.session_id,
            entry_type: e.entry_type.into(),
            action: e.action,
            input: e.input.to_string(),
            result: e.result.into(),
            risk: e.risk.map(|r| r.into()),
            auto_approved: e.auto_approved,
            reason: e.reason,
            metadata: e.metadata.to_string(),
        }
    }
}

/// Audit entry input for NAPI
#[napi(object)]
pub struct NapiAuditEntryInput {
    pub session_id: String,
    pub entry_type: NapiAuditEntryType,
    pub action: String,
    pub input: String, // JSON string
    pub result: NapiAuditResult,
    pub risk: Option<NapiRiskLevel>,
    pub auto_approved: Option<bool>,
    pub reason: String,
    pub metadata: String, // JSON string
}

impl TryFrom<NapiAuditEntryInput> for RustAuditEntryInput {
    type Error = serde_json::Error;

    fn try_from(e: NapiAuditEntryInput) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            session_id: e.session_id,
            entry_type: e.entry_type.into(),
            action: e.action,
            input: serde_json::from_str(&e.input).unwrap_or(serde_json::Value::Null),
            result: e.result.into(),
            risk: e.risk.map(|r| r.into()),
            auto_approved: e.auto_approved,
            reason: e.reason,
            metadata: serde_json::from_str(&e.metadata).unwrap_or(serde_json::json!({})),
        })
    }
}

/// Audit filter for NAPI
#[napi(object)]
pub struct NapiAuditFilter {
    pub session_id: Option<String>,
    pub entry_type: Option<NapiAuditEntryType>,
    pub result: Option<NapiAuditResult>,
    pub risk: Option<NapiRiskLevel>,
    pub auto_approved: Option<bool>,
    pub from_timestamp: Option<i64>,
    pub to_timestamp: Option<i64>,
    pub action: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl From<NapiAuditFilter> for RustAuditFilter {
    fn from(f: NapiAuditFilter) -> Self {
        Self {
            session_id: f.session_id,
            entry_type: f.entry_type.map(|t| t.into()),
            result: f.result.map(|r| r.into()),
            risk: f.risk.map(|r| r.into()),
            auto_approved: f.auto_approved,
            from_timestamp: f.from_timestamp,
            to_timestamp: f.to_timestamp,
            action: f.action,
            limit: f.limit.map(|l| l as usize),
            offset: f.offset.map(|o| o as usize),
        }
    }
}

/// Time range for NAPI
#[napi(object)]
pub struct NapiTimeRange {
    pub start: i64,
    pub end: i64,
    pub duration_ms: i64,
}

impl From<RustTimeRange> for NapiTimeRange {
    fn from(t: RustTimeRange) -> Self {
        Self {
            start: t.start,
            end: t.end,
            duration_ms: t.duration_ms,
        }
    }
}

/// Type summary for NAPI
#[napi(object)]
pub struct NapiTypeSummary {
    pub permission: u32,
    pub tool_call: u32,
    pub decision: u32,
    pub state_change: u32,
    pub checkpoint: u32,
    pub rollback: u32,
    pub error: u32,
    pub session_start: u32,
    pub session_end: u32,
}

impl From<RustTypeSummary> for NapiTypeSummary {
    fn from(s: RustTypeSummary) -> Self {
        Self {
            permission: s.permission,
            tool_call: s.tool_call,
            decision: s.decision,
            state_change: s.state_change,
            checkpoint: s.checkpoint,
            rollback: s.rollback,
            error: s.error,
            session_start: s.session_start,
            session_end: s.session_end,
        }
    }
}

/// Result summary for NAPI
#[napi(object)]
pub struct NapiResultSummary {
    pub approved: u32,
    pub rejected: u32,
    pub error: u32,
    pub success: u32,
    pub failed: u32,
}

impl From<RustResultSummary> for NapiResultSummary {
    fn from(s: RustResultSummary) -> Self {
        Self {
            approved: s.approved,
            rejected: s.rejected,
            error: s.error,
            success: s.success,
            failed: s.failed,
        }
    }
}

/// Risk summary for NAPI
#[napi(object)]
pub struct NapiRiskSummary {
    pub safe: u32,
    pub low: u32,
    pub medium: u32,
    pub high: u32,
    pub critical: u32,
}

impl From<RustRiskSummary> for NapiRiskSummary {
    fn from(s: RustRiskSummary) -> Self {
        Self {
            safe: s.safe,
            low: s.low,
            medium: s.medium,
            high: s.high,
            critical: s.critical,
        }
    }
}

/// Audit summary for NAPI
#[napi(object)]
pub struct NapiAuditSummary {
    pub total_entries: u32,
    pub by_type: NapiTypeSummary,
    pub by_result: NapiResultSummary,
    pub by_risk: NapiRiskSummary,
    pub auto_approved_count: u32,
    pub time_range: NapiTimeRange,
}

impl From<RustAuditSummary> for NapiAuditSummary {
    fn from(s: RustAuditSummary) -> Self {
        Self {
            total_entries: s.total_entries,
            by_type: s.by_type.into(),
            by_result: s.by_result.into(),
            by_risk: s.by_risk.into(),
            auto_approved_count: s.auto_approved_count,
            time_range: s.time_range.into(),
        }
    }
}

/// Audit report for NAPI
#[napi(object)]
pub struct NapiAuditReport {
    pub session_id: String,
    pub generated_at: String,
    pub summary: NapiAuditSummary,
    pub entries: Vec<NapiAuditEntry>,
}

impl From<RustAuditReport> for NapiAuditReport {
    fn from(r: RustAuditReport) -> Self {
        Self {
            session_id: r.session_id,
            generated_at: r.generated_at,
            summary: r.summary.into(),
            entries: r.entries.into_iter().map(|e| e.into()).collect(),
        }
    }
}

// ============================================================================
// Audit Log Manager (NAPI)
// ============================================================================

/// Audit log manager handle
#[napi]
pub struct NapiAuditLog {
    inner: Arc<RwLock<RustAuditLog>>,
}

#[napi]
impl NapiAuditLog {
    /// Create a new audit log
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RustAuditLog::new())),
        }
    }

    /// Log an audit entry
    #[napi]
    pub async fn log(&self, entry: NapiAuditEntryInput) -> Result<String> {
        let rust_entry: RustAuditEntryInput = entry.try_into().map_err(|e: serde_json::Error| {
            Error::new(Status::InvalidArg, format!("Invalid entry: {}", e))
        })?;

        let log = self.inner.read().await;
        log.log(rust_entry)
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to log: {}", e)))
    }

    /// Query audit entries
    #[napi]
    pub async fn query(&self, filter: NapiAuditFilter) -> Result<Vec<NapiAuditEntry>> {
        let rust_filter: RustAuditFilter = filter.into();
        let log = self.inner.read().await;

        log.query(&rust_filter)
            .await
            .map(|entries| entries.into_iter().map(|e| e.into()).collect())
            .map_err(|e| Error::new(Status::GenericFailure, format!("Query failed: {}", e)))
    }

    /// Export audit report for a session
    #[napi]
    pub async fn export_report(&self, session_id: String) -> Result<NapiAuditReport> {
        let log = self.inner.read().await;

        log.export_report(&session_id)
            .await
            .map(|r| r.into())
            .map_err(|e| Error::new(Status::GenericFailure, format!("Export failed: {}", e)))
    }

    /// Get entry count
    #[napi]
    pub async fn count(&self, filter: Option<NapiAuditFilter>) -> Result<u32> {
        let log = self.inner.read().await;
        let rust_filter = filter.map(|f| f.into());

        log.count(rust_filter.as_ref())
            .await
            .map(|c| c as u32)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Count failed: {}", e)))
    }

    /// Clear all entries
    #[napi]
    pub async fn clear(&self) -> Result<()> {
        let log = self.inner.read().await;
        log.clear()
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("Clear failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_type_conversion() {
        let napi = NapiAuditEntryType::Permission;
        let rust: RustAuditEntryType = napi.into();
        assert!(matches!(rust, RustAuditEntryType::Permission));
    }

    #[test]
    fn test_result_conversion() {
        let napi = NapiAuditResult::Success;
        let rust: RustAuditResult = napi.into();
        assert!(matches!(rust, RustAuditResult::Success));
    }

    #[test]
    fn test_risk_conversion() {
        let napi = NapiRiskLevel::High;
        let rust: RustRiskLevel = napi.into();
        assert!(matches!(rust, RustRiskLevel::High));
    }
}
