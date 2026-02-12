/**
 * Coverage Matrix
 *
 * Builds and manages requirement-test traceability matrix.
 * Maps requirements to test cases for coverage analysis.
 */

import { Log } from "@/util/log"
import type { MatrixEntry } from "../schema/verification-result"
import type { VerificationStatus, Evidence } from "../schema/verification-result"
import type { FunctionalGoal } from "../schema/functional-goal"

const log = Log.create({ service: "verifier.coverage.matrix" })

/**
 * Matrix cell - intersection of requirement and test
 */
export interface MatrixCell {
  requirementId: string
  testId: string
  status: VerificationStatus
  evidence?: Evidence[]
}

/**
 * Coverage matrix state
 */
export class CoverageMatrix {
  private sessionId: string
  private cells: Map<string, MatrixCell> = new Map()
  private goals: Map<string, FunctionalGoal> = new Map()

  constructor(sessionId: string) {
    this.sessionId = sessionId
  }

  /**
   * Add a functional goal to the matrix
   */
  addGoal(goal: FunctionalGoal): void {
    this.goals.set(goal.id, goal)

    log.debug("Added goal to matrix", {
      sessionId: this.sessionId,
      goalId: goal.id,
    })
  }

  /**
   * Link a test to a requirement
   */
  linkTest(requirementId: string, testId: string, status: VerificationStatus = "pass"): void {
    const key = this.makeKey(requirementId, testId)

    this.cells.set(key, {
      requirementId,
      testId,
      status,
    })

    log.debug("Linked test to requirement", {
      sessionId: this.sessionId,
      requirementId,
      testId,
      status,
    })
  }

  /**
   * Update test status
   */
  updateTestStatus(
    requirementId: string,
    testId: string,
    status: VerificationStatus,
    evidence?: Evidence[],
  ): void {
    const key = this.makeKey(requirementId, testId)

    const existing = this.cells.get(key)
    if (existing) {
      this.cells.set(key, {
        ...existing,
        status,
        evidence: evidence ?? existing.evidence,
      })
    } else {
      this.cells.set(key, {
        requirementId,
        testId,
        status,
        evidence,
      })
    }
  }

  /**
   * Get tests for a requirement
   */
  getTestsForRequirement(requirementId: string): string[] {
    const tests: string[] = []

    for (const [key, cell] of this.cells) {
      if (cell.requirementId === requirementId) {
        tests.push(cell.testId)
      }
    }

    return tests
  }

  /**
   * Get requirements for a test
   */
  getRequirementsForTest(testId: string): string[] {
    const requirements: string[] = []

    for (const [key, cell] of this.cells) {
      if (cell.testId === testId) {
        requirements.push(cell.requirementId)
      }
    }

    return requirements
  }

  /**
   * Generate matrix entries for all goals
   */
  generateEntries(): MatrixEntry[] {
    const entries: MatrixEntry[] = []

    for (const [goalId, goal] of this.goals) {
      const testCases = this.getTestsForRequirement(goalId)

      // Determine coverage status
      let coverage: "none" | "partial" | "full" = "none"
      let status: VerificationStatus = "skip"

      if (testCases.length === 0) {
        coverage = "none"
        status = "skip"
      } else {
        const allTests = testCases.map((t) => this.cells.get(this.makeKey(goalId, t)))
        const passCount = allTests.filter((t) => t?.status === "pass").length
        const failCount = allTests.filter((t) => t?.status === "fail").length

        if (passCount === allTests.length) {
          coverage = "full"
          status = "pass"
        } else if (failCount === allTests.length) {
          coverage = "partial"
          status = "fail"
        } else {
          coverage = "partial"
          status = "warn"
        }
      }

      entries.push({
        requirementId: goalId,
        testCases,
        status,
        coverage,
      })
    }

    return entries
  }

  /**
   * Find uncovered requirements
   */
  getUncoveredRequirements(): string[] {
    const entries = this.generateEntries()

    return entries
      .filter((e) => e.coverage === "none" || e.testCases.length === 0)
      .map((e) => e.requirementId)
  }

  /**
   * Find partially covered requirements
   */
  getPartiallyCoveredRequirements(): string[] {
    const entries = this.generateEntries()

    return entries
      .filter((e) => e.coverage === "partial")
      .map((e) => e.requirementId)
  }

