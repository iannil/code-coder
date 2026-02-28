//! SQLite state storage for Hands.
//!
//! Maintains:
//! - hand_executions: Complete execution history
//! - hand_state: Current state per hand (for persistence between runs)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection};

/// Execution status for a hand run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Scheduled,
    Running,
    Success,
    Failed,
    Cancelled,
}

/// A single execution record for a hand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandExecution {
    /// Unique execution ID
    pub id: String,

    /// Hand ID
    pub hand_id: String,

    /// Status of this execution
    pub status: ExecutionStatus,

    /// Start timestamp
    pub started_at: DateTime<Utc>,

    /// End timestamp (null if running)
    pub ended_at: Option<DateTime<Utc>>,

    /// Agent response output
    pub output: Option<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// Memory file path (if written)
    pub memory_path: Option<String>,

    /// Previous execution ID (for context chaining)
    pub previous_execution_id: Option<String>,

    /// Metadata (execution duration, etc.)
    pub metadata: serde_json::Value,
}

/// Current state for a hand (persistent across runs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandState {
    /// Hand ID
    pub hand_id: String,

    /// Last execution ID
    pub last_execution_id: Option<String>,

    /// Last successful execution timestamp
    pub last_success_at: Option<DateTime<Utc>>,

    /// Last failed execution timestamp
    pub last_failure_at: Option<DateTime<Utc>>,

    /// Run count (total)
    pub total_runs: i64,

    /// Success count
    pub success_count: i64,

    /// Failure count
    pub failure_count: i64,

    /// Custom state data (agent-defined)
    pub custom_state: serde_json::Value,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// State store for Hands using SQLite.
pub struct StateStore {
    #[allow(dead_code)]
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl StateStore {
    /// Create a new state store with default data directory.
    pub fn new() -> Result<Self> {
        let data_dir = zero_common::config::config_dir().join("workflow");
        Self::with_data_dir(data_dir)
    }

    /// Create a new state store with a specific data directory.
    pub fn with_data_dir(data_dir: PathBuf) -> Result<Self> {
        let db_path = data_dir.join("hands.db");

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create state directory: {}", parent.display()))?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open hands database: {}", db_path.display()))?;

