# Omni-Nexus HitL Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a centralized Human-in-the-Loop approval system with interactive cards across Telegram, Feishu, Slack, and DingTalk.

**Architecture:** HitL service in zero-gateway with SQLite persistence, CardRenderer trait for channel-specific UI, and HitLClient for workflow integration.

**Tech Stack:** Rust, axum, rusqlite, reqwest, serde_json, async-trait

---

## Task 1: Core HitL Types

**Files:**
- Create: `services/zero-gateway/src/hitl/mod.rs`

**Step 1: Write the failing test**

```rust
// services/zero-gateway/src/hitl/mod.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_type_serialization() {
        let mr = ApprovalType::MergeRequest {
            platform: "github".into(),
            repo: "owner/repo".into(),
            mr_id: 123,
        };
        let json = serde_json::to_string(&mr).unwrap();
        assert!(json.contains("MergeRequest"));
        assert!(json.contains("github"));
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Critical as u8 > RiskLevel::High as u8);
        assert!(RiskLevel::High as u8 > RiskLevel::Medium as u8);
    }

    #[test]
    fn test_approval_status_is_terminal() {
        assert!(!ApprovalStatus::Pending.is_terminal());
        assert!(ApprovalStatus::Approved { by: "user".into(), at: chrono::Utc::now() }.is_terminal());
        assert!(ApprovalStatus::Rejected { by: "user".into(), reason: None, at: chrono::Utc::now() }.is_terminal());
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd services && cargo test -p zero-gateway hitl::tests --no-run 2>&1 | head -20`
Expected: FAIL with "can't find crate for `hitl`" or similar

**Step 3: Write minimal implementation**

```rust
// services/zero-gateway/src/hitl/mod.rs

//! Human-in-the-Loop (HitL) approval system.
//!
//! Provides centralized approval workflow for critical operations:
//! - MR/PR merge approvals
//! - Trading command confirmations
//! - Config changes
//! - High-cost operations
//! - Risk operations

pub mod store;
pub mod routes;
pub mod actions;
pub mod cards;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of operation requiring approval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ApprovalType {
    MergeRequest {
        platform: String,
        repo: String,
        mr_id: i64,
    },
    TradingCommand {
        asset: String,
        action: String,
        amount: f64,
    },
    ConfigChange {
        key: String,
        old_value: String,
        new_value: String,
    },
    HighCostOperation {
        operation: String,
        estimated_cost: f64,
    },
    RiskOperation {
        description: String,
        risk_level: RiskLevel,
    },
}

impl ApprovalType {
    /// Get a human-readable type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::MergeRequest { .. } => "merge_request",
            Self::TradingCommand { .. } => "trading_command",
            Self::ConfigChange { .. } => "config_change",
            Self::HighCostOperation { .. } => "high_cost_operation",
            Self::RiskOperation { .. } => "risk_operation",
        }
    }
}

/// Risk level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum RiskLevel {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: String,
    pub approval_type: ApprovalType,
    pub requester_id: String,
    pub approvers: Vec<String>,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub channel_type: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub context: serde_json::Value,
}

/// Approval status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status")]
pub enum ApprovalStatus {
    Pending,
    Approved {
        by: String,
        at: DateTime<Utc>,
    },
    Rejected {
        by: String,
        reason: Option<String>,
        at: DateTime<Utc>,
    },
    Cancelled {
        reason: String,
    },
}

impl ApprovalStatus {
    /// Check if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Pending)
    }

    /// Get status name for database storage.
    pub fn status_name(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved { .. } => "approved",
            Self::Rejected { .. } => "rejected",
            Self::Cancelled { .. } => "cancelled",
        }
    }
}

/// Request to create an approval.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateApprovalRequest {
    pub approval_type: ApprovalType,
    pub requester_id: String,
    pub approvers: Vec<String>,
    pub channel_type: String,
    pub channel_id: String,
    pub context: Option<serde_json::Value>,
}

/// Response after creating approval.
#[derive(Debug, Clone, Serialize)]
pub struct ApprovalResponse {
    pub request_id: String,
    pub status: ApprovalStatus,
    pub message_id: Option<String>,
}

/// Decision from an approver.
#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalDecision {
    pub approved: bool,
    pub approver_id: String,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_type_serialization() {
        let mr = ApprovalType::MergeRequest {
            platform: "github".into(),
            repo: "owner/repo".into(),
            mr_id: 123,
        };
        let json = serde_json::to_string(&mr).unwrap();
        assert!(json.contains("MergeRequest"));
        assert!(json.contains("github"));
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Critical as u8 > RiskLevel::High as u8);
        assert!(RiskLevel::High as u8 > RiskLevel::Medium as u8);
    }

    #[test]
    fn test_approval_status_is_terminal() {
        assert!(!ApprovalStatus::Pending.is_terminal());
        assert!(ApprovalStatus::Approved { by: "user".into(), at: Utc::now() }.is_terminal());
        assert!(ApprovalStatus::Rejected { by: "user".into(), reason: None, at: Utc::now() }.is_terminal());
    }

    #[test]
    fn test_approval_type_name() {
        let mr = ApprovalType::MergeRequest {
            platform: "github".into(),
            repo: "owner/repo".into(),
            mr_id: 123,
        };
        assert_eq!(mr.type_name(), "merge_request");
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd services && cargo test -p zero-gateway hitl::tests -- --nocapture`
Expected: PASS (4 tests)

**Step 5: Commit**

