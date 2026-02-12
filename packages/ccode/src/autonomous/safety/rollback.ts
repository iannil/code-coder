import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { CheckpointManager, type Checkpoint } from "../execution/checkpoint"
import { AutonomousState } from "../state/states"

const log = Log.create({ service: "autonomous.safety.rollback" })

/**
 * Rollback trigger types
 */
export type RollbackTrigger =
  | "test_failure"
  | "verification_failure"
  | "resource_exceeded"
  | "loop_detected"
  | "user_request"
  | "critical_error"

/**
 * Rollback options
 */
export interface RollbackOptions {
  createCheckpoint: boolean
  clearAfterRollback: boolean
  maxRetries: number
  retryDelay: number
}

/**
 * Rollback result
 */
export interface RollbackResult {
  success: boolean
  checkpointId?: string
  reason: string
  filesRestored: string[]
  retryPossible: boolean
}

/**
 * Default rollback options
 */
const DEFAULT_OPTIONS: RollbackOptions = {
  createCheckpoint: true,
  clearAfterRollback: false,
  maxRetries: 2,
  retryDelay: 1000,
}

/**
 * Rollback manager
 *
 * Handles automatic rollback on failures
 */
export class RollbackManager {
  private checkpointManager: CheckpointManager
  private options: RollbackOptions
  private sessionId: string
  private rollbackCount: number = 0
  private lastRollbackTime: number = 0

  constructor(sessionId: string, options: Partial<RollbackOptions> = {}) {
    this.sessionId = sessionId
    this.options = { ...DEFAULT_OPTIONS, ...options }
    this.checkpointManager = new CheckpointManager(sessionId)
  }

  /**
   * Initialize rollback manager
   */
  async initialize(): Promise<void> {
    // Load existing checkpoints
    await this.checkpointManager.load()
  }

  /**
   * Create a checkpoint before potentially dangerous operations
   */
  async createCheckpoint(reason = "Pre-operation checkpoint"): Promise<string | undefined> {
    if (!this.options.createCheckpoint) {
      return undefined
    }

    try {
      const checkpointId = await this.checkpointManager.create("state", reason)
      log.info("Pre-operation checkpoint created", { checkpointId, reason })
      return checkpointId
    } catch (error) {
      log.error("Failed to create checkpoint", {
        error: error instanceof Error ? error.message : String(error),
      })
      return undefined
    }
  }

  /**
   * Execute with automatic rollback on failure
   */
  async withRollback<T>(
    operation: () => Promise<T>,
    trigger: RollbackTrigger,
    context?: {
      operationName?: string
      metadata?: Record<string, unknown>
    },
  ): Promise<{
    success: boolean
    result?: T
    rollback?: RollbackResult
  }> {
    const checkpointId = await this.createCheckpoint(
      context?.operationName ?? `Before ${trigger} operation`,
    )

    try {
      const result = await operation()

      return { success: true, result }
    } catch (error) {
      log.warn("Operation failed, initiating rollback", {
        trigger,
        error: error instanceof Error ? error.message : String(error),
        checkpointId,
      })

      const rollback = await this.performRollback(checkpointId, trigger, error instanceof Error ? error.message : String(error))

      return {
        success: false,
        rollback,
      }
    }
  }

  /**
   * Perform a rollback
   */
  async performRollback(
    checkpointId: string | undefined,
    trigger: RollbackTrigger,
    reason: string,
  ): Promise<RollbackResult> {
    // Rate limit rollbacks
    const now = Date.now()
    if (now - this.lastRollbackTime < this.options.retryDelay) {
      log.warn("Rollback rate limited", {
        timeSinceLast: now - this.lastRollbackTime,
        minDelay: this.options.retryDelay,
      })
    }

    this.lastRollbackTime = now

    if (!checkpointId) {
      return {
        success: false,
        reason: "No checkpoint available for rollback",
        filesRestored: [],
        retryPossible: false,
      }
    }

    log.info("Performing rollback", { checkpointId, trigger, reason })

    // Get checkpoint info
    const checkpoint = this.checkpointManager.get(checkpointId)

    // Attempt rollback
    const success = await this.checkpointManager.restore(checkpointId, `${trigger}: ${reason}`)

    if (success) {
      this.rollbackCount++

      // Publish event
      await Bus.publish(AutonomousEvent.RollbackPerformed, {
        sessionId: this.sessionId,
        checkpointId,
        reason: `${trigger}: ${reason}`,
        success: true,
      })

      return {
        success: true,
        checkpointId,
        reason: `Rollback successful: ${reason}`,
        filesRestored: checkpoint?.files ?? [],
        retryPossible: this.rollbackCount < this.options.maxRetries,
      }
    }

    // Rollback failed
    await Bus.publish(AutonomousEvent.RollbackPerformed, {
      sessionId: this.sessionId,
      checkpointId,
      reason: `Rollback failed: ${reason}`,
      success: false,
    })

    return {
      success: false,
      checkpointId,
      reason: `Rollback failed: ${reason}`,
      filesRestored: [],
      retryPossible: false,
    }
  }

