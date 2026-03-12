//! NAPI bindings for memory module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - Text chunking
//! - Vector operations
//! - Embedding utilities
//! - Unified memory system

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;

use crate::memory::{
    chunker::{chunk_markdown, ChunkerConfig as RustChunkerConfig, Chunk as RustChunk},
    vector::{
        bytes_to_vec as rust_bytes_to_vec, cosine_similarity as rust_cosine_similarity,
        euclidean_distance, hybrid_merge as rust_hybrid_merge, normalize,
        vec_to_bytes as rust_vec_to_bytes, ScoredResult as RustScoredResult, KnnResult,
    },
    history::{DecisionType as RustDecisionType, FileEditType as RustFileEditType},
    system::{
        MemorySystem as RustMemorySystem,
        MemorySnapshot as RustMemorySnapshot,
        MemoryStats as RustMemoryStats,
        HistoryStats as RustHistoryStats,
        VectorStats as RustVectorStats,
        TokenizerStats as RustTokenizerStats,
        HistorySnapshot as RustHistorySnapshot,
        VectorSnapshot as RustVectorSnapshot,
        ImportOptions as RustImportOptions,
        ImportResult as RustImportResult,
        CleanupResult as RustCleanupResult,
        StoredEmbedding as RustStoredEmbedding,
        ToolDefinition as RustToolDefinition,
        ToolMatch as RustToolMatch,
    },
};

// ============================================================================
// Chunker Types (NAPI)
// ============================================================================

/// Chunker configuration for NAPI
#[napi(object)]
pub struct NapiChunkerConfig {
    /// Maximum tokens per chunk (~4 chars per token)
    pub max_tokens: u32,
    /// Overlap tokens between chunks
    pub overlap_tokens: u32,
    /// Preserve heading context in split chunks
    pub preserve_headings: bool,
}

impl Default for NapiChunkerConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 0,
            preserve_headings: true,
        }
    }
}

impl From<NapiChunkerConfig> for RustChunkerConfig {
    fn from(config: NapiChunkerConfig) -> Self {
        Self {
            max_tokens: config.max_tokens as usize,
            overlap_tokens: config.overlap_tokens as usize,
            preserve_headings: config.preserve_headings,
        }
    }
}

/// A chunk of text for NAPI
#[napi(object)]
pub struct NapiChunk {
    /// Chunk index (0-based)
    pub index: u32,
    /// Chunk content
    pub content: String,
    /// Heading context (if any)
    pub heading: Option<String>,
    /// Start offset in original text
    pub start_offset: u32,
    /// End offset in original text
    pub end_offset: u32,
}

impl From<RustChunk> for NapiChunk {
    fn from(chunk: RustChunk) -> Self {
        Self {
            index: chunk.index as u32,
            content: chunk.content,
            heading: chunk.heading,
            start_offset: chunk.start_offset as u32,
            end_offset: chunk.end_offset as u32,
        }
    }
}

// ============================================================================
// Vector Types (NAPI)
// ============================================================================

/// Scored result for NAPI
#[napi(object)]
pub struct NapiScoredResult {
    /// Result identifier
    pub id: String,
    /// Vector similarity score
    pub vector_score: Option<f64>,
    /// Keyword/BM25 score
    pub keyword_score: Option<f64>,
    /// Final combined score
    pub final_score: f64,
}

impl From<RustScoredResult> for NapiScoredResult {
    fn from(result: RustScoredResult) -> Self {
        Self {
            id: result.id,
            vector_score: result.vector_score.map(f64::from),
            keyword_score: result.keyword_score.map(f64::from),
            final_score: f64::from(result.final_score),
        }
    }
}

/// Vector result pair for hybrid merge
#[napi(object)]
pub struct NapiVectorResult {
    pub id: String,
    pub score: f64,
}

// ============================================================================
// Chunker Functions (NAPI)
// ============================================================================

/// Chunk markdown text into semantic chunks
#[napi]
pub fn chunk_text(text: String, max_tokens: Option<u32>) -> Vec<NapiChunk> {
    let max_tokens = max_tokens.unwrap_or(512) as usize;
    chunk_markdown(&text, max_tokens)
        .into_iter()
        .map(|c| c.into())
        .collect()
}

/// Chunk markdown text with custom configuration
#[napi]
pub fn chunk_text_with_config(text: String, config: NapiChunkerConfig) -> Vec<NapiChunk> {
    let max_tokens = config.max_tokens as usize;
    chunk_markdown(&text, max_tokens)
        .into_iter()
        .map(|c| c.into())
        .collect()
}

/// Estimate token count for text (~4 chars per token)
#[napi]
pub fn estimate_tokens(text: String) -> u32 {
    crate::memory::tokenizer::estimate_tokens(&text) as u32
}

