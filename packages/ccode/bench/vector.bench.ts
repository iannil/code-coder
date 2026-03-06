/**
 * Vector operations benchmark
 *
 * Compares native SIMD-accelerated Rust implementation vs TypeScript fallback
 */

import { cosineSimilarity, normalizeVector } from "@codecoder-ai/core"

// Generate random vectors of various dimensions
function generateVector(dimension: number): number[] {
  return Array.from({ length: dimension }, () => Math.random())
}

// TypeScript fallback implementations for comparison
function cosineSimilarityTS(a: number[], b: number[]): number {
  if (a.length !== b.length || a.length === 0) return 0

  let dot = 0
  let normA = 0
  let normB = 0

  for (let i = 0; i < a.length; i++) {
    const ai = a[i]!
    const bi = b[i]!
    dot += ai * bi
    normA += ai * ai
    normB += bi * bi
  }

  const denom = Math.sqrt(normA) * Math.sqrt(normB)
  if (denom < 1e-10) return 0

  return Math.max(0, Math.min(1, dot / denom))
}

function normalizeVectorTS(v: number[]): number[] {
  const norm = Math.sqrt(v.reduce((sum, x) => sum + x * x, 0))
  if (norm < 1e-10) return v.map(() => 0)
  return v.map((x) => x / norm)
}

// Benchmark helper
async function benchmark(name: string, fn: () => void, iterations: number = 10000): Promise<number> {
  // Warmup
  for (let i = 0; i < 100; i++) fn()

  const start = performance.now()
  for (let i = 0; i < iterations; i++) {
    fn()
  }
  const end = performance.now()

  const avgMs = (end - start) / iterations
  console.log(`${name}: ${avgMs.toFixed(6)} ms/op (${iterations} iterations)`)
  return avgMs
}

export async function runVectorBenchmarks() {
  console.log("=== Vector Operations Benchmark ===\n")

  const dimensions = [128, 512, 1536, 4096] // Common embedding sizes

  for (const dim of dimensions) {
    console.log(`\n--- Dimension: ${dim} ---`)

    const a = generateVector(dim)
    const b = generateVector(dim)

    // Cosine similarity
    const tsCosineSim = await benchmark(`  TypeScript cosine`, () => cosineSimilarityTS(a, b))
    const nativeCosineSim = await benchmark(`  Native cosine    `, () => cosineSimilarity!(a, b))
    const cosineSpeedup = tsCosineSim / nativeCosineSim
    console.log(`  → Speedup: ${cosineSpeedup.toFixed(2)}x`)

    // Normalize
    const tsNorm = await benchmark(`  TypeScript norm  `, () => normalizeVectorTS(a))
    const nativeNorm = await benchmark(`  Native norm      `, () => normalizeVector!(a))
    const normSpeedup = tsNorm / nativeNorm
    console.log(`  → Speedup: ${normSpeedup.toFixed(2)}x`)
  }

  console.log("\n=== Benchmark Complete ===")
}

// Run if executed directly
if (import.meta.main) {
  runVectorBenchmarks()
}
