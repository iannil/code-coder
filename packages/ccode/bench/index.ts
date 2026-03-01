/**
 * CodeCoder Performance Benchmark Suite
 *
 * This module provides comprehensive performance benchmarks to validate
 * NFR-04 requirements from docs/standards/goals.md:
 *
 * - NFR-04-1: ZeroBot startup time ≤ 0.5s
 * - NFR-04-2: Plan mode scan for 100k LOC ≤ 15s
 * - NFR-04-3: Gateway memory < 5MB
 *
 * Run with: bun run bench
 *
 * Output formats:
 * - Console: Human-readable table (default)
 * - JSON: Machine-readable for CI (--json)
 * - Markdown: For PR comments (--markdown)
 *
 * Regression detection:
 * - Compare against baseline: --baseline .benchmarks/baseline.json
 * - Fail on regression > threshold: --threshold 10 (percent)
 * - Save new baseline: --save-baseline
 */

import fs from "fs/promises"
import path from "path"
import { runStartupBenchmarks } from "./startup.bench"
import { runPlanScanBenchmarks } from "./plan-scan.bench"
import { runApiLatencyBenchmarks } from "./api-latency.bench"
import { runMcpBenchmarks } from "./mcp.bench"
import { runToolBenchmarks } from "./tool.bench"

// ============================================================================
// Types
// ============================================================================

export interface BenchmarkResult {
  name: string
  target: string
  result: string
  pass: boolean
  details?: Record<string, unknown>
  // Numeric value for regression comparison (extracted from result)
  numericValue?: number
  unit?: string
}

export interface BenchmarkReport {
  date: string
  commit?: string
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
  regressions?: RegressionResult[]
}

export interface Baseline {
  date: string
  commit?: string
  results: Record<string, { value: number; unit: string }>
}

export interface RegressionResult {
  name: string
  baseline: number
  current: number
  unit: string
  changePercent: number
  severity: "none" | "minor" | "major" | "critical"
}

export interface BenchmarkOptions {
  outputFormat: "console" | "json" | "markdown"
  baselinePath?: string
  threshold: number // Regression threshold in percent
  saveBaseline: boolean
  baselineOutputPath?: string
  // Suite selection flags
  includeTools: boolean
  includeMemory: boolean
  includeLlm: boolean
}

// ============================================================================
// Configuration
// ============================================================================

const DEFAULT_BASELINE_PATH = path.join(process.cwd(), ".benchmarks", "baseline.json")
const REGRESSION_THRESHOLDS = {
  minor: 5, // 5% slower
  major: 10, // 10% slower
  critical: 25, // 25% slower
}

// ============================================================================
// Environment & Git
// ============================================================================

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

async function getGitCommit(): Promise<string | undefined> {
  try {
    const proc = Bun.spawn(["git", "rev-parse", "--short", "HEAD"], {
      stdout: "pipe",
      stderr: "ignore",
    })
    const output = await new Response(proc.stdout).text()
    return output.trim() || undefined
  } catch {
    return undefined
  }
}

// ============================================================================
// Numeric Value Extraction
// ============================================================================

/**
 * Extract numeric value and unit from result string
 * Examples: "320ms" -> { value: 320, unit: "ms" }
 *           "3.2MB" -> { value: 3.2, unit: "MB" }
 *           "12.3s" -> { value: 12.3, unit: "s" }
 */
function extractNumericValue(result: string): { value: number; unit: string } | null {
  const match = result.match(/^([\d.]+)\s*([a-zA-Z]+)?$/)
  if (!match) return null
  return {
    value: parseFloat(match[1]),
    unit: match[2] || "",
  }
}

/**
 * Normalize values to common units for comparison
 */
function normalizeToMs(value: number, unit: string): number {
  switch (unit.toLowerCase()) {
    case "s":
      return value * 1000
    case "ms":
      return value
    case "μs":
    case "us":
      return value / 1000
    default:
      return value
  }
}

// ============================================================================
// Baseline Management
// ============================================================================

