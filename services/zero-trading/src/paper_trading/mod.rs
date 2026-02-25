//! Paper trading verification module.
//!
//! This module provides a complete paper trading simulation system
//! to validate signals before enabling real trading mode.
//!
//! # Usage
//!
//! ```ignore
//! let runner = PaperTradingRunner::new(&config);
//! let result = runner.run_session(Duration::from_hours(4)).await?;
//! println!("Verification result: {:?}", result.summary);
//! ```

mod runner;
mod validator;
pub mod report;

pub use runner::{PaperTradingRunner, PaperTradingConfig, SessionResult};
pub use validator::{SignalValidator, ValidationResult, ValidationMetrics};
pub use report::{PaperTradingReport, SessionSummary, VerificationResult};

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Paper trading session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session not started
    Idle,
    /// Session running
    Running,
    /// Session paused
    Paused,
    /// Session completed
    Completed,
    /// Session failed
    Failed,
}

/// A paper trade record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperTrade {
    /// Trade ID
    pub id: String,
    /// Symbol traded
    pub symbol: String,
    /// Trade direction
    pub direction: TradeDirection,
    /// Entry price
    pub entry_price: f64,
    /// Exit price (if closed)
    pub exit_price: Option<f64>,
    /// Quantity
    pub quantity: f64,
    /// Entry timestamp
    pub entry_time: chrono::DateTime<chrono::Utc>,
    /// Exit timestamp (if closed)
    pub exit_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Signal that triggered this trade
    pub signal_id: String,
    /// Trade status
    pub status: PaperTradeStatus,
    /// Realized P&L (if closed)
    pub realized_pnl: Option<f64>,
}

/// Trade direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeDirection {
    /// Long position
    Long,
    /// Short (or exit for T+1)
    Short,
}

/// Paper trade status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperTradeStatus {
    /// Trade is open
    Open,
    /// Trade closed with profit
    ClosedProfit,
    /// Trade closed with loss
    ClosedLoss,
    /// Trade cancelled
    Cancelled,
}

impl PaperTrade {
    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        let multiplier = match self.direction {
            TradeDirection::Long => 1.0,
            TradeDirection::Short => -1.0,
        };
        (current_price - self.entry_price) * self.quantity * multiplier
    }

    /// Check if trade is profitable
    pub fn is_profitable(&self, current_price: f64) -> bool {
        self.unrealized_pnl(current_price) > 0.0
    }

    /// Calculate return percentage
    pub fn return_pct(&self, current_price: f64) -> f64 {
        ((current_price - self.entry_price) / self.entry_price) * 100.0
    }

    /// Close the trade
    pub fn close(&mut self, exit_price: f64) {
        self.exit_price = Some(exit_price);
        self.exit_time = Some(chrono::Utc::now());
        self.realized_pnl = Some(self.unrealized_pnl(exit_price));
        self.status = if self.realized_pnl.unwrap_or(0.0) > 0.0 {
            PaperTradeStatus::ClosedProfit
        } else {
            PaperTradeStatus::ClosedLoss
        };
    }
}

// ============================================================================
// Paper Trading Manager
// ============================================================================

/// Status of a paper trading session (API response)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSessionStatus {
    /// Current session state
    pub state: SessionState,
    /// Session start time
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Elapsed seconds since session started
    pub elapsed_seconds: Option<i64>,
}

/// Manager for paper trading sessions (API-friendly wrapper).
///
/// This struct provides a safe, concurrent interface for managing paper trading
/// sessions. It handles session lifecycle, state management, and provides access
/// to trade data and reports.
///
/// # Concurrency
///
/// The manager uses `Arc<RwLock<_>>` for all mutable state to ensure thread-safe
/// access. State transitions are managed exclusively by the background task to
/// avoid race conditions.
pub struct PaperTradingManager {
    config: zero_common::config::Config,
    runner: Arc<RwLock<Option<Arc<PaperTradingRunner>>>>,
    state: Arc<RwLock<SessionState>>,
    last_result: Arc<RwLock<Option<SessionResult>>>,
    start_time: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
}

