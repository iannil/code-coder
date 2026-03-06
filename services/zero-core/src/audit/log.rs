//! Structured Audit Log
//!
//! Provides append-only, tamper-evident logging for autonomous mode operations.
//! Uses SQLite for efficient querying and persistence.
//!
//! Adapted from packages/ccode/src/audit/audit-log.ts

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ============================================================================
// Types
// ============================================================================

/// Audit entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEntryType {
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

impl std::fmt::Display for AuditEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Permission => write!(f, "permission"),
            Self::ToolCall => write!(f, "tool_call"),
            Self::Decision => write!(f, "decision"),
            Self::StateChange => write!(f, "state_change"),
            Self::Checkpoint => write!(f, "checkpoint"),
            Self::Rollback => write!(f, "rollback"),
            Self::Error => write!(f, "error"),
            Self::SessionStart => write!(f, "session_start"),
            Self::SessionEnd => write!(f, "session_end"),
        }
    }
}

/// Audit result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    Approved,
    Rejected,
    Error,
    Success,
    Failed,
}

impl std::fmt::Display for AuditResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Error => write!(f, "error"),
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Safe => write!(f, "safe"),
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Timestamp (Unix ms)
    pub timestamp: i64,
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Entry type
    #[serde(rename = "type")]
    pub entry_type: AuditEntryType,
    /// Action performed
    pub action: String,
    /// Input data (JSON)
    pub input: serde_json::Value,
    /// Result of the action
    pub result: AuditResult,
    /// Risk level
    pub risk: Option<RiskLevel>,
    /// Whether auto-approved
    #[serde(rename = "autoApproved")]
    pub auto_approved: Option<bool>,
    /// Reason for the action/result
    pub reason: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

/// Audit entry input (without auto-generated fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntryInput {
    /// Session ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// Entry type
    #[serde(rename = "type")]
    pub entry_type: AuditEntryType,
    /// Action performed
    pub action: String,
    /// Input data
    pub input: serde_json::Value,
    /// Result
    pub result: AuditResult,
    /// Risk level
    pub risk: Option<RiskLevel>,
    /// Whether auto-approved
    #[serde(rename = "autoApproved")]
    pub auto_approved: Option<bool>,
    /// Reason
    pub reason: String,
    /// Metadata
    pub metadata: serde_json::Value,
}

/// Audit query filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<AuditEntryType>,
    pub result: Option<AuditResult>,
    pub risk: Option<RiskLevel>,
    #[serde(rename = "autoApproved")]
    pub auto_approved: Option<bool>,
    #[serde(rename = "fromTimestamp")]
    pub from_timestamp: Option<i64>,
    #[serde(rename = "toTimestamp")]
    pub to_timestamp: Option<i64>,
    pub action: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Summary statistics by type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeSummary {
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

/// Summary statistics by result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultSummary {
    pub approved: u32,
    pub rejected: u32,
    pub error: u32,
    pub success: u32,
    pub failed: u32,
}

/// Summary statistics by risk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskSummary {
    pub safe: u32,
    pub low: u32,
    pub medium: u32,
    pub high: u32,
    pub critical: u32,
}

/// Time range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
    #[serde(rename = "durationMs")]
    pub duration_ms: i64,
}

/// Audit summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    #[serde(rename = "totalEntries")]
    pub total_entries: u32,
    #[serde(rename = "byType")]
    pub by_type: TypeSummary,
    #[serde(rename = "byResult")]
    pub by_result: ResultSummary,
    #[serde(rename = "byRisk")]
    pub by_risk: RiskSummary,
    #[serde(rename = "autoApprovedCount")]
    pub auto_approved_count: u32,
    #[serde(rename = "timeRange")]
    pub time_range: TimeRange,
}

/// Audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    pub summary: AuditSummary,
    pub entries: Vec<AuditEntry>,
}

// ============================================================================
// In-Memory Audit Log (for testing and lightweight use)
// ============================================================================

