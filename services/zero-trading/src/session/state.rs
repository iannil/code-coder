//! SQLite state persistence for trading sessions.
//!
//! This module provides durable storage for trading sessions and positions,
//! enabling recovery after service restarts.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

use super::SessionState;
use crate::r#loop::TradingMode;

/// Position status in the database
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionStatus {
    /// Position is open
    Open,
    /// Position closed with profit
    ClosedProfit,
    /// Position closed with loss
    ClosedLoss,
    /// Position closed due to stop loss
    StoppedOut,
    /// Position closed due to take profit
    TakenProfit,
    /// Position cancelled before fill
    Cancelled,
}

impl PositionStatus {
    /// Convert to database string
    pub fn to_db_string(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::ClosedProfit => "closed_profit",
            Self::ClosedLoss => "closed_loss",
            Self::StoppedOut => "stopped_out",
            Self::TakenProfit => "taken_profit",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse from database string
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "closed_profit" => Some(Self::ClosedProfit),
            "closed_loss" => Some(Self::ClosedLoss),
            "stopped_out" => Some(Self::StoppedOut),
            "taken_profit" => Some(Self::TakenProfit),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Check if position is still open
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open)
    }
}

/// Stored session record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    /// Session ID (UUID)
    pub id: String,
    /// Current state
    pub state: SessionState,
    /// Trading mode (paper or live)
    pub mode: TradingMode,
    /// Session configuration as JSON
    pub config: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Stored position record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPosition {
    /// Position ID (UUID)
    pub id: String,
    /// Session ID this position belongs to
    pub session_id: String,
    /// Symbol
    pub symbol: String,
    /// Quantity
    pub quantity: f64,
    /// Entry price
    pub entry_price: f64,
    /// Current/exit price
    pub current_price: f64,
    /// Stop loss level
    pub stop_loss: f64,
    /// Take profit level
    pub take_profit: f64,
    /// Entry timestamp
    pub entry_time: DateTime<Utc>,
    /// Exit timestamp (if closed)
    pub exit_time: Option<DateTime<Utc>>,
    /// Position status
    pub status: PositionStatus,
    /// Realized P&L (if closed)
    pub realized_pnl: Option<f64>,
}

impl StoredPosition {
    /// Calculate unrealized P&L
    pub fn unrealized_pnl(&self) -> f64 {
        (self.current_price - self.entry_price) * self.quantity
    }

    /// Calculate return percentage
    pub fn return_pct(&self) -> f64 {
        ((self.current_price - self.entry_price) / self.entry_price) * 100.0
    }
}

/// SQLite state store for trading sessions
pub struct StateStore {
    conn: Arc<Mutex<Connection>>,
}

impl StateStore {
    /// Open or create a state store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .with_context(|| format!("Failed to open database at {:?}", path.as_ref()))?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        store.init_schema()?;

