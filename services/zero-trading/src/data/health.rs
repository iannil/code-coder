//! Health monitoring for data providers.
//!
//! Tracks provider health status and manages background health checks
//! to enable fast failover decisions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::provider::DataProvider;

// ============================================================================
// Health Status
// ============================================================================

/// Health status for a single provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Provider name
    pub name: String,
    /// Whether the provider is currently healthy
    pub healthy: bool,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Last health check timestamp
    pub last_check: Option<DateTime<Utc>>,
    /// Last successful check timestamp
    pub last_success: Option<DateTime<Utc>>,
    /// Last error message (if unhealthy)
    pub last_error: Option<String>,
    /// Total health checks performed
    pub total_checks: u64,
    /// Total successful checks
    pub successful_checks: u64,
}

impl ProviderHealth {
    /// Create new health status for a provider
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            healthy: true, // Assume healthy until proven otherwise
            consecutive_failures: 0,
            last_check: None,
            last_success: None,
            last_error: None,
            total_checks: 0,
            successful_checks: 0,
        }
    }

    /// Record a successful health check
    pub fn record_success(&mut self) {
        self.healthy = true;
        self.consecutive_failures = 0;
        self.last_check = Some(Utc::now());
        self.last_success = Some(Utc::now());
        self.last_error = None;
        self.total_checks += 1;
        self.successful_checks += 1;
    }

    /// Record a failed health check
    pub fn record_failure(&mut self, error: &str, unhealthy_threshold: u32) {
        self.consecutive_failures += 1;
        self.last_check = Some(Utc::now());
        self.last_error = Some(error.to_string());
        self.total_checks += 1;

        // Mark unhealthy after threshold consecutive failures
        if self.consecutive_failures >= unhealthy_threshold {
            if self.healthy {
                warn!(
                    provider = %self.name,
                    failures = self.consecutive_failures,
                    "Provider marked unhealthy after {} consecutive failures",
                    unhealthy_threshold
                );
            }
            self.healthy = false;
        }
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_checks == 0 {
            100.0
        } else {
            (self.successful_checks as f64 / self.total_checks as f64) * 100.0
        }
    }
}

// ============================================================================
// Health Monitor Configuration
// ============================================================================

/// Configuration for the health monitor.
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    /// Interval between health checks in seconds
    pub check_interval_secs: u64,
    /// Number of consecutive failures before marking unhealthy
    pub unhealthy_threshold: u32,
    /// Timeout for health check requests in seconds
    pub check_timeout_secs: u64,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            unhealthy_threshold: 3,
            check_timeout_secs: 10,
        }
    }
}

// ============================================================================
// Health Monitor
// ============================================================================

/// Monitors health of multiple data providers.
///
/// Runs background health checks and maintains a real-time view of
/// which providers are available for use.
pub struct HealthMonitor {
    /// Health status for each provider (keyed by name)
    health: Arc<RwLock<HashMap<String, ProviderHealth>>>,
    /// Configuration
    config: HealthMonitorConfig,
    /// Flag to stop the background monitor
    stop_flag: Arc<RwLock<bool>>,
}

impl HealthMonitor {
    /// Create a new health monitor with default config
    pub fn new() -> Self {
        Self::with_config(HealthMonitorConfig::default())
    }

    /// Create a new health monitor with custom config
    pub fn with_config(config: HealthMonitorConfig) -> Self {
        Self {
            health: Arc::new(RwLock::new(HashMap::new())),
            config,
            stop_flag: Arc::new(RwLock::new(false)),
        }
    }

    /// Register a provider for health monitoring
    pub async fn register_provider(&self, name: &str) {
        let mut health = self.health.write().await;
        if !health.contains_key(name) {
            health.insert(name.to_string(), ProviderHealth::new(name));
            debug!(provider = name, "Registered provider for health monitoring");
        }
    }

    /// Unregister a provider from health monitoring
    pub async fn unregister_provider(&self, name: &str) {
        let mut health = self.health.write().await;
        health.remove(name);
        debug!(provider = name, "Unregistered provider from health monitoring");
    }

    /// Check if a provider is currently healthy
    pub async fn is_healthy(&self, name: &str) -> bool {
        let health = self.health.read().await;
        health.get(name).map(|h| h.healthy).unwrap_or(false)
    }

    /// Get health status for a provider
    pub async fn get_health(&self, name: &str) -> Option<ProviderHealth> {
        let health = self.health.read().await;
        health.get(name).cloned()
    }

    /// Get health status for all providers
    pub async fn get_all_health(&self) -> HashMap<String, ProviderHealth> {
        let health = self.health.read().await;
        health.clone()
    }

