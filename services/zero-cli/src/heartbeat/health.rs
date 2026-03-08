//! Service health status types for heartbeat monitoring.
//!
//! This module defines the health state model for tracked services,
//! following the principle: **deterministic health checks in Rust**.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Health status of a monitored service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HealthStatus {
    /// Service is responding normally
    Healthy,
    /// Service is responding but with issues
    Degraded { reason: String },
    /// Service is not responding
    Unhealthy {
        reason: String,
        last_seen: DateTime<Utc>,
    },
    /// Service status is unknown (not yet checked)
    Unknown,
}

impl HealthStatus {
    /// Returns true if the service is considered operational.
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded { .. })
    }

    /// Returns the status name for logging/display.
    pub fn name(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded { .. } => "degraded",
            HealthStatus::Unhealthy { .. } => "unhealthy",
            HealthStatus::Unknown => "unknown",
        }
    }
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Unknown
    }
}

/// Health information for a single service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Unique identifier for the service
    pub service_id: String,
    /// Current health status
    pub status: HealthStatus,
    /// Timestamp of the last successful heartbeat
    pub last_heartbeat: Option<DateTime<Utc>>,
    /// Number of consecutive heartbeat failures
    pub consecutive_failures: u32,
    /// Timestamp when monitoring started for this service
    pub monitored_since: DateTime<Utc>,
    /// Optional endpoint URL for health checks
    pub health_endpoint: Option<String>,
}

impl ServiceHealth {
    /// Create a new service health entry.
    pub fn new(service_id: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
            status: HealthStatus::Unknown,
            last_heartbeat: None,
            consecutive_failures: 0,
            monitored_since: Utc::now(),
            health_endpoint: None,
        }
    }

    /// Create a new service health entry with a health endpoint.
    pub fn with_endpoint(service_id: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
            status: HealthStatus::Unknown,
            last_heartbeat: None,
            consecutive_failures: 0,
            monitored_since: Utc::now(),
            health_endpoint: Some(endpoint.into()),
        }
    }

    /// Record a successful heartbeat.
    pub fn record_success(&mut self) {
        self.status = HealthStatus::Healthy;
        self.last_heartbeat = Some(Utc::now());
        self.consecutive_failures = 0;
    }

    /// Record a failed heartbeat.
    pub fn record_failure(&mut self, reason: impl Into<String>) {
        self.consecutive_failures += 1;
        let last_seen = self.last_heartbeat.unwrap_or(self.monitored_since);
        self.status = HealthStatus::Unhealthy {
            reason: reason.into(),
            last_seen,
        };
    }

    /// Mark the service as degraded.
    pub fn mark_degraded(&mut self, reason: impl Into<String>) {
        self.status = HealthStatus::Degraded {
            reason: reason.into(),
        };
        // Don't reset consecutive_failures - degraded is still a problem
    }

    /// Returns true if the service has exceeded the failure threshold.
    pub fn exceeds_threshold(&self, max_failures: u32) -> bool {
        self.consecutive_failures >= max_failures
    }

    /// Returns the duration since the last successful heartbeat.
    pub fn time_since_heartbeat(&self) -> Option<chrono::Duration> {
        self.last_heartbeat.map(|t| Utc::now() - t)
    }
}

/// Aggregate health status for all monitored services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall system status
    pub status: HealthStatus,
    /// Individual service health entries
    pub services: Vec<ServiceHealth>,
    /// Timestamp of the last health check
    pub checked_at: DateTime<Utc>,
}

impl SystemHealth {
    /// Compute system health from individual service health entries.
    pub fn from_services(services: Vec<ServiceHealth>) -> Self {
        let status = Self::compute_status(&services);
        Self {
            status,
            services,
            checked_at: Utc::now(),
        }
    }

    /// Compute aggregate status from service health entries.
    fn compute_status(services: &[ServiceHealth]) -> HealthStatus {
        if services.is_empty() {
            return HealthStatus::Unknown;
        }

        let unhealthy_count = services
            .iter()
            .filter(|s| matches!(s.status, HealthStatus::Unhealthy { .. }))
            .count();

        let degraded_count = services
            .iter()
            .filter(|s| matches!(s.status, HealthStatus::Degraded { .. }))
            .count();

        if unhealthy_count > 0 {
            HealthStatus::Unhealthy {
                reason: format!("{} service(s) unhealthy", unhealthy_count),
                last_seen: Utc::now(),
            }
        } else if degraded_count > 0 {
            HealthStatus::Degraded {
                reason: format!("{} service(s) degraded", degraded_count),
            }
        } else {
            HealthStatus::Healthy
        }
    }

