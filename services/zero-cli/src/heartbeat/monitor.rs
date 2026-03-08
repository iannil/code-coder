//! Service health monitoring system.
//!
//! This module provides the `HeartbeatMonitor` which periodically checks
//! service health and takes configurable actions on stale services.
//!
//! ## Design Principle
//!
//! This is a **deterministic** component in Rust - it handles predictable
//! health checks and actions, while the TypeScript layer handles
//! decisions requiring LLM reasoning.

use crate::observability::{Observer, ObserverEvent};
use anyhow::Result;
use chrono::Duration as ChronoDuration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};

use super::health::{HealthStatus, ServiceHealth, SystemHealth};

/// Action to take when a service becomes stale.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum StaleAction {
    /// Attempt to restart the service
    Restart,
    /// Send alerts to specified channels
    Alert { channels: Vec<String> },
    /// Escalate to specified users/roles
    Escalate { to: Vec<String> },
    /// Log only, no action
    LogOnly,
}

impl Default for StaleAction {
    fn default() -> Self {
        StaleAction::LogOnly
    }
}

/// Configuration for the heartbeat monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Interval between health checks
    #[serde(with = "humantime_serde", default = "default_interval")]
    pub interval: Duration,
    /// Timeout for individual health checks
    #[serde(with = "humantime_serde", default = "default_timeout")]
    pub timeout: Duration,
    /// Maximum missed heartbeats before marking unhealthy
    #[serde(default = "default_max_missed")]
    pub max_missed: u32,
    /// Action to take on stale services
    #[serde(default)]
    pub on_stale: StaleAction,
    /// Whether the monitor is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_interval() -> Duration {
    Duration::from_secs(30)
}

fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

fn default_max_missed() -> u32 {
    3
}

fn default_enabled() -> bool {
    true
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            interval: default_interval(),
            timeout: default_timeout(),
            max_missed: default_max_missed(),
            on_stale: StaleAction::default(),
            enabled: default_enabled(),
        }
    }
}

/// Service registration for monitoring.
#[derive(Debug, Clone)]
pub struct ServiceRegistration {
    /// Service identifier
    pub service_id: String,
    /// Health check endpoint (HTTP GET)
    pub health_endpoint: Option<String>,
    /// Custom action for this service (overrides global)
    pub on_stale: Option<StaleAction>,
}

/// The main heartbeat monitor for service health.
pub struct HeartbeatMonitor {
    config: MonitorConfig,
    services: Arc<RwLock<HashMap<String, ServiceHealth>>>,
    registrations: Arc<RwLock<HashMap<String, ServiceRegistration>>>,
    observer: Arc<dyn Observer>,
    http_client: reqwest::Client,
}

impl HeartbeatMonitor {
    /// Create a new heartbeat monitor.
    pub fn new(config: MonitorConfig, observer: Arc<dyn Observer>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_default();

        Self {
            config,
            services: Arc::new(RwLock::new(HashMap::new())),
            registrations: Arc::new(RwLock::new(HashMap::new())),
            observer,
            http_client,
        }
    }

    /// Register a service for monitoring.
    pub async fn register(&self, registration: ServiceRegistration) {
        let service_id = registration.service_id.clone();
        let health = match &registration.health_endpoint {
            Some(endpoint) => ServiceHealth::with_endpoint(&service_id, endpoint),
            None => ServiceHealth::new(&service_id),
        };

        let mut services = self.services.write().await;
        let mut registrations = self.registrations.write().await;

        services.insert(service_id.clone(), health);
        registrations.insert(service_id.clone(), registration);

        info!("Registered service for monitoring: {}", service_id);
    }

    /// Unregister a service from monitoring.
    pub async fn unregister(&self, service_id: &str) {
        let mut services = self.services.write().await;
        let mut registrations = self.registrations.write().await;

        services.remove(service_id);
        registrations.remove(service_id);

        info!("Unregistered service from monitoring: {}", service_id);
    }

    /// Record a heartbeat from a service (push model).
    pub async fn record_heartbeat(&self, service_id: &str) {
        let mut services = self.services.write().await;

        if let Some(health) = services.get_mut(service_id) {
            let previous = health.status.clone();
            health.record_success();

            if previous != health.status {
                self.emit_health_change(service_id, previous, health.status.clone());
            }

            debug!("Recorded heartbeat for: {}", service_id);
        } else {
            // Auto-register unknown services
            let mut health = ServiceHealth::new(service_id);
            health.record_success();
            services.insert(service_id.to_string(), health);
            info!("Auto-registered and recorded heartbeat for: {}", service_id);
        }
    }

