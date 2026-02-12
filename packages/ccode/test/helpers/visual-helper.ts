/**
 * Visual Regression Test Helper
 *
 * Provides utilities for capturing and comparing terminal screenshots
 * for visual regression testing of the TUI.
 */

import { writeFileSync, readFileSync, existsSync, mkdirSync, rmSync } from "fs"
import { join } from "path"

/**
 * Terminal snapshot data structure
 */
export interface TerminalSnapshot {
  timestamp: number
  rows: number
  cols: number
  output: string
  ansiOutput: string
  metadata?: {
    testName?: string
    description?: string
    tags?: string[]
  }
}

/**
 * Comparison result with differences
 */
export interface ComparisonResult {
  passed: boolean
  baselineExists: boolean
  differences: DiffRegion[]
  similarity: number
  baseline: TerminalSnapshot | null
  current: TerminalSnapshot
}

/**
 * Region where differences were detected
 */
export interface DiffRegion {
  line: number
  start: number
  end: number
  baseline: string
  current: string
}

/**
 * Visual test configuration
 */
export interface VisualTestConfig {
  baselineDir: string
  actualDir: string
  diffDir: string
  ignorePatterns?: IgnorePattern[]
  tolerance?: number
}

/**
 * Pattern for content to ignore during comparison
 */
export interface IgnorePattern {
  type: "regex" | "wildcard" | "timestamp" | "duration" | "sessionId"
  pattern: string
  description?: string
}

// Default configuration
const DEFAULT_CONFIG: VisualTestConfig = {
  baselineDir: join(process.cwd(), "test", "visual", "baselines"),
  actualDir: join(process.cwd(), "test", "visual", "actual"),
  diffDir: join(process.cwd(), "test", "visual", "diffs"),
  ignorePatterns: [
    { type: "timestamp", pattern: "\\d{13}", description: "Unix timestamps" },
    { type: "timestamp", pattern: "\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}", description: "ISO dates" },
    { type: "sessionId", pattern: "sess_[a-zA-Z0-9]{8,}", description: "Session IDs" },
    { type: "wildcard", pattern: "\\[Loading...\\]", description: "Loading indicators" },
    { type: "wildcard", pattern: "\\[█+\\s*\\]", description: "Progress bars" },
  ],
  tolerance: 0.95, // 95% similarity threshold
}

/**
 * Visual regression tester class
 */
export class VisualTester {
  private config: VisualTestConfig
  private ignoreRegexes: RegExp[]

  constructor(config: Partial<VisualTestConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.ignoreRegexes = this.compileIgnorePatterns()
    this.ensureDirectories()
  }

  /**
   * Compile ignore patterns to regexes
   */
  private compileIgnorePatterns(): RegExp[] {
    return (this.config.ignorePatterns ?? []).map((p) => {
      if (p.type === "regex" || p.type === "wildcard") {
        return new RegExp(p.pattern, "g")
      }
      return new RegExp(p.pattern, "g")
    })
  }

