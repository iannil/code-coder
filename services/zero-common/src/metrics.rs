//! Unified Metrics Collection for Zero Services
//!
//! Provides a lightweight metrics system with Prometheus-compatible text output.
//! Supports counters, gauges, and histograms with sliding window percentile calculation.
//!
//! # Features
//!
//! - Thread-safe metrics collection via `RwLock`
//! - Sliding window for accurate percentile calculations (5-minute window)
//! - Prometheus-compatible text format output
//! - Process resource metrics (memory, CPU)
//! - Per-service isolation
//!
//! # Example
//!
//! ```ignore
//! use zero_common::metrics::{Metrics, MetricsRegistry};
//!
//! // Create a metrics registry for a service
//! let registry = MetricsRegistry::new("zero-gateway");
//!
//! // Record a request
//! registry.record_request("/api/task", "POST", 200, 45.0);
//!
//! // Get metrics as Prometheus text format
//! let output = registry.render();
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Default window size for sliding histograms (5 minutes)
const DEFAULT_WINDOW_SIZE: Duration = Duration::from_secs(300);

/// Maximum number of samples to keep in a histogram
const MAX_HISTOGRAM_SAMPLES: usize = 10_000;

// ============================================================================
// Core Types
// ============================================================================

/// A timestamped sample for sliding window calculations.
#[derive(Debug, Clone, Copy)]
struct Sample {
    value: f64,
    timestamp: Instant,
}

/// Counter metric: monotonically increasing value.
#[derive(Debug, Default)]
pub struct Counter {
    value: u64,
}

impl Counter {
    /// Increment the counter by 1.
    pub fn inc(&mut self) {
        self.value = self.value.saturating_add(1);
    }

    /// Increment the counter by a specific amount.
    pub fn inc_by(&mut self, n: u64) {
        self.value = self.value.saturating_add(n);
    }

    /// Get the current value.
    pub fn get(&self) -> u64 {
        self.value
    }
}

/// Gauge metric: value that can go up or down.
#[derive(Debug, Default)]
pub struct Gauge {
    value: f64,
}

impl Gauge {
    /// Set the gauge to a specific value.
    pub fn set(&mut self, value: f64) {
        self.value = value;
    }

    /// Increment the gauge.
    pub fn inc(&mut self) {
        self.value += 1.0;
    }

    /// Decrement the gauge.
    pub fn dec(&mut self) {
        self.value -= 1.0;
    }

    /// Get the current value.
    pub fn get(&self) -> f64 {
        self.value
    }
}

/// Histogram metric: tracks distribution of values.
#[derive(Debug)]
pub struct Histogram {
    samples: Vec<Sample>,
    window_size: Duration,
    count: u64,
    sum: f64,
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            samples: Vec::with_capacity(1000),
            window_size: DEFAULT_WINDOW_SIZE,
            count: 0,
            sum: 0.0,
        }
    }
}

impl Histogram {
    /// Create a new histogram with custom window size.
    pub fn with_window(window: Duration) -> Self {
        Self {
            samples: Vec::with_capacity(1000),
            window_size: window,
            count: 0,
            sum: 0.0,
        }
    }

    /// Record a sample.
    pub fn observe(&mut self, value: f64) {
        let now = Instant::now();
        self.samples.push(Sample {
            value,
            timestamp: now,
        });
        self.count += 1;
        self.sum += value;

        // Prune old samples and keep size bounded
        self.prune(now);
    }

    /// Prune samples outside the sliding window.
    fn prune(&mut self, now: Instant) {
        let cutoff = now - self.window_size;
        self.samples.retain(|s| s.timestamp > cutoff);

        // Also bound the total number of samples
        if self.samples.len() > MAX_HISTOGRAM_SAMPLES {
            let excess = self.samples.len() - MAX_HISTOGRAM_SAMPLES;
            self.samples.drain(0..excess);
        }
    }

    /// Calculate a percentile (0.0 to 1.0).
    pub fn percentile(&mut self, p: f64) -> Option<f64> {
        self.prune(Instant::now());

        if self.samples.is_empty() {
            return None;
        }

        let mut values: Vec<f64> = self.samples.iter().map(|s| s.value).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let idx = ((values.len() as f64 - 1.0) * p).round() as usize;
        Some(values[idx.min(values.len() - 1)])
    }

    /// Get p50 (median).
    pub fn p50(&mut self) -> Option<f64> {
        self.percentile(0.5)
    }

    /// Get p95.
    pub fn p95(&mut self) -> Option<f64> {
        self.percentile(0.95)
    }

    /// Get p99.
    pub fn p99(&mut self) -> Option<f64> {
        self.percentile(0.99)
    }

    /// Get total count (all-time).
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get sum of all values (all-time).
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// Get current sample count (within window).
    pub fn window_count(&self) -> usize {
        self.samples.len()
    }
}

