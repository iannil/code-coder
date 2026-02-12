/**
 * Coverage Analyzer
 *
 * Analyzes test and code coverage to generate coverage reports.
 * Integrates with Bun test coverage and custom coverage metrics.
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import type { CoverageAnalysis } from "../schema/verification-result"

const log = Log.create({ service: "verifier.coverage.analyzer" })

/**
 * Coverage analysis configuration
 */
export interface CoverageAnalyzerConfig {
  threshold?: number
  includePatterns?: string[]
  excludePatterns?: string[]
}

/**
 * Code coverage result
 */
export interface CodeCoverage {
  percentage: number
  coveredLines: number
  totalLines: number
  byFile: Map<string, FileCoverage>
}

/**
 * Per-file coverage
 */
export interface FileCoverage {
  path: string
  percentage: number
  coveredLines: number
  totalLines: number
  uncoveredLines: number[]
}

/**
 * Coverage analyzer state
 */
export class CoverageAnalyzer {
  private sessionId: string
  private config: CoverageAnalyzerConfig

  constructor(sessionId: string, config: CoverageAnalyzerConfig = {}) {
    this.sessionId = sessionId
    this.config = {
      threshold: 80,
      includePatterns: ["**/*.ts", "**/*.js", "**/*.tsx", "**/*.jsx"],
      excludePatterns: [
        "node_modules/**",
        "dist/**",
        "build/**",
        "**/*.test.ts",
        "**/*.test.js",
        "**/*.spec.ts",
        "**/*.spec.js",
      ],
      ...config,
    }
  }

  /**
   * Run full coverage analysis
   */
  async analyze(): Promise<CoverageAnalysis> {
    log.info("Starting coverage analysis", {
      sessionId: this.sessionId,
      threshold: this.config.threshold,
    })

    // Get code coverage
    const codeCoverage = await this.getCodeCoverage()

    // Get requirement coverage (from matrix)
    // This would typically come from the matrix
    const requirementCoverage = 0 // Placeholder
    const propertyCoverage = 0 // Placeholder

    const analysis: CoverageAnalysis = {
      requirementCoverage,
      testCoverage: codeCoverage.percentage,
      propertyCoverage,
      uncoveredRequirements: [],
      partiallyCoveredRequirements: [],
    }

    log.info("Coverage analysis completed", {
      sessionId: this.sessionId,
      testCoverage: codeCoverage.percentage,
      meetsThreshold: codeCoverage.percentage >= (this.config.threshold ?? 80),
    })

    return analysis
  }

