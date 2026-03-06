/**
 * Performance benchmarks for TypeScript fallback implementations.
 *
 * Run with: cd packages/core && bun run benchmarks/performance.ts
 *
 * This benchmarks the pure TypeScript implementations to establish
 * a baseline for comparison with Rust native bindings.
 */

import { Bench } from 'tinybench'
import {
  chunkTextFallback,
  cosineSimilarityFallback,
  hybridMergeResultsFallback,
  normalizeVectorFallback,
  vectorDistanceFallback,
  type VectorResult,
} from '../src/memory.js'

// ============================================================================
// Test Data Generation
// ============================================================================

function generateVector(size: number): number[] {
  return Array.from({ length: size }, (_, i) => Math.sin(i / size))
}

function generateMarkdownDocument(sections: number): string {
  let doc = '# Document Title\n\nIntroduction paragraph with some content.\n\n'

  for (let i = 0; i < sections; i++) {
    doc += `## Section ${i + 1}\n\n`
    doc += 'This is a paragraph of text that contains some content. '
    doc += 'It has multiple sentences to simulate real documentation. '
    doc += `This is section number ${i + 1}.\n\n`

    // Add code block occasionally
    if (i % 5 === 0) {
      doc += '```rust\nfn example() -> u32 {\n    42\n}\n```\n\n'
    }

    // Add list occasionally
    if (i % 3 === 0) {
      doc += '- Item one\n- Item two\n- Item three\n\n'
    }
  }

  return doc
}

function generateVectorResults(size: number): VectorResult[] {
  return Array.from({ length: size }, (_, i) => ({
    id: `doc_${i}`,
    score: 0.95 - i * 0.0001,
  }))
}

function generateKeywordResults(size: number): VectorResult[] {
  return Array.from({ length: size }, (_, i) => ({
    id: i % 2 === 0 ? `doc_${i}` : `kw_doc_${i}`,
    score: 15.0 - i * 0.01,
  }))
}

// ============================================================================
// Benchmarks
// ============================================================================

async function runBenchmarks() {
  console.log('🚀 TypeScript Performance Benchmarks\n')
  console.log('=' .repeat(70))

  // ----- Cosine Similarity Benchmarks -----
  console.log('\n📊 Cosine Similarity Benchmarks\n')

  const cosineBench = new Bench({ time: 1000 })

  for (const size of [128, 512, 1024, 1536, 4096]) {
    const a = generateVector(size)
    const b = Array.from({ length: size }, (_, i) => Math.cos(i / size))

    cosineBench.add(`cosine_similarity_dim_${size}`, () => {
      cosineSimilarityFallback(a, b)
    })
  }

  await cosineBench.warmup()
  await cosineBench.run()
  console.table(cosineBench.table())

  // ----- Chunk Markdown Benchmarks -----
  console.log('\n📊 Chunk Markdown Benchmarks\n')

  const chunkBench = new Bench({ time: 1000 })

  const smallDoc = '# Title\n\nShort content paragraph here.\n\n## Section\n\nMore text.'
  const mediumDoc = generateMarkdownDocument(50)  // ~10KB
  const largeDoc = generateMarkdownDocument(500)  // ~100KB

  console.log(`Document sizes: small=${smallDoc.length}b, medium=${mediumDoc.length}b, large=${largeDoc.length}b`)

  chunkBench.add('chunk_small_100b', () => chunkTextFallback(smallDoc, 512))
  chunkBench.add('chunk_medium_10kb', () => chunkTextFallback(mediumDoc, 512))
  chunkBench.add('chunk_large_100kb', () => chunkTextFallback(largeDoc, 512))
  chunkBench.add('chunk_medium_256tok', () => chunkTextFallback(mediumDoc, 256))
  chunkBench.add('chunk_medium_1024tok', () => chunkTextFallback(mediumDoc, 1024))

  await chunkBench.warmup()
  await chunkBench.run()
  console.table(chunkBench.table())

  // ----- Hybrid Merge Benchmarks -----
  console.log('\n📊 Hybrid Merge Benchmarks\n')

  const hybridBench = new Bench({ time: 1000 })

  for (const size of [50, 100, 500, 1000, 5000]) {
    const vecResults = generateVectorResults(size)
    const kwResults = generateKeywordResults(Math.floor(size / 2))

    hybridBench.add(`hybrid_merge_results_${size}`, () => {
      hybridMergeResultsFallback(vecResults, kwResults, 0.7, 0.3, 50)
    })
  }

  await hybridBench.warmup()
  await hybridBench.run()
  console.table(hybridBench.table())

  // ----- Vector Operations Benchmarks -----
  console.log('\n📊 Vector Operations Benchmarks\n')

  const vectorBench = new Bench({ time: 1000 })

  const vec1024 = generateVector(1024)
  const vec1536 = generateVector(1536)

  vectorBench.add('normalize_1024', () => normalizeVectorFallback(vec1024))
  vectorBench.add('normalize_1536', () => normalizeVectorFallback(vec1536))
  vectorBench.add('distance_1024', () => vectorDistanceFallback(vec1024, vec1024))
  vectorBench.add('distance_1536', () => vectorDistanceFallback(vec1536, vec1536))

  await vectorBench.warmup()
  await vectorBench.run()
  console.table(vectorBench.table())

  // ----- Edge Cases -----
  console.log('\n📊 Edge Case Benchmarks\n')

  const edgeBench = new Bench({ time: 500 })

  const emptyVec: number[] = []
  const identicalVec = Array.from({ length: 1024 }, (_, i) => i)
  const longLine = 'word '.repeat(10000)

  edgeBench.add('cosine_empty', () => cosineSimilarityFallback(emptyVec, emptyVec))
  edgeBench.add('cosine_identical_1024', () => cosineSimilarityFallback(identicalVec, identicalVec))
  edgeBench.add('chunk_empty', () => chunkTextFallback('', 512))
  edgeBench.add('chunk_long_line_50k', () => chunkTextFallback(longLine, 512))

  await edgeBench.warmup()
  await edgeBench.run()
  console.table(edgeBench.table())

  // ----- Summary -----
  console.log('\n' + '=' .repeat(70))
  console.log('✅ TypeScript benchmarks complete!')
  console.log('\nKey metrics to compare with Rust:')
  console.log('  - cosine_similarity_dim_1024')
  console.log('  - chunk_medium_10kb')
  console.log('  - hybrid_merge_results_1000')
}

runBenchmarks().catch(console.error)