        let store = Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
        };

        store.init_schema()?;

        Ok(store)
    }

    /// Initialize database schema.
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS hand_executions (
                id                  TEXT PRIMARY KEY,
                hand_id             TEXT NOT NULL,
                status              TEXT NOT NULL,
                started_at          TEXT NOT NULL,
                ended_at            TEXT,
                output              TEXT,
                error               TEXT,
                memory_path         TEXT,
                previous_execution_id TEXT,
                metadata            TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_hand_executions_hand_id ON hand_executions(hand_id);
            CREATE INDEX IF NOT EXISTS idx_hand_executions_started_at ON hand_executions(started_at);

            CREATE TABLE IF NOT EXISTS hand_state (
                hand_id             TEXT PRIMARY KEY,
                last_execution_id   TEXT,
                last_success_at     TEXT,
                last_failure_at     TEXT,
                total_runs          INTEGER NOT NULL DEFAULT 0,
                success_count       INTEGER NOT NULL DEFAULT 0,
                failure_count       INTEGER NOT NULL DEFAULT 0,
                custom_state        TEXT,
                updated_at          TEXT NOT NULL
            );",
        )
        .context("Failed to initialize hands schema")?;

        Ok(())
    }

    /// Create a new execution record.
    pub fn create_execution(
        &self,
        hand_id: &str,
        previous_execution_id: Option<String>,
    ) -> Result<HandExecution> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        conn.execute(
            "INSERT INTO hand_executions (id, hand_id, status, started_at, previous_execution_id, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                hand_id,
                "scheduled",
                now.to_rfc3339(),
                previous_execution_id,
                "{}",
            ],
        )
        .context("Failed to create execution record")?;

        Ok(HandExecution {
            id,
            hand_id: hand_id.to_string(),
            status: ExecutionStatus::Scheduled,
            started_at: now,
            ended_at: None,
            output: None,
            error: None,
            memory_path: None,
            previous_execution_id,
            metadata: serde_json::json!({}),
        })
    }

    /// Update an execution record.
    pub fn update_execution(
        &self,
        execution: &HandExecution,
    ) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        let ended_at = execution.ended_at.map(|t| t.to_rfc3339());
        let metadata = serde_json::to_string(&execution.metadata)
            .unwrap_or_else(|_| "{}".to_string());

        // Convert ExecutionStatus to string for storage
        let status_str = match execution.status {
            ExecutionStatus::Scheduled => "scheduled",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Success => "success",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Cancelled => "cancelled",
        };

        conn.execute(
            "UPDATE hand_executions
             SET status = ?1, ended_at = ?2, output = ?3, error = ?4, memory_path = ?5, metadata = ?6
             WHERE id = ?7",
            params![
                status_str,
                ended_at,
                execution.output.as_deref(),
                execution.error.as_deref(),
                execution.memory_path.as_deref(),
                metadata,
                execution.id,
            ],
        )
        .context("Failed to update execution record")?;

        Ok(())
    }

    /// Get executions for a hand, most recent first.
    pub fn get_executions(
        &self,
        hand_id: &str,
        limit: usize,
    ) -> Result<Vec<HandExecution>> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, hand_id, status, started_at, ended_at, output, error, memory_path, previous_execution_id, metadata
             FROM hand_executions
             WHERE hand_id = ?1
             ORDER BY started_at DESC
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![hand_id, limit], |row| {
            let status_str: String = row.get(2)?;
            let status = match status_str.as_str() {
                "scheduled" => ExecutionStatus::Scheduled,
                "running" => ExecutionStatus::Running,
                "success" => ExecutionStatus::Success,
                "failed" => ExecutionStatus::Failed,
                "cancelled" => ExecutionStatus::Cancelled,
                _ => ExecutionStatus::Failed,
            };

            let started_at_str: String = row.get(3)?;
            let ended_at_str: Option<String> = row.get(4)?;
            let metadata_str: String = row.get(9)?;

            // Parse timestamps, using Utc::now() as fallback on error
            let started_at = parse_rfc3339(&started_at_str).unwrap_or_else(|_| Utc::now());
            let ended_at = ended_at_str.as_deref()
                .and_then(|s| parse_rfc3339(s).ok());

            Ok(HandExecution {
                id: row.get(0)?,
                hand_id: row.get(1)?,
                status,
                started_at,
                ended_at,
                output: row.get(5)?,
                error: row.get(6)?,
                memory_path: row.get(7)?,
                previous_execution_id: row.get(8)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_else(|_| serde_json::json!({})),
            })
        })?;

        let mut executions = Vec::new();
        for row in rows {
            executions.push(row?);
        }

        Ok(executions)
    }

    /// Get a specific execution by ID.
    pub fn get_execution(&self, execution_id: &str) -> Result<Option<HandExecution>> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, hand_id, status, started_at, ended_at, output, error, memory_path, previous_execution_id, metadata
             FROM hand_executions
             WHERE id = ?1",
        )?;

        let rows = stmt.query_map(params![execution_id], |row| {
            let status_str: String = row.get(2)?;
            let status = match status_str.as_str() {
                "scheduled" => ExecutionStatus::Scheduled,
                "running" => ExecutionStatus::Running,
                "success" => ExecutionStatus::Success,
                "failed" => ExecutionStatus::Failed,
                "cancelled" => ExecutionStatus::Cancelled,
                _ => ExecutionStatus::Failed,
            };

            let started_at_str: String = row.get(3)?;
            let ended_at_str: Option<String> = row.get(4)?;
            let metadata_str: String = row.get(9)?;

            // Parse timestamps, using Utc::now() as fallback on error
            let started_at = parse_rfc3339(&started_at_str).unwrap_or_else(|_| Utc::now());
            let ended_at = ended_at_str.as_deref()
                .and_then(|s| parse_rfc3339(s).ok());

            Ok(HandExecution {
                id: row.get(0)?,
                hand_id: row.get(1)?,
                status,
                started_at,
                ended_at,
                output: row.get(5)?,
                error: row.get(6)?,
                memory_path: row.get(7)?,
                previous_execution_id: row.get(8)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_else(|_| serde_json::json!({})),
            })
        })?;

        for row in rows {
            return Ok(Some(row?));
        }

        Ok(None)
    }

    /// Get or create state for a hand.
    pub fn get_state(&self, hand_id: &str) -> Result<HandState> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT hand_id, last_execution_id, last_success_at, last_failure_at, total_runs, success_count, failure_count, custom_state, updated_at
             FROM hand_state
             WHERE hand_id = ?1",
        )?;

        let rows = stmt.query_map(params![hand_id], |row| {
            let last_success_str: Option<String> = row.get(2)?;
            let last_failure_str: Option<String> = row.get(3)?;
            let custom_state_str: String = row.get(7)?;
            let updated_at_str: String = row.get(8)?;

            // Parse timestamps, using Utc::now() as fallback on error
            let last_success_at = last_success_str.as_deref()
                .and_then(|s| parse_rfc3339(s).ok());
            let last_failure_at = last_failure_str.as_deref()
                .and_then(|s| parse_rfc3339(s).ok());
            let updated_at = parse_rfc3339(&updated_at_str).unwrap_or_else(|_| Utc::now());

            Ok(HandState {
                hand_id: row.get(0)?,
                last_execution_id: row.get(1)?,
                last_success_at,
                last_failure_at,
                total_runs: row.get(4)?,
                success_count: row.get(5)?,
                failure_count: row.get(6)?,
                custom_state: serde_json::from_str(&custom_state_str).unwrap_or_else(|_| serde_json::json!({})),
                updated_at,
            })
        })?;

        for row in rows {
            return Ok(row?);
        }

        // Create new state
        let now = Utc::now();
        conn.execute(
            "INSERT INTO hand_state (hand_id, total_runs, success_count, failure_count, custom_state, updated_at)
             VALUES (?1, 0, 0, 0, ?2, ?3)",
            params![hand_id, "{}", now.to_rfc3339()],
        )
        .context("Failed to create hand state")?;

        Ok(HandState {
            hand_id: hand_id.to_string(),
            last_execution_id: None,
            last_success_at: None,
            last_failure_at: None,
            total_runs: 0,
            success_count: 0,
            failure_count: 0,
            custom_state: serde_json::json!({}),
            updated_at: now,
        })
    }

    /// Update state for a hand after execution.
    pub fn update_state(
        &self,
        hand_id: &str,
        execution: &HandExecution,
    ) -> Result<()> {
        // Get current state first (releases lock before we update)
        let mut state = self.get_state(hand_id)?;

        let now = Utc::now();
        state.last_execution_id = Some(execution.id.clone());
        state.total_runs += 1;

        match execution.status {
            ExecutionStatus::Success => {
                state.success_count += 1;
                state.last_success_at = Some(now);
            }
            ExecutionStatus::Failed | ExecutionStatus::Cancelled => {
                state.failure_count += 1;
                state.last_failure_at = Some(now);
            }
            _ => {}
        }

        state.updated_at = now;

        let custom_state = serde_json::to_string(&state.custom_state)
            .unwrap_or_else(|_| "{}".to_string());

        // Now lock the connection for the update
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        conn.execute(
            "UPDATE hand_state
             SET last_execution_id = ?1, last_success_at = ?2, last_failure_at = ?3,
                 total_runs = ?4, success_count = ?5, failure_count = ?6, custom_state = ?7, updated_at = ?8
             WHERE hand_id = ?9",
            params![
                state.last_execution_id,
                state.last_success_at.map(|t| t.to_rfc3339()),
                state.last_failure_at.map(|t| t.to_rfc3339()),
                state.total_runs,
                state.success_count,
                state.failure_count,
                custom_state,
                now.to_rfc3339(),
                hand_id,
            ],
        )
        .context("Failed to update hand state")?;

        Ok(())
    }

    /// Clean up old execution records for a hand.
    pub fn cleanup_executions(&self, hand_id: &str, keep: usize) -> Result<()> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Connection lock failed: {}", e))?;

        // Get total count
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM hand_executions WHERE hand_id = ?1",
            params![hand_id],
            |row| row.get(0),
        )?;

        if count <= keep as i64 {
            return Ok(());
        }

        // Delete oldest records beyond keep limit
        conn.execute(
            "DELETE FROM hand_executions
             WHERE id IN (
                 SELECT id FROM hand_executions
                 WHERE hand_id = ?1
                 ORDER BY started_at DESC
                 LIMIT -1 OFFSET ?2
             )",
            params![hand_id, keep as i64],
        )
        .context("Failed to cleanup old executions")?;

        tracing::debug!(
            hand_id = %hand_id,
            deleted = count - keep as i64,
            "Cleaned up old execution records"
        );

        Ok(())
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new().expect("Failed to create StateStore")
    }
}

