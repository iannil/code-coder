/**
 * Unified PDCA Controller
 *
 * Central controller that orchestrates the PDCA (Plan-Do-Check-Act) cycle
 * for all task types. Uses task-type-specific strategies for Check and Act phases.
 */

import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import type { TaskType } from "../classification/types"
import type {
  TaskExecutionResult,
  PDCACheckResult,
  PDCAActResult,
  PDCACycleResult,
  PDCAConfig,
  DEFAULT_PDCA_CONFIG,
} from "./types"
import { StrategyFactory, type AcceptanceStrategy } from "./strategies"

const log = Log.create({ service: "pdca.controller" })

// ============================================================================
// PDCA Controller Configuration
// ============================================================================

export interface PDCAControllerOptions {
  /** Task type to use for strategy selection */
  taskType: TaskType
  /** Session ID for tracking */
  sessionId: string
  /** Maximum PDCA cycles (default: 3) */
  maxCycles?: number
  /** CLOSE score threshold to pass (default: 6.0) */
  passThreshold?: number
  /** CLOSE score threshold for fix vs rework (default: 4.0) */
  fixThreshold?: number
  /** Enable automatic fixing (default: true) */
  enableFix?: boolean
  /** Enable learning from fixes (default: true) */
  enableLearning?: boolean
  /** Working directory for file operations */
  workingDir?: string
}

// ============================================================================
// Unified PDCA Controller
// ============================================================================

/**
 * UnifiedPDCAController orchestrates the PDCA cycle for any task type.
 *
 * Usage:
 * ```typescript
 * const pdca = new UnifiedPDCAController({
 *   taskType: "research",
 *   sessionId: "session-123",
 * })
 *
 * const result = await pdca.execute(
 *   async () => {
 *     // Do phase: execute the task
 *     const research = await executeResearch()
 *     return {
 *       taskType: "research",
 *       success: true,
 *       output: research,
 *       durationMs: 1000,
 *     }
 *   },
 *   "原始用户请求"
 * )
 * ```
 */
export class UnifiedPDCAController<TOutput = unknown> {
  private readonly strategy: AcceptanceStrategy<TOutput>
  private readonly config: PDCAConfig
  private cycles = 0
  private readonly startTime: number

  constructor(options: PDCAControllerOptions) {
    this.strategy = StrategyFactory.create(options.taskType) as AcceptanceStrategy<TOutput>
    this.config = {
      taskType: options.taskType,
      sessionId: options.sessionId,
      maxCycles: options.maxCycles ?? 3,
      passThreshold: options.passThreshold ?? 6.0,
      fixThreshold: options.fixThreshold ?? 4.0,
      enableFix: options.enableFix ?? true,
      enableLearning: options.enableLearning ?? true,
    }
    this.startTime = Date.now()

    log.debug("PDCA controller initialized", {
      taskType: options.taskType,
      sessionId: options.sessionId,
      strategy: this.strategy.name,
    })
  }

