//! NAPI bindings for embedding index
//!
//! Provides JavaScript/TypeScript bindings for:
//! - In-memory embedding storage and retrieval
//! - KNN search with SIMD-accelerated similarity computation
//! - Serialization/deserialization for persistence

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::memory::vector::{
    batch_cosine_similarity, cosine_similarity as rust_cosine_similarity, knn_search, normalize,
    KnnResult,
};

// ============================================================================
// Types
// ============================================================================

/// Search result from embedding index
#[napi(object)]
pub struct NapiEmbeddingSearchResult {
    /// Embedding identifier
    pub id: String,
    /// Similarity score (0.0-1.0)
    pub score: f64,
}

impl From<KnnResult> for NapiEmbeddingSearchResult {
    fn from(r: KnnResult) -> Self {
        Self {
            id: r.id,
            score: f64::from(r.score),
        }
    }
}

/// Item for batch add operations
#[napi(object)]
pub struct NapiEmbeddingItem {
    /// Embedding identifier
    pub id: String,
    /// Embedding vector
    pub vector: Vec<f64>,
}

/// Index statistics
#[napi(object)]
pub struct NapiEmbeddingIndexStats {
    /// Total number of embeddings in the index
    pub count: u32,
    /// Embedding dimension
    pub dimension: u32,
    /// Approximate memory usage in bytes
    pub memory_bytes: u32,
}

// ============================================================================
// EmbeddingIndexHandle
// ============================================================================

/// Handle to an in-memory embedding index for fast KNN search.
///
/// Uses SIMD-accelerated cosine similarity for efficient search operations.
/// All vectors are stored as f32 internally for memory efficiency.
#[napi]
pub struct EmbeddingIndexHandle {
    vectors: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    dimension: usize,
}

#[napi]
impl EmbeddingIndexHandle {
    /// Create a new embedding index with the specified dimension.
    ///
    /// # Arguments
    /// * `dimension` - The dimension of vectors that will be stored (e.g., 1536 for OpenAI)
    #[napi(constructor)]
    pub fn new(dimension: u32) -> Self {
        Self {
            vectors: Arc::new(Mutex::new(HashMap::new())),
            dimension: dimension as usize,
        }
    }

