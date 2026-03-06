//! Performance benchmarks for zero-core memory operations.
//!
//! Run with: cargo bench -p zero-core
//!
//! SIMD benchmarks require the `simd` feature (enabled by default).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use zero_core::memory::{
    chunk_markdown, cosine_similarity, dot_product, euclidean_distance, hybrid_merge, normalize,
};

/// Benchmark cosine similarity across different vector dimensions.
fn bench_cosine_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_similarity");

    for size in [128, 512, 1024, 1536, 4096, 16384] {
        group.throughput(Throughput::Elements(size as u64));

        // Generate test vectors
        let a: Vec<f32> = (0..size).map(|i| (i as f32 / size as f32).sin()).collect();
        let b: Vec<f32> = (0..size).map(|i| (i as f32 / size as f32).cos()).collect();

        group.bench_with_input(BenchmarkId::new("dim", size), &size, |bench, _| {
            bench.iter(|| cosine_similarity(&a, &b))
        });
    }
    group.finish();
}

/// Benchmark euclidean distance across different vector dimensions.
fn bench_euclidean_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("euclidean_distance");

    for size in [128, 512, 1024, 1536, 4096] {
        group.throughput(Throughput::Elements(size as u64));

        let a: Vec<f32> = (0..size).map(|i| (i as f32 / size as f32).sin()).collect();
        let b: Vec<f32> = (0..size).map(|i| (i as f32 / size as f32).cos()).collect();

        group.bench_with_input(BenchmarkId::new("dim", size), &size, |bench, _| {
            bench.iter(|| euclidean_distance(&a, &b))
        });
    }
    group.finish();
}

/// Benchmark dot product across different vector dimensions.
fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("dot_product");

    for size in [128, 512, 1024, 1536, 4096] {
        group.throughput(Throughput::Elements(size as u64));

        let a: Vec<f32> = (0..size).map(|i| (i as f32 / size as f32).sin()).collect();
        let b: Vec<f32> = (0..size).map(|i| (i as f32 / size as f32).cos()).collect();

        group.bench_with_input(BenchmarkId::new("dim", size), &size, |bench, _| {
            bench.iter(|| dot_product(&a, &b))
        });
    }
    group.finish();
}

/// Benchmark vector normalization across different dimensions.
fn bench_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize");

    for size in [128, 512, 1024, 1536, 4096] {
        group.throughput(Throughput::Elements(size as u64));

        let v: Vec<f32> = (0..size).map(|i| (i as f32 / size as f32).sin()).collect();

        group.bench_with_input(BenchmarkId::new("dim", size), &size, |bench, _| {
            bench.iter(|| normalize(&v))
        });
    }
    group.finish()
}

/// Benchmark markdown chunking with varying document sizes.
fn bench_chunk_markdown(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_markdown");

    // Small document (~100 chars)
    let small = "# Title\n\nShort content paragraph here.\n\n## Section\n\nMore text.";

    // Medium document (~10KB) - typical code file
    let medium = generate_markdown_document(50);

    // Large document (~100KB) - large documentation file
    let large = generate_markdown_document(500);

    group.bench_function("small_100b", |b| b.iter(|| chunk_markdown(small, 512)));
    group.bench_function("medium_10kb", |b| b.iter(|| chunk_markdown(&medium, 512)));
    group.bench_function("large_100kb", |b| b.iter(|| chunk_markdown(&large, 512)));

    // Also test with different chunk sizes
    group.bench_function("medium_256tok", |b| b.iter(|| chunk_markdown(&medium, 256)));
    group.bench_function("medium_1024tok", |b| b.iter(|| chunk_markdown(&medium, 1024)));

    group.finish();
}

/// Generate a synthetic markdown document with n sections.
fn generate_markdown_document(sections: usize) -> String {
    let mut doc = String::with_capacity(sections * 200);
    doc.push_str("# Document Title\n\n");
    doc.push_str("Introduction paragraph with some content.\n\n");

    for i in 0..sections {
        doc.push_str(&format!("## Section {}\n\n", i + 1));
        doc.push_str("This is a paragraph of text that contains some content. ");
        doc.push_str("It has multiple sentences to simulate real documentation. ");
        doc.push_str(&format!("This is section number {}.\n\n", i + 1));

        // Add code block occasionally
        if i % 5 == 0 {
            doc.push_str("```rust\nfn example() -> u32 {\n    42\n}\n```\n\n");
        }

        // Add list occasionally
        if i % 3 == 0 {
            doc.push_str("- Item one\n- Item two\n- Item three\n\n");
        }
    }

    doc
}

