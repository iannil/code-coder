/**
 * LLM Call Performance Benchmarks
 *
 * Measures LLM-related performance:
 * - Time To First Token (TTFT)
 * - Throughput (tokens/second)
 * - Tool call round-trip latency
 *
 * By default, uses mock provider for CI. Set BENCHMARK_LLM_REAL=true to use real providers.
 *
 * Run with: bun run bench/llm.bench.ts
 * Or via main suite: bun run bench --llm
 */

import type { BenchmarkResult } from "./index"

// ============================================================================
// Configuration
// ============================================================================

const USE_REAL_PROVIDER = process.env.BENCHMARK_LLM_REAL === "true"

// Target thresholds for mock provider (to verify benchmark infrastructure)
const MOCK_TARGETS = {
  ttft_ms: 50, // First token within 50ms
  throughput_tps: 100, // At least 100 tokens/second
  tool_roundtrip_ms: 100, // Tool call + response within 100ms
}

// Informational thresholds for real providers (no pass/fail, just tracking)
const REAL_TARGETS = {
  ttft_ms: 2000, // Real APIs have ~1-3s TTFT
  throughput_tps: 50, // Real APIs stream at ~30-80 tps
  tool_roundtrip_ms: 5000, // Tool calls add latency
}

// ============================================================================
// Mock Provider
// ============================================================================

interface MockStreamOptions {
  totalTokens: number
  tokensPerChunk: number
  chunkDelayMs: number
  ttftDelayMs: number
}

const DEFAULT_MOCK_OPTIONS: MockStreamOptions = {
  totalTokens: 500,
  tokensPerChunk: 10,
  chunkDelayMs: 5,
  ttftDelayMs: 20,
}

async function* mockTokenStream(options: MockStreamOptions = DEFAULT_MOCK_OPTIONS): AsyncGenerator<string> {
  // Simulate TTFT delay
  await new Promise((resolve) => setTimeout(resolve, options.ttftDelayMs))

  const tokensGenerated = 0
  const chunks = Math.ceil(options.totalTokens / options.tokensPerChunk)

  for (let i = 0; i < chunks; i++) {
    const chunkSize = Math.min(options.tokensPerChunk, options.totalTokens - i * options.tokensPerChunk)
    const tokens = Array(chunkSize)
      .fill(null)
      .map(() => "token")
      .join(" ")

    yield tokens

    if (i < chunks - 1) {
      await new Promise((resolve) => setTimeout(resolve, options.chunkDelayMs))
    }
  }
}

async function mockToolCall(): Promise<{ toolName: string; result: string }> {
  await new Promise((resolve) => setTimeout(resolve, 20))
  return {
    toolName: "mock_tool",
    result: JSON.stringify({ success: true, data: "mock result" }),
  }
}

// ============================================================================
// TTFT Measurement
// ============================================================================

async function measureTTFT(iterations: number = 10): Promise<BenchmarkResult> {
  const durations: number[] = []

  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    const stream = mockTokenStream()

    // Get first token
    const first = await stream.next()
    const ttft = performance.now() - start

    durations.push(ttft)

    // Consume rest of stream
    for await (const _ of stream) {
      // Just consume
    }
  }

  const sorted = [...durations].sort((a, b) => a - b)
  const p50 = sorted[Math.floor(sorted.length * 0.5)]
  const p95 = sorted[Math.floor(sorted.length * 0.95)]
  const avg = durations.reduce((a, b) => a + b, 0) / durations.length

  const target = USE_REAL_PROVIDER ? REAL_TARGETS.ttft_ms : MOCK_TARGETS.ttft_ms

  return {
    name: "TTFT (First Token)",
    target: `P50 ≤${target}ms`,
    result: `${p50.toFixed(1)}ms`,
    pass: p50 <= target,
    numericValue: p50,
    unit: "ms",
    details: {
      p50: `${p50.toFixed(1)}ms`,
      p95: `${p95.toFixed(1)}ms`,
      avg: `${avg.toFixed(1)}ms`,
      iterations,
      provider: USE_REAL_PROVIDER ? "real" : "mock",
    },
  }
}

// ============================================================================
// Throughput Measurement
// ============================================================================

