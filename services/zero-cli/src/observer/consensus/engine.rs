//! Consensus Engine
//!
//! The core of the Observer Network that aggregates observations from
//! multiple watchers and forms a unified understanding of the world.
//!
//! Implements the "祝融说" philosophy of "观察共识" (observation consensus):
//! Multiple observers contribute to a shared understanding through
//! attention-weighted aggregation, pattern detection, and convergence.

use super::world_model::{WorldModelBuilder, WorldModelConfig};
use crate::observer::types::{
    Anomaly, AnomalyType, AttentionWeights, EmergentPattern, Observation, Opportunity,
    OpportunityType, PatternType, Severity, WatcherType, WorldModel,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, VecDeque};
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

// ══════════════════════════════════════════════════════════════════════════════
// Configuration
// ══════════════════════════════════════════════════════════════════════════════

/// Consensus engine configuration
#[derive(Debug, Clone)]
pub struct ConsensusConfig {
    /// Consensus window in ms
    pub window_ms: i64,
    /// Update interval in ms
    pub update_interval_ms: u64,
    /// Pattern detection config
    pub pattern_min_observations: usize,
    /// Pattern detection window in ms
    pub pattern_window_ms: i64,
    /// Minimum confidence for pattern
    pub pattern_min_confidence: f32,
    /// Anomaly detection sensitivity
    pub anomaly_sensitivity: f32,
    /// World model config
    pub world_model: WorldModelConfig,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            window_ms: 60_000,           // 1 minute
            update_interval_ms: 5_000,   // 5 seconds
            pattern_min_observations: 3,
            pattern_window_ms: 300_000,  // 5 minutes
            pattern_min_confidence: 0.5,
            anomaly_sensitivity: 0.7,
            world_model: WorldModelConfig::default(),
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Consensus Snapshot
// ══════════════════════════════════════════════════════════════════════════════

/// Snapshot of consensus state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsensusSnapshot {
    /// World model
    pub world_model: Option<WorldModel>,
    /// Active patterns
    pub patterns: Vec<EmergentPattern>,
    /// Active anomalies
    pub anomalies: Vec<Anomaly>,
    /// Active opportunities
    pub opportunities: Vec<Opportunity>,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Overall confidence
    pub confidence: f32,
}

