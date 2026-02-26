//! Session recovery after service restart.
//!
//! This module provides recovery capabilities for trading sessions
//! that were interrupted by service restarts or crashes.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use super::{SessionState, StateStore, StoredPosition, StoredSession};
use crate::r#loop::MonitoredPosition;

/// Result of a recovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    /// Sessions recovered
    pub sessions_recovered: usize,
    /// Positions recovered
    pub positions_recovered: usize,
    /// Sessions marked as failed (unrecoverable)
    pub sessions_failed: usize,
    /// Recovery details
    pub details: Vec<RecoveryDetail>,
}

/// Detail of a single session recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryDetail {
    /// Session ID
    pub session_id: String,
    /// Recovery status
    pub status: RecoveryStatus,
    /// Number of positions recovered
    pub positions: usize,
    /// Error message if failed
    pub error: Option<String>,
}

/// Recovery status for a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStatus {
    /// Session fully recovered
    Recovered,
    /// Session recovered with warnings
    RecoveredWithWarnings,
    /// Session could not be recovered
    Failed,
    /// Session was already in terminal state
    AlreadyComplete,
}

/// Manager for session recovery operations
pub struct RecoveryManager {
    store: Arc<StateStore>,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(store: Arc<StateStore>) -> Self {
        Self { store }
    }

    /// Check for sessions that need recovery
    pub fn check_for_recovery(&self) -> Result<Vec<StoredSession>> {
        let active = self.store.get_active_sessions()?;

        // Sessions that were running or paused when service stopped
        let need_recovery: Vec<StoredSession> = active
            .into_iter()
            .filter(|s| {
                matches!(
                    s.state,
                    SessionState::Running | SessionState::Paused | SessionState::Starting
                )
            })
            .collect();

        if !need_recovery.is_empty() {
            info!(
                count = need_recovery.len(),
                "Found sessions that need recovery"
            );
        }

        Ok(need_recovery)
    }

    /// Recover all interrupted sessions
    pub fn recover_all(&self) -> Result<RecoveryResult> {
        let sessions = self.check_for_recovery()?;
        let mut result = RecoveryResult {
            sessions_recovered: 0,
            positions_recovered: 0,
            sessions_failed: 0,
            details: Vec::new(),
        };

        for session in sessions {
            let detail = self.recover_session(&session)?;

            match detail.status {
                RecoveryStatus::Recovered | RecoveryStatus::RecoveredWithWarnings => {
                    result.sessions_recovered += 1;
                    result.positions_recovered += detail.positions;
                }
                RecoveryStatus::Failed => {
                    result.sessions_failed += 1;
                }
                RecoveryStatus::AlreadyComplete => {
                    // No action needed
                }
            }

            result.details.push(detail);
        }

        info!(
            sessions_recovered = result.sessions_recovered,
            positions_recovered = result.positions_recovered,
            sessions_failed = result.sessions_failed,
            "Recovery complete"
        );

        Ok(result)
    }

    /// Recover a single session
    pub fn recover_session(&self, session: &StoredSession) -> Result<RecoveryDetail> {
        info!(session_id = %session.id, state = ?session.state, "Attempting session recovery");

        // Check if session is recoverable
        if !self.is_recoverable(session) {
            return Ok(RecoveryDetail {
                session_id: session.id.clone(),
                status: RecoveryStatus::AlreadyComplete,
                positions: 0,
                error: None,
            });
        }

        // Get open positions
        let positions = self.store.get_open_positions(&session.id)?;

        // Mark session as paused (ready to be resumed)
        self.store.update_session_state(
            &session.id,
            SessionState::Paused,
            Some("Recovered after service restart"),
        )?;

        let status = if positions.is_empty() {
            RecoveryStatus::Recovered
        } else {
            // We have open positions that need attention
            warn!(
                session_id = %session.id,
                positions = positions.len(),
                "Session recovered with open positions"
            );
            RecoveryStatus::RecoveredWithWarnings
        };

        Ok(RecoveryDetail {
            session_id: session.id.clone(),
            status,
            positions: positions.len(),
            error: None,
        })
    }

    /// Check if a session is recoverable
    fn is_recoverable(&self, session: &StoredSession) -> bool {
        matches!(
            session.state,
            SessionState::Running | SessionState::Paused | SessionState::Starting
        )
    }