/// Estimate token count for text - alias for compatibility
#[napi]
pub fn estimate_chunk_tokens_native(text: String) -> u32 {
    crate::memory::tokenizer::estimate_tokens(&text) as u32
}

/// Batch token count result
#[napi(object)]
pub struct NapiBatchCountResult {
    /// Token counts for each input
    pub counts: Vec<u32>,
    /// Total tokens across all inputs
    pub total: u32,
    /// Number of cache hits
    pub cache_hits: u32,
}

/// Estimate tokens for multiple texts efficiently
#[napi]
pub fn estimate_tokens_batch(texts: Vec<String>) -> NapiBatchCountResult {
    let refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let result = crate::memory::tokenizer::estimate_tokens_batch(&refs);
    NapiBatchCountResult {
        counts: result.counts.iter().map(|&c| c as u32).collect(),
        total: result.total as u32,
        cache_hits: result.cache_hits as u32,
    }
}

/// Truncate text to fit within a token budget
#[napi]
pub fn truncate_to_tokens(text: String, max_tokens: u32) -> String {
    crate::memory::tokenizer::truncate_to_tokens(&text, max_tokens as usize)
}

/// Check if text fits within a token budget
#[napi]
pub fn fits_token_budget(text: String, budget: u32) -> bool {
    crate::memory::tokenizer::fits_token_budget(&text, budget as usize)
}

// ============================================================================
// Vector Functions (NAPI)
// ============================================================================

/// Calculate cosine similarity between two vectors
#[napi]
pub fn cosine_similarity(a: Vec<f64>, b: Vec<f64>) -> f64 {
    let a: Vec<f32> = a.into_iter().map(|x| x as f32).collect();
    let b: Vec<f32> = b.into_iter().map(|x| x as f32).collect();
    f64::from(rust_cosine_similarity(&a, &b))
}

/// Calculate Euclidean distance between two vectors
#[napi]
pub fn vector_distance(a: Vec<f64>, b: Vec<f64>) -> f64 {
    let a: Vec<f32> = a.into_iter().map(|x| x as f32).collect();
    let b: Vec<f32> = b.into_iter().map(|x| x as f32).collect();
    f64::from(euclidean_distance(&a, &b))
}

/// Normalize a vector to unit length (L2 normalization)
#[napi]
pub fn normalize_vector(v: Vec<f64>) -> Vec<f64> {
    let v: Vec<f32> = v.into_iter().map(|x| x as f32).collect();
    normalize(&v).into_iter().map(f64::from).collect()
}

/// Serialize f32 vector to bytes (little-endian)
#[napi]
pub fn vector_to_bytes(v: Vec<f64>) -> Buffer {
    let v: Vec<f32> = v.into_iter().map(|x| x as f32).collect();
    Buffer::from(rust_vec_to_bytes(&v))
}

/// Deserialize bytes to f32 vector (little-endian)
#[napi]
pub fn bytes_to_vector(bytes: Buffer) -> Vec<f64> {
    rust_bytes_to_vec(&bytes)
        .into_iter()
        .map(f64::from)
        .collect()
}

/// Hybrid merge: combine vector and keyword results with weighted fusion
#[napi]
pub fn hybrid_merge_results(
    vector_results: Vec<NapiVectorResult>,
    keyword_results: Vec<NapiVectorResult>,
    vector_weight: f64,
    keyword_weight: f64,
    limit: u32,
) -> Vec<NapiScoredResult> {
    let vec_results: Vec<(String, f32)> = vector_results
        .into_iter()
        .map(|r| (r.id, r.score as f32))
        .collect();
    let kw_results: Vec<(String, f32)> = keyword_results
        .into_iter()
        .map(|r| (r.id, r.score as f32))
        .collect();

    rust_hybrid_merge(
        &vec_results,
        &kw_results,
        vector_weight as f32,
        keyword_weight as f32,
        limit as usize,
    )
    .into_iter()
    .map(|r| r.into())
    .collect()
}

// ============================================================================
// Memory System Types (NAPI)
// ============================================================================

/// History statistics for NAPI
#[napi(object)]
pub struct NapiHistoryStats {
    /// Total edit records
    pub total_edits: u32,
    /// Total edit sessions
    pub total_sessions: u32,
    /// Total decisions
    pub total_decisions: u32,
    /// Total ADRs
    pub total_adrs: u32,
    /// Total additions across all edits
    pub total_additions: u32,
    /// Total deletions across all edits
    pub total_deletions: u32,
}

impl From<RustHistoryStats> for NapiHistoryStats {
    fn from(s: RustHistoryStats) -> Self {
        Self {
            total_edits: s.total_edits as u32,
            total_sessions: s.total_sessions as u32,
            total_decisions: s.total_decisions as u32,
            total_adrs: s.total_adrs as u32,
            total_additions: s.total_additions as u32,
            total_deletions: s.total_deletions as u32,
        }
    }
}

