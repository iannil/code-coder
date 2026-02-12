import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { SafetyGuard, type SafetyCheckResult, type ResourceUsage } from "./constraints"
import { SafetyGuardrails, type LoopPattern } from "./guardrails"
import { RollbackManager, type RollbackTrigger, type RollbackOptions } from "./rollback"
import { AutonomousState } from "../state/states"

const log = Log.create({ service: "autonomous.safety.integration" })

/**
 * Destructive operation categories
 */
export type DestructiveCategory =
  | "git_operations"
  | "file_deletion"
  | "file_overwrite"
  | "dependency_change"
  | "database_migration"
  | "configuration_change"
  | "network_request"

/**
 * Destructive operation check
 */
export interface DestructiveOperation {
  category: DestructiveCategory
  description: string
  files?: string[]
  reversible: boolean
  riskLevel: "low" | "medium" | "high" | "critical"
}

/**
 * Safety integration configuration
 */
export interface SafetyIntegrationConfig {
  enableDoomLoopBridge: boolean
  enableDestructiveProtection: boolean
  autoRollbackOnFailure: boolean
  rollbackOptions: Partial<RollbackOptions>
}

/**
 * Default configuration
 */
const DEFAULT_CONFIG: SafetyIntegrationConfig = {
  enableDoomLoopBridge: true,
  enableDestructiveProtection: true,
  autoRollbackOnFailure: true,
  rollbackOptions: {
    createCheckpoint: true,
    maxRetries: 2,
  },
}

/**
 * Safety integration status
 */
export interface SafetyStatus {
  resources: {
    usage: ResourceUsage
    remaining: ReturnType<SafetyGuard["getRemaining"]>
    surplusRatio: number
    warnings: number
  }
  loops: {
    stateLoops: number
    toolLoops: number
    decisionHesitations: number
    loopsBroken: number
  }
  rollbacks: {
    count: number
    canRetry: boolean
  }
  safe: boolean
}

/**
 * Integrated safety system
 *
 * Bridges Autonomous Mode safety layer with existing session DOOM_LOOP detection
 * and provides unified safety checks
 */
export class SafetyIntegration {
  private safetyGuard: SafetyGuard
  private guardrails: SafetyGuardrails
  private rollbackManager: RollbackManager
  private config: SafetyIntegrationConfig
  private sessionId: string
  private destructiveOpsHistory: DestructiveOperation[] = []
  private doomLoopCallbacks: Set<(loop: LoopPattern) => void> = new Set()

  constructor(sessionId: string, config: Partial<SafetyIntegrationConfig> = {}) {
    this.sessionId = sessionId
    this.config = { ...DEFAULT_CONFIG, ...config }

    // Initialize safety components
    this.safetyGuard = new SafetyGuard(sessionId)
    this.guardrails = new SafetyGuardrails(sessionId, {
      loopDetectionEnabled: true,
      autoBreakLoops: true,
    })
    this.rollbackManager = new RollbackManager(sessionId, this.config.rollbackOptions)

    // Setup event handlers
    this.setupEventHandlers()
  }

  /**
   * Initialize safety integration
   */
  async initialize(): Promise<void> {
    await this.rollbackManager.initialize()
    log.info("Safety integration initialized", { sessionId: this.sessionId })
  }

  /**
   * Check if operation is safe
   */
  async checkSafety(operation?: {
    category?: keyof import("./constraints").ResourceBudget
    destructive?: DestructiveOperation
  }): Promise<SafetyCheckResult & {
    destructiveAllowed?: boolean
    rollbackAvailable?: boolean
  }> {
    // Check resource limits
    const resourceCheck = await this.safetyGuard.check(
      operation?.category,
      operation?.destructive ? { filesChanged: 1, actionsPerformed: 1 } : undefined,
    )

    // Check guardrail limits
    const limitsCheck = this.guardrails.checkLimits()

    // Check destructive operations
    let destructiveAllowed = true
    let destructiveReason: string | undefined

    if (operation?.destructive && this.config.enableDestructiveProtection) {
      const destructiveCheck = this.checkDestructiveOperation(operation.destructive)
      destructiveAllowed = destructiveCheck.allowed
      destructiveReason = destructiveCheck.reason
    }

    const safe = resourceCheck.safe &&
      limitsCheck.safe &&
      destructiveAllowed

    return {
      safe,
      reason: resourceCheck.reason ??
        limitsCheck.reason ??
        destructiveReason,
      resource: resourceCheck.resource,
      current: resourceCheck.current,
      limit: resourceCheck.limit,
      destructiveAllowed,
      rollbackAvailable: this.rollbackManager.canRetry(),
    }
  }