async function loadBaseline(baselinePath: string): Promise<Baseline | null> {
  try {
    const content = await fs.readFile(baselinePath, "utf-8")
    return JSON.parse(content) as Baseline
  } catch {
    return null
  }
}

async function saveBaseline(baseline: Baseline, outputPath: string): Promise<void> {
  const dir = path.dirname(outputPath)
  await fs.mkdir(dir, { recursive: true })
  await fs.writeFile(outputPath, JSON.stringify(baseline, null, 2))
}

function createBaselineFromResults(results: BenchmarkResult[], commit?: string): Baseline {
  const baselineResults: Record<string, { value: number; unit: string }> = {}

  for (const result of results) {
    const numeric = extractNumericValue(result.result)
    if (numeric) {
      baselineResults[result.name] = numeric
    }
  }

  return {
    date: new Date().toISOString().split("T")[0],
    commit,
    results: baselineResults,
  }
}

// ============================================================================
// Regression Detection
// ============================================================================

function detectRegressions(
  results: BenchmarkResult[],
  baseline: Baseline,
  threshold: number,
): RegressionResult[] {
  const regressions: RegressionResult[] = []

  for (const result of results) {
    const baselineEntry = baseline.results[result.name]
    if (!baselineEntry) continue

    const numeric = extractNumericValue(result.result)
    if (!numeric) continue

    // Normalize to same unit for comparison
    const baselineMs = normalizeToMs(baselineEntry.value, baselineEntry.unit)
    const currentMs = normalizeToMs(numeric.value, numeric.unit)

    // Calculate percent change (positive = regression/slower)
    const changePercent = ((currentMs - baselineMs) / baselineMs) * 100

    let severity: RegressionResult["severity"] = "none"
    if (changePercent >= REGRESSION_THRESHOLDS.critical) {
      severity = "critical"
    } else if (changePercent >= REGRESSION_THRESHOLDS.major) {
      severity = "major"
    } else if (changePercent >= REGRESSION_THRESHOLDS.minor) {
      severity = "minor"
    }

    regressions.push({
      name: result.name,
      baseline: baselineEntry.value,
      current: numeric.value,
      unit: numeric.unit || baselineEntry.unit,
      changePercent,
      severity,
    })
  }

  return regressions
}

// ============================================================================
// Report Formatting
// ============================================================================

