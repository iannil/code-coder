//! Meta Watcher (MetaWatch)
//!
//! Observes the Observer Network itself including:
//! - Observation quality and coverage
//! - System health
//! - Blind spots in observation
//! - Consensus drift
//! - Watcher performance
//!
//! This implements the "元观察" (meta-observation) aspect,
//! enabling the system to observe its own observation process.

use crate::observer::types::{
    HealthAssessment, HealthStatus, MetaObservation, MetaObservationType, Observation,
    ObserverIssue, Severity, WatcherType,
};
use crate::observer::watchers::{
    BaseWatcherState, Watcher, WatcherHealth, WatcherMetrics, WatcherOptions, WatcherStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{info, warn};

// ══════════════════════════════════════════════════════════════════════════════
// Configuration
// ══════════════════════════════════════════════════════════════════════════════

/// Configuration for MetaWatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaWatchConfig {
    /// Quality threshold for warnings (0-1)
    #[serde(default = "default_quality_threshold")]
    pub quality_threshold: f32,
    /// Coverage threshold for warnings (0-1)
    #[serde(default = "default_coverage_threshold")]
    pub coverage_threshold: f32,
    /// Maximum consensus drift before warning (0-1)
    #[serde(default = "default_consensus_drift")]
    pub max_consensus_drift: f32,
    /// Watched watcher IDs (if empty, watches all)
    #[serde(default)]
    pub watched_watchers: Vec<String>,
    /// Latency threshold in ms
    #[serde(default = "default_latency_threshold")]
    pub latency_threshold: u64,
    /// Common watcher options
    #[serde(flatten)]
    pub options: WatcherOptions,
}

fn default_quality_threshold() -> f32 {
    0.7
}

fn default_coverage_threshold() -> f32 {
    0.6
}

fn default_consensus_drift() -> f32 {
    0.3
}

fn default_latency_threshold() -> u64 {
    1000
}

