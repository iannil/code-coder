//! Trading session manager for lifecycle control.
//!
//! This module provides the main interface for managing trading sessions,
//! coordinating the trading loop, state persistence, and session lifecycle.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::{SessionState, StateStore, StoredPosition, StoredSession};
use crate::data::MarketDataAggregator;
use crate::execution::ExecutionEngine;
use crate::r#loop::{LoopConfig, LoopEvent, MonitoredPosition, TradingLoop, TradingMode};
use crate::strategy::StrategyEngine;

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Trading mode (paper or live)
    pub mode: TradingMode,
    /// Loop configuration
    pub loop_config: LoopConfig,
    /// Initial capital (for paper trading)
    pub initial_capital: f64,
    /// Maximum positions
    pub max_positions: usize,
    /// Auto-start when service starts
    pub auto_start: bool,
    /// Session name/description
    pub name: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            mode: TradingMode::Paper,
            loop_config: LoopConfig::default(),
            initial_capital: 100_000.0,
            max_positions: 5,
            auto_start: false,
            name: None,
        }
    }
}

/// Session information (API response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub id: String,
    /// Current state
    pub state: SessionState,
    /// Trading mode
    pub mode: TradingMode,
    /// Session name
    pub name: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Number of open positions
    pub open_positions: usize,
    /// Total P&L
    pub total_pnl: f64,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Trading session manager
///
/// Manages the lifecycle of trading sessions, including:
/// - Starting and stopping sessions
/// - State persistence to SQLite
/// - Session recovery after restart
/// - Position tracking
pub struct TradingSessionManager {
    /// State store (SQLite)
    store: Arc<StateStore>,
    /// Current session ID
    current_session_id: Arc<RwLock<Option<String>>>,
    /// Active trading loop
    trading_loop: Arc<RwLock<Option<Arc<TradingLoop>>>>,
    /// Market data aggregator
    data: Arc<MarketDataAggregator>,
    /// Strategy engine
    strategy: Arc<StrategyEngine>,
    /// Execution engine
    execution: Arc<RwLock<ExecutionEngine>>,
    /// Default configuration
    default_config: SessionConfig,
}

impl TradingSessionManager {
    /// Create a new session manager
    pub fn new(
        db_path: PathBuf,
        data: Arc<MarketDataAggregator>,
        strategy: Arc<StrategyEngine>,
        execution: Arc<RwLock<ExecutionEngine>>,
    ) -> Result<Self> {
        let store = StateStore::open(&db_path)
            .with_context(|| format!("Failed to open state store at {:?}", db_path))?;

        Ok(Self {
            store: Arc::new(store),
            current_session_id: Arc::new(RwLock::new(None)),
            trading_loop: Arc::new(RwLock::new(None)),
            data,
            strategy,
            execution,
            default_config: SessionConfig::default(),
        })
    }

    /// Create with custom default configuration
    pub fn with_default_config(mut self, config: SessionConfig) -> Self {
        self.default_config = config;
        self
    }

    /// Get the state store
    pub fn store(&self) -> &Arc<StateStore> {
        &self.store
    }

    /// Get current session ID
    pub async fn current_session_id(&self) -> Option<String> {
        self.current_session_id.read().await.clone()
    }

    /// Get current session info
    pub async fn current_session(&self) -> Result<Option<SessionInfo>> {
        let session_id = self.current_session_id.read().await;

        if let Some(id) = session_id.as_ref() {
            return self.get_session(id).await;
        }

        Ok(None)
    }

    /// Get session info by ID
    pub async fn get_session(&self, id: &str) -> Result<Option<SessionInfo>> {
        let stored = self.store.get_session(id)?;

        if let Some(session) = stored {
            let positions = self.store.get_open_positions(id)?;
            let all_positions = self.store.get_session_positions(id)?;

            let total_pnl: f64 = all_positions
                .iter()
                .map(|p| p.realized_pnl.unwrap_or_else(|| p.unrealized_pnl()))
                .sum();

            let config: SessionConfig =
                serde_json::from_str(&session.config).unwrap_or_default();

            Ok(Some(SessionInfo {
                id: session.id,
                state: session.state,
                mode: session.mode,
                name: config.name,
                created_at: session.created_at,
                updated_at: session.updated_at,
                open_positions: positions.len(),
                total_pnl,
                error_message: session.error_message,
            }))
        } else {
            Ok(None)
        }
    }