  /**
   * Check destructive operation
   */
  checkDestructiveOperation(op: DestructiveOperation): {
    allowed: boolean
    reason?: string
    requiresConfirmation: boolean
  } {
    // High and critical operations always require confirmation
    if (op.riskLevel === "critical" || op.riskLevel === "high") {
      return {
        allowed: false,
        reason: `Critical operation requires confirmation: ${op.description}`,
        requiresConfirmation: true,
      }
    }

    // Check for repeated destructive operations
    const recentOps = this.destructiveOpsHistory.slice(-10)
    const similarOps = recentOps.filter(
      (h) =>
        h.category === op.category &&
        h.description === op.description &&
        h.files?.some((f) => op.files?.includes(f)),
    )

    if (similarOps.length >= 2) {
      return {
        allowed: false,
        reason: `Repeated destructive operation detected: ${op.description}`,
        requiresConfirmation: true,
      }
    }

    // Medium risk operations in unattended mode
    if (op.riskLevel === "medium" && !op.reversible) {
      return {
        allowed: false,
        reason: `Irreversible medium-risk operation: ${op.description}`,
        requiresConfirmation: true,
      }
    }

    return { allowed: true, requiresConfirmation: false }
  }

  /**
   * Record destructive operation
   */
  recordDestructiveOperation(op: DestructiveOperation): void {
    this.destructiveOpsHistory.push({
      ...op,
      description: `${new Date().toISOString()}: ${op.description}`,
    })

    log.info("Destructive operation recorded", {
      category: op.category,
      riskLevel: op.riskLevel,
      reversible: op.reversible,
    })

    // Publish event
    Bus.publish(AutonomousEvent.ResourceWarning, {
      sessionId: this.sessionId,
      resource: "destructive_operation",
      current: this.destructiveOpsHistory.length,
      limit: 100,
      percentage: 0,
    })
  }

  /**
   * Record state transition
   */
  recordStateTransition(from: string, to: string): void {
    this.guardrails.recordStateTransition(from, to)
  }

  /**
   * Record tool call
   */
  recordToolCall(tool: string, input: unknown, result: "success" | "error"): void {
    this.guardrails.recordToolCall(tool, input, result)

    // Bridge to existing DOOM_LOOP detection
    if (this.config.enableDoomLoopBridge && result === "error") {
      this.checkForDoomLoop(tool, input)
    }
  }

  /**
   * Record decision
   */
  recordDecision(id: string, type: string, result: string): void {
    this.guardrails.recordDecision(id, type, result)
  }

  /**
   * Record resource usage
   */
  recordResourceUsage(resource: keyof ResourceUsage, value: number): void {
    this.safetyGuard.record(resource, value)
  }

  /**
   * Handle failure with optional rollback
   */
  async handleFailure(trigger: RollbackTrigger, context: {
    reason: string
    operation?: () => Promise<unknown>
  }): Promise<{
    rolledBack: boolean
    canRetry: boolean
  }> {
    log.warn("Handling failure", { trigger, reason: context.reason })

    let rolledBack = false

    if (this.config.autoRollbackOnFailure) {
      if (context.operation) {
        const result = await this.rollbackManager.withRollback(
          context.operation,
          trigger,
          { operationName: `Recovery from ${trigger}` },
        )
        rolledBack = result.rollback?.success ?? false
      } else {
        const rollbackResult = await this.rollbackManager.performRollback(
          this.rollbackManager.getCheckpointManager().getLatest()?.id,
          trigger,
          context.reason,
        )
        rolledBack = rollbackResult.success
      }
    }

    return {
      rolledBack,
      canRetry: this.rollbackManager.canRetry(),
    }
  }

  /**
   * Handle test failure
   */
  async handleTestFailure(failureInfo: {
    failedTests: string[]
    totalTests: number
    error: string
  }): Promise<boolean> {
    const rollbackResult = await this.rollbackManager.handleTestFailure(failureInfo)
    return rollbackResult?.success ?? false
  }

