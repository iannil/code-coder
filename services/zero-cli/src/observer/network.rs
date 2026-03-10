//! Observer Network
//!
//! Main orchestrator for the Observer Network that coordinates watchers,
//! event streaming, consensus engine, and integrates with the Gear System.

use super::consensus::{ConsensusConfig, ConsensusEngine, ConsensusSnapshot};
use super::types::{Observation, WorldModel};
use super::watchers::WatcherManager;
use crate::gear::{GearPreset, GearState};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch, RwLock};
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use zero_hub::observer::{ObserverEvent, ObserverStream, StreamConfig};

// ══════════════════════════════════════════════════════════════════════════════
// Configuration
// ══════════════════════════════════════════════════════════════════════════════

/// Observer network configuration
#[derive(Debug, Clone)]
pub struct ObserverNetworkConfig {
    /// Enable the observer network
    pub enabled: bool,
    /// Default gear preset
    pub default_gear: GearPreset,
    /// Consensus engine configuration
    pub consensus: ConsensusConfig,
    /// Stream configuration
    pub stream: StreamConfig,
    /// Auto-switch gear based on CLOSE evaluation
    pub auto_gear_switch: bool,
    /// CLOSE evaluation interval in ms
    pub close_eval_interval_ms: u64,
    /// Watcher observation interval in ms
    pub observation_interval_ms: u64,
}

impl Default for ObserverNetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_gear: GearPreset::D,
            consensus: ConsensusConfig::default(),
            stream: StreamConfig::default(),
            auto_gear_switch: false,
            close_eval_interval_ms: 30_000, // 30 seconds
            observation_interval_ms: 5_000, // 5 seconds
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Observer Network State
// ══════════════════════════════════════════════════════════════════════════════

/// Observer network state for API integration
#[derive(Clone)]
pub struct ObserverNetworkState {
    /// Event stream
    pub stream: Arc<RwLock<ObserverStream>>,
    /// Consensus engine
    pub consensus: Arc<ConsensusEngine>,
    /// Watcher manager
    pub watcher_manager: Arc<RwLock<WatcherManager>>,
    /// Running state
    pub running: Arc<RwLock<bool>>,
    /// Event broadcast sender
    pub event_tx: broadcast::Sender<ObserverNetworkEvent>,
    /// Shutdown signal sender for observation loop
    shutdown_tx: Arc<watch::Sender<bool>>,
    /// Shutdown signal receiver (for cloning)
    shutdown_rx: watch::Receiver<bool>,
}

/// Events from the observer network
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObserverNetworkEvent {
    Started,
    Stopped,
    ObservationReceived { observation_id: String, watcher_type: String },
    ConsensusUpdated { confidence: f32, patterns: usize, anomalies: usize },
    GearSwitchRecommended { current: String, recommended: String, reason: String },
    WorldModelUpdated { model_id: String },
    WatcherStarted { watcher_id: String, watcher_type: String },
    WatcherStopped { watcher_id: String, watcher_type: String },
}

impl ObserverNetworkState {
    /// Create new observer network state
    pub fn new(config: &ObserverNetworkConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            stream: Arc::new(RwLock::new(ObserverStream::new(config.stream.clone()))),
            consensus: Arc::new(ConsensusEngine::new(config.consensus.clone())),
            watcher_manager: Arc::new(RwLock::new(WatcherManager::new())),
            running: Arc::new(RwLock::new(false)),
            event_tx,
            shutdown_tx: Arc::new(shutdown_tx),
            shutdown_rx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ObserverNetworkEvent> {
        self.event_tx.subscribe()
    }

    /// Check if running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get a read lock on the watcher manager
    pub async fn get_watcher_manager(&self) -> tokio::sync::RwLockReadGuard<'_, WatcherManager> {
        self.watcher_manager.read().await
    }

