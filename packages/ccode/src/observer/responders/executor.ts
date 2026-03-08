/**
 * Executor Responder
 *
 * Executes automated actions based on observation events.
 * Integrates with the Hands system for autonomous execution.
 *
 * @module observer/responders/executor
 */

import { Log } from "@/util/log"
import type { Anomaly, Opportunity, OperatingMode, GearPreset } from "../types"
import { gearToOperatingMode } from "../types"
import type { ModeDecision } from "../controller"
import { ObserverEvent } from "../events"
import { getBridge, type TriggerResponse, type HandExecution } from "@/autonomous/hands/bridge"
import { getDialPanel, type DialPanel } from "../panel"

const log = Log.create({ service: "observer.responders.executor" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type ExecutionType =
  | "auto_fix"
  | "auto_optimize"
  | "auto_cleanup"
  | "scheduled_task"
  | "triggered_workflow"
  | "hands_action"

export type ExecutionStatus = "pending" | "approved" | "running" | "completed" | "failed" | "rejected"

export interface ExecutionRequest {
  id: string
  type: ExecutionType
  description: string
  trigger: {
    type: "anomaly" | "opportunity" | "pattern" | "schedule" | "manual" | "mode_switch"
    id?: string
  }
  actions: ExecutionAction[]
  mode: OperatingMode
  requiresApproval: boolean
  createdAt: Date
  approvedAt?: Date
  startedAt?: Date
  completedAt?: Date
  status: ExecutionStatus
  result?: ExecutionResult
  error?: string
}

export interface ExecutionAction {
  id: string
  type: string
  description: string
  command?: string
  params?: Record<string, unknown>
  status: "pending" | "running" | "completed" | "failed" | "skipped"
  output?: string
  error?: string
}

export interface ExecutionResult {
  success: boolean
  actionsCompleted: number
  actionsFailed: number
  summary: string
  outputs: Record<string, unknown>
}

export interface ExecutorConfig {
  /** Enable automatic execution */
  autoExecute: boolean
  /** Current operating mode (deprecated, use dialPanel) */
  mode: OperatingMode
  /** Execution types that require approval */
  requireApproval: ExecutionType[]
  /** Maximum concurrent executions */
  maxConcurrent: number
  /** Execution timeout (ms) */
  timeoutMs: number
  /** Enable dry-run mode */
  dryRun: boolean
  /** Use dial-based control (new architecture) */
  useDialControl: boolean
  /** Act dial threshold for immediate execution (0-100) */
  actThreshold: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: ExecutorConfig = {
  autoExecute: false,
  mode: "HYBRID",
  requireApproval: ["auto_fix", "hands_action", "triggered_workflow"],
  maxConcurrent: 1,
  timeoutMs: 600000, // 10 minutes
  dryRun: false,
  useDialControl: true,
  actThreshold: 50, // Act dial > 50% means immediate execution
}

// ─────────────────────────────────────────────────────────────────────────────
// Executor
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Executes automated actions based on observer events.
 */
export class Executor {
  private config: ExecutorConfig
  private executions: Map<string, ExecutionRequest> = new Map()
  private runningCount = 0
  private queue: ExecutionRequest[] = []
  private idCounter = 0
  private running = false
  private eventSubscriptions: Array<() => void> = []
  private approvalHandler: ((request: ExecutionRequest) => Promise<boolean>) | null = null
  private dialPanel: DialPanel | null = null

  constructor(config: Partial<ExecutorConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    if (this.config.useDialControl) {
      this.dialPanel = getDialPanel()
    }
  }

  /**
   * Get the current act dial value.
   */
  getActDialValue(): number {
    return this.dialPanel?.getDial("act") ?? 0
  }

  /**
   * Check if immediate execution is allowed based on act dial.
   */
  shouldActImmediately(): boolean {
    if (!this.config.useDialControl || !this.dialPanel) {
      return this.config.mode === "AUTO"
    }
    return this.dialPanel.shouldActImmediately()
  }

  /**
   * Check if execution requires approval based on dial value and type.
   */
  requiresApprovalForType(type: ExecutionType): boolean {
    const actValue = this.getActDialValue()

    // Always require approval for dangerous actions when act < 70%
    if (this.config.requireApproval.includes(type) && actValue < 70) {
      return true
    }

    // Act dial >= threshold means immediate execution
    return actValue < this.config.actThreshold
  }

  /**
   * Start the executor.
   */
  async start(): Promise<void> {
    if (this.running) return

    this.running = true

    if (this.config.autoExecute && this.config.mode !== "MANUAL") {
      const Bus = (await import("@/bus")).Bus

      // Listen for mode switches
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.ModeSwitched, (event) => {
          const { newMode } = event.properties as { newMode: OperatingMode }
          this.config.mode = newMode
          log.info("Executor mode updated", { mode: newMode })
        }),
      )

      // Auto-execute for certain opportunities in AUTO mode
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.OpportunityIdentified, async (event) => {
          if (this.config.mode !== "AUTO") return
          const opportunity = event.properties as Opportunity
          if (opportunity.type === "optimization" && opportunity.urgency === "high") {
            await this.executeOpportunity(opportunity)
          }
        }),
      )
    }

    log.info("Executor started", {
      autoExecute: this.config.autoExecute,
      mode: this.config.mode,
      dryRun: this.config.dryRun,
    })
  }

  /**
   * Stop the executor.
   */
  stop(): void {
    if (!this.running) return

    this.running = false

    for (const unsubscribe of this.eventSubscriptions) {
      unsubscribe()
    }
    this.eventSubscriptions = []

    // Cancel pending executions
    for (const execution of this.queue) {
      execution.status = "rejected"
    }
    this.queue = []

    log.info("Executor stopped")
  }

  /**
   * Set approval handler.
   */
  setApprovalHandler(handler: (request: ExecutionRequest) => Promise<boolean>): void {
    this.approvalHandler = handler
  }

  /**
   * Execute actions for an opportunity.
   */
  async executeOpportunity(opportunity: Opportunity): Promise<ExecutionRequest> {
    const actions = opportunity.suggestedActions.map((action, index) => ({
      id: `action_${index}`,
      type: "suggested_action",
      description: action,
      status: "pending" as const,
    }))

    return this.requestExecution({
      type: "auto_optimize",
      description: `Execute opportunity: ${opportunity.description}`,
      trigger: { type: "opportunity", id: opportunity.id },
      actions,
    })
  }

  /**
   * Execute a fix for an anomaly.
   */
  async executeAnomalyFix(
    anomaly: Anomaly,
    fix: { description: string; command?: string; params?: Record<string, unknown> },
  ): Promise<ExecutionRequest> {
    return this.requestExecution({
      type: "auto_fix",
      description: `Fix anomaly: ${anomaly.description}`,
      trigger: { type: "anomaly", id: anomaly.id },
      actions: [
        {
          id: "fix_action",
          type: "anomaly_fix",
          description: fix.description,
          command: fix.command,
          params: fix.params,
          status: "pending",
        },
      ],
    })
  }

  /**
   * Execute a cleanup action.
   */
  async executeCleanup(actions: ExecutionAction[]): Promise<ExecutionRequest> {
    return this.requestExecution({
      type: "auto_cleanup",
      description: "Automated cleanup",
      trigger: { type: "schedule" },
      actions,
    })
  }

  /**
   * Execute a Hands action.
   */
  async executeHandsAction(
    description: string,
    actions: ExecutionAction[],
  ): Promise<ExecutionRequest> {
    return this.requestExecution({
      type: "hands_action",
      description,
      trigger: { type: "manual" },
      actions,
    })
  }

  /**
   * Request an execution.
   */
  async requestExecution(options: {
    type: ExecutionType
    description: string
    trigger: ExecutionRequest["trigger"]
    actions: ExecutionAction[]
  }): Promise<ExecutionRequest> {
    // Use dial-based approval if enabled, otherwise fall back to mode-based
    const requiresApproval = this.config.useDialControl
      ? this.requiresApprovalForType(options.type)
      : (this.config.mode !== "AUTO" || this.config.requireApproval.includes(options.type))

    const request: ExecutionRequest = {
      id: `exec_${Date.now()}_${++this.idCounter}`,
      type: options.type,
      description: options.description,
      trigger: options.trigger,
      actions: options.actions,
      mode: this.config.mode,
      requiresApproval,
      createdAt: new Date(),
      status: requiresApproval ? "pending" : "approved",
    }

    this.executions.set(request.id, request)

    if (requiresApproval) {
      const actValue = this.getActDialValue()
      // Queue for approval
      log.info("Execution requires approval", {
        id: request.id,
        type: request.type,
        description: request.description,
        actDial: actValue,
        threshold: this.config.actThreshold,
      })

      // Try automatic approval if handler is set
      if (this.approvalHandler) {
        const approved = await this.approvalHandler(request)
        if (approved) {
          request.status = "approved"
          request.approvedAt = new Date()
        } else {
          request.status = "rejected"
          return request
        }
      }
    }

    if (request.status === "approved") {
      if (this.runningCount < this.config.maxConcurrent) {
        void this.runExecution(request)
      } else {
        this.queue.push(request)
      }
    }

    return request
  }

  /**
   * Approve an execution.
   */
  async approve(id: string): Promise<boolean> {
    const execution = this.executions.get(id)
    if (!execution || execution.status !== "pending") return false

    execution.status = "approved"
    execution.approvedAt = new Date()

    if (this.runningCount < this.config.maxConcurrent) {
      void this.runExecution(execution)
    } else {
      this.queue.push(execution)
    }

    return true
  }

  /**
   * Reject an execution.
   */
  reject(id: string, reason?: string): boolean {
    const execution = this.executions.get(id)
    if (!execution || execution.status !== "pending") return false

    execution.status = "rejected"
    execution.error = reason ?? "Rejected by user"
    return true
  }

  /**
   * Get execution by ID.
   */
  getExecution(id: string): ExecutionRequest | null {
    return this.executions.get(id) ?? null
  }

  /**
   * Get execution history.
   */
  getHistory(limit?: number): ExecutionRequest[] {
    return Array.from(this.executions.values())
      .sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime())
      .slice(0, limit ?? 50)
  }

  /**
   * Get pending executions (awaiting approval).
   */
  getPending(): ExecutionRequest[] {
    return Array.from(this.executions.values())
      .filter((e) => e.status === "pending")
  }

  /**
   * Get running executions.
   */
  getRunning(): ExecutionRequest[] {
    return Array.from(this.executions.values())
      .filter((e) => e.status === "running")
  }

  /**
   * Clear execution history.
   */
  clear(): void {
    this.executions.clear()
    this.queue = []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async runExecution(request: ExecutionRequest): Promise<void> {
    request.status = "running"
    request.startedAt = new Date()
    this.runningCount++

    log.info("Starting execution", {
      id: request.id,
      type: request.type,
      actionCount: request.actions.length,
      dryRun: this.config.dryRun,
    })

    // Publish started event
    const Bus = (await import("@/bus")).Bus
    await Bus.publish(ObserverEvent.ExecutionTriggered, {
      executionType: request.type,
      triggeredBy: request.trigger.id ?? "manual",
      status: "started",
    })

    try {
      const result = await this.executeActions(request)
      request.result = result
      request.status = result.success ? "completed" : "failed"
      request.completedAt = new Date()

      await Bus.publish(ObserverEvent.ExecutionTriggered, {
        executionType: request.type,
        triggeredBy: request.trigger.id ?? "manual",
        status: result.success ? "completed" : "failed",
        result: result.summary,
      })

      log.info("Execution completed", {
        id: request.id,
        success: result.success,
        completed: result.actionsCompleted,
        failed: result.actionsFailed,
      })
    } catch (error) {
      request.status = "failed"
      request.error = String(error)
      request.completedAt = new Date()

      await Bus.publish(ObserverEvent.ExecutionTriggered, {
        executionType: request.type,
        triggeredBy: request.trigger.id ?? "manual",
        status: "failed",
        result: request.error,
      })

      log.error("Execution failed", {
        id: request.id,
        error: request.error,
      })
    } finally {
      this.runningCount--
      this.processQueue()
    }
  }

  private async executeActions(request: ExecutionRequest): Promise<ExecutionResult> {
    const outputs: Record<string, unknown> = {}
    let completed = 0
    let failed = 0

    for (const action of request.actions) {
      action.status = "running"

      try {
        if (this.config.dryRun) {
          // Dry run - just log
          log.info("Dry run action", {
            id: action.id,
            type: action.type,
            description: action.description,
          })
          action.output = "[DRY RUN] Would execute: " + action.description
        } else {
          // Execute the action
          // TODO: Integrate with Hands system
          const output = await this.executeAction(action)
          action.output = output
          outputs[action.id] = output
        }

        action.status = "completed"
        completed++
      } catch (error) {
        action.status = "failed"
        action.error = String(error)
        failed++

        // Continue with other actions unless critical
        log.warn("Action failed", {
          id: action.id,
          error: action.error,
        })
      }
    }

    return {
      success: failed === 0,
      actionsCompleted: completed,
      actionsFailed: failed,
      summary: `Executed ${completed}/${request.actions.length} actions`,
      outputs,
    }
  }

  private async executeAction(action: ExecutionAction): Promise<string> {
    // Route execution based on action type
    const handsBridge = getBridge()

    // Check if Hands service is available
    const isHealthy = await handsBridge.health()

    if (action.type === "trigger_hand" || action.type === "hands_action") {
      if (!isHealthy) {
        throw new Error("Hands service not available")
      }

      const handId = (action.params?.handId as string) ?? action.id
      const triggerResult = await handsBridge.trigger({
        handId,
        params: action.params,
      })

      if (!triggerResult.success) {
        throw new Error(triggerResult.error ?? "Hand trigger failed")
      }

      return `Hand ${handId} triggered: ${triggerResult.executionId}`
    }

    if (action.type === "suggested_action" || action.type === "anomaly_fix") {
      // For generic actions, try to find an appropriate hand
      if (isHealthy) {
        const hands = await handsBridge.list()
        const matchingHand = hands.find((h) =>
          h.enabled &&
          (h.name.toLowerCase().includes(action.type.replace("_", " ")) ||
           h.id.includes(action.type))
        )

        if (matchingHand) {
          const result = await handsBridge.trigger({
            handId: matchingHand.id,
            params: {
              action: action.description,
              ...action.params,
            },
          })

          if (result.success) {
            return `Executed via hand ${matchingHand.id}: ${result.executionId}`
          }
        }
      }
    }

    // Fall back to command execution if provided
    if (action.command) {
      // Execute shell command (should be sandboxed in production)
      log.info("Executing command", { command: action.command })
      return `Command execution: ${action.command} (pending sandbox integration)`
    }

    return `Action ${action.id} processed: ${action.description}`
  }

  private processQueue(): void {
    while (this.runningCount < this.config.maxConcurrent && this.queue.length > 0) {
      const next = this.queue.shift()
      if (next && next.status === "approved") {
        void this.runExecution(next)
      }
    }
  }
}

/**
 * Create an executor.
 */
export function createExecutor(config?: Partial<ExecutorConfig>): Executor {
  return new Executor(config)
}