  /**
   * Handle verification failure
   */
  async handleVerificationFailure(failureInfo: {
    typecheckFailed: boolean
    lintFailed: boolean
    coverageTooLow: boolean
    issues: string[]
  }): Promise<boolean> {
    const rollbackResult = await this.rollbackManager.handleVerificationFailure(failureInfo)
    return rollbackResult?.success ?? false
  }

  /**
   * Bridge to existing DOOM_LOOP detection
   *
   * This integrates with session/processor.ts DOOM_LOOP_THRESHOLD detection
   */
  private checkForDoomLoop(tool: string, input: unknown): void {
    const recentCalls = this.guardrails["toolCalls"] as Array<{
      tool: string
      input: unknown
      result: "success" | "error"
      timestamp: number
    }>

    // Find similar recent error calls
    const matchingCalls = recentCalls.filter(
      (c) =>
        c.tool === tool &&
        c.result === "error" &&
        JSON.stringify(c.input) === JSON.stringify(input),
    )

    if (matchingCalls.length >= 3) {
      const loopPattern: LoopPattern = {
        type: "tool",
        pattern: [tool, input],
        count: matchingCalls.length,
        window: 60000, // 1 minute
      }

      log.warn("DOOM_LOOP detected, notifying callbacks", {
        tool,
        count: matchingCalls.length,
      })

      // Notify registered callbacks
      for (const callback of this.doomLoopCallbacks) {
        try {
          callback(loopPattern)
        } catch (error) {
          log.error("DOOM_LOOP callback error", {
            error: error instanceof Error ? error.message : String(error),
          })
        }
      }

      // Publish event
      Bus.publish(AutonomousEvent.LoopDetected, {
        sessionId: this.sessionId,
        loopType: "tool",
        pattern: [tool, input],
        count: matchingCalls.length,
        broken: true,
      })
    }
  }

  /**
   * Register DOOM_LOOP callback
   */
  onDoomLoop(callback: (loop: LoopPattern) => void): () => void {
    this.doomLoopCallbacks.add(callback)
    return () => {
      this.doomLoopCallbacks.delete(callback)
    }
  }

  /**
   * Get safety status
   */
  getStatus(): SafetyStatus {
    const usage = this.safetyGuard.getCurrentUsage()
    const remaining = this.safetyGuard.getRemaining()
    const surplusRatio = this.safetyGuard.getSurplusRatio()
    const guardrailsStats = this.guardrails.getStats()

    return {
      resources: {
        usage,
        remaining,
        surplusRatio,
        warnings: this.safetyGuard["warningSent"].size,
      },
      loops: {
        stateLoops: guardrailsStats.stateTransitions,
        toolLoops: guardrailsStats.toolCalls,
        decisionHesitations: guardrailsStats.decisions,
        loopsBroken: guardrailsStats.loopsBroken,
      },
      rollbacks: {
        count: this.rollbackManager.getRollbackCount(),
        canRetry: this.rollbackManager.canRetry(),
      },
      safe: this.guardrails.checkLimits().safe,
    }
  }

  /**
   * Create checkpoint
   */
  async createCheckpoint(reason = "Manual checkpoint"): Promise<string | undefined> {
    return this.rollbackManager.createCheckpoint(reason)
  }

  /**
   * Rollback to checkpoint
   */
  async rollback(checkpointId: string, reason = ""): Promise<boolean> {
    const result = await this.rollbackManager.performRollback(
      checkpointId,
      "user_request",
      reason,
    )
    return result.success
  }

  /**
   * Pause session (safety check passed)
   */
  async pause(reason: string): Promise<void> {
    await Bus.publish(AutonomousEvent.SessionPaused, {
      sessionId: this.sessionId,
      reason,
      state: AutonomousState.PAUSED,
      canResume: true,
    })
  }

  /**
   * Resume session
   */
  async resume(): Promise<void> {
    await Bus.publish(AutonomousEvent.SessionStarted, {
      sessionId: this.sessionId,
      requestId: "resume",
      autonomyLevel: "wild",
    })
  }

  /**
   * Get safety guard
   */
  getSafetyGuard(): SafetyGuard {
    return this.safetyGuard
  }

  /**
   * Get guardrails
   */
  getGuardrails(): SafetyGuardrails {
    return this.guardrails
  }

  /**
   * Get rollback manager
   */
  getRollbackManager(): RollbackManager {
    return this.rollbackManager
  }

