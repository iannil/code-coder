//! Self Watcher (SelfWatch)
//!
//! Observes the system's own behavior including:
//! - Agent decisions and actions
//! - Resource usage (tokens, cost, time)
//! - Error patterns
//! - Quality metrics
//! - Tool invocations
//!
//! This implements the "观察自己" (self-observation) aspect of
//! the Observer Network, enabling meta-cognitive awareness.

use crate::observer::types::{
    Observation, QualityMetrics, SelfObservation, SelfObservationDetails, SelfObservationType,
    WatcherType,
};
use crate::observer::watchers::{
    BaseWatcherState, Watcher, WatcherMetrics, WatcherOptions, WatcherStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;
use tracing::{info, warn};

// ══════════════════════════════════════════════════════════════════════════════
// Configuration
// ══════════════════════════════════════════════════════════════════════════════

/// Configuration for SelfWatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfWatchConfig {
    /// Session ID to monitor (if not provided, monitors all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Track resource usage
    #[serde(default = "default_true")]
    pub track_resources: bool,
    /// Track decision history
    #[serde(default = "default_true")]
    pub track_decisions: bool,
    /// Track tool invocations
    #[serde(default = "default_true")]
    pub track_tools: bool,
    /// Error pattern window in ms
    #[serde(default = "default_error_window")]
    pub error_window_ms: u64,
    /// Cost spike multiplier threshold
    #[serde(default = "default_cost_spike_threshold")]
    pub cost_spike_threshold: f64,
    /// Cost history window size for spike detection
    #[serde(default = "default_cost_history_size")]
    pub cost_history_size: usize,
    /// Common watcher options
    #[serde(flatten)]
    pub options: WatcherOptions,
}

fn default_true() -> bool {
    true
}

fn default_error_window() -> u64 {
    300000 // 5 minutes
}

fn default_cost_spike_threshold() -> f64 {
    2.0
}

fn default_cost_history_size() -> usize {
    50
}