```bash
cd services && git add zero-gateway/src/hitl/mod.rs && git commit -m "feat(hitl): add core HitL types and ApprovalRequest model"
```

---

## Task 2: HitL SQLite Store

**Files:**
- Create: `services/zero-gateway/src/hitl/store.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_store_create_and_get() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = HitLStore::new(&db_path).await.unwrap();

        let request = ApprovalRequest {
            id: "test-123".into(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".into(),
                repo: "owner/repo".into(),
                mr_id: 42,
            },
            requester_id: "user-1".into(),
            approvers: vec!["admin-1".into()],
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            channel_type: "telegram".into(),
            channel_id: "123456".into(),
            message_id: None,
            context: serde_json::json!({"title": "Test PR"}),
        };

        store.create(&request).await.unwrap();
        let retrieved = store.get("test-123").await.unwrap().unwrap();

        assert_eq!(retrieved.id, "test-123");
        assert_eq!(retrieved.requester_id, "user-1");
    }

    #[tokio::test]
    async fn test_store_update_status() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = HitLStore::new(&db_path).await.unwrap();

        // Create request first
        let request = ApprovalRequest {
            id: "test-456".into(),
            approval_type: ApprovalType::ConfigChange {
                key: "quota".into(),
                old_value: "100".into(),
                new_value: "200".into(),
            },
            requester_id: "user-1".into(),
            approvers: vec!["admin-1".into()],
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            channel_type: "feishu".into(),
            channel_id: "oc_xxx".into(),
            message_id: None,
            context: serde_json::json!({}),
        };
        store.create(&request).await.unwrap();

        // Update to approved
        let new_status = ApprovalStatus::Approved {
            by: "admin-1".into(),
            at: Utc::now(),
        };
        store.update_status("test-456", &new_status).await.unwrap();

        let retrieved = store.get("test-456").await.unwrap().unwrap();
        assert!(matches!(retrieved.status, ApprovalStatus::Approved { .. }));
    }

    #[tokio::test]
    async fn test_store_list_pending() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = HitLStore::new(&db_path).await.unwrap();

        // Create 2 pending, 1 approved
        for i in 0..3 {
            let status = if i == 2 {
                ApprovalStatus::Approved { by: "admin".into(), at: Utc::now() }
            } else {
                ApprovalStatus::Pending
            };
            let request = ApprovalRequest {
                id: format!("req-{}", i),
                approval_type: ApprovalType::RiskOperation {
                    description: "test".into(),
                    risk_level: RiskLevel::Medium,
                },
                requester_id: "user-1".into(),
                approvers: vec!["admin-1".into()],
                status,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                channel_type: "slack".into(),
                channel_id: "C123".into(),
                message_id: None,
                context: serde_json::json!({}),
            };
            store.create(&request).await.unwrap();
        }

        let pending = store.list_pending(None).await.unwrap();
        assert_eq!(pending.len(), 2);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd services && cargo test -p zero-gateway hitl::store::tests --no-run 2>&1 | head -20`
Expected: FAIL with "can't find `store`"

**Step 3: Write minimal implementation**

