import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { ContextManager, createExecutionContext } from "./context"
import { CheckpointManager, createCheckpointManager } from "./checkpoint"
import { AutonomousState } from "../state/states"
import { AgentInvoker } from "./agent-invoker"
import { TestRunner } from "./test-runner"
import type { SafetyIntegration } from "../safety/integration"

const log = Log.create({ service: "autonomous.executor" })

/**
 * Executor configuration
 */
export interface ExecutionConfig {
  unattended: boolean
  maxRetries: number
  checkpointInterval: number
  safetyIntegration?: SafetyIntegration
}

/**
 * TDD phase
 */
export type TDDPhase = "red" | "green" | "refactor"

/**
 * Test result
 */
export interface TestResult {
  success: boolean
  passed: number
  failed: number
  skipped: number
  duration: number
  errors: string[]
}

/**
 * Verification result
 */
export interface VerificationResult {
  success: boolean
  typecheck: boolean
  lint: boolean
  coverage: number
  issues: string[]
}

/**
 * TDD cycle result
 */
export interface TDDCycleResult {
  phase: TDDPhase
  success: boolean
  duration: number
  changes: string[]
}

/**
 * Executor for running TDD cycles
 *
 * Manages the Test-Driven Development workflow
 */
export class Executor {
  private context: ContextManager
  private checkpointManager: CheckpointManager
  private config: ExecutionConfig
  private sessionId: string
  private currentCycle = 0
  private safetyIntegration?: SafetyIntegration

  constructor(sessionId: string, config: ExecutionConfig) {
    this.sessionId = sessionId
    this.config = config
    this.safetyIntegration = config.safetyIntegration
    this.context = createExecutionContext({ sessionId, requestId: `exec_${sessionId}` })
    this.checkpointManager = createCheckpointManager(sessionId)
  }

  /**
   * Initialize the executor
   */
  async initialize(): Promise<void> {
    await this.context.setMetadata("initialized", true)
    log.info("Executor initialized", { sessionId: this.sessionId })
  }

  /**
   * Run a complete TDD cycle
   */
  async runTDDCycle(): Promise<TDDCycleResult[]> {
    const results: TDDCycleResult[] = []
    this.currentCycle++

    log.info("Starting TDD cycle", { cycle: this.currentCycle })

    await Bus.publish(AutonomousEvent.TDDCycleStarted, {
      sessionId: this.sessionId,
      cycleId: `${this.sessionId}_${this.currentCycle}`,
      phase: "red",
    })

    // Phase 1: RED - Write failing test
    const redResult = await this.runRedPhase()
    results.push(redResult)

    if (!redResult.success) {
      return results
    }

    // Phase 2: GREEN - Make test pass
    await Bus.publish(AutonomousEvent.TDDCycleStarted, {
      sessionId: this.sessionId,
      cycleId: `${this.sessionId}_${this.currentCycle}`,
      phase: "green",
    })

    const greenResult = await this.runGreenPhase()
    results.push(greenResult)

    if (!greenResult.success) {
      return results
    }

    // Phase 3: REFACTOR - Improve code
    await Bus.publish(AutonomousEvent.TDDCycleStarted, {
      sessionId: this.sessionId,
      cycleId: `${this.sessionId}_${this.currentCycle}`,
      phase: "refactor",
    })

    const refactorResult = await this.runRefactorPhase()
    results.push(refactorResult)

    await Bus.publish(AutonomousEvent.TDDCycleCompleted, {
      sessionId: this.sessionId,
      cycleId: `${this.sessionId}_${this.currentCycle}`,
      phase: "refactor",
      success: refactorResult.success,
      duration: results.reduce((sum, r) => sum + r.duration, 0),
    })

    return results
  }

  /**
   * Run RED phase - write failing test
   *
   * Uses tdd-guide agent to generate a failing test
   */
  private async runRedPhase(): Promise<TDDCycleResult> {
    const startTime = Date.now()
    const changes: string[] = []

    await this.context.setPhase("red")
    await this.context.setTask(`red_phase_${this.currentCycle}`)

    log.info("Running RED phase", { cycle: this.currentCycle })

    try {
      // Get current requirement from metadata
      const metadata = this.context.get()
      const requirement = (metadata.requirement as string | undefined) ?? "Implement as feature"

      // Safety check before starting RED phase
      if (this.safetyIntegration) {
        const safetyCheck = await this.safetyIntegration.checkSafety({
          category: "maxActions",
        })
        if (!safetyCheck.safe) {
          log.warn("RED phase blocked by safety check", { reason: safetyCheck.reason })
          return {
            phase: "red",
            success: false,
            duration: Date.now() - startTime,
            changes: [],
          }
        }
      }

      // Invoke tdd-guide agent to write a failing test
      const guidance = await AgentInvoker.tddRed(
        requirement,
        { sessionId: this.sessionId },
      )

      // Extract test file path from guidance
      const testFileMatch = guidance.output.match(/Test File:\s*([^\n]+)/)
      const testFile = testFileMatch?.[1]?.trim() ?? `test_${Date.now()}.test.ts`

      changes.push(testFile)

      // Store test file for tracking
      await this.context.setMetadata("lastTestFile", testFile)

      log.info("RED phase completed", { cycle: this.currentCycle, changes })

      return {
        phase: "red",
        success: guidance.success,
        duration: Date.now() - startTime,
        changes,
      }
    } catch (error) {
      log.error("RED phase failed", {
        cycle: this.currentCycle,
        error: error instanceof Error ? error.message : String(error),
      })

      return {
        phase: "red",
        success: false,
        duration: Date.now() - startTime,
        changes,
      }
    }
  }