function formatReportConsole(report: BenchmarkReport): string {
  const lines: string[] = [
    "=== CodeCoder Performance Report ===",
    `Date: ${report.date}${report.commit ? ` | Commit: ${report.commit}` : ""}`,
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

  // Add regression info if present
  if (report.regressions && report.regressions.length > 0) {
    const significantRegressions = report.regressions.filter((r) => r.severity !== "none")
    if (significantRegressions.length > 0) {
      lines.push("")
      lines.push("## Regressions Detected")
      lines.push("")
      lines.push("| Metric | Baseline | Current | Change | Severity |")
      lines.push("|--------|----------|---------|--------|----------|")

      for (const reg of significantRegressions) {
        const changeStr = reg.changePercent >= 0 ? `+${reg.changePercent.toFixed(1)}%` : `${reg.changePercent.toFixed(1)}%`
        const severityIcon = reg.severity === "critical" ? "🔴" : reg.severity === "major" ? "🟠" : "🟡"
        lines.push(
          `| ${reg.name} | ${reg.baseline}${reg.unit} | ${reg.current}${reg.unit} | ${changeStr} | ${severityIcon} ${reg.severity} |`,
        )
      }
    }
  }

  lines.push("")

  return lines.join("\n")
}

function formatReportMarkdown(report: BenchmarkReport): string {
  const lines: string[] = [
    "# CodeCoder Performance Report",
    "",
    `**Date**: ${report.date}${report.commit ? ` | **Commit**: \`${report.commit}\`` : ""}`,
    "",
    "## Environment",
    "",
    `| Property | Value |`,
    `|----------|-------|`,
    `| Platform | ${report.environment.platform} |`,
    `| Architecture | ${report.environment.arch} |`,
    `| CPUs | ${report.environment.cpus} |`,
    `| Memory | ${report.environment.memory} |`,
    `| Bun Version | ${report.environment.bunVersion} |`,
    "",
    "## NFR Compliance",
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
  lines.push("")
  lines.push(`- **Total**: ${report.summary.total}`)
  lines.push(`- **Passed**: ${report.summary.passed}`)
  lines.push(`- **Failed**: ${report.summary.failed}`)

  // Regression section
  if (report.regressions && report.regressions.length > 0) {
    const significantRegressions = report.regressions.filter((r) => r.severity !== "none")

    if (significantRegressions.length > 0) {
      lines.push("")
      lines.push("## ⚠️ Performance Regressions")
      lines.push("")
      lines.push("| Metric | Baseline | Current | Change | Severity |")
      lines.push("|--------|----------|---------|--------|----------|")

      for (const reg of significantRegressions) {
        const changeStr = reg.changePercent >= 0 ? `+${reg.changePercent.toFixed(1)}%` : `${reg.changePercent.toFixed(1)}%`
        const severityIcon = reg.severity === "critical" ? "🔴" : reg.severity === "major" ? "🟠" : "🟡"
        lines.push(
          `| ${reg.name} | ${reg.baseline}${reg.unit} | ${reg.current}${reg.unit} | ${changeStr} | ${severityIcon} ${reg.severity.toUpperCase()} |`,
        )
      }
    } else {
      lines.push("")
      lines.push("## ✅ No Regressions Detected")
    }
  }

  lines.push("")
  lines.push("---")
  lines.push("*Generated by CodeCoder Benchmark Suite*")

  return lines.join("\n")
}

function formatReportJson(report: BenchmarkReport): string {
  return JSON.stringify(report, null, 2)
}

// ============================================================================
// CLI Argument Parsing
// ============================================================================

function parseArgs(): BenchmarkOptions {
  const args = process.argv.slice(2)

  const options: BenchmarkOptions = {
    outputFormat: "console",
    threshold: 10,
    saveBaseline: false,
    includeTools: false,
    includeMemory: false,
    includeLlm: false,
  }

  for (let i = 0; i < args.length; i++) {
    const arg = args[i]
    switch (arg) {
      case "--json":
        options.outputFormat = "json"
        break
      case "--markdown":
      case "--md":
        options.outputFormat = "markdown"
        break
      case "--baseline":
        options.baselinePath = args[++i]
        break
      case "--threshold":
        options.threshold = parseInt(args[++i], 10)
        break
      case "--save-baseline":
        options.saveBaseline = true
        if (args[i + 1] && !args[i + 1].startsWith("--")) {
          options.baselineOutputPath = args[++i]
        }
        break
      case "--tools":
        options.includeTools = true
        break
      case "--memory":
        options.includeMemory = true
        break
      case "--llm":
        options.includeLlm = true
        break
      case "--all":
        options.includeTools = true
        options.includeMemory = true
        options.includeLlm = true
        break
    }
  }

  return options
}

// ============================================================================
// Main Benchmark Runner
// ============================================================================

export async function runAllBenchmarks(options?: Partial<BenchmarkOptions>): Promise<BenchmarkReport> {
  const opts: BenchmarkOptions = {
    outputFormat: options?.outputFormat ?? "console",
    baselinePath: options?.baselinePath,
    threshold: options?.threshold ?? 10,
    saveBaseline: options?.saveBaseline ?? false,
    baselineOutputPath: options?.baselineOutputPath,
    includeTools: options?.includeTools ?? false,
    includeMemory: options?.includeMemory ?? false,
    includeLlm: options?.includeLlm ?? false,
  }

  // Only print to console in console mode
  const log = opts.outputFormat === "console" ? console.log : () => {}

  log("Starting CodeCoder Performance Benchmarks...")
  log("")

  const results: BenchmarkResult[] = []
  const commit = await getGitCommit()

  // Run startup benchmarks
  log(">>> Running Startup Benchmarks...")
  const startupResults = await runStartupBenchmarks()
  results.push(...startupResults)

  // Run plan scan benchmarks
  log("\n>>> Running Plan Scan Benchmarks...")
  const planResults = await runPlanScanBenchmarks()
  results.push(...planResults)

  // Run API latency benchmarks
  log("\n>>> Running API Latency Benchmarks...")
  const apiResults = await runApiLatencyBenchmarks()
  results.push(...apiResults)

  // Run MCP benchmarks
  log("\n>>> Running MCP Benchmarks...")
  const mcpResults = await runMcpBenchmarks()
  results.push(...mcpResults)

  // Run tool benchmarks (optional)
  if (opts.includeTools) {
    log("\n>>> Running Tool Benchmarks...")
    const toolResults = await runToolBenchmarks()
    results.push(...toolResults)
  }

  // Run memory benchmarks (optional)
  if (opts.includeMemory) {
    log("\n>>> Running Memory Benchmarks...")
    try {
      const { runMemoryBenchmarks } = await import("./memory.bench")
      const memoryResults = await runMemoryBenchmarks()
      results.push(...memoryResults)
    } catch (err) {
      log("  Memory benchmarks not available")
    }
  }

  // Run LLM benchmarks (optional)
  if (opts.includeLlm) {
    log("\n>>> Running LLM Benchmarks...")
    try {
      const { runLlmBenchmarks } = await import("./llm.bench")
      const llmResults = await runLlmBenchmarks()
      results.push(...llmResults)
    } catch (err) {
      log("  LLM benchmarks not available")
    }
  }

  // Load baseline for regression detection
  let regressions: RegressionResult[] | undefined
  if (opts.baselinePath) {
    const baseline = await loadBaseline(opts.baselinePath)
    if (baseline) {
      regressions = detectRegressions(results, baseline, opts.threshold)
      log(`\n>>> Comparing against baseline from ${baseline.date}`)
    } else {
      log(`\n>>> Warning: Could not load baseline from ${opts.baselinePath}`)
    }
  }

  const report: BenchmarkReport = {
    date: new Date().toISOString().split("T")[0],
    commit,
    environment: getEnvironment(),
    results,
    summary: {
      total: results.length,
      passed: results.filter((r) => r.pass).length,
      failed: results.filter((r) => !r.pass).length,
    },
    regressions,
  }

  // Save baseline if requested
  if (opts.saveBaseline) {
    const outputPath = opts.baselineOutputPath ?? DEFAULT_BASELINE_PATH
    const baseline = createBaselineFromResults(results, commit)
    await saveBaseline(baseline, outputPath)
    log(`\n>>> Saved baseline to ${outputPath}`)
  }

  // Output report in requested format
  switch (opts.outputFormat) {
    case "json":
      console.log(formatReportJson(report))
      break
    case "markdown":
      console.log(formatReportMarkdown(report))
      break
    default:
      console.log("\n" + formatReportConsole(report))
  }

  return report
}

/**
 * Check if any regression exceeds the threshold
 */
function hasSignificantRegressions(regressions: RegressionResult[] | undefined, threshold: number): boolean {
  if (!regressions) return false
  return regressions.some((r) => r.changePercent >= threshold)
}

// ============================================================================
// Entry Point
// ============================================================================

// Entry point when run directly
if (import.meta.main) {
  const options = parseArgs()

  runAllBenchmarks(options)
    .then((report) => {
      // Exit with error code if any benchmark failed
      if (report.summary.failed > 0) {
        process.exit(1)
      }

      // Exit with error code if significant regressions detected
      if (hasSignificantRegressions(report.regressions, options.threshold)) {
        if (options.outputFormat === "console") {
          console.error(`\n❌ Performance regressions detected (threshold: ${options.threshold}%)`)
        }
        process.exit(2)
      }

      process.exit(0)
    })
    .catch((error) => {
      console.error("Benchmark failed:", error)
      process.exit(1)
    })
}

// Export for programmatic use
export {
  loadBaseline,
  saveBaseline,
  createBaselineFromResults,
  detectRegressions,
  formatReportConsole,
  formatReportMarkdown,
  formatReportJson,
}
