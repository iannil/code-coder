//! Trading session management module.
//!
//! This module provides persistent session management for automated trading,
//! enabling:
//! - Session state persistence across service restarts
//! - Position tracking with stop-loss/take-profit levels
//! - Automatic session recovery
//!
//! # Architecture
//!
//! ```text
//! TradingSessionManager
//!        |
//!    +---+---+
//!    |       |
//!    v       v
//! StateStore  TradingLoop
//! (SQLite)       |
//!             +--+--+
//!             |     |
//!             v     v
//!          Paper  Live
//!          Mode   Mode
//! ```
//!
//! # Database Schema
//!
//! The module uses SQLite for persistence with two main tables:
//! - `trading_sessions`: Session metadata and configuration
//! - `positions`: Position tracking with P&L

mod manager;
mod recovery;
mod state;

pub use manager::{TradingSessionManager, SessionConfig, SessionInfo};
pub use recovery::{RecoveryManager, RecoveryResult};
pub use state::{StateStore, StoredSession, StoredPosition, PositionStatus};

use serde::{Deserialize, Serialize};

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session created but not started
    Created,
    /// Session is starting up
    Starting,
    /// Session is actively running
    Running,
    /// Session is paused (e.g., lunch break)
    Paused,
    /// Session is stopping
    Stopping,
    /// Session has stopped normally
    Stopped,
    /// Session failed with error
    Failed,
}

impl SessionState {
    /// Convert to database string
    pub fn to_db_string(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }

    /// Parse from database string
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "created" => Some(Self::Created),
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "stopping" => Some(Self::Stopping),
            "stopped" => Some(Self::Stopped),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    /// Check if session can be started
    pub fn can_start(&self) -> bool {
        matches!(self, Self::Created | Self::Stopped | Self::Failed)
    }

    /// Check if session can be paused
    pub fn can_pause(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// Check if session can be resumed
    pub fn can_resume(&self) -> bool {
        matches!(self, Self::Paused)
    }

    /// Check if session can be stopped
    pub fn can_stop(&self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Created
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_db_roundtrip() {
        let states = [
            SessionState::Created,
            SessionState::Starting,
            SessionState::Running,
            SessionState::Paused,
            SessionState::Stopping,
            SessionState::Stopped,
            SessionState::Failed,
        ];

        for state in states {
            let db_str = state.to_db_string();
            let parsed = SessionState::from_db_string(db_str);
            assert_eq!(parsed, Some(state));
        }
    }

    #[test]
    fn test_session_state_transitions() {
        assert!(SessionState::Created.can_start());
        assert!(SessionState::Stopped.can_start());
        assert!(SessionState::Failed.can_start());
        assert!(!SessionState::Running.can_start());

        assert!(SessionState::Running.can_pause());
        assert!(!SessionState::Paused.can_pause());

        assert!(SessionState::Paused.can_resume());
        assert!(!SessionState::Running.can_resume());

        assert!(SessionState::Running.can_stop());
        assert!(SessionState::Paused.can_stop());
        assert!(!SessionState::Stopped.can_stop());
    }

    #[test]
    fn test_session_state_serialization() {
        let state = SessionState::Running;
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("Running"));

        let parsed: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SessionState::Running);
    }
}
