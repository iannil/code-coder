/**
 * NAPI Overhead Analysis Benchmarks
 *
 * Run with: cd packages/core && bun run benchmarks/napi-overhead.ts
 *
 * This benchmark compares:
 * 1. Pure TypeScript fallback implementations
 * 2. Rust implementations via NAPI bindings
 *
 * To run this benchmark:
 * 1. Build native bindings: cd services/zero-core && napi build --release
 * 2. Copy bindings: cp ../../release/*.node ../packages/core/
 * 3. Run benchmark: bun run benchmarks/napi-overhead.ts
 */

import { Bench } from 'tinybench'

// Import fallback (always available)
import * as fallback from '../src/memory.js'

// Try to import native bindings
type NativeBindings = typeof fallback

let native: NativeBindings | null = null
let isNativeAvailable = false

try {
  // Dynamic import to avoid build-time errors
  const bindings = await import('../binding.js')
  native = bindings as unknown as NativeBindings
  isNativeAvailable = true
  console.log('✅ Native bindings loaded successfully\n')
} catch (e) {
  console.log('⚠️  Native bindings not available - running fallback-only benchmarks')
  console.log('   To enable native benchmarks:')
  console.log('   1. cd services/zero-core && napi build --release')
  console.log('   2. cp target/release/libcodecoder_core.* ../../packages/core/binding.*')
  console.log('')
}

// ============================================================================
// Test Data
// ============================================================================

// Vector sizes that match common embedding dimensions
const VECTOR_SIZES = [512, 1024, 1536, 3072] as const

// Pre-generate test vectors for consistent benchmarks
const vectors = Object.fromEntries(
  VECTOR_SIZES.map(size => [
    size,
    {
      a: Array.from({ length: size }, (_, i) => Math.sin(i / size)),
      b: Array.from({ length: size }, (_, i) => Math.cos(i / size)),
    }
  ])
) as Record<typeof VECTOR_SIZES[number], { a: number[], b: number[] }>

// Document sizes
const smallDoc = '# Title\n\nShort content.\n\n## Section\n\nMore text.'
const mediumDoc = generateMarkdownDocument(50)  // ~10KB
const largeDoc = generateMarkdownDocument(200)  // ~40KB

function generateMarkdownDocument(sections: number): string {
  let doc = '# Document Title\n\nIntro paragraph.\n\n'
  for (let i = 0; i < sections; i++) {
    doc += `## Section ${i + 1}\n\nContent paragraph ${i}.\n\n`
    if (i % 3 === 0) doc += '- Item 1\n- Item 2\n\n'
  }
  return doc
}

// Hybrid merge test data
const vecResults100 = Array.from({ length: 100 }, (_, i) => ({ id: `doc_${i}`, score: 0.95 - i * 0.001 }))
const kwResults50 = Array.from({ length: 50 }, (_, i) => ({ id: `doc_${i * 2}`, score: 15.0 - i * 0.1 }))

const vecResults1000 = Array.from({ length: 1000 }, (_, i) => ({ id: `doc_${i}`, score: 0.95 - i * 0.0001 }))
const kwResults500 = Array.from({ length: 500 }, (_, i) => ({ id: `doc_${i * 2}`, score: 15.0 - i * 0.01 }))

// ============================================================================
// Benchmark Execution
// ============================================================================