impl Default for SelfWatchConfig {
    fn default() -> Self {
        Self {
            session_id: None,
            track_resources: true,
            track_decisions: true,
            track_tools: true,
            error_window_ms: 300000,
            cost_spike_threshold: 2.0,
            cost_history_size: 50,
            options: WatcherOptions {
                interval_ms: 0, // Event-driven primarily
                ..Default::default()
            },
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Internal Types
// ══════════════════════════════════════════════════════════════════════════════

/// Agent action record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAction {
    pub agent_id: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Resource usage snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub tokens: u64,
    pub cost: f64,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Session cost record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCostRecord {
    pub session_id: String,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Error record.
#[derive(Debug, Clone)]
struct ErrorRecord {
    error: String,
    timestamp: DateTime<Utc>,
}

// ══════════════════════════════════════════════════════════════════════════════
// SelfWatch Implementation
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher that observes the system's own behavior.
pub struct SelfWatch {
    /// Base watcher state
    state: BaseWatcherState,
    /// Configuration
    config: SelfWatchConfig,
    /// Recent actions
    recent_actions: VecDeque<AgentAction>,
    /// Recent errors for pattern detection
    recent_errors: VecDeque<ErrorRecord>,
    /// Resource snapshots
    resource_snapshots: VecDeque<ResourceSnapshot>,
    /// Session cost history
    session_cost_history: VecDeque<SessionCostRecord>,
    /// Maximum history size
    max_history_size: usize,
    /// Pending observations queue
    pending_observations: Vec<Observation>,
}

impl SelfWatch {
    /// Create a new SelfWatch instance.
    pub fn new(config: SelfWatchConfig) -> Self {
        let id = config
            .options
            .id
            .clone()
            .unwrap_or_else(|| BaseWatcherState::generate_id("self"));

        Self {
            state: BaseWatcherState::new(id),
            config,
            recent_actions: VecDeque::new(),
            recent_errors: VecDeque::new(),
            resource_snapshots: VecDeque::new(),
            session_cost_history: VecDeque::new(),
            max_history_size: 100,
            pending_observations: Vec::new(),
        }
    }

    /// Create a self observation.
    fn create_observation(
        &self,
        obs_type: SelfObservationType,
        agent_id: impl Into<String>,
        details: SelfObservationDetails,
    ) -> SelfObservation {
        let mut obs = SelfObservation::new(&self.state.id, obs_type, agent_id);
        obs.observation = details;
        obs
    }

    /// Detect error patterns in recent errors.
    fn detect_error_pattern(&mut self) -> Option<Observation> {
        let now = Utc::now();
        let window_start = now - chrono::Duration::milliseconds(self.config.error_window_ms as i64);

        // Filter recent errors within window
        let recent: Vec<_> = self
            .recent_errors
            .iter()
            .filter(|e| e.timestamp > window_start)
            .collect();

        if recent.len() < 3 {
            return None;
        }

        // Group by error message prefix (first 50 chars)
        let mut error_groups: HashMap<String, usize> = HashMap::new();
        for error in &recent {
            let prefix: String = error.error.chars().take(50).collect();
            *error_groups.entry(prefix).or_insert(0) += 1;
        }

        // Find patterns (errors occurring 3+ times)
        for (prefix, count) in error_groups {
            if count >= 3 {
                let obs = self.create_observation(
                    SelfObservationType::ErrorPattern,
                    "system",
                    SelfObservationDetails {
                        action: "pattern_detected".to_string(),
                        input: Some(serde_json::json!({
                            "errorPrefix": prefix,
                            "count": count,
                        })),
                        output: None,
                        duration: None,
                        success: false,
                        error: Some(format!("Repeated error pattern: {} ({} occurrences)", prefix, count)),
                    },
                );
                return Some(Observation::Self_(obs));
            }
        }

        None
    }

    /// Calculate efficiency metric.
    fn calculate_efficiency(&self, tokens: u64, cost: f64, duration_ms: u64) -> f32 {
        if cost == 0.0 || duration_ms == 0 {
            return 1.0;
        }

        let tokens_per_second = tokens as f64 / (duration_ms as f64 / 1000.0);
        let cost_efficiency = tokens_per_second / cost;

        // Normalize to 0-1 range (assuming 1000 tokens/sec/$ is excellent)
        (cost_efficiency / 1000.0).min(1.0) as f32
    }

    /// Calculate average cost from history.
    fn calculate_average_cost(&self) -> f64 {
        if self.session_cost_history.is_empty() {
            return 0.0;
        }

        let total: f64 = self.session_cost_history.iter().map(|r| r.cost_usd).sum();
        total / self.session_cost_history.len() as f64
    }

    /// Normalize efficiency to 0-1 range.
    fn normalize_efficiency(&self, tokens_per_dollar: f64) -> f32 {
        // Assuming 100,000 tokens/$ is excellent efficiency
        (tokens_per_dollar / 100_000.0).min(1.0) as f32
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Public Methods for Manual Observation
    // ──────────────────────────────────────────────────────────────────────────

    /// Observe an agent action.
    pub fn observe_agent_action(&mut self, action: AgentAction) {
        if !self.state.running {
            return;
        }

        let start = Instant::now();

        // Track action
        self.recent_actions.push_back(action.clone());
        if self.recent_actions.len() > self.max_history_size {
            self.recent_actions.pop_front();
        }

        // Track error if failed
        if !action.success {
            if let Some(ref error) = action.error {
                self.recent_errors.push_back(ErrorRecord {
                    error: error.clone(),
                    timestamp: action.timestamp,
                });
                if self.recent_errors.len() > self.max_history_size {
                    self.recent_errors.pop_front();
                }
            }
        }

        let mut obs = self.create_observation(
            SelfObservationType::AgentBehavior,
            &action.agent_id,
            SelfObservationDetails {
                action: action.action.clone(),
                input: action.input.clone(),
                output: action.output.clone(),
                duration: Some(action.duration_ms as f64),
                success: action.success,
                error: action.error.clone(),
            },
        );

        obs.base.confidence = if action.success { 0.9 } else { 0.7 };

        self.pending_observations.push(Observation::Self_(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe tool invocation.
    pub fn observe_tool_invocation(
        &mut self,
        tool_name: &str,
        agent_id: &str,
        input: Option<serde_json::Value>,
        output: Option<serde_json::Value>,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    ) {
        if !self.state.running {
            return;
        }

        let start = Instant::now();

        let obs = self.create_observation(
            SelfObservationType::ToolInvocation,
            agent_id,
            SelfObservationDetails {
                action: format!("tool:{}", tool_name),
                input,
                output,
                duration: Some(duration_ms as f64),
                success,
                error,
            },
        );

        self.pending_observations.push(Observation::Self_(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe resource usage.
    pub fn observe_resource_usage(&mut self, agent_id: &str, tokens: u64, cost: f64, duration_ms: u64) {
        if !self.state.running {
            return;
        }

        let start = Instant::now();

        // Track resource snapshot
        self.resource_snapshots.push_back(ResourceSnapshot {
            tokens,
            cost,
            duration_ms,
            timestamp: Utc::now(),
        });
        if self.resource_snapshots.len() > self.max_history_size {
            self.resource_snapshots.pop_front();
        }

        let efficiency = self.calculate_efficiency(tokens, cost, duration_ms);

        let mut obs = self.create_observation(
            SelfObservationType::ResourceUsage,
            agent_id,
            SelfObservationDetails {
                action: "resource_update".to_string(),
                input: Some(serde_json::json!({
                    "tokens": tokens,
                    "cost": cost,
                    "duration_ms": duration_ms,
                })),
                output: None,
                duration: Some(duration_ms as f64),
                success: true,
                error: None,
            },
        );

        obs.quality = QualityMetrics {
            close_score: None,
            accuracy: None,
            efficiency: Some(efficiency),
        };

        self.pending_observations.push(Observation::Self_(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe session cost.
    pub fn observe_session_cost(
        &mut self,
        session_id: &str,
        tokens_used: u64,
        cost_usd: f64,
        duration_ms: u64,
        success: bool,
    ) {
        if !self.state.running {
            return;
        }

        let start = Instant::now();

        let record = SessionCostRecord {
            session_id: session_id.to_string(),
            tokens_used,
            cost_usd,
            duration_ms,
            timestamp: Utc::now(),
        };

        // Add to history
        self.session_cost_history.push_back(record.clone());
        if self.session_cost_history.len() > self.config.cost_history_size {
            self.session_cost_history.pop_front();
        }

        // Calculate efficiency metrics
        let tokens_per_second = if duration_ms > 0 {
            tokens_used as f64 / (duration_ms as f64 / 1000.0)
        } else {
            0.0
        };
        let cost_efficiency = if cost_usd > 0.0 {
            tokens_used as f64 / cost_usd
        } else {
            0.0
        };

        // Check for cost spike
        let avg_cost = self.calculate_average_cost();
        let is_cost_spike = avg_cost > 0.0 && cost_usd > avg_cost * self.config.cost_spike_threshold;

        let mut obs = self.create_observation(
            SelfObservationType::Cost,
            "session",
            SelfObservationDetails {
                action: "session_completed".to_string(),
                input: Some(serde_json::json!({ "sessionId": session_id })),
                output: Some(serde_json::json!({
                    "tokensUsed": tokens_used,
                    "costUSD": cost_usd,
                    "duration": duration_ms,
                    "tokensPerSecond": tokens_per_second,
                    "costEfficiency": cost_efficiency,
                    "isCostSpike": is_cost_spike,
                })),
                duration: Some(duration_ms as f64),
                success,
                error: None,
            },
        );

        obs.quality = QualityMetrics {
            close_score: None,
            accuracy: None,
            efficiency: Some(self.normalize_efficiency(cost_efficiency)),
        };

        if is_cost_spike {
            obs.base.tags = vec!["cost_spike".to_string()];
            obs.base.confidence = 0.95;
            warn!(
                watcher_id = %self.state.id,
                session_id = %session_id,
                cost = %cost_usd,
                avg_cost = %avg_cost,
                threshold = %self.config.cost_spike_threshold,
                "Cost spike detected"
            );
        }

        self.pending_observations.push(Observation::Self_(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe quality metric.
    pub fn observe_quality_metric(
        &mut self,
        agent_id: &str,
        metric_name: &str,
        value: f64,
        close_score: Option<f32>,
    ) {
        if !self.state.running {
            return;
        }

        let start = Instant::now();

        let mut obs = self.create_observation(
            SelfObservationType::QualityMetric,
            agent_id,
            SelfObservationDetails {
                action: format!("metric:{}", metric_name),
                input: None,
                output: Some(serde_json::json!(value)),
                duration: None,
                success: true,
                error: None,
            },
        );

        obs.quality = QualityMetrics {
            close_score,
            accuracy: None,
            efficiency: None,
        };

        self.pending_observations.push(Observation::Self_(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Get recent actions.
    pub fn get_recent_actions(&self, limit: Option<usize>) -> Vec<AgentAction> {
        let limit = limit.unwrap_or(20);
        self.recent_actions
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get resource summary.
    pub fn get_resource_summary(&self) -> ResourceSummary {
        let (total_tokens, total_cost, total_duration) = self.resource_snapshots.iter().fold(
            (0u64, 0.0f64, 0u64),
            |(t, c, d), snap| (t + snap.tokens, c + snap.cost, d + snap.duration_ms),
        );

        let avg_efficiency = if total_duration > 0 {
            total_tokens as f64 / total_duration as f64
        } else {
            0.0
        };

        ResourceSummary {
            total_tokens,
            total_cost,
            total_duration_ms: total_duration,
            avg_efficiency,
        }
    }

    /// Get cost statistics.
    pub fn get_cost_statistics(&self) -> CostStatistics {
        if self.session_cost_history.is_empty() {
            return CostStatistics::default();
        }

        let costs: Vec<f64> = self.session_cost_history.iter().map(|r| r.cost_usd).collect();
        let total_cost: f64 = costs.iter().sum();
        let avg_cost = total_cost / costs.len() as f64;
        let max_cost = costs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_cost = costs.iter().cloned().fold(f64::INFINITY, f64::min);

        CostStatistics {
            total_cost,
            avg_cost,
            max_cost,
            min_cost,
            session_count: costs.len(),
        }
    }
}

/// Resource usage summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSummary {
    pub total_tokens: u64,
    pub total_cost: f64,
    pub total_duration_ms: u64,
    pub avg_efficiency: f64,
}

/// Cost statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostStatistics {
    pub total_cost: f64,
    pub avg_cost: f64,
    pub max_cost: f64,
    pub min_cost: f64,
    pub session_count: usize,
}

#[async_trait]
impl Watcher for SelfWatch {
    fn id(&self) -> &str {
        &self.state.id
    }

    fn watcher_type(&self) -> WatcherType {
        WatcherType::Self_
    }

    fn is_running(&self) -> bool {
        self.state.running
    }

    async fn start(&mut self) -> anyhow::Result<()> {
        if self.state.running {
            warn!(watcher_id = %self.state.id, "SelfWatch already running");
            return Ok(());
        }

        self.state.running = true;

        info!(
            watcher_id = %self.state.id,
            session_id = ?self.config.session_id,
            track_resources = %self.config.track_resources,
            track_decisions = %self.config.track_decisions,
            track_tools = %self.config.track_tools,
            "SelfWatch started"
        );

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        self.state.running = false;
        info!(watcher_id = %self.state.id, "SelfWatch stopped");
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

        // Periodic self-check for error patterns
        self.detect_error_pattern()
    }

    fn get_status(&self) -> WatcherStatus {
        WatcherStatus {
            id: self.state.id.clone(),
            watcher_type: WatcherType::Self_,
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
    fn test_self_watch_creation() {
        let config = SelfWatchConfig::default();
        let watch = SelfWatch::new(config);

        assert!(!watch.is_running());
        assert!(watch.state.id.starts_with("self_"));
    }

    #[test]
    fn test_self_watch_config_defaults() {
        let config = SelfWatchConfig::default();

        assert!(config.track_resources);
        assert!(config.track_decisions);
        assert!(config.track_tools);
        assert_eq!(config.error_window_ms, 300000);
        assert!((config.cost_spike_threshold - 2.0).abs() < 0.001);
        assert_eq!(config.cost_history_size, 50);
    }

    #[test]
    fn test_calculate_efficiency() {
        let config = SelfWatchConfig::default();
        let watch = SelfWatch::new(config);

        // 1000 tokens, $0.01, 1000ms = 1000 tokens/sec / $0.01 = 100000
        let eff = watch.calculate_efficiency(1000, 0.01, 1000);
        assert!(eff > 0.0);
        assert!(eff <= 1.0);
    }

    #[test]
    fn test_agent_action_observation() {
        let config = SelfWatchConfig::default();
        let mut watch = SelfWatch::new(config);
        watch.state.running = true;

        let action = AgentAction {
            agent_id: "test-agent".to_string(),
            action: "test-action".to_string(),
            input: Some(serde_json::json!({"test": true})),
            output: Some(serde_json::json!({"result": "ok"})),
            duration_ms: 100,
            success: true,
            error: None,
            timestamp: Utc::now(),
        };

        watch.observe_agent_action(action);

        assert_eq!(watch.pending_observations.len(), 1);
        assert_eq!(watch.recent_actions.len(), 1);
        assert_eq!(watch.state.observation_count, 1);
    }

    #[test]
    fn test_error_pattern_detection() {
        let config = SelfWatchConfig::default();
        let mut watch = SelfWatch::new(config);
        watch.state.running = true;

        // Add 3 similar errors
        for _ in 0..3 {
            watch.recent_errors.push_back(ErrorRecord {
                error: "Connection timeout to API server".to_string(),
                timestamp: Utc::now(),
            });
        }

        let pattern = watch.detect_error_pattern();
        assert!(pattern.is_some());
    }

    #[test]
    fn test_cost_spike_detection() {
        let config = SelfWatchConfig {
            cost_spike_threshold: 2.0,
            ..Default::default()
        };
        let mut watch = SelfWatch::new(config);
        watch.state.running = true;

        // Add baseline costs
        for _ in 0..5 {
            watch.session_cost_history.push_back(SessionCostRecord {
                session_id: "test".to_string(),
                tokens_used: 1000,
                cost_usd: 0.01,
                duration_ms: 1000,
                timestamp: Utc::now(),
            });
        }

        // Observe a cost spike (3x the average)
        watch.observe_session_cost("spike-session", 3000, 0.03, 1000, true);

        // Check the last observation has cost_spike tag
        let obs = watch.pending_observations.last().unwrap();
        if let Observation::Self_(self_obs) = obs {
            assert!(self_obs.base.tags.contains(&"cost_spike".to_string()));
        }
    }

    #[test]
    fn test_resource_summary() {
        let config = SelfWatchConfig::default();
        let mut watch = SelfWatch::new(config);

        watch.resource_snapshots.push_back(ResourceSnapshot {
            tokens: 1000,
            cost: 0.01,
            duration_ms: 1000,
            timestamp: Utc::now(),
        });
        watch.resource_snapshots.push_back(ResourceSnapshot {
            tokens: 2000,
            cost: 0.02,
            duration_ms: 2000,
            timestamp: Utc::now(),
        });

        let summary = watch.get_resource_summary();
        assert_eq!(summary.total_tokens, 3000);
        assert!((summary.total_cost - 0.03).abs() < 0.001);
        assert_eq!(summary.total_duration_ms, 3000);
    }
}