  /**
   * Get coverage statistics
   */
  getStats(): {
    totalRequirements: number
    covered: number
    partiallyCovered: number
    uncovered: number
    coveragePercentage: number
  } {
    const entries = this.generateEntries()

    const totalRequirements = entries.length
    const covered = entries.filter((e) => e.coverage === "full").length
    const partiallyCovered = entries.filter((e) => e.coverage === "partial").length
    const uncovered = entries.filter((e) => e.coverage === "none").length

    // Weighted coverage: full=1, partial=0.5, none=0
    const weightedCoverage = covered + partiallyCovered * 0.5
    const coveragePercentage =
      totalRequirements > 0 ? (weightedCoverage / totalRequirements) * 100 : 0

    return {
      totalRequirements,
      covered,
      partiallyCovered,
      uncovered,
      coveragePercentage,
    }
  }

  /**
   * Generate matrix table as Markdown
   */
  toMarkdown(): string {
    const entries = this.generateEntries()
    const stats = this.getStats()

    let md = `# Requirement-Test Coverage Matrix

> Session: ${this.sessionId}
> Generated: ${new Date().toISOString()}

## Summary

| Metric | Value |
|--------|-------|
| Total Requirements | ${stats.totalRequirements} |
| Fully Covered | ${stats.covered} |
| Partially Covered | ${stats.partiallyCovered} |
| Uncovered | ${stats.uncovered} |
| Coverage % | ${stats.coveragePercentage.toFixed(1)}% |

## Matrix

| Requirement | Tests | Status | Coverage |
|-------------|--------|--------|----------|
`

    for (const entry of entries) {
      const tests = entry.testCases.length > 0 ? entry.testCases.join(", ") : "None"
      const statusEmoji =
        entry.status === "pass" ? "✅" : entry.status === "fail" ? "❌" : entry.status === "warn" ? "⚠️" : "⏭️"
      const coverageBadge =
        entry.coverage === "full" ? "Full" : entry.coverage === "partial" ? "Partial" : "None"

      md += `| ${entry.requirementId} | ${tests} | ${statusEmoji} ${entry.status} | ${coverageBadge} |\n`
    }

    if (stats.uncovered > 0) {
      md += `\n## Uncovered Requirements\n\n`
      for (const entry of entries) {
        if (entry.coverage === "none") {
          md += `- ${entry.requirementId}: No tests found\n`
        }
      }
    }

    return md
  }

  /**
   * Make a unique key for requirement-test pair
   */
  private makeKey(requirementId: string, testId: string): string {
    return `${requirementId}:${testId}`
  }

  /**
   * Clear matrix data
   */
  clear(): void {
    this.cells.clear()
    this.goals.clear()
  }
}

/**
 * Create a coverage matrix
 */
export function createCoverageMatrix(sessionId: string): CoverageMatrix {
  return new CoverageMatrix(sessionId)
}

/**
 * Parse requirement IDs from test files
 */
export async function extractRequirementsFromTests(
  testFilePaths: string[],
): Promise<Map<string, string[]>> {
  const fs = require("fs")
  const testToRequirements = new Map<string, string[]>()

  // Common patterns for linking tests to requirements
  const patterns = [
    /REQ-(\d+[-\w]*)/gi,
    /requirement:\s*([A-Z0-9-]+)/gi,
    /covers:\s*([A-Z0-9-]+)/gi,
    /V-[A-Z0-9]+-[A-Z0-9]+-\d{3}/gi,
  ]

  for (const filePath of testFilePaths) {
    try {
      const content = fs.readFileSync(filePath, "utf-8")
      const requirements = new Set<string>()

      for (const pattern of patterns) {
        let match
        while ((match = pattern.exec(content)) !== null) {
          requirements.add(match[1] || match[0])
        }
      }

      testToRequirements.set(filePath, Array.from(requirements))
    } catch (error) {
      log.error("Failed to extract requirements from test file", {
        filePath,
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  return testToRequirements
}

/**
 * Auto-build matrix from goals and test files
 */
export async function buildMatrix(
  sessionId: string,
  goals: FunctionalGoal[],
  testFilePaths: string[],
): Promise<CoverageMatrix> {
  const matrix = createCoverageMatrix(sessionId)

  // Add all goals
  for (const goal of goals) {
    matrix.addGoal(goal)

    // Link tests from goal's testTrace
    for (const testId of goal.testTrace) {
      matrix.linkTest(goal.id, testId, "pass")
    }
  }

  // Extract additional requirement links from test files
  const testToRequirements = await extractRequirementsFromTests(testFilePaths)

  for (const [testPath, requirements] of testToRequirements) {
    const testId = testPath.split("/").pop() ?? testPath

    for (const reqId of requirements) {
      // Find matching goal
      const matchingGoal = goals.find((g) =>
        g.requirementTrace.includes(reqId) || g.id === reqId,
      )

      if (matchingGoal) {
        matrix.linkTest(matchingGoal.id, testId, "pass")
      }
    }
  }

  return matrix
}
