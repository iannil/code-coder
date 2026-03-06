//! Error aggregation for trace data
//!
//! Groups and analyzes error events by service, function, or error message

use crate::trace::storage::TraceStore;
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// How to group errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupBy {
    Service,
    Function,
    Error,
}

impl Default for GroupBy {
    fn default() -> Self {
        Self::Service
    }
}

impl std::str::FromStr for GroupBy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "service" => Ok(Self::Service),
            "function" => Ok(Self::Function),
            "error" => Ok(Self::Error),
            _ => Err(anyhow::anyhow!("Invalid group_by value: {}", s)),
        }
    }
}

/// A sample error instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSample {
    pub error: String,
    pub timestamp: String,
    pub trace_id: String,
}

/// A group of errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorGroup {
    pub key: String,
    pub count: usize,
    pub samples: Vec<ErrorSample>,
}

/// Summary of all errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSummary {
    pub total: usize,
    pub groups: Vec<ErrorGroup>,
}

/// Aggregate errors from trace data
pub fn aggregate_errors(
    store: &TraceStore,
    from_ts: &str,
    group_by: GroupBy,
) -> Result<ErrorSummary> {
    let conn = store.connection();
    let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    // Query all error events
    let mut stmt = conn.prepare(
        r#"
        SELECT ts, trace_id, service, payload
        FROM traces
        WHERE event_type = 'error' AND ts >= ?1
        ORDER BY ts DESC
        "#,
    )?;

    let mut groups: HashMap<String, ErrorGroup> = HashMap::new();
    let mut total = 0;

    let rows = stmt.query_map(params![from_ts], |row| {
        let ts: String = row.get(0)?;
        let trace_id: String = row.get(1)?;
        let service: String = row.get(2)?;
        let payload_str: String = row.get(3)?;
        Ok((ts, trace_id, service, payload_str))
    })?;

    for row_result in rows {
        let (ts, trace_id, service, payload_str) = row_result?;
        total += 1;

        let payload: serde_json::Value =
            serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);

        let function = payload
            .get("function")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let error = payload
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let key = match group_by {
            GroupBy::Service => service.clone(),
            GroupBy::Function => function.to_string(),
            GroupBy::Error => error.to_string(),
        };

        let group = groups.entry(key.clone()).or_insert(ErrorGroup {
            key: key.clone(),
            count: 0,
            samples: Vec::new(),
        });

        group.count += 1;

        // Keep up to 5 samples per group
        if group.samples.len() < 5 {
            group.samples.push(ErrorSample {
                error: error.to_string(),
                timestamp: ts,
                trace_id,
            });
        }
    }

    // Sort groups by count descending
    let mut sorted_groups: Vec<ErrorGroup> = groups.into_values().collect();
    sorted_groups.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(ErrorSummary {
        total,
        groups: sorted_groups,
    })
}

