//! NAPI bindings for trace module
//!
//! Exposes TraceStore functionality to Node.js/TypeScript.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;

use crate::trace::{
    aggregate_errors, compare_periods, error_rates_by_service, generate_comparison_report,
    generate_detailed_report, profile_traces, recent_errors, GroupBy, TraceEntry, TraceFilter,
    TraceQuery, TraceStore, TraceStoreStats,
};

// ============================================================================
// NAPI Data Transfer Objects
// ============================================================================

/// Trace entry for NAPI
#[napi(object)]
pub struct NapiTraceEntry {
    pub ts: String,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service: String,
    pub event_type: String,
    pub level: String,
    pub payload: String, // JSON string
}

impl From<TraceEntry> for NapiTraceEntry {
    fn from(e: TraceEntry) -> Self {
        Self {
            ts: e.ts,
            trace_id: e.trace_id,
            span_id: e.span_id,
            parent_span_id: e.parent_span_id,
            service: e.service,
            event_type: e.event_type,
            level: e.level,
            payload: e.payload.to_string(),
        }
    }
}

impl TryFrom<NapiTraceEntry> for TraceEntry {
    type Error = napi::Error;

    fn try_from(e: NapiTraceEntry) -> Result<Self> {
        let payload: serde_json::Value = serde_json::from_str(&e.payload)
            .map_err(|e| Error::from_reason(format!("Invalid payload JSON: {}", e)))?;

        Ok(Self {
            ts: e.ts,
            trace_id: e.trace_id,
            span_id: e.span_id,
            parent_span_id: e.parent_span_id,
            service: e.service,
            event_type: e.event_type,
            level: e.level,
            payload,
        })
    }
}