        info!(path = ?path.as_ref(), "State store opened");
        Ok(store)
    }

    /// Create an in-memory state store (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        store.init_schema()?;

        debug!("In-memory state store created");
        Ok(store)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS trading_sessions (
                id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                mode TEXT NOT NULL,
                config TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                error_message TEXT
            );

            CREATE TABLE IF NOT EXISTS positions (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                symbol TEXT NOT NULL,
                quantity REAL NOT NULL,
                entry_price REAL NOT NULL,
                current_price REAL NOT NULL,
                stop_loss REAL NOT NULL,
                take_profit REAL NOT NULL,
                entry_time TEXT NOT NULL,
                exit_time TEXT,
                status TEXT NOT NULL,
                realized_pnl REAL,
                FOREIGN KEY (session_id) REFERENCES trading_sessions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_positions_session ON positions(session_id);
            CREATE INDEX IF NOT EXISTS idx_positions_status ON positions(status);
            CREATE INDEX IF NOT EXISTS idx_sessions_state ON trading_sessions(state);
            "#,
        )?;

        debug!("Database schema initialized");
        Ok(())
    }

    // ========================================================================
    // Session Operations
    // ========================================================================

    /// Create a new session
    pub fn create_session(&self, session: &StoredSession) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT INTO trading_sessions (id, state, mode, config, created_at, updated_at, error_message)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                session.id,
                session.state.to_db_string(),
                match session.mode {
                    TradingMode::Paper => "paper",
                    TradingMode::Live => "live",
                },
                session.config,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.error_message,
            ],
        )?;

        debug!(session_id = %session.id, "Session created");
        Ok(())
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Option<StoredSession>> {
        let conn = self.conn.lock().unwrap();

        let result = conn
            .query_row(
                "SELECT id, state, mode, config, created_at, updated_at, error_message FROM trading_sessions WHERE id = ?1",
                params![id],
                |row| {
                    let state_str: String = row.get(1)?;
                    let mode_str: String = row.get(2)?;
                    let created_str: String = row.get(4)?;
                    let updated_str: String = row.get(5)?;

                    Ok(StoredSession {
                        id: row.get(0)?,
                        state: SessionState::from_db_string(&state_str).unwrap_or(SessionState::Failed),
                        mode: match mode_str.as_str() {
                            "live" => TradingMode::Live,
                            _ => TradingMode::Paper,
                        },
                        config: row.get(3)?,
                        created_at: DateTime::parse_from_rfc3339(&created_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: DateTime::parse_from_rfc3339(&updated_str)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        error_message: row.get(6)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// Update session state
    pub fn update_session_state(&self, id: &str, state: SessionState, error: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            UPDATE trading_sessions
            SET state = ?1, updated_at = ?2, error_message = ?3
            WHERE id = ?4
            "#,
            params![
                state.to_db_string(),
                Utc::now().to_rfc3339(),
                error,
                id,
            ],
        )?;

        debug!(session_id = %id, state = ?state, "Session state updated");
        Ok(())
    }

    /// Get active sessions (running or paused)
    pub fn get_active_sessions(&self) -> Result<Vec<StoredSession>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, state, mode, config, created_at, updated_at, error_message
            FROM trading_sessions
            WHERE state IN ('running', 'paused', 'starting')
            ORDER BY created_at DESC
            "#,
        )?;

        let sessions = stmt
            .query_map([], |row| {
                let state_str: String = row.get(1)?;
                let mode_str: String = row.get(2)?;
                let created_str: String = row.get(4)?;
                let updated_str: String = row.get(5)?;

                Ok(StoredSession {
                    id: row.get(0)?,
                    state: SessionState::from_db_string(&state_str).unwrap_or(SessionState::Failed),
                    mode: match mode_str.as_str() {
                        "live" => TradingMode::Live,
                        _ => TradingMode::Paper,
                    },
                    config: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    error_message: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// Get recent sessions
    pub fn get_recent_sessions(&self, limit: usize) -> Result<Vec<StoredSession>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, state, mode, config, created_at, updated_at, error_message
            FROM trading_sessions
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )?;

        let sessions = stmt
            .query_map(params![limit as i64], |row| {
                let state_str: String = row.get(1)?;
                let mode_str: String = row.get(2)?;
                let created_str: String = row.get(4)?;
                let updated_str: String = row.get(5)?;

                Ok(StoredSession {
                    id: row.get(0)?,
                    state: SessionState::from_db_string(&state_str).unwrap_or(SessionState::Failed),
                    mode: match mode_str.as_str() {
                        "live" => TradingMode::Live,
                        _ => TradingMode::Paper,
                    },
                    config: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    error_message: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    // ========================================================================
    // Position Operations
    // ========================================================================

    /// Create a new position
    pub fn create_position(&self, position: &StoredPosition) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            INSERT INTO positions (
                id, session_id, symbol, quantity, entry_price, current_price,
                stop_loss, take_profit, entry_time, exit_time, status, realized_pnl
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                position.id,
                position.session_id,
                position.symbol,
                position.quantity,
                position.entry_price,
                position.current_price,
                position.stop_loss,
                position.take_profit,
                position.entry_time.to_rfc3339(),
                position.exit_time.map(|t| t.to_rfc3339()),
                position.status.to_db_string(),
                position.realized_pnl,
            ],
        )?;

        debug!(position_id = %position.id, symbol = %position.symbol, "Position created");
        Ok(())
    }

    /// Get a position by ID
    pub fn get_position(&self, id: &str) -> Result<Option<StoredPosition>> {
        let conn = self.conn.lock().unwrap();

        let result = conn
            .query_row(
                r#"
                SELECT id, session_id, symbol, quantity, entry_price, current_price,
                       stop_loss, take_profit, entry_time, exit_time, status, realized_pnl
                FROM positions WHERE id = ?1
                "#,
                params![id],
                |row| Self::row_to_position(row),
            )
            .optional()?;

        Ok(result)
    }

    /// Get all positions for a session
    pub fn get_session_positions(&self, session_id: &str) -> Result<Vec<StoredPosition>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, session_id, symbol, quantity, entry_price, current_price,
                   stop_loss, take_profit, entry_time, exit_time, status, realized_pnl
            FROM positions
            WHERE session_id = ?1
            ORDER BY entry_time DESC
            "#,
        )?;

        let positions = stmt
            .query_map(params![session_id], |row| Self::row_to_position(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(positions)
    }

    /// Get open positions for a session
    pub fn get_open_positions(&self, session_id: &str) -> Result<Vec<StoredPosition>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            r#"
            SELECT id, session_id, symbol, quantity, entry_price, current_price,
                   stop_loss, take_profit, entry_time, exit_time, status, realized_pnl
            FROM positions
            WHERE session_id = ?1 AND status = 'open'
            ORDER BY entry_time DESC
            "#,
        )?;

        let positions = stmt
            .query_map(params![session_id], |row| Self::row_to_position(row))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(positions)
    }

    /// Update position price
    pub fn update_position_price(&self, id: &str, current_price: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE positions SET current_price = ?1 WHERE id = ?2",
            params![current_price, id],
        )?;

        Ok(())
    }

    /// Close a position
    pub fn close_position(
        &self,
        id: &str,
        exit_price: f64,
        status: PositionStatus,
        realized_pnl: f64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            r#"
            UPDATE positions
            SET current_price = ?1, exit_time = ?2, status = ?3, realized_pnl = ?4
            WHERE id = ?5
            "#,
            params![
                exit_price,
                Utc::now().to_rfc3339(),
                status.to_db_string(),
                realized_pnl,
                id,
            ],
        )?;

        debug!(position_id = %id, status = ?status, pnl = realized_pnl, "Position closed");
        Ok(())
    }

    /// Helper to convert a row to StoredPosition
    fn row_to_position(row: &rusqlite::Row) -> Result<StoredPosition, rusqlite::Error> {
        let entry_time_str: String = row.get(8)?;
        let exit_time_str: Option<String> = row.get(9)?;
        let status_str: String = row.get(10)?;

        Ok(StoredPosition {
            id: row.get(0)?,
            session_id: row.get(1)?,
            symbol: row.get(2)?,
            quantity: row.get(3)?,
            entry_price: row.get(4)?,
            current_price: row.get(5)?,
            stop_loss: row.get(6)?,
            take_profit: row.get(7)?,
            entry_time: DateTime::parse_from_rfc3339(&entry_time_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            exit_time: exit_time_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            status: PositionStatus::from_db_string(&status_str).unwrap_or(PositionStatus::Open),
            realized_pnl: row.get(11)?,
        })
    }

    // ========================================================================
    // Cleanup Operations
    // ========================================================================

    /// Delete old sessions (older than days)
    pub fn cleanup_old_sessions(&self, days: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();

        let cutoff = Utc::now() - chrono::Duration::days(days);

        // Delete positions first (foreign key)
        conn.execute(
            r#"
            DELETE FROM positions
            WHERE session_id IN (
                SELECT id FROM trading_sessions
                WHERE created_at < ?1 AND state IN ('stopped', 'failed')
            )
            "#,
            params![cutoff.to_rfc3339()],
        )?;

        // Delete sessions
        let deleted = conn.execute(
            r#"
            DELETE FROM trading_sessions
            WHERE created_at < ?1 AND state IN ('stopped', 'failed')
            "#,
            params![cutoff.to_rfc3339()],
        )?;

        info!(deleted, "Cleaned up old sessions");
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_status_db_roundtrip() {
        let statuses = [
            PositionStatus::Open,
            PositionStatus::ClosedProfit,
            PositionStatus::ClosedLoss,
            PositionStatus::StoppedOut,
            PositionStatus::TakenProfit,
            PositionStatus::Cancelled,
        ];

        for status in statuses {
            let db_str = status.to_db_string();
            let parsed = PositionStatus::from_db_string(db_str);
            assert_eq!(parsed, Some(status));
        }
    }

    #[test]
    fn test_stored_position_pnl() {
        let position = StoredPosition {
            id: "test-1".to_string(),
            session_id: "session-1".to_string(),
            symbol: "000001.SZ".to_string(),
            quantity: 100.0,
            entry_price: 10.0,
            current_price: 11.0,
            stop_loss: 9.5,
            take_profit: 12.0,
            entry_time: Utc::now(),
            exit_time: None,
            status: PositionStatus::Open,
            realized_pnl: None,
        };

        assert!((position.unrealized_pnl() - 100.0).abs() < 0.01);
        assert!((position.return_pct() - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_state_store_in_memory() {
        let store = StateStore::in_memory().unwrap();

        // Create a session
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

        // Retrieve it
        let retrieved = store.get_session("test-session-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().state, SessionState::Running);
    }

    #[test]
    fn test_state_store_positions() {
        let store = StateStore::in_memory().unwrap();

        // Create a session first
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

        // Create a position
        let position = StoredPosition {
            id: "test-pos-1".to_string(),
            session_id: "test-session-1".to_string(),
            symbol: "000001.SZ".to_string(),
            quantity: 100.0,
            entry_price: 10.0,
            current_price: 10.5,
            stop_loss: 9.5,
            take_profit: 12.0,
            entry_time: Utc::now(),
            exit_time: None,
            status: PositionStatus::Open,
            realized_pnl: None,
        };

        store.create_position(&position).unwrap();

        // Get positions
        let positions = store.get_session_positions("test-session-1").unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "000001.SZ");

        // Get open positions
        let open = store.get_open_positions("test-session-1").unwrap();
        assert_eq!(open.len(), 1);

        // Close position
        store
            .close_position("test-pos-1", 11.0, PositionStatus::ClosedProfit, 100.0)
            .unwrap();

        // Should have no open positions now
        let open = store.get_open_positions("test-session-1").unwrap();
        assert_eq!(open.len(), 0);
    }

    #[test]
    fn test_state_store_active_sessions() {
        let store = StateStore::in_memory().unwrap();

        // Create sessions with different states
        for (i, state) in [
            SessionState::Running,
            SessionState::Paused,
            SessionState::Stopped,
        ]
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

        // Get active sessions
        let active = store.get_active_sessions().unwrap();
        assert_eq!(active.len(), 2); // Running and Paused
    }
}