/// Get error rate by service
pub fn error_rates_by_service(store: &TraceStore, from_ts: &str) -> Result<HashMap<String, f64>> {
    let conn = store.connection();
    let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    // Get total events per service
    let mut stmt = conn.prepare(
        r#"
        SELECT service, COUNT(*) as total,
               SUM(CASE WHEN event_type = 'error' THEN 1 ELSE 0 END) as errors
        FROM traces
        WHERE ts >= ?1
        GROUP BY service
        "#,
    )?;

    let mut rates = HashMap::new();

    let rows = stmt.query_map(params![from_ts], |row| {
        let service: String = row.get(0)?;
        let total: i64 = row.get(1)?;
        let errors: i64 = row.get(2)?;
        Ok((service, total, errors))
    })?;

    for row_result in rows {
        let (service, total, errors) = row_result?;
        let rate = if total > 0 {
            (errors as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        rates.insert(service, rate);
    }

    Ok(rates)
}

/// Get recent errors (last N)
pub fn recent_errors(store: &TraceStore, limit: usize) -> Result<Vec<ErrorSample>> {
    let conn = store.connection();
    let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    let mut stmt = conn.prepare(
        r#"
        SELECT ts, trace_id, payload
        FROM traces
        WHERE event_type = 'error'
        ORDER BY ts DESC
        LIMIT ?1
        "#,
    )?;

    let samples: Vec<ErrorSample> = stmt
        .query_map(params![limit as i64], |row| {
            let ts: String = row.get(0)?;
            let trace_id: String = row.get(1)?;
            let payload_str: String = row.get(2)?;
            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
            let error = payload
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            Ok(ErrorSample {
                error,
                timestamp: ts,
                trace_id,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::TraceEntry;

    fn create_error_entry(trace_id: &str, service: &str, error: &str) -> TraceEntry {
        TraceEntry {
            ts: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            span_id: "abcd1234".to_string(),
            parent_span_id: None,
            service: service.to_string(),
            event_type: "error".to_string(),
            level: "error".to_string(),
            payload: serde_json::json!({
                "function": "handleRequest",
                "error": error
            }),
        }
    }

    fn create_success_entry(trace_id: &str, service: &str) -> TraceEntry {
        TraceEntry {
            ts: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            span_id: "abcd1234".to_string(),
            parent_span_id: None,
            service: service.to_string(),
            event_type: "function_end".to_string(),
            level: "info".to_string(),
            payload: serde_json::json!({"function": "handleRequest"}),
        }
    }

    #[test]
    fn test_aggregate_errors_by_service() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_error_entry("t1", "ccode-api", "timeout")).unwrap();
        store.append(&create_error_entry("t2", "ccode-api", "invalid input")).unwrap();
        store.append(&create_error_entry("t3", "zero-channels", "connection refused")).unwrap();

        let summary = aggregate_errors(&store, "2000-01-01T00:00:00Z", GroupBy::Service).unwrap();

        assert_eq!(summary.total, 3);
        assert_eq!(summary.groups.len(), 2);
        assert_eq!(summary.groups[0].key, "ccode-api");
        assert_eq!(summary.groups[0].count, 2);
    }

    #[test]
    fn test_aggregate_errors_by_error() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_error_entry("t1", "svc1", "timeout")).unwrap();
        store.append(&create_error_entry("t2", "svc2", "timeout")).unwrap();
        store.append(&create_error_entry("t3", "svc3", "connection refused")).unwrap();

        let summary = aggregate_errors(&store, "2000-01-01T00:00:00Z", GroupBy::Error).unwrap();

        assert_eq!(summary.total, 3);
        assert_eq!(summary.groups.len(), 2);
        assert_eq!(summary.groups[0].key, "timeout");
        assert_eq!(summary.groups[0].count, 2);
    }

    #[test]
    fn test_error_rates_by_service() {
        let store = TraceStore::in_memory().unwrap();

        // Service A: 2 errors out of 4 = 50%
        store.append(&create_error_entry("t1", "svc-a", "err")).unwrap();
        store.append(&create_error_entry("t2", "svc-a", "err")).unwrap();
        store.append(&create_success_entry("t3", "svc-a")).unwrap();
        store.append(&create_success_entry("t4", "svc-a")).unwrap();

        // Service B: 0 errors out of 2 = 0%
        store.append(&create_success_entry("t5", "svc-b")).unwrap();
        store.append(&create_success_entry("t6", "svc-b")).unwrap();

        let rates = error_rates_by_service(&store, "2000-01-01T00:00:00Z").unwrap();

        assert_eq!(rates["svc-a"], 50.0);
        assert_eq!(rates["svc-b"], 0.0);
    }

    #[test]
    fn test_recent_errors() {
        let store = TraceStore::in_memory().unwrap();

        store.append(&create_error_entry("t1", "svc", "error 1")).unwrap();
        store.append(&create_error_entry("t2", "svc", "error 2")).unwrap();
        store.append(&create_error_entry("t3", "svc", "error 3")).unwrap();

        let errors = recent_errors(&store, 2).unwrap();
        assert_eq!(errors.len(), 2);
    }
}
