/**
 * Tool Performance Benchmarks
 *
 * Measures execution time for core tool operations:
 * - Read: Reading files of various sizes
 * - Write: Writing files of various sizes
 * - Glob: Pattern matching with different complexity
 * - Grep: Content search with simple/regex/multiline patterns
 * - Bash: Command execution latency
 *
 * These benchmarks measure the raw performance of underlying operations,
 * not the full tool wrapper (which includes permission checks, etc.).
 */

import path from "path"
import fs from "fs/promises"
import type { BenchmarkResult } from "./index"
import { ensureFixtures, getFixturePath, FIXTURE_DIR } from "./fixture"
import { Ripgrep } from "../src/file/ripgrep"

// ============================================================================
// Configuration
// ============================================================================

const WARMUP_ITERATIONS = 3
const BENCHMARK_ITERATIONS = 10

// Target performance thresholds (P95)
const TARGETS = {
  read_1kb: 10, // ms
  read_100kb: 50,
  read_1mb: 200,
  read_10mb: 1000,
  write_1kb: 10,
  write_100kb: 50,
  write_1mb: 200,
  glob_simple: 100,
  glob_recursive: 500,
  glob_complex: 1000,
  grep_simple: 200,
  grep_regex: 500,
  bash_simple: 100,
  bash_pipeline: 500,
}

// ============================================================================
// Performance Utilities
// ============================================================================

interface LatencyStats {
  p50: number
  p95: number
  p99: number
  avg: number
  min: number
  max: number
}

function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0
  const index = Math.ceil((p / 100) * sorted.length) - 1
  return sorted[Math.max(0, index)]
}

function calculateStats(durations: number[]): LatencyStats {
  const sorted = [...durations].sort((a, b) => a - b)
  const sum = sorted.reduce((a, b) => a + b, 0)
  return {
    p50: percentile(sorted, 50),
    p95: percentile(sorted, 95),
    p99: percentile(sorted, 99),
    avg: sum / sorted.length,
    min: sorted[0] || 0,
    max: sorted[sorted.length - 1] || 0,
  }
}

async function benchmark<T>(
  name: string,
  fn: () => Promise<T>,
  iterations: number = BENCHMARK_ITERATIONS,
): Promise<{ stats: LatencyStats; results: T[] }> {
  // Warmup
  for (let i = 0; i < WARMUP_ITERATIONS; i++) {
    await fn()
  }

  const durations: number[] = []
  const results: T[] = []

  for (let i = 0; i < iterations; i++) {
    const start = performance.now()
    const result = await fn()
    durations.push(performance.now() - start)
    results.push(result)
  }

  return { stats: calculateStats(durations), results }
}

// ============================================================================
// Read Benchmarks
// ============================================================================

async function benchmarkRead(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  const testCases: Array<{ name: string; file: string; target: number }> = [
    { name: "Read 1KB", file: "1KB.txt", target: TARGETS.read_1kb },
    { name: "Read 100KB", file: "100KB.txt", target: TARGETS.read_100kb },
    { name: "Read 1MB", file: "1MB.txt", target: TARGETS.read_1mb },
    { name: "Read 10MB", file: "10MB.txt", target: TARGETS.read_10mb },
  ]

  for (const tc of testCases) {
    const filePath = getFixturePath(tc.file)

    const { stats } = await benchmark(tc.name, async () => {
      const file = Bun.file(filePath)
      return file.text()
    })

    results.push({
      name: tc.name,
      target: `P95 ≤${tc.target}ms`,
      result: `${stats.p95.toFixed(1)}ms`,
      pass: stats.p95 <= tc.target,
      numericValue: stats.p95,
      unit: "ms",
      details: {
        p50: `${stats.p50.toFixed(1)}ms`,
        p99: `${stats.p99.toFixed(1)}ms`,
        avg: `${stats.avg.toFixed(1)}ms`,
      },
    })
  }

  return results
}

// ============================================================================
// Write Benchmarks
// ============================================================================

async function benchmarkWrite(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []
  const tempDir = path.join(FIXTURE_DIR, "temp-write")
  await fs.mkdir(tempDir, { recursive: true })

  const testCases: Array<{ name: string; sizeBytes: number; target: number }> = [
    { name: "Write 1KB", sizeBytes: 1024, target: TARGETS.write_1kb },
    { name: "Write 100KB", sizeBytes: 100 * 1024, target: TARGETS.write_100kb },
    { name: "Write 1MB", sizeBytes: 1024 * 1024, target: TARGETS.write_1mb },
  ]

  for (const tc of testCases) {
    const content = "x".repeat(tc.sizeBytes)

    const { stats } = await benchmark(tc.name, async () => {
      const filePath = path.join(tempDir, `write-${tc.sizeBytes}-${Date.now()}.txt`)
      await Bun.write(filePath, content)
      return filePath
    })

    results.push({
      name: tc.name,
      target: `P95 ≤${tc.target}ms`,
      result: `${stats.p95.toFixed(1)}ms`,
      pass: stats.p95 <= tc.target,
      numericValue: stats.p95,
      unit: "ms",
      details: {
        p50: `${stats.p50.toFixed(1)}ms`,
        p99: `${stats.p99.toFixed(1)}ms`,
        avg: `${stats.avg.toFixed(1)}ms`,
      },
    })
  }

  // Cleanup temp directory
  await fs.rm(tempDir, { recursive: true, force: true })

  return results
}

// ============================================================================
// Glob Benchmarks
// ============================================================================

