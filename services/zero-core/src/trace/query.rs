//! Trace query engine
//!
//! Provides flexible querying of trace data with filters

use crate::trace::storage::{TraceEntry, TraceStore};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Filter criteria for trace queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TraceFilter {
    /// Filter by trace_id
    pub trace_id: Option<String>,
    /// Filter by service name
    pub service: Option<String>,
    /// Filter by event type
    pub event_type: Option<String>,
    /// Filter by log level
    pub level: Option<String>,
    /// Filter from timestamp (inclusive)
    pub from_ts: Option<String>,
    /// Filter to timestamp (inclusive)
    pub to_ts: Option<String>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl TraceFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = Some(level.into());
        self
    }

    pub fn from_ts(mut self, ts: impl Into<String>) -> Self {
        self.from_ts = Some(ts.into());
        self
    }

    pub fn to_ts(mut self, ts: impl Into<String>) -> Self {
        self.to_ts = Some(ts.into());
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Query engine for trace data
pub struct TraceQuery;

impl TraceQuery {
    /// Query traces with flexible filters
    pub fn query(store: &TraceStore, filter: &TraceFilter) -> Result<Vec<TraceEntry>> {
        let conn = store.connection();
        let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        // Build WHERE clause dynamically
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref trace_id) = filter.trace_id {
            conditions.push("trace_id = ?");
            param_values.push(Box::new(trace_id.clone()));
        }
        if let Some(ref service) = filter.service {
            conditions.push("service = ?");
            param_values.push(Box::new(service.clone()));
        }
        if let Some(ref event_type) = filter.event_type {
            conditions.push("event_type = ?");
            param_values.push(Box::new(event_type.clone()));
        }
        if let Some(ref level) = filter.level {
            conditions.push("level = ?");
            param_values.push(Box::new(level.clone()));
        }
        if let Some(ref from_ts) = filter.from_ts {
            conditions.push("ts >= ?");
            param_values.push(Box::new(from_ts.clone()));
        }
        if let Some(ref to_ts) = filter.to_ts {
            conditions.push("ts <= ?");
            param_values.push(Box::new(to_ts.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit_clause = match (filter.limit, filter.offset) {
            (Some(lim), Some(off)) => format!("LIMIT {} OFFSET {}", lim, off),
            (Some(lim), None) => format!("LIMIT {}", lim),
            (None, Some(off)) => format!("LIMIT -1 OFFSET {}", off),
            (None, None) => String::new(),
        };

        let sql = format!(
            r#"
            SELECT ts, trace_id, span_id, parent_span_id, service, event_type, level, payload
            FROM traces
            {}
            ORDER BY ts DESC
            {}
            "#,
            where_clause, limit_clause
        );

        let mut stmt = conn.prepare(&sql)?;

        // Convert params to references
        let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let entries = stmt
            .query_map(params_refs.as_slice(), |row| {
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

    /// Count traces matching filter
    pub fn count(store: &TraceStore, filter: &TraceFilter) -> Result<usize> {
        let conn = store.connection();
        let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        // Build WHERE clause dynamically
        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref trace_id) = filter.trace_id {
            conditions.push("trace_id = ?");
            param_values.push(Box::new(trace_id.clone()));
        }
        if let Some(ref service) = filter.service {
            conditions.push("service = ?");
            param_values.push(Box::new(service.clone()));
        }
        if let Some(ref event_type) = filter.event_type {
            conditions.push("event_type = ?");
            param_values.push(Box::new(event_type.clone()));
        }
        if let Some(ref level) = filter.level {
            conditions.push("level = ?");
            param_values.push(Box::new(level.clone()));
        }
        if let Some(ref from_ts) = filter.from_ts {
            conditions.push("ts >= ?");
            param_values.push(Box::new(from_ts.clone()));
        }
        if let Some(ref to_ts) = filter.to_ts {
            conditions.push("ts <= ?");
            param_values.push(Box::new(to_ts.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT COUNT(*) FROM traces {}", where_clause);

        // Convert params to references
        let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))?;

        Ok(count as usize)
    }

    /// Get distinct trace IDs matching filter
    pub fn get_trace_ids(store: &TraceStore, filter: &TraceFilter) -> Result<Vec<String>> {
        let conn = store.connection();
        let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref service) = filter.service {
            conditions.push("service = ?");
            param_values.push(Box::new(service.clone()));
        }
        if let Some(ref from_ts) = filter.from_ts {
            conditions.push("ts >= ?");
            param_values.push(Box::new(from_ts.clone()));
        }
        if let Some(ref to_ts) = filter.to_ts {
            conditions.push("ts <= ?");
            param_values.push(Box::new(to_ts.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit_clause = filter
            .limit
            .map(|l| format!("LIMIT {}", l))
            .unwrap_or_default();

        let sql = format!(
            "SELECT DISTINCT trace_id FROM traces {} ORDER BY ts DESC {}",
            where_clause, limit_clause
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let ids: Vec<String> = stmt
            .query_map(params_refs.as_slice(), |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::TraceEntry;

    fn create_test_entry(trace_id: &str, service: &str, level: &str) -> TraceEntry {
        TraceEntry {
            ts: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            span_id: "abcd1234".to_string(),
            parent_span_id: None,
            service: service.to_string(),
            event_type: "function_end".to_string(),
            level: level.to_string(),
            payload: serde_json::json!({}),
        }
    }

    #[test]
    fn test_query_with_filter() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_test_entry("t1", "ccode-api", "info")).unwrap();
        store.append(&create_test_entry("t2", "ccode-api", "error")).unwrap();
        store.append(&create_test_entry("t3", "zero-channels", "info")).unwrap();

        // Query by service
        let filter = TraceFilter::new().service("ccode-api");
        let results = TraceQuery::query(&store, &filter).unwrap();
        assert_eq!(results.len(), 2);

        // Query by level
        let filter = TraceFilter::new().level("error");
        let results = TraceQuery::query(&store, &filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].trace_id, "t2");
    }

    #[test]
    fn test_query_with_limit() {
        let store = TraceStore::in_memory().unwrap();

        for i in 0..10 {
            store.append(&create_test_entry(&format!("t{}", i), "svc", "info")).unwrap();
        }

        let filter = TraceFilter::new().limit(5);
        let results = TraceQuery::query(&store, &filter).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_count_with_filter() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_test_entry("t1", "ccode-api", "info")).unwrap();
        store.append(&create_test_entry("t2", "ccode-api", "error")).unwrap();
        store.append(&create_test_entry("t3", "zero-channels", "info")).unwrap();

        let filter = TraceFilter::new().service("ccode-api");
        let count = TraceQuery::count(&store, &filter).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_trace_ids() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_test_entry("t1", "ccode-api", "info")).unwrap();
        store.append(&create_test_entry("t1", "ccode-api", "info")).unwrap();
        store.append(&create_test_entry("t2", "ccode-api", "info")).unwrap();

        let filter = TraceFilter::new();
        let ids = TraceQuery::get_trace_ids(&store, &filter).unwrap();
        assert_eq!(ids.len(), 2);
    }
}