    /// Add a single embedding to the index.
    ///
    /// If an embedding with the same ID already exists, it will be replaced.
    /// The vector will be normalized before storage for consistent similarity computation.
    ///
    /// # Arguments
    /// * `id` - Unique identifier for this embedding
    /// * `vector` - The embedding vector (must match index dimension)
    #[napi]
    pub fn add(&self, id: String, vector: Vec<f64>) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "Vector dimension mismatch: expected {}, got {}",
                    self.dimension,
                    vector.len()
                ),
            ));
        }

        let v: Vec<f32> = vector.into_iter().map(|x| x as f32).collect();
        let normalized = normalize(&v);

        let mut vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;
        vectors.insert(id, normalized);
        Ok(())
    }

    /// Add multiple embeddings in a batch operation.
    ///
    /// More efficient than calling add() repeatedly.
    ///
    /// # Arguments
    /// * `items` - Vector of {id, vector} items to add
    #[napi]
    pub fn add_batch(&self, items: Vec<NapiEmbeddingItem>) -> Result<()> {
        let mut vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;

        for item in items {
            if item.vector.len() != self.dimension {
                return Err(Error::new(
                    Status::InvalidArg,
                    format!(
                        "Vector dimension mismatch for '{}': expected {}, got {}",
                        item.id,
                        self.dimension,
                        item.vector.len()
                    ),
                ));
            }

            let v: Vec<f32> = item.vector.into_iter().map(|x| x as f32).collect();
            let normalized = normalize(&v);
            vectors.insert(item.id, normalized);
        }

        Ok(())
    }

    /// Search for the K nearest neighbors to a query vector.
    ///
    /// Uses SIMD-accelerated cosine similarity for fast computation.
    ///
    /// # Arguments
    /// * `query` - The query vector to search for
    /// * `k` - Maximum number of results to return
    /// * `threshold` - Minimum similarity threshold (0.0-1.0)
    ///
    /// # Returns
    /// Vector of {id, score} results sorted by descending similarity
    #[napi]
    pub fn search(
        &self,
        query: Vec<f64>,
        k: u32,
        threshold: f64,
    ) -> Result<Vec<NapiEmbeddingSearchResult>> {
        if query.len() != self.dimension {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "Query dimension mismatch: expected {}, got {}",
                    self.dimension,
                    query.len()
                ),
            ));
        }

        let q: Vec<f32> = query.into_iter().map(|x| x as f32).collect();
        let normalized_query = normalize(&q);

        let vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;

        let results = knn_search(&normalized_query, &vectors, k as usize, threshold as f32);
        Ok(results.into_iter().map(NapiEmbeddingSearchResult::from).collect())
    }

    /// Compute similarity between a query and a specific embedding.
    ///
    /// # Arguments
    /// * `query` - The query vector
    /// * `id` - The ID of the embedding to compare against
    ///
    /// # Returns
    /// Similarity score (0.0-1.0), or null if embedding not found
    #[napi]
    pub fn similarity(&self, query: Vec<f64>, id: String) -> Result<Option<f64>> {
        if query.len() != self.dimension {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "Query dimension mismatch: expected {}, got {}",
                    self.dimension,
                    query.len()
                ),
            ));
        }

        let vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;

        let q: Vec<f32> = query.into_iter().map(|x| x as f32).collect();
        let normalized_query = normalize(&q);

        Ok(vectors.get(&id).map(|v| {
            f64::from(rust_cosine_similarity(&normalized_query, v))
        }))
    }

    /// Batch similarity: compute similarities between a query and multiple embeddings.
    ///
    /// # Arguments
    /// * `query` - The query vector
    /// * `ids` - IDs of embeddings to compare against
    ///
    /// # Returns
    /// Vector of {id, score} results (only for existing embeddings)
    #[napi]
    pub fn batch_similarity(
        &self,
        query: Vec<f64>,
        ids: Vec<String>,
    ) -> Result<Vec<NapiEmbeddingSearchResult>> {
        if query.len() != self.dimension {
            return Err(Error::new(
                Status::InvalidArg,
                format!(
                    "Query dimension mismatch: expected {}, got {}",
                    self.dimension,
                    query.len()
                ),
            ));
        }

        let vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;

        let q: Vec<f32> = query.into_iter().map(|x| x as f32).collect();
        let normalized_query = normalize(&q);

        let target_vectors: Vec<Vec<f32>> = ids
            .iter()
            .filter_map(|id| vectors.get(id).cloned())
            .collect();

        let scores = batch_cosine_similarity(&normalized_query, &target_vectors);

        let mut results: Vec<NapiEmbeddingSearchResult> = ids
            .into_iter()
            .zip(scores.into_iter())
            .filter_map(|(id, score)| {
                if vectors.contains_key(&id) {
                    Some(NapiEmbeddingSearchResult {
                        id,
                        score: f64::from(score),
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

        Ok(results)
    }

    /// Remove an embedding from the index.
    ///
    /// # Arguments
    /// * `id` - The ID of the embedding to remove
    ///
    /// # Returns
    /// True if the embedding was removed, false if it didn't exist
    #[napi]
    pub fn remove(&self, id: String) -> Result<bool> {
        let mut vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;
        Ok(vectors.remove(&id).is_some())
    }

    /// Check if an embedding exists in the index.
    ///
    /// # Arguments
    /// * `id` - The ID to check
    #[napi]
    pub fn has(&self, id: String) -> Result<bool> {
        let vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;
        Ok(vectors.contains_key(&id))
    }

    /// Get all embedding IDs in the index.
    #[napi]
    pub fn ids(&self) -> Result<Vec<String>> {
        let vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;
        Ok(vectors.keys().cloned().collect())
    }

    /// Get index statistics.
    #[napi]
    pub fn stats(&self) -> Result<NapiEmbeddingIndexStats> {
        let vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;

        let count = vectors.len();
        // Each f32 is 4 bytes, plus overhead for HashMap
        let memory_bytes = count * self.dimension * 4 + count * 64; // 64 bytes overhead per entry

        Ok(NapiEmbeddingIndexStats {
            count: count as u32,
            dimension: self.dimension as u32,
            memory_bytes: memory_bytes as u32,
        })
    }

    /// Clear all embeddings from the index.
    #[napi]
    pub fn clear(&self) -> Result<()> {
        let mut vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;
        vectors.clear();
        Ok(())
    }

    /// Serialize the index to bytes for persistence.
    ///
    /// Format: dimension (4 bytes) + count (4 bytes) + entries
    /// Each entry: id_len (4 bytes) + id (utf8) + vector (dimension * 4 bytes)
    #[napi]
    pub fn to_bytes(&self) -> Result<Buffer> {
        let vectors = self.vectors.lock().map_err(|e| {
            Error::new(Status::GenericFailure, format!("Lock error: {}", e))
        })?;

        let mut bytes = Vec::new();

        // Write dimension
        bytes.extend_from_slice(&(self.dimension as u32).to_le_bytes());

        // Write count
        bytes.extend_from_slice(&(vectors.len() as u32).to_le_bytes());

        // Write each entry
        for (id, vector) in vectors.iter() {
            let id_bytes = id.as_bytes();
            bytes.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(id_bytes);

            for &f in vector.iter() {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
        }

        Ok(Buffer::from(bytes))
    }

    /// Deserialize an index from bytes.
    ///
    /// # Arguments
    /// * `bytes` - Serialized index data from to_bytes()
    #[napi(factory)]
    pub fn from_bytes(bytes: Buffer) -> Result<Self> {
        let data: &[u8] = &bytes;
        let mut offset = 0;

        // Read dimension
        if data.len() < 8 {
            return Err(Error::new(
                Status::InvalidArg,
                "Invalid buffer: too short for header",
            ));
        }

        let dimension = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|_| Error::new(Status::InvalidArg, "Invalid dimension"))?,
        ) as usize;
        offset += 4;

        let count = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|_| Error::new(Status::InvalidArg, "Invalid count"))?,
        ) as usize;
        offset += 4;

        let mut vectors = HashMap::with_capacity(count);

        for _ in 0..count {
            // Read id length
            if offset + 4 > data.len() {
                return Err(Error::new(Status::InvalidArg, "Invalid buffer: truncated"));
            }
            let id_len = u32::from_le_bytes(
                data[offset..offset + 4]
                    .try_into()
                    .map_err(|_| Error::new(Status::InvalidArg, "Invalid id length"))?,
            ) as usize;
            offset += 4;

            // Read id
            if offset + id_len > data.len() {
                return Err(Error::new(Status::InvalidArg, "Invalid buffer: truncated id"));
            }
            let id = String::from_utf8(data[offset..offset + id_len].to_vec())
                .map_err(|_| Error::new(Status::InvalidArg, "Invalid UTF-8 in id"))?;
            offset += id_len;

            // Read vector
            let vector_bytes = dimension * 4;
            if offset + vector_bytes > data.len() {
                return Err(Error::new(
                    Status::InvalidArg,
                    "Invalid buffer: truncated vector",
                ));
            }

            let mut vector = Vec::with_capacity(dimension);
            for i in 0..dimension {
                let f = f32::from_le_bytes(
                    data[offset + i * 4..offset + i * 4 + 4]
                        .try_into()
                        .map_err(|_| Error::new(Status::InvalidArg, "Invalid float"))?,
                );
                vector.push(f);
            }
            offset += vector_bytes;

            vectors.insert(id, vector);
        }

        Ok(Self {
            vectors: Arc::new(Mutex::new(vectors)),
            dimension,
        })
    }
}

// ============================================================================
// Factory Functions
// ============================================================================

/// Create a new embedding index with the specified dimension.
///
/// # Arguments
/// * `dimension` - The dimension of vectors that will be stored (e.g., 1536 for OpenAI)
#[napi]
pub fn create_embedding_index(dimension: u32) -> EmbeddingIndexHandle {
    EmbeddingIndexHandle::new(dimension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_index_add_and_search() {
        let index = EmbeddingIndexHandle::new(3);

        // Add some vectors
        index.add("a".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
        index.add("b".to_string(), vec![0.0, 1.0, 0.0]).unwrap();
        index.add("c".to_string(), vec![0.7, 0.7, 0.0]).unwrap();

        // Search
        let results = index.search(vec![1.0, 0.0, 0.0], 2, 0.5).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!(results[0].score > 0.99);
    }

    #[test]
    fn test_embedding_index_batch_add() {
        let index = EmbeddingIndexHandle::new(3);

        let items = vec![
            NapiEmbeddingItem {
                id: "a".to_string(),
                vector: vec![1.0, 0.0, 0.0],
            },
            NapiEmbeddingItem {
                id: "b".to_string(),
                vector: vec![0.0, 1.0, 0.0],
            },
        ];

        index.add_batch(items).unwrap();

        let stats = index.stats().unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.dimension, 3);
    }

    #[test]
    fn test_embedding_index_remove() {
        let index = EmbeddingIndexHandle::new(3);
        index.add("a".to_string(), vec![1.0, 0.0, 0.0]).unwrap();

        assert!(index.has("a".to_string()).unwrap());
        assert!(index.remove("a".to_string()).unwrap());
        assert!(!index.has("a".to_string()).unwrap());
        assert!(!index.remove("a".to_string()).unwrap());
    }

    #[test]
    fn test_embedding_index_serialization() {
        let index = EmbeddingIndexHandle::new(3);
        index.add("a".to_string(), vec![1.0, 0.0, 0.0]).unwrap();
        index.add("b".to_string(), vec![0.0, 1.0, 0.0]).unwrap();

        let bytes = index.to_bytes().unwrap();
        let restored = EmbeddingIndexHandle::from_bytes(bytes).unwrap();

        let stats = restored.stats().unwrap();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.dimension, 3);
        assert!(restored.has("a".to_string()).unwrap());
        assert!(restored.has("b".to_string()).unwrap());
    }

    #[test]
    fn test_embedding_index_dimension_check() {
        let index = EmbeddingIndexHandle::new(3);

        // Wrong dimension should fail
        let result = index.add("a".to_string(), vec![1.0, 0.0]);
        assert!(result.is_err());
    }
}

// ============================================================================
// Hash Embedding NAPI Bindings (Phase 12)
// ============================================================================

use crate::memory::hash_embedding;

/// Result of generating a hash-based embedding
#[napi(object)]
pub struct NapiHashEmbeddingResult {
    /// The embedding vector
    pub vector: Vec<f64>,
    /// Embedding dimension
    pub dimension: u32,
    /// Model identifier (always "hash")
    pub model: String,
}

/// Generate a hash-based embedding for the given text.
///
/// This is a deterministic, offline-capable fallback when real embeddings
/// (OpenAI, Ollama) are not available. While not semantically meaningful,
/// it provides consistent results for the same input.
///
/// # Arguments
/// * `text` - The text to embed
/// * `dimension` - The desired embedding dimension (default: 1536)
#[napi]
pub fn generate_hash_embedding(text: String, dimension: Option<u32>) -> Vec<f64> {
    let dim = dimension.unwrap_or(hash_embedding::DEFAULT_DIMENSION as u32) as usize;
    let vector = hash_embedding::generate_hash_embedding(&text, dim);
    vector.into_iter().map(f64::from).collect()
}

/// Generate a hash-based embedding with full result info.
///
/// # Arguments
/// * `text` - The text to embed
#[napi]
pub fn generate_hash_embedding_with_info(text: String) -> NapiHashEmbeddingResult {
    let result = hash_embedding::generate_hash_embedding_default(&text);
    NapiHashEmbeddingResult {
        vector: result.vector.into_iter().map(f64::from).collect(),
        dimension: result.dimension as u32,
        model: result.model,
    }
}

/// Generate hash-based embeddings for multiple texts (batch operation).
///
/// More efficient than calling generate_hash_embedding repeatedly.
///
/// # Arguments
/// * `texts` - The texts to embed
/// * `dimension` - The desired embedding dimension (default: 1536)
#[napi]
pub fn generate_hash_embeddings_batch(
    texts: Vec<String>,
    dimension: Option<u32>,
) -> Vec<Vec<f64>> {
    let dim = dimension.unwrap_or(hash_embedding::DEFAULT_DIMENSION as u32) as usize;
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let embeddings = hash_embedding::generate_hash_embeddings_batch(&text_refs, dim);

    embeddings
        .into_iter()
        .map(|v| v.into_iter().map(f64::from).collect())
        .collect()
}

/// Generate a combined hash embedding from multiple texts.
///
/// Useful for document-level embeddings from chunks.
///
/// # Arguments
/// * `texts` - The texts to combine
/// * `dimension` - The desired embedding dimension (default: 1536)
#[napi]
pub fn generate_combined_hash_embedding(texts: Vec<String>, dimension: Option<u32>) -> Vec<f64> {
    let dim = dimension.unwrap_or(hash_embedding::DEFAULT_DIMENSION as u32) as usize;
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    let vector = hash_embedding::generate_combined_hash_embedding(&text_refs, dim);
    vector.into_iter().map(f64::from).collect()
}

/// Generate a hash embedding with position encoding.
///
/// Useful for sequences where position matters.
///
/// # Arguments
/// * `text` - The text to embed
/// * `position` - Position in the sequence
/// * `max_position` - Maximum position in the sequence
/// * `dimension` - The desired embedding dimension (default: 1536)
#[napi]
pub fn generate_positional_hash_embedding(
    text: String,
    position: u32,
    max_position: u32,
    dimension: Option<u32>,
) -> Vec<f64> {
    let dim = dimension.unwrap_or(hash_embedding::DEFAULT_DIMENSION as u32) as usize;
    let vector = hash_embedding::generate_positional_hash_embedding(
        &text,
        position as usize,
        max_position as usize,
        dim,
    );
    vector.into_iter().map(f64::from).collect()
}

/// Calculate cosine similarity between two embeddings.
///
/// Uses SIMD-accelerated computation.
///
/// # Arguments
/// * `a` - First embedding vector
/// * `b` - Second embedding vector
#[napi]
pub fn hash_embedding_similarity(a: Vec<f64>, b: Vec<f64>) -> f64 {
    let a_f32: Vec<f32> = a.into_iter().map(|x| x as f32).collect();
    let b_f32: Vec<f32> = b.into_iter().map(|x| x as f32).collect();
    f64::from(rust_cosine_similarity(&a_f32, &b_f32))
}
