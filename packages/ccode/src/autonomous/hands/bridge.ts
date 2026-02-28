/**
 * TypeScript Hands Bridge
 *
 * Provides TypeScript bindings for the Rust Hands autonomous agent system.
 * Communicates with zero-workflow service via HTTP API.
 *
 * @package autonomous/hands
 */

import { Log } from "@/util/log"
import z from "zod"

const log = Log.create({ service: "autonomous.hands.bridge" })

// ============================================================================
// Types
// ============================================================================

/**
 * Autonomy levels (matches Rust AutonomyLevel)
 */
export type AutonomyLevel = "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"

/**
 * Risk threshold for auto-approval
 */
export type RiskThreshold = "safe" | "low" | "medium" | "high"

/**
 * Hand trigger types
 */
export type HandTrigger =
  | { type: "cron"; expression: string }
  | { type: "webhook"; path: string; method: string }
  | { type: "git"; repo: string; events: GitEvent[] }
  | { type: "file_watch"; patterns: string[] }

/**
 * Git events for trigger
 */
export type GitEvent = "push" | "pull_request" | "issue" | "release"

/**
 * Auto-approve configuration
 */
export interface AutoApproveConfig {
  enabled: boolean
  allowedTools: string[]
  riskThreshold: RiskThreshold
  timeoutMs: number
}

/**
 * Resource limits
 */
export interface ResourceLimits {
  maxTokens: number
  maxCostUsd: number
  maxDurationSec: number
}

/**
 * Autonomy configuration
 */
export interface AutonomyConfig {
  level: AutonomyLevel
  unattended: boolean
  maxIterations: number
  autoApprove?: AutoApproveConfig
}

/**
 * Decision configuration
 */
export interface DecisionConfig {
  useCLOSE: boolean
  evolution: boolean
  webSearch: boolean
}

/**
 * Pipeline execution mode
 */
export type PipelineMode = "sequential" | "parallel" | "conditional"

/**
 * Hand configuration
 */
export interface HandConfig {
  /** Unique hand identifier */
  id: string

  /** Human-readable name */
  name: string

  /** Version string */
  version: string

  /** Description of what this hand does */
  description: string

  /** Cron schedule expression */
  schedule?: string

  /** Agent to use for execution (single-agent mode) */
  agent: string

  /**
   * List of agents for pipeline execution.
   * When specified, `agent` field is ignored and agents are executed according to `pipeline` mode.
   */
  agents?: string[]

  /**
   * Pipeline execution mode for multi-agent hands.
   * - sequential: Execute agents in order, passing output from one to the next
   * - parallel: Execute all agents simultaneously and merge outputs
   * - conditional: Execute agents based on CLOSE framework decisions
   */
  pipeline?: PipelineMode

  /** Whether the hand is enabled */
  enabled: boolean

  /** Memory path template */
  memoryPath?: string

  /** Custom parameters */
  params?: Record<string, unknown>

  /** Trigger configuration */
  trigger?: HandTrigger

  /** Autonomy configuration */
  autonomy?: AutonomyConfig

  /** Resource limits */
  resources?: ResourceLimits

  /** Decision configuration */
  decision?: DecisionConfig
}

/**
 * Hand manifest (full definition)
 */
export interface HandManifest {
  config: HandConfig
  content: string
  filePath: string
}

/**
 * Execution status
 */
export type ExecutionStatus = "pending" | "running" | "waiting_approval" | "completed" | "failed" | "paused"

/**
 * Risk level for tool operations
 */
export type RiskLevelValue = "safe" | "low" | "medium" | "high" | "critical"

/**
 * Approval decision from auto-approver
 */
export type ApprovalDecision = "auto_approve" | "queue" | "deny"

/**
 * Risk evaluation result
 */
export interface RiskEvaluation {
  tool: string
  riskLevel: RiskLevelValue
  reasons: string[]
  adjustments: Array<{
    pattern: string
    adjustment: number
    reason: string
  }>
}

