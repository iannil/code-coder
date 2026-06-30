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

/// Current on-disk schema version. Increment when changing the shape of
/// `Session` or `MessageItem`; add a corresponding `migrate_vN_to_vN+1`.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Default used by serde when loading a session file that omits
/// `schema_version` (i.e. files written before ADR 0004 landed).
fn default_schema_version() -> u32 {
    0
}

/// Full session with messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// ADR 0004: schema version for forward/backward compatibility.
    /// Older files (pre-ADR-0004) lack this field and default to 0; the
    /// migrate chain walks them forward to CURRENT_SCHEMA_VERSION.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Unique session id (timestamp-based, e.g. "20260627_144500")
    pub id: String,
    /// Model name used in this session
    pub model: String,
    /// ISO 8601 creation time
    #[serde(default)]
    pub created_at: String,
    /// ISO 8601 last-update time
    #[serde(default)]
    pub updated_at: String,
    /// Number of user+assistant messages
    #[serde(default)]
    pub message_count: usize,
    /// Approximate token count (from status bar data)
    #[serde(default)]
    pub token_count: usize,
    /// Full message history
    #[serde(default)]
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

    /// Load a session by id. ADR 0004: deserializes first to
    /// `serde_json::Value` so the migrate chain can transform old shapes
    /// before re-deserializing into the current `Session` struct.
    pub fn load(&self, session_id: &str) -> anyhow::Result<Session> {
        let path = self.file_path(session_id);
        let json = std::fs::read_to_string(&path)?;
        let mut value: serde_json::Value = serde_json::from_str(&json)?;
        let from_version = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(0);
        value = migrate(from_version, CURRENT_SCHEMA_VERSION, value)?;
        let session: Session = serde_json::from_value(value)?;
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
            schema_version: CURRENT_SCHEMA_VERSION,
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
    let secs = nanos / 1_000_000_000;
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

// ─── ADR 0004 — Schema Migration ────────────────────────────────────────────
//
// Walks a deserialized session `Value` from `from_version` to `to_version`
// by chaining per-step migrations. Each step is small and total — it must
// not panic, must not lose data, and must produce a `Value` matching the
// next version's shape.
//
// When incrementing CURRENT_SCHEMA_VERSION, add a `migrate_vN_to_vN+1`
// function and a match arm below. The chain handles arbitrary from→to
// jumps automatically.

/// Apply the migration chain. Returns the transformed Value.
pub fn migrate(
    from: u32,
    to: u32,
    mut value: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let mut current = from;
    while current < to {
        value = match current {
            0 => migrate_v0_to_v1(value)?,
            // 1 => migrate_v1_to_v2(value)?,  // future
            v => return Err(anyhow::anyhow!("no migrator from schema v{v}")),
        };
        current += 1;
    }
    Ok(value)
}

/// v0 → v1: stamp `schema_version: 1` onto the session.
///
/// v0 files are pre-ADR-0004 files (no schema_version field). All other
/// fields already match v1's shape because the v0 Session struct is a
/// strict subset of v1. The migration only needs to add the version tag
/// so subsequent loads short-circuit the chain.
fn migrate_v0_to_v1(mut value: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "schema_version".into(),
            serde_json::Value::Number(1.into()),
        );
    }
    Ok(value)
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

    // ─── ADR 0004 — Schema Migration ─────────────────────────────────────

    #[test]
    fn adr0004_new_session_has_current_schema_version() {
        let session = Session::new("gpt-4o");
        assert_eq!(session.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn adr0004_save_writes_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let session = Session::new("gpt-4o");
        store.save(&session).unwrap();
        let raw = std::fs::read_to_string(
            dir.path().join("sessions").join(format!("{}.json", sanitize_id(&session.id))),
        ).unwrap();
        assert!(raw.contains("\"schema_version\""), "saved JSON should include schema_version");
    }

    #[test]
    fn adr0004_loads_legacy_file_without_schema_version() {
        // v0 file: no schema_version field. Load should still succeed and
        // stamp schema_version = 1 via migrate_v0_to_v1.
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let legacy_json = r#"{
            "id": "legacy_session",
            "model": "gpt-4o",
            "created_at": "ts_old",
            "updated_at": "ts_old",
            "message_count": 1,
            "token_count": 100,
            "messages": [
                {"User": {"text": "hello from the past"}}
            ]
        }"#;
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();
        let path = sessions_dir.join("legacy_session.json");
        std::fs::write(&path, legacy_json).unwrap();

        let loaded = store.load("legacy_session").unwrap();
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.model, "gpt-4o");
        assert_eq!(loaded.messages.len(), 1);
    }

    #[test]
    fn adr0004_migrate_v0_to_v1_stamps_version() {
        let v0: serde_json::Value = serde_json::json!({
            "id": "test",
            "model": "gpt-4o",
            "messages": []
        });
        let v1 = migrate(0, 1, v0).unwrap();
        assert_eq!(v1.get("schema_version").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn adr0004_migrate_chain_handles_jumps() {
        // from=0, to=1: walks one step. Future to=2 etc. will chain more.
        let v0: serde_json::Value = serde_json::json!({"id": "x"});
        let v1 = migrate(0, 1, v0).unwrap();
        assert_eq!(v1.get("schema_version").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn adr0004_migrate_is_idempotent_when_already_at_target() {
        // from == to: no-op, returns input unchanged.
        let v: serde_json::Value = serde_json::json!({"schema_version": 1, "id": "x"});
        let out = migrate(1, 1, v.clone()).unwrap();
        assert_eq!(v, out);
    }

    #[test]
    fn adr0004_unknown_target_version_errors() {
        let v: serde_json::Value = serde_json::json!({"id": "x"});
        // No migrator from v99.
        assert!(migrate(99, 100, v).is_err());
    }

    #[test]
    fn adr0004_serde_default_fills_missing_optional_fields() {
        // A file missing created_at/updated_at/etc should still load via
        // serde defaults — they default to empty strings / 0.
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let minimal_json = r#"{
            \"id\": \"min\",
            \"model\": \"gpt-4o\",
            \"messages\": []
        }"#.replace("\\\"", "\"");
        let sessions_dir = dir.path().join("sessions");
        std::fs::create_dir_all(&sessions_dir).unwrap();
        let path = sessions_dir.join("min.json");
        std::fs::write(&path, minimal_json).unwrap();

        let loaded = store.load("min").unwrap();
        assert_eq!(loaded.created_at, "");
        assert_eq!(loaded.message_count, 0);
        assert_eq!(loaded.token_count, 0);
    }

    #[test]
    fn adr0004_roundtrip_preserves_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let mut session = Session::new("gpt-4o");
        session.messages.push(MessageItem::User { text: "round".into() });
        session.touch();
        store.save(&session).unwrap();

        let loaded = store.load(&session.id).unwrap();
        assert_eq!(loaded.schema_version, session.schema_version);
        assert_eq!(loaded.messages.len(), 1);
    }

    // ─── ADR 0004 — Debounce / Background Save ───────────────────────────

    #[test]
    fn adr0004_auto_save_does_not_immediately_save() {
        // mark_dirty only — file should NOT exist immediately after.
        use crate::tui::TuiApp;
        use crate::tui::commands::{auto_save_session, build_session_from_app};

        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let mut app = TuiApp::default();
        app.session_store = Some(store);
        app.messages.push(MessageItem::User { text: "hi".into() });

        auto_save_session(&mut app);
        assert!(app.dirty, "dirty flag must be set");
        assert!(app.last_dirty_at.is_some());

        // No file exists yet — no synchronous save ran.
        let session = build_session_from_app(&app);
        let path = dir.path().join("sessions").join(format!("{}.json", sanitize_id(&session.id)));
        assert!(!path.exists(), "no file should exist immediately after mark_dirty");
    }

    #[test]
    fn adr0004_flush_pending_save_writes_when_debounce_elapsed() {
        use crate::tui::TuiApp;
        use crate::tui::commands::{auto_save_session, flush_pending_save};

        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let mut app = TuiApp::default();
        app.session_store = Some(store);
        app.messages.push(MessageItem::User { text: "flush me".into() });
        auto_save_session(&mut app);

        // Simulate the debounce window elapsing by backdating last_dirty_at.
        app.last_dirty_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(10));

        flush_pending_save(&mut app);
        assert!(!app.dirty, "flush should clear dirty");
        assert!(app.last_dirty_at.is_none());

        // Session file should now exist in dir/sessions/.
        let sessions_dir = dir.path().join("sessions");
        let entries: Vec<_> = std::fs::read_dir(&sessions_dir).unwrap().collect();
        assert_eq!(entries.len(), 1, "exactly one session file should exist");
    }

    #[test]
    fn adr0004_flush_skipped_within_debounce_window() {
        use crate::tui::TuiApp;
        use crate::tui::commands::{auto_save_session, flush_pending_save};

        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let mut app = TuiApp::default();
        app.session_store = Some(store);
        app.messages.push(MessageItem::User { text: "x".into() });
        auto_save_session(&mut app);

        // Within the 5s window — flush should be a no-op.
        flush_pending_save(&mut app);
        assert!(app.dirty, "dirty should remain within debounce window");

        // And no file written.
        let sessions_dir = dir.path().join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        let entries: Vec<_> = std::fs::read_dir(&sessions_dir).unwrap().collect();
        assert!(entries.is_empty(), "no file should be written within window");
    }

    #[test]
    fn adr0004_multiple_marks_collapse_into_one_save() {
        use crate::tui::TuiApp;
        use crate::tui::commands::{auto_save_session, flush_pending_save};

        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let mut app = TuiApp::default();
        app.session_store = Some(store);

        for i in 0..5 {
            app.messages.push(MessageItem::User { text: format!("msg{i}") });
            auto_save_session(&mut app);
        }

        // Backdate and flush.
        app.last_dirty_at = Some(std::time::Instant::now() - std::time::Duration::from_secs(10));
        flush_pending_save(&mut app);

        // Exactly one session file (the latest snapshot) in dir/sessions/.
        let sessions_dir = dir.path().join("sessions");
        let entries: Vec<_> = std::fs::read_dir(&sessions_dir).unwrap().collect();
        assert_eq!(entries.len(), 1, "5 marks should collapse to 1 save");
    }

    #[test]
    fn adr0004_no_store_means_mark_dirty_is_noop() {
        // When session_store is None, mark_dirty should not crash and
        // should not set the flag (nothing to save).
        use crate::tui::TuiApp;
        use crate::tui::commands::auto_save_session;
        let mut app = TuiApp::default();
        app.session_store = None;
        auto_save_session(&mut app);
        // Flag stays false because we short-circuit on None store.
        // (auto_save_session's "if app.session_store.is_none() return" path.)
        assert!(!app.dirty, "no-op when no store");
    }

    #[test]
    fn adr0004_background_thread_writes_sessions_off_main() {
        // End-to-end: spawn real background thread, send snapshot, verify
        // it gets written. Simulates the production main-loop path.
        use crate::tui::commands::spawn_save_thread;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::open(dir.path().to_str().unwrap());
        let tx = spawn_save_thread(store);

        let mut session = Session::new("gpt-4o");
        session.messages.push(MessageItem::User { text: "bg".into() });
        session.touch();

        let expected_path = dir.path().join("sessions").join(format!("{}.json", sanitize_id(&session.id)));
        tx.send(session).unwrap();
        // Drop sender to signal EOF so the thread exits and flushes.
        drop(tx);

        // Spin briefly until the file appears (thread may need a moment).
        let done = Arc::new(AtomicBool::new(false));
        let done_clone = done.clone();
        let watcher = std::thread::spawn(move || {
            for _ in 0..50 {
                if done_clone.load(Ordering::Relaxed) { return; }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        for _ in 0..50 {
            if expected_path.exists() { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        done.store(true, Ordering::Relaxed);
        let _ = watcher.join();

        assert!(expected_path.exists(), "background thread should write the session file");
    }
}