```rust
// services/zero-gateway/src/hitl/store.rs

//! SQLite persistence for HitL approval requests.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{ApprovalRequest, ApprovalStatus, ApprovalType};

/// SQLite store for HitL requests.
pub struct HitLStore {
    conn: Arc<Mutex<Connection>>,
}

impl HitLStore {
    /// Create a new store, initializing the database schema.
    pub async fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database: {}", db_path.display()))?;

        // Initialize schema
        conn.execute_batch(include_str!("schema.sql"))
            .context("Failed to initialize HitL schema")?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create a new approval request.
    pub async fn create(&self, request: &ApprovalRequest) -> Result<()> {
        let conn = self.conn.lock().await;

        let approval_type_json = serde_json::to_string(&request.approval_type)?;
        let approvers_json = serde_json::to_string(&request.approvers)?;
        let context_json = serde_json::to_string(&request.context)?;
        let status_name = request.status.status_name();

        conn.execute(
            r#"
            INSERT INTO hitl_requests (
                id, approval_type, requester_id, approvers, status,
                channel_type, channel_id, message_id, context,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                request.id,
                approval_type_json,
                request.requester_id,
                approvers_json,
                status_name,
                request.channel_type,
                request.channel_id,
                request.message_id,
                context_json,
                request.created_at.to_rfc3339(),
                request.updated_at.to_rfc3339(),
            ],
        )
        .context("Failed to insert approval request")?;

        // Add audit log entry
        self.add_audit_log_internal(&conn, &request.id, "created", &request.requester_id, None)?;

        Ok(())
    }

    /// Get an approval request by ID.
    pub async fn get(&self, id: &str) -> Result<Option<ApprovalRequest>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, approval_type, requester_id, approvers, status,
                   decided_by, decided_at, rejection_reason,
                   channel_type, channel_id, message_id, context,
                   created_at, updated_at
            FROM hitl_requests WHERE id = ?1
            "#,
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(self.row_to_request(row))
        });

        match result {
            Ok(request) => Ok(Some(request?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update request status.
    pub async fn update_status(&self, id: &str, status: &ApprovalStatus) -> Result<()> {
        let conn = self.conn.lock().await;

        let (status_name, decided_by, decided_at, rejection_reason) = match status {
            ApprovalStatus::Pending => ("pending", None, None, None),
            ApprovalStatus::Approved { by, at } => {
                ("approved", Some(by.as_str()), Some(at.to_rfc3339()), None)
            }
            ApprovalStatus::Rejected { by, reason, at } => {
                ("rejected", Some(by.as_str()), Some(at.to_rfc3339()), reason.as_deref())
            }
            ApprovalStatus::Cancelled { reason } => {
                ("cancelled", None, None, Some(reason.as_str()))
            }
        };

        conn.execute(
            r#"
            UPDATE hitl_requests
            SET status = ?1, decided_by = ?2, decided_at = ?3, rejection_reason = ?4,
                updated_at = ?5
            WHERE id = ?6
            "#,
            params![
                status_name,
                decided_by,
                decided_at,
                rejection_reason,
                Utc::now().to_rfc3339(),
                id,
            ],
        )
        .context("Failed to update request status")?;

        // Add audit log
        let action = status_name;
        let actor = decided_by.unwrap_or("system");
        self.add_audit_log_internal(&conn, id, action, actor, rejection_reason)?;

        Ok(())
    }

    /// Update message ID after card is sent.
    pub async fn update_message_id(&self, id: &str, message_id: &str) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE hitl_requests SET message_id = ?1, updated_at = ?2 WHERE id = ?3",
            params![message_id, Utc::now().to_rfc3339(), id],
        )
        .context("Failed to update message ID")?;

        Ok(())
    }

    /// List pending requests, optionally filtered by approver.
    pub async fn list_pending(&self, approver_id: Option<&str>) -> Result<Vec<ApprovalRequest>> {
        let conn = self.conn.lock().await;

        let query = if approver_id.is_some() {
            r#"
            SELECT id, approval_type, requester_id, approvers, status,
                   decided_by, decided_at, rejection_reason,
                   channel_type, channel_id, message_id, context,
                   created_at, updated_at
            FROM hitl_requests
            WHERE status = 'pending' AND approvers LIKE ?1
            ORDER BY created_at ASC
            "#
        } else {
            r#"
            SELECT id, approval_type, requester_id, approvers, status,
                   decided_by, decided_at, rejection_reason,
                   channel_type, channel_id, message_id, context,
                   created_at, updated_at
            FROM hitl_requests
            WHERE status = 'pending'
            ORDER BY created_at ASC
            "#
        };

        let mut stmt = conn.prepare(query)?;

        let rows = if let Some(approver) = approver_id {
            stmt.query_map(params![format!("%\"{}\"", approver)], |row| {
                Ok(self.row_to_request(row))
            })?
        } else {
            stmt.query_map([], |row| Ok(self.row_to_request(row)))?
        };

        let mut requests = Vec::new();
        for row in rows {
            requests.push(row??);
        }

        Ok(requests)
    }

    /// Add audit log entry (internal, must hold lock).
    fn add_audit_log_internal(
        &self,
        conn: &Connection,
        request_id: &str,
        action: &str,
        actor_id: &str,
        details: Option<&str>,
    ) -> Result<()> {
        conn.execute(
            r#"
            INSERT INTO hitl_audit_log (request_id, action, actor_id, details)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![request_id, action, actor_id, details],
        )
        .context("Failed to add audit log")?;

        Ok(())
    }

    /// Parse a database row into ApprovalRequest.
    fn row_to_request(&self, row: &rusqlite::Row) -> Result<ApprovalRequest> {
        let id: String = row.get(0)?;
        let approval_type_json: String = row.get(1)?;
        let requester_id: String = row.get(2)?;
        let approvers_json: String = row.get(3)?;
        let status_name: String = row.get(4)?;
        let decided_by: Option<String> = row.get(5)?;
        let decided_at: Option<String> = row.get(6)?;
        let rejection_reason: Option<String> = row.get(7)?;
        let channel_type: String = row.get(8)?;
        let channel_id: String = row.get(9)?;
        let message_id: Option<String> = row.get(10)?;
        let context_json: String = row.get(11)?;
        let created_at_str: String = row.get(12)?;
        let updated_at_str: String = row.get(13)?;

        let approval_type: ApprovalType = serde_json::from_str(&approval_type_json)?;
        let approvers: Vec<String> = serde_json::from_str(&approvers_json)?;
        let context: serde_json::Value = serde_json::from_str(&context_json)?;

        let status = match status_name.as_str() {
            "pending" => ApprovalStatus::Pending,
            "approved" => ApprovalStatus::Approved {
                by: decided_by.unwrap_or_default(),
                at: DateTime::parse_from_rfc3339(&decided_at.unwrap_or_default())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            },
            "rejected" => ApprovalStatus::Rejected {
                by: decided_by.unwrap_or_default(),
                reason: rejection_reason,
                at: DateTime::parse_from_rfc3339(&decided_at.unwrap_or_default())
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            },
            "cancelled" => ApprovalStatus::Cancelled {
                reason: rejection_reason.unwrap_or_default(),
            },
            _ => ApprovalStatus::Pending,
        };

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(ApprovalRequest {
            id,
            approval_type,
            requester_id,
            approvers,
            status,
            created_at,
            updated_at,
            channel_type,
            channel_id,
            message_id,
            context,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hitl::RiskLevel;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_store_create_and_get() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = HitLStore::new(&db_path).await.unwrap();

        let request = ApprovalRequest {
            id: "test-123".into(),
            approval_type: ApprovalType::MergeRequest {
                platform: "github".into(),
                repo: "owner/repo".into(),
                mr_id: 42,
            },
            requester_id: "user-1".into(),
            approvers: vec!["admin-1".into()],
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            channel_type: "telegram".into(),
            channel_id: "123456".into(),
            message_id: None,
            context: serde_json::json!({"title": "Test PR"}),
        };

        store.create(&request).await.unwrap();
        let retrieved = store.get("test-123").await.unwrap().unwrap();

        assert_eq!(retrieved.id, "test-123");
        assert_eq!(retrieved.requester_id, "user-1");
    }

    #[tokio::test]
    async fn test_store_update_status() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = HitLStore::new(&db_path).await.unwrap();

        let request = ApprovalRequest {
            id: "test-456".into(),
            approval_type: ApprovalType::ConfigChange {
                key: "quota".into(),
                old_value: "100".into(),
                new_value: "200".into(),
            },
            requester_id: "user-1".into(),
            approvers: vec!["admin-1".into()],
            status: ApprovalStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            channel_type: "feishu".into(),
            channel_id: "oc_xxx".into(),
            message_id: None,
            context: serde_json::json!({}),
        };
        store.create(&request).await.unwrap();

        let new_status = ApprovalStatus::Approved {
            by: "admin-1".into(),
            at: Utc::now(),
        };
        store.update_status("test-456", &new_status).await.unwrap();

        let retrieved = store.get("test-456").await.unwrap().unwrap();
        assert!(matches!(retrieved.status, ApprovalStatus::Approved { .. }));
    }

    #[tokio::test]
    async fn test_store_list_pending() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = HitLStore::new(&db_path).await.unwrap();

        for i in 0..3 {
            let status = if i == 2 {
                ApprovalStatus::Approved { by: "admin".into(), at: Utc::now() }
            } else {
                ApprovalStatus::Pending
            };
            let request = ApprovalRequest {
                id: format!("req-{}", i),
                approval_type: ApprovalType::RiskOperation {
                    description: "test".into(),
                    risk_level: RiskLevel::Medium,
                },
                requester_id: "user-1".into(),
                approvers: vec!["admin-1".into()],
                status,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                channel_type: "slack".into(),
                channel_id: "C123".into(),
                message_id: None,
                context: serde_json::json!({}),
            };
            store.create(&request).await.unwrap();
        }

        let pending = store.list_pending(None).await.unwrap();
        assert_eq!(pending.len(), 2);
    }
}
```

