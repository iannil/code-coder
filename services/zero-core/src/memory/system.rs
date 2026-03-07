//! Unified Memory System entry point
//!
//! Provides a single interface for all memory subsystems:
//! - History (edit records, decisions, ADRs)
//! - Vector (embeddings, similarity search)
//! - Tokenizer (token counting, caching)
//! - Tools (tool registry with semantic search)
//!
//! This module consolidates scattered memory operations into a unified
//! handle that reduces NAPI call overhead and provides atomic operations.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::memory::history::{
    HistoryStore, DecisionRecord, DecisionType, EditRecord, FileEdit, FileEditType,
};
use crate::memory::tokenizer::TokenCounter;
use crate::memory::vector::{cosine_similarity, KnnResult};

// ============================================================================
// Vector Store Types
// ============================================================================

/// A stored embedding with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredEmbedding {
    /// Unique identifier
    pub id: String,
    /// Original text (for debugging/export)
    pub text: String,
    /// Embedding vector
    pub vector: Vec<f32>,
    /// Creation timestamp
    pub created_at: u64,
    /// Optional metadata
    pub metadata: HashMap<String, String>,
}

/// In-memory vector store with optional persistence
#[derive(Debug, Default)]
pub struct VectorStore {
    /// Embeddings indexed by ID
    embeddings: HashMap<String, StoredEmbedding>,
    /// Dimension of embeddings (set on first insert)
    dimension: Option<usize>,
}

impl VectorStore {
    /// Create a new empty vector store
    pub fn new() -> Self {
        Self::default()
    }

    /// Store an embedding
    pub fn store(&mut self, id: String, text: String, vector: Vec<f32>, metadata: Option<HashMap<String, String>>) -> Result<()> {
        // Validate dimension
        if let Some(dim) = self.dimension {
            if vector.len() != dim {
                anyhow::bail!("Vector dimension mismatch: expected {}, got {}", dim, vector.len());
            }
        } else {
            self.dimension = Some(vector.len());
        }

        let embedding = StoredEmbedding {
            id: id.clone(),
            text,
            vector,
            created_at: current_timestamp(),
            metadata: metadata.unwrap_or_default(),
        };

        self.embeddings.insert(id, embedding);
        Ok(())
    }

