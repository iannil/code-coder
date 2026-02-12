import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import type { TestResult } from "../execution/executor"

const log = Log.create({ service: "autonomous.execution.test-runner" })

/**
 * Bun test result
 */
interface BunTestResult {
  status: "passed" | "failed" | "skipped"
  file: string
  duration: number
  error?: string
}

/**
 * Bun test output format
 */
interface BunTestOutput {
  ok: boolean
  tests: BunTestResult[]
  stats: {
    passed: number
    failed: number
    skipped: number
  }
}

/**
 * Test runner for Autonomous Mode
 *
 * Executes tests using Bun and parses results
 */
export namespace TestRunner {
  /**
   * Run all tests
   */
  export async function runAll(): Promise<TestResult> {
    return await runFiles([])
  }

  /**
   * Run specific test files
   */
  export async function runFiles(pattern: string[]): Promise<TestResult> {
    const startTime = Date.now()

    try {
      const { execSync } = require("child_process")

      // Build bun test command
      const args = ["test", "--reporter", "json"]
      if (pattern.length > 0) {
        args.push(...pattern)
      }

      log.info("Running tests", { args })

      const output = execSync("bun", args, {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
        env: {
          ...process.env,
          NODE_ENV: "test",
        },
      })

      // Parse JSON output
      const jsonOutput: BunTestOutput = JSON.parse(output)

      const duration = Date.now() - startTime

      const result: TestResult = {
        success: jsonOutput.ok,
        passed: jsonOutput.stats.passed,
        failed: jsonOutput.stats.failed,
        skipped: jsonOutput.stats.skipped,
        duration,
        errors: [],
      }

      // Collect errors from failed tests
      if (!jsonOutput.ok) {
        for (const test of jsonOutput.tests) {
          if (test.status === "failed") {
            result.errors.push(`${test.file}: ${test.error || "Unknown error"}`)
          }
        }
      }

      log.info("Tests completed", {
        success: result.success,
        passed: result.passed,
        failed: result.failed,
        duration,
      })

      // Publish event
      await Bus.publish(AutonomousEvent.TaskCompleted, {
        sessionId: "test-runner",
        taskId: `test-${Date.now()}`,
        success: result.success,
        duration,
        metadata: {
          passed: result.passed,
          failed: result.failed,
        },
      })

      return result
    } catch (error) {
      const duration = Date.now() - startTime

      log.error("Test execution failed", {
        error: error instanceof Error ? error.message : String(error),
      })

      return {
        success: false,
        passed: 0,
        failed: 1,
        skipped: 0,
        duration,
        errors: [error instanceof Error ? error.message : String(error)],
      }
    }
  }

  /**
   * Run tests with coverage
   */
  export async function runCoverage(threshold = 80): Promise<{
    testResult: TestResult
    coverage: number
    passesThreshold: boolean
  }> {
    const startTime = Date.now()

    try {
      const { execSync } = require("child_process")

      log.info("Running tests with coverage", { threshold })

      const testOutput = execSync("bun test --coverage", {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
        env: {
          ...process.env,
          NODE_ENV: "test",
        },
      })

      // Parse test results
      const jsonOutput: BunTestOutput = JSON.parse(testOutput)

      // Get coverage from coverage output
      let coverage = 0
      try {
        const coverageOutput = execSync("bun test --coverage json", {
          cwd: Instance.worktree,
          encoding: "utf-8",
          stdio: ["pipe", "pipe", "pipe"],
          env: {
            ...process.env,
            NODE_ENV: "test",
          },
        }).trim()

        const coverageData = JSON.parse(coverageOutput) as { coverage?: number } | null
        // Parse coverage from bun output (format may vary)
        coverage = (coverageData?.coverage ?? 0)
      } catch {
        // Fallback: try to extract from text output
        const coverageMatch = testOutput.match(/coverage:\s*(\d+\.?\d*)%/)
        if (coverageMatch) {
          coverage = parseFloat(coverageMatch[1])
        }
      }

      const testResult: TestResult = {
        success: jsonOutput.ok,
        passed: jsonOutput.stats.passed,
        failed: jsonOutput.stats.failed,
        skipped: jsonOutput.stats.skipped,
        duration: Date.now() - startTime,
        errors: [],
      }

      const passesThreshold = coverage >= threshold

      log.info("Coverage completed", {
        coverage,
        threshold,
        passes: passesThreshold,
      })

      return {
        testResult,
        coverage,
        passesThreshold,
      }
    } catch (error) {
      log.error("Coverage test execution failed", {
        error: error instanceof Error ? error.message : String(error),
      })

      return {
        testResult: {
          success: false,
          passed: 0,
          failed: 1,
          skipped: 0,
          duration: Date.now() - startTime,
          errors: [error instanceof Error ? error.message : String(error)],
        },
        coverage: 0,
        passesThreshold: false,
      }
    }
  }

  /**
   * Run specific test pattern
   */
  export async function runPattern(pattern: string): Promise<TestResult> {
    return await runFiles([pattern])
  }

  /**
   * Parse test file path from test name
   */
  export function findTestFile(testName: string): string | undefined {
    try {
      const { execSync } = require("child_process")

      // Search for test files containing the test name
      const output = execSync(`grep -r "${testName}" --include="*.test.ts" --include="*.test.js"`, {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
      }).trim()

      if (!output) return undefined

      // Return first match
      return output.split("\n")[0]?.split(":")[0]
    } catch {
      return undefined
    }
  }

  /**
   * Get list of all test files
   */
  export async function listTestFiles(): Promise<string[]> {
    try {
      const { readdirSync, statSync } = require("fs")
      const path = require("path")

      const testFiles: string[] = []

      function scanDir(dir: string) {
        try {
          const entries = readdirSync(dir, { withFileTypes: true })
          for (const entry of entries) {
            const fullPath = path.join(dir, entry.name)

            if (entry.isDirectory()) {
              scanDir(fullPath)
            } else if (
              entry.name.endsWith(".test.ts") ||
              entry.name.endsWith(".test.js") ||
              entry.name.endsWith(".spec.ts") ||
              entry.name.endsWith(".spec.js")
            ) {
              testFiles.push(fullPath)
            }
          }
        } catch {
          // Skip directories we can't read
        }
      }

      scanDir(Instance.worktree)
      return testFiles
    } catch {
      return []
    }
  }

  /**
   * Quick test: Check if a specific test file exists and is valid
   */
  export async function testFileExists(testPath: string): Promise<boolean> {
    try {
      const { existsSync } = require("fs")
      return existsSync(testPath)
    } catch {
      return false
    }
  }
}