Also create the schema file:

```sql
-- services/zero-gateway/src/hitl/schema.sql

CREATE TABLE IF NOT EXISTS hitl_requests (
    id TEXT PRIMARY KEY,
    approval_type TEXT NOT NULL,
    requester_id TEXT NOT NULL,
    approvers TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    decided_by TEXT,
    decided_at TEXT,
    rejection_reason TEXT,
    channel_type TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    message_id TEXT,
    context TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_hitl_status ON hitl_requests(status);
CREATE INDEX IF NOT EXISTS idx_hitl_requester ON hitl_requests(requester_id);
CREATE INDEX IF NOT EXISTS idx_hitl_channel ON hitl_requests(channel_type, channel_id);

CREATE TABLE IF NOT EXISTS hitl_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    details TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_hitl_audit_request ON hitl_audit_log(request_id);
```

**Step 4: Run test to verify it passes**

Run: `cd services && cargo test -p zero-gateway hitl::store::tests -- --nocapture`
Expected: PASS (3 tests)

**Step 5: Commit**

```bash
cd services && git add zero-gateway/src/hitl/store.rs zero-gateway/src/hitl/schema.sql && git commit -m "feat(hitl): add SQLite persistence store with audit logging"
```

---

## Task 3: CardRenderer Trait

**Files:**
- Create: `services/zero-gateway/src/hitl/cards/mod.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_action_from_string() {
        let approve = CallbackAction::from_str("approve");
        assert!(matches!(approve, CallbackAction::Approve));

        let reject = CallbackAction::from_str("reject");
        assert!(matches!(reject, CallbackAction::Reject { .. }));
    }

    #[test]
    fn test_callback_data_parse() {
        let data = CallbackData {
            request_id: "req-123".into(),
            action: CallbackAction::Approve,
            user_id: "user-1".into(),
            platform_callback_id: "cb-456".into(),
        };
        assert_eq!(data.request_id, "req-123");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd services && cargo test -p zero-gateway hitl::cards::tests --no-run 2>&1 | head -20`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// services/zero-gateway/src/hitl/cards/mod.rs

//! Card renderers for different IM channels.

pub mod telegram;
pub mod feishu;
pub mod slack;
pub mod dingtalk;

use anyhow::Result;
use async_trait::async_trait;

use super::ApprovalRequest;

/// Trait for rendering approval cards on different platforms.
#[async_trait]
pub trait CardRenderer: Send + Sync {
    /// Get the channel type name.
    fn channel_type(&self) -> &'static str;

    /// Send an approval card, returning the platform message ID.
    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String>;

    /// Update an existing card after a decision is made.
    async fn update_card(
        &self,
        request: &ApprovalRequest,
        message_id: &str,
    ) -> Result<()>;

    /// Parse callback data from platform webhook payload.
    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData>;
}

/// Parsed callback data from any platform.
#[derive(Debug, Clone)]
pub struct CallbackData {
    /// The approval request ID.
    pub request_id: String,
    /// The action taken.
    pub action: CallbackAction,
    /// The user who took the action.
    pub user_id: String,
    /// Platform-specific callback ID for acknowledgement.
    pub platform_callback_id: String,
}

/// Action from a callback.
#[derive(Debug, Clone)]
pub enum CallbackAction {
    Approve,
    Reject { reason: Option<String> },
}

