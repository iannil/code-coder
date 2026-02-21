//! SQLite-backed memory with FTS5 and vector search.
//!
//! Features:
//! - Full-text search via FTS5 with BM25 scoring
//! - Optional vector similarity search via cosine distance
//! - Hybrid search combining both approaches
//! - Embedding cache with LRU eviction

use crate::traits::{Memory, MemoryCategory, MemoryEntry};
use crate::vector::{bytes_to_vec, cosine_similarity, hybrid_merge, vec_to_bytes};
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// SQLite memory backend with FTS5 and optional vector search.
pub struct SqliteMemory {
    db_path: PathBuf,
    embedding_cache: Arc<RwLock<EmbeddingCache>>,
}

/// LRU cache for embeddings to avoid redundant API calls.
struct EmbeddingCache {
    entries: HashMap<String, Vec<f32>>,
    order: Vec<String>,
    max_size: usize,
}

impl EmbeddingCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
            max_size,
        }
    }

    fn get(&self, key: &str) -> Option<&Vec<f32>> {
        self.entries.get(key)
    }

    fn insert(&mut self, key: String, embedding: Vec<f32>) {
        if self.entries.contains_key(&key) {
            return;
        }

        while self.order.len() >= self.max_size {
            if let Some(oldest) = self.order.first().cloned() {
                self.entries.remove(&oldest);
                self.order.remove(0);
            }
        }

        self.entries.insert(key.clone(), embedding);
        self.order.push(key);
    }

    fn remove(&mut self, key: &str) {
        self.entries.remove(key);
        self.order.retain(|k| k != key);
    }
}

impl SqliteMemory {
    /// Create a new SQLite memory at the given workspace directory.
    ///
    /// Creates the database at `{workspace}/memory/brain.db`.
    pub fn new(workspace: &Path) -> anyhow::Result<Self> {
        let memory_dir = workspace.join("memory");
        std::fs::create_dir_all(&memory_dir)?;

        let db_path = memory_dir.join("brain.db");
        let conn = Connection::open(&db_path)?;

        // Initialize schema
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                key TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                category TEXT NOT NULL,
                embedding BLOB,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                key,
                content,
                content='memories',
                content_rowid='rowid'
            );

            CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, key, content)
                VALUES (new.rowid, new.key, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content)
                VALUES ('delete', old.rowid, old.key, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content)
                VALUES ('delete', old.rowid, old.key, old.content);
                INSERT INTO memories_fts(rowid, key, content)
                VALUES (new.rowid, new.key, new.content);
            END;

            CREATE TABLE IF NOT EXISTS embedding_cache (
                key TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )?;

        Ok(Self {
            db_path,
            embedding_cache: Arc::new(RwLock::new(EmbeddingCache::new(1000))),
        })
    }

    /// Store an embedding for a key.
    pub async fn store_embedding(&self, key: &str, embedding: &[f32]) -> anyhow::Result<()> {
        let db_path = self.db_path.clone();
        let key_for_db = key.to_string();
        let key_for_cache = key.to_string();
        let bytes = vec_to_bytes(embedding);
        let embedding = embedding.to_vec();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = Connection::open(&db_path)?;
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR REPLACE INTO embedding_cache (key, embedding, created_at) VALUES (?1, ?2, ?3)",
                params![key_for_db, bytes, now],
            )?;
            Ok(())
        }).await??;

        // Update in-memory cache
        self.embedding_cache
            .write()
            .await
            .insert(key_for_cache, embedding);

        Ok(())
    }

    /// Get an embedding for a key.
    pub async fn get_embedding(&self, key: &str) -> anyhow::Result<Option<Vec<f32>>> {
        // Check in-memory cache first
        if let Some(cached) = self.embedding_cache.read().await.get(key) {
            return Ok(Some(cached.clone()));
        }

        // Check database
        let db_path = self.db_path.clone();
        let key = key.to_string();
        let key_clone = key.clone();

        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<Vec<u8>>> {
            let conn = Connection::open(&db_path)?;
            let mut stmt = conn.prepare("SELECT embedding FROM embedding_cache WHERE key = ?1")?;
            let result: Option<Vec<u8>> = stmt.query_row(params![key], |row| row.get(0)).ok();
            Ok(result)
        }).await??;

        if let Some(bytes) = result {
            let embedding = bytes_to_vec(&bytes);
            self.embedding_cache
                .write()
                .await
                .insert(key_clone, embedding.clone());
            return Ok(Some(embedding));
        }

        Ok(None)
    }

    /// Vector search using cosine similarity.
    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        let db_path = self.db_path.clone();
        let query_embedding = query_embedding.to_vec();

        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, f32)>> {
            let conn = Connection::open(&db_path)?;
            let mut stmt = conn.prepare("SELECT key, embedding FROM memories WHERE embedding IS NOT NULL")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;

            let mut results: Vec<(String, f32)> = Vec::new();
            for row in rows.flatten() {
                let embedding = bytes_to_vec(&row.1);
                let score = cosine_similarity(&query_embedding, &embedding);
                if score > 0.0 {
                    results.push((row.0, score));
                }
            }

            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(limit);
            Ok(results)
        }).await?
    }

    /// Keyword search using FTS5 BM25.
    pub async fn keyword_search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<(String, f32)>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let escaped = escape_fts5_query(query);
        let db_path = self.db_path.clone();

        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<(String, f32)>> {
            let conn = Connection::open(&db_path)?;
            let sql = format!(
                "SELECT key, -bm25(memories_fts) as score FROM memories_fts WHERE memories_fts MATCH ?1 ORDER BY score DESC LIMIT {limit}"
            );

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![escaped], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)? as f32))
            })?;

            Ok(rows.flatten().collect())
        }).await?
    }

    /// Hybrid search combining vector and keyword results.
    pub async fn hybrid_search(
        &self,
        query: &str,
        query_embedding: Option<&[f32]>,
        limit: usize,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let vector_results = if let Some(emb) = query_embedding {
            self.vector_search(emb, limit * 2).await?
        } else {
            Vec::new()
        };

        let keyword_results = self.keyword_search(query, limit * 2).await?;

        // Hybrid merge with weighted fusion
        let merged = hybrid_merge(&vector_results, &keyword_results, 0.7, 0.3, limit);

        // Fetch full entries
        let mut entries = Vec::new();
        for result in merged {
            if let Some(mut entry) = self.get(&result.id).await? {
                entry.score = result.final_score;
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Get a specific memory entry by key (internal helper).
    async fn get_entry(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let db_path = self.db_path.clone();
        let key = key.to_string();

        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<MemoryEntry>> {
            let conn = Connection::open(&db_path)?;
            let mut stmt = conn.prepare(
                "SELECT key, content, category, created_at, updated_at FROM memories WHERE key = ?1",
            )?;

            let entry = stmt
                .query_row(params![key], |row| {
                    let key: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let category_str: String = row.get(2)?;
                    let created_at: String = row.get(3)?;
                    let updated_at: String = row.get(4)?;

                    let created_ts = chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0);
                    let updated_ts = chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0);

                    Ok(MemoryEntry {
                        key,
                        content,
                        category: MemoryCategory::from(category_str.as_str()),
                        created_at: created_ts,
                        updated_at: updated_ts,
                        score: 0.0,
                    })
                })
                .ok();

            Ok(entry)
        }).await?
    }
}

