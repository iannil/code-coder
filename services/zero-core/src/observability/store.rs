//! SQLite-backed observability store
//!
//! Provides persistent storage for observability events with:
//! - High-performance querying by trace_id, session, agent, time range
//! - Automatic cleanup of old events
//! - Metrics aggregation support

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use super::event::{
    AgentLifecycleEvent, Event, EventType, LlmCallEvent, SpanEvent,
    ToolExecutionEvent,
};
use super::metrics::{MetricsAggregator, MetricsSummary};

/// Schema version for migrations
const SCHEMA_VERSION: i32 = 1;

/// Configuration for observability store
#[derive(Debug, Clone)]
pub struct ObservabilityStoreConfig {
    /// Database file path
    pub db_path: PathBuf,
    /// Number of days to retain events
    pub retention_days: u32,
    /// Whether to enable WAL mode
    pub wal_mode: bool,
}

impl Default for ObservabilityStoreConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from(":memory:"),
            retention_days: 30,
            wal_mode: true,
        }
    }
}

/// SQLite-backed observability store
pub struct ObservabilityStore {
    config: ObservabilityStoreConfig,
    conn: Arc<Mutex<Connection>>,
}

impl ObservabilityStore {
    /// Open or create an observability store at the given path
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
            config: ObservabilityStoreConfig {
                db_path,
                retention_days: 30,
                wal_mode: true,
            },
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory store (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;

        Ok(Self {
            config: ObservabilityStoreConfig::default(),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS obs_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Main events table with flexible JSON payload
            CREATE TABLE IF NOT EXISTS obs_events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                trace_id TEXT NOT NULL,
                span_id TEXT NOT NULL,
                parent_span_id TEXT,
                session_id TEXT,
                agent_id TEXT,
                payload TEXT NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- LLM calls table for efficient cost/token queries
            CREATE TABLE IF NOT EXISTS obs_llm_calls (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL REFERENCES obs_events(id),
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cache_read_tokens INTEGER DEFAULT 0,
                cache_write_tokens INTEGER DEFAULT 0,
                latency_ms INTEGER NOT NULL,
                cost_usd REAL NOT NULL,
                success INTEGER NOT NULL,
                session_id TEXT,
                agent_id TEXT,
                timestamp TEXT NOT NULL
            );

            -- Tool executions table
            CREATE TABLE IF NOT EXISTS obs_tool_executions (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL REFERENCES obs_events(id),
                tool_name TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                status TEXT NOT NULL,
                input_size_bytes INTEGER DEFAULT 0,
                output_size_bytes INTEGER DEFAULT 0,
                session_id TEXT,
                agent_id TEXT,
                timestamp TEXT NOT NULL
            );

            -- Agent lifecycle table
            CREATE TABLE IF NOT EXISTS obs_agent_lifecycle (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL REFERENCES obs_events(id),
                agent_id TEXT NOT NULL,
                agent_type TEXT NOT NULL,
                lifecycle_type TEXT NOT NULL,
                duration_ms INTEGER,
                turn_count INTEGER,
                session_id TEXT,
                timestamp TEXT NOT NULL
            );

            -- Indexes for efficient queries
            CREATE INDEX IF NOT EXISTS idx_obs_events_trace_id ON obs_events(trace_id);
            CREATE INDEX IF NOT EXISTS idx_obs_events_session_id ON obs_events(session_id);
            CREATE INDEX IF NOT EXISTS idx_obs_events_agent_id ON obs_events(agent_id);
            CREATE INDEX IF NOT EXISTS idx_obs_events_timestamp ON obs_events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_obs_events_type ON obs_events(event_type);
            CREATE INDEX IF NOT EXISTS idx_obs_events_created_at ON obs_events(created_at);

            CREATE INDEX IF NOT EXISTS idx_obs_llm_provider_model ON obs_llm_calls(provider, model);
            CREATE INDEX IF NOT EXISTS idx_obs_llm_session ON obs_llm_calls(session_id);
            CREATE INDEX IF NOT EXISTS idx_obs_llm_timestamp ON obs_llm_calls(timestamp);

            CREATE INDEX IF NOT EXISTS idx_obs_tool_name ON obs_tool_executions(tool_name);
            CREATE INDEX IF NOT EXISTS idx_obs_tool_session ON obs_tool_executions(session_id);

            CREATE INDEX IF NOT EXISTS idx_obs_agent_type ON obs_agent_lifecycle(agent_type);
            CREATE INDEX IF NOT EXISTS idx_obs_agent_session ON obs_agent_lifecycle(session_id);
            "#,
        )?;

        // Check and run migrations
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(CAST(value AS INTEGER), 0) FROM obs_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current_version < SCHEMA_VERSION {
            Self::run_migrations(conn, current_version)?;
            conn.execute(
                "INSERT OR REPLACE INTO obs_meta (key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }

        Ok(())
    }

    fn run_migrations(_conn: &Connection, _from_version: i32) -> Result<()> {
        // Add migrations here as needed
        Ok(())
    }

    // ========================================================================
    // Emit methods (write)
    // ========================================================================

    /// Emit an LLM call event
    pub fn emit_llm_call(&self, event: LlmCallEvent) -> Result<()> {
        let event_wrapper = Event::LlmCall(event.clone());
        self.emit_event(&event_wrapper)?;

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        conn.execute(
            r#"
            INSERT INTO obs_llm_calls
            (id, event_id, provider, model, input_tokens, output_tokens,
             cache_read_tokens, cache_write_tokens, latency_ms, cost_usd,
             success, session_id, agent_id, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
            params![
                event.id,
                event.id,
                event.provider,
                event.model,
                event.input_tokens,
                event.output_tokens,
                event.cache_read_tokens,
                event.cache_write_tokens,
                event.latency_ms,
                event.cost_usd,
                event.success as i32,
                event.session_id,
                event.agent_id,
                event.timestamp.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Emit a tool execution event
    pub fn emit_tool_execution(&self, event: ToolExecutionEvent) -> Result<()> {
        let event_wrapper = Event::ToolExecution(event.clone());
        self.emit_event(&event_wrapper)?;

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        conn.execute(
            r#"
            INSERT INTO obs_tool_executions
            (id, event_id, tool_name, duration_ms, status, input_size_bytes,
             output_size_bytes, session_id, agent_id, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                event.id,
                event.id,
                event.tool_name,
                event.duration_ms,
                event.status.to_string(),
                event.input_size_bytes,
                event.output_size_bytes,
                event.session_id,
                event.agent_id,
                event.timestamp.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Emit an agent lifecycle event
    pub fn emit_agent_lifecycle(&self, event: AgentLifecycleEvent) -> Result<()> {
        let event_wrapper = Event::AgentLifecycle(event.clone());
        self.emit_event(&event_wrapper)?;

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        conn.execute(
            r#"
            INSERT INTO obs_agent_lifecycle
            (id, event_id, agent_id, agent_type, lifecycle_type, duration_ms,
             turn_count, session_id, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                event.id,
                event.id,
                event.agent_id,
                event.agent_type,
                event.lifecycle_type.to_string(),
                event.duration_ms,
                event.turn_count,
                event.session_id,
                event.timestamp.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Emit a span event
    pub fn emit_span(&self, event: SpanEvent) -> Result<()> {
        let event_wrapper = Event::Span(event);
        self.emit_event(&event_wrapper)
    }

    /// Emit a generic event
    fn emit_event(&self, event: &Event) -> Result<()> {
        let payload = serde_json::to_string(event)?;
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        conn.execute(
            r#"
            INSERT INTO obs_events
            (id, event_type, timestamp, trace_id, span_id, parent_span_id,
             session_id, agent_id, payload)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                event.id(),
                event.event_type().to_string(),
                event.timestamp().to_rfc3339(),
                event.trace_id(),
                event.span_id(),
                match event {
                    Event::LlmCall(e) => e.parent_span_id.as_deref(),
                    Event::ToolExecution(e) => e.parent_span_id.as_deref(),
                    Event::AgentLifecycle(e) => e.parent_span_id.as_deref(),
                    Event::Span(e) => e.parent_span_id.as_deref(),
                },
                event.session_id(),
                event.agent_id(),
                payload,
            ],
        )?;

        Ok(())
    }

    /// Emit multiple events in a batch
    pub fn emit_batch(&self, events: &[Event]) -> Result<usize> {
        if events.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        conn.execute("BEGIN TRANSACTION", [])?;

        let mut inserted = 0;
        for event in events {
            let payload = serde_json::to_string(event)?;
            conn.execute(
                r#"
                INSERT INTO obs_events
                (id, event_type, timestamp, trace_id, span_id, parent_span_id,
                 session_id, agent_id, payload)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    event.id(),
                    event.event_type().to_string(),
                    event.timestamp().to_rfc3339(),
                    event.trace_id(),
                    event.span_id(),
                    match event {
                        Event::LlmCall(e) => e.parent_span_id.as_deref(),
                        Event::ToolExecution(e) => e.parent_span_id.as_deref(),
                        Event::AgentLifecycle(e) => e.parent_span_id.as_deref(),
                        Event::Span(e) => e.parent_span_id.as_deref(),
                    },
                    event.session_id(),
                    event.agent_id(),
                    payload,
                ],
            )?;
            inserted += 1;
        }

        conn.execute("COMMIT", [])?;
        Ok(inserted)
    }

    // ========================================================================
    // Query methods (read)
    // ========================================================================

    /// Query events by trace ID
    pub fn query_by_trace_id(&self, trace_id: &str) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT payload FROM obs_events WHERE trace_id = ?1 ORDER BY timestamp ASC",
        )?;

        let events: Vec<Event> = stmt
            .query_map(params![trace_id], |row| {
                let payload: String = row.get(0)?;
                Ok(serde_json::from_str(&payload).ok())
            })?
            .filter_map(|r| r.ok().flatten())
            .collect();

        Ok(events)
    }

    /// Query events by session ID
    pub fn query_by_session(&self, session_id: &str) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT payload FROM obs_events WHERE session_id = ?1 ORDER BY timestamp ASC",
        )?;

        let events: Vec<Event> = stmt
            .query_map(params![session_id], |row| {
                let payload: String = row.get(0)?;
                Ok(serde_json::from_str(&payload).ok())
            })?
            .filter_map(|r| r.ok().flatten())
            .collect();

        Ok(events)
    }

    /// Query events within a time range
    pub fn query_time_range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        event_type: Option<EventType>,
        limit: Option<usize>,
    ) -> Result<Vec<Event>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();

        let events = match (event_type, limit) {
            (Some(et), Some(lim)) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT payload FROM obs_events
                    WHERE timestamp >= ?1 AND timestamp <= ?2 AND event_type = ?3
                    ORDER BY timestamp DESC
                    LIMIT ?4
                    "#,
                )?;
                Self::collect_events(&mut stmt, params![from_str, to_str, et.to_string(), lim as i64])
            }
            (Some(et), None) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT payload FROM obs_events
                    WHERE timestamp >= ?1 AND timestamp <= ?2 AND event_type = ?3
                    ORDER BY timestamp DESC
                    "#,
                )?;
                Self::collect_events(&mut stmt, params![from_str, to_str, et.to_string()])
            }
            (None, Some(lim)) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT payload FROM obs_events
                    WHERE timestamp >= ?1 AND timestamp <= ?2
                    ORDER BY timestamp DESC
                    LIMIT ?3
                    "#,
                )?;
                Self::collect_events(&mut stmt, params![from_str, to_str, lim as i64])
            }
            (None, None) => {
                let mut stmt = conn.prepare(
                    r#"
                    SELECT payload FROM obs_events
                    WHERE timestamp >= ?1 AND timestamp <= ?2
                    ORDER BY timestamp DESC
                    "#,
                )?;
                Self::collect_events(&mut stmt, params![from_str, to_str])
            }
        };