    /// Start a new session
    pub async fn start_session(&self, config: Option<SessionConfig>) -> Result<String> {
        // Check if already running
        {
            let current_id = self.current_session_id.read().await;
            if current_id.is_some() {
                anyhow::bail!("A session is already active");
            }
        }

        let config = config.unwrap_or_else(|| self.default_config.clone());
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create stored session
        let stored = StoredSession {
            id: session_id.clone(),
            state: SessionState::Starting,
            mode: config.mode,
            config: serde_json::to_string(&config)?,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error_message: None,
        };

        self.store.create_session(&stored)?;

        // Create trading loop
        let loop_config = LoopConfig {
            mode: config.mode,
            auto_execute: config.loop_config.auto_execute,
            ..config.loop_config
        };

        let trading_loop = Arc::new(TradingLoop::new(
            loop_config,
            Arc::clone(&self.data),
            Arc::clone(&self.strategy),
            Arc::clone(&self.execution),
        ));

        // Store references
        {
            let mut current_id = self.current_session_id.write().await;
            *current_id = Some(session_id.clone());
        }
        {
            let mut loop_ref = self.trading_loop.write().await;
            *loop_ref = Some(Arc::clone(&trading_loop));
        }

        // Update state to running
        self.store
            .update_session_state(&session_id, SessionState::Running, None)?;

        // Clone for async task
        let store_clone = Arc::clone(&self.store);
        let session_id_clone = session_id.clone();
        let current_session_clone = Arc::clone(&self.current_session_id);
        let loop_ref_clone = Arc::clone(&self.trading_loop);

        // Spawn the trading loop
        tokio::spawn(async move {
            let result = trading_loop.run().await;

            // Update state based on result
            let final_state = match &result {
                Ok(_) => SessionState::Stopped,
                Err(e) => {
                    error!(error = %e, "Trading loop failed");
                    SessionState::Failed
                }
            };

            let error_msg = result.err().map(|e| e.to_string());
            let _ = store_clone.update_session_state(
                &session_id_clone,
                final_state,
                error_msg.as_deref(),
            );

            // Clear current session
            {
                let mut current_id = current_session_clone.write().await;
                *current_id = None;
            }
            {
                let mut loop_ref = loop_ref_clone.write().await;
                *loop_ref = None;
            }

            info!(session_id = %session_id_clone, state = ?final_state, "Session ended");
        });

        info!(session_id = %session_id, mode = ?config.mode, "Session started");
        Ok(session_id)
    }

    /// Stop the current session
    pub async fn stop_session(&self) -> Result<()> {
        let session_id = {
            let current_id = self.current_session_id.read().await;
            current_id.clone().ok_or_else(|| anyhow::anyhow!("No active session"))?
        };

        // Update state to stopping
        self.store
            .update_session_state(&session_id, SessionState::Stopping, None)?;

        // Stop the trading loop
        {
            let loop_ref = self.trading_loop.read().await;
            if let Some(ref trading_loop) = *loop_ref {
                trading_loop.stop().await;
            }
        }

        info!(session_id = %session_id, "Session stop requested");
        Ok(())
    }

    /// Pause the current session
    pub async fn pause_session(&self) -> Result<()> {
        let session_id = {
            let current_id = self.current_session_id.read().await;
            current_id.clone().ok_or_else(|| anyhow::anyhow!("No active session"))?
        };

        // Pause the trading loop
        {
            let loop_ref = self.trading_loop.read().await;
            if let Some(ref trading_loop) = *loop_ref {
                trading_loop.pause().await;
            }
        }

        // Update state
        self.store
            .update_session_state(&session_id, SessionState::Paused, None)?;

        info!(session_id = %session_id, "Session paused");
        Ok(())
    }

    /// Resume the current session
    pub async fn resume_session(&self) -> Result<()> {
        let session_id = {
            let current_id = self.current_session_id.read().await;
            current_id.clone().ok_or_else(|| anyhow::anyhow!("No active session"))?
        };

        // Resume the trading loop
        {
            let loop_ref = self.trading_loop.read().await;
            if let Some(ref trading_loop) = *loop_ref {
                trading_loop.resume().await;
            }
        }

        // Update state
        self.store
            .update_session_state(&session_id, SessionState::Running, None)?;

        info!(session_id = %session_id, "Session resumed");
        Ok(())
    }

    /// Subscribe to session events
    pub async fn subscribe(&self) -> Option<tokio::sync::broadcast::Receiver<LoopEvent>> {
        let loop_ref = self.trading_loop.read().await;
        loop_ref.as_ref().map(|l| l.subscribe())
    }

    /// Get recent sessions
    pub async fn get_recent_sessions(&self, limit: usize) -> Result<Vec<SessionInfo>> {
        let stored = self.store.get_recent_sessions(limit)?;
        let mut sessions = Vec::new();

        for s in stored {
            if let Some(info) = self.get_session(&s.id).await? {
                sessions.push(info);
            }
        }

        Ok(sessions)
    }

    /// Get positions for a session
    pub async fn get_positions(&self, session_id: &str) -> Result<Vec<StoredPosition>> {
        self.store.get_session_positions(session_id)
    }

    /// Get open positions for current session
    pub async fn get_open_positions(&self) -> Result<Vec<StoredPosition>> {
        let session_id = {
            let current_id = self.current_session_id.read().await;
            current_id.clone()
        };

        if let Some(id) = session_id {
            self.store.get_open_positions(&id)
        } else {
            Ok(vec![])
        }
    }

    /// Cleanup old sessions
    pub async fn cleanup(&self, days: i64) -> Result<usize> {
        self.store.cleanup_old_sessions(days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert_eq!(config.mode, TradingMode::Paper);
        assert!((config.initial_capital - 100_000.0).abs() < 0.01);
        assert_eq!(config.max_positions, 5);
        assert!(!config.auto_start);
    }

    #[test]
    fn test_session_config_serialization() {
        let config = SessionConfig {
            mode: TradingMode::Live,
            initial_capital: 200_000.0,
            max_positions: 10,
            auto_start: true,
            name: Some("Test Session".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: SessionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.mode, TradingMode::Live);
        assert!((parsed.initial_capital - 200_000.0).abs() < 0.01);
        assert_eq!(parsed.name, Some("Test Session".to_string()));
    }

    #[test]
    fn test_session_info_serialization() {
        let info = SessionInfo {
            id: "test-123".to_string(),
            state: SessionState::Running,
            mode: TradingMode::Paper,
            name: Some("Test".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            open_positions: 3,
            total_pnl: 1500.0,
            error_message: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Running"));
        assert!(json.contains("1500"));
    }
}