impl Default for MetaWatchConfig {
    fn default() -> Self {
        Self {
            quality_threshold: 0.7,
            coverage_threshold: 0.6,
            max_consensus_drift: 0.3,
            watched_watchers: Vec::new(),
            latency_threshold: 1000,
            options: WatcherOptions {
                interval_ms: 60000, // Check every minute
                ..Default::default()
            },
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Internal Types
// ══════════════════════════════════════════════════════════════════════════════

/// Metrics for a specific watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherMetricRecord {
    pub watcher_id: String,
    pub watcher_type: WatcherType,
    pub observation_count: u64,
    pub error_rate: f32,
    pub avg_latency: u64,
    pub last_observation: Option<DateTime<Utc>>,
    pub health: WatcherHealth,
}

// ══════════════════════════════════════════════════════════════════════════════
// MetaWatch Implementation
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher that observes the Observer Network itself.
pub struct MetaWatch {
    /// Base watcher state
    state: BaseWatcherState,
    /// Configuration
    config: MetaWatchConfig,
    /// Tracked watcher metrics
    watcher_metrics: HashMap<String, WatcherMetricRecord>,
    /// Last consensus strength
    last_consensus_strength: f32,
    /// Observation counts by watcher type
    observations_by_type: HashMap<WatcherType, u64>,
    /// Pending observations queue
    pending_observations: Vec<Observation>,
    /// Recent confidence samples for quality calculation
    recent_confidences: Vec<f32>,
}

impl MetaWatch {
    /// Create a new MetaWatch instance.
    pub fn new(config: MetaWatchConfig) -> Self {
        let id = config
            .options
            .id
            .clone()
            .unwrap_or_else(|| BaseWatcherState::generate_id("meta"));

        Self {
            state: BaseWatcherState::new(id),
            config,
            watcher_metrics: HashMap::new(),
            last_consensus_strength: 0.5,
            observations_by_type: HashMap::new(),
            pending_observations: Vec::new(),
            recent_confidences: Vec::new(),
        }
    }

    /// Check if a watcher should be monitored.
    fn should_watch(&self, watcher_id: &str) -> bool {
        if self.config.watched_watchers.is_empty() {
            return true;
        }
        self.config.watched_watchers.contains(&watcher_id.to_string())
    }

    /// Initialize metrics for a watcher.
    pub fn register_watcher(&mut self, watcher_id: &str, watcher_type: WatcherType) {
        if !self.should_watch(watcher_id) {
            return;
        }

        self.watcher_metrics.insert(
            watcher_id.to_string(),
            WatcherMetricRecord {
                watcher_id: watcher_id.to_string(),
                watcher_type,
                observation_count: 0,
                error_rate: 0.0,
                avg_latency: 0,
                last_observation: None,
                health: WatcherHealth::Healthy,
            },
        );
    }

    /// Update watcher status.
    pub fn update_watcher_status(&mut self, status: &WatcherStatus) {
        if !self.should_watch(&status.id) {
            return;
        }

        let error_rate = if status.observation_count > 0 {
            status.error_count as f32 / status.observation_count as f32
        } else {
            0.0
        };

        self.watcher_metrics.insert(
            status.id.clone(),
            WatcherMetricRecord {
                watcher_id: status.id.clone(),
                watcher_type: status.watcher_type,
                observation_count: status.observation_count,
                error_rate,
                avg_latency: status.avg_latency_ms,
                last_observation: status.last_observation,
                health: status.health,
            },
        );
    }

    /// Track observation from a watcher type.
    pub fn track_observation(&mut self, watcher_type: WatcherType, confidence: f32) {
        *self.observations_by_type.entry(watcher_type).or_insert(0) += 1;

        self.recent_confidences.push(confidence);
        if self.recent_confidences.len() > 100 {
            self.recent_confidences.remove(0);
        }
    }

    /// Handle consensus change.
    pub fn handle_consensus_change(&mut self, new_strength: f32) {
        let drift = (new_strength - self.last_consensus_strength).abs();

        if drift > self.config.max_consensus_drift {
            let start = Instant::now();

            let severity = if drift > 0.5 {
                Severity::High
            } else {
                Severity::Medium
            };

            let mut obs = self.create_observation(
                MetaObservationType::ConsensusDrift,
                HealthAssessment {
                    health: HealthStatus::Degraded,
                    coverage: self.calculate_coverage(),
                    accuracy: self.calculate_quality(),
                    latency: self.calculate_avg_latency(),
                },
            );

            obs.issues = vec![ObserverIssue {
                issue_type: "consensus_drift".to_string(),
                severity,
                description: format!(
                    "Consensus strength changed from {:.0}% to {:.0}%",
                    self.last_consensus_strength * 100.0,
                    new_strength * 100.0
                ),
            }];

            obs.recommendations = vec![
                "Review recent observations for anomalies".to_string(),
                "Check for external events affecting consensus".to_string(),
                "Consider adjusting attention weights".to_string(),
            ];

            self.pending_observations.push(Observation::Meta(obs));
            self.state
                .record_observation(start.elapsed().as_millis() as u64);
        }

        self.last_consensus_strength = new_strength;
    }

    /// Create a meta observation.
    fn create_observation(
        &self,
        obs_type: MetaObservationType,
        assessment: HealthAssessment,
    ) -> MetaObservation {
        let mut obs = MetaObservation::new(&self.state.id, obs_type);
        obs.assessment = assessment;
        obs
    }

    /// Calculate observation coverage.
    fn calculate_coverage(&self) -> f32 {
        // Check if we have observations from the three main watcher types
        let types = [WatcherType::Code, WatcherType::World, WatcherType::Self_];
        let covered = types
            .iter()
            .filter(|t| self.observations_by_type.get(t).copied().unwrap_or(0) > 0)
            .count();
        covered as f32 / types.len() as f32
    }

    /// Calculate observation quality (average confidence).
    fn calculate_quality(&self) -> f32 {
        if self.recent_confidences.is_empty() {
            return 1.0;
        }
        let sum: f32 = self.recent_confidences.iter().sum();
        sum / self.recent_confidences.len() as f32
    }

    /// Calculate average latency across watchers.
    fn calculate_avg_latency(&self) -> f64 {
        let metrics: Vec<_> = self.watcher_metrics.values().collect();
        if metrics.is_empty() {
            return 0.0;
        }
        let sum: u64 = metrics.iter().map(|m| m.avg_latency).sum();
        sum as f64 / metrics.len() as f64
    }

    /// Identify coverage gaps.
    fn identify_gaps(&self) -> Vec<String> {
        let mut gaps = Vec::new();
        let types = [
            (WatcherType::Code, "code"),
            (WatcherType::World, "world"),
            (WatcherType::Self_, "self"),
        ];

        for (wt, name) in types {
            if self.observations_by_type.get(&wt).copied().unwrap_or(0) == 0 {
                gaps.push(format!("No observations from {} watcher", name));
            }
        }

        gaps
    }

    /// Perform health check.
    fn perform_health_check(&mut self) -> Option<Observation> {
        let watchers: Vec<_> = self.watcher_metrics.values().collect();
        let failing: Vec<_> = watchers
            .iter()
            .filter(|w| w.health == WatcherHealth::Failing)
            .collect();

        if !failing.is_empty() {
            let start = Instant::now();

            let mut obs = self.create_observation(
                MetaObservationType::SystemHealth,
                HealthAssessment {
                    health: HealthStatus::Failing,
                    coverage: self.calculate_coverage(),
                    accuracy: self.calculate_quality(),
                    latency: self.calculate_avg_latency(),
                },
            );

            obs.issues = failing
                .iter()
                .map(|w| ObserverIssue {
                    issue_type: "watcher_failing".to_string(),
                    severity: Severity::High,
                    description: format!(
                        "Watcher {} ({:?}) is failing with {:.0}% error rate",
                        w.watcher_id,
                        w.watcher_type,
                        w.error_rate * 100.0
                    ),
                })
                .collect();

            obs.recommendations = vec![
                "Check watcher logs for errors".to_string(),
                "Consider restarting failing watchers".to_string(),
                "Review watcher configurations".to_string(),
            ];

            self.state
                .record_observation(start.elapsed().as_millis() as u64);
            return Some(Observation::Meta(obs));
        }

        None
    }

    /// Check coverage gaps.
    fn check_coverage_gaps(&mut self) -> Option<Observation> {
        let coverage = self.calculate_coverage();

        if coverage < self.config.coverage_threshold {
            let start = Instant::now();
            let gaps = self.identify_gaps();

            let mut obs = self.create_observation(
                MetaObservationType::CoverageGap,
                HealthAssessment {
                    health: HealthStatus::Degraded,
                    coverage,
                    accuracy: self.calculate_quality(),
                    latency: self.calculate_avg_latency(),
                },
            );

            obs.issues = gaps
                .iter()
                .map(|gap| ObserverIssue {
                    issue_type: "coverage_gap".to_string(),
                    severity: Severity::Medium,
                    description: gap.clone(),
                })
                .collect();

            obs.recommendations = vec![
                "Start additional watchers for uncovered areas".to_string(),
                "Increase observation frequency".to_string(),
                "Review filter configurations".to_string(),
            ];

            self.state
                .record_observation(start.elapsed().as_millis() as u64);
            return Some(Observation::Meta(obs));
        }

        None
    }

    /// Check observation quality.
    fn check_observation_quality(&mut self) -> Option<Observation> {
        let quality = self.calculate_quality();

        if quality < self.config.quality_threshold {
            let start = Instant::now();

            let mut obs = self.create_observation(
                MetaObservationType::ObservationQuality,
                HealthAssessment {
                    health: HealthStatus::Degraded,
                    coverage: self.calculate_coverage(),
                    accuracy: quality,
                    latency: self.calculate_avg_latency(),
                },
            );

            obs.issues = vec![ObserverIssue {
                issue_type: "low_quality".to_string(),
                severity: Severity::Medium,
                description: format!("Overall observation quality is {:.0}%", quality * 100.0),
            }];

            obs.recommendations = vec![
                "Review observation confidence thresholds".to_string(),
                "Check data source reliability".to_string(),
                "Investigate high-latency watchers".to_string(),
            ];

            self.state
                .record_observation(start.elapsed().as_millis() as u64);
            return Some(Observation::Meta(obs));
        }

        None
    }

    /// Check latency threshold.
    fn check_latency_threshold(&mut self) -> Option<Observation> {
        let high_latency_watchers: Vec<_> = self
            .watcher_metrics
            .values()
            .filter(|w| {
                w.avg_latency > self.config.latency_threshold && w.health != WatcherHealth::Stopped
            })
            .collect();

        if !high_latency_watchers.is_empty() {
            let start = Instant::now();
            let avg_latency = self.calculate_avg_latency();

            let health = if avg_latency > (self.config.latency_threshold * 2) as f64 {
                HealthStatus::Failing
            } else {
                HealthStatus::Degraded
            };

            let mut obs = self.create_observation(
                MetaObservationType::ObservationQuality,
                HealthAssessment {
                    health,
                    coverage: self.calculate_coverage(),
                    accuracy: self.calculate_quality(),
                    latency: avg_latency,
                },
            );

            obs.issues = high_latency_watchers
                .iter()
                .map(|w| {
                    let severity = if w.avg_latency > self.config.latency_threshold * 2 {
                        Severity::High
                    } else {
                        Severity::Medium
                    };
                    ObserverIssue {
                        issue_type: "latency_exceeded".to_string(),
                        severity,
                        description: format!(
                            "Watcher {} ({:?}) has latency {}ms (threshold: {}ms)",
                            w.watcher_id, w.watcher_type, w.avg_latency, self.config.latency_threshold
                        ),
                    }
                })
                .collect();

            obs.base.tags = vec!["latency_exceeded".to_string()];

            obs.recommendations = vec![
                "Check system load and available resources".to_string(),
                "Consider reducing observation frequency".to_string(),
                "Review watcher configurations for optimization".to_string(),
                "Investigate slow data sources or API calls".to_string(),
            ];

            warn!(
                watcher_id = %self.state.id,
                watcher_count = %high_latency_watchers.len(),
                threshold = %self.config.latency_threshold,
                avg_latency = %avg_latency,
                "Latency threshold exceeded"
            );

            self.state
                .record_observation(start.elapsed().as_millis() as u64);
            return Some(Observation::Meta(obs));
        }

        None
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Public Methods
    // ──────────────────────────────────────────────────────────────────────────

    /// Get metrics for all watched watchers.
    pub fn get_watcher_metrics(&self) -> Vec<WatcherMetricRecord> {
        self.watcher_metrics.values().cloned().collect()
    }

    /// Get overall system health.
    pub fn get_system_health(&self) -> SystemHealth {
        let watchers: Vec<_> = self.watcher_metrics.values().collect();

        let mut health_counts = HashMap::new();
        let mut watcher_health = HashMap::new();

        for w in &watchers {
            *health_counts.entry(w.health).or_insert(0) += 1;
            watcher_health.insert(w.watcher_id.clone(), w.health);
        }

        let overall = if health_counts.get(&WatcherHealth::Failing).copied().unwrap_or(0) > 0 {
            WatcherHealth::Failing
        } else if health_counts.get(&WatcherHealth::Degraded).copied().unwrap_or(0) > 0
            || health_counts.get(&WatcherHealth::Stopped).copied().unwrap_or(0) > 0
        {
            WatcherHealth::Degraded
        } else {
            WatcherHealth::Healthy
        };

        SystemHealth {
            overall,
            watcher_health,
            coverage: self.calculate_coverage(),
            quality: self.calculate_quality(),
        }
    }

    /// Get latency status.
    pub fn get_latency_status(&self) -> LatencyStatus {
        let watchers: Vec<_> = self.watcher_metrics.values().collect();
        let avg_latency = self.calculate_avg_latency();
        let exceeded_count = watchers
            .iter()
            .filter(|w| w.avg_latency > self.config.latency_threshold && w.health != WatcherHealth::Stopped)
            .count();

        let watcher_latencies: HashMap<String, u64> = watchers
            .iter()
            .map(|w| (w.watcher_id.clone(), w.avg_latency))
            .collect();

        LatencyStatus {
            avg_latency,
            threshold: self.config.latency_threshold,
            exceeded_count,
            watcher_latencies,
        }
    }

    /// Perform a manual calibration check.
    pub fn calibrate(&mut self) -> Observation {
        let start = Instant::now();
        let health = self.get_system_health();
        let latency_status = self.get_latency_status();

        let mut issues = Vec::new();

        // Check for failing watchers
        for (id, status) in &health.watcher_health {
            match status {
                WatcherHealth::Failing => {
                    issues.push(ObserverIssue {
                        issue_type: "watcher_failing".to_string(),
                        severity: Severity::High,
                        description: format!("Watcher {} is failing", id),
                    });
                }
                WatcherHealth::Stopped => {
                    issues.push(ObserverIssue {
                        issue_type: "watcher_stopped".to_string(),
                        severity: Severity::Medium,
                        description: format!("Watcher {} is stopped", id),
                    });
                }
                _ => {}
            }
        }

        // Check coverage
        if health.coverage < self.config.coverage_threshold {
            issues.push(ObserverIssue {
                issue_type: "low_coverage".to_string(),
                severity: Severity::Medium,
                description: format!(
                    "Observation coverage is {:.0}% (threshold: {:.0}%)",
                    health.coverage * 100.0,
                    self.config.coverage_threshold * 100.0
                ),
            });
        }

        // Check quality
        if health.quality < self.config.quality_threshold {
            issues.push(ObserverIssue {
                issue_type: "low_quality".to_string(),
                severity: Severity::Medium,
                description: format!(
                    "Observation quality is {:.0}% (threshold: {:.0}%)",
                    health.quality * 100.0,
                    self.config.quality_threshold * 100.0
                ),
            });
        }

        // Check latency
        if latency_status.avg_latency > self.config.latency_threshold as f64 {
            let severity = if latency_status.avg_latency > (self.config.latency_threshold * 2) as f64 {
                Severity::High
            } else {
                Severity::Medium
            };
            issues.push(ObserverIssue {
                issue_type: "latency_exceeded".to_string(),
                severity,
                description: format!(
                    "Average latency is {:.0}ms (threshold: {}ms)",
                    latency_status.avg_latency, self.config.latency_threshold
                ),
            });
        }

        let mut obs = self.create_observation(
            MetaObservationType::Calibration,
            HealthAssessment {
                health: match health.overall {
                    WatcherHealth::Healthy => HealthStatus::Healthy,
                    WatcherHealth::Degraded => HealthStatus::Degraded,
                    WatcherHealth::Failing => HealthStatus::Failing,
                    WatcherHealth::Stopped => HealthStatus::Stopped,
                },
                coverage: health.coverage,
                accuracy: health.quality,
                latency: latency_status.avg_latency,
            },
        );

        obs.issues = issues;
        obs.recommendations = self.generate_recommendations(&obs.issues);

        self.state
            .record_observation(start.elapsed().as_millis() as u64);

        Observation::Meta(obs)
    }

    /// Generate recommendations based on issues.
    fn generate_recommendations(&self, issues: &[ObserverIssue]) -> Vec<String> {
        let mut recommendations = Vec::new();

        for issue in issues {
            match issue.issue_type.as_str() {
                "watcher_failing" => {
                    recommendations.push("Restart failing watcher".to_string());
                    recommendations.push("Check watcher configuration".to_string());
                }
                "coverage_gap" => {
                    recommendations.push("Start additional watchers".to_string());
                }
                "low_quality" => {
                    recommendations.push("Review data sources".to_string());
                }
                "consensus_drift" => {
                    recommendations.push("Investigate recent changes".to_string());
                }
                _ => {}
            }
        }

        // Deduplicate
        recommendations.sort();
        recommendations.dedup();
        recommendations
    }
}

/// Overall system health.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealth {
    pub overall: WatcherHealth,
    pub watcher_health: HashMap<String, WatcherHealth>,
    pub coverage: f32,
    pub quality: f32,
}

/// Latency status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyStatus {
    pub avg_latency: f64,
    pub threshold: u64,
    pub exceeded_count: usize,
    pub watcher_latencies: HashMap<String, u64>,
}

#[async_trait]
impl Watcher for MetaWatch {
    fn id(&self) -> &str {
        &self.state.id
    }