  /**
   * Ensure test directories exist
   */
  private ensureDirectories(): void {
    for (const dir of [this.config.baselineDir, this.config.actualDir, this.config.diffDir]) {
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true })
      }
    }
  }

  /**
   * Sanitize output by removing ignored patterns
   */
  private sanitizeOutput(output: string): string {
    let sanitized = output
    for (const regex of this.ignoreRegexes) {
      sanitized = sanitized.replace(regex, "[REDACTED]")
    }
    return sanitized
  }

  /**
   * Normalize output for comparison (remove extra whitespace, etc)
   */
  private normalizeOutput(output: string): string {
    return (
      this.sanitizeOutput(output)
        // Normalize line endings
        .replace(/\r\n/g, "\n")
        // Remove trailing whitespace from each line
        .split("\n")
        .map((line) => line.trimEnd())
        .join("\n")
    )
  }

  /**
   * Calculate similarity between two strings (0-1)
   */
  private calculateSimilarity(a: string, b: string): number {
    const linesA = a.split("\n")
    const linesB = b.split("\n")
    const maxLines = Math.max(linesA.length, linesB.length)
    let matchingLines = 0

    for (let i = 0; i < maxLines; i++) {
      const lineA = linesA[i] ?? ""
      const lineB = linesB[i] ?? ""
      if (lineA === lineB) {
        matchingLines++
      } else {
        // Calculate character-level similarity for this line
        const maxChars = Math.max(lineA.length, lineB.length)
        let matchingChars = 0
        for (let j = 0; j < maxChars; j++) {
          if (lineA[j] === lineB[j]) matchingChars++
        }
        matchingLines += matchingChars / maxChars
      }
    }

    return matchingLines / maxLines
  }

  /**
   * Find differences between two outputs
   */
  private findDifferences(a: string, b: string): DiffRegion[] {
    const diffs: DiffRegion[] = []
    const linesA = a.split("\n")
    const linesB = b.split("\n")
    const maxLines = Math.max(linesA.length, linesB.length)

    for (let i = 0; i < maxLines; i++) {
      const lineA = linesA[i] ?? ""
      const lineB = linesB[i] ?? ""
      if (lineA !== lineB) {
        // Find character differences
        const maxChars = Math.max(lineA.length, lineB.length)
        let start = -1
        let end = -1
        for (let j = 0; j < maxChars; j++) {
          if (lineA[j] !== lineB[j]) {
            if (start === -1) start = j
            end = j + 1
          }
        }
        if (start !== -1) {
          diffs.push({
            line: i,
            start,
            end,
            baseline: lineA.slice(start, end),
            current: lineB.slice(start, end),
          })
        } else {
          diffs.push({
            line: i,
            start: 0,
            end: Math.max(lineA.length, lineB.length),
            baseline: lineA,
            current: lineB,
          })
        }
      }
    }

    return diffs
  }

  /**
   * Capture a snapshot from PTY output
   */
  captureSnapshot(
    output: string,
    metadata?: {
      testName?: string
      description?: string
      tags?: string[]
      rows?: number
      cols?: number
    },
  ): TerminalSnapshot {
    return {
      timestamp: Date.now(),
      output: this.normalizeOutput(output),
      ansiOutput: output,
      rows: metadata?.rows ?? 40,
      cols: metadata?.cols ?? 120,
      metadata,
    }
  }

  /**
   * Save a snapshot to disk
   */
  saveSnapshot(snapshot: TerminalSnapshot, dir: string, name: string): string {
    const filePath = join(dir, `${name}.json`)
    writeFileSync(filePath, JSON.stringify(snapshot, null, 2))
    return filePath
  }

  /**
   * Load a snapshot from disk
   */
  loadSnapshot(dir: string, name: string): TerminalSnapshot | null {
    const filePath = join(dir, `${name}.json`)
    if (!existsSync(filePath)) return null
    try {
      const content = readFileSync(filePath, "utf-8")
      return JSON.parse(content) as TerminalSnapshot
    } catch {
      return null
    }
  }

  /**
   * Compare current snapshot with baseline
   */
  compare(snapshot: TerminalSnapshot, testName: string): ComparisonResult {
    const baseline = this.loadSnapshot(this.config.baselineDir, testName)
    const tolerance = this.config.tolerance ?? DEFAULT_CONFIG.tolerance ?? 0.95

    if (!baseline) {
      return {
        passed: false,
        baselineExists: false,
        differences: [],
        similarity: 0,
        baseline: null,
        current: snapshot,
      }
    }

    const baselineOutput = baseline.output
    const currentOutput = snapshot.output

    const similarity = this.calculateSimilarity(baselineOutput, currentOutput)
    const differences = this.findDifferences(baselineOutput, currentOutput)
    const passed = similarity >= tolerance

    return {
      passed,
      baselineExists: true,
      differences,
      similarity,
      baseline,
      current: snapshot,
    }
  }

  /**
   * Assert that snapshot matches baseline
   * If baseline doesn't exist, create it
   */
  async assertMatches(snapshot: TerminalSnapshot, testName: string): Promise<void> {
    // Save actual snapshot
    this.saveSnapshot(snapshot, this.config.actualDir, testName)

    const result = this.compare(snapshot, testName)

    if (!result.baselineExists) {
      // Create new baseline
      this.saveSnapshot(snapshot, this.config.baselineDir, testName)
      throw new Error(
        `Baseline not found for "${testName}". Created new baseline. Review and commit if correct.`,
      )
    }

    if (!result.passed) {
      // Save diff info
      this.saveSnapshot(
        {
          ...result.current,
          metadata: {
            ...result.current.metadata,
            description: `Diff: ${result.differences.length} regions, ${Math.round(result.similarity * 100)}% similar`,
          },
        },
        this.config.diffDir,
        testName,
      )

      const diffLines = result.differences
        .slice(0, 10) // Limit output
        .map(
          (d) =>
            `  Line ${d.line}: "${d.baseline}" -> "${d.current}"`,
        )
        .join("\n")

      throw new Error(
        `Visual regression detected for "${testName}"\n` +
          `Similarity: ${Math.round(result.similarity * 100)}% (threshold: ${Math.round((this.config.tolerance ?? DEFAULT_CONFIG.tolerance ?? 0.95) * 100)}%)\n` +
          `Differences: ${result.differences.length} regions\n` +
          `First differences:\n${diffLines}\n` +
          `Check: ${join(this.config.diffDir, `${testName}.json`)}`,
      )
    }
  }

  /**
   * Update baseline for a test
   */
  updateBaseline(snapshot: TerminalSnapshot, testName: string): void {
    this.saveSnapshot(snapshot, this.config.baselineDir, testName)
  }

  /**
   * Remove baseline for a test
   */
  removeBaseline(testName: string): void {
    const filePath = join(this.config.baselineDir, `${testName}.json`)
    if (existsSync(filePath)) {
      rmSync(filePath)
    }
  }

  /**
   * Get all baseline names
   */
  getBaselines(): string[] {
    const baselines: string[] = []
    try {
      const files = require("fs").readdirSync(this.config.baselineDir)
      for (const file of files) {
        if (file.endsWith(".json")) {
          baselines.push(file.slice(0, -5))
        }
      }
    } catch {
      // Directory might not exist yet
    }
    return baselines
  }
}

/**
 * Create a visual tester with default config
 */
export function createVisualTester(config?: Partial<VisualTestConfig>): VisualTester {
  return new VisualTester(config)
}

/**
 * Assert visual regression - convenience function
 */
export async function assertVisual(
  output: string,
  testName: string,
  metadata?: {
    description?: string
    tags?: string[]
    rows?: number
    cols?: number
  },
  config?: Partial<VisualTestConfig>,
): Promise<void> {
  const tester = new VisualTester(config)
  const snapshot = tester.captureSnapshot(output, {
    testName,
    ...metadata,
  })
  await tester.assertMatches(snapshot, testName)
}

/**
 * Update visual baseline - convenience function
 */
export function updateVisualBaseline(
  output: string,
  testName: string,
  metadata?: {
    description?: string
    tags?: string[]
    rows?: number
    cols?: number
  },
  config?: Partial<VisualTestConfig>,
): void {
  const tester = new VisualTester(config)
  const snapshot = tester.captureSnapshot(output, {
    testName,
    ...metadata,
  })
  tester.updateBaseline(snapshot, testName)
}
