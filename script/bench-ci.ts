#!/usr/bin/env bun
/**
 * CI Benchmark Script
 *
 * Runs the benchmark suite and handles CI-specific concerns:
 * - Baseline comparison
 * - Regression detection
 * - PR comment generation
 * - Exit code management
 *
 * Usage:
 *   ./script/bench-ci.ts                      # Run benchmarks, compare to baseline
 *   ./script/bench-ci.ts --update-baseline    # Run and save new baseline
 *   ./script/bench-ci.ts --threshold 15       # Custom regression threshold (%)
 *
 * Environment variables:
 *   CI=true                       # Running in CI environment
 *   GITHUB_OUTPUT                 # Path to GitHub output file
 *   BENCHMARK_BASELINE_PATH       # Custom baseline path
 *   BENCHMARK_THRESHOLD           # Regression threshold percentage
 */

import fs from "fs/promises"
import path from "path"
import { runAllBenchmarks, type BenchmarkReport, type BenchmarkOptions } from "../packages/ccode/bench"

// ============================================================================
// Configuration
// ============================================================================

const DEFAULT_BASELINE_PATH = ".benchmarks/baseline.json"
const DEFAULT_THRESHOLD = 10 // 10% regression threshold
const REPORT_OUTPUT_PATH = ".benchmarks/report.md"

interface CIOptions {
  updateBaseline: boolean
  threshold: number
  baselinePath: string
  outputMarkdown: boolean
}

function parseArgs(): CIOptions {
  const args = process.argv.slice(2)

  const options: CIOptions = {
    updateBaseline: false,
    threshold: parseInt(process.env.BENCHMARK_THRESHOLD ?? "", 10) || DEFAULT_THRESHOLD,
    baselinePath: process.env.BENCHMARK_BASELINE_PATH ?? DEFAULT_BASELINE_PATH,
    outputMarkdown: false,
  }

  for (let i = 0; i < args.length; i++) {
    const arg = args[i]
    switch (arg) {
      case "--update-baseline":
        options.updateBaseline = true
        break
      case "--threshold":
        options.threshold = parseInt(args[++i], 10)
        break
      case "--baseline":
        options.baselinePath = args[++i]
        break
      case "--output-markdown":
      case "--md":
        options.outputMarkdown = true
        break
    }
  }

  return options
}

// ============================================================================
// GitHub Actions Integration
// ============================================================================

async function setGitHubOutput(name: string, value: string): Promise<void> {
  const outputPath = process.env.GITHUB_OUTPUT
  if (!outputPath) return

  const output = `${name}<<EOF\n${value}\nEOF\n`
  await fs.appendFile(outputPath, output)
}

async function writeGitHubSummary(summary: string): Promise<void> {
  const summaryPath = process.env.GITHUB_STEP_SUMMARY
  if (!summaryPath) return

  await fs.writeFile(summaryPath, summary)
}

// ============================================================================
// Report Generation
// ============================================================================