/// Parse an RFC3339 timestamp.
fn parse_rfc3339(raw: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(raw)
        .with_context(|| format!("Invalid RFC3339 timestamp: {raw}"))?;
    Ok(parsed.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_store(tmp: &TempDir) -> StateStore {
        StateStore::with_data_dir(tmp.path().to_path_buf()).unwrap()
    }

    #[test]
    fn test_create_execution() {
        let tmp = TempDir::new().unwrap();
        let store = test_store(&tmp);

        let exec = store.create_execution("test-hand", None).unwrap();
        assert_eq!(exec.hand_id, "test-hand");
        assert_eq!(exec.status, ExecutionStatus::Scheduled);
        assert!(exec.ended_at.is_none());
    }

    #[test]
    fn test_update_execution() {
        let tmp = TempDir::new().unwrap();
        let store = test_store(&tmp);

        let mut exec = store.create_execution("test-hand", None).unwrap();
        exec.status = ExecutionStatus::Success;
        exec.output = Some("Test output".to_string());
        exec.ended_at = Some(Utc::now());

        store.update_execution(&exec).unwrap();

        let retrieved = store.get_execution(&exec.id).unwrap().unwrap();
        assert_eq!(retrieved.status, ExecutionStatus::Success);
        assert_eq!(retrieved.output.as_deref(), Some("Test output"));
    }

    #[test]
    fn test_state_tracking() {
        let tmp = TempDir::new().unwrap();
        let store = test_store(&tmp);

        // New state should have zeros
        let state = store.get_state("test-hand").unwrap();
        assert_eq!(state.hand_id, "test-hand");
        assert_eq!(state.total_runs, 0);
        assert_eq!(state.success_count, 0);

        // After success execution
        let mut exec = store.create_execution("test-hand", None).unwrap();
        exec.status = ExecutionStatus::Success;
        exec.ended_at = Some(Utc::now());
        store.update_state("test-hand", &exec).unwrap();

        let state = store.get_state("test-hand").unwrap();
        assert_eq!(state.total_runs, 1);
        assert_eq!(state.success_count, 1);
        assert!(state.last_success_at.is_some());
    }

    #[test]
    fn test_get_executions_limit() {
        let tmp = TempDir::new().unwrap();
        let store = test_store(&tmp);

        let hand_id = "limit-hand";
        for _ in 0..5 {
            let exec = store.create_execution(hand_id, None).unwrap();
            store.update_execution(&exec).unwrap();
        }

        let executions = store.get_executions(hand_id, 3).unwrap();
        assert_eq!(executions.len(), 3);
    }

    #[test]
    fn test_cleanup_old_executions() {
        let tmp = TempDir::new().unwrap();
        let store = test_store(&tmp);

        let hand_id = "cleanup-hand";
        for _ in 0..10 {
            let exec = store.create_execution(hand_id, None).unwrap();
            store.update_execution(&exec).unwrap();
        }

        store.cleanup_executions(hand_id, 5).unwrap();

        let executions = store.get_executions(hand_id, 100).unwrap();
        assert_eq!(executions.len(), 5);
    }
}