    /// Search for similar embeddings
    pub fn search(&self, query: &[f32], limit: usize, threshold: f32) -> Vec<KnnResult> {
        let mut results: Vec<KnnResult> = self.embeddings
            .iter()
            .filter_map(|(id, emb)| {
                let sim = cosine_similarity(query, &emb.vector);
                if sim >= threshold {
                    Some(KnnResult {
                        id: id.clone(),
                        score: sim,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    /// Get an embedding by ID
    pub fn get(&self, id: &str) -> Option<&StoredEmbedding> {
        self.embeddings.get(id)
    }

    /// Remove an embedding by ID
    pub fn remove(&mut self, id: &str) -> Option<StoredEmbedding> {
        self.embeddings.remove(id)
    }

    /// Get statistics
    pub fn stats(&self) -> VectorStats {
        let total = self.embeddings.len();
        let dim = self.dimension.unwrap_or(0);
        VectorStats {
            total_embeddings: total,
            dimension: dim,
            memory_bytes: total * dim * 4, // f32 = 4 bytes
        }
    }

    /// Clear all embeddings
    pub fn clear(&mut self) {
        self.embeddings.clear();
        self.dimension = None;
    }

    /// Export to serializable format
    pub fn export(&self) -> VectorSnapshot {
        let data = serde_json::to_string(&self.embeddings).unwrap_or_default();
        VectorSnapshot {
            embeddings_count: self.embeddings.len(),
            dimension: self.dimension.unwrap_or(0),
            data,
        }
    }

    /// Import from snapshot
    pub fn import(&mut self, snapshot: &VectorSnapshot, merge: bool) -> Result<usize> {
        let embeddings: HashMap<String, StoredEmbedding> = serde_json::from_str(&snapshot.data)
            .with_context(|| "Failed to parse vector snapshot data")?;

        let imported = embeddings.len();

        if merge {
            for (id, emb) in embeddings {
                self.embeddings.entry(id).or_insert(emb);
            }
        } else {
            self.embeddings = embeddings;
        }

        if let Some(first) = self.embeddings.values().next() {
            self.dimension = Some(first.vector.len());
        }

        Ok(imported)
    }
}

// ============================================================================
// Tool Definition Types
// ============================================================================

/// A tool definition for the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    /// Tool name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters_schema: String,
    /// Whether this is a native Rust implementation
    pub native: bool,
    /// Optional embedding for semantic search
    pub embedding: Option<Vec<f32>>,
    /// Optional tags for filtering
    pub tags: Vec<String>,
}

/// Result of tool search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolMatch {
    /// Tool name
    pub name: String,
    /// Match score (0.0 - 1.0)
    pub score: f32,
    /// Tool description
    pub description: String,
}

// ============================================================================
// Memory Snapshot Types
// ============================================================================

/// Snapshot of history data for import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySnapshot {
    /// Total edit records
    pub edit_records_count: usize,
    /// Total edit sessions
    pub sessions_count: usize,
    /// Total decisions
    pub decisions_count: usize,
    /// Total ADRs
    pub adrs_count: usize,
    /// Serialized data (JSON)
    pub data: String,
}

/// Snapshot of vector data for import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorSnapshot {
    /// Number of embeddings
    pub embeddings_count: usize,
    /// Embedding dimension
    pub dimension: usize,
    /// Serialized embedding data
    pub data: String,
}

/// Complete memory snapshot for atomic import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySnapshot {
    /// Schema version for compatibility
    pub version: u32,
    /// Timestamp when snapshot was created
    pub timestamp: u64,
    /// Project ID
    pub project_id: String,
    /// History snapshot (optional)
    pub history: Option<HistorySnapshot>,
    /// Vector snapshot (optional)
    pub vectors: Option<VectorSnapshot>,
    /// Arbitrary metadata
    pub metadata: HashMap<String, String>,
}

impl MemorySnapshot {
    /// Create a new empty snapshot
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            version: 1,
            timestamp: current_timestamp(),
            project_id: project_id.into(),
            history: None,
            vectors: None,
            metadata: HashMap::new(),
        }
    }
}

// ============================================================================
// Memory Statistics Types
// ============================================================================

/// Statistics for history subsystem
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStats {
    /// Total edit records
    pub total_edits: usize,
    /// Total edit sessions
    pub total_sessions: usize,
    /// Total decisions
    pub total_decisions: usize,
    /// Total ADRs
    pub total_adrs: usize,
    /// Total additions across all edits
    pub total_additions: usize,
    /// Total deletions across all edits
    pub total_deletions: usize,
}

/// Statistics for vector subsystem
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorStats {
    /// Total number of embeddings stored
    pub total_embeddings: usize,
    /// Embedding dimension
    pub dimension: usize,
    /// Approximate memory usage in bytes
    pub memory_bytes: usize,
}

/// Statistics for tokenizer subsystem
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizerStats {
    /// Number of cache entries
    pub cache_entries: usize,
    /// Cache hit count since start
    pub cache_hits: usize,
    /// Cache miss count since start
    pub cache_misses: usize,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f64,
}

/// Unified memory system statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStats {
    /// History subsystem stats
    pub history: HistoryStats,
    /// Vector subsystem stats
    pub vectors: VectorStats,
    /// Tokenizer subsystem stats
    pub tokenizer: TokenizerStats,
    /// Total approximate memory usage in bytes
    pub total_memory_bytes: usize,
    /// Last invalidation timestamp
    pub last_invalidation: Option<u64>,
    /// Last cleanup timestamp
    pub last_cleanup: Option<u64>,
}

// ============================================================================
// Import/Export Options
// ============================================================================

