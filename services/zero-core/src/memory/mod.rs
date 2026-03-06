//! Memory module - text chunking, vector operations, embeddings, history, tokenization
//!
//! This module provides:
//! - **chunker**: Line-based markdown chunking
//! - **vector**: Cosine similarity, normalization, hybrid merge
//! - **embedding**: Embedding provider trait and implementations
//! - **history**: Edit and decision history tracking
//! - **tokenizer**: LLM token counting with caching
//!
//! Adapted from zero-memory for use in zero-core NAPI bindings.

pub mod chunker;
pub mod embedding;
pub mod history;
pub mod tokenizer;
pub mod vector;

// Re-export main types
pub use chunker::{chunk_markdown, Chunk, ChunkerConfig};
pub use embedding::{create_embedding_provider, EmbeddingProvider, EmbeddingConfig, NoopEmbedding, OpenAiEmbedding};
pub use vector::{
    bytes_to_vec, cosine_similarity, dot_product, euclidean_distance, hybrid_merge, normalize,
    vec_to_bytes, ScoredResult,
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