// ============================================================================
// Labeled Metrics
// ============================================================================

/// Labels for a metric.
pub type Labels = HashMap<String, String>;

/// Convert labels to a canonical string key for HashMap usage.
/// Labels are sorted by key to ensure consistent hashing.
fn labels_to_key(labels: &Labels) -> String {
    let mut pairs: Vec<_> = labels.iter().collect();
    pairs.sort_by_key(|(k, _)| *k);
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(",")
}

/// A metric with labels.
#[derive(Debug, Default)]
struct LabeledCounters {
    counters: HashMap<String, (Labels, Counter)>,
}

impl LabeledCounters {
    fn get_or_create(&mut self, labels: Labels) -> &mut Counter {
        let key = labels_to_key(&labels);
        &mut self.counters.entry(key).or_insert_with(|| (labels, Counter::default())).1
    }

    fn iter(&self) -> impl Iterator<Item = (&Labels, &Counter)> {
        self.counters.values().map(|(l, c)| (l, c))
    }
}

#[derive(Debug, Default)]
struct LabeledGauges {
    gauges: HashMap<String, (Labels, Gauge)>,
}

impl LabeledGauges {
    fn get_or_create(&mut self, labels: Labels) -> &mut Gauge {
        let key = labels_to_key(&labels);
        &mut self.gauges.entry(key).or_insert_with(|| (labels, Gauge::default())).1
    }

    fn iter(&self) -> impl Iterator<Item = (&Labels, &Gauge)> {
        self.gauges.values().map(|(l, g)| (l, g))
    }
}

#[derive(Debug, Default)]
struct LabeledHistograms {
    histograms: HashMap<String, (Labels, Histogram)>,
}