    /// Get list of healthy providers sorted by some criteria
    pub async fn healthy_providers(&self) -> Vec<String> {
        let health = self.health.read().await;
        health
            .iter()
            .filter(|(_, h)| h.healthy)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Record a successful request to a provider
    pub async fn record_success(&self, name: &str) {
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(name) {
            h.record_success();
        }
    }

    /// Record a failed request to a provider
    pub async fn record_failure(&self, name: &str, error: &str) {
        let mut health = self.health.write().await;
        if let Some(h) = health.get_mut(name) {
            h.record_failure(error, self.config.unhealthy_threshold);
        }
    }

    /// Perform a health check on a single provider
    pub async fn check_provider<P: DataProvider + ?Sized>(&self, provider: &P) {
        let name = provider.name();
        let timeout = Duration::from_secs(self.config.check_timeout_secs);

        let check_result = tokio::time::timeout(timeout, provider.health_check()).await;

        let mut health = self.health.write().await;
        let status = health
            .entry(name.to_string())
            .or_insert_with(|| ProviderHealth::new(name));

        match check_result {
            Ok(Ok(())) => {
                if !status.healthy {
                    info!(provider = name, "Provider recovered and is now healthy");
                }
                status.record_success();
            }
            Ok(Err(e)) => {
                status.record_failure(&e.to_string(), self.config.unhealthy_threshold);
                debug!(provider = name, error = %e, "Provider health check failed");
            }
            Err(_) => {
                status.record_failure("Health check timed out", self.config.unhealthy_threshold);
                debug!(provider = name, "Provider health check timed out");
            }
        }
    }

    /// Start background health monitoring for multiple providers.
    ///
    /// This spawns a background task that periodically checks all providers.
    pub fn start_background_monitor<P>(
        self: Arc<Self>,
        providers: Arc<Vec<Arc<P>>>,
    ) -> tokio::task::JoinHandle<()>
    where
        P: DataProvider + 'static,
    {
        let monitor = self;

        tokio::spawn(async move {
            info!(
                interval_secs = monitor.config.check_interval_secs,
                "Starting background health monitor"
            );

            loop {
                // Check if we should stop
                if *monitor.stop_flag.read().await {
                    info!("Health monitor stopping");
                    break;
                }

                // Check all providers
                for provider in providers.iter() {
                    monitor.check_provider(provider.as_ref()).await;
                }

                // Sleep until next check
                tokio::time::sleep(Duration::from_secs(monitor.config.check_interval_secs)).await;
            }
        })
    }

    /// Start background health monitoring with boxed providers (type-erased)
    pub fn start_background_monitor_dyn(
        self: Arc<Self>,
        providers: Arc<Vec<Arc<dyn DataProvider>>>,
    ) -> tokio::task::JoinHandle<()> {
        let monitor = self;

        tokio::spawn(async move {
            info!(
                interval_secs = monitor.config.check_interval_secs,
                "Starting background health monitor (dyn)"
            );

            loop {
                // Check if we should stop
                if *monitor.stop_flag.read().await {
                    info!("Health monitor stopping");
                    break;
                }

                // Check all providers
                for provider in providers.iter() {
                    monitor.check_provider(provider.as_ref()).await;
                }

                // Sleep until next check
                tokio::time::sleep(Duration::from_secs(monitor.config.check_interval_secs)).await;
            }
        })
    }

    /// Stop the background monitor
    pub async fn stop(&self) {
        let mut flag = self.stop_flag.write().await;
        *flag = true;
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_health_new() {
        let health = ProviderHealth::new("test");
        assert!(health.healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_check.is_none());
    }

    #[test]
    fn test_provider_health_record_success() {
        let mut health = ProviderHealth::new("test");
        health.consecutive_failures = 2;
        health.healthy = false;

        health.record_success();

        assert!(health.healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_check.is_some());
        assert!(health.last_success.is_some());
        assert_eq!(health.total_checks, 1);
        assert_eq!(health.successful_checks, 1);
    }

    #[test]
    fn test_provider_health_record_failure_below_threshold() {
        let mut health = ProviderHealth::new("test");

        health.record_failure("error 1", 3);
        assert!(health.healthy);
        assert_eq!(health.consecutive_failures, 1);

        health.record_failure("error 2", 3);
        assert!(health.healthy);
        assert_eq!(health.consecutive_failures, 2);
    }

    #[test]
    fn test_provider_health_record_failure_at_threshold() {
        let mut health = ProviderHealth::new("test");

        health.record_failure("error 1", 3);
        health.record_failure("error 2", 3);
        health.record_failure("error 3", 3);

        assert!(!health.healthy);
        assert_eq!(health.consecutive_failures, 3);
        assert_eq!(health.last_error, Some("error 3".to_string()));
    }

    #[test]
    fn test_provider_health_success_rate() {
        let mut health = ProviderHealth::new("test");
        assert_eq!(health.success_rate(), 100.0);

        health.record_success();
        health.record_success();
        health.record_failure("error", 3);
        health.record_success();

        // 3 successes out of 4 checks = 75%
        assert!((health.success_rate() - 75.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_health_monitor_register() {
        let monitor = HealthMonitor::new();

        monitor.register_provider("test").await;
        assert!(monitor.is_healthy("test").await);

        monitor.unregister_provider("test").await;
        assert!(!monitor.is_healthy("test").await);
    }

    #[tokio::test]
    async fn test_health_monitor_record_events() {
        let monitor = HealthMonitor::new();
        monitor.register_provider("test").await;

        monitor.record_success("test").await;
        let health = monitor.get_health("test").await.unwrap();
        assert!(health.healthy);
        assert_eq!(health.successful_checks, 1);

        monitor.record_failure("test", "error").await;
        monitor.record_failure("test", "error").await;
        monitor.record_failure("test", "error").await;

        let health = monitor.get_health("test").await.unwrap();
        assert!(!health.healthy);
        assert_eq!(health.consecutive_failures, 3);
    }

    #[tokio::test]
    async fn test_health_monitor_healthy_providers() {
        let monitor = HealthMonitor::new();
        monitor.register_provider("provider1").await;
        monitor.register_provider("provider2").await;

        // Make provider2 unhealthy
        for _ in 0..3 {
            monitor.record_failure("provider2", "error").await;
        }

        let healthy = monitor.healthy_providers().await;
        assert_eq!(healthy.len(), 1);
        assert!(healthy.contains(&"provider1".to_string()));
    }
}