/// Trace filter for NAPI
#[napi(object)]
pub struct NapiTraceFilter {
    pub trace_id: Option<String>,
    pub service: Option<String>,
    pub event_type: Option<String>,
    pub level: Option<String>,
    pub from_ts: Option<String>,
    pub to_ts: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl From<NapiTraceFilter> for TraceFilter {
    fn from(f: NapiTraceFilter) -> Self {
        Self {
            trace_id: f.trace_id,
            service: f.service,
            event_type: f.event_type,
            level: f.level,
            from_ts: f.from_ts,
            to_ts: f.to_ts,
            limit: f.limit.map(|v| v as usize),
            offset: f.offset.map(|v| v as usize),
        }
    }
}

/// Store statistics for NAPI
#[napi(object)]
pub struct NapiTraceStoreStats {
    pub total_entries: u32,
    pub total_size_bytes: u32,
    pub oldest_ts: Option<String>,
    pub newest_ts: Option<String>,
    pub by_service: HashMap<String, u32>,
    pub by_event_type: HashMap<String, u32>,
}

impl From<TraceStoreStats> for NapiTraceStoreStats {
    fn from(s: TraceStoreStats) -> Self {
        Self {
            total_entries: s.total_entries as u32,
            total_size_bytes: s.total_size_bytes as u32,
            oldest_ts: s.oldest_ts,
            newest_ts: s.newest_ts,
            by_service: s.by_service.into_iter().map(|(k, v)| (k, v as u32)).collect(),
            by_event_type: s.by_event_type.into_iter().map(|(k, v)| (k, v as u32)).collect(),
        }
    }
}

/// Slow operation for NAPI
#[napi(object)]
pub struct NapiSlowOperation {
    pub function: String,
    pub service: String,
    pub duration_ms: f64,
    pub trace_id: String,
    pub timestamp: String,
}

/// Service stats for NAPI
#[napi(object)]
pub struct NapiServiceStats {
    pub service: String,
    pub event_count: u32,
    pub error_count: u32,
    pub avg_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
}

/// Function stats for NAPI
#[napi(object)]
pub struct NapiFunctionStats {
    pub function: String,
    pub call_count: u32,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub min_duration_ms: f64,
}

/// Profile result for NAPI
#[napi(object)]
pub struct NapiProfileResult {
    pub total_traces: u32,
    pub total_events: u32,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub min_duration_ms: f64,
    pub slowest: Vec<NapiSlowOperation>,
    pub by_service: Vec<NapiServiceStats>,
    pub by_function: Vec<NapiFunctionStats>,
}

/// Error sample for NAPI
#[napi(object)]
pub struct NapiErrorSample {
    pub error: String,
    pub timestamp: String,
    pub trace_id: String,
}

/// Error group for NAPI
#[napi(object)]
pub struct NapiErrorGroup {
    pub key: String,
    pub count: u32,
    pub samples: Vec<NapiErrorSample>,
}

/// Error summary for NAPI
#[napi(object)]
pub struct NapiErrorSummary {
    pub total: u32,
    pub groups: Vec<NapiErrorGroup>,
}

/// Period summary for NAPI
#[napi(object)]
pub struct NapiPeriodSummary {
    pub period1_traces: u32,
    pub period2_traces: u32,
    pub period1_events: u32,
    pub period2_events: u32,
    pub period1_avg_ms: f64,
    pub period2_avg_ms: f64,
    pub period1_max_ms: f64,
    pub period2_max_ms: f64,
}

/// Service change for NAPI
#[napi(object)]
pub struct NapiServiceChange {
    pub service: String,
    pub period1_avg_ms: f64,
    pub period2_avg_ms: f64,
    pub change_pct: f64,
    pub period1_errors: u32,
    pub period2_errors: u32,
}

/// Period comparison result for NAPI
#[napi(object)]
pub struct NapiPeriodComparison {
    pub period1_label: String,
    pub period2_label: String,
    pub summary: NapiPeriodSummary,
    pub service_changes: Vec<NapiServiceChange>,
    pub regressions: Vec<NapiServiceChange>,
    pub improvements: Vec<NapiServiceChange>,
}

// ============================================================================
// TraceStore Handle
// ============================================================================

/// Handle to a TraceStore
#[napi]
pub struct TraceStoreHandle {
    inner: Arc<TraceStore>,
}

/// Open or create a trace store at the given path
#[napi]
pub fn open_trace_store(db_path: String) -> Result<TraceStoreHandle> {
    let store = TraceStore::open(&db_path).map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(TraceStoreHandle {
        inner: Arc::new(store),
    })
}

/// Create an in-memory trace store (for testing)
#[napi]
pub fn create_memory_trace_store() -> Result<TraceStoreHandle> {
    let store = TraceStore::in_memory().map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(TraceStoreHandle {
        inner: Arc::new(store),
    })
}

#[napi]
impl TraceStoreHandle {
    /// Append a single trace entry
    #[napi]
    pub fn append(&self, entry: NapiTraceEntry) -> Result<()> {
        let entry: TraceEntry = entry.try_into()?;
        self.inner
            .append(&entry)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Append multiple trace entries in batch
    #[napi]
    pub fn append_batch(&self, entries: Vec<NapiTraceEntry>) -> Result<u32> {
        let entries: Result<Vec<TraceEntry>> = entries.into_iter().map(|e| e.try_into()).collect();
        let entries = entries?;

        self.inner
            .append_batch(&entries)
            .map(|c| c as u32)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Query traces by trace_id
    #[napi]
    pub fn query_by_trace_id(&self, trace_id: String) -> Result<Vec<NapiTraceEntry>> {
        self.inner
            .query_by_trace_id(&trace_id)
            .map(|entries| entries.into_iter().map(Into::into).collect())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Query traces by service
    #[napi]
    pub fn query_by_service(
        &self,
        service: String,
        from_ts: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<NapiTraceEntry>> {
        self.inner
            .query_by_service(&service, from_ts.as_deref(), limit.map(|l| l as usize))
            .map(|entries| entries.into_iter().map(Into::into).collect())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Query traces with flexible filter
    #[napi]
    pub fn query(&self, filter: NapiTraceFilter) -> Result<Vec<NapiTraceEntry>> {
        let filter: TraceFilter = filter.into();
        TraceQuery::query(&self.inner, &filter)
            .map(|entries| entries.into_iter().map(Into::into).collect())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Count traces matching filter
    #[napi]
    pub fn count(&self, filter: NapiTraceFilter) -> Result<u32> {
        let filter: TraceFilter = filter.into();
        TraceQuery::count(&self.inner, &filter)
            .map(|c| c as u32)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get distinct trace IDs
    #[napi]
    pub fn get_trace_ids(&self, from_ts: String, limit: Option<u32>) -> Result<Vec<String>> {
        self.inner
            .get_trace_ids(&from_ts, limit.map(|l| l as usize))
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get distinct services
    #[napi]
    pub fn get_services(&self) -> Result<Vec<String>> {
        self.inner
            .get_services()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Profile traces for performance analysis
    #[napi]
    pub fn profile(&self, from_ts: String, top_n: u32) -> Result<NapiProfileResult> {
        let result = profile_traces(&self.inner, &from_ts, top_n as usize)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(NapiProfileResult {
            total_traces: result.total_traces as u32,
            total_events: result.total_events as u32,
            avg_duration_ms: result.avg_duration_ms,
            max_duration_ms: result.max_duration_ms,
            min_duration_ms: result.min_duration_ms,
            slowest: result
                .slowest
                .into_iter()
                .map(|op| NapiSlowOperation {
                    function: op.function,
                    service: op.service,
                    duration_ms: op.duration_ms,
                    trace_id: op.trace_id,
                    timestamp: op.timestamp,
                })
                .collect(),
            by_service: result
                .by_service
                .into_values()
                .map(|s| NapiServiceStats {
                    service: s.service,
                    event_count: s.event_count as u32,
                    error_count: s.error_count as u32,
                    avg_duration_ms: s.avg_duration_ms,
                    p50_duration_ms: s.p50_duration_ms,
                    p95_duration_ms: s.p95_duration_ms,
                    p99_duration_ms: s.p99_duration_ms,
                })
                .collect(),
            by_function: result
                .by_function
                .into_values()
                .map(|f| NapiFunctionStats {
                    function: f.function,
                    call_count: f.call_count as u32,
                    avg_duration_ms: f.avg_duration_ms,
                    max_duration_ms: f.max_duration_ms,
                    min_duration_ms: f.min_duration_ms,
                })
                .collect(),
        })
    }

    /// Generate a detailed performance report as text
    #[napi]
    pub fn generate_report(&self, from_ts: String, top_n: u32) -> Result<String> {
        generate_detailed_report(&self.inner, &from_ts, top_n as usize)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Compare performance between two time periods
    #[napi]
    pub fn compare_periods(
        &self,
        period1_start: String,
        period1_end: String,
        period2_start: String,
        period2_end: String,
    ) -> Result<NapiPeriodComparison> {
        let result = compare_periods(
            &self.inner,
            &period1_start,
            &period1_end,
            &period2_start,
            &period2_end,
        )
        .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(NapiPeriodComparison {
            period1_label: result.period1_label,
            period2_label: result.period2_label,
            summary: NapiPeriodSummary {
                period1_traces: result.summary.period1_traces as u32,
                period2_traces: result.summary.period2_traces as u32,
                period1_events: result.summary.period1_events as u32,
                period2_events: result.summary.period2_events as u32,
                period1_avg_ms: result.summary.period1_avg_ms,
                period2_avg_ms: result.summary.period2_avg_ms,
                period1_max_ms: result.summary.period1_max_ms,
                period2_max_ms: result.summary.period2_max_ms,
            },
            service_changes: result
                .service_changes
                .into_iter()
                .map(|sc| NapiServiceChange {
                    service: sc.service,
                    period1_avg_ms: sc.period1_avg_ms,
                    period2_avg_ms: sc.period2_avg_ms,
                    change_pct: sc.change_pct,
                    period1_errors: sc.period1_errors as u32,
                    period2_errors: sc.period2_errors as u32,
                })
                .collect(),
            regressions: result
                .regressions
                .into_iter()
                .map(|sc| NapiServiceChange {
                    service: sc.service,
                    period1_avg_ms: sc.period1_avg_ms,
                    period2_avg_ms: sc.period2_avg_ms,
                    change_pct: sc.change_pct,
                    period1_errors: sc.period1_errors as u32,
                    period2_errors: sc.period2_errors as u32,
                })
                .collect(),
            improvements: result
                .improvements
                .into_iter()
                .map(|sc| NapiServiceChange {
                    service: sc.service,
                    period1_avg_ms: sc.period1_avg_ms,
                    period2_avg_ms: sc.period2_avg_ms,
                    change_pct: sc.change_pct,
                    period1_errors: sc.period1_errors as u32,
                    period2_errors: sc.period2_errors as u32,
                })
                .collect(),
        })
    }

    /// Generate a text comparison report
    #[napi]
    pub fn generate_comparison_report(
        &self,
        period1_start: String,
        period1_end: String,
        period2_start: String,
        period2_end: String,
    ) -> Result<String> {
        generate_comparison_report(
            &self.inner,
            &period1_start,
            &period1_end,
            &period2_start,
            &period2_end,
        )
        .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Aggregate errors by grouping
    #[napi]
    pub fn aggregate_errors(&self, from_ts: String, group_by: String) -> Result<NapiErrorSummary> {
        let group_by: GroupBy = group_by
            .parse()
            .map_err(|e: anyhow::Error| Error::from_reason(e.to_string()))?;

        let summary = aggregate_errors(&self.inner, &from_ts, group_by)
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(NapiErrorSummary {
            total: summary.total as u32,
            groups: summary
                .groups
                .into_iter()
                .map(|g| NapiErrorGroup {
                    key: g.key,
                    count: g.count as u32,
                    samples: g
                        .samples
                        .into_iter()
                        .map(|s| NapiErrorSample {
                            error: s.error,
                            timestamp: s.timestamp,
                            trace_id: s.trace_id,
                        })
                        .collect(),
                })
                .collect(),
        })
    }

    /// Get error rates by service
    #[napi]
    pub fn error_rates(&self, from_ts: String) -> Result<HashMap<String, f64>> {
        error_rates_by_service(&self.inner, &from_ts)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get recent errors
    #[napi]
    pub fn recent_errors(&self, limit: u32) -> Result<Vec<NapiErrorSample>> {
        recent_errors(&self.inner, limit as usize)
            .map(|samples| {
                samples
                    .into_iter()
                    .map(|s| NapiErrorSample {
                        error: s.error,
                        timestamp: s.timestamp,
                        trace_id: s.trace_id,
                    })
                    .collect()
            })
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Delete traces older than retention days
    #[napi]
    pub fn cleanup(&self, retention_days: u32) -> Result<u32> {
        self.inner
            .cleanup(retention_days)
            .map(|c| c as u32)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Compact the database
    #[napi]
    pub fn compact(&self) -> Result<()> {
        self.inner
            .compact()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get store statistics
    #[napi]
    pub fn stats(&self) -> Result<NapiTraceStoreStats> {
        self.inner
            .stats()
            .map(Into::into)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Health check
    #[napi]
    pub fn health_check(&self) -> bool {
        self.inner.health_check()
    }

    /// Get database path
    #[napi]
    pub fn path(&self) -> String {
        self.inner.path().to_string_lossy().to_string()
    }
}