/**
 * Approval result from the auto-approver
 */
export interface ApprovalResult {
  decision: ApprovalDecision
  riskEvaluation: RiskEvaluation
  reasons: string[]
  timeoutApplicable: boolean
  timeoutMs?: number
}

/**
 * Hand execution result
 */
export interface HandExecution {
  /** Execution ID */
  id: string

  /** Hand ID */
  handId: string

  /** Execution status */
  status: ExecutionStatus

  /** Start timestamp */
  startedAt: number

  /** End timestamp */
  completedAt?: number

  /** Duration in milliseconds */
  durationMs?: number

  /** Output content */
  output?: string

  /** Error message if failed */
  error?: string

  /** Whether execution succeeded */
  success?: boolean

  /** Quality score (0-100) */
  qualityScore?: number

  /** Tokens used */
  tokensUsed?: number

  /** Cost in USD */
  costUsd?: number
}

/**
 * Hands summary (for listing)
 */
export interface HandSummary {
  id: string
  name: string
  enabled: boolean
  schedule?: string
  agent: string
  lastExecution?: {
    status: ExecutionStatus
    timestamp: number
    success: boolean
  }
}

/**
 * Trigger request
 */
export interface TriggerRequest {
  handId: string
  params?: Record<string, unknown>
  sessionId?: string
}

/**
 * Trigger response
 */
export interface TriggerResponse {
  success: boolean
  executionId: string
  message?: string
  error?: string
}

// ============================================================================
// Zod Schemas
// ============================================================================

export const PipelineModeSchema = z.enum(["sequential", "parallel", "conditional"])

export const HandConfigSchema = z.object({
  id: z.string(),
  name: z.string(),
  version: z.string().default("1.0.0"),
  description: z.string().default(""),
  schedule: z.string().optional(),
  agent: z.string().default(""),
  agents: z.array(z.string()).optional(),
  pipeline: PipelineModeSchema.optional(),
  enabled: z.boolean().default(true),
  memoryPath: z.string().optional(),
  params: z.record(z.string(), z.unknown()).optional(),
})

export const HandExecutionSchema = z.object({
  id: z.string(),
  handId: z.string(),
  status: z.enum(["pending", "running", "completed", "failed", "paused"]),
  startedAt: z.number(),
  completedAt: z.number().optional(),
  durationMs: z.number().optional(),
  output: z.string().optional(),
  error: z.string().optional(),
  success: z.boolean().optional(),
  qualityScore: z.number().optional(),
  tokensUsed: z.number().optional(),
  costUsd: z.number().optional(),
})

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_WORKFLOW_URL = "http://127.0.0.1:4432"
const DEFAULT_TIMEOUT_MS = 30000

// ============================================================================
// Hands Bridge Client
// ============================================================================

/**
 * Hands Bridge Configuration
 */
export interface HandsBridgeConfig {
  /** Base URL for zero-workflow service */
  baseUrl: string

  /** Request timeout in milliseconds */
  timeoutMs: number
}

/**
 * Hands Bridge
 *
 * TypeScript client for the Rust Hands system.
 */
export class HandsBridge {
  private baseUrl: string
  private timeoutMs: number

  constructor(config?: Partial<HandsBridgeConfig>) {
    this.baseUrl = (config?.baseUrl ?? DEFAULT_WORKFLOW_URL).replace(/\/$/, "")
    this.timeoutMs = config?.timeoutMs ?? DEFAULT_TIMEOUT_MS
  }

  // ============================================================================
  // Hand Management
  // ============================================================================

  /**
   * List all hands
   */
  async list(): Promise<HandSummary[]> {
    const response = await this.fetch("/api/v1/hands")
    return response as HandSummary[]
  }

  /**
   * Get a hand by ID
   */
  async get(handId: string): Promise<HandManifest | null> {
    try {
      const response = await this.fetch(`/api/v1/hands/${handId}`)
      return response as HandManifest
    } catch (error) {
      if (this.isNotFoundError(error)) {
        return null
      }
      throw error
    }
  }