async function runBenchmarks() {
  console.log('🔬 NAPI Overhead Analysis\n')
  console.log('=' .repeat(70))

  // ----- Cosine Similarity -----
  console.log('\n📊 Cosine Similarity: TypeScript vs Rust (via NAPI)\n')

  const cosineBench = new Bench({ time: 2000, iterations: 1000 })

  for (const size of VECTOR_SIZES) {
    const { a, b } = vectors[size]

    cosineBench.add(`ts_cosine_${size}`, () => {
      fallback.cosineSimilarityFallback(a, b)
    })

    if (isNativeAvailable && native?.cosineSimilarity) {
      cosineBench.add(`rust_cosine_${size}`, () => {
        native!.cosineSimilarity(a, b)
      })
    }
  }

  await cosineBench.warmup()
  await cosineBench.run()
  console.table(cosineBench.table())

  if (isNativeAvailable) {
    printSpeedupAnalysis('cosine', cosineBench, VECTOR_SIZES)
  }

  // ----- Chunk Markdown -----
  console.log('\n📊 Chunk Markdown: TypeScript vs Rust (via NAPI)\n')

  const chunkBench = new Bench({ time: 2000 })

  chunkBench.add('ts_chunk_small', () => fallback.chunkTextFallback(smallDoc, 512))
  chunkBench.add('ts_chunk_medium', () => fallback.chunkTextFallback(mediumDoc, 512))
  chunkBench.add('ts_chunk_large', () => fallback.chunkTextFallback(largeDoc, 512))

  if (isNativeAvailable && native?.chunkText) {
    chunkBench.add('rust_chunk_small', () => native!.chunkText(smallDoc, 512))
    chunkBench.add('rust_chunk_medium', () => native!.chunkText(mediumDoc, 512))
    chunkBench.add('rust_chunk_large', () => native!.chunkText(largeDoc, 512))
  }

  await chunkBench.warmup()
  await chunkBench.run()
  console.table(chunkBench.table())

  if (isNativeAvailable) {
    printChunkSpeedup(chunkBench)
  }

  // ----- Hybrid Merge -----
  console.log('\n📊 Hybrid Merge: TypeScript vs Rust (via NAPI)\n')

  const hybridBench = new Bench({ time: 2000 })

  hybridBench.add('ts_hybrid_100', () => {
    fallback.hybridMergeResultsFallback(vecResults100, kwResults50, 0.7, 0.3, 50)
  })
  hybridBench.add('ts_hybrid_1000', () => {
    fallback.hybridMergeResultsFallback(vecResults1000, kwResults500, 0.7, 0.3, 50)
  })

  if (isNativeAvailable && native?.hybridMergeResults) {
    hybridBench.add('rust_hybrid_100', () => {
      native!.hybridMergeResults(vecResults100, kwResults50, 0.7, 0.3, 50)
    })
    hybridBench.add('rust_hybrid_1000', () => {
      native!.hybridMergeResults(vecResults1000, kwResults500, 0.7, 0.3, 50)
    })
  }

  await hybridBench.warmup()
  await hybridBench.run()
  console.table(hybridBench.table())

  if (isNativeAvailable) {
    printHybridSpeedup(hybridBench)
  }

  // ----- NAPI Call Overhead -----
  if (isNativeAvailable && native?.cosineSimilarity) {
    console.log('\n📊 NAPI Call Overhead Analysis\n')
    console.log('Testing minimum FFI overhead with small inputs...\n')

    const overheadBench = new Bench({ time: 2000, iterations: 10000 })

    // Tiny vectors to isolate FFI overhead
    const tiny = [1.0, 0.0]

    overheadBench.add('ts_cosine_2dim', () => fallback.cosineSimilarityFallback(tiny, tiny))
    overheadBench.add('rust_cosine_2dim', () => native!.cosineSimilarity(tiny, tiny))

    // Empty inputs (should be fastest)
    const empty: number[] = []
    overheadBench.add('ts_cosine_empty', () => fallback.cosineSimilarityFallback(empty, empty))
    overheadBench.add('rust_cosine_empty', () => native!.cosineSimilarity(empty, empty))

    await overheadBench.warmup()
    await overheadBench.run()
    console.table(overheadBench.table())

    // Calculate overhead
    const tsTime = overheadBench.tasks.find(t => t.name === 'ts_cosine_2dim')?.result?.mean ?? 0
    const rustTime = overheadBench.tasks.find(t => t.name === 'rust_cosine_2dim')?.result?.mean ?? 0
    const overheadNs = (rustTime - tsTime) * 1000000

    console.log('\n💡 NAPI Overhead Estimate:')
    if (overheadNs > 0) {
      console.log(`   For tiny operations, NAPI adds ~${overheadNs.toFixed(0)}ns overhead`)
      console.log('   This overhead becomes negligible for operations >10μs')
    } else {
      console.log('   Rust is faster even for tiny operations!')
    }
  }

  // ----- Summary -----
  console.log('\n' + '=' .repeat(70))
  console.log('✅ NAPI overhead analysis complete!')

  if (!isNativeAvailable) {
    console.log('\n⚠️  Run with native bindings for full comparison:')
    console.log('   cd services/zero-core && cargo build --release --features napi-bindings')
    console.log('   napi build --release')
    console.log('   cp target/release/libcodecoder_core.dylib ../../packages/core/codecoder-core.darwin-arm64.node')
  }
}

// ============================================================================
// Analysis Helpers
// ============================================================================

function printSpeedupAnalysis(name: string, bench: Bench, sizes: readonly number[]) {
  console.log(`\n💡 ${name} speedup analysis:`)

  for (const size of sizes) {
    const tsTask = bench.tasks.find(t => t.name === `ts_${name}_${size}`)
    const rustTask = bench.tasks.find(t => t.name === `rust_${name}_${size}`)

    if (tsTask?.result && rustTask?.result) {
      const speedup = tsTask.result.mean / rustTask.result.mean
      const tsUs = (tsTask.result.mean * 1000).toFixed(2)
      const rustUs = (rustTask.result.mean * 1000).toFixed(2)
      console.log(`   dim=${size}: TS=${tsUs}μs, Rust=${rustUs}μs → ${speedup.toFixed(1)}x faster`)
    }
  }
}

function printChunkSpeedup(bench: Bench) {
  console.log('\n💡 Chunk markdown speedup analysis:')

  for (const size of ['small', 'medium', 'large']) {
    const tsTask = bench.tasks.find(t => t.name === `ts_chunk_${size}`)
    const rustTask = bench.tasks.find(t => t.name === `rust_chunk_${size}`)

    if (tsTask?.result && rustTask?.result) {
      const speedup = tsTask.result.mean / rustTask.result.mean
      const tsUs = (tsTask.result.mean * 1000).toFixed(2)
      const rustUs = (rustTask.result.mean * 1000).toFixed(2)
      console.log(`   ${size}: TS=${tsUs}μs, Rust=${rustUs}μs → ${speedup.toFixed(1)}x faster`)
    }
  }
}

function printHybridSpeedup(bench: Bench) {
  console.log('\n💡 Hybrid merge speedup analysis:')

  for (const size of ['100', '1000']) {
    const tsTask = bench.tasks.find(t => t.name === `ts_hybrid_${size}`)
    const rustTask = bench.tasks.find(t => t.name === `rust_hybrid_${size}`)

    if (tsTask?.result && rustTask?.result) {
      const speedup = tsTask.result.mean / rustTask.result.mean
      const tsUs = (tsTask.result.mean * 1000).toFixed(2)
      const rustUs = (rustTask.result.mean * 1000).toFixed(2)
      console.log(`   n=${size}: TS=${tsUs}μs, Rust=${rustUs}μs → ${speedup.toFixed(1)}x faster`)
    }
  }
}

runBenchmarks().catch(console.error)
