#!/usr/bin/env bun
/**
 * Test Coverage Verification Script
 *
 * Parses the test case documentation and compares against implemented tests
 * to generate a coverage report. Also reads code coverage data when available.
 *
 * Usage:
 *   bun test/verify-coverage.ts
 *   bun test/verify-coverage.ts --verbose
 *   bun test/verify-coverage.ts --json
 */

import { Glob } from "bun"
import path from "path"
import fs from "fs/promises"

interface TestCase {
  id: string
  title: string
  type: string
  priority: string
  implemented: boolean
  testFile?: string
}

interface CodeCoverage {
  lines: { total: number; covered: number; percent: number }
  branches: { total: number; covered: number; percent: number }
  functions: { total: number; covered: number; percent: number }
}

interface CoverageReport {
  totalCases: number
  implementedCases: number
  missingCases: number
  coveragePercent: number
  byPriority: Record<string, { total: number; implemented: number }>
  byUserType: Record<string, { total: number; implemented: number }>
  details: TestCase[]
  codeCoverage?: CodeCoverage
}

const TEST_DIR = import.meta.dir
const DOC_PATH = path.join(TEST_DIR, "..", "docs", "User-Lifecycle-and-Test-Cases.md")

/**
 * Parse test case IDs from the documentation
 */
async function parseDocumentation(): Promise<TestCase[]> {
  const testCases: TestCase[] = []

  try {
    const content = await Bun.file(DOC_PATH).text()

    // Pattern to match test case IDs like ULC-NU-AUTH-001
    const idPattern = /ULC-([A-Z]{2,3})-([A-Z]{2,4})-(\d{3})/g
    const matches = content.matchAll(idPattern)

    const seen = new Set<string>()
    for (const match of matches) {
      const id = match[0]
      if (seen.has(id)) continue
      seen.add(id)

      // Extract user type code
      const userType = match[1]
      const featureCode = match[2]

      // Try to find the title (usually on the same line or next line)
      const linePattern = new RegExp(`${id}[:\\s]+([^\\n]+)`, "m")
      const titleMatch = content.match(linePattern)
      const title = titleMatch?.[1]?.replace(/^#+\s*/, "").trim() ?? ""

      // Determine priority from context
      let priority = "medium"
      const contextStart = content.indexOf(id)
      const contextEnd = Math.min(contextStart + 500, content.length)
      const context = content.slice(contextStart, contextEnd).toLowerCase()

      if (context.includes("critical")) priority = "critical"
      else if (context.includes("high")) priority = "high"
      else if (context.includes("low")) priority = "low"

      // Determine test type
      let type = "integration"
      if (context.includes("e2e")) type = "e2e"
      else if (context.includes("unit")) type = "unit"

      testCases.push({
        id,
        title,
        type,
        priority,
        implemented: false,
      })
    }
  } catch (error) {
    console.error("Warning: Could not read documentation file:", error)
  }

  return testCases
}

/**
 * Scan test files for implemented test case IDs
 */
async function scanTestFiles(): Promise<Map<string, string>> {
  const implemented = new Map<string, string>()

  const glob = new Glob("**/*.test.ts")

  for await (const file of glob.scan(TEST_DIR)) {
    const filePath = path.join(TEST_DIR, file)
    const content = await Bun.file(filePath).text()

    // Find all test case IDs mentioned in the file
    const idPattern = /ULC-([A-Z]{2,3})-([A-Z]{2,4})-(\d{3})/g
    const matches = content.matchAll(idPattern)

    for (const match of matches) {
      implemented.set(match[0], file)
    }
  }

  return implemented
}

/**
 * Parse code coverage data from coverage directory
 */
async function parseCodeCoverage(): Promise<CodeCoverage | undefined> {
  const coverageDir = path.join(TEST_DIR, "..", "coverage")

  try {
    // Check if coverage directory exists
    const dirExists = await fs.stat(coverageDir).catch(() => null)
    if (!dirExists) {
      return undefined
    }

    // Try to read lcov.info or similar coverage file
    const lcovPath = path.join(coverageDir, "lcov.info")
    const lcovExists = await fs.stat(lcovPath).catch(() => null)

    if (lcovExists) {
      const content = await Bun.file(lcovPath).text()
      return parseLcov(content)
    }

    // Try to read coverage-summary.json (common format)
    const summaryPath = path.join(coverageDir, "coverage-summary.json")
    const summaryExists = await fs.stat(summaryPath).catch(() => null)

    if (summaryExists) {
      const summary = await Bun.file(summaryPath).json()
      if (summary.total) {
        return {
          lines: {
            total: summary.total.lines.total,
            covered: summary.total.lines.covered,
            percent: summary.total.lines.pct,
          },
          branches: {
            total: summary.total.branches.total,
            covered: summary.total.branches.covered,
            percent: summary.total.branches.pct,
          },
          functions: {
            total: summary.total.functions.total,
            covered: summary.total.functions.covered,
            percent: summary.total.functions.pct,
          },
        }
      }
    }

    return undefined
  } catch {
    return undefined
  }
}

/**
 * Parse LCOV format coverage data
 */
function parseLcov(content: string): CodeCoverage {
  const lines = { total: 0, covered: 0, percent: 0 }
  const branches = { total: 0, covered: 0, percent: 0 }
  const functions = { total: 0, covered: 0, percent: 0 }

  const lcovLines = content.split("\n")

  for (const line of lcovLines) {
    // Line coverage: LF (lines found), LH (lines hit)
    if (line.startsWith("LF:")) {
      lines.total += parseInt(line.slice(3), 10)
    } else if (line.startsWith("LH:")) {
      lines.covered += parseInt(line.slice(3), 10)
    }
    // Branch coverage: BRF (branches found), BRH (branches hit)
    else if (line.startsWith("BRF:")) {
      branches.total += parseInt(line.slice(4), 10)
    } else if (line.startsWith("BRH:")) {
      branches.covered += parseInt(line.slice(4), 10)
    }
    // Function coverage: FNF (functions found), FNH (functions hit)
    else if (line.startsWith("FNF:")) {
      functions.total += parseInt(line.slice(4), 10)
    } else if (line.startsWith("FNH:")) {
      functions.covered += parseInt(line.slice(4), 10)
    }
  }

  lines.percent = lines.total > 0 ? (lines.covered / lines.total) * 100 : 0
  branches.percent = branches.total > 0 ? (branches.covered / branches.total) * 100 : 0
  functions.percent = functions.total > 0 ? (functions.covered / functions.total) * 100 : 0

  return { lines, branches, functions }
}

/**
 * Generate coverage report
 */
async function generateReport(verbose = false): Promise<CoverageReport> {
  const testCases = await parseDocumentation()
  const implemented = await scanTestFiles()
  const codeCoverage = await parseCodeCoverage()

  // Update test cases with implementation status
  for (const tc of testCases) {
    if (implemented.has(tc.id)) {
      tc.implemented = true
      tc.testFile = implemented.get(tc.id)
    }
  }

  // Calculate statistics
  const byPriority: Record<string, { total: number; implemented: number }> = {}
  const byUserType: Record<string, { total: number; implemented: number }> = {}

  for (const tc of testCases) {
    // By priority
    if (!byPriority[tc.priority]) {
      byPriority[tc.priority] = { total: 0, implemented: 0 }
    }
    byPriority[tc.priority].total++
    if (tc.implemented) byPriority[tc.priority].implemented++

    // By user type
    const userType = tc.id.split("-")[1]
    if (!byUserType[userType]) {
      byUserType[userType] = { total: 0, implemented: 0 }
    }
    byUserType[userType].total++
    if (tc.implemented) byUserType[userType].implemented++
  }

  const implementedCount = testCases.filter((tc) => tc.implemented).length
  const coveragePercent = testCases.length > 0 ? (implementedCount / testCases.length) * 100 : 0

  return {
    totalCases: testCases.length,
    implementedCases: implementedCount,
    missingCases: testCases.length - implementedCount,
    coveragePercent,
    byPriority,
    byUserType,
    details: testCases,
    codeCoverage,
  }
}

/**
 * User type code to name mapping
 */
const USER_TYPE_NAMES: Record<string, string> = {
  NU: "New User",
  CD: "CLI Developer",
  TU: "TUI User",
  WU: "Web User",
  RU: "Remote User",
  MU: "MCP User",
  PU: "Power User",
  SU: "API/Server User",
  ALL: "Cross-User",
}

/**
 * Print report to console
 */
function printReport(report: CoverageReport, verbose: boolean) {
  console.log("\n" + "=".repeat(60))
  console.log("CodeCoder Test Coverage Report")
  console.log("=".repeat(60) + "\n")

  // Summary
  console.log("## Summary\n")
  console.log(`Total Test Cases: ${report.totalCases}`)
  console.log(`Implemented: ${report.implementedCases}`)
  console.log(`Missing: ${report.missingCases}`)
  console.log(`Coverage: ${report.coveragePercent.toFixed(1)}%\n`)

  // Code Coverage (if available)
  if (report.codeCoverage) {
    console.log("## Code Coverage\n")
    console.log("| Metric | Covered | Total | Percent |")
    console.log("|--------|---------|-------|---------|")
    const { lines, branches, functions } = report.codeCoverage
    console.log(`| Lines | ${lines.covered} | ${lines.total} | ${lines.percent.toFixed(1)}% |`)
    console.log(`| Branches | ${branches.covered} | ${branches.total} | ${branches.percent.toFixed(1)}% |`)
    console.log(`| Functions | ${functions.covered} | ${functions.total} | ${functions.percent.toFixed(1)}% |`)
    console.log()

    // Check thresholds
    const lineThreshold = 80
    const branchThreshold = 70
    const functionThreshold = 75

    if (lines.percent < lineThreshold) {
      console.log(`  \u26a0\ufe0f Line coverage (${lines.percent.toFixed(1)}%) below ${lineThreshold}% threshold`)
    }
    if (branches.percent < branchThreshold) {
      console.log(`  \u26a0\ufe0f Branch coverage (${branches.percent.toFixed(1)}%) below ${branchThreshold}% threshold`)
    }
    if (functions.percent < functionThreshold) {
      console.log(`  \u26a0\ufe0f Function coverage (${functions.percent.toFixed(1)}%) below ${functionThreshold}% threshold`)
    }
    if (lines.percent >= lineThreshold && branches.percent >= branchThreshold && functions.percent >= functionThreshold) {
      console.log("  \u2705 All code coverage thresholds met!")
    }
    console.log()
  } else {
    console.log("## Code Coverage\n")
    console.log("(No coverage data available. Run `bun test --coverage` first.)\n")
  }

  // By Priority
  console.log("## By Priority\n")
  console.log("| Priority | Total | Implemented | Coverage |")
  console.log("|----------|-------|-------------|----------|")
  for (const [priority, stats] of Object.entries(report.byPriority)) {
    const pct = stats.total > 0 ? ((stats.implemented / stats.total) * 100).toFixed(0) : "0"
    console.log(`| ${priority.padEnd(8)} | ${String(stats.total).padEnd(5)} | ${String(stats.implemented).padEnd(11)} | ${pct}% |`)
  }
  console.log()

  // By User Type
  console.log("## By User Type\n")
  console.log("| User Type | Total | Implemented | Coverage |")
  console.log("|-----------|-------|-------------|----------|")
  for (const [code, stats] of Object.entries(report.byUserType)) {
    const name = USER_TYPE_NAMES[code] ?? code
    const pct = stats.total > 0 ? ((stats.implemented / stats.total) * 100).toFixed(0) : "0"
    console.log(`| ${name.padEnd(12)} | ${String(stats.total).padEnd(5)} | ${String(stats.implemented).padEnd(11)} | ${pct}% |`)
  }
  console.log()

  // Missing tests
  if (report.missingCases > 0) {
    console.log("## Missing Test Cases\n")
    const missing = report.details.filter((tc) => !tc.implemented)
    for (const tc of missing) {
      console.log(`- ${tc.id}: ${tc.title || "(no title)"} [${tc.priority}]`)
    }
    console.log()
  }

  // Verbose: all test cases
  if (verbose) {
    console.log("## All Test Cases\n")
    console.log("| ID | Status | Priority | File |")
    console.log("|-----|--------|----------|------|")
    for (const tc of report.details) {
      const status = tc.implemented ? "✅" : "❌"
      const file = tc.testFile ?? "-"
      console.log(`| ${tc.id} | ${status} | ${tc.priority} | ${file} |`)
    }
    console.log()
  }

  // Exit with error if coverage is below threshold
  if (report.coveragePercent < 80) {
    console.log(`⚠️  Coverage is below 80% threshold (${report.coveragePercent.toFixed(1)}%)\n`)
  } else {
    console.log(`✅ Coverage meets 80% threshold (${report.coveragePercent.toFixed(1)}%)\n`)
  }
}

// Main execution
const args = process.argv.slice(2)
const verbose = args.includes("--verbose") || args.includes("-v")
const jsonOutput = args.includes("--json")

const report = await generateReport(verbose)

if (jsonOutput) {
  console.log(JSON.stringify(report, null, 2))
} else {
  printReport(report, verbose)
}

// Exit with appropriate code
process.exit(report.coveragePercent >= 80 ? 0 : 1)