  /**
   * Register a new hand
   */
  async register(config: HandConfig): Promise<{ success: boolean; handId: string }> {
    const response = await this.fetch("/api/v1/hands", {
      method: "POST",
      body: JSON.stringify(config),
    })
    return response as { success: boolean; handId: string }
  }

  /**
   * Update a hand's configuration
   */
  async update(handId: string, config: Partial<HandConfig>): Promise<{ success: boolean }> {
    const response = await this.fetch(`/api/v1/hands/${handId}`, {
      method: "PATCH",
      body: JSON.stringify(config),
    })
    return response as { success: boolean }
  }

  /**
   * Delete a hand
   */
  async delete(handId: string): Promise<{ success: boolean }> {
    const response = await this.fetch(`/api/v1/hands/${handId}`, {
      method: "DELETE",
    })
    return response as { success: boolean }
  }

  /**
   * Enable a hand
   */
  async enable(handId: string): Promise<{ success: boolean }> {
    return this.update(handId, { enabled: true })
  }

  /**
   * Disable a hand
   */
  async disable(handId: string): Promise<{ success: boolean }> {
    return this.update(handId, { enabled: false })
  }

  // ============================================================================
  // Execution
  // ============================================================================

  /**
   * Trigger a hand execution
   */
  async trigger(request: TriggerRequest): Promise<TriggerResponse> {
    const response = await this.fetch(`/api/v1/hands/${request.handId}/trigger`, {
      method: "POST",
      body: JSON.stringify({
        params: request.params,
        sessionId: request.sessionId,
      }),
    })
    return response as TriggerResponse
  }

  /**
   * Get execution status
   */
  async getExecution(executionId: string): Promise<HandExecution | null> {
    try {
      const response = await this.fetch(`/api/v1/hands/executions/${executionId}`)
      return HandExecutionSchema.parse(response)
    } catch (error) {
      if (this.isNotFoundError(error)) {
        return null
      }
      throw error
    }
  }

  /**
   * List executions for a hand
   */
  async listExecutions(handId: string, limit = 10): Promise<HandExecution[]> {
    const response = await this.fetch(`/api/v1/hands/${handId}/executions?limit=${limit}`)
    return (response as unknown[]).map((e) => HandExecutionSchema.parse(e))
  }

  /**
   * Pause a running execution
   */
  async pauseExecution(executionId: string): Promise<{ success: boolean }> {
    const response = await this.fetch(`/api/v1/hands/executions/${executionId}/pause`, {
      method: "POST",
    })
    return response as { success: boolean }
  }

  /**
   * Resume a paused execution
   */
  async resumeExecution(executionId: string): Promise<{ success: boolean }> {
    const response = await this.fetch(`/api/v1/hands/executions/${executionId}/resume`, {
      method: "POST",
    })
    return response as { success: boolean }
  }

  /**
   * Cancel a running execution
   */
  async cancelExecution(executionId: string): Promise<{ success: boolean }> {
    const response = await this.fetch(`/api/v1/hands/executions/${executionId}/cancel`, {
      method: "POST",
    })
    return response as { success: boolean }
  }

  // ============================================================================
  // Scheduler
  // ============================================================================

  /**
   * Get scheduler status
   */
  async getSchedulerStatus(): Promise<{
    running: boolean
    activeHands: number
    pendingExecutions: number
    nextExecution?: { handId: string; scheduledAt: number }
  }> {
    const response = await this.fetch("/api/v1/hands/scheduler/status")
    return response as {
      running: boolean
      activeHands: number
      pendingExecutions: number
      nextExecution?: { handId: string; scheduledAt: number }
    }
  }

  /**
   * Start the scheduler
   */
  async startScheduler(): Promise<{ success: boolean }> {
    const response = await this.fetch("/api/v1/hands/scheduler/start", {
      method: "POST",
    })
    return response as { success: boolean }
  }