impl LabeledHistograms {
    fn get_or_create(&mut self, labels: Labels) -> &mut Histogram {
        let key = labels_to_key(&labels);
        &mut self.histograms.entry(key).or_insert_with(|| (labels, Histogram::default())).1
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = (&Labels, &mut Histogram)> {
        self.histograms.values_mut().map(|(l, h)| (l as &Labels, h))
    }
}

// ============================================================================
// Metrics Registry
// ============================================================================

/// Inner state for the metrics registry.
#[derive(Debug, Default)]
struct MetricsInner {
    /// HTTP request counters by {method, path, status}
    http_requests: LabeledCounters,
    /// HTTP request duration histograms by {method, path}
    http_duration: LabeledHistograms,
    /// Active connections gauge
    active_connections: Gauge,
    /// Error counters by {type}
    errors: LabeledCounters,
    /// Process memory in bytes
    process_memory: Gauge,
    /// Process start time (unix timestamp)
    start_time: u64,
}

/// Thread-safe metrics registry for a service.
#[derive(Debug, Clone)]
pub struct MetricsRegistry {
    service: String,
    inner: Arc<RwLock<MetricsInner>>,
}

impl MetricsRegistry {
    /// Create a new metrics registry for a service.
    pub fn new(service: impl Into<String>) -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            service: service.into(),
            inner: Arc::new(RwLock::new(MetricsInner {
                start_time,
                ..Default::default()
            })),
        }
    }

    /// Record an HTTP request.
    pub async fn record_request(&self, path: &str, method: &str, status: u16, duration_ms: f64) {
        let mut inner = self.inner.write().await;

        // Increment request counter
        let mut labels = Labels::new();
        labels.insert("service".to_string(), self.service.clone());
        labels.insert("method".to_string(), method.to_string());
        labels.insert("path".to_string(), path.to_string());
        labels.insert("status".to_string(), status.to_string());
        inner.http_requests.get_or_create(labels).inc();

        // Record duration
        let mut duration_labels = Labels::new();
        duration_labels.insert("service".to_string(), self.service.clone());
        duration_labels.insert("method".to_string(), method.to_string());
        duration_labels.insert("path".to_string(), path.to_string());
        inner
            .http_duration
            .get_or_create(duration_labels)
            .observe(duration_ms);
    }

    /// Record an error.
    pub async fn record_error(&self, error_type: &str) {
        let mut inner = self.inner.write().await;
        let mut labels = Labels::new();
        labels.insert("service".to_string(), self.service.clone());
        labels.insert("type".to_string(), error_type.to_string());
        inner.errors.get_or_create(labels).inc();
    }

    /// Set active connections count.
    pub async fn set_connections(&self, count: u64) {
        let mut inner = self.inner.write().await;
        inner.active_connections.set(count as f64);
    }

    /// Increment active connections.
    pub async fn inc_connections(&self) {
        let mut inner = self.inner.write().await;
        inner.active_connections.inc();
    }

    /// Decrement active connections.
    pub async fn dec_connections(&self) {
        let mut inner = self.inner.write().await;
        inner.active_connections.dec();
    }

    /// Update process memory metric.
    pub async fn update_memory(&self) {
        // Get memory from /proc/self/statm on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
                if let Some(rss_pages) = statm.split_whitespace().nth(1) {
                    if let Ok(pages) = rss_pages.parse::<u64>() {
                        let page_size = 4096u64; // Usually 4KB
                        let bytes = pages * page_size;
                        let mut inner = self.inner.write().await;
                        inner.process_memory.set(bytes as f64);
                    }
                }
            }
        }

        // Fallback for other platforms - use a rough estimate
        #[cfg(not(target_os = "linux"))]
        {
            // No-op on macOS/Windows for now
            // Could use mach APIs on macOS or GetProcessMemoryInfo on Windows
        }
    }

    /// Get a snapshot of metrics for JSON output.
    pub async fn snapshot(&self) -> MetricsSnapshot {
        let mut inner = self.inner.write().await;

        // Calculate totals
        let total_requests: u64 = inner.http_requests.iter().map(|(_, c)| c.get()).sum();
        let error_requests: u64 = inner
            .http_requests
            .iter()
            .filter(|(l, _)| {
                l.get("status")
                    .is_some_and(|s| s.starts_with('4') || s.starts_with('5'))
            })
            .map(|(_, c)| c.get())
            .sum();

        // Calculate aggregated percentiles
        let mut all_durations: Vec<f64> = Vec::new();
        for (_, hist) in inner.http_duration.iter_mut() {
            for sample in &hist.samples {
                all_durations.push(sample.value);
            }
        }

        all_durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let percentile = |p: f64| -> f64 {
            if all_durations.is_empty() {
                return 0.0;
            }
            let idx = ((all_durations.len() as f64 - 1.0) * p).round() as usize;
            all_durations[idx.min(all_durations.len() - 1)]
        };

        MetricsSnapshot {
            service: self.service.clone(),
            total_requests,
            error_requests,
            error_rate: if total_requests > 0 {
                (error_requests as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            },
            p50_ms: percentile(0.5),
            p95_ms: percentile(0.95),
            p99_ms: percentile(0.99),
            active_connections: inner.active_connections.get() as u64,
            memory_bytes: inner.process_memory.get() as u64,
            uptime_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(inner.start_time),
        }
    }

    /// Render metrics in Prometheus text format.
    pub async fn render(&self) -> String {
        let mut inner = self.inner.write().await;
        let mut output = String::with_capacity(4096);

        // ── HTTP Request Counter ──
        output.push_str("# TYPE http_requests_total counter\n");
        for (labels, counter) in inner.http_requests.iter() {
            let label_str = format_labels(labels);
            output.push_str(&format!(
                "http_requests_total{{{}}} {}\n",
                label_str,
                counter.get()
            ));
        }

        // ── HTTP Duration Histogram ──
        output.push_str("\n# TYPE http_request_duration_ms histogram\n");
        for (labels, hist) in inner.http_duration.iter_mut() {
            let label_str = format_labels(labels);
            if let Some(p50) = hist.p50() {
                output.push_str(&format!(
                    "http_request_duration_ms_p50{{{}}} {:.2}\n",
                    label_str, p50
                ));
            }
            if let Some(p95) = hist.p95() {
                output.push_str(&format!(
                    "http_request_duration_ms_p95{{{}}} {:.2}\n",
                    label_str, p95
                ));
            }
            if let Some(p99) = hist.p99() {
                output.push_str(&format!(
                    "http_request_duration_ms_p99{{{}}} {:.2}\n",
                    label_str, p99
                ));
            }
            output.push_str(&format!(
                "http_request_duration_ms_count{{{}}} {}\n",
                label_str,
                hist.count()
            ));
            output.push_str(&format!(
                "http_request_duration_ms_sum{{{}}} {:.2}\n",
                label_str,
                hist.sum()
            ));
        }

        // ── Error Counter ──
        output.push_str("\n# TYPE errors_total counter\n");
        for (labels, counter) in inner.errors.iter() {
            let label_str = format_labels(labels);
            output.push_str(&format!("errors_total{{{}}} {}\n", label_str, counter.get()));
        }

        // ── Active Connections ──
        output.push_str(&format!(
            "\n# TYPE active_connections gauge\nactive_connections{{service=\"{}\"}} {}\n",
            self.service,
            inner.active_connections.get()
        ));

        // ── Process Memory ──
        output.push_str(&format!(
            "\n# TYPE process_memory_bytes gauge\nprocess_memory_bytes{{service=\"{}\"}} {}\n",
            self.service,
            inner.process_memory.get()
        ));

        // ── Process Start Time ──
        output.push_str(&format!(
            "\n# TYPE process_start_time_seconds gauge\nprocess_start_time_seconds{{service=\"{}\"}} {}\n",
            self.service, inner.start_time
        ));

        output
    }
}

