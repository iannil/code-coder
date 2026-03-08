//! Quota management for Zero Gateway.
//!
//! Tracks token usage and enforces limits.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Quota manager for tracking and enforcing usage limits.
pub struct QuotaManager {
    conn: Arc<Mutex<Connection>>,
}

/// Usage record for a user.
#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub user_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub requests: i64,
    pub period_start: DateTime<Utc>,
}

/// User quota limits.
#[derive(Debug, Clone)]
pub struct QuotaLimits {
    pub daily_input_tokens: i64,
    pub daily_output_tokens: i64,
    pub daily_requests: i64,
    pub monthly_input_tokens: i64,
    pub monthly_output_tokens: i64,
}

impl Default for QuotaLimits {
    fn default() -> Self {
        Self {
            daily_input_tokens: 1_000_000,     // 1M tokens/day
            daily_output_tokens: 500_000,      // 500K tokens/day
            daily_requests: 1000,              // 1000 requests/day
            monthly_input_tokens: 10_000_000,  // 10M tokens/month
            monthly_output_tokens: 5_000_000,  // 5M tokens/month
        }
    }
}

impl QuotaManager {
    /// Create a new quota manager with the given database path.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create tables if they don't exist
        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS usage_daily (
                user_id TEXT NOT NULL,
                date TEXT NOT NULL,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                requests INTEGER DEFAULT 0,
                PRIMARY KEY (user_id, date)
            );

            CREATE TABLE IF NOT EXISTS usage_monthly (
                user_id TEXT NOT NULL,
                month TEXT NOT NULL,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                requests INTEGER DEFAULT 0,
                PRIMARY KEY (user_id, month)
            );

            CREATE TABLE IF NOT EXISTS quota_limits (
                user_id TEXT PRIMARY KEY,
                daily_input_tokens INTEGER,
                daily_output_tokens INTEGER,
                daily_requests INTEGER,
                monthly_input_tokens INTEGER,
                monthly_output_tokens INTEGER
            );
            ",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Record token usage for a user.
    pub fn record_usage(
        &self,
        user_id: &str,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let month = Utc::now().format("%Y-%m").to_string();

        // Update daily usage
        conn.execute(
            r"
            INSERT INTO usage_daily (user_id, date, input_tokens, output_tokens, requests)
            VALUES (?1, ?2, ?3, ?4, 1)
            ON CONFLICT(user_id, date) DO UPDATE SET
                input_tokens = input_tokens + ?3,
                output_tokens = output_tokens + ?4,
                requests = requests + 1
            ",
            params![user_id, today, input_tokens, output_tokens],
        )?;

        // Update monthly usage
        conn.execute(
            r"
            INSERT INTO usage_monthly (user_id, month, input_tokens, output_tokens, requests)
            VALUES (?1, ?2, ?3, ?4, 1)
            ON CONFLICT(user_id, month) DO UPDATE SET
                input_tokens = input_tokens + ?3,
                output_tokens = output_tokens + ?4,
                requests = requests + 1
            ",
            params![user_id, month, input_tokens, output_tokens],
        )?;

        Ok(())
    }

    /// Check if a user is within their quota limits.
    pub fn check_quota(&self, user_id: &str) -> Result<bool> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let month = Utc::now().format("%Y-%m").to_string();

        // Get limits (or use defaults)
        let limits = self.get_limits_internal(&conn, user_id)?;

        // Check daily usage
        let daily: (i64, i64, i64) = conn
            .query_row(
                "SELECT COALESCE(input_tokens, 0), COALESCE(output_tokens, 0), COALESCE(requests, 0)
                 FROM usage_daily WHERE user_id = ?1 AND date = ?2",
                params![user_id, today],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap_or((0, 0, 0));

        if daily.0 >= limits.daily_input_tokens
            || daily.1 >= limits.daily_output_tokens
            || daily.2 >= limits.daily_requests
        {
            return Ok(false);
        }

        // Check monthly usage
        let monthly: (i64, i64) = conn
            .query_row(
                "SELECT COALESCE(input_tokens, 0), COALESCE(output_tokens, 0)
                 FROM usage_monthly WHERE user_id = ?1 AND month = ?2",
                params![user_id, month],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or((0, 0));

        if monthly.0 >= limits.monthly_input_tokens || monthly.1 >= limits.monthly_output_tokens {
            return Ok(false);
        }

        Ok(true)
    }

    /// Get quota limits for a user.
    pub fn get_limits(&self, user_id: &str) -> Result<QuotaLimits> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        self.get_limits_internal(&conn, user_id)
    }

