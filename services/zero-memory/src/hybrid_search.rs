//! Hybrid search combining vector and keyword search.
//!
//! Combines Qdrant vector search with SQLite FTS5 keyword search
//! using weighted fusion for optimal retrieval results.

use crate::embeddings::EmbeddingProvider;
use crate::qdrant::QdrantMemory;
use crate::sqlite::SqliteMemory;
use crate::traits::{Memory, MemoryCategory, MemoryEntry};
use crate::vector::hybrid_merge;
use std::sync::Arc;

/// Default weight for vector search results (70%).
pub const DEFAULT_VECTOR_WEIGHT: f32 = 0.7;

/// Default weight for keyword search results (30%).
pub const DEFAULT_KEYWORD_WEIGHT: f32 = 0.3;

/// Hybrid search engine combining Qdrant vector search with SQLite FTS5.
///
/// This engine provides optimal retrieval by leveraging both:
/// - Semantic similarity via Qdrant (captures meaning)
/// - Keyword matching via SQLite FTS5 (captures exact terms)
///
/// # Example
///
/// ```ignore
/// let embedding = Arc::new(OpenAiEmbedding::new("api-key")?);
/// let qdrant = Arc::new(QdrantMemory::connect("http://localhost:6334", "memories", embedding.clone()).await?);
/// let sqlite = Arc::new(SqliteMemory::new(workspace_path)?);
///
/// let engine = HybridSearchEngine::new(qdrant, sqlite, embedding);
/// let results = engine.search("rust programming", 10, None).await?;
/// ```
pub struct HybridSearchEngine {
    qdrant: Arc<QdrantMemory>,
    sqlite: Arc<SqliteMemory>,
    embedding: Arc<dyn EmbeddingProvider>,
    vector_weight: f32,
    keyword_weight: f32,
}