impl PaperTradingManager {
    /// Create a new paper trading manager
    pub fn new(config: &zero_common::config::Config) -> Self {
        Self {
            config: config.clone(),
            runner: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(SessionState::Idle)),
            last_result: Arc::new(RwLock::new(None)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Start a paper trading session in the background
    pub async fn start_session(
        &self,
        paper_config: PaperTradingConfig,
        duration: Option<Duration>,
    ) -> Result<()> {
        // Check if already running
        let current_state = *self.state.read().await;
        if current_state == SessionState::Running {
            return Err(anyhow::anyhow!("Session already running"));
        }

        // Create the runner and wrap in Arc for shared ownership
        let runner = Arc::new(PaperTradingRunner::new(&self.config, paper_config));

        // Store the runner (clone Arc, not move)
        {
            let mut runner_guard = self.runner.write().await;
            *runner_guard = Some(Arc::clone(&runner));
        }

        // Update state and start time
        {
            let mut state = self.state.write().await;
            *state = SessionState::Running;
        }
        {
            let mut start_time = self.start_time.write().await;
            *start_time = Some(chrono::Utc::now());
        }

        // Clone Arcs for the spawned task
        let state_clone = Arc::clone(&self.state);
        let last_result_clone = Arc::clone(&self.last_result);

        // Spawn the session in a background task
        // Note: We use the cloned Arc<runner> directly, avoiding holding locks during execution
        tokio::spawn(async move {
            let result = runner.run_session(duration).await;

            // Update state based on result
            match result {
                Ok(session_result) => {
                    let mut state = state_clone.write().await;
                    *state = session_result.final_state;
                    let mut last = last_result_clone.write().await;
                    *last = Some(session_result);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Paper trading session failed");
                    let mut state = state_clone.write().await;
                    *state = SessionState::Failed;
                }
            }
        });

        Ok(())
    }

    /// Stop the current session.
    ///
    /// This only signals the runner to stop. The actual state transition
    /// to `Completed` is handled by the background task to avoid race conditions.
    pub async fn stop_session(&self) -> Result<()> {
        let current_state = *self.state.read().await;
        if current_state != SessionState::Running {
            return Err(anyhow::anyhow!("No session running"));
        }

        // Stop the runner - state transition is handled by the background task
        let runner_guard = self.runner.read().await;
        if let Some(ref runner) = *runner_guard {
            runner.stop().await;
        }

        Ok(())
    }

    /// Get the current session state
    pub async fn get_state(&self) -> SessionState {
        *self.state.read().await
    }

    /// Get the full session status
    pub async fn get_status(&self) -> PaperSessionStatus {
        let state = *self.state.read().await;
        let start_time = *self.start_time.read().await;
        let elapsed_seconds = start_time.map(|st| {
            (chrono::Utc::now() - st).num_seconds()
        });

        PaperSessionStatus {
            state,
            start_time,
            elapsed_seconds,
        }
    }

    /// Get trades from the current or last session.
    ///
    /// Priority: running session trades > last completed session trades.
    /// This ensures that when a new session starts after one completes,
    /// we return the current session's trades, not the old ones.
    pub async fn get_trades(&self) -> Vec<PaperTrade> {
        // Check current session state
        let current_state = *self.state.read().await;

        // If session is running, get trades from the runner
        if current_state == SessionState::Running {
            let runner_guard = self.runner.read().await;
            if let Some(ref runner) = *runner_guard {
                return runner.get_all_trades().await;
            }
        }

        // Otherwise, fall back to last completed session's results
        let result_guard = self.last_result.read().await;
        if let Some(ref result) = *result_guard {
            return result.trades.clone();
        }

        Vec::new()
    }

    /// Get the result of the last completed session
    pub async fn get_result(&self) -> Option<SessionResult> {
        self.last_result.read().await.clone()
    }

    /// Get a report for the last completed session
    pub async fn get_report(&self) -> Option<PaperTradingReport> {
        let result_guard = self.last_result.read().await;
        result_guard.as_ref().map(|result| {
            PaperTradingReport {
                title: "Paper Trading Session Report".to_string(),
                period: format!(
                    "{} to {}",
                    result.start_time.format("%Y-%m-%d %H:%M"),
                    result.end_time.format("%Y-%m-%d %H:%M")
                ),
                summary: result.summary.clone(),
                trades: result.trades.clone(),
                validations: result.validations.clone(),
            }
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_trade_pnl() {
        let trade = PaperTrade {
            id: "test".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: TradeDirection::Long,
            entry_price: 10.0,
            exit_price: None,
            quantity: 100.0,
            entry_time: chrono::Utc::now(),
            exit_time: None,
            signal_id: "sig-1".to_string(),
            status: PaperTradeStatus::Open,
            realized_pnl: None,
        };

        // Price up 10% -> P&L = 100.0
        assert!((trade.unrealized_pnl(11.0) - 100.0).abs() < 0.01);
        assert!(trade.is_profitable(11.0));
        assert!((trade.return_pct(11.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_paper_trade_close() {
        let mut trade = PaperTrade {
            id: "test".to_string(),
            symbol: "000001.SZ".to_string(),
            direction: TradeDirection::Long,
            entry_price: 10.0,
            exit_price: None,
            quantity: 100.0,
            entry_time: chrono::Utc::now(),
            exit_time: None,
            signal_id: "sig-1".to_string(),
            status: PaperTradeStatus::Open,
            realized_pnl: None,
        };

        trade.close(11.0);
        assert_eq!(trade.status, PaperTradeStatus::ClosedProfit);
        assert!(trade.realized_pnl.is_some());
        assert!((trade.realized_pnl.unwrap() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_session_state() {
        let state = SessionState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("Running"));
    }

    #[test]
    fn test_paper_session_status_serialization() {
        let status = PaperSessionStatus {
            state: SessionState::Running,
            start_time: Some(chrono::Utc::now()),
            elapsed_seconds: Some(120),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Running"));
        assert!(json.contains("120"));

        // Test deserialization
        let deserialized: PaperSessionStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.state, SessionState::Running);
        assert_eq!(deserialized.elapsed_seconds, Some(120));
    }

    #[test]
    fn test_paper_session_status_idle() {
        let status = PaperSessionStatus {
            state: SessionState::Idle,
            start_time: None,
            elapsed_seconds: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Idle"));
        assert!(json.contains("null"));
    }

    #[tokio::test]
    async fn test_paper_trading_manager_new() {
        let config = zero_common::config::Config::default();
        let manager = PaperTradingManager::new(&config);

        // Initial state should be Idle
        assert_eq!(manager.get_state().await, SessionState::Idle);

        // Status should reflect idle state
        let status = manager.get_status().await;
        assert_eq!(status.state, SessionState::Idle);
        assert!(status.start_time.is_none());
        assert!(status.elapsed_seconds.is_none());
    }

    #[tokio::test]
    async fn test_paper_trading_manager_initial_state() {
        let config = zero_common::config::Config::default();
        let manager = PaperTradingManager::new(&config);

        // get_trades should return empty vec when no session
        let trades = manager.get_trades().await;
        assert!(trades.is_empty());

        // get_result should return None when no session
        let result = manager.get_result().await;
        assert!(result.is_none());

        // get_report should return None when no session
        let report = manager.get_report().await;
        assert!(report.is_none());
    }

    #[tokio::test]
    async fn test_paper_trading_manager_stop_without_running() {
        let config = zero_common::config::Config::default();
        let manager = PaperTradingManager::new(&config);

        // Stopping when not running should fail
        let result = manager.stop_session().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_paper_trading_manager_start_session_state_transition() {
        let config = zero_common::config::Config::default();
        let manager = PaperTradingManager::new(&config);

        // Initial state should be Idle
        assert_eq!(manager.get_state().await, SessionState::Idle);

        // Start a session with very short duration
        let paper_config = PaperTradingConfig {
            scan_interval_secs: 1,
            max_duration: Some(Duration::from_millis(10)),
            ..Default::default()
        };
        let result = manager.start_session(paper_config.clone(), Some(Duration::from_millis(10))).await;
        assert!(result.is_ok());

        // State should immediately transition to Running
        assert_eq!(manager.get_state().await, SessionState::Running);

        // Status should reflect running state with start_time set
        let status = manager.get_status().await;
        assert_eq!(status.state, SessionState::Running);
        assert!(status.start_time.is_some());

        // Attempting to start another session while running should fail
        let result = manager.start_session(paper_config, None).await;
        assert!(result.is_err());
    }
}