  /**
   * Get code coverage from Bun test runner
   */
  async getCodeCoverage(): Promise<CodeCoverage> {
    try {
      const { execSync } = require("child_process")

      log.info("Running tests with coverage")

      const output = execSync(
        "bun test --coverage-json",
        {
          cwd: Instance.worktree,
          encoding: "utf-8",
          stdio: ["pipe", "pipe", "pipe"],
          env: {
            ...process.env,
            NODE_ENV: "test",
          },
          timeout: 60000,
        },
      )

      // Parse coverage output
      return this.parseCoverageOutput(output)
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)

      log.error("Failed to get code coverage", {
        error: errorMessage,
      })

      // Return empty coverage
      return {
        percentage: 0,
        coveredLines: 0,
        totalLines: 0,
        byFile: new Map(),
      }
    }
  }

  /**
   * Parse coverage output from Bun
   */
  private parseCoverageOutput(output: string): CodeCoverage {
    const byFile = new Map<string, FileCoverage>()

    try {
      // Try to parse as JSON
      const data = JSON.parse(output)

      if (data.coverage) {
        for (const [filePath, fileData] of Object.entries(data.coverage as Record<string, any>)) {
          if (this.shouldIncludeFile(filePath)) {
            const fileCoverage = this.parseFileCoverage(filePath, fileData)
            byFile.set(filePath, fileCoverage)
          }
        }
      }
    } catch {
      // Try to extract from text output
      const lines = output.split("\n")

      for (const line of lines) {
        const match = line.match(/(\S+):\s*(\d+\.?\d*)%/)

        if (match) {
          const [, filePath, percentage] = match

          if (this.shouldIncludeFile(filePath)) {
            byFile.set(filePath, {
              path: filePath,
              percentage: parseFloat(percentage),
              coveredLines: 0,
              totalLines: 0,
              uncoveredLines: [],
            })
          }
        }
      }
    }

    // Calculate totals
    let totalLines = 0
    let coveredLines = 0

    for (const coverage of byFile.values()) {
      totalLines += coverage.totalLines
      coveredLines += coverage.coveredLines
    }

    const percentage = totalLines > 0 ? (coveredLines / totalLines) * 100 : 0

    return {
      percentage,
      coveredLines,
      totalLines,
      byFile,
    }
  }

  /**
   * Parse file coverage data
   */
  private parseFileCoverage(filePath: string, data: any): FileCoverage {
    // Coverage data format varies by tool
    // Adjust based on actual Bun coverage output format

    if (data.coverage !== undefined) {
      const totalLines = data.total ?? 0
      const coveredLines = data.covered ?? 0
      const percentage = totalLines > 0 ? (coveredLines / totalLines) * 100 : 0

      return {
        path: filePath,
        percentage,
        coveredLines,
        totalLines,
        uncoveredLines: data.uncovered ?? [],
      }
    }

    // Default fallback
    return {
      path: filePath,
      percentage: 0,
      coveredLines: 0,
      totalLines: 0,
      uncoveredLines: [],
    }
  }

  /**
   * Check if file should be included in coverage
   */
  private shouldIncludeFile(filePath: string): boolean {
    // Check exclude patterns
    for (const pattern of this.config.excludePatterns ?? []) {
      if (filePath.includes(pattern) || this.matchesGlob(filePath, pattern)) {
        return false
      }
    }

    // Check include patterns
    if (this.config.includePatterns && this.config.includePatterns.length > 0) {
      return this.config.includePatterns.some((p) => this.matchesGlob(filePath, p))
    }

    return true
  }

  /**
   * Simple glob pattern matching
   */
  private matchesGlob(filePath: string, pattern: string): boolean {
    // Convert glob to regex
    const regexPattern = pattern
      .replace(/\*/g, ".*")
      .replace(/\?/g, ".")

    const regex = new RegExp(regexPattern)
    return regex.test(filePath)
  }

  /**
   * Find uncovered files
   */
  async findUncoveredFiles(): Promise<string[]> {
    const coverage = await this.getCodeCoverage()
    const threshold = this.config.threshold ?? 80

    return Array.from(coverage.byFile.values())
      .filter((f) => f.percentage < threshold)
      .map((f) => f.path)
  }

  /**
   * Find uncovered lines in a file
   */
  async getUncoveredLines(filePath: string): Promise<number[]> {
    const coverage = await this.getCodeCoverage()
    const fileCoverage = coverage.byFile.get(filePath)

    return fileCoverage?.uncoveredLines ?? []
  }

  /**
   * Generate coverage report
   */
  generateReport(coverage: CodeCoverage): string {
    let report = `# Code Coverage Report

> Session: ${this.sessionId}
> Generated: ${new Date().toISOString()}

## Summary

| Metric | Value |
|--------|-------|
| Overall Coverage | ${coverage.percentage.toFixed(2)}% |
| Covered Lines | ${coverage.coveredLines} |
| Total Lines | ${coverage.totalLines} |
| Files Analyzed | ${coverage.byFile.size} |

## File Breakdown

| File | Coverage | Covered/Total |
|------|----------|---------------|
`

    // Sort by coverage (lowest first)
    const sortedFiles = Array.from(coverage.byFile.entries()).sort(
      (a, b) => a[1].percentage - b[1].percentage,
    )

    for (const [path, fileCoverage] of sortedFiles) {
      const status =
        fileCoverage.percentage >= (this.config.threshold ?? 80)
          ? "✅"
          : fileCoverage.percentage >= 50
            ? "⚠️"
            : "❌"

      report += `| ${status} ${path} | ${fileCoverage.percentage.toFixed(2)}% | ${fileCoverage.coveredLines}/${fileCoverage.totalLines} |\n`
    }

    // Add uncovered files section
    const uncoveredFiles = sortedFiles.filter(
      ([, fc]) => fc.percentage < (this.config.threshold ?? 80),
    )

    if (uncoveredFiles.length > 0) {
      report += `\n## Files Below Threshold (${this.config.threshold}%)\n\n`

      for (const [path, fileCoverage] of uncoveredFiles) {
        report += `### ${path}\n\n`
        report += `- Coverage: ${fileCoverage.percentage.toFixed(2)}%\n`
        report += `- Gap: ${(fileCoverage.totalLines - fileCoverage.coveredLines)} lines\n`

        if (fileCoverage.uncoveredLines.length > 0) {
          report += `- Uncovered lines: ${fileCoverage.uncoveredLines.slice(0, 20).join(", ")}`
          if (fileCoverage.uncoveredLines.length > 20) {
            report += ` ... and ${fileCoverage.uncoveredLines.length - 20} more`
          }
          report += "\n"
        }
        report += "\n"
      }
    }

    return report
  }

  /**
   * Get coverage statistics
   */
  getStats(coverage: CodeCoverage): {
    average: number
    min: number
    max: number
    filesBelowThreshold: number
    filesAboveThreshold: number
  } {
    const percentages = Array.from(coverage.byFile.values()).map((f) => f.percentage)
    const threshold = this.config.threshold ?? 80

    return {
      average: percentages.length > 0
        ? percentages.reduce((a, b) => a + b, 0) / percentages.length
        : 0,
      min: percentages.length > 0 ? Math.min(...percentages) : 0,
      max: percentages.length > 0 ? Math.max(...percentages) : 0,
      filesBelowThreshold: percentages.filter((p) => p < threshold).length,
      filesAboveThreshold: percentages.filter((p) => p >= threshold).length,
    }
  }
}

/**
 * Create a coverage analyzer
 */
export function createCoverageAnalyzer(
  sessionId: string,
  config?: CoverageAnalyzerConfig,
): CoverageAnalyzer {
  return new CoverageAnalyzer(sessionId, config)
}

/**
 * Analyze coverage for specific file
 */
export async function analyzeFileCoverage(
  sessionId: string,
  filePath: string,
): Promise<FileCoverage | null> {
  const analyzer = createCoverageAnalyzer(sessionId)
  const coverage = await analyzer.getCodeCoverage()

  return coverage.byFile.get(filePath) ?? null
}
