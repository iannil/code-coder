//! Vector operations — cosine similarity, normalization, hybrid merge.
//!
//! SIMD-accelerated implementations using the `wide` crate for 4-8x speedup
//! on typical embedding dimensions (512-4096).
//!
//! Adapted from zero-memory/src/vector.rs

use serde::{Deserialize, Serialize};

#[cfg(feature = "simd")]
use wide::f32x8;

/// Horizontal sum: reduce f32x8 to a single f32 by summing all lanes.
#[cfg(feature = "simd")]
#[inline]
fn horizontal_sum(v: f32x8) -> f32 {
    let arr = v.to_array();
    arr[0] + arr[1] + arr[2] + arr[3] + arr[4] + arr[5] + arr[6] + arr[7]
}

/// Cosine similarity between two vectors. Returns 0.0–1.0.
///
/// When the `simd` feature is enabled (default), this uses SIMD instructions
/// to process 8 elements per iteration, providing ~5-8x speedup on large vectors.
#[cfg(feature = "simd")]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let len = a.len();
    let simd_len = len - (len % 8);

    let mut dot_sum = f32x8::ZERO;
    let mut norm_a_sum = f32x8::ZERO;
    let mut norm_b_sum = f32x8::ZERO;

    // SIMD loop: process 8 elements at a time
    let mut i = 0;
    while i < simd_len {
        let va = f32x8::new([
            a[i], a[i + 1], a[i + 2], a[i + 3], a[i + 4], a[i + 5], a[i + 6], a[i + 7],
        ]);
        let vb = f32x8::new([
            b[i], b[i + 1], b[i + 2], b[i + 3], b[i + 4], b[i + 5], b[i + 6], b[i + 7],
        ]);

        dot_sum += va * vb;
        norm_a_sum += va * va;
        norm_b_sum += vb * vb;

        i += 8;
    }

    // Horizontal sum: reduce f32x8 to f32
    let mut dot = horizontal_sum(dot_sum);
    let mut norm_a = horizontal_sum(norm_a_sum);
    let mut norm_b = horizontal_sum(norm_b_sum);

    // Scalar tail: handle remaining elements
    for j in simd_len..len {
        let x = a[j];
        let y = b[j];
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = (norm_a * norm_b).sqrt();
    if denom < f32::EPSILON {
        return 0.0;
    }

    (dot / denom).clamp(0.0, 1.0)
}

/// Scalar fallback for cosine similarity when SIMD is disabled.
#[cfg(not(feature = "simd"))]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f64;
    let mut norm_a = 0.0_f64;
    let mut norm_b = 0.0_f64;

    for (x, y) in a.iter().zip(b.iter()) {
        let x = f64::from(*x);
        let y = f64::from(*y);
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if !denom.is_finite() || denom < f64::EPSILON {
        return 0.0;
    }

    let raw = dot / denom;
    if !raw.is_finite() {
        return 0.0;
    }

    #[allow(clippy::cast_possible_truncation)]
    let sim = raw.clamp(0.0, 1.0) as f32;
    sim
}

/// Euclidean distance between two vectors (SIMD-accelerated)
#[cfg(feature = "simd")]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return f32::MAX;
    }

    let len = a.len();
    let simd_len = len - (len % 8);

    let mut sum_sq = f32x8::ZERO;

    // SIMD loop
    let mut i = 0;
    while i < simd_len {
        let va = f32x8::new([
            a[i], a[i + 1], a[i + 2], a[i + 3], a[i + 4], a[i + 5], a[i + 6], a[i + 7],
        ]);
        let vb = f32x8::new([
            b[i], b[i + 1], b[i + 2], b[i + 3], b[i + 4], b[i + 5], b[i + 6], b[i + 7],
        ]);
        let diff = va - vb;
        sum_sq += diff * diff;
        i += 8;
    }

    let mut sum = horizontal_sum(sum_sq);

    // Scalar tail
    for j in simd_len..len {
        let diff = a[j] - b[j];
        sum += diff * diff;
    }

    sum.sqrt()
}

/// Scalar fallback for euclidean distance
#[cfg(not(feature = "simd"))]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return f32::MAX;
    }

    let sum: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = f64::from(*x) - f64::from(*y);
            diff * diff
        })
        .sum();

    #[allow(clippy::cast_possible_truncation)]
    let dist = sum.sqrt() as f32;
    dist
}