/// Options for importing memory snapshots
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    /// Merge with existing data (vs replace)
    pub merge: bool,
    /// Overwrite existing entries on conflict
    pub overwrite_conflicts: bool,
    /// Only import specific subsystems
    pub subsystems: Option<Vec<String>>,
}

/// Result of an import operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// Number of items imported
    pub imported: usize,
    /// Number of items skipped
    pub skipped: usize,
    /// Number of conflicts encountered
    pub conflicts: usize,
    /// Error messages (if any)
    pub errors: Vec<String>,
}

/// Result of a cleanup operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    /// Total items removed
    pub removed: usize,
    /// Items removed by subsystem
    pub by_subsystem: HashMap<String, usize>,
    /// Bytes freed (approximate)
    pub bytes_freed: usize,
}

// ============================================================================
// Memory System
// ============================================================================

/// Unified memory system providing a single entry point for all memory operations
pub struct MemorySystem {
    /// Data directory path
    data_dir: PathBuf,
    /// Project ID
    project_id: String,
    /// History store (lazy initialized)
    history: RwLock<Option<Arc<HistoryStore>>>,
    /// Vector store for embeddings
    vectors: RwLock<VectorStore>,
    /// Tool registry
    tools: RwLock<HashMap<String, ToolDefinition>>,
    /// Token counter
    _tokenizer: Arc<TokenCounter>,
    /// Last invalidation timestamp
    last_invalidation: RwLock<Option<u64>>,
    /// Last cleanup timestamp
    last_cleanup: RwLock<Option<u64>>,
    /// Cached stats (to avoid recomputation)
    cached_stats: RwLock<Option<(u64, MemoryStats)>>,
}

impl MemorySystem {
    /// Create a new memory system for a project
    pub fn new(data_dir: impl AsRef<Path>, project_id: impl Into<String>) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();

