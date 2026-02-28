//! Hands scheduler for autonomous execution.
//!
//! Runs a background loop that:
//! 1. Discovers hands from HAND.md files
//! 2. Checks which hands are due based on their cron schedule
//! 3. Executes due hands via the executor
//! 4. Handles errors and retries

use super::executor::HandExecutor;
use super::manifest::{discover_hands, HandManifest};
use super::state::StateStore;
use anyhow::{Context, Result};
use chrono::Utc;
use cron::Schedule;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

/// Scheduler for autonomous Hands execution.
pub struct HandsScheduler {
    /// CodeCoder API endpoint
    #[allow(dead_code)]
    codecoder_endpoint: String,

    /// State store
    state_store: Arc<StateStore>,

    /// Executor for running hands
    executor: Arc<HandExecutor>,

    /// Manifest cache (hand_id -> manifest)
    manifests: Arc<RwLock<HashMap<String, HandManifest>>>,

    /// Next run times cache (hand_id -> next_run)
    next_runs: Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,

    /// Shutdown channel sender
    shutdown_tx: Option<mpsc::Sender<()>>,

    /// Check interval (seconds)
    check_interval_secs: u64,
}

impl HandsScheduler {
    /// Create a new hands scheduler.
    pub fn new(codecoder_endpoint: String) -> Result<Self> {
        let state_store = Arc::new(
            StateStore::new()
                .context("Failed to create state store")?
        );

        let executor = Arc::new(
            HandExecutor::new(codecoder_endpoint.clone())
                .context("Failed to create executor")?
        );

        Ok(Self {
            codecoder_endpoint,
            state_store,
            executor,
            manifests: Arc::new(RwLock::new(HashMap::new())),
            next_runs: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
            check_interval_secs: 60, // Check every minute
        })
    }

    /// Set the check interval.
    pub fn with_check_interval(mut self, secs: u64) -> Self {
        self.check_interval_secs = secs;
        self
    }

    /// Discover and load all hands.
    pub async fn reload_hands(&self) -> Result<usize> {
        let hands = discover_hands()
            .context("Failed to discover hands")?;

        let mut manifests = self.manifests.write().await;
        let mut next_runs = self.next_runs.write().await;

        let now = Utc::now();
        let mut loaded_count = 0;

        for hand in hands {
            // Only load enabled hands
            if !hand.config.enabled {
                tracing::debug!(
                    hand_id = %hand.config.id,
                    "Skipping disabled hand"
                );
                continue;
            }

            // Parse schedule
            let schedule = match Schedule::from_str(&hand.config.schedule) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(
                        hand_id = %hand.config.id,
                        schedule = %hand.config.schedule,
                        error = %e,
                        "Invalid cron expression"
                    );
                    continue;
                }
            };

            // Calculate next run time
            let next_run = schedule.after(&now).next()
                .unwrap_or_else(|| now + chrono::Duration::days(365));

            tracing::info!(
                hand_id = %hand.config.id,
                agent = %hand.config.agent,
                next_run = %next_run.to_rfc3339(),
                "Loaded hand"
            );