/// Normalize a vector to unit length (L2 normalization) - SIMD-accelerated
#[cfg(feature = "simd")]
pub fn normalize(v: &[f32]) -> Vec<f32> {
    if v.is_empty() {
        return vec![];
    }

    let len = v.len();
    let simd_len = len - (len % 8);

    // Compute norm using SIMD
    let mut norm_sum = f32x8::ZERO;
    let mut i = 0;
    while i < simd_len {
        let va = f32x8::new([
            v[i], v[i + 1], v[i + 2], v[i + 3], v[i + 4], v[i + 5], v[i + 6], v[i + 7],
        ]);
        norm_sum += va * va;
        i += 8;
    }

    let mut norm_sq = horizontal_sum(norm_sum);

    // Scalar tail for norm
    for j in simd_len..len {
        norm_sq += v[j] * v[j];
    }

    let norm = norm_sq.sqrt();
    if norm < f32::EPSILON {
        return vec![0.0; len];
    }

    let inv_norm = 1.0 / norm;

    // Scale using SIMD
    let mut result = Vec::with_capacity(len);
    let inv_norm_v = f32x8::splat(inv_norm);

    i = 0;
    while i < simd_len {
        let va = f32x8::new([
            v[i], v[i + 1], v[i + 2], v[i + 3], v[i + 4], v[i + 5], v[i + 6], v[i + 7],
        ]);
        let scaled = va * inv_norm_v;
        let arr = scaled.to_array();
        result.extend_from_slice(&arr);
        i += 8;
    }

    // Scalar tail for scaling
    for j in simd_len..len {
        result.push(v[j] * inv_norm);
    }

    result
}

/// Scalar fallback for normalize
#[cfg(not(feature = "simd"))]
pub fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f64 = v.iter().map(|x| f64::from(*x) * f64::from(*x)).sum::<f64>().sqrt();

    if norm < f64::EPSILON {
        return vec![0.0; v.len()];
    }

    #[allow(clippy::cast_possible_truncation)]
    v.iter().map(|x| (*x as f64 / norm) as f32).collect()
}

/// Serialize f32 vector to bytes (little-endian)
pub fn vec_to_bytes(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for &f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

/// Deserialize bytes to f32 vector (little-endian)
pub fn bytes_to_vec(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
            f32::from_le_bytes(arr)
        })
        .collect()
}

/// A scored result for hybrid merging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredResult {
    /// Result identifier
    pub id: String,
    /// Vector similarity score (if available)
    #[serde(rename = "vectorScore")]
    pub vector_score: Option<f32>,
    /// Keyword/BM25 score (if available)
    #[serde(rename = "keywordScore")]
    pub keyword_score: Option<f32>,
    /// Final combined score
    #[serde(rename = "finalScore")]
    pub final_score: f32,
}

/// Hybrid merge: combine vector and keyword results with weighted fusion.
///
/// Normalizes each score set to [0, 1], then computes:
///   `final_score` = `vector_weight` * `vector_score` + `keyword_weight` * `keyword_score`
///
/// Deduplicates by id, keeping the best score from each source.
pub fn hybrid_merge(
    vector_results: &[(String, f32)], // (id, cosine_similarity)
    keyword_results: &[(String, f32)], // (id, bm25_score)
    vector_weight: f32,
    keyword_weight: f32,
    limit: usize,
) -> Vec<ScoredResult> {
    use std::collections::HashMap;

    let mut map: HashMap<String, ScoredResult> = HashMap::new();

    // Normalize vector scores (already 0–1 from cosine similarity)
    for (id, score) in vector_results {
        map.entry(id.clone())
            .and_modify(|r| r.vector_score = Some(*score))
            .or_insert_with(|| ScoredResult {
                id: id.clone(),
                vector_score: Some(*score),
                keyword_score: None,
                final_score: 0.0,
            });
    }

    // Normalize keyword scores (BM25 can be any positive number)
    let max_kw = keyword_results
        .iter()
        .map(|(_, s)| *s)
        .fold(0.0_f32, f32::max);
    let max_kw = if max_kw < f32::EPSILON { 1.0 } else { max_kw };

    for (id, score) in keyword_results {
        let normalized = score / max_kw;
        map.entry(id.clone())
            .and_modify(|r| r.keyword_score = Some(normalized))
            .or_insert_with(|| ScoredResult {
                id: id.clone(),
                vector_score: None,
                keyword_score: Some(normalized),
                final_score: 0.0,
            });
    }

    // Compute final scores
    let mut results: Vec<ScoredResult> = map
        .into_values()
        .map(|mut r| {
            let vs = r.vector_score.unwrap_or(0.0);
            let ks = r.keyword_score.unwrap_or(0.0);
            r.final_score = vector_weight * vs + keyword_weight * ks;
            r
        })
        .collect();

    results.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    results
}