  /**
   * Run GREEN phase - make test pass
   *
   * Uses tdd-guide agent to generate minimal implementation
   */
  private async runGreenPhase(): Promise<TDDCycleResult> {
    const startTime = Date.now()
    const changes: string[] = []

    await this.context.setPhase("green")
    await this.context.setTask(`green_phase_${this.currentCycle}`)

    log.info("Running GREEN phase", { cycle: this.currentCycle })

    try {
      // Get test file from RED phase
      const metadata = this.context.get()
      const testFile = metadata.lastTestFile as string | undefined ?? "test.test.ts"

      // Invoke tdd-guide agent for GREEN phase
      const guidance = await AgentInvoker.tddGreen(
        testFile,
        "Test is currently failing",
        { sessionId: this.sessionId },
      )

      // Extract implementation code from guidance
      const implMatch = guidance.output.match(/Implementation File:\s*([^\n]+)/)
      const implFile = implMatch?.[1]?.trim() ?? `impl_${Date.now()}.ts`

      changes.push(implFile)

      // Track implementation file and update filesModified
      await this.context.setMetadata("lastImplFile", implFile)
      // Track implementation file and update filesModified
      await this.context.addFile(implFile)

      log.info("GREEN phase completed", { cycle: this.currentCycle, changes })

      return {
        phase: "green",
        success: guidance.success,
        duration: Date.now() - startTime,
        changes,
      }
    } catch (error) {
      log.error("GREEN phase failed", {
        cycle: this.currentCycle,
        error: error instanceof Error ? error.message : String(error),
      })

      return {
        phase: "green",
        success: false,
        duration: Date.now() - startTime,
        changes,
      }
    }
  }