  /**
   * Execute the complete PDCA cycle.
   *
   * @param doFn - Function that executes the Do phase (task execution)
   * @param originalRequest - The original user request for context
   * @returns Complete cycle result
   */
  async execute(
    doFn: () => Promise<TaskExecutionResult<TOutput>>,
    originalRequest: string,
  ): Promise<PDCACycleResult<TOutput>> {
    // Publish PDCA cycle started event
    await this.publishPDCAStarted(originalRequest)

    let lastDoResult: TaskExecutionResult<TOutput> | undefined
    let lastCheckResult: PDCACheckResult | undefined
    let lastActResult: PDCAActResult | undefined

    while (this.cycles < this.config.maxCycles) {
      this.cycles++

      log.debug("PDCA cycle starting", {
        sessionId: this.config.sessionId,
        cycle: this.cycles,
        maxCycles: this.config.maxCycles,
      })

      // Publish cycle started event
      await this.publishCycleStarted()

      try {
        // =====================================================================
        // DO Phase
        // =====================================================================
        await this.publishPhaseChanged("do")
        lastDoResult = await doFn()

        log.debug("Do phase completed", {
          sessionId: this.config.sessionId,
          cycle: this.cycles,
          success: lastDoResult.success,
          durationMs: lastDoResult.durationMs,
        })

        // =====================================================================
        // CHECK Phase
        // =====================================================================
        await this.publishPhaseChanged("check")
        lastCheckResult = await this.strategy.check(
          lastDoResult,
          originalRequest,
          this.config,
        )

        log.debug("Check phase completed", {
          sessionId: this.config.sessionId,
          cycle: this.cycles,
          passed: lastCheckResult.passed,
          recommendation: lastCheckResult.recommendation,
          closeScore: lastCheckResult.closeScore.total,
          issueCount: lastCheckResult.issues.length,
        })

        // Publish check result
        await this.publishCheckCompleted(lastCheckResult)

        // If passed, we're done!
        if (lastCheckResult.passed) {
          const totalDurationMs = Date.now() - this.startTime
          await this.publishPDCACompleted(true, this.cycles, totalDurationMs)

          return {
            success: true,
            result: lastDoResult,
            checkResult: lastCheckResult,
            cycles: this.cycles,
            totalDurationMs,
          }
        }

        // If recommendation is rework, don't try to fix
        if (lastCheckResult.recommendation === "rework") {
          log.info("Check recommends rework - stopping PDCA", {
            sessionId: this.config.sessionId,
            cycle: this.cycles,
            closeScore: lastCheckResult.closeScore.total,
          })

          const totalDurationMs = Date.now() - this.startTime
          await this.publishPDCACompleted(false, this.cycles, totalDurationMs, "rework_recommended")

          return {
            success: false,
            result: lastDoResult,
            checkResult: lastCheckResult,
            cycles: this.cycles,
            reason: "rework_recommended",
            totalDurationMs,
          }
        }

        // =====================================================================
        // ACT Phase (Fix)
        // =====================================================================
        if (this.config.enableFix && lastCheckResult.recommendation === "fix") {
          await this.publishPhaseChanged("act")
          lastActResult = await this.strategy.fix(
            lastCheckResult.issues,
            lastDoResult,
            this.config,
          )

          log.debug("Act phase completed", {
            sessionId: this.config.sessionId,
            cycle: this.cycles,
            fixed: lastActResult.fixed,
            fixedCount: lastActResult.fixedIssues.length,
            remainingCount: lastActResult.remainingIssues.length,
            shouldRecheck: lastActResult.shouldRecheck,
          })

          // Publish act result
          await this.publishActCompleted(lastActResult)

          // If no fixes were made or we shouldn't recheck, stop
          if (!lastActResult.shouldRecheck) {
            log.info("Act phase indicates no recheck needed", {
              sessionId: this.config.sessionId,
              cycle: this.cycles,
            })

            const totalDurationMs = Date.now() - this.startTime
            const success = lastActResult.fixed || lastCheckResult.closeScore.total >= this.config.passThreshold

            await this.publishPDCACompleted(success, this.cycles, totalDurationMs)

            return {
              success,
              result: lastDoResult,
              checkResult: lastCheckResult,
              actResult: lastActResult,
              cycles: this.cycles,
              reason: success ? undefined : "fix_incomplete",
              totalDurationMs,
            }
          }
        }

        // Publish cycle completed
        await this.publishCycleCompleted(false)

      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)
        log.error("PDCA cycle error", {
          sessionId: this.config.sessionId,
          cycle: this.cycles,
          error: errorMsg,
        })

        const totalDurationMs = Date.now() - this.startTime
        await this.publishPDCACompleted(false, this.cycles, totalDurationMs, errorMsg)

        return {
          success: false,
          result: lastDoResult,
          checkResult: lastCheckResult,
          actResult: lastActResult,
          cycles: this.cycles,
          reason: errorMsg,
          totalDurationMs,
        }
      }
    }

    // Max cycles exceeded
    log.warn("Max PDCA cycles exceeded", {
      sessionId: this.config.sessionId,
      cycles: this.cycles,
    })

    const totalDurationMs = Date.now() - this.startTime
    await this.publishPDCACompleted(false, this.cycles, totalDurationMs, "max_cycles_exceeded")

