//! Base Watcher State
//!
//! Common state and utilities shared by all watchers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

// ══════════════════════════════════════════════════════════════════════════════
// Health Status
// ══════════════════════════════════════════════════════════════════════════════

/// Health status of a watcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatcherHealth {
    /// Watcher is functioning normally
    Healthy,
    /// Watcher is experiencing issues but still operational
    Degraded,
    /// Watcher is failing
    Failing,
    /// Watcher is stopped
    Stopped,
}

impl Default for WatcherHealth {
    fn default() -> Self {
        Self::Stopped
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Watcher Options
// ══════════════════════════════════════════════════════════════════════════════

/// Common options for all watchers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherOptions {
    /// Custom watcher ID (auto-generated if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Observation interval in ms (0 = event-driven only)
    #[serde(default)]
    pub interval_ms: u64,
    /// Filter patterns
    #[serde(default)]
    pub filters: Vec<String>,
    /// Priority (0-10)
    #[serde(default = "default_priority")]
    pub priority: u8,
}

fn default_priority() -> u8 {
    5
}

impl Default for WatcherOptions {
    fn default() -> Self {
        Self {
            id: None,
            interval_ms: 0,
            filters: Vec::new(),
            priority: 5,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Watcher Metrics
// ══════════════════════════════════════════════════════════════════════════════

/// Metrics collected by a watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherMetrics {
    /// Total observations emitted
    pub observation_count: u64,
    /// Total errors encountered
    pub error_count: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: u64,
    /// Last observation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_observation: Option<DateTime<Utc>>,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f32,
}

impl Default for WatcherMetrics {
    fn default() -> Self {
        Self {
            observation_count: 0,
            error_count: 0,
            avg_latency_ms: 0,
            last_observation: None,
            error_rate: 0.0,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Base Watcher State
// ══════════════════════════════════════════════════════════════════════════════

/// Common state shared by all watcher implementations.
///
/// This struct manages lifecycle, metrics, and latency tracking that all
/// watchers need. Individual watcher implementations embed this and add
/// their domain-specific state.
pub struct BaseWatcherState {
    /// Unique watcher ID
    pub id: String,
    /// Whether the watcher is currently running
    pub running: bool,
    /// Total observations emitted
    pub observation_count: u64,
    /// Total errors encountered
    pub error_count: u64,
    /// Last observation timestamp
    pub last_observation: Option<DateTime<Utc>>,
    /// Latency samples (in milliseconds)
    latencies: VecDeque<u64>,
    /// Maximum latency samples to keep
    max_latency_samples: usize,
}

impl BaseWatcherState {
    /// Create a new base watcher state.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            running: false,
            observation_count: 0,
            error_count: 0,
            last_observation: None,
            latencies: VecDeque::new(),
            max_latency_samples: 100,
        }
    }

    /// Generate a unique watcher ID.
    pub fn generate_id(watcher_type: &str) -> String {
        let timestamp = Utc::now().timestamp_millis();
        let random: u32 = rand::random::<u32>() % 100000;
        format!("{}_{timestamp}_{random}", watcher_type)
    }

    /// Record a successful observation.
    pub fn record_observation(&mut self, latency_ms: u64) {
        self.observation_count += 1;
        self.last_observation = Some(Utc::now());
        self.track_latency(latency_ms);
    }

    /// Record an error.
    pub fn record_error(&mut self) {
        self.error_count += 1;
    }

    /// Track a latency sample.
    fn track_latency(&mut self, latency_ms: u64) {
        self.latencies.push_back(latency_ms);
        if self.latencies.len() > self.max_latency_samples {
            self.latencies.pop_front();
        }
    }

    /// Calculate average latency.
    pub fn avg_latency(&self) -> u64 {
        if self.latencies.is_empty() {
            return 0;
        }
        let sum: u64 = self.latencies.iter().sum();
        sum / self.latencies.len() as u64
    }

    /// Calculate error rate.
    pub fn error_rate(&self) -> f32 {
        if self.observation_count == 0 {
            return 0.0;
        }
        self.error_count as f32 / (self.observation_count + self.error_count) as f32
    }

    /// Calculate health based on error rate.
    pub fn calculate_health(&self) -> WatcherHealth {
        if !self.running {
            return WatcherHealth::Stopped;
        }

        let error_rate = self.error_rate();
        if error_rate > 0.5 {
            WatcherHealth::Failing
        } else if error_rate > 0.1 {
            WatcherHealth::Degraded
        } else {
            WatcherHealth::Healthy
        }
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> WatcherMetrics {
        WatcherMetrics {
            observation_count: self.observation_count,
            error_count: self.error_count,
            avg_latency_ms: self.avg_latency(),
            last_observation: self.last_observation,
            error_rate: self.error_rate(),
        }
    }

    /// Check if a source matches the configured filters.
    /// Returns true if no filters configured or if source matches any filter.
    pub fn matches_filters(source: &str, filters: &[String]) -> bool {
        if filters.is_empty() {
            return true;
        }

        filters.iter().any(|filter| {
            // Support glob-like patterns
            if filter.contains('*') {
                // Convert glob to regex:
                // Handle **/  -> (.*/)? (match zero or more directories)
                // Handle /**  -> (/.*)?  (match zero or more path suffix)
                // Handle **   -> .*      (match any characters including /)
                // Handle *    -> [^/]*   (match within path segment)
                let pattern = filter
                    .replace("**/", "\x00SLASHSTAR\x00")
                    .replace("/**", "\x00STARSLASH\x00")
                    .replace("**", "\x00DOUBLESTAR\x00")
                    .replace('.', r"\.")
                    .replace('?', ".")
                    .replace('*', "[^/]*")
                    .replace("\x00SLASHSTAR\x00", "(.*/)?" )
                    .replace("\x00STARSLASH\x00", "(/.*)?")
                    .replace("\x00DOUBLESTAR\x00", ".*");

                if let Ok(regex) = regex::Regex::new(&format!("^{pattern}$")) {
                    return regex.is_match(source);
                }
            }
            source.contains(filter)
        })
    }
}

impl Default for BaseWatcherState {
    fn default() -> Self {
        Self::new("default")
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_watcher_state_creation() {
        let state = BaseWatcherState::new("test-watcher");
        assert_eq!(state.id, "test-watcher");
        assert!(!state.running);
        assert_eq!(state.observation_count, 0);
        assert_eq!(state.error_count, 0);
    }

    #[test]
    fn test_record_observation() {
        let mut state = BaseWatcherState::new("test");
        state.running = true;

        state.record_observation(50);
        state.record_observation(100);
        state.record_observation(75);

        assert_eq!(state.observation_count, 3);
        assert_eq!(state.avg_latency(), 75);
        assert!(state.last_observation.is_some());
    }

    #[test]
    fn test_error_rate_calculation() {
        let mut state = BaseWatcherState::new("test");
        state.running = true;

        state.record_observation(10);
        state.record_observation(10);
        state.record_error();
        state.record_observation(10);

        // 1 error out of 4 total (3 observations + 1 error)
        assert!((state.error_rate() - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_health_calculation() {
        let mut state = BaseWatcherState::new("test");

        // Stopped
        assert_eq!(state.calculate_health(), WatcherHealth::Stopped);

        state.running = true;
        state.record_observation(10);

        // Healthy (no errors)
        assert_eq!(state.calculate_health(), WatcherHealth::Healthy);

        // Add errors to make it degraded
        for _ in 0..2 {
            state.record_error();
        }

        // Should be degraded (2 errors / 3 total = 66%)
        assert_eq!(state.calculate_health(), WatcherHealth::Failing);
    }

    #[test]
    fn test_filter_matching() {
        let filters = vec!["src/**/*.rs".to_string(), "*.json".to_string()];

        assert!(BaseWatcherState::matches_filters("src/main.rs", &filters));
        assert!(BaseWatcherState::matches_filters("config.json", &filters));
        assert!(!BaseWatcherState::matches_filters("README.md", &filters));

        // Empty filters = match all
        assert!(BaseWatcherState::matches_filters("anything", &[]));
    }

    #[test]
    fn test_generate_id() {
        let id1 = BaseWatcherState::generate_id("code");
        let id2 = BaseWatcherState::generate_id("code");

        assert!(id1.starts_with("code_"));
        assert!(id2.starts_with("code_"));
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_watcher_health_serialization() {
        let health = WatcherHealth::Healthy;
        let json = serde_json::to_string(&health).unwrap();
        assert_eq!(json, "\"healthy\"");
    }
}
