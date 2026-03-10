//! Zero Memory - Memory system for the Zero ecosystem.
//!
//! This module provides memory backends for storing and retrieving information:
//! - SQLite with FTS5 for keyword search and vector similarity
//! - Markdown files for human-readable memory
//!
//! ## Architecture
//!
//! The memory system supports hybrid search combining:
//! - Vector similarity (cosine distance on embeddings)
//! - Keyword search (BM25 via FTS5)
//!
//! ```text
//! Query → Embeddings → Vector Search ──┐
//!                                      ├── Hybrid Merge → Results
//! Query → FTS5 ─────→ Keyword Search ──┘
//! ```

pub mod chunker;
pub mod embeddings;
pub mod hybrid_search;
pub mod markdown;
pub mod qdrant;
pub mod sqlite;
pub mod traits;
pub mod vector;

// Re-export commonly used types
// Note: chunker re-exports from crate::memory::chunker for consistency
pub use chunker::{chunk_markdown, Chunk, ChunkerConfig};
pub use embeddings::{create_embedding_provider, EmbeddingProvider, NoopEmbedding, OpenAiEmbedding};
pub use hybrid_search::{HybridSearchEngine, DEFAULT_KEYWORD_WEIGHT, DEFAULT_VECTOR_WEIGHT};
pub use markdown::MarkdownMemory;
pub use qdrant::{QdrantMemory, QdrantMetadata};
pub use sqlite::SqliteMemory;
pub use traits::{Memory, MemoryCategory, MemoryEntry};
pub use vector::{bytes_to_vec, cosine_similarity, hybrid_merge, vec_to_bytes, ScoredResult};