/// Calculate dot product of two vectors (SIMD-accelerated)
#[cfg(feature = "simd")]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let len = a.len();
    let simd_len = len - (len % 8);

    let mut dot_sum = f32x8::ZERO;

    let mut i = 0;
    while i < simd_len {
        let va = f32x8::new([
            a[i], a[i + 1], a[i + 2], a[i + 3], a[i + 4], a[i + 5], a[i + 6], a[i + 7],
        ]);
        let vb = f32x8::new([
            b[i], b[i + 1], b[i + 2], b[i + 3], b[i + 4], b[i + 5], b[i + 6], b[i + 7],
        ]);
        dot_sum += va * vb;
        i += 8;
    }

    let mut dot = horizontal_sum(dot_sum);

    // Scalar tail
    for j in simd_len..len {
        dot += a[j] * b[j];
    }

    dot
}

/// Scalar fallback for dot product
#[cfg(not(feature = "simd"))]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let sum: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();

    sum as f32
}

/// Add two vectors element-wise
pub fn vec_add(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b.iter()).map(|(x, y)| x + y).collect()
}

/// Multiply vector by scalar
pub fn vec_scale(v: &[f32], scalar: f32) -> Vec<f32> {
    v.iter().map(|x| x * scalar).collect()
}

/// Batch cosine similarity: compute similarities between a query and multiple vectors.
/// Uses SIMD acceleration when available for significant speedup on large batches.
///
/// # Arguments
/// * `query` - The query vector to compare against
/// * `vectors` - Slice of vectors to compare with the query
///
/// # Returns
/// Vector of similarity scores (0.0-1.0), one per input vector
pub fn batch_cosine_similarity(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors
        .iter()
        .map(|v| cosine_similarity(query, v))
        .collect()
}

/// Search result from KNN search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnnResult {
    /// Result identifier
    pub id: String,
    /// Similarity score (0.0-1.0)
    pub score: f32,
}