    /// Get health status for a specific service.
    pub async fn get_health(&self, service_id: &str) -> Option<ServiceHealth> {
        let services = self.services.read().await;
        services.get(service_id).cloned()
    }

    /// Get aggregate health for all monitored services.
    pub async fn get_system_health(&self) -> SystemHealth {
        let services = self.services.read().await;
        SystemHealth::from_services(services.values().cloned().collect())
    }

    /// Start the monitoring loop (runs until cancelled).
    pub async fn run(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Heartbeat monitor disabled");
            return Ok(());
        }

        info!(
            "Starting heartbeat monitor: interval={:?}, max_missed={}",
            self.config.interval, self.config.max_missed
        );

        let mut interval = time::interval(self.config.interval);

        loop {
            interval.tick().await;
            self.check_all_services().await;
        }
    }

    /// Perform a single health check pass on all services.
    async fn check_all_services(&self) {
        let registrations = self.registrations.read().await;

        for (service_id, registration) in registrations.iter() {
            if let Some(endpoint) = &registration.health_endpoint {
                self.check_service_health(service_id, endpoint).await;
            } else {
                // For services without endpoints, check time since last heartbeat
                self.check_heartbeat_timeout(service_id).await;
            }
        }

        // Handle stale services
        self.process_stale_services().await;
    }

    /// Check service health via HTTP endpoint.
    async fn check_service_health(&self, service_id: &str, endpoint: &str) {
        let result = self.http_client.get(endpoint).send().await;

        let mut services = self.services.write().await;
        let health = services
            .entry(service_id.to_string())
            .or_insert_with(|| ServiceHealth::with_endpoint(service_id, endpoint));

        let previous = health.status.clone();

        match result {
            Ok(response) if response.status().is_success() => {
                health.record_success();
            }
            Ok(response) => {
                health.record_failure(format!("HTTP {}", response.status()));
            }
            Err(e) => {
                health.record_failure(e.to_string());
            }
        }

        if previous != health.status {
            self.emit_health_change(service_id, previous, health.status.clone());
        }
    }

    /// Check if a service has missed heartbeats (push model timeout).
    async fn check_heartbeat_timeout(&self, service_id: &str) {
        let mut services = self.services.write().await;

        if let Some(health) = services.get_mut(service_id) {
            if let Some(duration) = health.time_since_heartbeat() {
                let threshold = ChronoDuration::from_std(self.config.interval * self.config.max_missed)
                    .unwrap_or_else(|_| ChronoDuration::seconds(90));

                if duration > threshold {
                    let previous = health.status.clone();
                    health.record_failure("heartbeat timeout");

                    if previous != health.status {
                        self.emit_health_change(service_id, previous, health.status.clone());
                    }
                }
            }
        }
    }

    /// Process services that have exceeded the failure threshold.
    async fn process_stale_services(&self) {
        let services = self.services.read().await;
        let registrations = self.registrations.read().await;

        for (service_id, health) in services.iter() {
            if health.exceeds_threshold(self.config.max_missed) {
                let action = registrations
                    .get(service_id)
                    .and_then(|r| r.on_stale.as_ref())
                    .unwrap_or(&self.config.on_stale);

                self.handle_stale_service(service_id, health, action).await;
            }
        }
    }

    /// Handle a stale service according to configured action.
    async fn handle_stale_service(
        &self,
        service_id: &str,
        health: &ServiceHealth,
        action: &StaleAction,
    ) {
        match action {
            StaleAction::Restart => {
                warn!(
                    "Service {} is stale ({} failures), would restart",
                    service_id, health.consecutive_failures
                );
                self.observer.record_event(&ObserverEvent::ServiceRestart {
                    service_id: service_id.to_string(),
                });
                // Actual restart logic would be implemented here
            }
            StaleAction::Alert { channels } => {
                warn!(
                    "Service {} is stale, alerting channels: {:?}",
                    service_id, channels
                );
                self.observer.record_event(&ObserverEvent::Alert {
                    service_id: service_id.to_string(),
                    channels: channels.clone(),
                });
            }
            StaleAction::Escalate { to } => {
                error!(
                    "Service {} is stale, escalating to: {:?}",
                    service_id, to
                );
                self.observer.record_event(&ObserverEvent::Escalation {
                    service_id: service_id.to_string(),
                    escalate_to: to.clone(),
                });
            }
            StaleAction::LogOnly => {
                warn!(
                    "Service {} is stale ({} consecutive failures)",
                    service_id, health.consecutive_failures
                );
            }
        }
    }

    /// Emit a health change event.
    fn emit_health_change(&self, service_id: &str, previous: HealthStatus, current: HealthStatus) {
        match (&previous, &current) {
            (HealthStatus::Healthy, HealthStatus::Unhealthy { .. }) => {
                warn!("Service {} became unhealthy", service_id);
            }
            (HealthStatus::Unhealthy { .. }, HealthStatus::Healthy) => {
                info!("Service {} recovered", service_id);
            }
            _ => {
                debug!("Service {} health changed: {:?} -> {:?}", service_id, previous, current);
            }
        }

        self.observer
            .record_event(&ObserverEvent::HealthChange {
                service_id: service_id.to_string(),
                previous_status: previous.name().to_string(),
                current_status: current.name().to_string(),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::NoopObserver;

    fn create_test_monitor() -> HeartbeatMonitor {
        let config = MonitorConfig::default();
        let observer: Arc<dyn Observer> = Arc::new(NoopObserver);
        HeartbeatMonitor::new(config, observer)
    }

    #[tokio::test]
    async fn test_register_and_get_health() {
        let monitor = create_test_monitor();

        let registration = ServiceRegistration {
            service_id: "test-service".to_string(),
            health_endpoint: None,
            on_stale: None,
        };
        monitor.register(registration).await;

        let health = monitor.get_health("test-service").await;
        assert!(health.is_some());
        assert!(matches!(
            health.unwrap().status,
            HealthStatus::Unknown
        ));
    }

    #[tokio::test]
    async fn test_record_heartbeat() {
        let monitor = create_test_monitor();

        let registration = ServiceRegistration {
            service_id: "test-service".to_string(),
            health_endpoint: None,
            on_stale: None,
        };
        monitor.register(registration).await;

        monitor.record_heartbeat("test-service").await;

        let health = monitor.get_health("test-service").await.unwrap();
        assert!(matches!(health.status, HealthStatus::Healthy));
        assert!(health.last_heartbeat.is_some());
    }

    #[tokio::test]
    async fn test_auto_register_on_heartbeat() {
        let monitor = create_test_monitor();

        // Record heartbeat for unregistered service
        monitor.record_heartbeat("auto-service").await;

        let health = monitor.get_health("auto-service").await;
        assert!(health.is_some());
        assert!(matches!(health.unwrap().status, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_unregister() {
        let monitor = create_test_monitor();

        let registration = ServiceRegistration {
            service_id: "temp-service".to_string(),
            health_endpoint: None,
            on_stale: None,
        };
        monitor.register(registration).await;
        assert!(monitor.get_health("temp-service").await.is_some());

        monitor.unregister("temp-service").await;
        assert!(monitor.get_health("temp-service").await.is_none());
    }

    #[tokio::test]
    async fn test_system_health() {
        let monitor = create_test_monitor();

        monitor.record_heartbeat("service-1").await;
        monitor.record_heartbeat("service-2").await;

        let system = monitor.get_system_health().await;
        assert_eq!(system.healthy_count(), 2);
        assert!(matches!(system.status, HealthStatus::Healthy));
    }

    #[test]
    fn test_monitor_config_defaults() {
        let config = MonitorConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.max_missed, 3);
        assert!(config.enabled);
        assert!(matches!(config.on_stale, StaleAction::LogOnly));
    }

    #[test]
    fn test_stale_action_serialization() {
        let restart = StaleAction::Restart;
        let json = serde_json::to_string(&restart).unwrap();
        assert!(json.contains("\"action\":\"restart\""));

        let alert = StaleAction::Alert {
            channels: vec!["telegram".to_string()],
        };
        let json = serde_json::to_string(&alert).unwrap();
        assert!(json.contains("\"action\":\"alert\""));
        assert!(json.contains("telegram"));
    }

    #[test]
    fn test_monitor_config_serialization() {
        let json = r#"{
            "interval": "1m",
            "timeout": "10s",
            "max_missed": 5,
            "on_stale": {"action": "restart"},
            "enabled": true
        }"#;

        let config: MonitorConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.interval, Duration::from_secs(60));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_missed, 5);
        assert!(matches!(config.on_stale, StaleAction::Restart));
    }
}