/// Vector statistics for NAPI
#[napi(object)]
pub struct NapiVectorStats {
    /// Total number of embeddings stored
    pub total_embeddings: u32,
    /// Embedding dimension
    pub dimension: u32,
    /// Approximate memory usage in bytes
    pub memory_bytes: u32,
}

impl From<RustVectorStats> for NapiVectorStats {
    fn from(s: RustVectorStats) -> Self {
        Self {
            total_embeddings: s.total_embeddings as u32,
            dimension: s.dimension as u32,
            memory_bytes: s.memory_bytes as u32,
        }
    }
}

/// Tokenizer statistics for NAPI
#[napi(object)]
pub struct NapiTokenizerStats {
    /// Number of cache entries
    pub cache_entries: u32,
    /// Cache hit count
    pub cache_hits: u32,
    /// Cache miss count
    pub cache_misses: u32,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f64,
}

impl From<RustTokenizerStats> for NapiTokenizerStats {
    fn from(s: RustTokenizerStats) -> Self {
        Self {
            cache_entries: s.cache_entries as u32,
            cache_hits: s.cache_hits as u32,
            cache_misses: s.cache_misses as u32,
            hit_rate: s.hit_rate,
        }
    }
}

/// Unified memory statistics for NAPI
#[napi(object)]
pub struct NapiMemoryStats {
    /// History subsystem stats
    pub history: NapiHistoryStats,
    /// Vector subsystem stats
    pub vectors: NapiVectorStats,
    /// Tokenizer subsystem stats
    pub tokenizer: NapiTokenizerStats,
    /// Total approximate memory usage in bytes
    pub total_memory_bytes: u32,
    /// Last invalidation timestamp (ms since epoch)
    pub last_invalidation: Option<i64>,
    /// Last cleanup timestamp (ms since epoch)
    pub last_cleanup: Option<i64>,
}

impl From<RustMemoryStats> for NapiMemoryStats {
    fn from(s: RustMemoryStats) -> Self {
        Self {
            history: s.history.into(),
            vectors: s.vectors.into(),
            tokenizer: s.tokenizer.into(),
            total_memory_bytes: s.total_memory_bytes as u32,
            last_invalidation: s.last_invalidation.map(|t| t as i64),
            last_cleanup: s.last_cleanup.map(|t| t as i64),
        }
    }
}

/// History snapshot for NAPI
#[napi(object)]
pub struct NapiHistorySnapshot {
    /// Total edit records
    pub edit_records_count: u32,
    /// Total edit sessions
    pub sessions_count: u32,
    /// Total decisions
    pub decisions_count: u32,
    /// Total ADRs
    pub adrs_count: u32,
    /// Serialized data (JSON)
    pub data: String,
}

impl From<RustHistorySnapshot> for NapiHistorySnapshot {
    fn from(s: RustHistorySnapshot) -> Self {
        Self {
            edit_records_count: s.edit_records_count as u32,
            sessions_count: s.sessions_count as u32,
            decisions_count: s.decisions_count as u32,
            adrs_count: s.adrs_count as u32,
            data: s.data,
        }
    }
}

impl From<NapiHistorySnapshot> for RustHistorySnapshot {
    fn from(s: NapiHistorySnapshot) -> Self {
        Self {
            edit_records_count: s.edit_records_count as usize,
            sessions_count: s.sessions_count as usize,
            decisions_count: s.decisions_count as usize,
            adrs_count: s.adrs_count as usize,
            data: s.data,
        }
    }
}

/// Vector snapshot for NAPI
#[napi(object)]
pub struct NapiVectorSnapshotData {
    /// Number of embeddings
    pub embeddings_count: u32,
    /// Embedding dimension
    pub dimension: u32,
    /// Serialized embedding data
    pub data: String,
}

impl From<RustVectorSnapshot> for NapiVectorSnapshotData {
    fn from(s: RustVectorSnapshot) -> Self {
        Self {
            embeddings_count: s.embeddings_count as u32,
            dimension: s.dimension as u32,
            data: s.data,
        }
    }
}

impl From<NapiVectorSnapshotData> for RustVectorSnapshot {
    fn from(s: NapiVectorSnapshotData) -> Self {
        Self {
            embeddings_count: s.embeddings_count as usize,
            dimension: s.dimension as usize,
            data: s.data,
        }
    }
}

/// Complete memory snapshot for NAPI
#[napi(object)]
pub struct NapiMemorySnapshot {
    /// Schema version
    pub version: u32,
    /// Timestamp (ms since epoch)
    pub timestamp: i64,
    /// Project ID
    pub project_id: String,
    /// History snapshot (optional)
    pub history: Option<NapiHistorySnapshot>,
    /// Vector snapshot (optional)
    pub vectors: Option<NapiVectorSnapshotData>,
    /// Metadata as JSON string
    pub metadata: String,
}