    /// Returns the count of healthy services.
    pub fn healthy_count(&self) -> usize {
        self.services
            .iter()
            .filter(|s| matches!(s.status, HealthStatus::Healthy))
            .count()
    }

    /// Returns the count of unhealthy services.
    pub fn unhealthy_count(&self) -> usize {
        self.services
            .iter()
            .filter(|s| matches!(s.status, HealthStatus::Unhealthy { .. }))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_is_operational() {
        assert!(HealthStatus::Healthy.is_operational());
        assert!(HealthStatus::Degraded {
            reason: "slow".into()
        }
        .is_operational());
        assert!(!HealthStatus::Unhealthy {
            reason: "down".into(),
            last_seen: Utc::now()
        }
        .is_operational());
        assert!(!HealthStatus::Unknown.is_operational());
    }

    #[test]
    fn test_service_health_record_success() {
        let mut health = ServiceHealth::new("test-service");
        health.record_failure("timeout");
        health.record_failure("timeout");
        assert_eq!(health.consecutive_failures, 2);

        health.record_success();
        assert_eq!(health.consecutive_failures, 0);
        assert!(matches!(health.status, HealthStatus::Healthy));
        assert!(health.last_heartbeat.is_some());
    }

    #[test]
    fn test_service_health_exceeds_threshold() {
        let mut health = ServiceHealth::new("test-service");
        assert!(!health.exceeds_threshold(3));

        health.record_failure("error 1");
        health.record_failure("error 2");
        assert!(!health.exceeds_threshold(3));

        health.record_failure("error 3");
        assert!(health.exceeds_threshold(3));
    }

    #[test]
    fn test_system_health_from_services() {
        let mut healthy = ServiceHealth::new("healthy-service");
        healthy.record_success();

        let mut unhealthy = ServiceHealth::new("unhealthy-service");
        unhealthy.record_failure("down");

        let system = SystemHealth::from_services(vec![healthy, unhealthy]);
        assert!(matches!(system.status, HealthStatus::Unhealthy { .. }));
        assert_eq!(system.healthy_count(), 1);
        assert_eq!(system.unhealthy_count(), 1);
    }

    #[test]
    fn test_system_health_all_healthy() {
        let mut s1 = ServiceHealth::new("s1");
        s1.record_success();
        let mut s2 = ServiceHealth::new("s2");
        s2.record_success();

        let system = SystemHealth::from_services(vec![s1, s2]);
        assert!(matches!(system.status, HealthStatus::Healthy));
    }

    #[test]
    fn test_system_health_empty() {
        let system = SystemHealth::from_services(vec![]);
        assert!(matches!(system.status, HealthStatus::Unknown));
    }

    #[test]
    fn test_health_status_serialization() {
        let healthy = HealthStatus::Healthy;
        let json = serde_json::to_string(&healthy).unwrap();
        assert!(json.contains("\"status\":\"healthy\""));

        let degraded = HealthStatus::Degraded {
            reason: "slow response".into(),
        };
        let json = serde_json::to_string(&degraded).unwrap();
        assert!(json.contains("\"status\":\"degraded\""));
        assert!(json.contains("slow response"));
    }

    #[test]
    fn test_service_health_with_endpoint() {
        let health = ServiceHealth::with_endpoint("api", "http://localhost:4400/health");
        assert_eq!(health.service_id, "api");
        assert_eq!(
            health.health_endpoint,
            Some("http://localhost:4400/health".to_string())
        );
    }

    #[test]
    fn test_mark_degraded() {
        let mut health = ServiceHealth::new("test");
        health.record_success();
        health.mark_degraded("high latency");

        assert!(matches!(health.status, HealthStatus::Degraded { .. }));
        // last_heartbeat should still be set from record_success
        assert!(health.last_heartbeat.is_some());
    }
}
