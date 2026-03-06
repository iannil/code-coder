//! Session persistence

use std::path::Path;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use super::message::Message;

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Session ID
    pub id: String,
    /// Session name
    pub name: Option<String>,
    /// Working directory
    pub cwd: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Messages in this session
    pub messages: Vec<Message>,
    /// Session metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl SessionData {
    /// Create a new session
    pub fn new(cwd: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: None,
            cwd: cwd.into(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }

    /// Set the session name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a message
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }
}

/// Session store using SQLite
pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    /// Open or create a session store
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open session store: {}", path.display()))?;

        // Initialize schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT,
                cwd TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                metadata TEXT
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tokens INTEGER,
                tool_call_id TEXT,
                tool_name TEXT,
                compacted INTEGER DEFAULT 0,
                metadata TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
            "#,
        )
        .with_context(|| "Failed to initialize session store schema")?;

        Ok(Self { conn })
    }

    /// Open an in-memory session store (for testing)
    pub fn in_memory() -> Result<Self> {
        Self::open(Path::new(":memory:"))
    }

    /// Save a session
    pub fn save(&self, session: &SessionData) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Upsert session
        tx.execute(
            r#"
            INSERT OR REPLACE INTO sessions (id, name, cwd, created_at, updated_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                &session.id,
                &session.name,
                &session.cwd,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                serde_json::to_string(&session.metadata)?,
            ],
        )?;

        // Delete existing messages
        tx.execute("DELETE FROM messages WHERE session_id = ?1", params![&session.id])?;

        // Insert messages
        {
            let mut stmt = tx.prepare(
                r#"
                INSERT INTO messages (id, session_id, role, content, timestamp, tokens, tool_call_id, tool_name, compacted, metadata)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )?;

            for msg in &session.messages {
                stmt.execute(params![
                    &msg.id,
                    &session.id,
                    msg.role.to_string(),
                    &msg.content,
                    msg.timestamp.to_rfc3339(),
                    msg.tokens,
                    &msg.tool_call_id,
                    &msg.tool_name,
                    msg.compacted as i32,
                    serde_json::to_string(&msg.metadata)?,
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Load a session by ID
    pub fn load(&self, session_id: &str) -> Result<Option<SessionData>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, cwd, created_at, updated_at, metadata FROM sessions WHERE id = ?1",
        )?;

        let session: Option<SessionData> = stmt
            .query_row(params![session_id], |row| {
                Ok(SessionData {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    cwd: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    messages: Vec::new(),
                })
            })
            .optional()?;

        let Some(mut session) = session else {
            return Ok(None);
        };

        // Load messages
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, role, content, timestamp, tokens, tool_call_id, tool_name, compacted, metadata
            FROM messages
            WHERE session_id = ?1
            ORDER BY timestamp ASC
            "#,
        )?;

        let messages = stmt.query_map(params![session_id], |row| {
            use super::message::MessageRole;

            let role_str: String = row.get(1)?;
            let role = match role_str.as_str() {
                "system" => MessageRole::System,
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "tool" => MessageRole::Tool,
                _ => MessageRole::User,
            };

            Ok(Message {
                id: row.get(0)?,
                role,
                content: row.get(2)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                tokens: row.get(4)?,
                tool_call_id: row.get(5)?,
                tool_name: row.get(6)?,
                compacted: row.get::<_, i32>(7)? != 0,
                metadata: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
            })
        })?;

        for msg in messages {
            session.messages.push(msg?);
        }

        Ok(Some(session))
    }

    /// List all sessions
    pub fn list(&self) -> Result<Vec<SessionData>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, cwd, created_at, updated_at, metadata FROM sessions ORDER BY updated_at DESC",
        )?;

        let sessions = stmt.query_map([], |row| {
            Ok(SessionData {
                id: row.get(0)?,
                name: row.get(1)?,
                cwd: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                messages: Vec::new(), // Don't load messages for list
            })
        })?;

        sessions.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Delete a session
    pub fn delete(&self, session_id: &str) -> Result<bool> {
        let deleted = self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            params![session_id],
        )?;
        self.conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::message::Message;

    #[test]
    fn test_session_store() {
        let store = SessionStore::in_memory().unwrap();

        // Create session
        let mut session = SessionData::new("/tmp/test");
        session.add_message(Message::user("Hello"));
        session.add_message(Message::assistant("Hi!"));

        // Save
        store.save(&session).unwrap();

        // Load
        let loaded = store.load(&session.id).unwrap().unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.messages.len(), 2);
    }

    #[test]
    fn test_session_list() {
        let store = SessionStore::in_memory().unwrap();

        let session1 = SessionData::new("/tmp/test1").with_name("Session 1");
        let session2 = SessionData::new("/tmp/test2").with_name("Session 2");

        store.save(&session1).unwrap();
        store.save(&session2).unwrap();

        let sessions = store.list().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_session_delete() {
        let store = SessionStore::in_memory().unwrap();

        let session = SessionData::new("/tmp/test");
        store.save(&session).unwrap();

        assert!(store.delete(&session.id).unwrap());
        assert!(store.load(&session.id).unwrap().is_none());
    }
}