    fn watcher_type(&self) -> WatcherType {
        WatcherType::Meta
    }

    fn is_running(&self) -> bool {
        self.state.running
    }

    async fn start(&mut self) -> anyhow::Result<()> {
        if self.state.running {
            warn!(watcher_id = %self.state.id, "MetaWatch already running");
            return Ok(());
        }

        self.state.running = true;

        info!(
            watcher_id = %self.state.id,
            quality_threshold = %self.config.quality_threshold,
            coverage_threshold = %self.config.coverage_threshold,
            max_consensus_drift = %self.config.max_consensus_drift,
            latency_threshold = %self.config.latency_threshold,
            "MetaWatch started"
        );

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        self.state.running = false;
        info!(watcher_id = %self.state.id, "MetaWatch stopped");
        Ok(())
    }

    async fn observe(&mut self) -> Option<Observation> {
        if !self.state.running {
            return None;
        }

        // Return any pending observations first
        if !self.pending_observations.is_empty() {
            return self.pending_observations.pop();
        }

        // Perform periodic health check
        if let Some(obs) = self.perform_health_check() {
            return Some(obs);
        }

        // Check for latency threshold breaches
        if let Some(obs) = self.check_latency_threshold() {
            return Some(obs);
        }

        // Check for coverage gaps
        if let Some(obs) = self.check_coverage_gaps() {
            return Some(obs);
        }

        // Check observation quality
        if let Some(obs) = self.check_observation_quality() {
            return Some(obs);
        }

        None
    }