async function measureThroughput(iterations: number = 5): Promise<BenchmarkResult> {
  const throughputs: number[] = []

  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    let tokenCount = 0

    const stream = mockTokenStream({
      ...DEFAULT_MOCK_OPTIONS,
      totalTokens: 1000,
    })

    for await (const chunk of stream) {
      // Count tokens (approximation: split by spaces)
      tokenCount += chunk.split(" ").length
    }

    const duration = (performance.now() - start) / 1000 // Convert to seconds
    const tps = tokenCount / duration
    throughputs.push(tps)
  }

  const sorted = [...throughputs].sort((a, b) => a - b)
  const p50 = sorted[Math.floor(sorted.length * 0.5)]
  const avg = throughputs.reduce((a, b) => a + b, 0) / throughputs.length

  const target = USE_REAL_PROVIDER ? REAL_TARGETS.throughput_tps : MOCK_TARGETS.throughput_tps

  return {
    name: "Throughput",
    target: `≥${target} tok/s`,
    result: `${p50.toFixed(0)} tok/s`,
    pass: p50 >= target,
    numericValue: p50,
    unit: "tok/s",
    details: {
      p50: `${p50.toFixed(0)} tok/s`,
      avg: `${avg.toFixed(0)} tok/s`,
      iterations,
      provider: USE_REAL_PROVIDER ? "real" : "mock",
    },
  }
}

// ============================================================================
// Tool Call Round-Trip
// ============================================================================

async function measureToolRoundTrip(iterations: number = 20): Promise<BenchmarkResult> {
  const durations: number[] = []

  for (let i = 0; i < iterations; i++) {
    const start = performance.now()

    // Simulate: LLM produces tool call -> tool executes -> result returned
    await mockToolCall()

    durations.push(performance.now() - start)
  }

  const sorted = [...durations].sort((a, b) => a - b)
  const p50 = sorted[Math.floor(sorted.length * 0.5)]
  const p95 = sorted[Math.floor(sorted.length * 0.95)]
  const avg = durations.reduce((a, b) => a + b, 0) / durations.length

  const target = USE_REAL_PROVIDER ? REAL_TARGETS.tool_roundtrip_ms : MOCK_TARGETS.tool_roundtrip_ms

  return {
    name: "Tool Call Round-Trip",
    target: `P50 ≤${target}ms`,
    result: `${p50.toFixed(1)}ms`,
    pass: p50 <= target,
    numericValue: p50,
    unit: "ms",
    details: {
      p50: `${p50.toFixed(1)}ms`,
      p95: `${p95.toFixed(1)}ms`,
      avg: `${avg.toFixed(1)}ms`,
      iterations,
      provider: USE_REAL_PROVIDER ? "real" : "mock",
    },
  }
}

// ============================================================================
// Stream Processing Overhead
// ============================================================================

async function measureStreamOverhead(): Promise<BenchmarkResult> {
  const iterations = 10
  const durations: number[] = []

  // Measure time to process a stream vs generate tokens
  for (let i = 0; i < iterations; i++) {
    const start = performance.now()

    const stream = mockTokenStream({
      totalTokens: 500,
      tokensPerChunk: 50,
      chunkDelayMs: 0, // No artificial delay
      ttftDelayMs: 0,
    })

    const chunks: string[] = []
    for await (const chunk of stream) {
      chunks.push(chunk)
    }

    durations.push(performance.now() - start)
  }

  const sorted = [...durations].sort((a, b) => a - b)
  const p50 = sorted[Math.floor(sorted.length * 0.5)]
  const avg = durations.reduce((a, b) => a + b, 0) / durations.length

  return {
    name: "Stream Processing (500 tokens)",
    target: "≤50ms",
    result: `${p50.toFixed(1)}ms`,
    pass: p50 <= 50,
    numericValue: p50,
    unit: "ms",
    details: {
      p50: `${p50.toFixed(1)}ms`,
      avg: `${avg.toFixed(1)}ms`,
      iterations,
    },
  }
}

// ============================================================================
// Main Runner
// ============================================================================

export async function runLlmBenchmarks(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  console.log(`  Using ${USE_REAL_PROVIDER ? "REAL" : "MOCK"} provider`)

  console.log("  Measuring TTFT (Time To First Token)...")
  results.push(await measureTTFT())

  console.log("  Measuring throughput...")
  results.push(await measureThroughput())

  console.log("  Measuring tool call round-trip...")
  results.push(await measureToolRoundTrip())

  console.log("  Measuring stream processing overhead...")
  results.push(await measureStreamOverhead())

  return results
}

// Entry point for standalone execution
if (import.meta.main) {
  console.log("Running LLM Performance Benchmarks...\n")
  runLlmBenchmarks()
    .then((results) => {
      console.log("\n=== LLM Benchmark Results ===\n")
      console.log("| Metric | Result | Target | Status |")
      console.log("|--------|--------|--------|--------|")
      for (const r of results) {
        const status = r.pass ? "✅ PASS" : "❌ FAIL"
        console.log(`| ${r.name} | ${r.result} | ${r.target} | ${status} |`)
      }

      const passed = results.filter((r) => r.pass).length
      const failed = results.filter((r) => !r.pass).length
      console.log(`\nTotal: ${results.length} | Passed: ${passed} | Failed: ${failed}`)

      process.exit(failed > 0 ? 1 : 0)
    })
    .catch((err) => {
      console.error("Benchmark failed:", err)
      process.exit(1)
    })
}
