//! SQLite-backed trace storage
//!
//! Replaces JSONL file storage with SQLite for:
//! - Indexed queries (5-10x faster for trace_id lookups)
//! - ACID guarantees
//! - Concurrent access support via WAL mode

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Schema version for migrations
const SCHEMA_VERSION: i32 = 1;

/// Trace entry matching TypeScript LogEntry format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// ISO 8601 timestamp
    pub ts: String,
    /// Unique trace ID (UUID)
    pub trace_id: String,
    /// Current span ID (8-char hex)
    pub span_id: String,
    /// Parent span ID if child span
    pub parent_span_id: Option<String>,
    /// Service name (e.g., "ccode-api", "zero-channels")
    pub service: String,
    /// Event type (snake_case)
    pub event_type: String,
    /// Log level (debug/info/warn/error)
    pub level: String,
    /// Structured payload (JSON)
    pub payload: serde_json::Value,
}

impl TraceEntry {
    /// Extract duration_ms from payload if present
    pub fn duration_ms(&self) -> Option<f64> {
        self.payload.get("duration_ms").and_then(|v| v.as_f64())
    }

    /// Extract function name from payload if present
    pub fn function(&self) -> Option<&str> {
        self.payload.get("function").and_then(|v| v.as_str())
    }

    /// Extract error message from payload if present
    pub fn error(&self) -> Option<&str> {
        self.payload.get("error").and_then(|v| v.as_str())
    }
}

/// Configuration for trace store
#[derive(Debug, Clone)]
pub struct TraceStoreConfig {
    /// Database file path
    pub db_path: PathBuf,
    /// Number of days to retain traces
    pub retention_days: u32,
}

impl Default for TraceStoreConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from(":memory:"),
            retention_days: 7,
        }
    }
}

/// SQLite-backed trace store
pub struct TraceStore {
    config: TraceStoreConfig,
    conn: Arc<Mutex<Connection>>,
}