  /**
   * Run REFACTOR phase - improve code
   *
   * Uses code-reviewer agent to suggest improvements
   */
  private async runRefactorPhase(): Promise<TDDCycleResult> {
    const startTime = Date.now()
    const changes: string[] = []

    await this.context.setPhase("refactor")
    await this.context.setTask(`refactor_phase_${this.currentCycle}`)

    log.info("Running REFACTOR phase", { cycle: this.currentCycle })

    try {
      // Get modified files to review
      const metadata = this.context.get()
      const filesToReview = metadata.filesModified ?? []

      // Invoke code-reviewer agent for refactoring suggestions
      const review = await AgentInvoker.codeReview(
        filesToReview.length > 0 ? filesToReview : ["*.ts"],
        {
          sessionId: this.sessionId,
          focus: ["code quality", "performance", "maintainability"],
        },
      )

      // Extract suggestions from review output
      const suggestionsMatch = review.output.match(/### Suggestions\n([\s\S]+)/s)
      const suggestions = suggestionsMatch?.[1]?.split("\n").filter(Boolean) ?? []

      for (const suggestion of suggestions) {
        const suggestionText = suggestion.trim()
        if (suggestionText.length > 50) {
          changes.push(`${suggestionText.slice(0, 50)}...`)
        } else {
          changes.push(suggestionText)
        }
      }

      log.info("REFACTOR phase completed", {
        cycle: this.currentCycle,
        changes,
        reviewSuccess: review.success,
      })

      // Run tests to ensure refactor doesn't break anything
      const testResult = await this.runTests()

      if (!testResult.success) {
        log.warn("Tests failed after refactor, triggering rollback", {
          cycle: this.currentCycle,
          failed: testResult.failed,
        })

        // Use safety integration for rollback
        if (this.safetyIntegration) {
          await this.safetyIntegration.handleTestFailure({
            failedTests: [], // Could extract from testResult
            totalTests: testResult.passed + testResult.failed + testResult.skipped,
            error: testResult.errors.join("; "),
          })
        }

        return {
          phase: "refactor",
          success: false,
          duration: Date.now() - startTime,
          changes,
        }
      }

      return {
        phase: "refactor",
        success: review.success && testResult.success,
        duration: Date.now() - startTime,
        changes,
      }
    } catch (error) {
      log.error("REFACTOR phase failed", {
        cycle: this.currentCycle,
        error: error instanceof Error ? error.message : String(error),
      })

      return {
        phase: "refactor",
        success: false,
        duration: Date.now() - startTime,
        changes,
      }
    }
  }

  /**
   * Run tests
   */
  async runTests(): Promise<TestResult> {
    log.info("Running tests", { sessionId: this.sessionId })

    await this.context.setPhase("test")

    const result = await TestRunner.runAll()

    // Record statistics
    await this.context.recordTestResults(
      result.passed + result.failed + result.skipped,
      result.passed,
      result.failed,
    )

    log.info("Tests completed", {
      sessionId: this.sessionId,
      passed: result.passed,
      failed: result.failed,
      success: result.success,
    })

    return result
  }

  /**
   * Run verification checks
   */
  async runVerification(): Promise<VerificationResult> {
    const startTime = Date.now()

    log.info("Running verification", { sessionId: this.sessionId })

    await this.context.setPhase("verify")

    try {
      const issues: string[] = []

      // 1. Run TypeScript type checking
      const typecheckResult = await this.runTypecheck()
      if (!typecheckResult.success) {
        issues.push(...typecheckResult.errors)
      }

      // 2. Run linter (prettier check)
      const lintResult = await this.runLint()
      if (!lintResult.success) {
        issues.push(...lintResult.errors)
      }

      // 3. Check test coverage
      const coverageResult = await TestRunner.runCoverage(80)

      const result: VerificationResult = {
        success: issues.length === 0 && coverageResult.passesThreshold,
        typecheck: typecheckResult.success,
        lint: lintResult.success,
        coverage: coverageResult.coverage,
        issues,
      }

      log.info("Verification completed", {
        sessionId: this.sessionId,
        success: result.success,
        typecheck: result.typecheck,
        lint: result.lint,
        coverage: result.coverage,
      })

      return result
    } catch (error) {
      log.error("Verification failed", {
        sessionId: this.sessionId,
        error: error instanceof Error ? error.message : String(error),
      })

      return {
        success: false,
        typecheck: false,
        lint: false,
        coverage: 0,
        issues: [error instanceof Error ? error.message : String(error)],
      }
    }
  }

  /**
   * Run TypeScript type checking
   */
  private async runTypecheck(): Promise<{ success: boolean; errors: string[] }> {
    try {
      const { execSync } = require("child_process")
      const Instance = await import("@/project/instance").then((m) => m.Instance)

      log.info("Running typecheck", { sessionId: this.sessionId })

      const output = execSync("bun", ["run", "typecheck"], {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
      })

      return { success: true, errors: [] }
    } catch (error) {
      const errorOutput = error instanceof Error ? error.message : String(error)
      const errors = errorOutput.split("\n").filter((line) => line.trim().length > 0)

      log.warn("Typecheck failed", {
        sessionId: this.sessionId,
        errors: errors.slice(0, 5),
      })

      return { success: false, errors }
    }
  }

  /**
   * Run linter (prettier check)
   */
  private async runLint(): Promise<{ success: boolean; errors: string[] }> {
    try {
      const { execSync } = require("child_process")
      const Instance = await import("@/project/instance").then((m) => m.Instance)

      log.info("Running linter", { sessionId: this.sessionId })

      // Check if prettier is available
      execSync("bun", ["x", "prettier", "--check", "src/**/*.ts"], {
        cwd: Instance.worktree,
        encoding: "utf-8",
        stdio: ["pipe", "pipe", "pipe"],
      })

      return { success: true, errors: [] }
    } catch (error) {
      const errorOutput = error instanceof Error ? error.message : String(error)
      const errors = errorOutput.split("\n").filter((line) => line.trim().length > 0)

      log.warn("Lint failed", {
        sessionId: this.sessionId,
        errors: errors.slice(0, 5),
      })

      return { success: false, errors }
    }
  }

  /**
   * Create a checkpoint
   */
  async createCheckpoint(type: "git" | "state" | "manual" = "state", reason = ""): Promise<string> {
    return this.checkpointManager.create(type, reason)
  }

  /**
   * Rollback to a checkpoint
   */
  async rollback(checkpointId: string, reason = ""): Promise<boolean> {
    return this.checkpointManager.restore(checkpointId, reason)
  }

  /**
   * Get execution context
   */
  getContext(): ContextManager {
    return this.context
  }

  /**
   * Get execution statistics
   */
  getStats(): {
    cyclesCompleted: number
    testsRun: number
    testsPassed: number
    testsFailed: number
    passRate: number
  } {
    const testStats = this.context.getTestStats()

    return {
      cyclesCompleted: this.currentCycle,
      testsRun: testStats.run,
      testsPassed: testStats.passed,
      testsFailed: testStats.failed,
      passRate: testStats.passRate,
    }
  }

  /**
   * Cleanup
   */
  async cleanup(): Promise<void> {
    await this.context.clear()
    log.info("Executor cleaned up", { sessionId: this.sessionId })
  }
}

/**
 * Create an executor
 */
export function createExecutor(sessionId: string, config?: Partial<ExecutionConfig>): Executor {
  return new Executor(sessionId, {
    unattended: false,
    maxRetries: 2,
    checkpointInterval: 5,
    ...config,
  })
}