  /**
   * Stop the scheduler
   */
  async stopScheduler(): Promise<{ success: boolean }> {
    const response = await this.fetch("/api/v1/hands/scheduler/stop", {
      method: "POST",
    })
    return response as { success: boolean }
  }

  // ============================================================================
  // Health
  // ============================================================================

  /**
   * Health check
   */
  async health(): Promise<boolean> {
    try {
      await this.fetch("/health", { timeout: 5000 })
      return true
    } catch {
      return false
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async fetch(path: string, options?: RequestInit & { timeout?: number }): Promise<unknown> {
    const url = `${this.baseUrl}${path}`
    const timeout = options?.timeout ?? this.timeoutMs

    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), timeout)

    try {
      const response = await fetch(url, {
        ...options,
        headers: {
          "Content-Type": "application/json",
          ...options?.headers,
        },
        signal: controller.signal,
      })

      if (!response.ok) {
        const body = await response.text()
        throw new HandsApiError(response.status, body, url)
      }

      return response.json()
    } finally {
      clearTimeout(timeoutId)
    }
  }

  private isNotFoundError(error: unknown): boolean {
    return error instanceof HandsApiError && error.status === 404
  }
}

/**
 * Hands API Error
 */
export class HandsApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
    public readonly url: string,
  ) {
    super(`Hands API error ${status} for ${url}: ${body}`)
    this.name = "HandsApiError"
  }
}

// ============================================================================
// Singleton and Convenience Functions
// ============================================================================

let bridgeInstance: HandsBridge | null = null

/**
 * Get the global bridge instance
 */
export function getBridge(config?: Partial<HandsBridgeConfig>): HandsBridge {
  if (!bridgeInstance) {
    bridgeInstance = new HandsBridge(config)
  }
  return bridgeInstance
}

/**
 * Create a new bridge instance
 */
export function createBridge(config?: Partial<HandsBridgeConfig>): HandsBridge {
  return new HandsBridge(config)
}

/**
 * Trigger a hand execution (convenience function)
 */
export async function triggerHands(
  handId: string,
  params?: Record<string, unknown>,
): Promise<TriggerResponse> {
  return getBridge().trigger({ handId, params })
}

/**
 * List all hands (convenience function)
 */
export async function listHands(): Promise<HandSummary[]> {
  return getBridge().list()
}

/**
 * Get hand execution (convenience function)
 */
export async function getHandExecution(executionId: string): Promise<HandExecution | null> {
  return getBridge().getExecution(executionId)
}

/**
 * Check hands service health (convenience function)
 */
export async function isHandsServiceHealthy(): Promise<boolean> {
  return getBridge().health()
}

// ============================================================================
// Pipeline Helper Functions
// ============================================================================

/**
 * Check if a hand uses pipeline mode (multiple agents)
 */
export function isPipelineHand(config: HandConfig): boolean {
  return (config.agents?.length ?? 0) > 1
}

/**
 * Get the list of agents for a hand.
 * Returns agents from `agents` field if set, otherwise wraps `agent` in an array.
 */
export function getHandAgents(config: HandConfig): string[] {
  if (config.agents && config.agents.length > 0) {
    return config.agents
  }
  return config.agent ? [config.agent] : []
}

/**
 * Get the pipeline mode for a hand, defaulting to sequential.
 */
export function getHandPipelineMode(config: HandConfig): PipelineMode {
  return config.pipeline ?? "sequential"
}

/**
 * Get pipeline mode description in Chinese
 */
export function getPipelineModeDescription(mode: PipelineMode): string {
  switch (mode) {
    case "sequential":
      return "顺序执行：前一个 Agent 的输出作为下一个的输入"
    case "parallel":
      return "并行执行：所有 Agent 同时执行并合并输出"
    case "conditional":
      return "条件执行：根据 CLOSE 框架决策选择下一个 Agent"
  }
}

