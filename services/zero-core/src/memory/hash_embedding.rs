//! Hash-based Embedding Generation
//!
//! Provides deterministic, hash-based embedding generation for offline use cases.
//! Uses xxHash for fast hashing and SIMD for vector normalization.
//!
//! This is a fallback when real embeddings (OpenAI, Ollama) are not available.
//! While not semantically meaningful, it provides consistent results for the same input.


use xxhash_rust::xxh3::xxh3_64;

#[cfg(feature = "simd")]
use wide::f32x8;

use crate::memory::vector::normalize;

// ============================================================================
// Constants
// ============================================================================

/// Default embedding dimension (same as text-embedding-3-small)
pub const DEFAULT_DIMENSION: usize = 1536;

/// Word contribution factor for vector modification
const WORD_CONTRIBUTION: f32 = 0.1;

/// N-gram size for additional features
const NGRAM_SIZE: usize = 3;

// ============================================================================
// Hash Embedding Result
// ============================================================================

/// Result of generating a hash-based embedding
#[derive(Debug, Clone)]
pub struct HashEmbeddingResult {
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Embedding dimension
    pub dimension: usize,
    /// Model identifier
    pub model: String,
}

// ============================================================================
// Core Implementation
// ============================================================================

/// Generate a hash-based embedding vector for the given text.
///
/// This implementation:
/// 1. Uses xxHash for fast, high-quality hashing
/// 2. Generates pseudo-random base vector using sine function
/// 3. Adds word-level features for better discrimination
/// 4. Adds character n-gram features for subword information
/// 5. Normalizes the final vector using SIMD when available
pub fn generate_hash_embedding(text: &str, dimension: usize) -> Vec<f32> {
    let mut vector = vec![0.0f32; dimension];

    // Hash the full text using xxHash (64-bit)
    let text_hash = xxh3_64(text.as_bytes());
    let seed = text_hash as f64;

    // Generate base vector using sine function (deterministic pseudo-random)
    generate_base_vector(&mut vector, seed);

    // Add word-level features
    add_word_features(&mut vector, text);

    // Add character n-gram features for subword information
    add_ngram_features(&mut vector, text);

    // Normalize the vector using SIMD-accelerated function
    normalize(&mut vector);

    vector
}

/// Generate a hash embedding with the default dimension (1536)
pub fn generate_hash_embedding_default(text: &str) -> HashEmbeddingResult {
    let vector = generate_hash_embedding(text, DEFAULT_DIMENSION);
    HashEmbeddingResult {
        dimension: DEFAULT_DIMENSION,
        model: "hash".to_string(),
        vector,
    }
}

/// Generate embeddings for multiple texts (batch operation)
pub fn generate_hash_embeddings_batch(texts: &[&str], dimension: usize) -> Vec<Vec<f32>> {
    texts
        .iter()
        .map(|text| generate_hash_embedding(text, dimension))
        .collect()
}

// ============================================================================
// Internal Functions - SIMD Accelerated
// ============================================================================

/// Generate base vector using sine function (SIMD version)
#[cfg(feature = "simd")]
fn generate_base_vector(vector: &mut [f32], seed: f64) {
    let dimension = vector.len();
    let simd_len = dimension - (dimension % 8);

    // Process 8 elements at a time
    let mut i = 0;
    while i < simd_len {
        // Generate 8 values at once
        let mut values = [0.0f32; 8];
        for j in 0..8 {
            let idx = (i + j) as f64;
            let x = (seed + idx).sin() * 10000.0;
            values[j] = (x - x.floor()) as f32;
        }

        let simd_values = f32x8::new(values);
        let arr = simd_values.to_array();
        vector[i..i + 8].copy_from_slice(&arr);

        i += 8;
    }

    // Handle remaining elements
    for j in simd_len..dimension {
        let idx = j as f64;
        let x = (seed + idx).sin() * 10000.0;
        vector[j] = (x - x.floor()) as f32;
    }
}

/// Generate base vector using sine function (scalar version)
#[cfg(not(feature = "simd"))]
fn generate_base_vector(vector: &mut [f32], seed: f64) {
    for (i, v) in vector.iter_mut().enumerate() {
        let idx = i as f64;
        let x = (seed + idx).sin() * 10000.0;
        *v = (x - x.floor()) as f32;
    }
}

/// Add word-level features to the vector
fn add_word_features(vector: &mut [f32], text: &str) {
    let dimension = vector.len();
    let text_lower = text.to_lowercase();

    // Split by whitespace and add contribution for each word
    for word in text_lower.split_whitespace() {
        // Skip very short words (articles, etc.)
        if word.len() < 2 {
            continue;
        }

        // Hash the word and distribute contribution
        let word_hash = xxh3_64(word.as_bytes());
        let primary_idx = (word_hash as usize) % dimension;

        // Add primary contribution
        vector[primary_idx] += WORD_CONTRIBUTION;

        // Add secondary contribution for better distribution
        let secondary_hash = xxh3_64(&word_hash.to_le_bytes());
        let secondary_idx = (secondary_hash as usize) % dimension;
        vector[secondary_idx] += WORD_CONTRIBUTION * 0.5;
    }
}