impl CallbackAction {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "approve" | "approved" => Self::Approve,
            _ => Self::Reject { reason: None },
        }
    }
}

/// Format approval request for display in card.
pub fn format_approval_summary(request: &ApprovalRequest) -> String {
    match &request.approval_type {
        super::ApprovalType::MergeRequest { platform, repo, mr_id } => {
            format!("**合并请求 #{}**\n平台: {}\n仓库: {}", mr_id, platform, repo)
        }
        super::ApprovalType::TradingCommand { asset, action, amount } => {
            format!("**交易指令**\n资产: {}\n操作: {}\n数量: {:.4}", asset, action, amount)
        }
        super::ApprovalType::ConfigChange { key, old_value, new_value } => {
            format!("**配置变更**\n键: {}\n旧值: {}\n新值: {}", key, old_value, new_value)
        }
        super::ApprovalType::HighCostOperation { operation, estimated_cost } => {
            format!("**高成本操作**\n操作: {}\n预估成本: ${:.2}", operation, estimated_cost)
        }
        super::ApprovalType::RiskOperation { description, risk_level } => {
            let level = match risk_level {
                super::RiskLevel::Low => "低",
                super::RiskLevel::Medium => "中",
                super::RiskLevel::High => "高",
                super::RiskLevel::Critical => "严重",
            };
            format!("**风险操作**\n描述: {}\n风险等级: {}", description, level)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_action_from_string() {
        let approve = CallbackAction::from_str("approve");
        assert!(matches!(approve, CallbackAction::Approve));

        let reject = CallbackAction::from_str("reject");
        assert!(matches!(reject, CallbackAction::Reject { .. }));
    }

    #[test]
    fn test_callback_data_parse() {
        let data = CallbackData {
            request_id: "req-123".into(),
            action: CallbackAction::Approve,
            user_id: "user-1".into(),
            platform_callback_id: "cb-456".into(),
        };
        assert_eq!(data.request_id, "req-123");
    }

    #[test]
    fn test_format_approval_summary_mr() {
        let request = ApprovalRequest {
            id: "test".into(),
            approval_type: super::super::ApprovalType::MergeRequest {
                platform: "github".into(),
                repo: "owner/repo".into(),
                mr_id: 42,
            },
            requester_id: "user".into(),
            approvers: vec![],
            status: super::super::ApprovalStatus::Pending,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            channel_type: "telegram".into(),
            channel_id: "123".into(),
            message_id: None,
            context: serde_json::json!({}),
        };

        let summary = format_approval_summary(&request);
        assert!(summary.contains("#42"));
        assert!(summary.contains("github"));
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd services && cargo test -p zero-gateway hitl::cards::tests -- --nocapture`
Expected: PASS (3 tests)

**Step 5: Commit**

```bash
cd services && git add zero-gateway/src/hitl/cards/mod.rs && git commit -m "feat(hitl): add CardRenderer trait and callback types"
```

---

## Task 4: Telegram Card Renderer

**Files:**
- Create: `services/zero-gateway/src/hitl/cards/telegram.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_data() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431".into());

        // Simulated Telegram callback_query payload
        let payload = serde_json::json!({
            "callback_query": {
                "id": "cb-123",
                "from": { "id": 12345, "username": "testuser" },
                "message": { "chat": { "id": 67890 }, "message_id": 111 },
                "data": "hitl:approve:req-abc"
            }
        });

        let bytes = serde_json::to_vec(&payload).unwrap();
        let result = renderer.parse_callback(&bytes).unwrap();

        assert_eq!(result.request_id, "req-abc");
        assert!(matches!(result.action, CallbackAction::Approve));
        assert_eq!(result.user_id, "12345");
    }

    #[test]
    fn test_parse_callback_reject() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431".into());

        let payload = serde_json::json!({
            "callback_query": {
                "id": "cb-456",
                "from": { "id": 12345 },
                "message": { "chat": { "id": 67890 }, "message_id": 222 },
                "data": "hitl:reject:req-xyz"
            }
        });

        let bytes = serde_json::to_vec(&payload).unwrap();
        let result = renderer.parse_callback(&bytes).unwrap();

        assert_eq!(result.request_id, "req-xyz");
        assert!(matches!(result.action, CallbackAction::Reject { .. }));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd services && cargo test -p zero-gateway hitl::cards::telegram::tests --no-run 2>&1 | head -20`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// services/zero-gateway/src/hitl/cards/telegram.rs

//! Telegram card renderer using InlineKeyboard.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{CallbackAction, CallbackData, CardRenderer, format_approval_summary};
use crate::hitl::{ApprovalRequest, ApprovalStatus};

/// Telegram card renderer.
pub struct TelegramCardRenderer {
    channels_endpoint: String,
    client: reqwest::Client,
}

impl TelegramCardRenderer {
    /// Create a new Telegram card renderer.
    pub fn new(channels_endpoint: String) -> Self {
        Self {
            channels_endpoint,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CardRenderer for TelegramCardRenderer {
    fn channel_type(&self) -> &'static str {
        "telegram"
    }

    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String> {
        let summary = format_approval_summary(request);
        let text = format!(
            "🔐 **审批请求**\n\n{}\n\n请求人: {}\n请求ID: `{}`",
            summary, request.requester_id, request.id
        );

        // Build inline keyboard
        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![
                InlineKeyboardButton {
                    text: "✅ 批准".into(),
                    callback_data: Some(format!("hitl:approve:{}", request.id)),
                    url: None,
                },
                InlineKeyboardButton {
                    text: "❌ 拒绝".into(),
                    callback_data: Some(format!("hitl:reject:{}", request.id)),
                    url: None,
                },
            ]],
        };

        let body = SendMessageRequest {
            channel_type: "telegram".into(),
            channel_id: channel_id.into(),
            content: MessageContent::InteractiveMarkdown {
                text,
                reply_markup: keyboard,
            },
        };

        let url = format!("{}/api/v1/send", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send Telegram card")?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            anyhow::bail!("Telegram send failed: {}", err);
        }

        let result: SendMessageResponse = response.json().await?;
        Ok(result.message_id.unwrap_or_default())
    }

    async fn update_card(
        &self,
        request: &ApprovalRequest,
        message_id: &str,
    ) -> Result<()> {
        let status_text = match &request.status {
            ApprovalStatus::Approved { by, .. } => format!("✅ 已批准 (by {})", by),
            ApprovalStatus::Rejected { by, reason, .. } => {
                let reason_text = reason.as_deref().unwrap_or("无");
                format!("❌ 已拒绝 (by {}, 原因: {})", by, reason_text)
            }
            ApprovalStatus::Cancelled { reason } => format!("⚪ 已取消: {}", reason),
            ApprovalStatus::Pending => "⏳ 待审批".into(),
        };

        let summary = format_approval_summary(request);
        let text = format!(
            "🔐 **审批请求** [{}]\n\n{}\n\n请求人: {}\n请求ID: `{}`",
            status_text, summary, request.requester_id, request.id
        );

        let body = EditMessageRequest {
            channel_type: "telegram".into(),
            channel_id: request.channel_id.clone(),
            message_id: message_id.into(),
            text,
        };

        let url = format!("{}/api/v1/edit", self.channels_endpoint);
        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            tracing::warn!("Failed to update Telegram card: {}", response.status());
        }

        Ok(())
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        let value: serde_json::Value = serde_json::from_slice(payload)
            .context("Failed to parse Telegram callback payload")?;

        let callback = value
            .get("callback_query")
            .context("Missing callback_query")?;

        let id = callback
            .get("id")
            .and_then(|v| v.as_str())
            .context("Missing callback id")?;

        let user_id = callback
            .get("from")
            .and_then(|f| f.get("id"))
            .and_then(|v| v.as_i64())
            .map(|id| id.to_string())
            .context("Missing from.id")?;

        let data = callback
            .get("data")
            .and_then(|v| v.as_str())
            .context("Missing callback data")?;

        // Parse data format: "hitl:action:request_id"
        let parts: Vec<&str> = data.split(':').collect();
        if parts.len() != 3 || parts[0] != "hitl" {
            anyhow::bail!("Invalid callback data format: {}", data);
        }

        let action = CallbackAction::from_str(parts[1]);
        let request_id = parts[2].to_string();

        Ok(CallbackData {
            request_id,
            action,
            user_id,
            platform_callback_id: id.into(),
        })
    }
}

// Request/Response types for zero-channels API

#[derive(Serialize)]
struct SendMessageRequest {
    channel_type: String,
    channel_id: String,
    content: MessageContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum MessageContent {
    InteractiveMarkdown {
        text: String,
        reply_markup: InlineKeyboardMarkup,
    },
}

#[derive(Serialize)]
struct InlineKeyboardMarkup {
    inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

#[derive(Serialize)]
struct InlineKeyboardButton {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Deserialize)]
struct SendMessageResponse {
    message_id: Option<String>,
}

#[derive(Serialize)]
struct EditMessageRequest {
    channel_type: String,
    channel_id: String,
    message_id: String,
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_data() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431".into());

        let payload = serde_json::json!({
            "callback_query": {
                "id": "cb-123",
                "from": { "id": 12345, "username": "testuser" },
                "message": { "chat": { "id": 67890 }, "message_id": 111 },
                "data": "hitl:approve:req-abc"
            }
        });

        let bytes = serde_json::to_vec(&payload).unwrap();
        let result = renderer.parse_callback(&bytes).unwrap();

        assert_eq!(result.request_id, "req-abc");
        assert!(matches!(result.action, CallbackAction::Approve));
        assert_eq!(result.user_id, "12345");
    }

    #[test]
    fn test_parse_callback_reject() {
        let renderer = TelegramCardRenderer::new("http://localhost:4431".into());

        let payload = serde_json::json!({
            "callback_query": {
                "id": "cb-456",
                "from": { "id": 12345 },
                "message": { "chat": { "id": 67890 }, "message_id": 222 },
                "data": "hitl:reject:req-xyz"
            }
        });

        let bytes = serde_json::to_vec(&payload).unwrap();
        let result = renderer.parse_callback(&bytes).unwrap();

        assert_eq!(result.request_id, "req-xyz");
        assert!(matches!(result.action, CallbackAction::Reject { .. }));
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd services && cargo test -p zero-gateway hitl::cards::telegram::tests -- --nocapture`
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
cd services && git add zero-gateway/src/hitl/cards/telegram.rs && git commit -m "feat(hitl): add Telegram card renderer with InlineKeyboard"
```

---

## Task 5: Feishu Card Renderer

**Files:**
- Create: `services/zero-gateway/src/hitl/cards/feishu.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feishu_callback() {
        let renderer = FeishuCardRenderer::new("http://localhost:4431".into());

        // Feishu card callback payload
        let payload = serde_json::json!({
            "open_id": "ou_xxx",
            "user_id": "user-123",
            "action": {
                "value": {
                    "action": "approve",
                    "request_id": "req-abc"
                }
            }
        });

        let bytes = serde_json::to_vec(&payload).unwrap();
        let result = renderer.parse_callback(&bytes).unwrap();

        assert_eq!(result.request_id, "req-abc");
        assert!(matches!(result.action, CallbackAction::Approve));
    }

    #[test]
    fn test_build_feishu_card() {
        let card = build_approval_card("req-123", "Test approval", "user-1");
        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("审批请求"));
        assert!(json.contains("req-123"));
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd services && cargo test -p zero-gateway hitl::cards::feishu::tests --no-run 2>&1 | head -20`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// services/zero-gateway/src/hitl/cards/feishu.rs

//! Feishu (飞书) card renderer using Interactive Message Cards.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{CallbackAction, CallbackData, CardRenderer, format_approval_summary};
use crate::hitl::{ApprovalRequest, ApprovalStatus};

/// Feishu card renderer.
pub struct FeishuCardRenderer {
    channels_endpoint: String,
    client: reqwest::Client,
}

impl FeishuCardRenderer {
    /// Create a new Feishu card renderer.
    pub fn new(channels_endpoint: String) -> Self {
        Self {
            channels_endpoint,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CardRenderer for FeishuCardRenderer {
    fn channel_type(&self) -> &'static str {
        "feishu"
    }

    async fn send_approval_card(
        &self,
        request: &ApprovalRequest,
        channel_id: &str,
    ) -> Result<String> {
        let summary = format_approval_summary(request);
        let card = build_approval_card(&request.id, &summary, &request.requester_id);

        let body = json!({
            "channel_type": "feishu",
            "channel_id": channel_id,
            "content": {
                "type": "interactive",
                "card": card
            }
        });

        let url = format!("{}/api/v1/send", self.channels_endpoint);
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send Feishu card")?;

        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            anyhow::bail!("Feishu send failed: {}", err);
        }

        let result: serde_json::Value = response.json().await?;
        let message_id = result
            .get("message_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(message_id)
    }

    async fn update_card(
        &self,
        request: &ApprovalRequest,
        message_id: &str,
    ) -> Result<()> {
        let status_text = match &request.status {
            ApprovalStatus::Approved { by, .. } => format!("✅ 已批准 (by {})", by),
            ApprovalStatus::Rejected { by, reason, .. } => {
                let reason_text = reason.as_deref().unwrap_or("无");
                format!("❌ 已拒绝 (by {}, 原因: {})", by, reason_text)
            }
            ApprovalStatus::Cancelled { reason } => format!("⚪ 已取消: {}", reason),
            ApprovalStatus::Pending => "⏳ 待审批".into(),
        };

        let summary = format_approval_summary(request);
        let card = build_result_card(&request.id, &summary, &request.requester_id, &status_text);

        let body = json!({
            "channel_type": "feishu",
            "message_id": message_id,
            "content": {
                "type": "interactive",
                "card": card
            }
        });

        let url = format!("{}/api/v1/update", self.channels_endpoint);
        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            tracing::warn!("Failed to update Feishu card: {}", response.status());
        }

        Ok(())
    }

    fn parse_callback(&self, payload: &[u8]) -> Result<CallbackData> {
        let value: serde_json::Value = serde_json::from_slice(payload)
            .context("Failed to parse Feishu callback payload")?;

        let user_id = value
            .get("user_id")
            .or_else(|| value.get("open_id"))
            .and_then(|v| v.as_str())
            .context("Missing user_id")?
            .to_string();

        let action_value = value
            .get("action")
            .and_then(|a| a.get("value"))
            .context("Missing action.value")?;

        let action_str = action_value
            .get("action")
            .and_then(|v| v.as_str())
            .context("Missing action.value.action")?;

        let request_id = action_value
            .get("request_id")
            .and_then(|v| v.as_str())
            .context("Missing action.value.request_id")?
            .to_string();

        let action = CallbackAction::from_str(action_str);

        Ok(CallbackData {
            request_id,
            action,
            user_id,
            platform_callback_id: uuid::Uuid::new_v4().to_string(),
        })
    }
}

/// Build Feishu approval card JSON.
pub fn build_approval_card(request_id: &str, summary: &str, requester: &str) -> serde_json::Value {
    json!({
        "header": {
            "title": {
                "tag": "plain_text",
                "content": "🔐 审批请求"
            },
            "template": "orange"
        },
        "elements": [
            {
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": summary
                }
            },
            {
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": format!("**请求人:** {}\n**请求ID:** `{}`", requester, request_id)
                }
            },
            {
                "tag": "hr"
            },
            {
                "tag": "action",
                "actions": [
                    {
                        "tag": "button",
                        "text": {
                            "tag": "plain_text",
                            "content": "✅ 批准"
                        },
                        "type": "primary",
                        "value": {
                            "action": "approve",
                            "request_id": request_id
                        }
                    },
                    {
                        "tag": "button",
                        "text": {
                            "tag": "plain_text",
                            "content": "❌ 拒绝"
                        },
                        "type": "danger",
                        "value": {
                            "action": "reject",
                            "request_id": request_id
                        }
                    }
                ]
            }
        ]
    })
}

/// Build Feishu result card (after decision).
fn build_result_card(
    request_id: &str,
    summary: &str,
    requester: &str,
    status: &str,
) -> serde_json::Value {
    let template = if status.contains("已批准") {
        "green"
    } else if status.contains("已拒绝") {
        "red"
    } else {
        "grey"
    };

    json!({
        "header": {
            "title": {
                "tag": "plain_text",
                "content": format!("🔐 审批请求 [{}]", status)
            },
            "template": template
        },
        "elements": [
            {
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": summary
                }
            },
            {
                "tag": "div",
                "text": {
                    "tag": "lark_md",
                    "content": format!("**请求人:** {}\n**请求ID:** `{}`", requester, request_id)
                }
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_feishu_callback() {
        let renderer = FeishuCardRenderer::new("http://localhost:4431".into());

        let payload = serde_json::json!({
            "open_id": "ou_xxx",
            "user_id": "user-123",
            "action": {
                "value": {
                    "action": "approve",
                    "request_id": "req-abc"
                }
            }
        });

        let bytes = serde_json::to_vec(&payload).unwrap();
        let result = renderer.parse_callback(&bytes).unwrap();

        assert_eq!(result.request_id, "req-abc");
        assert!(matches!(result.action, CallbackAction::Approve));
    }

    #[test]
    fn test_build_feishu_card() {
        let card = build_approval_card("req-123", "Test approval", "user-1");
        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("审批请求"));
        assert!(json.contains("req-123"));
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cd services && cargo test -p zero-gateway hitl::cards::feishu::tests -- --nocapture`
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
cd services && git add zero-gateway/src/hitl/cards/feishu.rs && git commit -m "feat(hitl): add Feishu card renderer with Interactive Message Cards"
```

---

## Task 6: Slack Card Renderer

**Files:**
- Create: `services/zero-gateway/src/hitl/cards/slack.rs`

**Step 1-5:** Follow same TDD pattern as Tasks 4-5. Implementation uses Slack Block Kit format.

**Commit message:** `feat(hitl): add Slack card renderer with Block Kit`

---

## Task 7: DingTalk Card Renderer

**Files:**
- Create: `services/zero-gateway/src/hitl/cards/dingtalk.rs`

**Step 1-5:** Follow same TDD pattern. Implementation uses DingTalk ActionCard format.

**Commit message:** `feat(hitl): add DingTalk card renderer with ActionCard`

---

## Task 8: HitL API Routes

**Files:**
- Create: `services/zero-gateway/src/hitl/routes.rs`

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_approval_request() {
        let app = create_test_app().await;

        let body = serde_json::json!({
            "approval_type": {
                "type": "MergeRequest",
                "platform": "github",
                "repo": "owner/repo",
                "mr_id": 123
            },
            "requester_id": "user-1",
            "approvers": ["admin-1"],
            "channel_type": "telegram",
            "channel_id": "123456"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/hitl/request")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
```

**Step 2-5:** Implement routes following the API design from the design document.

**Commit message:** `feat(hitl): add API routes for approval workflow`

---

## Task 9: Action Handlers

**Files:**
- Create: `services/zero-gateway/src/hitl/actions.rs`

**Step 1-5:** Implement ApprovalAction trait and ActionRegistry. Add MergeRequestAction as the first handler.

**Commit message:** `feat(hitl): add post-approval action handlers`

---

## Task 10: HitL Client for Other Services

**Files:**
- Create: `services/zero-common/src/hitl_client.rs`
- Modify: `services/zero-common/src/lib.rs`

**Step 1-5:** Implement HitLClient with create_request, check_status, and cancel methods.

**Commit message:** `feat(zero-common): add HitLClient for workflow integration`

---

## Task 11: Integration with review_bridge

**Files:**
- Modify: `services/zero-workflow/src/review_bridge.rs`

**Step 1-5:** Add HitL integration to ReviewBridge for MR merge approvals.

**Commit message:** `feat(review_bridge): integrate HitL for merge approvals`

---

## Task 12: Configuration Update

**Files:**
- Modify: `services/zero-common/src/config.rs`

**Step 1-5:** Add HitL configuration section.

**Commit message:** `feat(config): add HitL configuration section`

---

## Task 13: Integration Tests

**Files:**
- Create: `services/zero-gateway/tests/hitl_integration.rs`

**Step 1-5:** Write integration tests for full approval flow.

**Commit message:** `test(hitl): add integration tests for approval flow`

---

## Summary

| Task | Component | Files | Estimated Commits |
|------|-----------|-------|-------------------|
| 1 | Core Types | `hitl/mod.rs` | 1 |
| 2 | SQLite Store | `hitl/store.rs`, `hitl/schema.sql` | 1 |
| 3 | CardRenderer Trait | `hitl/cards/mod.rs` | 1 |
| 4 | Telegram Renderer | `hitl/cards/telegram.rs` | 1 |
| 5 | Feishu Renderer | `hitl/cards/feishu.rs` | 1 |
| 6 | Slack Renderer | `hitl/cards/slack.rs` | 1 |
| 7 | DingTalk Renderer | `hitl/cards/dingtalk.rs` | 1 |
| 8 | API Routes | `hitl/routes.rs` | 1 |
| 9 | Action Handlers | `hitl/actions.rs` | 1 |
| 10 | HitL Client | `zero-common/hitl_client.rs` | 1 |
| 11 | review_bridge Integration | `review_bridge.rs` | 1 |
| 12 | Configuration | `config.rs` | 1 |
| 13 | Integration Tests | `tests/hitl_integration.rs` | 1 |

**Total: 13 tasks, ~13 commits**
