//! Approval request storage and persistence.
//!
//! This module provides SQLite-backed storage for approval requests,
//! with thread-safe access and audit logging.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::{ApprovalRequest, ApprovalStatus, ApprovalType};

/// SQLite-backed store for HitL approval requests.
#[derive(Clone)]
pub struct HitLStore {
    conn: Arc<Mutex<Connection>>,
}

/// Audit log entry for tracking changes to approval requests.
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    /// Auto-incremented ID
    pub id: i64,
    /// ID of the approval request
    pub request_id: String,
    /// Action performed (e.g., "created", "approved", "rejected")
    pub action: String,
    /// User who performed the action
    pub actor_id: String,
    /// Additional details as JSON
    pub details: Option<String>,
    /// Timestamp of the action
    pub timestamp: DateTime<Utc>,
}

impl HitLStore {
    /// Create a new HitL store at the given database path.
    ///
    /// Initializes the database schema if it doesn't exist.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Initialize schema from embedded SQL
        conn.execute_batch(include_str!("schema.sql"))
            .context("Failed to initialize HitL database schema")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a new approval request.
    pub fn create(&self, request: &ApprovalRequest) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let approval_type_json = serde_json::to_string(&request.approval_type)?;
        let approvers_json = serde_json::to_string(&request.approvers)?;
        let status_json = serde_json::to_string(&request.status)?;
        let metadata_json = serde_json::to_string(&request.metadata)?;

        // Extract status info for database columns
        let (status_name, decided_by, decided_at, rejection_reason) =
            extract_status_fields(&request.status);

        conn.execute(
            r"
            INSERT INTO hitl_requests (
                id, approval_type, requester_id, approvers, status,
                decided_by, decided_at, rejection_reason,
                channel_type, channel_id, message_id, context,
                title, description, metadata, expires_at,
                created_at, updated_at
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8,
                ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16,
                ?17, ?18
            )
            ",
            params![
                request.id,
                approval_type_json,
                request.requester,
                approvers_json,
                status_name,
                decided_by,
                decided_at.map(|dt| dt.to_rfc3339()),
                rejection_reason,
                request.channel,
                request.channel, // channel_id same as channel for now
                request.message_id,
                status_json, // Store full status as context
                request.title,
                request.description,
                metadata_json,
                request.expires_at.map(|dt| dt.to_rfc3339()),
                request.created_at.to_rfc3339(),
                request.updated_at.to_rfc3339(),
            ],
        )
        .with_context(|| format!("Failed to create approval request '{}'", request.id))?;

        // Add audit log entry
        self.add_audit_log_internal(&conn, &request.id, "created", &request.requester, None)?;