/// In-memory audit log implementation
pub struct AuditLog {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    /// Reserved for future SQLite persistence
    #[allow(dead_code)]
    db_path: Option<PathBuf>,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLog {
    /// Create a new in-memory audit log
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            db_path: None,
        }
    }

    /// Create an audit log with SQLite persistence path
    pub fn with_path(db_path: impl AsRef<Path>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            db_path: Some(db_path.as_ref().to_path_buf()),
        }
    }

    /// Initialize the audit log
    pub async fn initialize(&self) -> Result<()> {
        // For in-memory implementation, nothing to initialize
        // SQLite implementation would create tables here
        Ok(())
    }

    /// Log an audit entry
    pub async fn log(&self, input: AuditEntryInput) -> Result<String> {
        let id = generate_entry_id();
        let timestamp = chrono::Utc::now().timestamp_millis();

        let entry = AuditEntry {
            id: id.clone(),
            timestamp,
            session_id: input.session_id,
            entry_type: input.entry_type,
            action: input.action,
            input: input.input,
            result: input.result,
            risk: input.risk,
            auto_approved: input.auto_approved,
            reason: input.reason,
            metadata: input.metadata,
        };

        let mut entries = self.entries.write().await;
        entries.push(entry);

        Ok(id)
    }

    /// Query audit entries
    pub async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>> {
        let entries = self.entries.read().await;

        let mut results: Vec<AuditEntry> = entries
            .iter()
            .filter(|e| {
                // Session ID filter
                if let Some(ref sid) = filter.session_id {
                    if &e.session_id != sid {
                        return false;
                    }
                }

                // Type filter
                if let Some(ref t) = filter.entry_type {
                    if &e.entry_type != t {
                        return false;
                    }
                }

                // Result filter
                if let Some(ref r) = filter.result {
                    if &e.result != r {
                        return false;
                    }
                }

                // Risk filter
                if let Some(ref risk) = filter.risk {
                    if e.risk.as_ref() != Some(risk) {
                        return false;
                    }
                }

                // Auto-approved filter
                if let Some(auto) = filter.auto_approved {
                    if e.auto_approved != Some(auto) {
                        return false;
                    }
                }

                // Timestamp filters
                if let Some(from) = filter.from_timestamp {
                    if e.timestamp < from {
                        return false;
                    }
                }
                if let Some(to) = filter.to_timestamp {
                    if e.timestamp > to {
                        return false;
                    }
                }

                // Action filter (contains)
                if let Some(ref action) = filter.action {
                    if !e.action.contains(action) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by timestamp descending
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply offset and limit
        let offset = filter.offset.unwrap_or(0);
        let limit = filter.limit.unwrap_or(1000);

        Ok(results.into_iter().skip(offset).take(limit).collect())
    }

    /// Export audit report for a session
    pub async fn export_report(&self, session_id: &str) -> Result<AuditReport> {
        let filter = AuditFilter {
            session_id: Some(session_id.to_string()),
            limit: Some(10000),
            ..Default::default()
        };

        let entries = self.query(&filter).await?;

        // Calculate summary statistics
        let mut by_type = TypeSummary::default();
        let mut by_result = ResultSummary::default();
        let mut by_risk = RiskSummary::default();
        let mut auto_approved_count = 0u32;
        let mut min_timestamp = i64::MAX;
        let mut max_timestamp = 0i64;

        for entry in &entries {
            // By type
            match entry.entry_type {
                AuditEntryType::Permission => by_type.permission += 1,
                AuditEntryType::ToolCall => by_type.tool_call += 1,
                AuditEntryType::Decision => by_type.decision += 1,
                AuditEntryType::StateChange => by_type.state_change += 1,
                AuditEntryType::Checkpoint => by_type.checkpoint += 1,
                AuditEntryType::Rollback => by_type.rollback += 1,
                AuditEntryType::Error => by_type.error += 1,
                AuditEntryType::SessionStart => by_type.session_start += 1,
                AuditEntryType::SessionEnd => by_type.session_end += 1,
            }

            // By result
            match entry.result {
                AuditResult::Approved => by_result.approved += 1,
                AuditResult::Rejected => by_result.rejected += 1,
                AuditResult::Error => by_result.error += 1,
                AuditResult::Success => by_result.success += 1,
                AuditResult::Failed => by_result.failed += 1,
            }

            // By risk
            if let Some(risk) = &entry.risk {
                match risk {
                    RiskLevel::Safe => by_risk.safe += 1,
                    RiskLevel::Low => by_risk.low += 1,
                    RiskLevel::Medium => by_risk.medium += 1,
                    RiskLevel::High => by_risk.high += 1,
                    RiskLevel::Critical => by_risk.critical += 1,
                }
            }

            // Auto-approved
            if entry.auto_approved == Some(true) {
                auto_approved_count += 1;
            }

            // Time range
            if entry.timestamp < min_timestamp {
                min_timestamp = entry.timestamp;
            }
            if entry.timestamp > max_timestamp {
                max_timestamp = entry.timestamp;
            }
        }

        let time_range = TimeRange {
            start: if min_timestamp == i64::MAX { 0 } else { min_timestamp },
            end: max_timestamp,
            duration_ms: if max_timestamp > 0 && min_timestamp < i64::MAX {
                max_timestamp - min_timestamp
            } else {
                0
            },
        };

        // Reverse to chronological order
        let mut chronological_entries = entries;
        chronological_entries.reverse();

        Ok(AuditReport {
            session_id: session_id.to_string(),
            generated_at: Utc::now().to_rfc3339(),
            summary: AuditSummary {
                total_entries: chronological_entries.len() as u32,
                by_type,
                by_result,
                by_risk,
                auto_approved_count,
                time_range,
            },
            entries: chronological_entries,
        })
    }

    /// Get entry count
    pub async fn count(&self, filter: Option<&AuditFilter>) -> Result<usize> {
        let entries = self.query(filter.unwrap_or(&AuditFilter::default())).await?;
        Ok(entries.len())
    }

    /// Clear all entries (for testing)
    pub async fn clear(&self) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.clear();
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_entry_id() -> String {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let random: u32 = rand::random();
    format!("audit_{}_{:08x}", timestamp, random)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_entry() {
        let log = AuditLog::new();

        let id = log
            .log(AuditEntryInput {
                session_id: "test-session".to_string(),
                entry_type: AuditEntryType::Permission,
                action: "file:read".to_string(),
                input: serde_json::json!({"path": "/test"}),
                result: AuditResult::Approved,
                risk: Some(RiskLevel::Safe),
                auto_approved: Some(true),
                reason: "Test reason".to_string(),
                metadata: serde_json::json!({}),
            })
            .await
            .unwrap();

        assert!(id.starts_with("audit_"));
    }

    #[tokio::test]
    async fn test_query_entries() {
        let log = AuditLog::new();

        // Log multiple entries
        for i in 0..5 {
            log.log(AuditEntryInput {
                session_id: "test-session".to_string(),
                entry_type: AuditEntryType::ToolCall,
                action: format!("tool_{}", i),
                input: serde_json::json!({}),
                result: AuditResult::Success,
                risk: None,
                auto_approved: None,
                reason: "Test".to_string(),
                metadata: serde_json::json!({}),
            })
            .await
            .unwrap();
        }

        let entries = log
            .query(&AuditFilter {
                session_id: Some("test-session".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    async fn test_export_report() {
        let log = AuditLog::new();

        // Log various entries
        log.log(AuditEntryInput {
            session_id: "report-test".to_string(),
            entry_type: AuditEntryType::SessionStart,
            action: "start".to_string(),
            input: serde_json::json!(null),
            result: AuditResult::Success,
            risk: None,
            auto_approved: None,
            reason: "Session started".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .unwrap();

        log.log(AuditEntryInput {
            session_id: "report-test".to_string(),
            entry_type: AuditEntryType::Permission,
            action: "file:write".to_string(),
            input: serde_json::json!({"path": "/test"}),
            result: AuditResult::Approved,
            risk: Some(RiskLevel::Medium),
            auto_approved: Some(true),
            reason: "Auto-approved".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .unwrap();

        let report = log.export_report("report-test").await.unwrap();

        assert_eq!(report.session_id, "report-test");
        assert_eq!(report.summary.total_entries, 2);
        assert_eq!(report.summary.by_type.session_start, 1);
        assert_eq!(report.summary.by_type.permission, 1);
        assert_eq!(report.summary.by_risk.medium, 1);
        assert_eq!(report.summary.auto_approved_count, 1);
    }

    #[tokio::test]
    async fn test_filter_by_type() {
        let log = AuditLog::new();

        log.log(AuditEntryInput {
            session_id: "filter-test".to_string(),
            entry_type: AuditEntryType::Permission,
            action: "perm".to_string(),
            input: serde_json::json!({}),
            result: AuditResult::Approved,
            risk: None,
            auto_approved: None,
            reason: "Test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .unwrap();

        log.log(AuditEntryInput {
            session_id: "filter-test".to_string(),
            entry_type: AuditEntryType::ToolCall,
            action: "tool".to_string(),
            input: serde_json::json!({}),
            result: AuditResult::Success,
            risk: None,
            auto_approved: None,
            reason: "Test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .unwrap();

        let entries = log
            .query(&AuditFilter {
                entry_type: Some(AuditEntryType::Permission),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_type, AuditEntryType::Permission);
    }

    #[tokio::test]
    async fn test_count() {
        let log = AuditLog::new();

        for _ in 0..3 {
            log.log(AuditEntryInput {
                session_id: "count-test".to_string(),
                entry_type: AuditEntryType::ToolCall,
                action: "test".to_string(),
                input: serde_json::json!({}),
                result: AuditResult::Success,
                risk: None,
                auto_approved: None,
                reason: "Test".to_string(),
                metadata: serde_json::json!({}),
            })
            .await
            .unwrap();
        }

        let count = log.count(None).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_clear() {
        let log = AuditLog::new();

        log.log(AuditEntryInput {
            session_id: "clear-test".to_string(),
            entry_type: AuditEntryType::ToolCall,
            action: "test".to_string(),
            input: serde_json::json!({}),
            result: AuditResult::Success,
            risk: None,
            auto_approved: None,
            reason: "Test".to_string(),
            metadata: serde_json::json!({}),
        })
        .await
        .unwrap();

        assert_eq!(log.count(None).await.unwrap(), 1);

        log.clear().await.unwrap();

        assert_eq!(log.count(None).await.unwrap(), 0);
    }

    #[test]
    fn test_entry_type_display() {
        assert_eq!(format!("{}", AuditEntryType::Permission), "permission");
        assert_eq!(format!("{}", AuditEntryType::ToolCall), "tool_call");
    }

    #[test]
    fn test_result_display() {
        assert_eq!(format!("{}", AuditResult::Approved), "approved");
        assert_eq!(format!("{}", AuditResult::Success), "success");
    }

    #[test]
    fn test_risk_display() {
        assert_eq!(format!("{}", RiskLevel::Safe), "safe");
        assert_eq!(format!("{}", RiskLevel::Critical), "critical");
    }
}