impl From<RustMemorySnapshot> for NapiMemorySnapshot {
    fn from(s: RustMemorySnapshot) -> Self {
        Self {
            version: s.version,
            timestamp: s.timestamp as i64,
            project_id: s.project_id,
            history: s.history.map(Into::into),
            vectors: s.vectors.map(Into::into),
            metadata: serde_json::to_string(&s.metadata).unwrap_or_default(),
        }
    }
}

impl From<NapiMemorySnapshot> for RustMemorySnapshot {
    fn from(s: NapiMemorySnapshot) -> Self {
        Self {
            version: s.version,
            timestamp: s.timestamp as u64,
            project_id: s.project_id,
            history: s.history.map(Into::into),
            vectors: s.vectors.map(Into::into),
            metadata: serde_json::from_str(&s.metadata).unwrap_or_default(),
        }
    }
}

/// Import options for NAPI
#[napi(object)]
pub struct NapiImportOptions {
    /// Merge with existing data
    pub merge: bool,
    /// Overwrite existing entries on conflict
    pub overwrite_conflicts: bool,
    /// Only import specific subsystems (JSON array)
    pub subsystems: Option<String>,
}

impl From<NapiImportOptions> for RustImportOptions {
    fn from(o: NapiImportOptions) -> Self {
        Self {
            merge: o.merge,
            overwrite_conflicts: o.overwrite_conflicts,
            subsystems: o.subsystems.and_then(|s| serde_json::from_str(&s).ok()),
        }
    }
}

/// Import result for NAPI
#[napi(object)]
pub struct NapiImportResult {
    /// Number of items imported
    pub imported: u32,
    /// Number of items skipped
    pub skipped: u32,
    /// Number of conflicts encountered
    pub conflicts: u32,
    /// Error messages (JSON array)
    pub errors: String,
}

impl From<RustImportResult> for NapiImportResult {
    fn from(r: RustImportResult) -> Self {
        Self {
            imported: r.imported as u32,
            skipped: r.skipped as u32,
            conflicts: r.conflicts as u32,
            errors: serde_json::to_string(&r.errors).unwrap_or_default(),
        }
    }
}

/// Cleanup result for NAPI
#[napi(object)]
pub struct NapiCleanupResult {
    /// Total items removed
    pub removed: u32,
    /// Items removed by subsystem (JSON object)
    pub by_subsystem: String,
    /// Bytes freed (approximate)
    pub bytes_freed: u32,
}

impl From<RustCleanupResult> for NapiCleanupResult {
    fn from(r: RustCleanupResult) -> Self {
        Self {
            removed: r.removed as u32,
            by_subsystem: serde_json::to_string(&r.by_subsystem).unwrap_or_default(),
            bytes_freed: r.bytes_freed as u32,
        }
    }
}

// ============================================================================
// Stored Embedding Types (NAPI)
// ============================================================================

/// A stored embedding for NAPI
#[napi(object)]
pub struct NapiStoredEmbedding {
    /// Unique identifier
    pub id: String,
    /// Original text
    pub text: String,
    /// Embedding vector
    pub vector: Vec<f64>,
    /// Creation timestamp (ms since epoch)
    pub created_at: i64,
    /// Metadata as JSON string
    pub metadata: String,
}

impl From<RustStoredEmbedding> for NapiStoredEmbedding {
    fn from(e: RustStoredEmbedding) -> Self {
        Self {
            id: e.id,
            text: e.text,
            vector: e.vector.into_iter().map(f64::from).collect(),
            created_at: e.created_at as i64,
            metadata: serde_json::to_string(&e.metadata).unwrap_or_default(),
        }
    }
}

/// KNN search result for NAPI
#[napi(object)]
pub struct NapiKnnResult {
    /// Result identifier
    pub id: String,
    /// Similarity score (0.0-1.0)
    pub score: f64,
}

impl From<KnnResult> for NapiKnnResult {
    fn from(r: KnnResult) -> Self {
        Self {
            id: r.id,
            score: f64::from(r.score),
        }
    }
}

// ============================================================================
// Tool Registry Types (NAPI)
// ============================================================================

/// Tool definition for NAPI
#[napi(object)]
pub struct NapiToolDefinition {
    /// Tool name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters_schema: String,
    /// Whether this is a native Rust implementation
    pub native: bool,
    /// Optional embedding for semantic search (as JSON array)
    pub embedding: Option<String>,
    /// Tags for filtering (as JSON array)
    pub tags: String,
}

