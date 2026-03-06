//! Performance profiling for trace data
//!
//! Provides statistical analysis of trace entries:
//! - Percentile calculations (p50, p95, p99)
//! - Service-level statistics
//! - Function-level statistics
//! - Slowest operation tracking

use crate::trace::storage::TraceStore;
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A slow operation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowOperation {
    pub function: String,
    pub service: String,
    pub duration_ms: f64,
    pub trace_id: String,
    pub timestamp: String,
}

/// Statistics for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    pub service: String,
    pub event_count: usize,
    pub error_count: usize,
    pub avg_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
}

/// Statistics for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionStats {
    pub function: String,
    pub call_count: usize,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub min_duration_ms: f64,
}

/// Complete profile result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResult {
    pub total_traces: usize,
    pub total_events: usize,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub min_duration_ms: f64,
    pub slowest: Vec<SlowOperation>,
    pub by_service: HashMap<String, ServiceStats>,
    pub by_function: HashMap<String, FunctionStats>,
}

/// Calculate percentile from sorted array
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * sorted.len() as f64).ceil() as usize - 1;
    sorted[idx.min(sorted.len() - 1)]
}

/// Profile trace data within a time range
pub fn profile_traces(store: &TraceStore, from_ts: &str, top_n: usize) -> Result<ProfileResult> {
    let conn = store.connection();
    let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

    // Get all entries with duration in the time range
    let mut stmt = conn.prepare(
        r#"
        SELECT ts, trace_id, span_id, service, event_type, level, payload
        FROM traces
        WHERE ts >= ?1
        ORDER BY ts ASC
        "#,
    )?;

    let mut trace_ids = std::collections::HashSet::new();
    let mut slow_operations: Vec<SlowOperation> = Vec::new();
    let mut service_data: HashMap<String, ServiceData> = HashMap::new();
    let mut function_data: HashMap<String, FunctionData> = HashMap::new();

    let mut total_events = 0;
    let mut total_duration = 0.0;
    let mut max_duration = 0.0f64;
    let mut min_duration = f64::INFINITY;

    let rows = stmt.query_map(params![from_ts], |row| {
        let payload_str: String = row.get(6)?;
        Ok((
            row.get::<_, String>(0)?, // ts
            row.get::<_, String>(1)?, // trace_id
            row.get::<_, String>(2)?, // span_id
            row.get::<_, String>(3)?, // service
            row.get::<_, String>(4)?, // event_type
            row.get::<_, String>(5)?, // level
            payload_str,
        ))
    })?;

    for row_result in rows {
        let (ts, trace_id, _span_id, service, event_type, _level, payload_str) = row_result?;
        total_events += 1;
        trace_ids.insert(trace_id.clone());

        let payload: serde_json::Value =
            serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);

        // Track service stats
        let service_entry = service_data.entry(service.clone()).or_insert(ServiceData {
            durations: Vec::new(),
            errors: 0,
        });

        if event_type == "error" {
            service_entry.errors += 1;
        }

        // Track durations for function_end and http_response events
        if (event_type == "function_end" || event_type == "http_response")
            && payload.get("duration_ms").is_some()
        {
            if let Some(duration) = payload.get("duration_ms").and_then(|v| v.as_f64()) {
                total_duration += duration;
                max_duration = max_duration.max(duration);
                min_duration = min_duration.min(duration);

                service_entry.durations.push(duration);

                // Get function name
                let func_name = if event_type == "function_end" {
                    payload
                        .get("function")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    // For http_response, construct name from method + path
                    let method = payload.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
                    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/");
                    format!("{} {}", method, path)
                };

                // Track function stats
                let func_entry = function_data.entry(func_name.clone()).or_insert(FunctionData {
                    durations: Vec::new(),
                });
                func_entry.durations.push(duration);

                // Track slow operations
                slow_operations.push(SlowOperation {
                    function: func_name,
                    service: service.clone(),
                    duration_ms: duration,
                    trace_id,
                    timestamp: ts,
                });
            }
        }
    }

    // Sort and get top N slowest
    slow_operations.sort_by(|a, b| b.duration_ms.partial_cmp(&a.duration_ms).unwrap());
    let event_count = slow_operations.len().max(1);
    let top_slowest = slow_operations.into_iter().take(top_n).collect();

    // Calculate service stats
    let mut by_service = HashMap::new();
    for (service, data) in service_data {
        let mut sorted = data.durations.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let total: f64 = sorted.iter().sum();
        let avg = if sorted.is_empty() {
            0.0
        } else {
            total / sorted.len() as f64
        };

        by_service.insert(
            service.clone(),
            ServiceStats {
                service,
                event_count: sorted.len(),
                error_count: data.errors,
                avg_duration_ms: avg,
                p50_duration_ms: percentile(&sorted, 50.0),
                p95_duration_ms: percentile(&sorted, 95.0),
                p99_duration_ms: percentile(&sorted, 99.0),
            },
        );
    }

    // Calculate function stats
    let mut by_function = HashMap::new();
    for (func, data) in function_data {
        let mut sorted = data.durations.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let total: f64 = sorted.iter().sum();
        let avg = if sorted.is_empty() {
            0.0
        } else {
            total / sorted.len() as f64
        };

        by_function.insert(
            func.clone(),
            FunctionStats {
                function: func,
                call_count: sorted.len(),
                avg_duration_ms: avg,
                max_duration_ms: sorted.last().copied().unwrap_or(0.0),
                min_duration_ms: sorted.first().copied().unwrap_or(0.0),
            },
        );
    }

    Ok(ProfileResult {
        total_traces: trace_ids.len(),
        total_events,
        avg_duration_ms: if event_count > 0 {
            total_duration / event_count as f64
        } else {
            0.0
        },
        max_duration_ms: if max_duration == 0.0 { 0.0 } else { max_duration },
        min_duration_ms: if min_duration == f64::INFINITY {
            0.0
        } else {
            min_duration
        },
        slowest: top_slowest,
        by_service,
        by_function,
    })
}