/// Add character n-gram features for subword information
fn add_ngram_features(vector: &mut [f32], text: &str) {
    let dimension = vector.len();
    let text_lower = text.to_lowercase();
    let chars: Vec<char> = text_lower.chars().collect();

    if chars.len() < NGRAM_SIZE {
        return;
    }

    // Generate n-grams and add features
    for window in chars.windows(NGRAM_SIZE) {
        let ngram: String = window.iter().collect();
        let ngram_hash = xxh3_64(ngram.as_bytes());
        let idx = (ngram_hash as usize) % dimension;

        // Smaller contribution for n-grams
        vector[idx] += WORD_CONTRIBUTION * 0.25;
    }
}

/// Calculate similarity between two hash embeddings
pub fn hash_embedding_similarity(a: &[f32], b: &[f32]) -> f32 {
    crate::memory::vector::cosine_similarity(a, b)
}

// ============================================================================
// Advanced Features
// ============================================================================

/// Generate a hash embedding with position encoding
/// Useful for sequences where position matters
pub fn generate_positional_hash_embedding(
    text: &str,
    position: usize,
    max_position: usize,
    dimension: usize,
) -> Vec<f32> {
    let mut vector = generate_hash_embedding(text, dimension);

    // Add sinusoidal position encoding
    let pos = position as f64;
    let max_pos = max_position as f64;

    for (i, v) in vector.iter_mut().enumerate() {
        let dim = i as f64;
        // Use different frequencies for different dimensions
        let angle = pos / (max_pos.powf(2.0 * dim / dimension as f64));
        if i % 2 == 0 {
            *v += (angle.sin() * 0.1) as f32;
        } else {
            *v += (angle.cos() * 0.1) as f32;
        }
    }

    normalize(&mut vector);
    vector
}

/// Generate a combined embedding from multiple texts
/// Useful for document-level embeddings from chunks
pub fn generate_combined_hash_embedding(texts: &[&str], dimension: usize) -> Vec<f32> {
    if texts.is_empty() {
        return vec![0.0f32; dimension];
    }

    let mut combined = vec![0.0f32; dimension];

    // Average all embeddings
    for text in texts {
        let embedding = generate_hash_embedding(text, dimension);
        for (i, v) in embedding.iter().enumerate() {
            combined[i] += v;
        }
    }

    // Divide by count and normalize
    let count = texts.len() as f32;
    for v in combined.iter_mut() {
        *v /= count;
    }

    normalize(&mut combined);
    combined
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_embedding() {
        let embedding = generate_hash_embedding("hello world", 512);
        assert_eq!(embedding.len(), 512);

        // Check normalization (magnitude should be close to 1)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_deterministic() {
        let e1 = generate_hash_embedding("test string", 256);
        let e2 = generate_hash_embedding("test string", 256);

        for (a, b) in e1.iter().zip(e2.iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_different_inputs() {
        let e1 = generate_hash_embedding("hello", 256);
        let e2 = generate_hash_embedding("world", 256);

        // Should be different
        let diff: f32 = e1
            .iter()
            .zip(e2.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(diff > 0.1);
    }

    #[test]
    fn test_similarity() {
        let e1 = generate_hash_embedding("the quick brown fox", 512);
        let e2 = generate_hash_embedding("the quick brown fox", 512);
        let e3 = generate_hash_embedding("completely different text", 512);

        // Same text should have similarity 1.0
        let sim_same = hash_embedding_similarity(&e1, &e2);
        assert!((sim_same - 1.0).abs() < 0.01);

        // Different text should have lower similarity
        let sim_diff = hash_embedding_similarity(&e1, &e3);
        assert!(sim_diff < 0.9);
    }

    #[test]
    fn test_batch() {
        let texts = vec!["hello", "world", "test"];
        let embeddings = generate_hash_embeddings_batch(&texts, 128);

        assert_eq!(embeddings.len(), 3);
        for e in embeddings {
            assert_eq!(e.len(), 128);
        }
    }

    #[test]
    fn test_default_dimension() {
        let result = generate_hash_embedding_default("test");
        assert_eq!(result.dimension, DEFAULT_DIMENSION);
        assert_eq!(result.vector.len(), DEFAULT_DIMENSION);
        assert_eq!(result.model, "hash");
    }

    #[test]
    fn test_empty_input() {
        let embedding = generate_hash_embedding("", 256);
        assert_eq!(embedding.len(), 256);

        // Should still produce a valid normalized vector
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(magnitude.is_finite());
    }

    #[test]
    fn test_unicode() {
        let embedding = generate_hash_embedding("你好世界 🌍", 256);
        assert_eq!(embedding.len(), 256);

        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_combined_embedding() {
        let texts = vec!["first chunk", "second chunk", "third chunk"];
        let combined = generate_combined_hash_embedding(&texts, 256);

        assert_eq!(combined.len(), 256);
        let magnitude: f32 = combined.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }
}
