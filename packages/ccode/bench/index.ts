/**
 * CodeCoder Performance Benchmark Suite
 *
 * This module provides comprehensive performance benchmarks to validate
 * NFR-04 requirements from docs/standards/goals.md:
 *
 * - ZeroBot startup time ≤ 0.5s
 * - Plan mode scan for 100k LOC ≤ 15s
 * - Gateway memory < 5MB
 *
 * Run with: bun run bench
 */

import { runStartupBenchmarks } from "./startup.bench"
import { runPlanScanBenchmarks } from "./plan-scan.bench"
import { runApiLatencyBenchmarks } from "./api-latency.bench"
import { runMcpBenchmarks } from "./mcp.bench"

export interface BenchmarkResult {
  name: string
  target: string
  result: string
  pass: boolean
  details?: Record<string, unknown>
}

export interface BenchmarkReport {
  date: string
  environment: {
    platform: string
    arch: string
    cpus: number
    memory: string
    nodeVersion: string
    bunVersion: string
  }
  results: BenchmarkResult[]
  summary: {
    total: number
    passed: number
    failed: number
  }
}

function getEnvironment(): BenchmarkReport["environment"] {
  const os = require("os")
  return {
    platform: `${os.platform()} ${os.release()}`,
    arch: os.arch(),
    cpus: os.cpus().length,
    memory: `${Math.round(os.totalmem() / (1024 * 1024 * 1024))}GB`,
    nodeVersion: process.version,
    bunVersion: typeof Bun !== "undefined" ? Bun.version : "N/A",
  }
}

function formatReport(report: BenchmarkReport): string {
  const lines: string[] = [
    "=== CodeCoder Performance Report ===",
    `Date: ${report.date}`,
    "",
    "## Environment",
    `- Platform: ${report.environment.platform}`,
    `- Architecture: ${report.environment.arch}`,
    `- CPUs: ${report.environment.cpus}`,
    `- Memory: ${report.environment.memory}`,
    `- Bun Version: ${report.environment.bunVersion}`,
    "",
    "## Results",
    "",
    "| Metric | Result | Target | Status |",
    "|--------|--------|--------|--------|",
  ]

  for (const result of report.results) {
    const status = result.pass ? "✅ PASS" : "❌ FAIL"
    lines.push(`| ${result.name} | ${result.result} | ${result.target} | ${status} |`)
  }

  lines.push("")
  lines.push("## Summary")
  lines.push(`- Total: ${report.summary.total}`)
  lines.push(`- Passed: ${report.summary.passed}`)
  lines.push(`- Failed: ${report.summary.failed}`)
  lines.push("")

  return lines.join("\n")
}

export async function runAllBenchmarks(): Promise<BenchmarkReport> {
  console.log("Starting CodeCoder Performance Benchmarks...")
  console.log("")

  const results: BenchmarkResult[] = []

  // Run startup benchmarks
  console.log(">>> Running Startup Benchmarks...")
  const startupResults = await runStartupBenchmarks()
  results.push(...startupResults)

  // Run plan scan benchmarks
  console.log("\n>>> Running Plan Scan Benchmarks...")
  const planResults = await runPlanScanBenchmarks()
  results.push(...planResults)

  // Run API latency benchmarks
  console.log("\n>>> Running API Latency Benchmarks...")
  const apiResults = await runApiLatencyBenchmarks()
  results.push(...apiResults)

  // Run MCP benchmarks
  console.log("\n>>> Running MCP Benchmarks...")
  const mcpResults = await runMcpBenchmarks()
  results.push(...mcpResults)

  const report: BenchmarkReport = {
    date: new Date().toISOString().split("T")[0],
    environment: getEnvironment(),
    results,
    summary: {
      total: results.length,
      passed: results.filter((r) => r.pass).length,
      failed: results.filter((r) => !r.pass).length,
    },
  }

  console.log("\n" + formatReport(report))

  return report
}

// Entry point when run directly
if (import.meta.main) {
  runAllBenchmarks()
    .then((report) => {
      // Exit with error code if any benchmark failed
      process.exit(report.summary.failed > 0 ? 1 : 0)
    })
    .catch((error) => {
      console.error("Benchmark failed:", error)
      process.exit(1)
    })
}