/// Benchmark hybrid merge with varying result set sizes.
fn bench_hybrid_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_merge");

    for size in [50, 100, 500, 1000, 5000, 10000] {
        // Generate vector results (simulating embeddings search)
        let vec_results: Vec<(String, f32)> = (0..size)
            .map(|i| (format!("doc_{i}"), 0.95 - (i as f32 * 0.0001)))
            .collect();

        // Generate keyword results (simulating BM25 search) - typically fewer
        let kw_results: Vec<(String, f32)> = (0..size / 2)
            .map(|i| {
                // Some overlap with vector results, some unique
                let id = if i % 2 == 0 {
                    format!("doc_{i}")
                } else {
                    format!("kw_doc_{i}")
                };
                (id, 15.0 - (i as f32 * 0.01))
            })
            .collect();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("results", size), &size, |bench, _| {
            bench.iter(|| hybrid_merge(&vec_results, &kw_results, 0.7, 0.3, 50))
        });
    }

    group.finish();
}

/// Benchmark edge cases and specific scenarios.
fn bench_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_cases");

    // Empty inputs
    let empty: Vec<f32> = vec![];
    group.bench_function("cosine_empty", |b| b.iter(|| cosine_similarity(&empty, &empty)));

    // Identical vectors (should be ~1.0)
    let identical: Vec<f32> = (0..1024).map(|i| i as f32).collect();
    group.bench_function("cosine_identical_1024", |b| {
        b.iter(|| cosine_similarity(&identical, &identical))
    });

    // Orthogonal vectors
    let ortho_a: Vec<f32> = (0..1024).map(|i| if i < 512 { 1.0 } else { 0.0 }).collect();
    let ortho_b: Vec<f32> = (0..1024).map(|i| if i >= 512 { 1.0 } else { 0.0 }).collect();
    group.bench_function("cosine_orthogonal_1024", |b| {
        b.iter(|| cosine_similarity(&ortho_a, &ortho_b))
    });

    // Empty markdown
    group.bench_function("chunk_empty", |b| b.iter(|| chunk_markdown("", 512)));

    // Single long line (stress test)
    let long_line = "word ".repeat(10000);
    group.bench_function("chunk_long_line_50k", |b| b.iter(|| chunk_markdown(&long_line, 512)));

    // Deep heading nesting
    let deep_headings = (1..50)
        .map(|i| format!("## Section {}\nContent for section {}.\n\n", i, i))
        .collect::<String>();
    group.bench_function("chunk_deep_headings", |b| {
        b.iter(|| chunk_markdown(&deep_headings, 512))
    });

    // Hybrid merge with no overlap
    let vec_only: Vec<(String, f32)> = (0..100).map(|i| (format!("vec_{i}"), 0.9)).collect();
    let kw_only: Vec<(String, f32)> = (0..100).map(|i| (format!("kw_{i}"), 10.0)).collect();
    group.bench_function("hybrid_no_overlap_100", |b| {
        b.iter(|| hybrid_merge(&vec_only, &kw_only, 0.7, 0.3, 50))
    });

    // Hybrid merge with full overlap
    let ids: Vec<String> = (0..100).map(|i| format!("doc_{i}")).collect();
    let vec_full: Vec<(String, f32)> = ids.iter().map(|id| (id.clone(), 0.9)).collect();
    let kw_full: Vec<(String, f32)> = ids.iter().map(|id| (id.clone(), 10.0)).collect();
    group.bench_function("hybrid_full_overlap_100", |b| {
        b.iter(|| hybrid_merge(&vec_full, &kw_full, 0.7, 0.3, 50))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cosine_similarity,
    bench_euclidean_distance,
    bench_dot_product,
    bench_normalize,
    bench_chunk_markdown,
    bench_hybrid_merge,
    bench_edge_cases
);
criterion_main!(benches);