            manifests.insert(hand.config.id.clone(), hand.clone());
            next_runs.insert(hand.config.id.clone(), next_run);
            loaded_count += 1;
        }

        tracing::info!(
            count = loaded_count,
            "Reloaded hands"
        );

        Ok(loaded_count)
    }

    /// Get a list of loaded hands.
    pub async fn list_hands(&self) -> Vec<super::manifest::HandSummary> {
        let manifests = self.manifests.read().await;
        manifests.values()
            .map(|h| h.summary())
            .collect()
    }

    /// Get a specific hand by ID.
    pub async fn get_hand(&self, id: &str) -> Option<HandManifest> {
        let manifests = self.manifests.read().await;
        manifests.get(id).cloned()
    }

    /// Start the scheduler loop.
    pub async fn start(&mut self) -> Result<()> {
        // Initial load
        self.reload_hands().await?;

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let manifests = Arc::clone(&self.manifests);
        let next_runs = Arc::clone(&self.next_runs);
        let executor = Arc::clone(&self.executor);
        let check_interval = self.check_interval_secs;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Hands scheduler shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_secs(check_interval)) => {
                        let now = Utc::now();

                        // Find due hands
                        let due_hands = {
                            let runs = next_runs.read().await;
                            let manifests = manifests.read().await;

                            let mut due = Vec::new();
                            for (hand_id, next_run) in runs.iter() {
                                if *next_run <= now {
                                    if let Some(hand) = manifests.get(hand_id) {
                                        due.push(hand.clone());
                                    }
                                }
                            }
                            due
                        };

                        // Execute due hands
                        for hand in due_hands {
                            let hand_id = hand.config.id.clone();
                            let executor = Arc::clone(&executor);
                            let next_runs = Arc::clone(&next_runs);

                            tokio::spawn(async move {
                                tracing::info!(
                                    hand_id = %hand_id,
                                    "Executing scheduled hand"
                                );

                                // Execute the hand
                                match executor.execute(&hand).await {
                                    Ok(execution) => {
                                        tracing::info!(
                                            hand_id = %hand_id,
                                            execution_id = %execution.id,
                                            status = ?execution.status,
                                            "Hand execution completed"
                                        );

                                        // Update next run time
                                        if let Ok(schedule) = cron::Schedule::from_str(&hand.config.schedule) {
                                            let next_run = schedule.after(&execution.started_at).next()
                                                .unwrap_or_else(|| execution.started_at + chrono::Duration::days(365));

                                            let mut runs = next_runs.write().await;
                                            runs.insert(hand_id.clone(), next_run);

                                            tracing::debug!(
                                                hand_id = %hand_id,
                                                next_run = %next_run.to_rfc3339(),
                                                "Scheduled next run"
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            hand_id = %hand_id,
                                            error = %e,
                                            "Hand execution failed"
                                        );
                                    }
                                }
                            });
                        }

                        // Periodically reload hands (every hour)
                        // This allows picking up new/modified HAND.md files
                        static mut LAST_RELOAD: i64 = 0;
                        let now_ts = now.timestamp();
                        unsafe {
                            if now_ts - LAST_RELOAD > 3600 {
                                LAST_RELOAD = now_ts;
                                if let Err(e) = Self::reload_hands_static(
                                    &manifests,
                                    &next_runs,
                                ).await {
                                    tracing::error!(error = %e, "Failed to reload hands");
                                }
                            }
                        }
                    }
                }
            }
        });

        tracing::info!("Hands scheduler started");
        Ok(())
    }

    /// Static helper for reloading hands (used in spawned task).
    async fn reload_hands_static(
        manifests: &Arc<RwLock<HashMap<String, HandManifest>>>,
        next_runs: &Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,
    ) -> Result<()> {
        let hands = discover_hands()?;
        let now = Utc::now();

        let mut manifests_lock = manifests.write().await;
        let mut next_runs_lock = next_runs.write().await;

        // Keep existing next runs for unchanged hands
        let mut existing_next_runs = std::mem::take(&mut *next_runs_lock);

        for hand in hands {
            if !hand.config.enabled {
                continue;
            }

            // Preserve next run time if hand exists
            let next_run = if let Some(existing) = existing_next_runs.remove(&hand.config.id) {
                existing
            } else {
                // New hand, calculate next run
                let schedule = Schedule::from_str(&hand.config.schedule)?;
                schedule.after(&now).next()
                    .unwrap_or_else(|| now + chrono::Duration::days(365))
            };

            manifests_lock.insert(hand.config.id.clone(), hand.clone());
            next_runs_lock.insert(hand.config.id.clone(), next_run);
        }

        tracing::debug!("Reloaded hands from disk");
        Ok(())
    }

    /// Stop the scheduler.
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    /// Manually trigger a hand execution.
    pub async fn trigger_hand(&self, hand_id: &str) -> Result<super::state::HandExecution> {
        let manifests = self.manifests.read().await;

        let hand = manifests.get(hand_id)
            .ok_or_else(|| anyhow::anyhow!("Hand '{}' not found", hand_id))?;

        let executor = Arc::clone(&self.executor);
        let hand_clone = hand.clone();

        // Execute in a task to avoid blocking
        let executor_ref = executor.as_ref();
        executor_ref.execute(&hand_clone).await
    }

    /// Get execution history for a hand.
    pub fn get_executions(&self, hand_id: &str, limit: usize) -> Result<Vec<super::state::HandExecution>> {
        self.state_store.get_executions(hand_id, limit)
    }

    /// Get a specific execution.
    pub fn get_execution(&self, execution_id: &str) -> Result<Option<super::state::HandExecution>> {
        self.state_store.get_execution(execution_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = HandsScheduler::new("http://localhost:4400".to_string());
        assert!(scheduler.is_ok());
    }

    #[tokio::test]
    async fn test_list_hands_empty() {
        let scheduler = HandsScheduler::new("http://localhost:4400".to_string()).unwrap();
        // No hands directory, so should be empty
        let _ = scheduler.reload_hands().await;
        let hands = scheduler.list_hands().await;
        assert!(hands.is_empty());
    }
}