    return {
      success: false,
      result: lastDoResult,
      checkResult: lastCheckResult,
      actResult: lastActResult,
      cycles: this.cycles,
      reason: "max_cycles_exceeded",
      totalDurationMs,
    }
  }

  /**
   * Get the current cycle count.
   */
  getCycles(): number {
    return this.cycles
  }

  /**
   * Get the strategy being used.
   */
  getStrategy(): AcceptanceStrategy<TOutput> {
    return this.strategy
  }

  /**
   * Get the configuration.
   */
  getConfig(): PDCAConfig {
    return { ...this.config }
  }

  // ==========================================================================
  // Event Publishing
  // ==========================================================================

  private async publishPDCAStarted(originalRequest: string): Promise<void> {
    // Use existing session started event
    await Bus.publish(AutonomousEvent.SessionStarted, {
      sessionId: this.config.sessionId,
      requestId: `pdca-${this.config.sessionId}`,
      autonomyLevel: "wild",
      config: {
        taskType: this.config.taskType,
        maxCycles: this.config.maxCycles,
        passThreshold: this.config.passThreshold,
        strategy: this.strategy.name,
      },
    })
  }

  private async publishCycleStarted(): Promise<void> {
    await Bus.publish(AutonomousEvent.IterationStarted, {
      sessionId: this.config.sessionId,
      iteration: this.cycles,
      remainingRequirements: 0,
      context: {
        phase: "pdca_cycle",
        taskType: this.config.taskType,
      },
    })
  }

  private async publishCycleCompleted(success: boolean): Promise<void> {
    await Bus.publish(AutonomousEvent.IterationCompleted, {
      sessionId: this.config.sessionId,
      iteration: this.cycles,
      completedRequirements: success ? 1 : 0,
      success,
      duration: Date.now() - this.startTime,
    })
  }

  private async publishPhaseChanged(phase: "do" | "check" | "act"): Promise<void> {
    await Bus.publish(AutonomousEvent.PhaseStarted, {
      sessionId: this.config.sessionId,
      phase: `pdca_${phase}`,
      metadata: {
        cycle: this.cycles,
        taskType: this.config.taskType,
      },
    })
  }

  private async publishCheckCompleted(checkResult: PDCACheckResult): Promise<void> {
    await Bus.publish(AutonomousEvent.AcceptanceCompleted, {
      sessionId: this.config.sessionId,
      success: checkResult.passed,
      overallScore: checkResult.closeScore.total,
      issueCount: checkResult.issues.length,
      recommendation: checkResult.recommendation,
      durationMs: checkResult.durationMs,
    })

    // Also publish individual issues
    for (const issue of checkResult.issues) {
      await Bus.publish(AutonomousEvent.AcceptanceIssueFound, {
        sessionId: this.config.sessionId,
        issueId: issue.id,
        type: issue.category as "test" | "type" | "lint" | "security" | "requirement" | "expectation",
        severity: issue.severity,
        description: issue.description,
        location: issue.location,
      })
    }
  }

  private async publishActCompleted(actResult: PDCAActResult): Promise<void> {
    await Bus.publish(AutonomousEvent.FixCompleted, {
      sessionId: this.config.sessionId,
      success: actResult.fixed,
      fixedCount: actResult.fixedIssues.length,
      remainingCount: actResult.remainingIssues.length,
      shouldRecheck: actResult.shouldRecheck,
      durationMs: actResult.durationMs,
    })
  }

  private async publishPDCACompleted(
    success: boolean,
    cycles: number,
    durationMs: number,
    reason?: string,
  ): Promise<void> {
    await Bus.publish(AutonomousEvent.SessionCompleted, {
      sessionId: this.config.sessionId,
      requestId: `pdca-${this.config.sessionId}`,
      result: {
        success,
        qualityScore: 0, // Would be filled from check result
        crazinessScore: 0,
        duration: durationMs,
        tokensUsed: 0,
        costUSD: 0,
      },
    })
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a new PDCA controller with the given options.
 */
export function createPDCAController<TOutput = unknown>(
  options: PDCAControllerOptions,
): UnifiedPDCAController<TOutput> {
  return new UnifiedPDCAController<TOutput>(options)
}