    fn get_status(&self) -> WatcherStatus {
        WatcherStatus {
            id: self.state.id.clone(),
            watcher_type: WatcherType::Meta,
            running: self.state.running,
            health: self.state.calculate_health(),
            last_observation: self.state.last_observation,
            observation_count: self.state.observation_count,
            error_count: self.state.error_count,
            avg_latency_ms: self.state.avg_latency(),
        }
    }

    fn get_metrics(&self) -> WatcherMetrics {
        self.state.get_metrics()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_watch_creation() {
        let config = MetaWatchConfig::default();
        let watch = MetaWatch::new(config);

        assert!(!watch.is_running());
        assert!(watch.state.id.starts_with("meta_"));
    }

    #[test]
    fn test_meta_watch_config_defaults() {
        let config = MetaWatchConfig::default();

        assert!((config.quality_threshold - 0.7).abs() < 0.001);
        assert!((config.coverage_threshold - 0.6).abs() < 0.001);
        assert!((config.max_consensus_drift - 0.3).abs() < 0.001);
        assert_eq!(config.latency_threshold, 1000);
        assert_eq!(config.options.interval_ms, 60000);
    }

    #[test]
    fn test_calculate_coverage() {
        let config = MetaWatchConfig::default();
        let mut watch = MetaWatch::new(config);

        // No observations yet
        assert!((watch.calculate_coverage() - 0.0).abs() < 0.001);

        // Track observations from code and world
        watch.track_observation(WatcherType::Code, 0.9);
        watch.track_observation(WatcherType::World, 0.8);

        // 2 out of 3 types covered
        assert!((watch.calculate_coverage() - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_quality() {
        let config = MetaWatchConfig::default();
        let mut watch = MetaWatch::new(config);

        // Track observations with different confidence
        watch.track_observation(WatcherType::Code, 0.9);
        watch.track_observation(WatcherType::World, 0.7);
        watch.track_observation(WatcherType::Self_, 0.8);

        // Average should be 0.8
        assert!((watch.calculate_quality() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_register_watcher() {
        let config = MetaWatchConfig::default();
        let mut watch = MetaWatch::new(config);

        watch.register_watcher("code_watch_1", WatcherType::Code);
        watch.register_watcher("world_watch_1", WatcherType::World);

        assert_eq!(watch.watcher_metrics.len(), 2);
        assert!(watch.watcher_metrics.contains_key("code_watch_1"));
    }

    #[test]
    fn test_consensus_drift_detection() {
        let config = MetaWatchConfig {
            max_consensus_drift: 0.2,
            ..Default::default()
        };
        let mut watch = MetaWatch::new(config);
        watch.state.running = true;
        watch.last_consensus_strength = 0.5;

        // Trigger a significant drift
        watch.handle_consensus_change(0.8);

        assert_eq!(watch.pending_observations.len(), 1);
    }

    #[test]
    fn test_system_health() {
        let config = MetaWatchConfig::default();
        let mut watch = MetaWatch::new(config);

        watch.register_watcher("code_watch_1", WatcherType::Code);
        watch.track_observation(WatcherType::Code, 0.9);

        let health = watch.get_system_health();
        assert_eq!(health.overall, WatcherHealth::Healthy);
    }

    #[test]
    fn test_latency_status() {
        let config = MetaWatchConfig::default();
        let mut watch = MetaWatch::new(config);

        watch.watcher_metrics.insert(
            "code_watch_1".to_string(),
            WatcherMetricRecord {
                watcher_id: "code_watch_1".to_string(),
                watcher_type: WatcherType::Code,
                observation_count: 10,
                error_rate: 0.0,
                avg_latency: 500,
                last_observation: Some(Utc::now()),
                health: WatcherHealth::Healthy,
            },
        );

        let status = watch.get_latency_status();
        assert_eq!(status.exceeded_count, 0);
        assert!(status.avg_latency < 1000.0);
    }

    #[test]
    fn test_calibrate() {
        let config = MetaWatchConfig::default();
        let mut watch = MetaWatch::new(config);
        watch.state.running = true;

        watch.register_watcher("code_watch_1", WatcherType::Code);
        watch.track_observation(WatcherType::Code, 0.9);

        let obs = watch.calibrate();
        assert!(matches!(obs, Observation::Meta(_)));
    }
}