/// Escape special characters for FTS5 queries.
fn escape_fts5_query(query: &str) -> String {
    // Simple word tokenization - remove special FTS5 operators
    query
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[async_trait]
impl Memory for SqliteMemory {
    fn name(&self) -> &str {
        "sqlite"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()> {
        let db_path = self.db_path.clone();
        let key = key.to_string();
        let content = content.to_string();
        let cat_str = category.to_string();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let conn = Connection::open(&db_path)?;
            let now = chrono::Utc::now().to_rfc3339();

            conn.execute(
                r#"
                INSERT INTO memories (key, content, category, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?4)
                ON CONFLICT(key) DO UPDATE SET
                    content = excluded.content,
                    category = excluded.category,
                    updated_at = excluded.updated_at
                "#,
                params![key, content, cat_str, now],
            )?;

            Ok(())
        }).await?
    }

    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
        // Use keyword search (vector search requires embedding provider)
        let keyword_results = self.keyword_search(query, limit).await?;

        let mut entries = Vec::new();
        for (key, score) in keyword_results {
            if let Some(mut entry) = self.get(&key).await? {
                entry.score = score;
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        self.get_entry(key).await
    }

    async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        let db_path = self.db_path.clone();
        let category = category.map(|c| c.to_string());

        tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<MemoryEntry>> {
            let conn = Connection::open(&db_path)?;

            let entries: Vec<MemoryEntry> = if let Some(cat_str) = category {
                let mut stmt = conn.prepare(
                    "SELECT key, content, category, created_at, updated_at FROM memories WHERE category = ?1 ORDER BY updated_at DESC",
                )?;
                let rows = stmt.query_map(params![cat_str], |row| {
                    let key: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let category_str: String = row.get(2)?;
                    let created_at: String = row.get(3)?;
                    let updated_at: String = row.get(4)?;

                    let created_ts = chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0);
                    let updated_ts = chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0);

                    Ok(MemoryEntry {
                        key,
                        content,
                        category: MemoryCategory::from(category_str.as_str()),
                        created_at: created_ts,
                        updated_at: updated_ts,
                        score: 0.0,
                    })
                })?;
                rows.flatten().collect()
            } else {
                let mut stmt = conn.prepare(
                    "SELECT key, content, category, created_at, updated_at FROM memories ORDER BY updated_at DESC",
                )?;
                let rows = stmt.query_map([], |row| {
                    let key: String = row.get(0)?;
                    let content: String = row.get(1)?;
                    let category_str: String = row.get(2)?;
                    let created_at: String = row.get(3)?;
                    let updated_at: String = row.get(4)?;

                    let created_ts = chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0);
                    let updated_ts = chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|dt| dt.timestamp_millis())
                        .unwrap_or(0);

                    Ok(MemoryEntry {
                        key,
                        content,
                        category: MemoryCategory::from(category_str.as_str()),
                        created_at: created_ts,
                        updated_at: updated_ts,
                        score: 0.0,
                    })
                })?;
                rows.flatten().collect()
            };

            Ok(entries)
        }).await?
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        let db_path = self.db_path.clone();
        let key_str = key.to_string();

        let affected = tokio::task::spawn_blocking(move || -> anyhow::Result<usize> {
            let conn = Connection::open(&db_path)?;
            let affected = conn.execute("DELETE FROM memories WHERE key = ?1", params![key_str])?;
            // Also delete from embedding cache
            let _ = conn.execute("DELETE FROM embedding_cache WHERE key = ?1", params![key_str]);
            Ok(affected)
        }).await??;

        // Remove from in-memory embedding cache
        self.embedding_cache.write().await.remove(key);

        Ok(affected > 0)
    }

    async fn count(&self, category: Option<&MemoryCategory>) -> anyhow::Result<usize> {
        let db_path = self.db_path.clone();
        let category = category.map(|c| c.to_string());

        tokio::task::spawn_blocking(move || -> anyhow::Result<usize> {
            let conn = Connection::open(&db_path)?;
            let count: i64 = match category {
                Some(cat_str) => conn.query_row(
                    "SELECT COUNT(*) FROM memories WHERE category = ?1",
                    params![cat_str],
                    |row| row.get(0),
                )?,
                None => conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?,
            };
            Ok(count as usize)
        }).await?
    }

    async fn health_check(&self) -> bool {
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || -> bool {
            Connection::open(&db_path)
                .and_then(|conn| conn.execute_batch("SELECT 1"))
                .is_ok()
        }).await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SqliteMemory) {
        let tmp = TempDir::new().unwrap();
        let mem = SqliteMemory::new(tmp.path()).unwrap();
        (tmp, mem)
    }

    #[tokio::test]
    async fn store_and_get() {
        let (_tmp, mem) = setup();

        mem.store("key1", "content1", MemoryCategory::Core)
            .await
            .unwrap();

        let entry = mem.get("key1").await.unwrap().unwrap();
        assert_eq!(entry.key, "key1");
        assert_eq!(entry.content, "content1");
        assert_eq!(entry.category, MemoryCategory::Core);
    }

    #[tokio::test]
    async fn store_updates_existing() {
        let (_tmp, mem) = setup();

        mem.store("key1", "original", MemoryCategory::Core)
            .await
            .unwrap();
        mem.store("key1", "updated", MemoryCategory::Project)
            .await
            .unwrap();

        let entry = mem.get("key1").await.unwrap().unwrap();
        assert_eq!(entry.content, "updated");
        assert_eq!(entry.category, MemoryCategory::Project);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let (_tmp, mem) = setup();
        let entry = mem.get("nonexistent").await.unwrap();
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn forget_removes_entry() {
        let (_tmp, mem) = setup();

        mem.store("key1", "content", MemoryCategory::Core)
            .await
            .unwrap();
        let deleted = mem.forget("key1").await.unwrap();
        assert!(deleted);

        let entry = mem.get("key1").await.unwrap();
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn forget_nonexistent_returns_false() {
        let (_tmp, mem) = setup();
        let deleted = mem.forget("nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn count_all() {
        let (_tmp, mem) = setup();

        mem.store("key1", "c1", MemoryCategory::Core).await.unwrap();
        mem.store("key2", "c2", MemoryCategory::Project)
            .await
            .unwrap();
        mem.store("key3", "c3", MemoryCategory::Core).await.unwrap();

        assert_eq!(mem.count(None).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn count_by_category() {
        let (_tmp, mem) = setup();

        mem.store("key1", "c1", MemoryCategory::Core).await.unwrap();
        mem.store("key2", "c2", MemoryCategory::Project)
            .await
            .unwrap();
        mem.store("key3", "c3", MemoryCategory::Core).await.unwrap();

        assert_eq!(mem.count(Some(&MemoryCategory::Core)).await.unwrap(), 2);
        assert_eq!(mem.count(Some(&MemoryCategory::Project)).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn list_all() {
        let (_tmp, mem) = setup();

        mem.store("key1", "c1", MemoryCategory::Core).await.unwrap();
        mem.store("key2", "c2", MemoryCategory::Project)
            .await
            .unwrap();

        let entries = mem.list(None).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn list_by_category() {
        let (_tmp, mem) = setup();

        mem.store("key1", "c1", MemoryCategory::Core).await.unwrap();
        mem.store("key2", "c2", MemoryCategory::Project)
            .await
            .unwrap();

        let core_entries = mem.list(Some(&MemoryCategory::Core)).await.unwrap();
        assert_eq!(core_entries.len(), 1);
        assert_eq!(core_entries[0].key, "key1");
    }

    #[tokio::test]
    async fn keyword_search_finds_matches() {
        let (_tmp, mem) = setup();

        mem.store("rust_guide", "Rust is a systems programming language", MemoryCategory::Core)
            .await
            .unwrap();
        mem.store("python_guide", "Python is a scripting language", MemoryCategory::Core)
            .await
            .unwrap();

        let results = mem.keyword_search("Rust systems", 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "rust_guide");
    }

    #[tokio::test]
    async fn keyword_search_empty_query() {
        let (_tmp, mem) = setup();
        let results = mem.keyword_search("", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn recall_uses_keyword_search() {
        let (_tmp, mem) = setup();

        mem.store("rust", "Rust programming guide", MemoryCategory::Core)
            .await
            .unwrap();

        let entries = mem.recall("Rust programming", 10).await.unwrap();
        assert!(!entries.is_empty());
        assert_eq!(entries[0].key, "rust");
    }

    #[tokio::test]
    async fn embedding_store_and_get() {
        let (_tmp, mem) = setup();

        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        mem.store_embedding("key1", &embedding).await.unwrap();

        let retrieved = mem.get_embedding("key1").await.unwrap().unwrap();
        assert_eq!(retrieved.len(), embedding.len());
    }

    #[tokio::test]
    async fn health_check_returns_true() {
        let (_tmp, mem) = setup();
        assert!(mem.health_check().await);
    }

    #[tokio::test]
    async fn name_returns_sqlite() {
        let (_tmp, mem) = setup();
        assert_eq!(mem.name(), "sqlite");
    }

    #[test]
    fn escape_fts5_special_chars() {
        let result = escape_fts5_query("hello \"world\" (test)");
        assert!(!result.contains('"'));
        assert!(!result.contains('('));
        assert!(!result.contains(')'));
    }

    #[test]
    fn escape_fts5_preserves_words() {
        let result = escape_fts5_query("hello world test");
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
        assert!(result.contains("test"));
    }
}
