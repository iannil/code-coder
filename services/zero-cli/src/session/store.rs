//! `SQLite`-backed session storage for conversation persistence.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::types::{estimate_tokens, MessageRole, SessionMessage};
use anyhow::Result;
use chrono::Local;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// `SQLite` session store for conversation persistence.
///
/// Database path: `{workspace}/sessions.db`
///
/// Stores messages per `session_key` (format: `{channel}:{sender}`)
pub struct SessionStore {
    conn: Mutex<Connection>,
}

impl SessionStore {
    /// Create a new session store at the given database path.
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Initialize database schema.
    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_key     TEXT NOT NULL,
                role            TEXT NOT NULL,
                content         TEXT NOT NULL,
                token_estimate  INTEGER NOT NULL,
                created_at      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_session_key ON sessions(session_key);
            CREATE INDEX IF NOT EXISTS idx_session_created ON sessions(session_key, created_at);",
        )?;
        Ok(())
    }

    /// Get all messages for a session, ordered by creation time.
    pub fn get_messages(&self, session_key: &str) -> Result<Vec<SessionMessage>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {e}"))?;

        let mut stmt = conn.prepare(
            "SELECT id, role, content, created_at, token_estimate
             FROM sessions
             WHERE session_key = ?1
             ORDER BY created_at ASC, id ASC",
        )?;

        let rows = stmt.query_map(params![session_key], |row| {
            Ok(SessionMessage {
                id: row.get(0)?,
                role: MessageRole::parse(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                timestamp: row.get(3)?,
                token_estimate: row.get::<_, i64>(4)? as usize,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    /// Add a message to the session.
    pub fn add_message(
        &self,
        session_key: &str,
        role: MessageRole,
        content: &str,
    ) -> Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {e}"))?;

        let now = Local::now().timestamp();
        let token_estimate = estimate_tokens(content) as i64;

        conn.execute(
            "INSERT INTO sessions (session_key, role, content, token_estimate, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_key, role.as_str(), content, token_estimate, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Clear all messages for a session (used by /new command).
    pub fn clear_session(&self, session_key: &str) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {e}"))?;

        let affected = conn.execute(
            "DELETE FROM sessions WHERE session_key = ?1",
            params![session_key],
        )?;

        Ok(affected)
    }

    /// Compact a session: delete old messages and insert a summary.
    ///
    /// Keeps the most recent `keep_recent` messages and prepends a system summary.
    pub fn compact_session(
        &self,
        session_key: &str,
        summary: &str,
        keep_recent: usize,
    ) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {e}"))?;

        // Get IDs of messages to keep (most recent N)
        let keep_ids: Vec<i64> = {
            let mut stmt = conn.prepare(
                "SELECT id FROM sessions
                 WHERE session_key = ?1
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![session_key, keep_recent as i64], |row| {
                row.get::<_, i64>(0)
            })?;
            rows.filter_map(Result::ok).collect()
        };

        // Delete all messages except the ones to keep
        let deleted = if keep_ids.is_empty() {
            conn.execute(
                "DELETE FROM sessions WHERE session_key = ?1",
                params![session_key],
            )?
        } else {
            // Build IN clause for IDs to keep
            let placeholders: Vec<String> = (0..keep_ids.len())
                .map(|i| format!("?{}", i + 2))
                .collect();
            let sql = format!(
                "DELETE FROM sessions WHERE session_key = ?1 AND id NOT IN ({})",
                placeholders.join(", ")
            );
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(session_key.to_string())];
            for id in &keep_ids {
                params_vec.push(Box::new(*id));
            }
            let params_ref: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(AsRef::as_ref).collect();
            conn.execute(&sql, params_ref.as_slice())?
        };

        // Insert summary as a system message at the beginning
        if !summary.trim().is_empty() {
            // Use a timestamp earlier than any kept message
            let earliest_timestamp = if keep_ids.is_empty() {
                Local::now().timestamp()
            } else {
                let mut stmt = conn.prepare(
                    "SELECT MIN(created_at) FROM sessions WHERE session_key = ?1",
                )?;
                stmt.query_row(params![session_key], |row| row.get::<_, i64>(0))
                    .unwrap_or_else(|_| Local::now().timestamp())
                    - 1
            };

            let token_estimate = estimate_tokens(summary) as i64;
            conn.execute(
                "INSERT INTO sessions (session_key, role, content, token_estimate, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    session_key,
                    MessageRole::System.as_str(),
                    summary,
                    token_estimate,
                    earliest_timestamp
                ],
            )?;
        }

        Ok(deleted)
    }

    /// Get total estimated token count for a session.
    pub fn get_token_count(&self, session_key: &str) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {e}"))?;

        let count: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(token_estimate), 0) FROM sessions WHERE session_key = ?1",
                params![session_key],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(count as usize)
    }

    /// Get the number of messages in a session.
    pub fn get_message_count(&self, session_key: &str) -> Result<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock error: {e}"))?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE session_key = ?1",
            params![session_key],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Health check: verify database is accessible.
    pub fn health_check(&self) -> bool {
        self.conn
            .lock()
            .map(|c| c.execute_batch("SELECT 1").is_ok())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_store() -> (TempDir, SessionStore) {
        let tmp = TempDir::new().unwrap();
        let store = SessionStore::new(&tmp.path().join("sessions.db")).unwrap();
        (tmp, store)
    }

    #[test]
    fn test_health_check() {
        let (_tmp, store) = temp_store();
        assert!(store.health_check());
    }

    #[test]
    fn test_add_and_get_messages() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";

        store.add_message(key, MessageRole::User, "Hello").unwrap();
        store
            .add_message(key, MessageRole::Assistant, "Hi there!")
            .unwrap();

        let messages = store.get_messages(key).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, MessageRole::Assistant);
        assert_eq!(messages[1].content, "Hi there!");
    }

    #[test]
    fn test_session_isolation() {
        let (_tmp, store) = temp_store();

        store
            .add_message("telegram:user1", MessageRole::User, "User 1 message")
            .unwrap();
        store
            .add_message("telegram:user2", MessageRole::User, "User 2 message")
            .unwrap();

        let user1_msgs = store.get_messages("telegram:user1").unwrap();
        let user2_msgs = store.get_messages("telegram:user2").unwrap();

        assert_eq!(user1_msgs.len(), 1);
        assert_eq!(user2_msgs.len(), 1);
        assert_eq!(user1_msgs[0].content, "User 1 message");
        assert_eq!(user2_msgs[0].content, "User 2 message");
    }

    #[test]
    fn test_clear_session() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";

        store.add_message(key, MessageRole::User, "Message 1").unwrap();
        store.add_message(key, MessageRole::User, "Message 2").unwrap();

        assert_eq!(store.get_message_count(key).unwrap(), 2);

        let deleted = store.clear_session(key).unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(store.get_message_count(key).unwrap(), 0);
    }

    #[test]
    fn test_get_token_count() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";

        // Empty session
        assert_eq!(store.get_token_count(key).unwrap(), 0);

        // Add messages
        store
            .add_message(key, MessageRole::User, "Hello world")
            .unwrap(); // ~3 tokens
        store
            .add_message(key, MessageRole::Assistant, "Hi there!")
            .unwrap(); // ~3 tokens

        let count = store.get_token_count(key).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_compact_session() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";

        // Add 10 messages
        for i in 0..10 {
            store
                .add_message(key, MessageRole::User, &format!("Message {i}"))
                .unwrap();
        }

        assert_eq!(store.get_message_count(key).unwrap(), 10);

        // Compact, keeping 3 recent messages
        let deleted = store
            .compact_session(key, "Summary of previous conversation", 3)
            .unwrap();

        assert_eq!(deleted, 7); // 10 - 3 = 7 deleted

        let messages = store.get_messages(key).unwrap();
        // Should have: 1 summary + 3 recent = 4 messages
        assert_eq!(messages.len(), 4);

        // First message should be the summary
        assert_eq!(messages[0].role, MessageRole::System);
        assert!(messages[0].content.contains("Summary"));

        // Last 3 should be the recent user messages
        assert!(messages[3].content.contains("Message 9"));
    }

    #[test]
    fn test_compact_session_empty_summary() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";

        for i in 0..5 {
            store
                .add_message(key, MessageRole::User, &format!("Message {i}"))
                .unwrap();
        }

        // Compact with empty summary
        store.compact_session(key, "", 2).unwrap();

        let messages = store.get_messages(key).unwrap();
        // Should have: 2 recent (no summary added for empty string)
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, MessageRole::User);
    }

    #[test]
    fn test_messages_ordered_by_time() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";

        store.add_message(key, MessageRole::User, "First").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.add_message(key, MessageRole::Assistant, "Second").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.add_message(key, MessageRole::User, "Third").unwrap();

        let messages = store.get_messages(key).unwrap();
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
        assert_eq!(messages[2].content, "Third");
    }

    #[test]
    fn test_empty_session() {
        let (_tmp, store) = temp_store();
        let messages = store.get_messages("nonexistent:session").unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_unicode_content() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";
        let content = "你好世界 🚀 مرحبا";

        store.add_message(key, MessageRole::User, content).unwrap();

        let messages = store.get_messages(key).unwrap();
        assert_eq!(messages[0].content, content);
    }

    #[test]
    fn test_long_content() {
        let (_tmp, store) = temp_store();
        let key = "telegram:user123";
        let content = "x".repeat(100_000);

        store.add_message(key, MessageRole::User, &content).unwrap();

        let messages = store.get_messages(key).unwrap();
        assert_eq!(messages[0].content.len(), 100_000);
    }

    #[test]
    fn test_persistence() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("sessions.db");
        let key = "telegram:user123";

        // Create store and add message
        {
            let store = SessionStore::new(&db_path).unwrap();
            store.add_message(key, MessageRole::User, "Persistent").unwrap();
        }

        // Reopen and verify
        {
            let store = SessionStore::new(&db_path).unwrap();
            let messages = store.get_messages(key).unwrap();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].content, "Persistent");
        }
    }
}