/// Snapshot of metrics for JSON serialization.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub service: String,
    pub total_requests: u64,
    pub error_requests: u64,
    pub error_rate: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub active_connections: u64,
    pub memory_bytes: u64,
    pub uptime_secs: u64,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Format labels for Prometheus output.
fn format_labels(labels: &Labels) -> String {
    let mut parts: Vec<String> = labels
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, v.replace('\"', "\\\"")))
        .collect();
    parts.sort(); // Consistent ordering
    parts.join(",")
}

// ============================================================================
// Global Registry (for convenience)
// ============================================================================

use std::sync::OnceLock;

static GLOBAL_REGISTRY: OnceLock<MetricsRegistry> = OnceLock::new();

/// Initialize the global metrics registry.
pub fn init_global(service: impl Into<String>) {
    let _ = GLOBAL_REGISTRY.set(MetricsRegistry::new(service));
}

/// Get the global metrics registry.
pub fn global() -> Option<&'static MetricsRegistry> {
    GLOBAL_REGISTRY.get()
}

// ============================================================================
// Axum Integration
// ============================================================================

/// Axum handler for `/metrics` endpoint.
#[cfg(feature = "axum")]
pub async fn metrics_handler(
    axum::extract::State(registry): axum::extract::State<Arc<MetricsRegistry>>,
) -> impl axum::response::IntoResponse {
    registry.update_memory().await;
    let body = registry.render().await;
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
}

/// Axum handler for `/metrics` as JSON.
#[cfg(feature = "axum")]
pub async fn metrics_json_handler(
    axum::extract::State(registry): axum::extract::State<Arc<MetricsRegistry>>,
) -> impl axum::response::IntoResponse {
    registry.update_memory().await;
    let snapshot = registry.snapshot().await;
    axum::Json(snapshot)
}

// ============================================================================
// Middleware for Automatic Request Tracking
// ============================================================================

/// Metrics middleware state.
#[derive(Clone)]
pub struct MetricsMiddlewareState {
    pub registry: Arc<MetricsRegistry>,
}

/// Create metrics recording middleware for axum.
#[cfg(feature = "axum")]
pub async fn metrics_middleware(
    axum::extract::State(state): axum::extract::State<MetricsMiddlewareState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Increment active connections
    state.registry.inc_connections().await;

    // Process request
    let response = next.run(req).await;

    // Decrement active connections
    state.registry.dec_connections().await;

    // Record metrics
    let status = response.status().as_u16();
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    state
        .registry
        .record_request(&path, &method, status, duration_ms)
        .await;

    // Record errors
    if status >= 400 {
        let error_type = if status >= 500 {
            "server_error"
        } else {
            "client_error"
        };
        state.registry.record_error(error_type).await;
    }

    response
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let mut counter = Counter::default();
        assert_eq!(counter.get(), 0);
        counter.inc();
        assert_eq!(counter.get(), 1);
        counter.inc_by(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let mut gauge = Gauge::default();
        assert_eq!(gauge.get(), 0.0);
        gauge.set(10.0);
        assert_eq!(gauge.get(), 10.0);
        gauge.inc();
        assert_eq!(gauge.get(), 11.0);
        gauge.dec();
        assert_eq!(gauge.get(), 10.0);
    }

    #[test]
    fn test_histogram() {
        let mut hist = Histogram::default();

        // Add some samples
        for i in 1..=100 {
            hist.observe(i as f64);
        }

        assert_eq!(hist.count(), 100);
        assert_eq!(hist.sum(), 5050.0);

        // Check percentiles
        let p50 = hist.p50().unwrap();
        assert!((p50 - 50.0).abs() < 2.0);

        let p99 = hist.p99().unwrap();
        assert!((p99 - 99.0).abs() < 2.0);
    }

    #[tokio::test]
    async fn test_registry() {
        let registry = MetricsRegistry::new("test-service");

        // Record some requests
        registry.record_request("/api/test", "GET", 200, 50.0).await;
        registry.record_request("/api/test", "GET", 200, 75.0).await;
        registry.record_request("/api/test", "GET", 500, 100.0).await;

        // Get snapshot
        let snapshot = registry.snapshot().await;
        assert_eq!(snapshot.service, "test-service");
        assert_eq!(snapshot.total_requests, 3);
        assert_eq!(snapshot.error_requests, 1);
    }

    #[tokio::test]
    async fn test_render() {
        let registry = MetricsRegistry::new("test-service");
        registry.record_request("/api/test", "GET", 200, 50.0).await;

        let output = registry.render().await;
        assert!(output.contains("http_requests_total"));
        assert!(output.contains("http_request_duration_ms"));
        assert!(output.contains("test-service"));
    }
}
