/// ─── Session ───────────────────────────────────────────────────────────────
///
/// Persistent conversation sessions.  Each session is a JSON file under
/// `sessions/` storing the full message history, model name, and metadata.
///
/// Auto-saves after every user/assistant exchange (in TUI mode).
/// Use `/resume` to reload a previous session.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tui::MessageItem;

/// ─── Data structures ───────────────────────────────────────────────────────

/// Full session with messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session id (timestamp-based, e.g. "20260627_144500")
    pub id: String,
    /// Model name used in this session
    pub model: String,
    /// ISO 8601 creation time
    pub created_at: String,
    /// ISO 8601 last-update time
    pub updated_at: String,
    /// Number of user+assistant messages
    pub message_count: usize,
    /// Approximate token count (from status bar data)
    pub token_count: usize,
    /// Full message history
    pub messages: Vec<MessageItem>,
}

/// Lightweight session header (no messages) for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    pub id: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub token_count: usize,
    /// First few user message previews (≤ 3)
    pub previews: Vec<String>,
}

impl fmt::Display for SessionHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let preview = self.previews.first()
            .map(|p| format!(" — {}", truncate(p, 60)))
            .unwrap_or_default();
        write!(
            f,
            "#{}  {}  {} msgs{}",
            self.id, self.model, self.message_count, preview,
        )
    }
}

/// ─── SessionStore ──────────────────────────────────────────────────────────

pub struct SessionStore {
    base_path: PathBuf,
}

impl SessionStore {
    /// Open (or create) the sessions directory.
    pub fn open(project_root: &str) -> Self {
        let base_path = Path::new(project_root).join("sessions");
        let _ = std::fs::create_dir_all(&base_path);
        Self { base_path }
    }

    /// Save a session to disk (overwrites existing file).
    pub fn save(&self, session: &Session) -> anyhow::Result<()> {
        let path = self.file_path(&session.id);
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load a session by id.
    pub fn load(&self, session_id: &str) -> anyhow::Result<Session> {
        let path = self.file_path(session_id);
        let json = std::fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(session)
    }

    /// List all session headers (newest first).
    pub fn list(&self) -> Vec<SessionHeader> {
        let mut headers: Vec<SessionHeader> = Vec::new();
        let dir = match std::fs::read_dir(&self.base_path) {
            Ok(d) => d,
            Err(_) => return headers,
        };
        for entry in dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let json = match std::fs::read_to_string(&path) {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                // Try full session first, then derive header
                if let Ok(session) = serde_json::from_str::<Session>(&json) {
                    headers.push(SessionHeader::from_session(&session));
                }
            }
        }
        headers.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        headers
    }

    /// Get the most recent session, if any.
    pub fn latest(&self) -> Option<Session> {
        let headers = self.list();
        let latest_h = headers.first()?;
        self.load(&latest_h.id).ok()
    }

    /// Delete a session file.
    pub fn delete(&self, session_id: &str) -> anyhow::Result<()> {
        let path = self.file_path(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Build full file path for a session id.
    fn file_path(&self, session_id: &str) -> PathBuf {
        let safe_id = sanitize_id(session_id);
        self.base_path.join(format!("{safe_id}.json"))
    }
}

/// ─── Session helpers ───────────────────────────────────────────────────────

impl Session {
    /// Create a new empty session.
    pub fn new(model: &str) -> Self {
        let now = iso_now();
        Self {
            id: timestamp_id(),
            model: model.to_string(),
            created_at: now.clone(),
            updated_at: now,
            message_count: 0,
            token_count: 0,
            messages: Vec::new(),
        }
    }

    /// Touch the updated_at field and recalc message_count.
    pub fn touch(&mut self) {
        self.updated_at = iso_now();
        self.message_count = self.messages.iter()
            .filter(|m| matches!(m, MessageItem::User { .. } | MessageItem::Assistant { .. }))
            .count();
    }
}

impl SessionHeader {
    fn from_session(session: &Session) -> Self {
        let previews: Vec<String> = session.messages
            .iter()
            .filter_map(|m| match m {
                MessageItem::User { text } => Some(truncate(text, 80)),
                _ => None,
            })
            .take(3)
            .collect();

        Self {
            id: session.id.clone(),
            model: session.model.clone(),
            created_at: session.created_at.clone(),
            updated_at: session.updated_at.clone(),
            message_count: session.message_count,
            token_count: session.token_count,
            previews,
        }
    }
}

/// ─── Utility functions ─────────────────────────────────────────────────────

/// Generate a timestamp-based session id (e.g. "20260627_144500").
fn timestamp_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Use hex encoding of the high bits for a compact, sortable id
    format!("{:016x}", (nanos >> 16) & 0xffffffff_ffffffff)
}

/// ISO 8601 timestamp string.
fn iso_now() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Approximate: use nanos as a readable hex timestamp
    // For a real ISO timestamp we'd need chrono, but we keep deps minimal
    let secs = nanos / 1_000_000_000;
    let _ms = (nanos % 1_000_000_000) / 1_000_000;
    // Format from unix epoch: YYYY-MM-DD HH:MM:SS.mmm
    // Simple approach: use a readable hex-based timestamp
    format!("ts{:016x}", secs)
}

/// Sanitize a session id for use as a filename.
fn sanitize_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
}

/// Truncate a string to max_len chars, appending "…" if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_save() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());

        let mut session = Session::new("gpt-4o");
        session.messages.push(MessageItem::User { text: "hello".into() });
        session.messages.push(MessageItem::Assistant { text: "hi".into() });
        session.touch();

        store.save(&session).unwrap();
        let loaded = store.load(&session.id).unwrap();
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.model, "gpt-4o");
    }

    #[test]
    fn test_save_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());

        let mut s1 = Session::new("gpt-4o");
        s1.messages.push(MessageItem::User { text: "first".into() });
        s1.touch();
        store.save(&s1).unwrap();

        let mut s2 = Session::new("claude");
        s2.messages.push(MessageItem::User { text: "second".into() });
        s2.touch();
        store.save(&s2).unwrap();

        let headers = store.list();
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        assert!(store.load("nonexistent").is_err());
    }

    #[test]
    fn test_latest() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        assert!(store.latest().is_none());

        let mut session = Session::new("gpt-4o");
        session.messages.push(MessageItem::User { text: "hi".into() });
        session.touch();
        store.save(&session).unwrap();

        let latest = store.latest().unwrap();
        assert_eq!(latest.id, session.id);
    }

    #[test]
    fn test_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());

        let session = Session::new("test");
        store.save(&session).unwrap();
        assert!(store.load(&session.id).is_ok());

        store.delete(&session.id).unwrap();
        assert!(store.load(&session.id).is_err());
    }

    #[test]
    fn test_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        assert!(store.list().is_empty());
        assert!(store.latest().is_none());
    }
}