/// K-Nearest Neighbors search: find the K most similar vectors to a query.
///
/// # Arguments
/// * `query` - The query vector to search for
/// * `vectors` - Map of id -> vector to search through
/// * `k` - Maximum number of results to return
/// * `threshold` - Minimum similarity threshold (0.0-1.0)
///
/// # Returns
/// Vector of (id, score) pairs sorted by descending similarity
pub fn knn_search(
    query: &[f32],
    vectors: &std::collections::HashMap<String, Vec<f32>>,
    k: usize,
    threshold: f32,
) -> Vec<KnnResult> {
    let mut results: Vec<KnnResult> = vectors
        .iter()
        .filter_map(|(id, v)| {
            let sim = cosine_similarity(query, v);
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

    // Sort by similarity (descending)
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
    results
}

/// Batch KNN search with pre-sorted vectors (for when vectors are stored with IDs)
/// More efficient than knn_search when vectors are stored as a Vec with indices
pub fn knn_search_indexed(
    query: &[f32],
    vectors: &[(String, Vec<f32>)],
    k: usize,
    threshold: f32,
) -> Vec<KnnResult> {
    let mut results: Vec<KnnResult> = vectors
        .iter()
        .filter_map(|(id, v)| {
            let sim = cosine_similarity(query, v);
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

    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn cosine_similar_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.1, 2.1, 3.1];
        let sim = cosine_similarity(&a, &b);
        assert!(sim > 0.99);
    }

    #[test]
    fn cosine_empty_returns_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn cosine_mismatched_lengths() {
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn cosine_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn vec_bytes_roundtrip() {
        let original = vec![1.0_f32, -2.5, 3.14, 0.0, f32::MAX];
        let bytes = vec_to_bytes(&original);
        let restored = bytes_to_vec(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn vec_bytes_empty() {
        let bytes = vec_to_bytes(&[]);
        assert!(bytes.is_empty());
        let restored = bytes_to_vec(&bytes);
        assert!(restored.is_empty());
    }

    #[test]
    fn hybrid_merge_vector_only() {
        let vec_results = vec![("a".into(), 0.9), ("b".into(), 0.5)];
        let merged = hybrid_merge(&vec_results, &[], 0.7, 0.3, 10);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, "a");
        assert!(merged[0].final_score > merged[1].final_score);
    }

    #[test]
    fn hybrid_merge_keyword_only() {
        let kw_results = vec![("x".into(), 10.0), ("y".into(), 5.0)];
        let merged = hybrid_merge(&[], &kw_results, 0.7, 0.3, 10);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, "x");
    }

    #[test]
    fn hybrid_merge_deduplicates() {
        let vec_results = vec![("a".into(), 0.9)];
        let kw_results = vec![("a".into(), 10.0)];
        let merged = hybrid_merge(&vec_results, &kw_results, 0.7, 0.3, 10);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, "a");
        assert!(merged[0].vector_score.is_some());
        assert!(merged[0].keyword_score.is_some());
    }

    #[test]
    fn hybrid_merge_respects_limit() {
        let vec_results: Vec<(String, f32)> = (0..20)
            .map(|i| (format!("item_{i}"), 1.0 - i as f32 * 0.05))
            .collect();
        let merged = hybrid_merge(&vec_results, &[], 1.0, 0.0, 5);
        assert_eq!(merged.len(), 5);
    }

    #[test]
    fn hybrid_merge_empty_inputs() {
        let merged = hybrid_merge(&[], &[], 0.7, 0.3, 10);
        assert!(merged.is_empty());
    }

    #[test]
    fn normalize_unit_vector() {
        let v = vec![1.0, 0.0, 0.0];
        let n = normalize(&v);
        assert!((n[0] - 1.0).abs() < 0.001);
        assert!(n[1].abs() < 0.001);
        assert!(n[2].abs() < 0.001);
    }

    #[test]
    fn normalize_zero_vector() {
        let v = vec![0.0, 0.0, 0.0];
        let n = normalize(&v);
        assert_eq!(n, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn euclidean_same_vector() {
        let v = vec![1.0, 2.0, 3.0];
        let dist = euclidean_distance(&v, &v);
        assert!(dist.abs() < 0.001);
    }

    #[test]
    fn euclidean_different_vectors() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 0.001);
    }

    #[test]
    fn dot_product_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dot = dot_product(&a, &b);
        assert!((dot - 32.0).abs() < 0.001);
    }

    #[test]
    fn vec_add_basic() {
        let a = vec![1.0, 2.0];
        let b = vec![3.0, 4.0];
        let c = vec_add(&a, &b);
        assert_eq!(c, vec![4.0, 6.0]);
    }

    #[test]
    fn vec_scale_basic() {
        let v = vec![1.0, 2.0, 3.0];
        let s = vec_scale(&v, 2.0);
        assert_eq!(s, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn batch_cosine_similarity_basic() {
        let query = vec![1.0, 0.0, 0.0];
        let vectors = vec![
            vec![1.0, 0.0, 0.0], // identical
            vec![0.0, 1.0, 0.0], // orthogonal
            vec![0.7, 0.7, 0.0], // similar
        ];
        let scores = batch_cosine_similarity(&query, &vectors);
        assert_eq!(scores.len(), 3);
        assert!((scores[0] - 1.0).abs() < 0.001);
        assert!(scores[1].abs() < 0.001);
        assert!(scores[2] > 0.5);
    }

    #[test]
    fn batch_cosine_similarity_empty() {
        let query = vec![1.0, 0.0, 0.0];
        let vectors: Vec<Vec<f32>> = vec![];
        let scores = batch_cosine_similarity(&query, &vectors);
        assert!(scores.is_empty());
    }

    #[test]
    fn knn_search_basic() {
        use std::collections::HashMap;

        let query = vec![1.0, 0.0, 0.0];
        let mut vectors = HashMap::new();
        vectors.insert("a".to_string(), vec![1.0, 0.0, 0.0]); // sim = 1.0
        vectors.insert("b".to_string(), vec![0.0, 1.0, 0.0]); // sim = 0.0
        vectors.insert("c".to_string(), vec![0.7, 0.7, 0.0]); // sim ~= 0.707

        let results = knn_search(&query, &vectors, 2, 0.5);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "a");
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[test]
    fn knn_search_threshold() {
        use std::collections::HashMap;

        let query = vec![1.0, 0.0, 0.0];
        let mut vectors = HashMap::new();
        vectors.insert("a".to_string(), vec![1.0, 0.0, 0.0]);
        vectors.insert("b".to_string(), vec![0.0, 1.0, 0.0]);

        let results = knn_search(&query, &vectors, 10, 0.9);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "a");
    }

    #[test]
    fn knn_search_indexed_basic() {
        let query = vec![1.0, 0.0, 0.0];
        let vectors = vec![
            ("a".to_string(), vec![1.0, 0.0, 0.0]),
            ("b".to_string(), vec![0.0, 1.0, 0.0]),
            ("c".to_string(), vec![0.7, 0.7, 0.0]),
        ];

        let results = knn_search_indexed(&query, &vectors, 3, 0.0);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "a");
    }
}
