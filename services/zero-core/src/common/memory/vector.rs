//! Vector operations — cosine similarity, normalization, hybrid merge.
//!
//! This module re-exports vector operations from `crate::memory::vector` to maintain
//! backward compatibility. The main implementation uses SIMD acceleration when available.
//!
//! ## Migration Note
//!
//! This module previously contained a duplicate non-SIMD implementation.
//! It now re-exports from `crate::memory::vector` to consolidate code and
//! ensure consistent behavior across the codebase.

// Re-export all vector operations from the main memory::vector module
pub use crate::memory::vector::{
    // Core functions
    bytes_to_vec,
    cosine_similarity,
    hybrid_merge,
    vec_to_bytes,
    // Types
    ScoredResult,
};

#[cfg(test)]
mod tests {
    use super::*;

    // Basic smoke tests to verify re-exports work correctly

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
    fn vec_bytes_roundtrip() {
        let original = vec![1.0_f32, -2.5, 3.14, 0.0];
        let bytes = vec_to_bytes(&original);
        let restored = bytes_to_vec(&bytes);
        assert_eq!(original, restored);
    }

    #[test]
    fn hybrid_merge_basic() {
        let vec_results = vec![("a".into(), 0.9), ("b".into(), 0.5)];
        let merged = hybrid_merge(&vec_results, &[], 0.7, 0.3, 10);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, "a");
        assert!(merged[0].final_score > merged[1].final_score);
    }
}
