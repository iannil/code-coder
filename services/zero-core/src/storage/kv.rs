//! Key-Value storage with SQLite backend
//!
//! A general-purpose KV store that replaces JSON file-based storage
//! with SQLite for ACID guarantees and better reliability.
//!
//! # Example
//!
//! ```rust,no_run
//! use zero_core::storage::KVStore;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let store = KVStore::open("~/.codecoder/storage.db").await?;
//!
//! // Store JSON value
//! store.set(&["session", "abc123"], r#"{"id": "abc123"}"#).await?;
//!
//! // Read value
//! let value = store.get(&["session", "abc123"]).await?;
//!
//! // List keys with prefix
//! let keys = store.list(&["session"]).await?;
//!
//! // Delete
//! store.delete(&["session", "abc123"]).await?;
//! # Ok(())
//! # }
//! ```

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Schema version for migrations
const SCHEMA_VERSION: i32 = 1;

/// Key-Value store backed by SQLite
pub struct KVStore {
    db_path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

/// Entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMeta {
    pub key: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub size: i64,
}

impl KVStore {
    /// Open or create a KV store at the given path
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let path_clone = db_path.clone();
        let conn = tokio::task::spawn_blocking(move || -> Result<Connection> {
            let conn = Connection::open(&path_clone)?;

            // Enable WAL mode for better concurrent access
            conn.execute_batch(
                r#"
                PRAGMA journal_mode = WAL;
                PRAGMA synchronous = NORMAL;
                PRAGMA cache_size = -64000;
                PRAGMA temp_store = MEMORY;
                "#,
            )?;

            // Initialize schema
            Self::init_schema(&conn)?;

            Ok(conn)
        })
        .await??;

        Ok(Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory store (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;

        Ok(Self {
            db_path: PathBuf::from(":memory:"),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS kv_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS kv_store (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
            );

            CREATE INDEX IF NOT EXISTS idx_kv_store_prefix
            ON kv_store (key COLLATE NOCASE);

            CREATE INDEX IF NOT EXISTS idx_kv_store_updated
            ON kv_store (updated_at DESC);
            "#,
        )?;

        // Check and run migrations
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(CAST(value AS INTEGER), 0) FROM kv_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if current_version < SCHEMA_VERSION {
            Self::run_migrations(conn, current_version)?;
            conn.execute(
                "INSERT OR REPLACE INTO kv_meta (key, value) VALUES ('schema_version', ?1)",
                params![SCHEMA_VERSION.to_string()],
            )?;
        }

        Ok(())
    }

    fn run_migrations(conn: &Connection, from_version: i32) -> Result<()> {
        // Add migrations here as needed
        let _ = (conn, from_version); // Silence unused warnings
        Ok(())
    }

    /// Set a value for a key path
    pub async fn set(&self, key: &[&str], value: &str) -> Result<()> {
        let key_str = key.join("/");
        let value = value.to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            conn.execute(
                r#"
                INSERT INTO kv_store (key, value, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?3)
                ON CONFLICT(key) DO UPDATE SET
                    value = excluded.value,
                    updated_at = excluded.updated_at
                "#,
                params![key_str, value, now],
            )?;
            Ok(())
        })
        .await?
    }