function generatePRComment(report: BenchmarkReport, hasRegressions: boolean): string {
  const lines: string[] = []

  // Header with status
  const statusEmoji = report.summary.failed > 0 ? "❌" : hasRegressions ? "⚠️" : "✅"
  const statusText = report.summary.failed > 0
    ? "Benchmark Failures"
    : hasRegressions
      ? "Performance Regressions Detected"
      : "All Benchmarks Passed"

  lines.push(`## ${statusEmoji} Performance Benchmarks: ${statusText}`)
  lines.push("")
  lines.push(`**Date**: ${report.date}${report.commit ? ` | **Commit**: \`${report.commit}\`` : ""}`)
  lines.push("")

  // Summary badges
  lines.push(`| Passed | Failed | Total |`)
  lines.push(`|--------|--------|-------|`)
  lines.push(`| ${report.summary.passed} ✅ | ${report.summary.failed} ❌ | ${report.summary.total} |`)
  lines.push("")

  // Results table
  lines.push("### Results")
  lines.push("")
  lines.push("| Metric | Result | Target | Status |")
  lines.push("|--------|--------|--------|--------|")

  for (const result of report.results) {
    const status = result.pass ? "✅" : "❌"
    lines.push(`| ${result.name} | ${result.result} | ${result.target} | ${status} |`)
  }

  // Regression details
  if (report.regressions && report.regressions.length > 0) {
    const significant = report.regressions.filter((r) => r.severity !== "none")

    if (significant.length > 0) {
      lines.push("")
      lines.push("### ⚠️ Performance Regressions")
      lines.push("")
      lines.push("| Metric | Baseline | Current | Change | Severity |")
      lines.push("|--------|----------|---------|--------|----------|")

      for (const reg of significant) {
        const changeStr = reg.changePercent >= 0 ? `+${reg.changePercent.toFixed(1)}%` : `${reg.changePercent.toFixed(1)}%`
        const icon = reg.severity === "critical" ? "🔴" : reg.severity === "major" ? "🟠" : "🟡"
        lines.push(
          `| ${reg.name} | ${reg.baseline}${reg.unit} | ${reg.current}${reg.unit} | ${changeStr} | ${icon} ${reg.severity} |`,
        )
      }
    }
  }

  // Environment info (collapsed)
  lines.push("")
  lines.push("<details>")
  lines.push("<summary>Environment Details</summary>")
  lines.push("")
  lines.push("| Property | Value |")
  lines.push("|----------|-------|")
  lines.push(`| Platform | ${report.environment.platform} |`)
  lines.push(`| Architecture | ${report.environment.arch} |`)
  lines.push(`| CPUs | ${report.environment.cpus} |`)
  lines.push(`| Memory | ${report.environment.memory} |`)
  lines.push(`| Bun Version | ${report.environment.bunVersion} |`)
  lines.push("")
  lines.push("</details>")
  lines.push("")
  lines.push("---")
  lines.push("*Generated by CodeCoder Benchmark Suite*")

  return lines.join("\n")
}

// ============================================================================
// Main
// ============================================================================

async function main(): Promise<void> {
  const ciOptions = parseArgs()
  const isCI = process.env.CI === "true"

  console.log("🚀 CodeCoder CI Benchmark Runner")
  console.log(`   Baseline: ${ciOptions.baselinePath}`)
  console.log(`   Threshold: ${ciOptions.threshold}%`)
  console.log(`   Update baseline: ${ciOptions.updateBaseline}`)
  console.log("")

  // Check if baseline exists
  let baselineExists = false
  try {
    await fs.access(ciOptions.baselinePath)
    baselineExists = true
    console.log("📊 Found existing baseline")
  } catch {
    console.log("📊 No baseline found, will create one if --update-baseline is set")
  }

  // Run benchmarks
  const benchOptions: Partial<BenchmarkOptions> = {
    outputFormat: "console",
    threshold: ciOptions.threshold,
    saveBaseline: ciOptions.updateBaseline,
    baselineOutputPath: ciOptions.baselinePath,
  }

  // Only compare against baseline if it exists
  if (baselineExists && !ciOptions.updateBaseline) {
    benchOptions.baselinePath = ciOptions.baselinePath
  }

  const report = await runAllBenchmarks(benchOptions)

  // Check for regressions
  const hasRegressions = report.regressions?.some((r) => r.changePercent >= ciOptions.threshold) ?? false

  // Generate PR comment
  const prComment = generatePRComment(report, hasRegressions)

  // Save markdown report
  if (ciOptions.outputMarkdown) {
    await fs.mkdir(path.dirname(REPORT_OUTPUT_PATH), { recursive: true })
    await fs.writeFile(REPORT_OUTPUT_PATH, prComment)
    console.log(`\n📝 Saved report to ${REPORT_OUTPUT_PATH}`)
  }

  // GitHub Actions integration
  if (isCI) {
    await setGitHubOutput("passed", String(report.summary.passed))
    await setGitHubOutput("failed", String(report.summary.failed))
    await setGitHubOutput("has_regressions", String(hasRegressions))
    await setGitHubOutput("pr_comment", prComment)
    await writeGitHubSummary(prComment)
  }

  // Print summary
  console.log("\n" + "=".repeat(60))
  console.log("📊 SUMMARY")
  console.log("=".repeat(60))
  console.log(`   Total:  ${report.summary.total}`)
  console.log(`   Passed: ${report.summary.passed} ✅`)
  console.log(`   Failed: ${report.summary.failed} ${report.summary.failed > 0 ? "❌" : ""}`)

  if (hasRegressions) {
    console.log(`   Regressions: Yes ⚠️`)
  }

  console.log("=".repeat(60))

  // Exit code
  if (report.summary.failed > 0) {
    console.log("\n❌ Benchmark failures detected")
    process.exit(1)
  }

  if (hasRegressions) {
    console.log(`\n⚠️ Performance regressions detected (threshold: ${ciOptions.threshold}%)`)
    process.exit(2)
  }

  console.log("\n✅ All benchmarks passed")
  process.exit(0)
}

main().catch((err) => {
  console.error("Benchmark runner failed:", err)
  process.exit(1)
})