        events
    }

    fn collect_events(
        stmt: &mut rusqlite::Statement,
        params: impl rusqlite::Params,
    ) -> Result<Vec<Event>> {
        let events: Vec<Event> = stmt
            .query_map(params, |row| {
                let payload: String = row.get(0)?;
                Ok(serde_json::from_str(&payload).ok())
            })?
            .filter_map(|r| r.ok().flatten())
            .collect();

        Ok(events)
    }

    // ========================================================================
    // Aggregation methods
    // ========================================================================

    /// Aggregate metrics for a time period
    pub fn aggregate_metrics(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<MetricsSummary> {
        let events = self.query_time_range(from, to, None, None)?;
        let mut aggregator = MetricsAggregator::new();
        Ok(aggregator.aggregate(&events))
    }

    /// Get total cost for a time period
    pub fn total_cost(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<f64> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();

        let cost: f64 = conn.query_row(
            r#"
            SELECT COALESCE(SUM(cost_usd), 0.0) FROM obs_llm_calls
            WHERE timestamp >= ?1 AND timestamp <= ?2
            "#,
            params![from_str, to_str],
            |row| row.get(0),
        )?;

        Ok(cost)
    }

    /// Get total tokens for a time period
    pub fn total_tokens(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<(u64, u64)> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();

        let (input, output): (i64, i64) = conn.query_row(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0)
            FROM obs_llm_calls
            WHERE timestamp >= ?1 AND timestamp <= ?2
            "#,
            params![from_str, to_str],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok((input as u64, output as u64))
    }

    /// Get cost breakdown by model
    pub fn cost_by_model(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<HashMap<String, f64>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();

        let mut stmt = conn.prepare(
            r#"
            SELECT provider || ':' || model, SUM(cost_usd)
            FROM obs_llm_calls
            WHERE timestamp >= ?1 AND timestamp <= ?2
            GROUP BY provider, model
            "#,
        )?;

        let costs: HashMap<String, f64> = stmt
            .query_map(params![from_str, to_str], |row| {
                let model: String = row.get(0)?;
                let cost: f64 = row.get(1)?;
                Ok((model, cost))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(costs)
    }

    /// Get cost breakdown by agent
    pub fn cost_by_agent(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<HashMap<String, f64>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let from_str = from.to_rfc3339();
        let to_str = to.to_rfc3339();

        let mut stmt = conn.prepare(
            r#"
            SELECT COALESCE(agent_id, 'unknown'), SUM(cost_usd)
            FROM obs_llm_calls
            WHERE timestamp >= ?1 AND timestamp <= ?2
            GROUP BY agent_id
            "#,
        )?;

        let costs: HashMap<String, f64> = stmt
            .query_map(params![from_str, to_str], |row| {
                let agent: String = row.get(0)?;
                let cost: f64 = row.get(1)?;
                Ok((agent, cost))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(costs)
    }

    // ========================================================================
    // Maintenance methods
    // ========================================================================

    /// Delete events older than retention period
    pub fn cleanup(&self) -> Result<usize> {
        let cutoff = Utc::now() - Duration::days(self.config.retention_days as i64);
        let cutoff_secs = cutoff.timestamp();

        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        // Delete from child tables first (referential integrity)
        conn.execute(
            "DELETE FROM obs_llm_calls WHERE timestamp < datetime(?1, 'unixepoch')",
            params![cutoff_secs],
        )?;
        conn.execute(
            "DELETE FROM obs_tool_executions WHERE timestamp < datetime(?1, 'unixepoch')",
            params![cutoff_secs],
        )?;
        conn.execute(
            "DELETE FROM obs_agent_lifecycle WHERE timestamp < datetime(?1, 'unixepoch')",
            params![cutoff_secs],
        )?;

        let deleted = conn.execute(
            "DELETE FROM obs_events WHERE created_at < ?1",
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

    /// Get store statistics
    pub fn stats(&self) -> Result<ObservabilityStoreStats> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let total_events: i64 = conn.query_row(
            "SELECT COUNT(*) FROM obs_events",
            [],
            |row| row.get(0),
        )?;

        let llm_calls: i64 = conn.query_row(
            "SELECT COUNT(*) FROM obs_llm_calls",
            [],
            |row| row.get(0),
        )?;

        let tool_executions: i64 = conn.query_row(
            "SELECT COUNT(*) FROM obs_tool_executions",
            [],
            |row| row.get(0),
        )?;

        let agent_events: i64 = conn.query_row(
            "SELECT COUNT(*) FROM obs_agent_lifecycle",
            [],
            |row| row.get(0),
        )?;

        let total_cost: f64 = conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0) FROM obs_llm_calls",
            [],
            |row| row.get(0),
        )?;

        let (total_input_tokens, total_output_tokens): (i64, i64) = conn.query_row(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0)
            FROM obs_llm_calls
            "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let oldest_ts: Option<String> = conn
            .query_row("SELECT MIN(timestamp) FROM obs_events", [], |row| row.get(0))
            .ok();

        let newest_ts: Option<String> = conn
            .query_row("SELECT MAX(timestamp) FROM obs_events", [], |row| row.get(0))
            .ok();

        Ok(ObservabilityStoreStats {
            total_events: total_events as u64,
            llm_calls: llm_calls as u64,
            tool_executions: tool_executions as u64,
            agent_events: agent_events as u64,
            total_cost_usd: total_cost,
            total_input_tokens: total_input_tokens as u64,
            total_output_tokens: total_output_tokens as u64,
            oldest_ts,
            newest_ts,
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
}

// Implement Send + Sync for ObservabilityStore
unsafe impl Send for ObservabilityStore {}
unsafe impl Sync for ObservabilityStore {}

/// Statistics about the observability store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityStoreStats {
    pub total_events: u64,
    pub llm_calls: u64,
    pub tool_executions: u64,
    pub agent_events: u64,
    pub total_cost_usd: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub oldest_ts: Option<String>,
    pub newest_ts: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::event::ToolStatus;

    #[test]
    fn test_emit_llm_call() {
        let store = ObservabilityStore::in_memory().unwrap();

        let event = LlmCallEvent {
            provider: "anthropic".into(),
            model: "claude-opus-4-5".into(),
            input_tokens: 1000,
            output_tokens: 500,
            latency_ms: 2000,
            cost_usd: 0.05,
            ..Default::default()
        };

        store.emit_llm_call(event).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.llm_calls, 1);
        assert_eq!(stats.total_cost_usd, 0.05);
    }

    #[test]
    fn test_emit_tool_execution() {
        let store = ObservabilityStore::in_memory().unwrap();

        let event = ToolExecutionEvent {
            tool_name: "Read".into(),
            duration_ms: 50,
            status: ToolStatus::Success,
            ..Default::default()
        };

        store.emit_tool_execution(event).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.tool_executions, 1);
    }

    #[test]
    fn test_query_by_trace_id() {
        let store = ObservabilityStore::in_memory().unwrap();
        let trace_id = "trace-123";

        let event1 = LlmCallEvent {
            trace_id: trace_id.into(),
            provider: "anthropic".into(),
            model: "claude-opus-4-5".into(),
            ..Default::default()
        };

        let event2 = ToolExecutionEvent {
            trace_id: trace_id.into(),
            tool_name: "Read".into(),
            ..Default::default()
        };

        store.emit_llm_call(event1).unwrap();
        store.emit_tool_execution(event2).unwrap();

        let events = store.query_by_trace_id(trace_id).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_aggregate_metrics() {
        let store = ObservabilityStore::in_memory().unwrap();

        store.emit_llm_call(LlmCallEvent {
            provider: "anthropic".into(),
            model: "claude-opus-4-5".into(),
            input_tokens: 1000,
            output_tokens: 500,
            cost_usd: 0.05,
            ..Default::default()
        }).unwrap();

        store.emit_llm_call(LlmCallEvent {
            provider: "anthropic".into(),
            model: "claude-opus-4-5".into(),
            input_tokens: 500,
            output_tokens: 200,
            cost_usd: 0.02,
            ..Default::default()
        }).unwrap();

        let from = Utc::now() - Duration::hours(1);
        let to = Utc::now() + Duration::hours(1);
        let metrics = store.aggregate_metrics(from, to).unwrap();

        assert_eq!(metrics.llm.total_calls, 2);
        assert_eq!(metrics.llm.total_cost_usd, 0.07);
    }

    #[test]
    fn test_cost_by_model() {
        let store = ObservabilityStore::in_memory().unwrap();

        store.emit_llm_call(LlmCallEvent {
            provider: "anthropic".into(),
            model: "claude-opus-4-5".into(),
            cost_usd: 0.05,
            ..Default::default()
        }).unwrap();

        store.emit_llm_call(LlmCallEvent {
            provider: "openai".into(),
            model: "gpt-4".into(),
            cost_usd: 0.03,
            ..Default::default()
        }).unwrap();

        let from = Utc::now() - Duration::hours(1);
        let to = Utc::now() + Duration::hours(1);
        let costs = store.cost_by_model(from, to).unwrap();

        assert_eq!(costs.len(), 2);
        assert_eq!(costs.get("anthropic:claude-opus-4-5"), Some(&0.05));
        assert_eq!(costs.get("openai:gpt-4"), Some(&0.03));
    }
}
