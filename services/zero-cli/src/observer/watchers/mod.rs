//! Observer Watchers Module
//!
//! Four watchers that form the observation layer of the Observer Network:
//!
//! - **CodeWatch**: Observes codebase changes (Git, files, builds, tests)
//! - **WorldWatch**: Observes external world data (APIs, news, dependencies)
//! - **SelfWatch**: Observes system's own behavior (Agent actions, resources)
//! - **MetaWatch**: Observes the Observer Network itself (quality, coverage)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      Observer Watchers                                      │
//! │   CodeWatch │ WorldWatch │ SelfWatch │ MetaWatch                           │
//! └──────────────────────────────┬──────────────────────────────────────────────┘
//!                                │ emit observations
//!                                ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                      ConsensusEngine                                        │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

mod base;
mod code_watch;
mod meta_watch;
mod self_watch;
mod world_watch;

pub use base::{BaseWatcherState, WatcherHealth, WatcherMetrics, WatcherOptions};
pub use code_watch::{CodeWatch, CodeWatchConfig};
pub use meta_watch::{MetaWatch, MetaWatchConfig};
pub use self_watch::{SelfWatch, SelfWatchConfig};
pub use world_watch::{WorldWatch, WorldWatchConfig};

use crate::observer::types::{Observation, WatcherType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

// ══════════════════════════════════════════════════════════════════════════════
// Watcher Trait
// ══════════════════════════════════════════════════════════════════════════════

/// Core trait for all watchers in the Observer Network.
///
/// Watchers are responsible for observing specific domains (code, world, self, meta)
/// and emitting observations to the ConsensusEngine.
#[async_trait]
pub trait Watcher: Send + Sync {
    /// Get the unique watcher ID.
    fn id(&self) -> &str;

    /// Get the watcher type.
    fn watcher_type(&self) -> WatcherType;

    /// Check if the watcher is currently running.
    fn is_running(&self) -> bool;

    /// Start the watcher.
    async fn start(&mut self) -> anyhow::Result<()>;

    /// Stop the watcher.
    async fn stop(&mut self) -> anyhow::Result<()>;

    /// Perform one observation cycle.
    /// Returns an observation if one was produced, None otherwise.
    async fn observe(&mut self) -> Option<Observation>;

    /// Get the current watcher status.
    fn get_status(&self) -> WatcherStatus;

    /// Get the watcher metrics.
    fn get_metrics(&self) -> WatcherMetrics;
}

// ══════════════════════════════════════════════════════════════════════════════
// Status Types
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatcherStatus {
    /// Watcher ID
    pub id: String,
    /// Watcher type
    #[serde(rename = "type")]
    pub watcher_type: WatcherType,
    /// Whether the watcher is running
    pub running: bool,
    /// Current health status
    pub health: WatcherHealth,
    /// Last observation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_observation: Option<chrono::DateTime<chrono::Utc>>,
    /// Total observation count
    pub observation_count: u64,
    /// Total error count
    pub error_count: u64,
    /// Average latency in ms
    pub avg_latency_ms: u64,
}

// ══════════════════════════════════════════════════════════════════════════════
// Watcher Events
// ══════════════════════════════════════════════════════════════════════════════

/// Events emitted by watchers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WatcherEvent {
    /// Watcher started
    Started {
        watcher_id: String,
        watcher_type: WatcherType,
    },
    /// Watcher stopped
    Stopped {
        watcher_id: String,
        watcher_type: WatcherType,
        reason: Option<String>,
    },
    /// Watcher error occurred
    Error {
        watcher_id: String,
        watcher_type: WatcherType,
        error: String,
        recoverable: bool,
    },
    /// Observation emitted
    ObservationEmitted {
        watcher_id: String,
        observation_id: String,
        watcher_type: WatcherType,
    },
    /// Watcher health changed
    HealthChanged {
        watcher_id: String,
        watcher_type: WatcherType,
        previous: WatcherHealth,
        current: WatcherHealth,
    },
}

// ══════════════════════════════════════════════════════════════════════════════
// Watcher Manager
// ══════════════════════════════════════════════════════════════════════════════

/// Manages all watchers in the Observer Network.
pub struct WatcherManager {
    /// Registered watchers
    watchers: Vec<Arc<tokio::sync::Mutex<Box<dyn Watcher>>>>,
    /// Event broadcast sender
    event_tx: broadcast::Sender<WatcherEvent>,
}