impl From<NapiToolDefinition> for RustToolDefinition {
    fn from(t: NapiToolDefinition) -> Self {
        Self {
            name: t.name,
            description: t.description,
            parameters_schema: t.parameters_schema,
            native: t.native,
            embedding: t.embedding.and_then(|s| serde_json::from_str::<Vec<f32>>(&s).ok()),
            tags: serde_json::from_str(&t.tags).unwrap_or_default(),
        }
    }
}

impl From<RustToolDefinition> for NapiToolDefinition {
    fn from(t: RustToolDefinition) -> Self {
        Self {
            name: t.name,
            description: t.description,
            parameters_schema: t.parameters_schema,
            native: t.native,
            embedding: t.embedding.map(|e| serde_json::to_string(&e).unwrap_or_default()),
            tags: serde_json::to_string(&t.tags).unwrap_or_default(),
        }
    }
}

/// Tool match result for NAPI
#[napi(object)]
pub struct NapiToolMatch {
    /// Tool name
    pub name: String,
    /// Match score (0.0 - 1.0)
    pub score: f64,
    /// Tool description
    pub description: String,
}

impl From<RustToolMatch> for NapiToolMatch {
    fn from(m: RustToolMatch) -> Self {
        Self {
            name: m.name,
            score: f64::from(m.score),
            description: m.description,
        }
    }
}

// ============================================================================
// Decision/Edit Type Enums (NAPI)
// ============================================================================

/// Decision type for NAPI
#[napi(string_enum)]
pub enum NapiDecisionType {
    Architecture,
    Implementation,
    Refactor,
    Bugfix,
    Feature,
    Other,
}

impl From<NapiDecisionType> for RustDecisionType {
    fn from(t: NapiDecisionType) -> Self {
        match t {
            NapiDecisionType::Architecture => RustDecisionType::Architecture,
            NapiDecisionType::Implementation => RustDecisionType::Implementation,
            NapiDecisionType::Refactor => RustDecisionType::Refactor,
            NapiDecisionType::Bugfix => RustDecisionType::Bugfix,
            NapiDecisionType::Feature => RustDecisionType::Feature,
            NapiDecisionType::Other => RustDecisionType::Other,
        }
    }
}

/// File edit type for NAPI
#[napi(string_enum)]
pub enum NapiFileEditType {
    Create,
    Update,
    Delete,
    Move,
}

impl From<NapiFileEditType> for RustFileEditType {
    fn from(t: NapiFileEditType) -> Self {
        match t {
            NapiFileEditType::Create => RustFileEditType::Create,
            NapiFileEditType::Update => RustFileEditType::Update,
            NapiFileEditType::Delete => RustFileEditType::Delete,
            NapiFileEditType::Move => RustFileEditType::Move,
        }
    }
}

// ============================================================================
// Memory System Handle (NAPI)
// ============================================================================

/// Handle to a unified memory system
#[napi]
pub struct MemorySystemHandle {
    inner: Arc<RustMemorySystem>,
}