async function benchmarkGlob(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []
  const projectRoot = path.resolve(import.meta.dir, "../../..")

  const testCases: Array<{ name: string; pattern: string; cwd: string; target: number }> = [
    {
      name: "Glob *.ts (simple)",
      pattern: "*.ts",
      cwd: FIXTURE_DIR,
      target: TARGETS.glob_simple,
    },
    {
      name: "Glob **/*.ts (recursive)",
      pattern: "**/*.ts",
      cwd: FIXTURE_DIR,
      target: TARGETS.glob_recursive,
    },
    {
      name: "Glob **/*.{ts,tsx} (project)",
      pattern: "**/*.{ts,tsx}",
      cwd: path.join(projectRoot, "packages/ccode/src"),
      target: TARGETS.glob_complex,
    },
  ]

  for (const tc of testCases) {
    const { stats, results: matchResults } = await benchmark(tc.name, async () => {
      const files: string[] = []
      for await (const file of Ripgrep.files({
        cwd: tc.cwd,
        glob: [tc.pattern],
      })) {
        files.push(file)
        if (files.length >= 500) break // Prevent runaway
      }
      return files.length
    })

    const avgMatches = matchResults.reduce((a, b) => a + b, 0) / matchResults.length

    results.push({
      name: tc.name,
      target: `P95 ≤${tc.target}ms`,
      result: `${stats.p95.toFixed(1)}ms`,
      pass: stats.p95 <= tc.target,
      numericValue: stats.p95,
      unit: "ms",
      details: {
        p50: `${stats.p50.toFixed(1)}ms`,
        p99: `${stats.p99.toFixed(1)}ms`,
        matches: Math.round(avgMatches),
      },
    })
  }

  return results
}

// ============================================================================
// Grep Benchmarks
// ============================================================================

async function benchmarkGrep(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []
  const projectRoot = path.resolve(import.meta.dir, "../../..")
  const srcDir = path.join(projectRoot, "packages/ccode/src")

  // Use native grep from @codecoder-ai/core
  const { grep: nativeGrep } = await import("@codecoder-ai/core")
  if (!nativeGrep) {
    console.warn("Skipping grep benchmarks: native grep not available")
    return results
  }

  const testCases: Array<{ name: string; pattern: string; target: number }> = [
    { name: "Grep simple", pattern: "function", target: TARGETS.grep_simple },
    { name: "Grep regex", pattern: "async\\s+function\\s+\\w+", target: TARGETS.grep_regex },
  ]

  for (const tc of testCases) {
    const { stats, results: matchResults } = await benchmark(tc.name, async () => {
      // Native grep: (pattern, path, options?) - cast through unknown to bypass union type mismatch
      const grepFn = nativeGrep as unknown as (pattern: string, path: string, options?: any) => Promise<any[]>
      const result = await grepFn(tc.pattern, srcDir, {
        outputMode: "count",
      })

      // Result is array or object with totalMatches
      return Array.isArray(result) ? result.length : ((result as any).totalMatches ?? 0)
    })

    const avgMatches = matchResults.reduce((a, b) => a + b, 0) / matchResults.length

    results.push({
      name: tc.name,
      target: `P95 ≤${tc.target}ms`,
      result: `${stats.p95.toFixed(1)}ms`,
      pass: stats.p95 <= tc.target,
      numericValue: stats.p95,
      unit: "ms",
      details: {
        p50: `${stats.p50.toFixed(1)}ms`,
        p99: `${stats.p99.toFixed(1)}ms`,
        matches: Math.round(avgMatches),
      },
    })
  }

  return results
}

// ============================================================================
// Bash Benchmarks
// ============================================================================

async function benchmarkBash(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  const testCases: Array<{ name: string; command: string[]; target: number }> = [
    { name: "Bash echo", command: ["echo", "hello"], target: TARGETS.bash_simple },
    { name: "Bash ls", command: ["ls", "-la"], target: TARGETS.bash_simple },
    {
      name: "Bash pipeline",
      command: ["bash", "-c", "ls -la | head -5 | wc -l"],
      target: TARGETS.bash_pipeline,
    },
  ]

  for (const tc of testCases) {
    const { stats } = await benchmark(tc.name, async () => {
      const proc = Bun.spawn(tc.command, {
        stdout: "pipe",
        stderr: "pipe",
      })

      await new Response(proc.stdout).text()
      return proc.exited
    })

    results.push({
      name: tc.name,
      target: `P95 ≤${tc.target}ms`,
      result: `${stats.p95.toFixed(1)}ms`,
      pass: stats.p95 <= tc.target,
      numericValue: stats.p95,
      unit: "ms",
      details: {
        p50: `${stats.p50.toFixed(1)}ms`,
        p99: `${stats.p99.toFixed(1)}ms`,
      },
    })
  }

  return results
}

// ============================================================================
// Main Runner
// ============================================================================

export async function runToolBenchmarks(): Promise<BenchmarkResult[]> {
  console.log("  Ensuring fixtures exist...")
  await ensureFixtures()

  const results: BenchmarkResult[] = []

  console.log("  Benchmarking Read operations...")
  results.push(...(await benchmarkRead()))

  console.log("  Benchmarking Write operations...")
  results.push(...(await benchmarkWrite()))

  console.log("  Benchmarking Glob operations...")
  results.push(...(await benchmarkGlob()))

  console.log("  Benchmarking Grep operations...")
  results.push(...(await benchmarkGrep()))

  console.log("  Benchmarking Bash operations...")
  results.push(...(await benchmarkBash()))

  return results
}

// Entry point for standalone execution
if (import.meta.main) {
  console.log("Running Tool Performance Benchmarks...\n")
  runToolBenchmarks()
    .then((results) => {
      console.log("\n=== Tool Benchmark Results ===\n")
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