impl WatcherManager {
    /// Create a new watcher manager.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            watchers: Vec::new(),
            event_tx,
        }
    }

    /// Register a watcher.
    pub fn register(&mut self, watcher: Box<dyn Watcher>) {
        self.watchers
            .push(Arc::new(tokio::sync::Mutex::new(watcher)));
    }

    /// Subscribe to watcher events.
    pub fn subscribe(&self) -> broadcast::Receiver<WatcherEvent> {
        self.event_tx.subscribe()
    }

    /// Get all watcher statuses.
    pub async fn get_all_statuses(&self) -> Vec<WatcherStatus> {
        let mut statuses = Vec::new();
        for watcher in &self.watchers {
            let w = watcher.lock().await;
            statuses.push(w.get_status());
        }
        statuses
    }

    /// Start all watchers.
    pub async fn start_all(&mut self) -> anyhow::Result<()> {
        for watcher in &self.watchers {
            let mut w = watcher.lock().await;
            w.start().await?;
            let _ = self.event_tx.send(WatcherEvent::Started {
                watcher_id: w.id().to_string(),
                watcher_type: w.watcher_type(),
            });
        }
        Ok(())
    }

    /// Stop all watchers.
    pub async fn stop_all(&mut self, reason: Option<&str>) -> anyhow::Result<()> {
        for watcher in &self.watchers {
            let mut w = watcher.lock().await;
            w.stop().await?;
            let _ = self.event_tx.send(WatcherEvent::Stopped {
                watcher_id: w.id().to_string(),
                watcher_type: w.watcher_type(),
                reason: reason.map(|s| s.to_string()),
            });
        }
        Ok(())
    }

    /// Get a watcher by ID.
    pub async fn get_watcher(&self, id: &str) -> Option<WatcherStatus> {
        for watcher in &self.watchers {
            let w = watcher.lock().await;
            if w.id() == id {
                return Some(w.get_status());
            }
        }
        None
    }

    /// Start a specific watcher by ID.
    pub async fn start_watcher(&self, id: &str) -> anyhow::Result<bool> {
        for watcher in &self.watchers {
            let mut w = watcher.lock().await;
            if w.id() == id {
                w.start().await?;
                let _ = self.event_tx.send(WatcherEvent::Started {
                    watcher_id: w.id().to_string(),
                    watcher_type: w.watcher_type(),
                });
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Stop a specific watcher by ID.
    pub async fn stop_watcher(&self, id: &str, reason: Option<&str>) -> anyhow::Result<bool> {
        for watcher in &self.watchers {
            let mut w = watcher.lock().await;
            if w.id() == id {
                w.stop().await?;
                let _ = self.event_tx.send(WatcherEvent::Stopped {
                    watcher_id: w.id().to_string(),
                    watcher_type: w.watcher_type(),
                    reason: reason.map(|s| s.to_string()),
                });
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Run observation cycles for all running watchers.
    pub async fn run_observation_cycle(&self) -> Vec<Observation> {
        let mut observations = Vec::new();

        for watcher in &self.watchers {
            let mut w = watcher.lock().await;
            if w.is_running() {
                if let Some(obs) = w.observe().await {
                    let _ = self.event_tx.send(WatcherEvent::ObservationEmitted {
                        watcher_id: w.id().to_string(),
                        observation_id: obs.id().to_string(),
                        watcher_type: w.watcher_type(),
                    });
                    observations.push(obs);
                }
            }
        }

        observations
    }

    /// Run continuous observation loop, sending observations through the channel.
    ///
    /// This method runs indefinitely until the shutdown signal is received.
    /// It periodically runs observation cycles for all watchers and sends
    /// observations to the ConsensusEngine via the provided channel.
    ///
    /// # Arguments
    /// * `tx` - Channel sender for observations to ConsensusEngine
    /// * `interval_ms` - Observation interval in milliseconds
    /// * `mut shutdown_rx` - Shutdown signal receiver
    ///
    /// # Example
    /// ```ignore
    /// let (tx, rx) = mpsc::channel(100);
    /// let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    /// manager.run_observation_loop(tx, 1000, shutdown_rx).await;
    /// ```
    pub async fn run_observation_loop(
        &self,
        tx: mpsc::Sender<Vec<Observation>>,
        interval_ms: u64,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        info!(
            interval_ms = interval_ms,
            watcher_count = self.watchers.len(),
            "Starting observation loop"
        );

        let mut ticker = interval(Duration::from_millis(interval_ms));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let observations = self.run_observation_cycle().await;

                    if !observations.is_empty() {
                        debug!(
                            observation_count = observations.len(),
                            "Collected observations from watchers"
                        );

                        if let Err(e) = tx.send(observations).await {
                            warn!(error = %e, "Failed to send observations to consensus");
                            // Channel closed, exit loop
                            break;
                        }
                    }
                }
                result = shutdown_rx.changed() => {
                    if result.is_ok() && *shutdown_rx.borrow() {
                        info!("Observation loop received shutdown signal");
                        break;
                    }
                }
            }
        }

        info!("Observation loop stopped");
    }

    /// Get the number of registered watchers.
    pub fn watcher_count(&self) -> usize {
        self.watchers.len()
    }
}

impl Default for WatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_manager_creation() {
        let manager = WatcherManager::new();
        assert!(manager.watchers.is_empty());
    }

    #[test]
    fn test_watcher_event_serialization() {
        let event = WatcherEvent::Started {
            watcher_id: "code_watch_1".to_string(),
            watcher_type: WatcherType::Code,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("started"));
        assert!(json.contains("code_watch_1"));
    }

    #[test]
    fn test_watcher_status_serialization() {
        let status = WatcherStatus {
            id: "test".to_string(),
            watcher_type: WatcherType::Code,
            running: true,
            health: WatcherHealth::Healthy,
            last_observation: None,
            observation_count: 10,
            error_count: 0,
            avg_latency_ms: 50,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"running\":true"));
    }

    #[test]
    fn test_watcher_manager_count() {
        let manager = WatcherManager::new();
        assert_eq!(manager.watcher_count(), 0);
    }

    #[tokio::test]
    async fn test_observation_loop_shutdown() {
        let manager = WatcherManager::new();
        let (tx, mut rx) = mpsc::channel(10);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // Spawn the loop
        let handle = tokio::spawn(async move {
            manager.run_observation_loop(tx, 50, shutdown_rx).await;
        });

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Signal shutdown
        shutdown_tx.send(true).unwrap();

        // Wait for loop to exit
        let result = tokio::time::timeout(Duration::from_millis(500), handle).await;
        assert!(result.is_ok(), "Loop should exit on shutdown signal");

        // Channel should be empty (no watchers registered)
        assert!(rx.try_recv().is_err());
    }
}