    fn get_limits_internal(&self, conn: &Connection, user_id: &str) -> Result<QuotaLimits> {
        let result = conn.query_row(
            "SELECT daily_input_tokens, daily_output_tokens, daily_requests,
                    monthly_input_tokens, monthly_output_tokens
             FROM quota_limits WHERE user_id = ?1",
            params![user_id],
            |row| {
                Ok(QuotaLimits {
                    daily_input_tokens: row.get(0)?,
                    daily_output_tokens: row.get(1)?,
                    daily_requests: row.get(2)?,
                    monthly_input_tokens: row.get(3)?,
                    monthly_output_tokens: row.get(4)?,
                })
            },
        );

        match result {
            Ok(limits) => Ok(limits),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(QuotaLimits::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Set quota limits for a user.
    pub fn set_limits(&self, user_id: &str, limits: &QuotaLimits) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;

        conn.execute(
            r"
            INSERT INTO quota_limits
                (user_id, daily_input_tokens, daily_output_tokens, daily_requests,
                 monthly_input_tokens, monthly_output_tokens)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(user_id) DO UPDATE SET
                daily_input_tokens = ?2,
                daily_output_tokens = ?3,
                daily_requests = ?4,
                monthly_input_tokens = ?5,
                monthly_output_tokens = ?6
            ",
            params![
                user_id,
                limits.daily_input_tokens,
                limits.daily_output_tokens,
                limits.daily_requests,
                limits.monthly_input_tokens,
                limits.monthly_output_tokens,
            ],
        )?;

        Ok(())
    }

    /// Get daily usage for a user.
    pub fn get_daily_usage(&self, user_id: &str) -> Result<UsageRecord> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let result = conn.query_row(
            "SELECT input_tokens, output_tokens, requests
             FROM usage_daily WHERE user_id = ?1 AND date = ?2",
            params![user_id, today],
            |row| {
                Ok(UsageRecord {
                    user_id: user_id.to_string(),
                    input_tokens: row.get(0)?,
                    output_tokens: row.get(1)?,
                    requests: row.get(2)?,
                    period_start: Utc::now(),
                })
            },
        );

        match result {
            Ok(record) => Ok(record),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(UsageRecord {
                user_id: user_id.to_string(),
                input_tokens: 0,
                output_tokens: 0,
                requests: 0,
                period_start: Utc::now(),
            }),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_quota_manager_basic() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("quota.db");
        let manager = QuotaManager::new(&db_path).unwrap();

        // Initially should be within quota
        assert!(manager.check_quota("user1").unwrap());

        // Record some usage
        manager.record_usage("user1", 1000, 500).unwrap();

        // Should still be within quota
        assert!(manager.check_quota("user1").unwrap());

        // Check usage was recorded
        let usage = manager.get_daily_usage("user1").unwrap();
        assert_eq!(usage.input_tokens, 1000);
        assert_eq!(usage.output_tokens, 500);
        assert_eq!(usage.requests, 1);
    }

    #[test]
    fn test_quota_limits() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("quota.db");
        let manager = QuotaManager::new(&db_path).unwrap();

        // Set very low limits
        let limits = QuotaLimits {
            daily_input_tokens: 100,
            daily_output_tokens: 50,
            daily_requests: 2,
            monthly_input_tokens: 1000,
            monthly_output_tokens: 500,
        };
        manager.set_limits("user2", &limits).unwrap();

        // Use up quota
        manager.record_usage("user2", 50, 25).unwrap();
        assert!(manager.check_quota("user2").unwrap());

        manager.record_usage("user2", 50, 25).unwrap();
        // Now at limit
        assert!(!manager.check_quota("user2").unwrap());
    }
}