    /// Get a value by key path
    pub async fn get(&self, key: &[&str]) -> Result<Option<String>> {
        let key_str = key.join("/");
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<String>> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let result: Option<String> = conn
                .query_row(
                    "SELECT value FROM kv_store WHERE key = ?1",
                    params![key_str],
                    |row| row.get(0),
                )
                .ok();
            Ok(result)
        })
        .await?
    }

    /// Get metadata for a key
    pub async fn get_meta(&self, key: &[&str]) -> Result<Option<EntryMeta>> {
        let key_str = key.join("/");
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<Option<EntryMeta>> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let result = conn
                .query_row(
                    "SELECT key, created_at, updated_at, LENGTH(value) FROM kv_store WHERE key = ?1",
                    params![key_str],
                    |row| {
                        Ok(EntryMeta {
                            key: row.get(0)?,
                            created_at: row.get(1)?,
                            updated_at: row.get(2)?,
                            size: row.get(3)?,
                        })
                    },
                )
                .ok();
            Ok(result)
        })
        .await?
    }

    /// Delete a key
    pub async fn delete(&self, key: &[&str]) -> Result<bool> {
        let key_str = key.join("/");
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let affected = conn.execute("DELETE FROM kv_store WHERE key = ?1", params![key_str])?;
            Ok(affected > 0)
        })
        .await?
    }

    /// List keys with a prefix
    pub async fn list(&self, prefix: &[&str]) -> Result<Vec<String>> {
        let prefix_str = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.join("/"))
        };
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let mut stmt = conn.prepare("SELECT key FROM kv_store WHERE key LIKE ?1 ORDER BY key")?;
            let pattern = format!("{}%", prefix_str);

            let keys: Vec<String> = stmt
                .query_map(params![pattern], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(keys)
        })
        .await?
    }

    /// List keys with metadata
    pub async fn list_with_meta(&self, prefix: &[&str]) -> Result<Vec<EntryMeta>> {
        let prefix_str = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.join("/"))
        };
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<EntryMeta>> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let mut stmt = conn.prepare(
                "SELECT key, created_at, updated_at, LENGTH(value) FROM kv_store WHERE key LIKE ?1 ORDER BY updated_at DESC",
            )?;
            let pattern = format!("{}%", prefix_str);

            let entries: Vec<EntryMeta> = stmt
                .query_map(params![pattern], |row| {
                    Ok(EntryMeta {
                        key: row.get(0)?,
                        created_at: row.get(1)?,
                        updated_at: row.get(2)?,
                        size: row.get(3)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(entries)
        })
        .await?
    }

    /// Count entries with a prefix
    pub async fn count(&self, prefix: &[&str]) -> Result<usize> {
        let prefix_str = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.join("/"))
        };
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<usize> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let pattern = format!("{}%", prefix_str);
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM kv_store WHERE key LIKE ?1",
                params![pattern],
                |row| row.get(0),
            )?;
            Ok(count as usize)
        })
        .await?
    }

    /// Delete all keys with a prefix
    pub async fn delete_prefix(&self, prefix: &[&str]) -> Result<usize> {
        let prefix_str = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.join("/"))
        };
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<usize> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let pattern = format!("{}%", prefix_str);
            let affected = conn.execute("DELETE FROM kv_store WHERE key LIKE ?1", params![pattern])?;
            Ok(affected)
        })
        .await?
    }

    /// Check if a key exists
    pub async fn exists(&self, key: &[&str]) -> Result<bool> {
        let key_str = key.join("/");
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM kv_store WHERE key = ?1",
                params![key_str],
                |row| row.get(0),
            )?;
            Ok(exists > 0)
        })
        .await?
    }

    // ========================================================================
    // Batch Operations (for session performance)
    // ========================================================================

    /// Set multiple key-value pairs in a single transaction
    pub async fn batch_set(&self, items: Vec<(Vec<String>, String)>) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let mut conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let tx = conn.transaction()?;

            {
                let mut stmt = tx.prepare_cached(
                    r#"
                    INSERT INTO kv_store (key, value, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?3)
                    ON CONFLICT(key) DO UPDATE SET
                        value = excluded.value,
                        updated_at = excluded.updated_at
                    "#,
                )?;

                for (key, value) in items {
                    let key_str = key.join("/");
                    stmt.execute(params![key_str, value, now])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
        .await?
    }

    /// Get multiple values by key paths in a single query
    pub async fn batch_get(&self, keys: Vec<Vec<String>>) -> Result<Vec<Option<String>>> {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<Option<String>>> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            // For a reasonable number of keys, use individual queries
            // (SQLite IN clause is efficient up to ~1000 items)
            let mut results = Vec::with_capacity(keys.len());

            let mut stmt = conn.prepare_cached("SELECT value FROM kv_store WHERE key = ?1")?;

            for key in keys {
                let key_str = key.join("/");
                let value: Option<String> = stmt
                    .query_row(params![key_str], |row| row.get(0))
                    .ok();
                results.push(value);
            }

            Ok(results)
        })
        .await?
    }

    /// Delete multiple keys in a single transaction
    pub async fn batch_delete(&self, keys: Vec<Vec<String>>) -> Result<usize> {
        if keys.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<usize> {
            let mut conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let tx = conn.transaction()?;

            let mut deleted = 0;
            {
                let mut stmt = tx.prepare_cached("DELETE FROM kv_store WHERE key = ?1")?;

                for key in keys {
                    let key_str = key.join("/");
                    deleted += stmt.execute(params![key_str])?;
                }
            }

            tx.commit()?;
            Ok(deleted)
        })
        .await?
    }

    /// Get all values matching a prefix (returns key-value pairs)
    pub async fn get_prefix(&self, prefix: &[&str]) -> Result<Vec<(String, String)>> {
        let prefix_str = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", prefix.join("/"))
        };
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            let mut stmt = conn.prepare("SELECT key, value FROM kv_store WHERE key LIKE ?1 ORDER BY key")?;
            let pattern = format!("{}%", prefix_str);

            let pairs: Vec<(String, String)> = stmt
                .query_map(params![pattern], |row| Ok((row.get(0)?, row.get(1)?)))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(pairs)
        })
        .await?
    }

    /// Get the database file path
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Create a backup of the database
    pub async fn backup(&self, dest_path: impl AsRef<Path>) -> Result<()> {
        let dest = dest_path.as_ref().to_path_buf();
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            conn.execute(&format!("VACUUM INTO '{}'", dest.display()), [])
                .with_context(|| format!("Failed to backup to {:?}", dest))?;
            Ok(())
        })
        .await?
    }

    /// Compact the database (VACUUM)
    pub async fn compact(&self) -> Result<()> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
            conn.execute_batch("VACUUM")?;
            Ok(())
        })
        .await?
    }

    /// Health check
    pub async fn health_check(&self) -> bool {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().ok()?;
            conn.execute_batch("SELECT 1").ok()?;
            Some(true)
        })
        .await
        .unwrap_or(Some(false))
        .unwrap_or(false)
    }

    /// Get statistics about the store
    pub async fn stats(&self) -> Result<StoreStats> {
        let conn = self.conn.clone();

        tokio::task::spawn_blocking(move || -> Result<StoreStats> {
            let conn = conn.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            let total_entries: i64 =
                conn.query_row("SELECT COUNT(*) FROM kv_store", [], |row| row.get(0))?;

            let total_size: i64 = conn.query_row(
                "SELECT COALESCE(SUM(LENGTH(value)), 0) FROM kv_store",
                [],
                |row| row.get(0),
            )?;

            let oldest_entry: Option<i64> = conn
                .query_row(
                    "SELECT MIN(created_at) FROM kv_store",
                    [],
                    |row| row.get(0),
                )
                .ok();

            let newest_entry: Option<i64> = conn
                .query_row(
                    "SELECT MAX(updated_at) FROM kv_store",
                    [],
                    |row| row.get(0),
                )
                .ok();

            Ok(StoreStats {
                total_entries: total_entries as usize,
                total_size_bytes: total_size as usize,
                oldest_entry_ms: oldest_entry,
                newest_entry_ms: newest_entry,
            })
        })
        .await?
    }
}

