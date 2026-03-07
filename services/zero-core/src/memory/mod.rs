//! Memory module - text chunking, vector operations, embeddings, history, tokenization, unified system
//!
//! This module provides:
//! - **chunker**: Line-based markdown chunking
//! - **vector**: Cosine similarity, normalization, hybrid merge
//! - **embedding**: Embedding provider trait and implementations
//! - **history**: Edit and decision history tracking
//! - **tokenizer**: LLM token counting with caching
//! - **system**: Unified memory system entry point
//!
//! Adapted from zero-memory for use in zero-core NAPI bindings.

pub mod chunker;
pub mod embedding;
pub mod hash_embedding;
pub mod history;
pub mod system;
pub mod tokenizer;
pub mod vector;

// Re-export main types
pub use chunker::{chunk_markdown, Chunk, ChunkerConfig};
pub use embedding::{create_embedding_provider, EmbeddingProvider, EmbeddingConfig, NoopEmbedding, OpenAiEmbedding};
pub use hash_embedding::{
    generate_hash_embedding, generate_hash_embedding_default, generate_hash_embeddings_batch,
    generate_combined_hash_embedding, generate_positional_hash_embedding, hash_embedding_similarity,
    HashEmbeddingResult, DEFAULT_DIMENSION,
};
pub use vector::{
    bytes_to_vec, cosine_similarity, dot_product, euclidean_distance, hybrid_merge, normalize,
    vec_to_bytes, ScoredResult, KnnResult, knn_search, knn_search_indexed, batch_cosine_similarity,
};
pub use history::{
    // Edit types
    EditRecord, EditSession, FileEdit, FileEditType, EditStats, AgentStats,
    // Decision types
    DecisionRecord, DecisionType, ArchitectureDecisionRecord, AdrStatus, Alternative,
    // Store
    HistoryStore,
};
pub use tokenizer::{
    estimate_tokens, estimate_tokens_batch, fits_token_budget, truncate_to_tokens,
    BatchCountResult, TokenCounter, TokenCounterConfig, TokenizerModel,
};
pub use system::{
    // Snapshot types
    MemorySnapshot, HistorySnapshot, VectorSnapshot,
    // Stats types
    MemoryStats, HistoryStats, VectorStats, TokenizerStats,
    // Options and results
    ImportOptions, ImportResult, CleanupResult,
    // Main system
    MemorySystem,
    // Vector store types
    StoredEmbedding, VectorStore,
    // Tool registry types
    ToolDefinition, ToolMatch,
};