#[napi]
impl MemorySystemHandle {
    /// Create a new memory system for a project
    #[napi(constructor)]
    pub fn new(data_dir: String, project_id: String) -> Result<Self> {
        let system = RustMemorySystem::new(&data_dir, &project_id)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(system),
        })
    }

    /// Get unified statistics for all memory subsystems
    #[napi]
    pub fn stats(&self) -> NapiMemoryStats {
        self.inner.stats().into()
    }

    /// Invalidate all caches
    #[napi]
    pub fn invalidate(&self) {
        self.inner.invalidate()
    }

    /// Export memory snapshot
    #[napi]
    pub fn export(&self) -> NapiMemorySnapshot {
        self.inner.export().into()
    }

    /// Import memory snapshot
    #[napi]
    pub fn import_snapshot(
        &self,
        snapshot: NapiMemorySnapshot,
        options: NapiImportOptions,
    ) -> NapiImportResult {
        self.inner.import(snapshot.into(), options.into()).into()
    }

    /// Cleanup expired data
    #[napi]
    pub fn cleanup(&self, max_age_days: u32) -> NapiCleanupResult {
        self.inner.cleanup(max_age_days).into()
    }

    /// Get the project ID
    #[napi]
    pub fn project_id(&self) -> String {
        self.inner.project_id().to_string()
    }

    /// Get the data directory path
    #[napi]
    pub fn data_dir(&self) -> String {
        self.inner.data_dir().to_string_lossy().to_string()
    }

    // ========================================================================
    // Vector Operations
    // ========================================================================

    /// Store an embedding
    #[napi]
    pub fn store_embedding(
        &self,
        id: String,
        text: String,
        vector: Vec<f64>,
        metadata: Option<String>,
    ) -> Result<()> {
        let vec_f32: Vec<f32> = vector.into_iter().map(|x| x as f32).collect();
        let meta: Option<std::collections::HashMap<String, String>> = metadata
            .and_then(|s| serde_json::from_str(&s).ok());
        self.inner.store_embedding(id, text, vec_f32, meta)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Search for similar embeddings
    #[napi]
    pub fn search_similar(
        &self,
        query: Vec<f64>,
        limit: u32,
        threshold: f64,
    ) -> Vec<NapiKnnResult> {
        let query_f32: Vec<f32> = query.into_iter().map(|x| x as f32).collect();
        self.inner.search_similar(&query_f32, limit as usize, threshold as f32)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    /// Get an embedding by ID
    #[napi]
    pub fn get_embedding(&self, id: String) -> Option<NapiStoredEmbedding> {
        self.inner.get_embedding(&id).map(Into::into)
    }

    /// Remove an embedding by ID
    #[napi]
    pub fn remove_embedding(&self, id: String) -> Option<NapiStoredEmbedding> {
        self.inner.remove_embedding(&id).map(Into::into)
    }

    /// Clear all embeddings
    #[napi]
    pub fn clear_embeddings(&self) {
        self.inner.clear_embeddings()
    }

    // ========================================================================
    // History Operations
    // ========================================================================

    /// Record a decision
    #[napi]
    pub fn record_decision(
        &self,
        decision_type: NapiDecisionType,
        title: String,
        description: String,
        rationale: Option<String>,
        alternatives: Option<String>,
    ) -> Result<String> {
        let alts: Option<Vec<String>> = alternatives
            .and_then(|s| serde_json::from_str(&s).ok());
        self.inner.record_decision(
            decision_type.into(),
            title,
            description,
            rationale,
            alts,
        ).map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Record a file edit
    #[napi]
    pub fn record_edit(
        &self,
        session_id: Option<String>,
        file_path: String,
        edit_type: NapiFileEditType,
        additions: u32,
        deletions: u32,
        diff: Option<String>,
    ) -> Result<String> {
        self.inner.record_edit(
            session_id,
            file_path,
            edit_type.into(),
            additions as usize,
            deletions as usize,
            diff,
        ).map_err(|e| Error::from_reason(e.to_string()))
    }

    // ========================================================================
    // Tool Registry Operations
    // ========================================================================

    /// Register a tool
    #[napi]
    pub fn register_tool(&self, tool: NapiToolDefinition) -> Result<()> {
        self.inner.register_tool(tool.into())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Unregister a tool
    #[napi]
    pub fn unregister_tool(&self, name: String) -> Option<NapiToolDefinition> {
        self.inner.unregister_tool(&name).map(Into::into)
    }

    /// Get a tool by name
    #[napi]
    pub fn get_tool(&self, name: String) -> Option<NapiToolDefinition> {
        self.inner.get_tool(&name).map(Into::into)
    }

    /// List all registered tools
    #[napi]
    pub fn list_tools(&self) -> Vec<NapiToolDefinition> {
        self.inner.list_tools().into_iter().map(Into::into).collect()
    }

    /// Search tools by query (keyword search)
    #[napi]
    pub fn search_tools(&self, query: String, limit: u32) -> Vec<NapiToolMatch> {
        self.inner.search_tools(&query, limit as usize)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    /// Search tools using semantic similarity
    #[napi]
    pub fn search_tools_semantic(
        &self,
        query_embedding: Vec<f64>,
        limit: u32,
        threshold: f64,
    ) -> Vec<NapiToolMatch> {
        let query_f32: Vec<f32> = query_embedding.into_iter().map(|x| x as f32).collect();
        self.inner.search_tools_semantic(&query_f32, limit as usize, threshold as f32)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    /// Get tool count
    #[napi]
    pub fn tool_count(&self) -> u32 {
        self.inner.tool_count() as u32
    }
}

/// Create a new memory system
#[napi]
pub fn create_memory_system(data_dir: String, project_id: String) -> Result<MemorySystemHandle> {
    MemorySystemHandle::new(data_dir, project_id)
}

// ============================================================================
// Markdown Memory Types (NAPI)
// ============================================================================

use crate::memory::markdown::{
    DailyEntry as RustDailyEntry,
    DailyEntryType as RustDailyEntryType,
    MemoryCategory as RustMemoryCategory,
    MemorySection as RustMemorySection,
    MemoryContext as RustMemoryContext,
    MarkdownMemoryConfig as RustMarkdownMemoryConfig,
    MarkdownMemoryStore as RustMarkdownMemoryStore,
};

/// Daily entry type for NAPI
#[napi(string_enum)]
pub enum NapiDailyEntryType {
    Decision,
    Action,
    Output,
    Error,
    Solution,
}

impl From<NapiDailyEntryType> for RustDailyEntryType {
    fn from(t: NapiDailyEntryType) -> Self {
        match t {
            NapiDailyEntryType::Decision => RustDailyEntryType::Decision,
            NapiDailyEntryType::Action => RustDailyEntryType::Action,
            NapiDailyEntryType::Output => RustDailyEntryType::Output,
            NapiDailyEntryType::Error => RustDailyEntryType::Error,
            NapiDailyEntryType::Solution => RustDailyEntryType::Solution,
        }
    }
}

impl From<RustDailyEntryType> for NapiDailyEntryType {
    fn from(t: RustDailyEntryType) -> Self {
        match t {
            RustDailyEntryType::Decision => NapiDailyEntryType::Decision,
            RustDailyEntryType::Action => NapiDailyEntryType::Action,
            RustDailyEntryType::Output => NapiDailyEntryType::Output,
            RustDailyEntryType::Error => NapiDailyEntryType::Error,
            RustDailyEntryType::Solution => NapiDailyEntryType::Solution,
        }
    }
}

/// Memory category for NAPI
#[napi(string_enum)]
pub enum NapiMemoryCategory {
    UserPreferences,
    ProjectContext,
    KeyDecisions,
    LessonsLearned,
    SuccessfulSolutions,
}

impl From<NapiMemoryCategory> for RustMemoryCategory {
    fn from(c: NapiMemoryCategory) -> Self {
        match c {
            NapiMemoryCategory::UserPreferences => RustMemoryCategory::UserPreferences,
            NapiMemoryCategory::ProjectContext => RustMemoryCategory::ProjectContext,
            NapiMemoryCategory::KeyDecisions => RustMemoryCategory::KeyDecisions,
            NapiMemoryCategory::LessonsLearned => RustMemoryCategory::LessonsLearned,
            NapiMemoryCategory::SuccessfulSolutions => RustMemoryCategory::SuccessfulSolutions,
        }
    }
}

impl From<RustMemoryCategory> for NapiMemoryCategory {
    fn from(c: RustMemoryCategory) -> Self {
        match c {
            RustMemoryCategory::UserPreferences => NapiMemoryCategory::UserPreferences,
            RustMemoryCategory::ProjectContext => NapiMemoryCategory::ProjectContext,
            RustMemoryCategory::KeyDecisions => NapiMemoryCategory::KeyDecisions,
            RustMemoryCategory::LessonsLearned => NapiMemoryCategory::LessonsLearned,
            RustMemoryCategory::SuccessfulSolutions => NapiMemoryCategory::SuccessfulSolutions,
        }
    }
}

/// Daily entry for NAPI
#[napi(object)]
pub struct NapiDailyEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Entry type
    pub entry_type: String,
    /// Entry content
    pub content: String,
    /// Metadata as JSON string
    pub metadata: String,
}

impl From<RustDailyEntry> for NapiDailyEntry {
    fn from(e: RustDailyEntry) -> Self {
        Self {
            timestamp: e.timestamp,
            entry_type: e.entry_type.to_string(),
            content: e.content,
            metadata: serde_json::to_string(&e.metadata).unwrap_or_default(),
        }
    }
}

/// Memory section for NAPI
#[napi(object)]
pub struct NapiMemorySection {
    /// Category name (Chinese)
    pub category: String,
    /// Section content
    pub content: String,
    /// Last updated timestamp
    pub last_updated: String,
}

impl From<RustMemorySection> for NapiMemorySection {
    fn from(s: RustMemorySection) -> Self {
        Self {
            category: s.category.display_name().to_string(),
            content: s.content,
            last_updated: s.last_updated,
        }
    }
}

/// Memory context for NAPI
#[napi(object)]
pub struct NapiMemoryContext {
    /// Long-term memory content
    pub long_term: String,
    /// Recent daily notes
    pub daily: Vec<String>,
    /// Combined context string
    pub combined: String,
}

impl From<RustMemoryContext> for NapiMemoryContext {
    fn from(c: RustMemoryContext) -> Self {
        Self {
            long_term: c.long_term,
            daily: c.daily,
            combined: c.combined,
        }
    }
}

/// Markdown memory config for NAPI
#[napi(object)]
pub struct NapiMarkdownMemoryConfig {
    /// Base path for memory storage
    pub base_path: String,
    /// Project identifier
    pub project_id: String,
    /// Daily notes directory path
    pub daily_path: String,
    /// Long-term memory file path
    pub long_term_path: String,
}

impl From<RustMarkdownMemoryConfig> for NapiMarkdownMemoryConfig {
    fn from(c: RustMarkdownMemoryConfig) -> Self {
        Self {
            base_path: c.base_path.to_string_lossy().to_string(),
            project_id: c.project_id,
            daily_path: c.daily_path.to_string_lossy().to_string(),
            long_term_path: c.long_term_path.to_string_lossy().to_string(),
        }
    }
}

// ============================================================================
// Markdown Memory Store Handle (NAPI)
// ============================================================================

/// Handle to a markdown memory store
#[napi]
pub struct MarkdownMemoryHandle {
    inner: RustMarkdownMemoryStore,
}

#[napi]
impl MarkdownMemoryHandle {
    /// Create a new markdown memory store
    #[napi(constructor)]
    pub fn new(base_path: String, project_id: String) -> Self {
        let config = RustMarkdownMemoryConfig::new(&base_path, &project_id);
        Self {
            inner: RustMarkdownMemoryStore::new(config),
        }
    }

    /// Get the configuration
    #[napi]
    pub fn config(&self) -> NapiMarkdownMemoryConfig {
        self.inner.config().clone().into()
    }

    // ========================================================================
    // Daily Notes (Flow Layer)
    // ========================================================================

    /// Append a new entry to today's daily notes
    #[napi]
    pub fn append_daily_note(
        &self,
        entry_type: NapiDailyEntryType,
        content: String,
        metadata: Option<String>,
    ) -> Result<()> {
        let meta: std::collections::HashMap<String, serde_json::Value> = metadata
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let entry = RustMarkdownMemoryStore::create_entry(
            entry_type.into(),
            content,
            Some(meta),
        );

        self.inner
            .append_daily_note(&entry)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get content from today's daily notes
    #[napi]
    pub fn get_today_notes(&self) -> Result<String> {
        self.inner
            .get_today_notes()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load daily notes for a date range (starting from today, going back N days)
    #[napi]
    pub fn load_recent_daily_notes(&self, days: u32) -> Result<Vec<String>> {
        let today = chrono::Local::now().date_naive();
        let start = today - chrono::Duration::days((days - 1) as i64);
        self.inner
            .load_daily_notes(start, days as usize)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// List all available daily note dates
    #[napi]
    pub fn list_daily_note_dates(&self) -> Result<Vec<String>> {
        self.inner
            .list_daily_note_dates()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ========================================================================
    // Long-term Memory (Sediment Layer)
    // ========================================================================

    /// Load entire long-term memory file
    #[napi]
    pub fn load_long_term_memory(&self) -> Result<String> {
        self.inner
            .load_long_term_memory()
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Load specific category from long-term memory
    #[napi]
    pub fn load_category(&self, category: NapiMemoryCategory) -> Result<String> {
        self.inner
            .load_category(category.into())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Update or create a category in long-term memory
    #[napi]
    pub fn update_category(&self, category: NapiMemoryCategory, content: String) -> Result<()> {
        self.inner
            .update_category(category.into(), &content)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Merge new content into existing category
    #[napi]
    pub fn merge_to_category(&self, category: NapiMemoryCategory, update: String) -> Result<()> {
        self.inner
            .merge_to_category(category.into(), &update)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Get all memory sections
    #[napi]
    pub fn get_memory_sections(&self) -> Result<Vec<NapiMemorySection>> {
        self.inner
            .get_memory_sections()
            .map(|sections| sections.into_iter().map(Into::into).collect())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Add item to a category list
    #[napi]
    pub fn add_list_item(
        &self,
        category: NapiMemoryCategory,
        item: String,
        subtext: Option<String>,
    ) -> Result<()> {
        self.inner
            .add_list_item(category.into(), &item, subtext.as_deref())
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    /// Remove item from a category
    #[napi]
    pub fn remove_list_item(&self, category: NapiMemoryCategory, item_pattern: String) -> Result<()> {
        self.inner
            .remove_list_item(category.into(), &item_pattern)
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    // ========================================================================
    // Context Loading
    // ========================================================================

    /// Load combined memory context
    #[napi]
    pub fn load_context(&self, include_days: u32) -> Result<NapiMemoryContext> {
        self.inner
            .load_context(include_days as usize)
            .map(Into::into)
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}

/// Create a new markdown memory store
#[napi]
pub fn create_markdown_memory(base_path: String, project_id: String) -> MarkdownMemoryHandle {
    MarkdownMemoryHandle::new(base_path, project_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let chunks = chunk_text("Hello world".to_string(), None);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Hello world");
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(a, b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize_vector() {
        let v = vec![3.0, 4.0];
        let n = normalize_vector(v);
        assert!((n[0] - 0.6).abs() < 0.001);
        assert!((n[1] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_vector_bytes_roundtrip() {
        let original = vec![1.0, 2.0, 3.0];
        let bytes = vector_to_bytes(original.clone());
        let restored = bytes_to_vector(bytes);
        for (a, b) in original.iter().zip(restored.iter()) {
            assert!((*a - *b).abs() < 0.001);
        }
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("".to_string()), 0);
        assert_eq!(estimate_tokens("abcd".to_string()), 1);
        assert_eq!(estimate_tokens("abcdefgh".to_string()), 2);
    }
}