  /**
   * Handle test failure with potential rollback
   */
  async handleTestFailure(failureInfo: {
    failedTests: string[]
    totalTests: number
    error: string
  }): Promise<RollbackResult | undefined> {
    const { failedTests, totalTests, error } = failureInfo
    const failureRate = failedTests.length / totalTests

    // Only rollback if failure rate is high (>50%)
    if (failureRate > 0.5) {
      const latest = this.checkpointManager.getLatest()

      if (latest) {
        log.warn("High test failure rate, initiating rollback", {
          failureRate,
          failedTests: failedTests.length,
          totalTests,
        })

        return this.performRollback(latest.id, "test_failure", error)
      }
    }

    return undefined
  }

  /**
   * Handle verification failure
   */
  async handleVerificationFailure(failureInfo: {
    typecheckFailed: boolean
    lintFailed: boolean
    coverageTooLow: boolean
    issues: string[]
  }): Promise<RollbackResult | undefined> {
    // Rollback on critical failures
    if (failureInfo.typecheckFailed) {
      const latest = this.checkpointManager.getLatest()

      if (latest) {
        log.warn("Typecheck failed, initiating rollback", {
          issues: failureInfo.issues,
        })

        return this.performRollback(latest.id, "verification_failure", "Typecheck failed")
      }
    }

    return undefined
  }

  /**
   * Handle resource exceeded
   */
  async handleResourceExceeded(resource: string, current: number, limit: number): Promise<RollbackResult | undefined> {
    // For resource limits, we typically pause rather than rollback
    // But if we've exhausted budget, rollback to last checkpoint might help

    if (current >= limit) {
      const latest = this.checkpointManager.getLatest()

      if (latest) {
        log.warn("Resource limit exceeded, initiating rollback", {
          resource,
          current,
          limit,
        })

        return this.performRollback(
          latest.id,
          "resource_exceeded",
          `${resource} exceeded: ${current}/${limit}`,
        )
      }
    }

    return undefined
  }

  /**
   * Handle loop detection
   */
  async handleLoopDetected(loopInfo: {
    type: "state" | "tool" | "decision"
    pattern: unknown[]
    count: number
  }): Promise<RollbackResult | undefined> {
    // For loops, rollback to break the cycle
    const latest = this.checkpointManager.getLatest()

    if (latest) {
      log.warn("Loop detected, initiating rollback", {
        type: loopInfo.type,
        pattern: loopInfo.pattern,
        count: loopInfo.count,
      })

      return this.performRollback(
        latest.id,
        "loop_detected",
        `${loopInfo.type} loop detected: ${JSON.stringify(loopInfo.pattern)}`,
      )
    }

    return undefined
  }

  /**
   * Get rollback count
   */
  getRollbackCount(): number {
    return this.rollbackCount
  }

  /**
   * Check if retry is possible
   */
  canRetry(): boolean {
    return this.rollbackCount < this.options.maxRetries
  }

  /**
   * Reset rollback count
   */
  resetCount(): void {
    this.rollbackCount = 0
  }

  /**
   * Clear all checkpoints
   */
  async clearCheckpoints(): Promise<void> {
    await this.checkpointManager.clear()
  }

  /**
   * Get checkpoint manager
   */
  getCheckpointManager(): CheckpointManager {
    return this.checkpointManager
  }

  /**
   * Serialize
   */
  serialize(): {
    rollbackCount: number
    options: RollbackOptions
    checkpoints: ReturnType<CheckpointManager["serialize"]>
  } {
    return {
      rollbackCount: this.rollbackCount,
      options: this.options,
      checkpoints: this.checkpointManager.serialize(),
    }
  }

  /**
   * Deserialize
   */
  static deserialize(
    data: {
      rollbackCount: number
      options: RollbackOptions
      checkpoints: { checkpoints: Checkpoint[] }
    },
    sessionId: string,
  ): RollbackManager {
    const manager = new RollbackManager(sessionId, data.options)
    manager.rollbackCount = data.rollbackCount
    manager.checkpointManager = CheckpointManager.deserialize(data.checkpoints, sessionId)
    return manager
  }
}

/**
 * Create a rollback manager
 */
export function createRollbackManager(sessionId: string, options?: Partial<RollbackOptions>): RollbackManager {
  return new RollbackManager(sessionId, options)
}