impl Default for ConsensusSnapshot {
    fn default() -> Self {
        Self {
            world_model: None,
            patterns: Vec::new(),
            anomalies: Vec::new(),
            opportunities: Vec::new(),
            timestamp: Utc::now(),
            confidence: 0.0,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Consensus Events
// ══════════════════════════════════════════════════════════════════════════════

/// Events emitted by the consensus engine
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConsensusEvent {
    WorldModelUpdated(WorldModel),
    PatternDetected(EmergentPattern),
    PatternExpired { pattern_id: String, reason: String },
    AnomalyDetected(Anomaly),
    OpportunityIdentified(Opportunity),
    ConsensusStrengthChanged {
        previous_strength: f32,
        new_strength: f32,
        change: String,
    },
}

// ══════════════════════════════════════════════════════════════════════════════
// Consensus Engine
// ══════════════════════════════════════════════════════════════════════════════

/// Aggregates observations and forms consensus understanding
pub struct ConsensusEngine {
    config: ConsensusConfig,
    observations: RwLock<VecDeque<Observation>>,
    world_model_builder: RwLock<WorldModelBuilder>,

    // Pattern state
    active_patterns: RwLock<HashMap<String, EmergentPattern>>,
    pattern_history: RwLock<VecDeque<EmergentPattern>>,

    // Anomaly state
    active_anomalies: RwLock<HashMap<String, Anomaly>>,

    // Opportunity state
    active_opportunities: RwLock<HashMap<String, Opportunity>>,

    // Attention weights
    attention_weights: RwLock<AttentionWeights>,

    // State
    last_snapshot: RwLock<ConsensusSnapshot>,
    last_consensus_strength: RwLock<f32>,
    running: RwLock<bool>,

    // Event broadcasting
    event_tx: broadcast::Sender<ConsensusEvent>,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new(config: ConsensusConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            world_model_builder: RwLock::new(WorldModelBuilder::new(config.world_model.clone())),
            config,
            observations: RwLock::new(VecDeque::with_capacity(1000)),
            active_patterns: RwLock::new(HashMap::new()),
            pattern_history: RwLock::new(VecDeque::new()),
            active_anomalies: RwLock::new(HashMap::new()),
            active_opportunities: RwLock::new(HashMap::new()),
            attention_weights: RwLock::new(AttentionWeights::default()),
            last_snapshot: RwLock::new(ConsensusSnapshot::default()),
            last_consensus_strength: RwLock::new(0.5),
            running: RwLock::new(false),
            event_tx,
        }
    }

    /// Subscribe to consensus events
    pub fn subscribe(&self) -> broadcast::Receiver<ConsensusEvent> {
        self.event_tx.subscribe()
    }

    /// Add an observation
    pub async fn add_observation(&self, observation: Observation) {
        let mut obs = self.observations.write().await;
        obs.push_back(observation);

        // Limit buffer size
        while obs.len() > 10000 {
            obs.pop_front();
        }
    }

    /// Add multiple observations
    pub async fn add_observations(&self, observations: Vec<Observation>) {
        let mut obs = self.observations.write().await;
        for o in observations {
            obs.push_back(o);
        }

        while obs.len() > 10000 {
            obs.pop_front();
        }
    }

    /// Perform a consensus update
    pub async fn update(&self) -> ConsensusSnapshot {
        let now = Utc::now();
        let window_start = now - Duration::milliseconds(self.config.window_ms);

        // Get observations in window
        let obs_lock = self.observations.read().await;
        let window_obs: Vec<Observation> = obs_lock
            .iter()
            .filter(|o| o.timestamp() > window_start)
            .cloned()
            .collect();
        drop(obs_lock);

        // Apply attention weighting
        let weighted = self.apply_attention_weights(&window_obs).await;

        // Detect patterns
        let new_patterns = self.detect_patterns(&weighted).await;

        // Detect anomalies
        let new_anomalies = self.detect_anomalies(&weighted).await;

        // Identify opportunities
        let new_opportunities = self.identify_opportunities(&weighted).await;

        // Build world model
        let world_model = {
            let mut builder = self.world_model_builder.write().await;
            builder.build(&weighted)
        };

        // Calculate consensus strength
        let confidence = self.calculate_consensus_strength(&weighted, &new_patterns, &new_anomalies);

        // Publish events
        self.publish_events(
            world_model.as_ref(),
            &new_patterns,
            &new_anomalies,
            &new_opportunities,
            confidence,
        ).await;

        // Create snapshot
        let snapshot = ConsensusSnapshot {
            world_model,
            patterns: self.active_patterns.read().await.values().cloned().collect(),
            anomalies: self.active_anomalies.read().await.values().cloned().collect(),
            opportunities: self.active_opportunities.read().await.values().cloned().collect(),
            timestamp: now,
            confidence,
        };

        *self.last_snapshot.write().await = snapshot.clone();
        *self.last_consensus_strength.write().await = confidence;

        debug!(
            observations = window_obs.len(),
            patterns = new_patterns.len(),
            anomalies = new_anomalies.len(),
            opportunities = new_opportunities.len(),
            confidence = format!("{:.2}", confidence),
            "Consensus updated"
        );

        snapshot
    }

    /// Get current snapshot
    pub async fn get_snapshot(&self) -> ConsensusSnapshot {
        self.last_snapshot.read().await.clone()
    }

    /// Get current world model
    pub async fn get_world_model(&self) -> Option<WorldModel> {
        self.world_model_builder.read().await.get_current().cloned()
    }

    /// Get active patterns
    pub async fn get_patterns(&self) -> Vec<EmergentPattern> {
        self.active_patterns.read().await.values().cloned().collect()
    }

    /// Get active anomalies
    pub async fn get_anomalies(&self) -> Vec<Anomaly> {
        self.active_anomalies.read().await.values().cloned().collect()
    }

    /// Get active opportunities
    pub async fn get_opportunities(&self) -> Vec<Opportunity> {
        self.active_opportunities.read().await.values().cloned().collect()
    }

    /// Get attention weights
    pub async fn get_attention_weights(&self) -> AttentionWeights {
        self.attention_weights.read().await.clone()
    }

    /// Update attention weights
    pub async fn update_attention_weights(&self, weights: AttentionWeights) {
        *self.attention_weights.write().await = weights;
    }

    /// Clear all state
    pub async fn clear(&self) {
        self.observations.write().await.clear();
        self.active_patterns.write().await.clear();
        self.pattern_history.write().await.clear();
        self.active_anomalies.write().await.clear();
        self.active_opportunities.write().await.clear();
        self.world_model_builder.write().await.clear();
        *self.last_snapshot.write().await = ConsensusSnapshot::default();
        *self.last_consensus_strength.write().await = 0.5;
    }

    /// Check if running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Start the engine (sets running flag)
    pub async fn start(&self) {
        *self.running.write().await = true;
        info!(
            window_ms = self.config.window_ms,
            update_interval_ms = self.config.update_interval_ms,
            "Consensus engine started"
        );
    }

    /// Stop the engine
    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("Consensus engine stopped");
    }

    // ══════════════════════════════════════════════════════════════════════════════
    // Private Methods
    // ══════════════════════════════════════════════════════════════════════════════

    async fn apply_attention_weights(&self, observations: &[Observation]) -> Vec<Observation> {
        let weights = self.attention_weights.read().await;
        let now = Utc::now();

        let mut weighted: Vec<(f32, Observation)> = observations
            .iter()
            .map(|o| {
                let watcher_weight = match o.watcher_type() {
                    WatcherType::Code => weights.by_watcher.code,
                    WatcherType::World => weights.by_watcher.world,
                    WatcherType::Self_ => weights.by_watcher.self_,
                    WatcherType::Meta => weights.by_watcher.meta,
                };

                // Time decay
                let age_ms = (now - o.timestamp()).num_milliseconds() as f32;
                let time_factor = (-age_ms / (self.config.window_ms as f32) * weights.time_decay).exp();

                // Recency bonus
                let recency_factor = if age_ms < 10000.0 {
                    1.0 + weights.recency_bias * (1.0 - age_ms / 10000.0)
                } else {
                    1.0
                };

                let total_weight = o.confidence() * watcher_weight * time_factor * recency_factor;
                (total_weight, o.clone())
            })
            .collect();

        // Sort by weight descending
        weighted.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        weighted.into_iter().map(|(_, o)| o).collect()
    }

    async fn detect_patterns(&self, observations: &[Observation]) -> Vec<EmergentPattern> {
        let window_start = Utc::now() - Duration::milliseconds(self.config.pattern_window_ms);
        let recent: Vec<_> = observations
            .iter()
            .filter(|o| o.timestamp() > window_start)
            .collect();

        if recent.len() < self.config.pattern_min_observations {
            return Vec::new();
        }

        let mut new_patterns = Vec::new();

        // Detect trends by watcher type
        let mut by_watcher: HashMap<WatcherType, Vec<&Observation>> = HashMap::new();
        for obs in &recent {
            by_watcher.entry(obs.watcher_type()).or_default().push(obs);
        }

        for (watcher_type, obs) in by_watcher {
            if obs.len() >= self.config.pattern_min_observations {
                // Check for confidence trend
                let confidences: Vec<f32> = obs.iter().map(|o| o.confidence()).collect();
                let trend = calculate_trend(&confidences);

                if trend.slope.abs() > 0.1 {
                    let direction = if trend.slope > 0.0 { "increasing" } else { "decreasing" };
                    let mut pattern = EmergentPattern::new(
                        format!("{} {} trend", direction, watcher_type),
                        PatternType::Trend,
                    );
                    pattern.description = format!(
                        "{} observations showing {} confidence trend (slope: {:.3})",
                        watcher_type, direction, trend.slope
                    );
                    pattern.observation_ids = obs.iter().map(|o| o.id().to_string()).collect();
                    pattern.confidence = (trend.slope.abs() * 2.0).min(1.0);
                    pattern.strength = pattern.confidence;
                    pattern.metadata.insert(
                        "direction".to_string(),
                        serde_json::json!(direction),
                    );
                    pattern.metadata.insert(
                        "slope".to_string(),
                        serde_json::json!(trend.slope),
                    );

                    if trend.slope < 0.0 {
                        pattern.suggested_actions.push("Investigate declining confidence".to_string());
                    }

                    new_patterns.push(pattern);
                }
            }
        }

        // Update active patterns
        let mut active = self.active_patterns.write().await;
        for pattern in &new_patterns {
            // Check for similar existing pattern
            let similar = active.values().find(|p| {
                p.pattern_type == pattern.pattern_type && p.name == pattern.name
            });

            if let Some(existing_key) = similar.map(|p| p.id.clone()) {
                if let Some(existing) = active.get_mut(&existing_key) {
                    existing.last_seen_at = Utc::now();
                    existing.strength = existing.strength.max(pattern.strength);
                }
            } else {
                active.insert(pattern.id.clone(), pattern.clone());
            }
        }

        new_patterns
    }

    async fn detect_anomalies(&self, observations: &[Observation]) -> Vec<Anomaly> {
        let mut new_anomalies = Vec::new();

        if observations.is_empty() {
            return new_anomalies;
        }

        // Detect outliers based on confidence
        let avg_confidence: f32 = observations.iter().map(|o| o.confidence()).sum::<f32>()
            / observations.len() as f32;

        let std_dev: f32 = (observations
            .iter()
            .map(|o| (o.confidence() - avg_confidence).powi(2))
            .sum::<f32>() / observations.len() as f32)
            .sqrt();

        let threshold = avg_confidence - std_dev * 2.0 * self.config.anomaly_sensitivity;

        let low_confidence: Vec<_> = observations
            .iter()
            .filter(|o| o.confidence() < threshold)
            .collect();

        if low_confidence.len() >= 2 {
            let mut anomaly = Anomaly::new(
                AnomalyType::Outlier,
                format!(
                    "{} observations with unusually low confidence (< {:.2})",
                    low_confidence.len(),
                    threshold
                ),
            );
            anomaly.severity = Severity::Medium;
            anomaly.observation_ids = low_confidence.iter().map(|o| o.id().to_string()).collect();
            anomaly.confidence = 0.7;

            new_anomalies.push(anomaly);
        }

        // Update active anomalies
        let mut active = self.active_anomalies.write().await;
        for anomaly in &new_anomalies {
            active.insert(anomaly.id.clone(), anomaly.clone());
        }

        // Expire old anomalies
        let expiry = Utc::now() - Duration::milliseconds(self.config.window_ms * 5);
        active.retain(|_, a| a.detected_at > expiry);

        new_anomalies
    }

    async fn identify_opportunities(&self, _observations: &[Observation]) -> Vec<Opportunity> {
        let mut new_opportunities = Vec::new();

        // Check for optimization opportunities based on patterns
        let patterns = self.active_patterns.read().await;

        for pattern in patterns.values() {
            if pattern.pattern_type == PatternType::Trend && pattern.strength > 0.7 {
                // High strength trend might indicate optimization opportunity
                if pattern.name.contains("increasing") {
                    let mut opp = Opportunity::new(
                        OpportunityType::Optimization,
                        format!("Leverage positive trend: {}", pattern.name),
                    );
                    opp.observation_ids = pattern.observation_ids.clone();
                    opp.confidence = pattern.confidence;
                    opp.suggested_actions.push("Monitor and reinforce positive trend".to_string());

                    new_opportunities.push(opp);
                }
            }
        }

        // Update active opportunities
        let mut active = self.active_opportunities.write().await;
        for opp in &new_opportunities {
            active.insert(opp.id.clone(), opp.clone());
        }

        // Expire old opportunities
        let expiry = Utc::now() - Duration::milliseconds(self.config.window_ms * 10);
        active.retain(|_, o| o.detected_at > expiry);

        new_opportunities
    }

    fn calculate_consensus_strength(
        &self,
        observations: &[Observation],
        patterns: &[EmergentPattern],
        anomalies: &[Anomaly],
    ) -> f32 {
        if observations.is_empty() {
            return 0.0;
        }

        // Base from observation confidence
        let avg_confidence: f32 = observations.iter().map(|o| o.confidence()).sum::<f32>()
            / observations.len() as f32;

        // Pattern strength bonus
        let pattern_bonus = if !patterns.is_empty() {
            patterns.iter().map(|p| p.strength).sum::<f32>() / patterns.len() as f32 * 0.2
        } else {
            0.0
        };

        // Anomaly penalty
        let anomaly_penalty = (anomalies.len() as f32 * 0.05).min(0.3);

        // Coverage factor
        let mut watcher_types = std::collections::HashSet::new();
        for o in observations {
            watcher_types.insert(o.watcher_type());
        }
        let coverage = watcher_types.len() as f32 / 4.0;

        (avg_confidence * 0.4 + pattern_bonus + coverage * 0.4 - anomaly_penalty)
            .max(0.0)
            .min(1.0)
    }

    async fn publish_events(
        &self,
        world_model: Option<&WorldModel>,
        patterns: &[EmergentPattern],
        anomalies: &[Anomaly],
        opportunities: &[Opportunity],
        confidence: f32,
    ) {
        // Publish world model update
        if let Some(wm) = world_model {
            let _ = self.event_tx.send(ConsensusEvent::WorldModelUpdated(wm.clone()));
        }

        // Publish new patterns
        for pattern in patterns {
            let _ = self.event_tx.send(ConsensusEvent::PatternDetected(pattern.clone()));
        }

        // Publish new anomalies
        for anomaly in anomalies {
            let _ = self.event_tx.send(ConsensusEvent::AnomalyDetected(anomaly.clone()));
        }

        // Publish new opportunities
        for opp in opportunities {
            let _ = self.event_tx.send(ConsensusEvent::OpportunityIdentified(opp.clone()));
        }

        // Check for consensus strength change
        let last_strength = *self.last_consensus_strength.read().await;
        let strength_change = (confidence - last_strength).abs();

        if strength_change > 0.1 {
            let _ = self.event_tx.send(ConsensusEvent::ConsensusStrengthChanged {
                previous_strength: last_strength,
                new_strength: confidence,
                change: if confidence > last_strength { "increased" } else { "decreased" }.to_string(),
            });
        }
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new(ConsensusConfig::default())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper Functions
// ══════════════════════════════════════════════════════════════════════════════

struct TrendResult {
    slope: f32,
    #[allow(dead_code)]
    r2: f32,
}

fn calculate_trend(values: &[f32]) -> TrendResult {
    let n = values.len();
    if n < 2 {
        return TrendResult { slope: 0.0, r2: 0.0 };
    }

    let n_f = n as f32;
    let mut sum_x = 0.0_f32;
    let mut sum_y = 0.0_f32;
    let mut sum_xy = 0.0_f32;
    let mut sum_x2 = 0.0_f32;

    for (i, &y) in values.iter().enumerate() {
        let x = i as f32;
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
    }

    let denom = n_f * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-10 {
        return TrendResult { slope: 0.0, r2: 0.0 };
    }

    let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n_f;

    // Calculate R²
    let mean_y = sum_y / n_f;
    let mut ss_res = 0.0_f32;
    let mut ss_tot = 0.0_f32;

    for (i, &y) in values.iter().enumerate() {
        let predicted = slope * i as f32 + intercept;
        ss_res += (y - predicted).powi(2);
        ss_tot += (y - mean_y).powi(2);
    }

    let r2 = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 };

    TrendResult { slope, r2 }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::types::{CodeObservation, CodeObservationType};

    #[tokio::test]
    async fn test_consensus_engine_creation() {
        let engine = ConsensusEngine::new(ConsensusConfig::default());
        assert!(!engine.is_running().await);
    }

    #[tokio::test]
    async fn test_add_observations() {
        let engine = ConsensusEngine::new(ConsensusConfig::default());

        let obs = Observation::Code(CodeObservation::new(
            "watcher",
            CodeObservationType::GitChange,
            "file.rs",
        ));

        engine.add_observation(obs).await;

        let snapshot = engine.update().await;
        // With only 1 observation, world model won't be built (min 5)
        assert!(snapshot.world_model.is_none());
    }

    #[tokio::test]
    async fn test_start_stop() {
        let engine = ConsensusEngine::new(ConsensusConfig::default());

        assert!(!engine.is_running().await);
        engine.start().await;
        assert!(engine.is_running().await);
        engine.stop().await;
        assert!(!engine.is_running().await);
    }

    #[test]
    fn test_trend_calculation() {
        let increasing = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = calculate_trend(&increasing);
        assert!(result.slope > 0.0);

        let decreasing = vec![0.5, 0.4, 0.3, 0.2, 0.1];
        let result = calculate_trend(&decreasing);
        assert!(result.slope < 0.0);

        let flat = vec![0.5, 0.5, 0.5, 0.5];
        let result = calculate_trend(&flat);
        assert!(result.slope.abs() < 0.01);
    }
}
