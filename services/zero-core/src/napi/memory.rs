//! NAPI bindings for memory module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - Text chunking
//! - Vector operations
//! - Embedding utilities

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::memory::{
    chunker::{chunk_markdown, ChunkerConfig as RustChunkerConfig, Chunk as RustChunk},
    vector::{
        bytes_to_vec as rust_bytes_to_vec, cosine_similarity as rust_cosine_similarity,
        euclidean_distance, hybrid_merge as rust_hybrid_merge, normalize,
        vec_to_bytes as rust_vec_to_bytes, ScoredResult as RustScoredResult,
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