    /// Get positions that need attention (from recovered sessions)
    pub fn get_positions_needing_attention(&self) -> Result<Vec<StoredPosition>> {
        let sessions = self.check_for_recovery()?;
        let mut positions = Vec::new();

        for session in sessions {
            let open = self.store.get_open_positions(&session.id)?;
            positions.extend(open);
        }

        Ok(positions)
    }

    /// Convert stored positions to monitored positions
    pub fn to_monitored_positions(&self, stored: &[StoredPosition]) -> Vec<MonitoredPosition> {
        stored
            .iter()
            .map(|p| MonitoredPosition {
                id: p.id.clone(),
                symbol: p.symbol.clone(),
                entry_price: p.entry_price,
                current_price: p.current_price,
                quantity: p.quantity,
                stop_loss: p.stop_loss,
                take_profit: p.take_profit,
                entry_time: p.entry_time,
                is_paper: true, // Will be determined by session mode
            })
            .collect()
    }

    /// Mark a session as failed (unrecoverable)
    pub fn mark_session_failed(&self, session_id: &str, reason: &str) -> Result<()> {
        self.store
            .update_session_state(session_id, SessionState::Failed, Some(reason))?;

        warn!(session_id = %session_id, reason = %reason, "Session marked as failed");
        Ok(())
    }

    /// Close all positions for a session (emergency cleanup)
    pub fn emergency_close_positions(&self, session_id: &str) -> Result<usize> {
        let positions = self.store.get_open_positions(session_id)?;
        let count = positions.len();

        for position in positions {
            // Close at current price (or entry if no current)
            let exit_price = position.current_price;
            let pnl = (exit_price - position.entry_price) * position.quantity;

            let status = if pnl >= 0.0 {
                super::PositionStatus::ClosedProfit
            } else {
                super::PositionStatus::ClosedLoss
            };

            self.store
                .close_position(&position.id, exit_price, status, pnl)?;
        }

        info!(
            session_id = %session_id,
            positions_closed = count,
            "Emergency position close completed"
        );

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r#loop::TradingMode;

    #[test]
    fn test_recovery_status_serialization() {
        let status = RecoveryStatus::RecoveredWithWarnings;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("RecoveredWithWarnings"));
    }

    #[test]
    fn test_recovery_result_serialization() {
        let result = RecoveryResult {
            sessions_recovered: 2,
            positions_recovered: 5,
            sessions_failed: 1,
            details: vec![RecoveryDetail {
                session_id: "test-1".to_string(),
                status: RecoveryStatus::Recovered,
                positions: 3,
                error: None,
            }],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("sessions_recovered"));
        assert!(json.contains("test-1"));
    }

    #[test]
    fn test_recovery_manager_creation() {
        let store = Arc::new(StateStore::in_memory().unwrap());
        let _manager = RecoveryManager::new(store);
    }

    #[test]
    fn test_recovery_check() {
        let store = Arc::new(StateStore::in_memory().unwrap());
        let manager = RecoveryManager::new(Arc::clone(&store));

        // Create a running session
        let session = StoredSession {
            id: "test-session-1".to_string(),
            state: SessionState::Running,
            mode: TradingMode::Paper,
            config: "{}".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            error_message: None,
        };
        store.create_session(&session).unwrap();

        // Check for recovery
        let needs_recovery = manager.check_for_recovery().unwrap();
        assert_eq!(needs_recovery.len(), 1);
    }

    #[test]
    fn test_recovery_recover_all() {
        let store = Arc::new(StateStore::in_memory().unwrap());
        let manager = RecoveryManager::new(Arc::clone(&store));

        // Create sessions with different states
        for (i, state) in [SessionState::Running, SessionState::Paused, SessionState::Stopped]
            .iter()
            .enumerate()
        {
            let session = StoredSession {
                id: format!("session-{}", i),
                state: *state,
                mode: TradingMode::Paper,
                config: "{}".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                error_message: None,
            };
            store.create_session(&session).unwrap();
        }

        // Recover all
        let result = manager.recover_all().unwrap();

        // Should recover 2 (Running and Paused), not Stopped
        assert_eq!(result.sessions_recovered, 2);
        assert_eq!(result.details.len(), 2);
    }
}