// Implement Send + Sync for KVStore since we use Arc<Mutex<Connection>>
unsafe impl Send for KVStore {}
unsafe impl Sync for KVStore {}

/// Statistics about the KV store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub oldest_entry_ms: Option<i64>,
    pub newest_entry_ms: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["test", "key1"], "value1").await.unwrap();
        let value = store.get(&["test", "key1"]).await.unwrap();

        assert_eq!(value, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = KVStore::in_memory().unwrap();
        let value = store.get(&["nonexistent"]).await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["test", "key1"], "value1").await.unwrap();
        let deleted = store.delete(&["test", "key1"]).await.unwrap();
        assert!(deleted);

        let value = store.get(&["test", "key1"]).await.unwrap();
        assert!(value.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let store = KVStore::in_memory().unwrap();
        let deleted = store.delete(&["nonexistent"]).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_list() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["session", "abc"], "1").await.unwrap();
        store.set(&["session", "def"], "2").await.unwrap();
        store.set(&["project", "xyz"], "3").await.unwrap();

        let session_keys = store.list(&["session"]).await.unwrap();
        assert_eq!(session_keys.len(), 2);
        assert!(session_keys.contains(&"session/abc".to_string()));
        assert!(session_keys.contains(&"session/def".to_string()));

        let all_keys = store.list(&[]).await.unwrap();
        assert_eq!(all_keys.len(), 3);
    }

    #[tokio::test]
    async fn test_count() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["session", "abc"], "1").await.unwrap();
        store.set(&["session", "def"], "2").await.unwrap();
        store.set(&["project", "xyz"], "3").await.unwrap();

        assert_eq!(store.count(&["session"]).await.unwrap(), 2);
        assert_eq!(store.count(&["project"]).await.unwrap(), 1);
        assert_eq!(store.count(&[]).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_delete_prefix() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["session", "abc"], "1").await.unwrap();
        store.set(&["session", "def"], "2").await.unwrap();
        store.set(&["project", "xyz"], "3").await.unwrap();

        let deleted = store.delete_prefix(&["session"]).await.unwrap();
        assert_eq!(deleted, 2);

        assert_eq!(store.count(&["session"]).await.unwrap(), 0);
        assert_eq!(store.count(&["project"]).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_exists() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["test", "key"], "value").await.unwrap();

        assert!(store.exists(&["test", "key"]).await.unwrap());
        assert!(!store.exists(&["test", "nonexistent"]).await.unwrap());
    }

    #[tokio::test]
    async fn test_stats() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["test", "key1"], "value1").await.unwrap();
        store.set(&["test", "key2"], "longer_value").await.unwrap();

        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_entries, 2);
        assert!(stats.total_size_bytes > 0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let store = KVStore::in_memory().unwrap();
        assert!(store.health_check().await);
    }

    #[tokio::test]
    async fn test_update_existing() {
        let store = KVStore::in_memory().unwrap();

        store.set(&["test", "key"], "original").await.unwrap();
        store.set(&["test", "key"], "updated").await.unwrap();

        let value = store.get(&["test", "key"]).await.unwrap();
        assert_eq!(value, Some("updated".to_string()));

        // Should still be only 1 entry
        assert_eq!(store.count(&["test"]).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_json_values() {
        let store = KVStore::in_memory().unwrap();

        let json = r#"{"id": "123", "name": "test", "nested": {"value": 42}}"#;
        store.set(&["data", "obj"], json).await.unwrap();

        let value = store.get(&["data", "obj"]).await.unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&value).unwrap();
        assert_eq!(parsed["id"], "123");
    }
}