  /**
   * Reset all safety state
   */
  async reset(): Promise<void> {
    this.safetyGuard.reset()
    this.guardrails.clear()
    this.rollbackManager.resetCount()
    this.destructiveOpsHistory = []
    log.info("Safety integration reset", { sessionId: this.sessionId })
  }

  /**
   * Cleanup
   */
  async cleanup(): Promise<void> {
    await this.rollbackManager.clearCheckpoints()
    this.guardrails.clear()
    log.info("Safety integration cleaned up", { sessionId: this.sessionId })
  }

  /**
   * Serialize state
   */
  serialize(): {
    safetyGuard: ReturnType<SafetyGuard["serialize"]>
    guardrails: ReturnType<SafetyGuardrails["serialize"]>
    rollbackManager: ReturnType<RollbackManager["serialize"]>
    config: SafetyIntegrationConfig
    destructiveOpsHistory: DestructiveOperation[]
  } {
    return {
      safetyGuard: this.safetyGuard.serialize(),
      guardrails: this.guardrails.serialize(),
      rollbackManager: this.rollbackManager.serialize(),
      config: this.config,
      destructiveOpsHistory: this.destructiveOpsHistory,
    }
  }

  /**
   * Deserialize state
   */
  static deserialize(
    data: ReturnType<SafetyIntegration["serialize"]>,
    sessionId: string,
  ): SafetyIntegration {
    const integration = new SafetyIntegration(sessionId, data.config)

    integration.safetyGuard = SafetyGuard.deserialize(
      data.safetyGuard,
      sessionId,
    )
    integration.guardrails = SafetyGuardrails.deserialize(
      data.guardrails,
      sessionId,
    )
    integration.rollbackManager = RollbackManager.deserialize(
      data.rollbackManager,
      sessionId,
    )
    integration.destructiveOpsHistory = data.destructiveOpsHistory

    return integration
  }

  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Subscribe to resource warnings
    Bus.subscribe(AutonomousEvent.ResourceWarning, async (event) => {
      if (event.properties.sessionId === this.sessionId) {
        log.warn("Resource warning received", event.properties)
      }
    })

    // Subscribe to loop detection
    Bus.subscribe(AutonomousEvent.LoopDetected, async (event) => {
      if (event.properties.sessionId === this.sessionId) {
        log.warn("Loop detected received", event.properties)

        // Auto-handle loop if configured
        if (this.config.autoRollbackOnFailure) {
          await this.handleFailure("loop_detected", {
            reason: `Loop detected: ${event.properties.loopType}`,
          })
        }
      }
    })

    // Subscribe to rollback events
    Bus.subscribe(AutonomousEvent.RollbackPerformed, async (event) => {
      if (event.properties.sessionId === this.sessionId) {
        log.info("Rollback performed", event.properties)
      }
    })
  }
}

/**
 * Create safety integration
 */
export function createSafetyIntegration(
  sessionId: string,
  config?: Partial<SafetyIntegrationConfig>,
): SafetyIntegration {
  return new SafetyIntegration(sessionId, config)
}

/**
 * Check if operation is destructive
 */
export function isDestructiveOperation(operation: {
  tool: string
  input?: unknown
}): DestructiveOperation | null {
  const { tool, input } = operation

  // Define destructive tool patterns
  const destructiveTools: Record<
    string,
    Omit<DestructiveOperation, "description">
  > = {
    Bash: {
      category: "file_deletion",
      reversible: false,
      riskLevel: "high",
    },
    Write: {
      category: "file_overwrite",
      reversible: false,
      riskLevel: "medium",
    },
    Edit: {
      category: "file_overwrite",
      reversible: false,
      riskLevel: "medium",
    },
    // Add more as needed
  }

  const baseOp = destructiveTools[tool]
  if (!baseOp) return null

  // Extract file information if available
  let files: string[] | undefined
  if (input && typeof input === "object") {
    if ("file_path" in input) {
      files = [String(input.file_path)]
    } else if ("path" in input) {
      files = [String(input.path)]
    }
  }

  return {
    ...baseOp,
    description: `${tool} operation${files ? ` on ${files.join(", ")}` : ""}`,
    files,
  }
}

/**
 * Get destructive operation risk level
 */
export function getDestructiveRiskLevel(operation: {
  tool: string
  input?: unknown
}): "low" | "medium" | "high" | "critical" {
  const op = isDestructiveOperation(operation)
  return op?.riskLevel ?? "low"
}