/// Internal struct for collecting service data
struct ServiceData {
    durations: Vec<f64>,
    errors: usize,
}

/// Internal struct for collecting function data
struct FunctionData {
    durations: Vec<f64>,
}

// ============================================================================
// Report Generation
// ============================================================================

/// Generate a detailed performance profile report as text
pub fn generate_detailed_report(
    store: &TraceStore,
    from_ts: &str,
    top_n: usize,
) -> Result<String> {
    let profile = profile_traces(store, from_ts, top_n.max(20))?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut lines: Vec<String> = Vec::new();

    lines.push("=".repeat(80));
    lines.push("                         PERFORMANCE PROFILE REPORT".to_string());
    lines.push("=".repeat(80));
    lines.push(String::new());
    lines.push(format!("Time Range: {} - {}", from_ts, now));
    lines.push(String::new());

    // Summary
    lines.push("SUMMARY".to_string());
    lines.push("-".repeat(40));
    lines.push(format!("Total Traces:    {}", profile.total_traces));
    lines.push(format!("Total Events:    {}", profile.total_events));
    lines.push(format!("Avg Duration:    {:.2}ms", profile.avg_duration_ms));
    lines.push(format!("Min Duration:    {:.2}ms", profile.min_duration_ms));
    lines.push(format!("Max Duration:    {:.2}ms", profile.max_duration_ms));
    lines.push(String::new());

    // Service breakdown
    lines.push("BY SERVICE".to_string());
    lines.push("-".repeat(80));
    lines.push(format!(
        "{:<20}{:>10}{:>10}{:>12}{:>12}{:>12}",
        "Service", "Events", "Errors", "Avg(ms)", "P50(ms)", "P95(ms)"
    ));
    lines.push("-".repeat(80));

    // Sort services by avg duration descending
    let mut sorted_services: Vec<_> = profile.by_service.values().collect();
    sorted_services.sort_by(|a, b| {
        b.avg_duration_ms
            .partial_cmp(&a.avg_duration_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for stats in sorted_services {
        lines.push(format!(
            "{:<20}{:>10}{:>10}{:>12.2}{:>12.2}{:>12.2}",
            truncate_string(&stats.service, 19),
            stats.event_count,
            stats.error_count,
            stats.avg_duration_ms,
            stats.p50_duration_ms,
            stats.p95_duration_ms,
        ));
    }
    lines.push(String::new());

    // Top slowest operations
    lines.push(format!("TOP {} SLOWEST OPERATIONS", top_n.min(profile.slowest.len())));
    lines.push("-".repeat(80));

    for (i, op) in profile.slowest.iter().enumerate().take(top_n) {
        lines.push(format!("{:>2}. {}", i + 1, op.function));
        lines.push(format!("    Service: {}", op.service));
        lines.push(format!("    Duration: {}ms", op.duration_ms));
        lines.push(format!("    Trace ID: {}", op.trace_id));
        lines.push(format!("    Time: {}", op.timestamp));
        lines.push(String::new());
    }

    lines.push("=".repeat(80));

    Ok(lines.join("\n"))
}

/// Compare performance between two time periods
pub fn compare_periods(
    store: &TraceStore,
    period1_start: &str,
    period1_end: &str,
    period2_start: &str,
    period2_end: &str,
) -> Result<PeriodComparison> {
    // Profile both periods using their end dates as cutoffs
    let profile1 = profile_traces(store, period1_start, 10)?;
    let profile2 = profile_traces(store, period2_start, 10)?;

    // Calculate service-level comparisons
    let mut service_changes: Vec<ServiceChange> = Vec::new();
    let all_services: std::collections::HashSet<_> = profile1
        .by_service
        .keys()
        .chain(profile2.by_service.keys())
        .collect();

    for service in all_services {
        let s1 = profile1.by_service.get(service);
        let s2 = profile2.by_service.get(service);

        let avg1 = s1.map(|s| s.avg_duration_ms).unwrap_or(0.0);
        let avg2 = s2.map(|s| s.avg_duration_ms).unwrap_or(0.0);
        let err1 = s1.map(|s| s.error_count).unwrap_or(0);
        let err2 = s2.map(|s| s.error_count).unwrap_or(0);

        let change_pct = if avg1 > 0.0 {
            ((avg2 - avg1) / avg1) * 100.0
        } else if avg2 > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };

        service_changes.push(ServiceChange {
            service: service.clone(),
            period1_avg_ms: avg1,
            period2_avg_ms: avg2,
            change_pct,
            period1_errors: err1,
            period2_errors: err2,
        });
    }

    // Sort by absolute change
    service_changes.sort_by(|a, b| {
        b.change_pct
            .abs()
            .partial_cmp(&a.change_pct.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Identify regressions and improvements
    let regressions: Vec<_> = service_changes
        .iter()
        .filter(|s| s.change_pct > 10.0)
        .cloned()
        .collect();

    let improvements: Vec<_> = service_changes
        .iter()
        .filter(|s| s.change_pct < -10.0)
        .cloned()
        .collect();

    Ok(PeriodComparison {
        period1_label: format!("{} - {}", period1_start, period1_end),
        period2_label: format!("{} - {}", period2_start, period2_end),
        summary: PeriodSummary {
            period1_traces: profile1.total_traces,
            period2_traces: profile2.total_traces,
            period1_events: profile1.total_events,
            period2_events: profile2.total_events,
            period1_avg_ms: profile1.avg_duration_ms,
            period2_avg_ms: profile2.avg_duration_ms,
            period1_max_ms: profile1.max_duration_ms,
            period2_max_ms: profile2.max_duration_ms,
        },
        service_changes,
        regressions,
        improvements,
    })
}

/// Generate a text comparison report
pub fn generate_comparison_report(
    store: &TraceStore,
    period1_start: &str,
    period1_end: &str,
    period2_start: &str,
    period2_end: &str,
) -> Result<String> {
    let comparison = compare_periods(store, period1_start, period1_end, period2_start, period2_end)?;

    let mut lines: Vec<String> = Vec::new();

    lines.push("=".repeat(80));
    lines.push("                      PERFORMANCE COMPARISON REPORT".to_string());
    lines.push("=".repeat(80));
    lines.push(String::new());

    lines.push(format!("Period 1: {}", comparison.period1_label));
    lines.push(format!("Period 2: {}", comparison.period2_label));
    lines.push(String::new());

    // Summary comparison
    lines.push("OVERALL SUMMARY".to_string());
    lines.push("-".repeat(60));
    lines.push(format!(
        "{:<25}{:>15}{:>15}{:>15}",
        "Metric", "Period 1", "Period 2", "Change"
    ));
    lines.push("-".repeat(60));

    let summary = &comparison.summary;

    lines.push(format!(
        "{:<25}{:>15}{:>15}{:>15}",
        "Total Traces",
        summary.period1_traces,
        summary.period2_traces,
        format_change(summary.period1_traces as f64, summary.period2_traces as f64)
    ));

    lines.push(format!(
        "{:<25}{:>15}{:>15}{:>15}",
        "Total Events",
        summary.period1_events,
        summary.period2_events,
        format_change(summary.period1_events as f64, summary.period2_events as f64)
    ));

    lines.push(format!(
        "{:<25}{:>15.2}{:>15.2}{:>15}",
        "Avg Duration (ms)",
        summary.period1_avg_ms,
        summary.period2_avg_ms,
        format_change(summary.period1_avg_ms, summary.period2_avg_ms)
    ));

    lines.push(format!(
        "{:<25}{:>15.2}{:>15.2}{:>15}",
        "Max Duration (ms)",
        summary.period1_max_ms,
        summary.period2_max_ms,
        format_change(summary.period1_max_ms, summary.period2_max_ms)
    ));
    lines.push(String::new());

    // Service-level comparison
    lines.push("BY SERVICE COMPARISON".to_string());
    lines.push("-".repeat(80));
    lines.push(format!(
        "{:<20}{:>12}{:>12}{:>12}{:>12}{:>12}",
        "Service", "Avg P1 (ms)", "Avg P2 (ms)", "Change", "Errors P1", "Errors P2"
    ));
    lines.push("-".repeat(80));

    for sc in &comparison.service_changes {
        lines.push(format!(
            "{:<20}{:>12.2}{:>12.2}{:>12}{:>12}{:>12}",
            truncate_string(&sc.service, 19),
            sc.period1_avg_ms,
            sc.period2_avg_ms,
            format_change_pct(sc.change_pct),
            sc.period1_errors,
            sc.period2_errors
        ));
    }
    lines.push(String::new());

    // Regressions
    if !comparison.regressions.is_empty() {
        lines.push("⚠️  REGRESSIONS DETECTED (>10% slower)".to_string());
        lines.push("-".repeat(40));
        for r in &comparison.regressions {
            lines.push(format!("  {}: +{:.1}%", r.service, r.change_pct));
        }
        lines.push(String::new());
    }

    // Improvements
    if !comparison.improvements.is_empty() {
        lines.push("✅ IMPROVEMENTS DETECTED (>10% faster)".to_string());
        lines.push("-".repeat(40));
        for r in &comparison.improvements {
            lines.push(format!("  {}: {:.1}%", r.service, r.change_pct));
        }
        lines.push(String::new());
    }

    lines.push("=".repeat(80));
    lines.push(String::new());
    lines.push("Legend: 🔴 Regression (>10% slower)  🟢 Improvement (>10% faster)".to_string());

    Ok(lines.join("\n"))
}

// ============================================================================
// Period Comparison Types
// ============================================================================

/// Summary of a period comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodSummary {
    pub period1_traces: usize,
    pub period2_traces: usize,
    pub period1_events: usize,
    pub period2_events: usize,
    pub period1_avg_ms: f64,
    pub period2_avg_ms: f64,
    pub period1_max_ms: f64,
    pub period2_max_ms: f64,
}

/// Change in a service between periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceChange {
    pub service: String,
    pub period1_avg_ms: f64,
    pub period2_avg_ms: f64,
    pub change_pct: f64,
    pub period1_errors: usize,
    pub period2_errors: usize,
}

/// Full period comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodComparison {
    pub period1_label: String,
    pub period2_label: String,
    pub summary: PeriodSummary,
    pub service_changes: Vec<ServiceChange>,
    pub regressions: Vec<ServiceChange>,
    pub improvements: Vec<ServiceChange>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Truncate a string to max length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format a change as percentage string
fn format_change(p1: f64, p2: f64) -> String {
    if p1 == 0.0 && p2 == 0.0 {
        return "N/A".to_string();
    }
    if p1 == 0.0 {
        return "+∞".to_string();
    }
    let pct = ((p2 - p1) / p1) * 100.0;
    format_change_pct(pct)
}

/// Format a percentage change with indicator
fn format_change_pct(pct: f64) -> String {
    if pct.is_infinite() {
        return "+∞".to_string();
    }
    let sign = if pct >= 0.0 { "+" } else { "" };
    let indicator = if pct > 10.0 {
        " 🔴"
    } else if pct < -10.0 {
        " 🟢"
    } else {
        ""
    };
    format!("{}{:.1}%{}", sign, pct, indicator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::TraceEntry;

    fn create_entry_with_duration(
        trace_id: &str,
        service: &str,
        function: &str,
        duration_ms: f64,
    ) -> TraceEntry {
        TraceEntry {
            ts: chrono::Utc::now().to_rfc3339(),
            trace_id: trace_id.to_string(),
            span_id: "abcd1234".to_string(),
            parent_span_id: None,
            service: service.to_string(),
            event_type: "function_end".to_string(),
            level: "info".to_string(),
            payload: serde_json::json!({
                "function": function,
                "duration_ms": duration_ms
            }),
        }
    }

    #[test]
    fn test_percentile() {
        let sorted = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        assert_eq!(percentile(&sorted, 50.0), 50.0);
        assert_eq!(percentile(&sorted, 95.0), 100.0);
        assert_eq!(percentile(&sorted, 99.0), 100.0);
    }

    #[test]
    fn test_profile_traces() {
        let store = TraceStore::in_memory().unwrap();

        // Add test entries
        store
            .append(&create_entry_with_duration("t1", "ccode-api", "func1", 100.0))
            .unwrap();
        store
            .append(&create_entry_with_duration("t1", "ccode-api", "func2", 200.0))
            .unwrap();
        store
            .append(&create_entry_with_duration("t2", "zero-channels", "handler", 50.0))
            .unwrap();

        let from_ts = "2000-01-01T00:00:00Z";
        let profile = profile_traces(&store, from_ts, 10).unwrap();

        assert_eq!(profile.total_traces, 2);
        assert_eq!(profile.total_events, 3);
        assert_eq!(profile.slowest.len(), 3);
        assert_eq!(profile.slowest[0].duration_ms, 200.0);
        assert!(profile.by_service.contains_key("ccode-api"));
        assert!(profile.by_function.contains_key("func1"));
    }

    #[test]
    fn test_empty_profile() {
        let store = TraceStore::in_memory().unwrap();
        let profile = profile_traces(&store, "2000-01-01T00:00:00Z", 10).unwrap();

        assert_eq!(profile.total_traces, 0);
        assert_eq!(profile.total_events, 0);
        assert_eq!(profile.avg_duration_ms, 0.0);
    }

    #[test]
    fn test_generate_detailed_report() {
        let store = TraceStore::in_memory().unwrap();

        store
            .append(&create_entry_with_duration("t1", "ccode-api", "func1", 100.0))
            .unwrap();
        store
            .append(&create_entry_with_duration("t2", "zero-channels", "handler", 200.0))
            .unwrap();

        let report = generate_detailed_report(&store, "2000-01-01T00:00:00Z", 10).unwrap();

        assert!(report.contains("PERFORMANCE PROFILE REPORT"));
        assert!(report.contains("Total Traces:"));
        assert!(report.contains("BY SERVICE"));
        assert!(report.contains("TOP"));
        assert!(report.contains("ccode-api"));
        assert!(report.contains("zero-channels"));
    }

    #[test]
    fn test_compare_periods() {
        let store = TraceStore::in_memory().unwrap();

        // Period 1: 100ms avg
        store
            .append(&create_entry_with_duration("t1", "ccode-api", "func1", 100.0))
            .unwrap();

        // Period 2: 200ms avg (100% regression)
        store
            .append(&create_entry_with_duration("t2", "ccode-api", "func1", 200.0))
            .unwrap();

        let comparison = compare_periods(
            &store,
            "2000-01-01T00:00:00Z",
            "2050-01-01T00:00:00Z",
            "2000-01-01T00:00:00Z",
            "2050-01-01T00:00:00Z",
        )
        .unwrap();

        assert!(!comparison.service_changes.is_empty());
        assert!(comparison.summary.period1_traces > 0);
        assert!(comparison.summary.period2_traces > 0);
    }

    #[test]
    fn test_generate_comparison_report() {
        let store = TraceStore::in_memory().unwrap();

        store
            .append(&create_entry_with_duration("t1", "ccode-api", "func1", 100.0))
            .unwrap();

        let report = generate_comparison_report(
            &store,
            "2000-01-01T00:00:00Z",
            "2025-01-01T00:00:00Z",
            "2025-01-01T00:00:00Z",
            "2050-01-01T00:00:00Z",
        )
        .unwrap();

        assert!(report.contains("PERFORMANCE COMPARISON REPORT"));
        assert!(report.contains("Period 1:"));
        assert!(report.contains("Period 2:"));
        assert!(report.contains("BY SERVICE COMPARISON"));
    }

    #[test]
    fn test_format_change_helpers() {
        assert_eq!(format_change(0.0, 0.0), "N/A");
        assert_eq!(format_change(0.0, 100.0), "+∞");
        assert!(format_change(100.0, 200.0).contains("+100.0%"));
        assert!(format_change(200.0, 100.0).contains("-50.0%"));
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long string", 10), "this is...");
    }
}