    /// Get a write lock on the watcher manager for registration
    pub async fn get_watcher_manager_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, WatcherManager> {
        self.watcher_manager.write().await
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Observer Network
// ══════════════════════════════════════════════════════════════════════════════

/// Main Observer Network orchestrator
pub struct ObserverNetwork {
    config: ObserverNetworkConfig,
    state: ObserverNetworkState,
    gear_state: Option<GearState>,
}

impl ObserverNetwork {
    /// Create a new observer network
    pub fn new(config: ObserverNetworkConfig) -> Self {
        Self {
            state: ObserverNetworkState::new(&config),
            config,
            gear_state: None,
        }
    }

    /// Create with gear state integration
    pub fn with_gear(config: ObserverNetworkConfig, gear_state: GearState) -> Self {
        Self {
            state: ObserverNetworkState::new(&config),
            config,
            gear_state: Some(gear_state),
        }
    }

    /// Get the state for API integration
    pub fn state(&self) -> ObserverNetworkState {
        self.state.clone()
    }

    /// Start the observer network
    pub async fn start(&self) {
        if *self.state.running.read().await {
            return;
        }

        *self.state.running.write().await = true;
        self.state.consensus.start().await;

        // Start all registered watchers
        {
            let mut watcher_manager = self.state.watcher_manager.write().await;
            if let Err(e) = watcher_manager.start_all().await {
                warn!(error = %e, "Failed to start some watchers");
            }
        }

        let _ = self.state.event_tx.send(ObserverNetworkEvent::Started);

        info!(
            enabled = self.config.enabled,
            default_gear = ?self.config.default_gear,
            auto_gear_switch = self.config.auto_gear_switch,
            observation_interval_ms = self.config.observation_interval_ms,
            "Observer network started"
        );

        // Start watcher observation loop
        let (obs_tx, mut obs_rx) = mpsc::channel::<Vec<Observation>>(100);
        let watcher_manager = self.state.watcher_manager.clone();
        let shutdown_rx = self.state.shutdown_rx.clone();
        let observation_interval_ms = self.config.observation_interval_ms;

        // Spawn observation loop task
        tokio::spawn(async move {
            let manager = watcher_manager.read().await;
            manager.run_observation_loop(obs_tx, observation_interval_ms, shutdown_rx).await;
        });

        // Spawn consensus feeder task
        let consensus = self.state.consensus.clone();
        let event_tx = self.state.event_tx.clone();
        let running = self.state.running.clone();

        tokio::spawn(async move {
            while let Some(observations) = obs_rx.recv().await {
                if !*running.read().await {
                    break;
                }

                for obs in observations {
                    let obs_id = obs.id().to_string();
                    let watcher_type = obs.watcher_type().to_string();

                    consensus.add_observation(obs).await;

                    let _ = event_tx.send(ObserverNetworkEvent::ObservationReceived {
                        observation_id: obs_id,
                        watcher_type,
                    });
                }
            }
        });

        // Start background update loop for consensus and CLOSE evaluation
        let state = self.state.clone();
        let config = self.config.clone();
        let gear_state = self.gear_state.clone();

        tokio::spawn(async move {
            let mut update_interval = interval(Duration::from_millis(config.consensus.update_interval_ms));
            let mut close_interval = interval(Duration::from_millis(config.close_eval_interval_ms));

            loop {
                tokio::select! {
                    _ = update_interval.tick() => {
                        if !*state.running.read().await {
                            break;
                        }

                        let snapshot = state.consensus.update().await;

                        let _ = state.event_tx.send(ObserverNetworkEvent::ConsensusUpdated {
                            confidence: snapshot.confidence,
                            patterns: snapshot.patterns.len(),
                            anomalies: snapshot.anomalies.len(),
                        });

                        if let Some(ref wm) = snapshot.world_model {
                            let _ = state.event_tx.send(ObserverNetworkEvent::WorldModelUpdated {
                                model_id: wm.id.clone(),
                            });
                        }
                    }
                    _ = close_interval.tick() => {
                        if !*state.running.read().await {
                            break;
                        }

                        if config.auto_gear_switch {
                            if let Some(ref gear) = gear_state {
                                // TODO: Evaluate CLOSE and recommend gear switch
                                let _ = evaluate_gear_switch(&state, gear).await;
                            }
                        }
                    }
                }
            }
        });
    }

    /// Stop the observer network
    pub async fn stop(&self) {
        if !*self.state.running.read().await {
            return;
        }

        // Signal shutdown to observation loop
        let _ = self.state.shutdown_tx.send(true);

        *self.state.running.write().await = false;

        // Stop all watchers
        {
            let mut watcher_manager = self.state.watcher_manager.write().await;
            if let Err(e) = watcher_manager.stop_all(Some("network shutdown")).await {
                warn!(error = %e, "Failed to stop some watchers");
            }
        }

        self.state.consensus.stop().await;

        let _ = self.state.event_tx.send(ObserverNetworkEvent::Stopped);

        info!("Observer network stopped");
    }

    /// Add an observation
    pub async fn add_observation(&self, observation: Observation) {
        let obs_id = observation.id().to_string();
        let watcher_type = observation.watcher_type().to_string();

        self.state.consensus.add_observation(observation).await;

        let _ = self.state.event_tx.send(ObserverNetworkEvent::ObservationReceived {
            observation_id: obs_id,
            watcher_type,
        });
    }

    /// Ingest an event from the event stream
    pub async fn ingest_event(&self, event: ObserverEvent) {
        let mut stream = self.state.stream.write().await;
        stream.push(event);
    }

    /// Get current consensus snapshot
    pub async fn get_snapshot(&self) -> ConsensusSnapshot {
        self.state.consensus.get_snapshot().await
    }

    /// Get current world model
    pub async fn get_world_model(&self) -> Option<WorldModel> {
        self.state.consensus.get_world_model().await
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get stream stats
    pub async fn get_stream_stats(&self) -> zero_hub::observer::StreamStats {
        self.state.stream.read().await.stats()
    }

    /// Get all watcher statuses
    pub async fn get_watcher_statuses(&self) -> Vec<super::watchers::WatcherStatus> {
        let manager = self.state.watcher_manager.read().await;
        manager.get_all_statuses().await
    }
}

/// Evaluate whether to recommend a gear switch based on consensus
async fn evaluate_gear_switch(
    state: &ObserverNetworkState,
    gear_state: &GearState,
) -> Option<(GearPreset, String)> {
    let snapshot = state.consensus.get_snapshot().await;
    let current_gear = gear_state.get_gear().await;

    // Simple heuristic for now - can be expanded with full CLOSE evaluation
    let confidence = snapshot.confidence;
    let anomaly_count = snapshot.anomalies.len();

    let recommended = if confidence < 0.3 || anomaly_count > 5 {
        // Low confidence or many anomalies - switch to manual
        Some((GearPreset::N, "Low confidence or high anomaly count".to_string()))
    } else if confidence > 0.8 && anomaly_count == 0 {
        // High confidence, no anomalies - can use sport mode
        Some((GearPreset::S, "High confidence and stable".to_string()))
    } else if confidence > 0.5 {
        // Normal conditions - drive mode
        Some((GearPreset::D, "Normal operating conditions".to_string()))
    } else {
        None
    };

    if let Some((recommended_gear, reason)) = &recommended {
        if *recommended_gear != current_gear {
            let _ = state.event_tx.send(ObserverNetworkEvent::GearSwitchRecommended {
                current: format!("{:?}", current_gear),
                recommended: format!("{:?}", recommended_gear),
                reason: reason.clone(),
            });

            return Some((*recommended_gear, reason.clone()));
        }
    }

    None
}

impl Default for ObserverNetwork {
    fn default() -> Self {
        Self::new(ObserverNetworkConfig::default())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::types::{CodeObservation, CodeObservationType};

    #[tokio::test]
    async fn test_observer_network_creation() {
        let network = ObserverNetwork::new(ObserverNetworkConfig::default());
        assert!(network.is_enabled());
        assert!(!network.state.is_running().await);
    }

    #[tokio::test]
    async fn test_start_stop() {
        let network = ObserverNetwork::new(ObserverNetworkConfig::default());

        network.start().await;
        assert!(network.state.is_running().await);

        network.stop().await;
        // Give background task time to stop
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!network.state.is_running().await);
    }

    #[tokio::test]
    async fn test_add_observation() {
        let network = ObserverNetwork::new(ObserverNetworkConfig::default());
        network.start().await;

        let obs = Observation::Code(CodeObservation::new(
            "test-watcher",
            CodeObservationType::GitChange,
            "file.rs",
        ));

        network.add_observation(obs).await;

        // Verify observation was added (through consensus snapshot)
        let snapshot = network.get_snapshot().await;
        assert_eq!(snapshot.confidence, 0.0); // Not enough observations yet

        network.stop().await;
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let network = ObserverNetwork::new(ObserverNetworkConfig::default());
        let mut rx = network.state.subscribe();

        network.start().await;

        // Should receive Started event
        let event = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(event.is_ok());

        if let Ok(Ok(ObserverNetworkEvent::Started)) = event {
            // Good
        } else {
            panic!("Expected Started event");
        }

        network.stop().await;
    }
}