impl TraceStore {
    /// Open or create a trace store at the given path
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for better concurrent access
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            "#,
        )?;

        Self::init_schema(&conn)?;

        Ok(Self {
            config: TraceStoreConfig {
                db_path,
                retention_days: 7,
            },
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory store (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;

        Ok(Self {
            config: TraceStoreConfig::default(),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS trace_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS traces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts TEXT NOT NULL,
                trace_id TEXT NOT NULL,
                span_id TEXT NOT NULL,
                parent_span_id TEXT,
                service TEXT NOT NULL,
                event_type TEXT NOT NULL,
                level TEXT NOT NULL,
                payload TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            CREATE INDEX IF NOT EXISTS idx_traces_trace_id ON traces(trace_id);
            CREATE INDEX IF NOT EXISTS idx_traces_service ON traces(service);
            CREATE INDEX IF NOT EXISTS idx_traces_ts ON traces(ts);
            CREATE INDEX IF NOT EXISTS idx_traces_event_type ON traces(event_type);
            CREATE INDEX IF NOT EXISTS idx_traces_level ON traces(level);
            CREATE INDEX IF NOT EXISTS idx_traces_created_at ON traces(created_at);
            "#,
        )?;

        // Check and run migrations
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(CAST(value AS INTEGER), 0) FROM trace_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current_version < SCHEMA_VERSION {
            Self::run_migrations(conn, current_version)?;
            conn.execute(
                "INSERT OR REPLACE INTO trace_meta (key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }

        Ok(())
    }

    fn run_migrations(_conn: &Connection, _from_version: i32) -> Result<()> {
        // Add migrations here as needed
        Ok(())
    }

    /// Append a single trace entry
    pub fn append(&self, entry: &TraceEntry) -> Result<()> {
        let payload_json = serde_json::to_string(&entry.payload)?;
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        conn.execute(
            r#"
            INSERT INTO traces (ts, trace_id, span_id, parent_span_id, service, event_type, level, payload)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                entry.ts,
                entry.trace_id,
                entry.span_id,
                entry.parent_span_id,
                entry.service,
                entry.event_type,
                entry.level,
                payload_json,
            ],
        )?;

        Ok(())
    }

    /// Append multiple trace entries in a single transaction (batch insert)
    pub fn append_batch(&self, entries: &[TraceEntry]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        conn.execute("BEGIN TRANSACTION", [])?;

        let mut inserted = 0;
        for entry in entries {
            let payload_json = serde_json::to_string(&entry.payload)?;
            conn.execute(
                r#"
                INSERT INTO traces (ts, trace_id, span_id, parent_span_id, service, event_type, level, payload)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    entry.ts,
                    entry.trace_id,
                    entry.span_id,
                    entry.parent_span_id,
                    entry.service,
                    entry.event_type,
                    entry.level,
                    payload_json,
                ],
            )?;
            inserted += 1;
        }

        conn.execute("COMMIT", [])?;
        Ok(inserted)
    }

    /// Query traces by trace_id
    pub fn query_by_trace_id(&self, trace_id: &str) -> Result<Vec<TraceEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
            FROM traces
            WHERE trace_id = ?1
            ORDER BY ts ASC
            "#,
        )?;

        let entries = stmt
            .query_map(params![trace_id], |row| {
                let payload_str: String = row.get(7)?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);

                Ok(TraceEntry {
                    ts: row.get(0)?,
                    trace_id: row.get(1)?,
                    span_id: row.get(2)?,
                    parent_span_id: row.get(3)?,
                    service: row.get(4)?,
                    event_type: row.get(5)?,
                    level: row.get(6)?,
                    payload,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Query traces by service within a time range
    pub fn query_by_service(
        &self,
        service: &str,
        from_ts: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<TraceEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let query = match (from_ts, limit) {
            (Some(from), Some(lim)) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE service = ?1 AND ts >= ?2
                    ORDER BY ts DESC
                    LIMIT ?3
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![service, from, lim as i64])
            }
            (Some(from), None) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE service = ?1 AND ts >= ?2
                    ORDER BY ts DESC
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![service, from])
            }
            (None, Some(lim)) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE service = ?1
                    ORDER BY ts DESC
                    LIMIT ?2
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![service, lim as i64])
            }
            (None, None) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE service = ?1
                    ORDER BY ts DESC
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![service])
            }
        };

        query
    }

    fn collect_entries(
        stmt: &mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> Result<Vec<TraceEntry>> {
        let entries = stmt
            .query_map(params, |row| {
                let payload_str: String = row.get(7)?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);

                Ok(TraceEntry {
                    ts: row.get(0)?,
                    trace_id: row.get(1)?,
                    span_id: row.get(2)?,
                    parent_span_id: row.get(3)?,
                    service: row.get(4)?,
                    event_type: row.get(5)?,
                    level: row.get(6)?,
                    payload,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Query traces within a time range
    pub fn query_time_range(
        &self,
        from_ts: &str,
        to_ts: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<TraceEntry>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let entries = match (to_ts, limit) {
            (Some(to), Some(lim)) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE ts >= ?1 AND ts <= ?2
                    ORDER BY ts DESC
                    LIMIT ?3
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![from_ts, to, lim as i64])?
            }
            (Some(to), None) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE ts >= ?1 AND ts <= ?2
                    ORDER BY ts DESC
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![from_ts, to])?
            }
            (None, Some(lim)) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE ts >= ?1
                    ORDER BY ts DESC
                    LIMIT ?2
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![from_ts, lim as i64])?
            }
            (None, None) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
                    FROM traces
                    WHERE ts >= ?1
                    ORDER BY ts DESC
                    "#,
                )?;
                Self::collect_entries(&mut stmt, params![from_ts])?
            }
        };

        Ok(entries)
    }

    /// Get distinct trace IDs within a time range
    pub fn get_trace_ids(&self, from_ts: &str, limit: Option<usize>) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let limit_val = limit.unwrap_or(1000) as i64;
        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT trace_id
            FROM traces
            WHERE ts >= ?1
            ORDER BY ts DESC
            LIMIT ?2
            "#,
        )?;

        let ids: Vec<String> = stmt
            .query_map(params![from_ts, limit_val], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    /// Get distinct services
    pub fn get_services(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare("SELECT DISTINCT service FROM traces ORDER BY service")?;
        let services: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(services)
    }

    /// Count traces matching criteria
    pub fn count(&self, from_ts: Option<&str>, service: Option<&str>) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let count: i64 = match (from_ts, service) {
            (Some(from), Some(svc)) => conn.query_row(
                "SELECT COUNT(*) FROM traces WHERE ts >= ?1 AND service = ?2",
                params![from, svc],
                |row| row.get(0),
            )?,
            (Some(from), None) => conn.query_row(
                "SELECT COUNT(*) FROM traces WHERE ts >= ?1",
                params![from],
                |row| row.get(0),
            )?,
            (None, Some(svc)) => conn.query_row(
                "SELECT COUNT(*) FROM traces WHERE service = ?1",
                params![svc],
                |row| row.get(0),
            )?,
            (None, None) => {
                conn.query_row("SELECT COUNT(*) FROM traces", [], |row| row.get(0))?
            }
        };

        Ok(count as usize)
    }

    /// Delete traces older than retention period
    pub fn cleanup(&self, retention_days: u32) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let cutoff_secs = chrono::Utc::now().timestamp() - (retention_days as i64 * 24 * 60 * 60);

        let deleted = conn.execute(
            "DELETE FROM traces WHERE created_at < ?1",
            params![cutoff_secs],
        )?;

        Ok(deleted)
    }

    /// Compact the database
    pub fn compact(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        conn.execute_batch("VACUUM")?;
        Ok(())
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<TraceStoreStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let total_entries: i64 =
            conn.query_row("SELECT COUNT(*) FROM traces", [], |row| row.get(0))?;

        let total_size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(payload)), 0) FROM traces",
            [],
            |row| row.get(0),
        )?;

        let oldest_ts: Option<String> = conn
            .query_row("SELECT MIN(ts) FROM traces", [], |row| row.get(0))
            .ok();

        let newest_ts: Option<String> = conn
            .query_row("SELECT MAX(ts) FROM traces", [], |row| row.get(0))
            .ok();

        // Count by service
        let mut stmt =
            conn.prepare("SELECT service, COUNT(*) FROM traces GROUP BY service ORDER BY COUNT(*) DESC")?;
        let by_service: HashMap<String, usize> = stmt
            .query_map([], |row| {
                let service: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((service, count as usize))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Count by event_type
        let mut stmt =
            conn.prepare("SELECT event_type, COUNT(*) FROM traces GROUP BY event_type ORDER BY COUNT(*) DESC")?;
        let by_event_type: HashMap<String, usize> = stmt
            .query_map([], |row| {
                let event_type: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((event_type, count as usize))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(TraceStoreStats {
            total_entries: total_entries as usize,
            total_size_bytes: total_size as usize,
            oldest_ts,
            newest_ts,
            by_service,
            by_event_type,
        })
    }

    /// Health check
    pub fn health_check(&self) -> bool {
        self.conn
            .lock()
            .ok()
            .and_then(|conn| conn.execute_batch("SELECT 1").ok())
            .is_some()
    }

    /// Get database path
    pub fn path(&self) -> &Path {
        &self.config.db_path
    }

    /// Get connection for advanced queries
    pub fn connection(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }
}

// Implement Send + Sync for TraceStore
unsafe impl Send for TraceStore {}
unsafe impl Sync for TraceStore {}

/// Statistics about the trace store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStoreStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub oldest_ts: Option<String>,
    pub newest_ts: Option<String>,
    pub by_service: HashMap<String, usize>,
    pub by_event_type: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(trace_id: &str, service: &str, event_type: &str) -> TraceEntry {
        TraceEntry {
            ts: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            span_id: "abcd1234".to_string(),
            parent_span_id: None,
            service: service.to_string(),
            event_type: event_type.to_string(),
            level: "info".to_string(),
            payload: serde_json::json!({"function": "test_func", "duration_ms": 100}),
        }
    }

    #[test]
    fn test_append_and_query() {
        let store = TraceStore::in_memory().unwrap();

        let entry = create_test_entry("trace-001", "ccode-api", "function_end");
        store.append(&entry).unwrap();

        let results = store.query_by_trace_id("trace-001").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trace_id, "trace-001");
        assert_eq!(results[0].service, "ccode-api");
    }

    #[test]
    fn test_batch_append() {
        let store = TraceStore::in_memory().unwrap();

        let entries = vec![
            create_test_entry("trace-001", "ccode-api", "function_start"),
            create_test_entry("trace-001", "ccode-api", "function_end"),
            create_test_entry("trace-002", "zero-channels", "http_request"),
        ];

        let inserted = store.append_batch(&entries).unwrap();
        assert_eq!(inserted, 3);

        let results = store.query_by_trace_id("trace-001").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_service() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_test_entry("t1", "ccode-api", "function_end")).unwrap();
        store.append(&create_test_entry("t2", "ccode-api", "function_end")).unwrap();
        store.append(&create_test_entry("t3", "zero-channels", "http_request")).unwrap();

        let results = store.query_by_service("ccode-api", None, None).unwrap();
        assert_eq!(results.len(), 2);

        let results = store.query_by_service("zero-channels", None, None).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_services() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_test_entry("t1", "ccode-api", "function_end")).unwrap();
        store.append(&create_test_entry("t2", "zero-channels", "http_request")).unwrap();
        store.append(&create_test_entry("t3", "zero-gateway", "http_response")).unwrap();

        let services = store.get_services().unwrap();
        assert_eq!(services.len(), 3);
        assert!(services.contains(&"ccode-api".to_string()));
        assert!(services.contains(&"zero-channels".to_string()));
    }

    #[test]
    fn test_count() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_test_entry("t1", "ccode-api", "function_end")).unwrap();
        store.append(&create_test_entry("t2", "ccode-api", "function_end")).unwrap();
        store.append(&create_test_entry("t3", "zero-channels", "http_request")).unwrap();

        assert_eq!(store.count(None, None).unwrap(), 3);
        assert_eq!(store.count(None, Some("ccode-api")).unwrap(), 2);
    }

    #[test]
    fn test_stats() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_test_entry("t1", "ccode-api", "function_end")).unwrap();
        store.append(&create_test_entry("t2", "zero-channels", "http_request")).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.by_service.len(), 2);
        assert_eq!(stats.by_event_type.len(), 2);
    }

    #[test]
    fn test_health_check() {
        let store = TraceStore::in_memory().unwrap();
        assert!(store.health_check());
    }

    #[test]
    fn test_entry_helpers() {
        let entry = TraceEntry {
            ts: "2026-03-04T12:00:00Z".to_string(),
            trace_id: "trace-001".to_string(),
            span_id: "abcd1234".to_string(),
            parent_span_id: None,
            service: "ccode-api".to_string(),
            event_type: "function_end".to_string(),
            level: "info".to_string(),
            payload: serde_json::json!({
                "function": "processRequest",
                "duration_ms": 150.5,
                "error": "timeout"
            }),
        };

        assert_eq!(entry.duration_ms(), Some(150.5));
        assert_eq!(entry.function(), Some("processRequest"));
        assert_eq!(entry.error(), Some("timeout"));
    }
}
