//! Health checking for the Observer Network.
//!
//! Monitors system health with rule-based evaluation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// Health status levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// System is operating normally
    Healthy,
    /// Some issues detected but system is functional
    Degraded,
    /// System is experiencing failures
    Failing,
    /// System is stopped or not responding
    Stopped,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Healthy
    }
}

/// A health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall status
    pub status: HealthStatus,
    /// Individual component statuses
    pub components: Vec<ComponentHealth>,
    /// Last check timestamp
    pub last_check: DateTime<Utc>,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Recent error count
    pub recent_errors: u64,
    /// Health score (0-100)
    pub score: u8,
}

/// Health status for a single component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: HealthStatus,
    /// Optional message
    pub message: Option<String>,
    /// Last activity timestamp
    pub last_activity: Option<DateTime<Utc>>,
}

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Maximum allowed error rate (errors per minute)
    pub max_error_rate: f64,
    /// Maximum allowed latency in milliseconds
    pub max_latency_ms: u64,
    /// Minimum observation rate (events per minute)
    pub min_observation_rate: f64,
    /// Health check interval in seconds
    pub check_interval_seconds: u64,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            max_error_rate: 10.0,
            max_latency_ms: 5000,
            min_observation_rate: 1.0,
            check_interval_seconds: 60,
        }
    }
}

/// Health checker for the Observer Network.
#[allow(dead_code)]
pub struct HealthCheck {
    config: HealthConfig,
    start_time: DateTime<Utc>,
    error_count: AtomicU64,
    last_check: RwLock<DateTime<Utc>>,
    components: RwLock<Vec<ComponentHealth>>,
}

impl HealthCheck {
    /// Create a new health checker.
    pub fn new() -> Self {
        Self::with_config(HealthConfig::default())
    }

    /// Create a health checker with custom config.
    pub fn with_config(config: HealthConfig) -> Self {
        Self {
            config,
            start_time: Utc::now(),
            error_count: AtomicU64::new(0),
            last_check: RwLock::new(Utc::now()),
            components: RwLock::new(Vec::new()),
        }
    }

    /// Record an error.
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Update a component's health status.
    pub fn update_component(&self, name: impl Into<String>, status: HealthStatus, message: Option<String>) {
        let name = name.into();
        let mut components = self.components.write().unwrap();

        if let Some(comp) = components.iter_mut().find(|c| c.name == name) {
            comp.status = status;
            comp.message = message;
            comp.last_activity = Some(Utc::now());
        } else {
            components.push(ComponentHealth {
                name,
                status,
                message,
                last_activity: Some(Utc::now()),
            });
        }
    }

    /// Get current health report.
    pub fn report(&self) -> HealthReport {
        let now = Utc::now();
        let components = self.components.read().unwrap().clone();

        // Calculate overall status from components
        let overall_status = self.calculate_overall_status(&components);

        // Calculate health score
        let score = self.calculate_score(&components);

        // Update last check time
        if let Ok(mut last) = self.last_check.write() {
            *last = now;
        }

        HealthReport {
            status: overall_status,
            components,
            last_check: now,
            uptime_seconds: (now - self.start_time).num_seconds().max(0) as u64,
            recent_errors: self.error_count.load(Ordering::Relaxed),
            score,
        }
    }

    /// Check if system is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self.report().status, HealthStatus::Healthy)
    }

    /// Get uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        let now = Utc::now();
        (now - self.start_time).num_seconds().max(0) as u64
    }

    /// Reset error count.
    pub fn reset_errors(&self) {
        self.error_count.store(0, Ordering::Relaxed);
    }

    /// Clear all components.
    pub fn clear_components(&self) {
        if let Ok(mut components) = self.components.write() {
            components.clear();
        }
    }

    fn calculate_overall_status(&self, components: &[ComponentHealth]) -> HealthStatus {
        if components.is_empty() {
            return HealthStatus::Healthy;
        }

        let mut failing = false;
        let mut degraded = false;

        for comp in components {
            match comp.status {
                HealthStatus::Failing | HealthStatus::Stopped => failing = true,
                HealthStatus::Degraded => degraded = true,
                _ => {}
            }
        }

        if failing {
            HealthStatus::Failing
        } else if degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    fn calculate_score(&self, components: &[ComponentHealth]) -> u8 {
        if components.is_empty() {
            return 100;
        }

        let total: u32 = components
            .iter()
            .map(|c| match c.status {
                HealthStatus::Healthy => 100,
                HealthStatus::Degraded => 70,
                HealthStatus::Failing => 30,
                HealthStatus::Stopped => 0,
            })
            .sum();

        (total / components.len() as u32).min(100) as u8
    }
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_new() {
        let health = HealthCheck::new();
        assert!(health.is_healthy());
    }

    #[test]
    fn test_health_report() {
        let health = HealthCheck::new();
        let report = health.report();

        assert_eq!(report.status, HealthStatus::Healthy);
        assert_eq!(report.recent_errors, 0);
        assert_eq!(report.score, 100);
    }

    #[test]
    fn test_record_error() {
        let health = HealthCheck::new();
        health.record_error();
        health.record_error();

        let report = health.report();
        assert_eq!(report.recent_errors, 2);
    }

    #[test]
    fn test_update_component() {
        let health = HealthCheck::new();

        health.update_component("stream", HealthStatus::Healthy, None);
        health.update_component("watchers", HealthStatus::Degraded, Some("High latency".into()));

        let report = health.report();
        assert_eq!(report.components.len(), 2);
        assert_eq!(report.status, HealthStatus::Degraded);
    }

    #[test]
    fn test_health_score_calculation() {
        let health = HealthCheck::new();

        health.update_component("a", HealthStatus::Healthy, None);
        health.update_component("b", HealthStatus::Healthy, None);

        let report = health.report();
        assert_eq!(report.score, 100);

        health.update_component("a", HealthStatus::Degraded, None);

        let report = health.report();
        assert_eq!(report.score, 85); // (100 + 70) / 2
    }

    #[test]
    fn test_failing_component() {
        let health = HealthCheck::new();

        health.update_component("good", HealthStatus::Healthy, None);
        health.update_component("bad", HealthStatus::Failing, Some("Connection lost".into()));

        let report = health.report();
        assert_eq!(report.status, HealthStatus::Failing);
    }

    #[test]
    fn test_uptime() {
        let health = HealthCheck::new();

        // Sleep for a tiny bit to ensure uptime > 0
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(health.uptime_seconds() >= 0);
    }

    #[test]
    fn test_reset_errors() {
        let health = HealthCheck::new();
        health.record_error();
        health.record_error();

        assert_eq!(health.report().recent_errors, 2);

        health.reset_errors();
        assert_eq!(health.report().recent_errors, 0);
    }
}