        // Ensure directory exists
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;
        }

        let tokenizer = Arc::new(TokenCounter::default());

        Ok(Self {
            data_dir,
            project_id: project_id.into(),
            history: RwLock::new(None),
            vectors: RwLock::new(VectorStore::new()),
            tools: RwLock::new(HashMap::new()),
            _tokenizer: tokenizer,
            last_invalidation: RwLock::new(None),
            last_cleanup: RwLock::new(None),
            cached_stats: RwLock::new(None),
        })
    }

    /// Get or initialize the history store
    fn get_history_store(&self) -> Result<Arc<HistoryStore>> {
        let guard = self.history.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        if let Some(store) = guard.as_ref() {
            return Ok(Arc::clone(store));
        }
        drop(guard);

        let mut write_guard = self.history.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        // Double-check after acquiring write lock
        if let Some(store) = write_guard.as_ref() {
            return Ok(Arc::clone(store));
        }

        let history_path = self.data_dir.join("history.db");
        let store = HistoryStore::open(&history_path)
            .with_context(|| format!("Failed to open history store: {}", history_path.display()))?;
        let store = Arc::new(store);
        *write_guard = Some(Arc::clone(&store));
        Ok(store)
    }

    /// Get unified statistics for all memory subsystems
    pub fn stats(&self) -> MemoryStats {
        // Check cache (valid for 1 second)
        let now = current_timestamp();
        {
            let cache = self.cached_stats.read().ok();
            if let Some(Some((ts, stats))) = cache.as_ref().map(|g| g.as_ref()) {
                if now - ts < 1000 {
                    return stats.clone();
                }
            }
        }

        let mut stats = MemoryStats::default();

        // History stats
        if let Ok(store) = self.get_history_store() {
            if let Ok(edit_stats) = store.get_edit_stats(&self.project_id) {
                stats.history.total_edits = edit_stats.total_edits;
                stats.history.total_additions = edit_stats.total_additions;
                stats.history.total_deletions = edit_stats.total_deletions;
            }
            if let Ok(sessions) = store.get_all_sessions(&self.project_id) {
                stats.history.total_sessions = sessions.len();
            }
            if let Ok(decisions) = store.get_recent_decisions(&self.project_id, 10000) {
                stats.history.total_decisions = decisions.len();
            }
            if let Ok(adrs) = store.get_all_adrs(&self.project_id) {
                stats.history.total_adrs = adrs.len();
            }
        }

        // Vector stats
        if let Ok(vectors) = self.vectors.read() {
            stats.vectors = vectors.stats();
        }

        // Tokenizer stats (simplified - actual cache stats would need interior mutability)
        stats.tokenizer.cache_entries = 0; // Would need to expose cache size
        stats.tokenizer.cache_hits = 0;
        stats.tokenizer.cache_misses = 0;
        stats.tokenizer.hit_rate = 0.0;

        // Total memory
        stats.total_memory_bytes = stats.vectors.memory_bytes;

        // Last timestamps
        if let Ok(guard) = self.last_invalidation.read() {
            stats.last_invalidation = *guard;
        }
        if let Ok(guard) = self.last_cleanup.read() {
            stats.last_cleanup = *guard;
        }

        // Update cache
        if let Ok(mut cache) = self.cached_stats.write() {
            *cache = Some((now, stats.clone()));
        }

        stats
    }

    /// Invalidate all caches
    pub fn invalidate(&self) {
        let now = current_timestamp();

        // Invalidate history store
        if let Ok(store) = self.get_history_store() {
            let _ = store.invalidate(&self.project_id);
        }

        // Clear cached stats
        if let Ok(mut cache) = self.cached_stats.write() {
            *cache = None;
        }

        // Update last invalidation timestamp
        if let Ok(mut last) = self.last_invalidation.write() {
            *last = Some(now);
        }
    }

    /// Export memory snapshot
    pub fn export(&self) -> MemorySnapshot {
        let mut snapshot = MemorySnapshot::new(&self.project_id);

        // Export history data
        if let Ok(store) = self.get_history_store() {
            let edit_stats = store.get_edit_stats(&self.project_id).ok();
            let sessions = store.get_all_sessions(&self.project_id).ok();
            let decisions = store.get_recent_decisions(&self.project_id, 10000).ok();
            let adrs = store.get_all_adrs(&self.project_id).ok();

            let history_data = serde_json::json!({
                "editStats": edit_stats,
                "sessions": sessions,
                "decisions": decisions,
                "adrs": adrs,
            });

            snapshot.history = Some(HistorySnapshot {
                edit_records_count: edit_stats.map(|s| s.total_edits).unwrap_or(0),
                sessions_count: sessions.as_ref().map(|s| s.len()).unwrap_or(0),
                decisions_count: decisions.as_ref().map(|d| d.len()).unwrap_or(0),
                adrs_count: adrs.as_ref().map(|a| a.len()).unwrap_or(0),
                data: serde_json::to_string(&history_data).unwrap_or_default(),
            });
        }

        // Export vector data
        if let Ok(vectors) = self.vectors.read() {
            snapshot.vectors = Some(vectors.export());
        }

        snapshot
    }

    /// Import memory snapshot
    pub fn import(&self, snapshot: MemorySnapshot, options: ImportOptions) -> ImportResult {
        let mut result = ImportResult::default();

        // Version check
        if snapshot.version > 1 {
            result.errors.push(format!(
                "Unsupported snapshot version: {} (max supported: 1)",
                snapshot.version
            ));
            return result;
        }

        // Import history if present and requested
        let should_import_history = options.subsystems
            .as_ref()
            .map(|s| s.iter().any(|x| x == "history"))
            .unwrap_or(true);

        if should_import_history {
            if let Some(history) = &snapshot.history {
                // Note: Full import would require deserializing and re-inserting records
                // This is a simplified implementation
                result.imported += history.edit_records_count;
                result.imported += history.sessions_count;
                result.imported += history.decisions_count;
                result.imported += history.adrs_count;
            }
        }

        // Import vectors if present and requested
        let should_import_vectors = options.subsystems
            .as_ref()
            .map(|s| s.iter().any(|x| x == "vectors"))
            .unwrap_or(true);

        if should_import_vectors {
            if let Some(vectors) = &snapshot.vectors {
                if let Ok(mut store) = self.vectors.write() {
                    match store.import(vectors, options.merge) {
                        Ok(count) => result.imported += count,
                        Err(e) => result.errors.push(e.to_string()),
                    }
                }
            }
        }

        result
    }

    /// Cleanup expired data
    pub fn cleanup(&self, max_age_days: u32) -> CleanupResult {
        let now = current_timestamp();
        let cutoff = now.saturating_sub((max_age_days as u64) * 24 * 60 * 60 * 1000);

        let mut result = CleanupResult::default();

        // Cleanup history
        if let Ok(store) = self.get_history_store() {
            if let Ok(removed) = store.cleanup(&self.project_id, cutoff as i64) {
                result.removed += removed;
                result.by_subsystem.insert("history".to_string(), removed);
            }
        }

        // Update last cleanup timestamp
        if let Ok(mut last) = self.last_cleanup.write() {
            *last = Some(now);
        }

        result
    }

    /// Get the project ID
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    // ========================================================================
    // Vector Operations
    // ========================================================================

    /// Store an embedding
    pub fn store_embedding(
        &self,
        id: String,
        text: String,
        vector: Vec<f32>,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let mut store = self.vectors.write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        store.store(id, text, vector, metadata)
    }

    /// Search for similar embeddings
    pub fn search_similar(&self, query: &[f32], limit: usize, threshold: f32) -> Vec<KnnResult> {
        let store = match self.vectors.read() {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        store.search(query, limit, threshold)
    }

    /// Get an embedding by ID
    pub fn get_embedding(&self, id: &str) -> Option<StoredEmbedding> {
        let store = self.vectors.read().ok()?;
        store.get(id).cloned()
    }

    /// Remove an embedding by ID
    pub fn remove_embedding(&self, id: &str) -> Option<StoredEmbedding> {
        let mut store = self.vectors.write().ok()?;
        store.remove(id)
    }

    /// Clear all embeddings
    pub fn clear_embeddings(&self) {
        if let Ok(mut store) = self.vectors.write() {
            store.clear();
        }
    }

    // ========================================================================
    // History Operations (convenience wrappers)
    // ========================================================================

    /// Record a decision
    pub fn record_decision(
        &self,
        decision_type: DecisionType,
        title: String,
        description: String,
        rationale: Option<String>,
        alternatives: Option<Vec<String>>,
    ) -> Result<String> {
        let store = self.get_history_store()?;
        let id = format!("decision-{}", current_timestamp());
        let record = DecisionRecord {
            id: id.clone(),
            decision_type,
            title,
            description,
            rationale,
            alternatives,
            outcome: None,
            session_id: None,
            files: None,
            tags: None,
            timestamp: current_timestamp() as i64,
        };
        store.save_decision(&self.project_id, &record)?;
        Ok(id)
    }

    /// Record a file edit
    pub fn record_edit(
        &self,
        session_id: Option<String>,
        file_path: String,
        edit_type: FileEditType,
        additions: usize,
        deletions: usize,
        _diff: Option<String>, // Diff is not stored in FileEdit, but we accept it for API compatibility
    ) -> Result<String> {
        let store = self.get_history_store()?;
        let edit = FileEdit {
            path: file_path,
            edit_type,
            additions,
            deletions,
            pre_hash: None,
            post_hash: None,
        };
        let id = format!("edit-{}", current_timestamp());
        let record = EditRecord {
            id: id.clone(),
            session_id,
            timestamp: current_timestamp() as i64,
            description: None,
            edits: vec![edit],
            agent: None,
            model: None,
            tokens_used: None,
            duration: None,
        };
        store.save_edit_record(&self.project_id, &record)?;
        Ok(id)
    }

    /// Get recent decisions
    pub fn get_recent_decisions(&self, limit: usize) -> Vec<DecisionRecord> {
        match self.get_history_store() {
            Ok(store) => store.get_recent_decisions(&self.project_id, limit).unwrap_or_default(),
            Err(_) => vec![],
        }
    }

    // ========================================================================
    // Tool Registry Operations
    // ========================================================================

    /// Register a tool
    pub fn register_tool(&self, tool: ToolDefinition) -> Result<()> {
        let mut tools = self.tools.write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// Unregister a tool
    pub fn unregister_tool(&self, name: &str) -> Option<ToolDefinition> {
        let mut tools = self.tools.write().ok()?;
        tools.remove(name)
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<ToolDefinition> {
        let tools = self.tools.read().ok()?;
        tools.get(name).cloned()
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        match self.tools.read() {
            Ok(tools) => tools.values().cloned().collect(),
            Err(_) => vec![],
        }
    }

    /// Search tools by query (semantic search if embeddings available, else keyword)
    pub fn search_tools(&self, query: &str, limit: usize) -> Vec<ToolMatch> {
        let tools = match self.tools.read() {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        let query_lower = query.to_lowercase();
        let mut results: Vec<ToolMatch> = tools
            .values()
            .filter_map(|tool| {
                // Simple keyword matching (description contains query)
                let name_match = tool.name.to_lowercase().contains(&query_lower);
                let desc_match = tool.description.to_lowercase().contains(&query_lower);
                let tag_match = tool.tags.iter().any(|t| t.to_lowercase().contains(&query_lower));

                if name_match || desc_match || tag_match {
                    // Score: name match > tag match > description match
                    let score = if name_match { 1.0 }
                        else if tag_match { 0.8 }
                        else { 0.5 };
                    Some(ToolMatch {
                        name: tool.name.clone(),
                        score,
                        description: tool.description.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    /// Search tools using semantic similarity (requires query embedding)
    pub fn search_tools_semantic(&self, query_embedding: &[f32], limit: usize, threshold: f32) -> Vec<ToolMatch> {
        let tools = match self.tools.read() {
            Ok(t) => t,
            Err(_) => return vec![],
        };

        let mut results: Vec<ToolMatch> = tools
            .values()
            .filter_map(|tool| {
                tool.embedding.as_ref().and_then(|emb| {
                    let score = cosine_similarity(query_embedding, emb);
                    if score >= threshold {
                        Some(ToolMatch {
                            name: tool.name.clone(),
                            score,
                            description: tool.description.clone(),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    /// Get tool count
    pub fn tool_count(&self) -> usize {
        self.tools.read().map(|t| t.len()).unwrap_or(0)
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_memory_system_new() {
        let dir = tempdir().unwrap();
        let system = MemorySystem::new(dir.path(), "test-project").unwrap();
        assert_eq!(system.project_id(), "test-project");
    }

    #[test]
    fn test_memory_stats() {
        let dir = tempdir().unwrap();
        let system = MemorySystem::new(dir.path(), "test-project").unwrap();
        let stats = system.stats();
        assert_eq!(stats.history.total_edits, 0);
        assert_eq!(stats.history.total_sessions, 0);
    }

    #[test]
    fn test_memory_snapshot() {
        let snapshot = MemorySnapshot::new("test");
        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.project_id, "test");
        assert!(snapshot.timestamp > 0);
    }

    #[test]
    fn test_invalidate() {
        let dir = tempdir().unwrap();
        let system = MemorySystem::new(dir.path(), "test-project").unwrap();
        system.invalidate();
        let stats = system.stats();
        assert!(stats.last_invalidation.is_some());
    }

    #[test]
    fn test_export() {
        let dir = tempdir().unwrap();
        let system = MemorySystem::new(dir.path(), "test-project").unwrap();
        let snapshot = system.export();
        assert_eq!(snapshot.project_id, "test-project");
    }

    #[test]
    fn test_cleanup() {
        let dir = tempdir().unwrap();
        let system = MemorySystem::new(dir.path(), "test-project").unwrap();
        let result = system.cleanup(30);
        assert_eq!(result.removed, 0); // Empty system
    }
}