        Ok(())
    }

    /// Get an approval request by ID.
    pub fn get(&self, id: &str) -> Result<Option<ApprovalRequest>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.get_internal(&conn, id)
    }

    fn get_internal(&self, conn: &Connection, id: &str) -> Result<Option<ApprovalRequest>> {
        conn.query_row(
            r"
            SELECT id, approval_type, requester_id, approvers, status,
                   decided_by, decided_at, rejection_reason,
                   channel_type, message_id, title, description,
                   metadata, expires_at, created_at, updated_at
            FROM hitl_requests WHERE id = ?1
            ",
            params![id],
            |row| {
                let approval_type_json: String = row.get(1)?;
                let approvers_json: String = row.get(3)?;
                let status_name: String = row.get(4)?;
                let decided_by: Option<String> = row.get(5)?;
                let decided_at: Option<String> = row.get(6)?;
                let rejection_reason: Option<String> = row.get(7)?;
                let metadata_json: String = row.get(12)?;
                let expires_at: Option<String> = row.get(13)?;
                let created_at: String = row.get(14)?;
                let updated_at: String = row.get(15)?;

                Ok(ApprovalRequest {
                    id: row.get(0)?,
                    approval_type: serde_json::from_str(&approval_type_json)
                        .unwrap_or_else(|_| ApprovalType::RiskOperation {
                            description: "unknown".to_string(),
                            risk_level: super::RiskLevel::Low,
                        }),
                    requester: row.get(2)?,
                    approvers: serde_json::from_str(&approvers_json).unwrap_or_default(),
                    status: reconstruct_status(
                        &status_name,
                        decided_by,
                        decided_at.as_deref(),
                        rejection_reason,
                    ),
                    title: row.get(10)?,
                    description: row.get(11)?,
                    channel: row.get(8)?,
                    message_id: row.get(9)?,
                    metadata: serde_json::from_str(&metadata_json)
                        .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new())),
                    created_at: parse_datetime(&created_at),
                    updated_at: parse_datetime(&updated_at),
                    expires_at: expires_at.as_deref().map(parse_datetime),
                })
            },
        )
        .optional()
        .with_context(|| format!("Failed to get approval request '{}'", id))
    }

    /// Update the status of an approval request.
    pub fn update_status(&self, id: &str, status: &ApprovalStatus) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let (status_name, decided_by, decided_at, rejection_reason) = extract_status_fields(status);
        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            r"
            UPDATE hitl_requests
            SET status = ?1, decided_by = ?2, decided_at = ?3,
                rejection_reason = ?4, updated_at = ?5
            WHERE id = ?6
            ",
            params![
                status_name,
                decided_by.clone(),
                decided_at.map(|dt| dt.to_rfc3339()),
                rejection_reason.clone(),
                now,
                id,
            ],
        )?;

        if rows == 0 {
            anyhow::bail!("Approval request '{}' not found", id);
        }

        // Add audit log entry
        let action = match status {
            ApprovalStatus::Pending => "reset_to_pending",
            ApprovalStatus::Approved { .. } => "approved",
            ApprovalStatus::Rejected { .. } => "rejected",
            ApprovalStatus::Cancelled { .. } => "cancelled",
        };

        let actor = decided_by.unwrap_or_else(|| "system".to_string());
        let details = rejection_reason.map(|r| serde_json::json!({ "reason": r }).to_string());

        self.add_audit_log_internal(&conn, id, action, &actor, details.as_deref())?;

        Ok(())
    }

    /// Update the message ID for an approval request (after sending to IM channel).
    pub fn update_message_id(&self, id: &str, message_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let now = Utc::now().to_rfc3339();

        let rows = conn.execute(
            "UPDATE hitl_requests SET message_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![message_id, now, id],
        )?;

        if rows == 0 {
            anyhow::bail!("Approval request '{}' not found", id);
        }

        self.add_audit_log_internal(
            &conn,
            id,
            "message_sent",
            "system",
            Some(&serde_json::json!({ "message_id": message_id }).to_string()),
        )?;

        Ok(())
    }

    /// List pending approval requests.
    ///
    /// If `approver_id` is provided, only returns requests where the user is an approver.
    pub fn list_pending(&self, approver_id: Option<&str>) -> Result<Vec<ApprovalRequest>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut results = Vec::new();

        match approver_id {
            Some(approver) => {
                // Pattern to match approver in JSON array: ["user1","user2"]
                // Match the quoted approver name anywhere in the JSON string
                let pattern = format!("%\"{}\",%", approver);
                let pattern_last = format!("%\"{}\"]", approver);
                let mut stmt = conn.prepare(
                    r"
                    SELECT id, approval_type, requester_id, approvers, status,
                           decided_by, decided_at, rejection_reason,
                           channel_type, message_id, title, description,
                           metadata, expires_at, created_at, updated_at
                    FROM hitl_requests
                    WHERE status = 'pending' AND (approvers LIKE ?1 OR approvers LIKE ?2)
                    ORDER BY created_at ASC
                    ",
                )?;
                let rows = stmt.query_map(params![pattern, pattern_last], map_row)?;
                for row in rows {
                    results.push(row?);
                }
            }
            None => {
                let mut stmt = conn.prepare(
                    r"
                    SELECT id, approval_type, requester_id, approvers, status,
                           decided_by, decided_at, rejection_reason,
                           channel_type, message_id, title, description,
                           metadata, expires_at, created_at, updated_at
                    FROM hitl_requests
                    WHERE status = 'pending'
                    ORDER BY created_at ASC
                    ",
                )?;
                let rows = stmt.query_map([], map_row)?;
                for row in rows {
                    results.push(row?);
                }
            }
        }

        Ok(results)
    }

    /// Get audit log entries for a specific request.
    pub fn get_audit_log(&self, request_id: &str) -> Result<Vec<AuditLogEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        let mut stmt = conn.prepare(
            r"
            SELECT id, request_id, action, actor_id, details, timestamp
            FROM hitl_audit_log
            WHERE request_id = ?1
            ORDER BY timestamp ASC
            ",
        )?;

        let rows = stmt.query_map(params![request_id], |row| {
            let timestamp: String = row.get(5)?;
            Ok(AuditLogEntry {
                id: row.get(0)?,
                request_id: row.get(1)?,
                action: row.get(2)?,
                actor_id: row.get(3)?,
                details: row.get(4)?,
                timestamp: parse_datetime(&timestamp),
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Add an entry to the audit log (internal helper).
    fn add_audit_log_internal(
        &self,
        conn: &Connection,
        request_id: &str,
        action: &str,
        actor_id: &str,
        details: Option<&str>,
    ) -> Result<()> {
        conn.execute(
            r"
            INSERT INTO hitl_audit_log (request_id, action, actor_id, details)
            VALUES (?1, ?2, ?3, ?4)
            ",
            params![request_id, action, actor_id, details],
        )?;

        Ok(())
    }
}

/// Extract status fields for database storage.
fn extract_status_fields(
    status: &ApprovalStatus,
) -> (String, Option<String>, Option<DateTime<Utc>>, Option<String>) {
    match status {
        ApprovalStatus::Pending => ("pending".to_string(), None, None, None),
        ApprovalStatus::Approved { by, at } => {
            ("approved".to_string(), Some(by.clone()), Some(*at), None)
        }
        ApprovalStatus::Rejected { by, reason, at } => (
            "rejected".to_string(),
            Some(by.clone()),
            Some(*at),
            reason.clone(),
        ),
        ApprovalStatus::Cancelled { reason } => {
            ("cancelled".to_string(), None, None, Some(reason.clone()))
        }
    }
}

/// Reconstruct ApprovalStatus from database fields.
fn reconstruct_status(
    status_name: &str,
    decided_by: Option<String>,
    decided_at: Option<&str>,
    rejection_reason: Option<String>,
) -> ApprovalStatus {
    match status_name {
        "approved" => ApprovalStatus::Approved {
            by: decided_by.unwrap_or_else(|| "unknown".to_string()),
            at: decided_at.map(parse_datetime).unwrap_or_else(Utc::now),
        },
        "rejected" => ApprovalStatus::Rejected {
            by: decided_by.unwrap_or_else(|| "unknown".to_string()),
            reason: rejection_reason,
            at: decided_at.map(parse_datetime).unwrap_or_else(Utc::now),
        },
        "cancelled" => ApprovalStatus::Cancelled {
            reason: rejection_reason.unwrap_or_else(|| "unknown".to_string()),
        },
        _ => ApprovalStatus::Pending,
    }
}

/// Parse RFC3339 datetime string to DateTime<Utc>.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Map a database row to ApprovalRequest.
fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRequest> {
    let approval_type_json: String = row.get(1)?;
    let approvers_json: String = row.get(3)?;
    let status_name: String = row.get(4)?;
    let decided_by: Option<String> = row.get(5)?;
    let decided_at: Option<String> = row.get(6)?;
    let rejection_reason: Option<String> = row.get(7)?;
    let metadata_json: String = row.get(12)?;
    let expires_at: Option<String> = row.get(13)?;
    let created_at: String = row.get(14)?;
    let updated_at: String = row.get(15)?;

    Ok(ApprovalRequest {
        id: row.get(0)?,
        approval_type: serde_json::from_str(&approval_type_json).unwrap_or_else(|_| {
            ApprovalType::RiskOperation {
                description: "unknown".to_string(),
                risk_level: super::RiskLevel::Low,
            }
        }),
        requester: row.get(2)?,
        approvers: serde_json::from_str(&approvers_json).unwrap_or_default(),
        status: reconstruct_status(
            &status_name,
            decided_by,
            decided_at.as_deref(),
            rejection_reason,
        ),
        title: row.get(10)?,
        description: row.get(11)?,
        channel: row.get(8)?,
        message_id: row.get(9)?,
        metadata: serde_json::from_str(&metadata_json)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new())),
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
        expires_at: expires_at.as_deref().map(parse_datetime),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_store() -> (HitLStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("hitl.db");
        let store = HitLStore::new(&db_path).unwrap();
        (store, dir)
    }

    fn create_test_request(id: &str) -> ApprovalRequest {
        ApprovalRequest {
            id: id.to_string(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".to_string(),
                repo: "test/repo".to_string(),
                mr_id: 42,
            },
            status: ApprovalStatus::Pending,
            requester: "user1".to_string(),
            approvers: vec!["admin".to_string(), "reviewer".to_string()],
            title: "Test PR".to_string(),
            description: Some("Test description".to_string()),
            channel: "telegram".to_string(),
            message_id: None,
            metadata: serde_json::json!({"key": "value"}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        }
    }

    #[test]
    fn test_store_create_and_get() {
        let (store, _dir) = create_test_store();
        let request = create_test_request("test-1");

        // Create request
        store.create(&request).unwrap();

        // Get request
        let fetched = store.get("test-1").unwrap().unwrap();

        assert_eq!(fetched.id, "test-1");
        assert_eq!(fetched.requester, "user1");
        assert_eq!(fetched.title, "Test PR");
        assert_eq!(fetched.channel, "telegram");
        assert!(matches!(fetched.status, ApprovalStatus::Pending));
        assert_eq!(fetched.approvers.len(), 2);
        assert!(fetched.approvers.contains(&"admin".to_string()));

        // Verify audit log was created
        let audit_log = store.get_audit_log("test-1").unwrap();
        assert_eq!(audit_log.len(), 1);
        assert_eq!(audit_log[0].action, "created");
        assert_eq!(audit_log[0].actor_id, "user1");
    }

    #[test]
    fn test_store_get_nonexistent() {
        let (store, _dir) = create_test_store();
        let result = store.get("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_store_update_status() {
        let (store, _dir) = create_test_store();
        let request = create_test_request("test-2");
        store.create(&request).unwrap();

        // Update to approved
        let approved_status = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: Utc::now(),
        };
        store.update_status("test-2", &approved_status).unwrap();

        // Verify status changed
        let fetched = store.get("test-2").unwrap().unwrap();
        match fetched.status {
            ApprovalStatus::Approved { by, .. } => {
                assert_eq!(by, "admin");
            }
            _ => panic!("Expected Approved status"),
        }

        // Verify audit log
        let audit_log = store.get_audit_log("test-2").unwrap();
        assert_eq!(audit_log.len(), 2); // created + approved
        assert_eq!(audit_log[1].action, "approved");
        assert_eq!(audit_log[1].actor_id, "admin");
    }

    #[test]
    fn test_store_update_status_rejected() {
        let (store, _dir) = create_test_store();
        let request = create_test_request("test-3");
        store.create(&request).unwrap();

        // Update to rejected
        let rejected_status = ApprovalStatus::Rejected {
            by: "reviewer".to_string(),
            reason: Some("Does not meet requirements".to_string()),
            at: Utc::now(),
        };
        store.update_status("test-3", &rejected_status).unwrap();

        // Verify status changed
        let fetched = store.get("test-3").unwrap().unwrap();
        match fetched.status {
            ApprovalStatus::Rejected { by, reason, .. } => {
                assert_eq!(by, "reviewer");
                assert_eq!(reason, Some("Does not meet requirements".to_string()));
            }
            _ => panic!("Expected Rejected status"),
        }
    }

    #[test]
    fn test_store_update_status_nonexistent() {
        let (store, _dir) = create_test_store();
        let result = store.update_status(
            "nonexistent",
            &ApprovalStatus::Approved {
                by: "admin".to_string(),
                at: Utc::now(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_store_update_message_id() {
        let (store, _dir) = create_test_store();
        let request = create_test_request("test-4");
        store.create(&request).unwrap();

        // Update message ID
        store.update_message_id("test-4", "msg-123").unwrap();

        // Verify
        let fetched = store.get("test-4").unwrap().unwrap();
        assert_eq!(fetched.message_id, Some("msg-123".to_string()));

        // Verify audit log
        let audit_log = store.get_audit_log("test-4").unwrap();
        assert_eq!(audit_log.len(), 2); // created + message_sent
        assert_eq!(audit_log[1].action, "message_sent");
    }

    #[test]
    fn test_store_list_pending() {
        let (store, _dir) = create_test_store();

        // Create multiple requests
        let request1 = create_test_request("pending-1");
        let request2 = create_test_request("pending-2");
        let mut request3 = create_test_request("approved-1");
        request3.status = ApprovalStatus::Approved {
            by: "admin".to_string(),
            at: Utc::now(),
        };

        store.create(&request1).unwrap();
        store.create(&request2).unwrap();
        store.create(&request3).unwrap();

        // List all pending
        let pending = store.list_pending(None).unwrap();
        assert_eq!(pending.len(), 2);

        // List pending for specific approver
        let for_admin = store.list_pending(Some("admin")).unwrap();
        assert_eq!(for_admin.len(), 2);

        // List pending for non-approver
        let for_other = store.list_pending(Some("other-user")).unwrap();
        assert_eq!(for_other.len(), 0);
    }

    #[test]
    fn test_store_trading_command_type() {
        let (store, _dir) = create_test_store();
        let request = ApprovalRequest {
            id: "trade-1".to_string(),
            approval_type: ApprovalType::TradingCommand {
                asset: "BTC".to_string(),
                action: "buy".to_string(),
                amount: 1.5,
            },
            status: ApprovalStatus::Pending,
            requester: "trader".to_string(),
            approvers: vec!["risk-manager".to_string()],
            title: "Buy BTC".to_string(),
            description: None,
            channel: "slack".to_string(),
            message_id: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
        };

        store.create(&request).unwrap();

        let fetched = store.get("trade-1").unwrap().unwrap();
        match fetched.approval_type {
            ApprovalType::TradingCommand {
                asset,
                action,
                amount,
            } => {
                assert_eq!(asset, "BTC");
                assert_eq!(action, "buy");
                assert!((amount - 1.5).abs() < f64::EPSILON);
            }
            _ => panic!("Expected TradingCommand type"),
        }
    }

    #[test]
    fn test_store_cancelled_status() {
        let (store, _dir) = create_test_store();
        let request = create_test_request("cancel-1");
        store.create(&request).unwrap();

        let cancelled_status = ApprovalStatus::Cancelled {
            reason: "Request expired".to_string(),
        };
        store.update_status("cancel-1", &cancelled_status).unwrap();

        let fetched = store.get("cancel-1").unwrap().unwrap();
        match fetched.status {
            ApprovalStatus::Cancelled { reason } => {
                assert_eq!(reason, "Request expired");
            }
            _ => panic!("Expected Cancelled status"),
        }
    }
}