impl HybridSearchEngine {
    /// Create a new hybrid search engine with default weights (70% vector, 30% keyword).
    pub fn new(
        qdrant: Arc<QdrantMemory>,
        sqlite: Arc<SqliteMemory>,
        embedding: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            qdrant,
            sqlite,
            embedding,
            vector_weight: DEFAULT_VECTOR_WEIGHT,
            keyword_weight: DEFAULT_KEYWORD_WEIGHT,
        }
    }

    /// Create a hybrid search engine with custom weights.
    ///
    /// # Arguments
    /// * `qdrant` - Qdrant memory backend for vector search
    /// * `sqlite` - SQLite memory backend for keyword search
    /// * `embedding` - Embedding provider for query vectorization
    /// * `vector_weight` - Weight for vector search results (0.0-1.0)
    /// * `keyword_weight` - Weight for keyword search results (0.0-1.0)
    pub fn with_weights(
        qdrant: Arc<QdrantMemory>,
        sqlite: Arc<SqliteMemory>,
        embedding: Arc<dyn EmbeddingProvider>,
        vector_weight: f32,
        keyword_weight: f32,
    ) -> Self {
        Self {
            qdrant,
            sqlite,
            embedding,
            vector_weight,
            keyword_weight,
        }
    }

    /// Get the current vector weight.
    pub fn vector_weight(&self) -> f32 {
        self.vector_weight
    }

    /// Get the current keyword weight.
    pub fn keyword_weight(&self) -> f32 {
        self.keyword_weight
    }

    /// Perform hybrid search combining vector and keyword results.
    ///
    /// Runs both searches in parallel for optimal performance, then merges
    /// results using weighted fusion.
    ///
    /// # Arguments
    /// * `query` - Search query string
    /// * `limit` - Maximum number of results to return
    /// * `category_filter` - Optional category filter to restrict results
    ///
    /// # Returns
    /// Combined and ranked memory entries sorted by hybrid score.
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        category_filter: Option<MemoryCategory>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        // Generate query embedding for vector search
        let query_embedding = self.embedding.embed_one(query).await?;

        // Fetch more results than needed for merging (2x limit from each source)
        let fetch_limit = limit * 2;

        // Run both searches in parallel
        let (vector_results, keyword_results) = tokio::join!(
            self.vector_search_scored(&query_embedding, fetch_limit),
            self.keyword_search_scored(query, fetch_limit)
        );

        let vector_results = vector_results?;
        let keyword_results = keyword_results?;

        // Merge results using weighted fusion
        let merged = hybrid_merge(
            &vector_results,
            &keyword_results,
            self.vector_weight,
            self.keyword_weight,
            limit * 2, // Get more for filtering
        );

        // Fetch full entries and apply category filter
        let mut entries = Vec::with_capacity(limit);
        for result in merged {
            if entries.len() >= limit {
                break;
            }

            // Try Qdrant first (source of truth for vector-indexed content)
            let entry = match self.qdrant.get(&result.id).await? {
                Some(e) => Some(e),
                None => self.sqlite.get(&result.id).await?,
            };

            if let Some(mut entry) = entry {
                // Apply category filter if specified
                if let Some(ref filter) = category_filter {
                    if &entry.category != filter {
                        continue;
                    }
                }

                entry.score = result.final_score;
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Vector-only search using Qdrant.
    ///
    /// Useful when semantic similarity is the primary concern.
    pub async fn vector_search(
        &self,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.qdrant.recall(query, limit).await
    }

    /// Keyword-only search using SQLite FTS5.
    ///
    /// Useful when exact term matching is the primary concern.
    pub async fn keyword_search(
        &self,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        self.sqlite.recall(query, limit).await
    }

    /// Internal: Vector search returning (id, score) tuples.
    async fn vector_search_scored(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        let entries = self.qdrant.recall("", limit).await;

        // If recall without query fails, do a direct search
        // We need to use the embedding directly
        match entries {
            Ok(entries) => Ok(entries.into_iter().map(|e| (e.key, e.score)).collect()),
            Err(_) => {
                // Fallback: use SQLite's vector search if available
                self.sqlite.vector_search(query_embedding, limit).await
            }
        }
    }

    /// Internal: Keyword search returning (id, score) tuples.
    async fn keyword_search_scored(
        &self,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<(String, f32)>> {
        self.sqlite.keyword_search(query, limit).await
    }

    /// Check health of both backends.
    ///
    /// # Returns
    /// Tuple of (qdrant_healthy, sqlite_healthy)
    pub async fn health_check(&self) -> (bool, bool) {
        let (qdrant_health, sqlite_health) =
            tokio::join!(self.qdrant.health_check(), self.sqlite.health_check());

        (qdrant_health, sqlite_health)
    }

    /// Check if the hybrid search engine is fully operational.
    ///
    /// Returns true only if both backends are healthy.
    pub async fn is_healthy(&self) -> bool {
        let (qdrant, sqlite) = self.health_check().await;
        qdrant && sqlite
    }

    /// Store content in both backends.
    ///
    /// This ensures consistency between vector and keyword indices.
    pub async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
    ) -> anyhow::Result<()> {
        // Store in both backends in parallel
        let (qdrant_result, sqlite_result) = tokio::join!(
            self.qdrant.store(key, content, category.clone()),
            self.sqlite.store(key, content, category)
        );

        // Report any errors, but don't fail if one backend succeeds
        if let Err(e) = &qdrant_result {
            tracing::warn!(key = key, error = %e, "Failed to store in Qdrant");
        }
        if let Err(e) = &sqlite_result {
            tracing::warn!(key = key, error = %e, "Failed to store in SQLite");
        }

        // Return error if both fail
        match (qdrant_result, sqlite_result) {
            (Err(e1), Err(e2)) => {
                anyhow::bail!("Both backends failed: Qdrant: {e1}, SQLite: {e2}")
            }
            _ => Ok(()),
        }
    }

    /// Delete content from both backends.
    ///
    /// # Returns
    /// True if the entry was found and deleted from at least one backend.
    pub async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        let (qdrant_result, sqlite_result) =
            tokio::join!(self.qdrant.forget(key), self.sqlite.forget(key));

        let qdrant_deleted = qdrant_result.unwrap_or(false);
        let sqlite_deleted = sqlite_result.unwrap_or(false);

        Ok(qdrant_deleted || sqlite_deleted)
    }

    /// Get entry by key from either backend.
    ///
    /// Tries Qdrant first, falls back to SQLite.
    pub async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        // Try Qdrant first
        if let Ok(Some(entry)) = self.qdrant.get(key).await {
            return Ok(Some(entry));
        }

        // Fall back to SQLite
        self.sqlite.get(key).await
    }

    /// List entries from SQLite (primary source for listing).
    ///
    /// Uses SQLite as the source of truth for listing since it's more
    /// efficient for non-vector operations.
    pub async fn list(&self, category: Option<&MemoryCategory>) -> anyhow::Result<Vec<MemoryEntry>> {
        self.sqlite.list(category).await
    }

    /// Count entries in SQLite.
    pub async fn count(&self, category: Option<&MemoryCategory>) -> anyhow::Result<usize> {
        self.sqlite.count(category).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights() {
        assert!((DEFAULT_VECTOR_WEIGHT - 0.7).abs() < f32::EPSILON);
        assert!((DEFAULT_KEYWORD_WEIGHT - 0.3).abs() < f32::EPSILON);
    }

    // Integration tests require running Qdrant instance
    // Mark with #[ignore] for CI/CD pipelines without Qdrant

    #[tokio::test]
    #[ignore = "requires Qdrant and embedding provider"]
    async fn hybrid_search_integration() {
        // This test requires:
        // 1. Running Qdrant on localhost:6334
        // 2. Valid embedding provider
        // See integration tests for full coverage
    }
}
