/**
 * HITL (Human-in-the-Loop) TypeScript Client
 *
 * Provides TypeScript bindings for the Rust HITL approval system.
 * Communicates with zero-gateway service via HTTP API.
 *
 * @package hitl
 */

import { Log } from "@/util/log"
import z from "zod"

const log = Log.create({ service: "hitl.client" })

// ============================================================================
// Types
// ============================================================================

/**
 * Risk levels for operations
 */
export type RiskLevel = "Low" | "Medium" | "High" | "Critical"

/**
 * Approval type discriminated union
 */
export type ApprovalType =
  | { type: "merge_request"; platform: string; repo: string; mr_id: number }
  | { type: "trading_command"; asset: string; action: string; amount: number }
  | { type: "config_change"; key: string; old_value: string; new_value: string }
  | { type: "high_cost_operation"; operation: string; estimated_cost: number }
  | { type: "risk_operation"; description: string; risk_level: RiskLevel }
  | { type: "tool_execution"; tool: string; args: unknown; risk_level: RiskLevel; hand_id: string; execution_id: string }

/**
 * Approval status discriminated union
 */
export type ApprovalStatus =
  | { status: "pending" }
  | { status: "approved"; by: string; at: string }
  | { status: "rejected"; by: string; reason?: string; at: string }
  | { status: "cancelled"; reason: string }

/**
 * Approval request
 */
export interface ApprovalRequest {
  id: string
  approval_type: ApprovalType
  status: ApprovalStatus
  requester: string
  approvers: string[]
  title: string
  description?: string
  channel: string
  message_id?: string
  metadata: Record<string, unknown>
  created_at: string
  updated_at: string
  expires_at?: string
}

/**
 * Create approval request payload
 */
export interface CreateApprovalRequest {
  approval_type: ApprovalType
  requester: string
  approvers: string[]
  title: string
  description?: string
  channel: string
  metadata?: Record<string, unknown>
  ttl_seconds?: number
}

/**
 * Approval response
 */
export interface ApprovalResponse {
  success: boolean
  approval?: ApprovalRequest
  error?: string
}

/**
 * List pending response
 */
export interface ListPendingResponse {
  requests: ApprovalRequest[]
  total: number
}

/**
 * Decision request
 */
export interface DecideRequest {
  decided_by: string
  approved: boolean
  reason?: string
}

// ============================================================================
// Zod Schemas
// ============================================================================

export const RiskLevelSchema = z.enum(["Low", "Medium", "High", "Critical"])

export const ApprovalTypeSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("merge_request"),
    platform: z.string(),
    repo: z.string(),
    mr_id: z.number(),
  }),
  z.object({
    type: z.literal("trading_command"),
    asset: z.string(),
    action: z.string(),
    amount: z.number(),
  }),
  z.object({
    type: z.literal("config_change"),
    key: z.string(),
    old_value: z.string(),
    new_value: z.string(),
  }),
  z.object({
    type: z.literal("high_cost_operation"),
    operation: z.string(),
    estimated_cost: z.number(),
  }),
  z.object({
    type: z.literal("risk_operation"),
    description: z.string(),
    risk_level: RiskLevelSchema,
  }),
  z.object({
    type: z.literal("tool_execution"),
    tool: z.string(),
    args: z.unknown(),
    risk_level: RiskLevelSchema,
    hand_id: z.string(),
    execution_id: z.string(),
  }),
])

export const ApprovalStatusSchema = z.discriminatedUnion("status", [
  z.object({ status: z.literal("pending") }),
  z.object({ status: z.literal("approved"), by: z.string(), at: z.string() }),
  z.object({ status: z.literal("rejected"), by: z.string(), reason: z.string().optional(), at: z.string() }),
  z.object({ status: z.literal("cancelled"), reason: z.string() }),
])

export const ApprovalRequestSchema = z.object({
  id: z.string(),
  approval_type: ApprovalTypeSchema,
  status: ApprovalStatusSchema,
  requester: z.string(),
  approvers: z.array(z.string()),
  title: z.string(),
  description: z.string().optional(),
  channel: z.string(),
  message_id: z.string().optional(),
  metadata: z.record(z.string(), z.unknown()),
  created_at: z.string(),
  updated_at: z.string(),
  expires_at: z.string().optional(),
})

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_GATEWAY_URL = "http://127.0.0.1:4430"
const DEFAULT_TIMEOUT_MS = 30000

// ============================================================================
// HITL Client
// ============================================================================

/**
 * HITL Client Configuration
 */
export interface HitLClientConfig {
  /** Base URL for zero-gateway service */
  baseUrl: string
  /** Request timeout in milliseconds */
  timeoutMs: number
}

/**
 * HITL Client
 *
 * TypeScript client for the Rust HITL approval system.
 */
export class HitLClient {
  private baseUrl: string
  private timeoutMs: number

  constructor(config?: Partial<HitLClientConfig>) {
    this.baseUrl = (config?.baseUrl ?? DEFAULT_GATEWAY_URL).replace(/\/$/, "")
    this.timeoutMs = config?.timeoutMs ?? DEFAULT_TIMEOUT_MS
  }

  // ============================================================================
  // Approval Management
  // ============================================================================

  /**
   * Create a new approval request
   */
  async createRequest(request: CreateApprovalRequest): Promise<ApprovalResponse> {
    const response = await this.fetch("/api/v1/hitl/request", {
      method: "POST",
      body: JSON.stringify(request),
    })
    return response as ApprovalResponse
  }

  /**
   * List pending approval requests
   */
  async listPending(approverId?: string): Promise<ListPendingResponse> {
    const params = approverId ? `?approver_id=${encodeURIComponent(approverId)}` : ""
    const response = await this.fetch(`/api/v1/hitl/pending${params}`)
    return response as ListPendingResponse
  }

  /**
   * Get a specific approval request by ID
   */
  async get(id: string): Promise<ApprovalRequest | null> {
    try {
      const response = await this.fetch(`/api/v1/hitl/${id}`)
      const result = response as ApprovalResponse
      return result.approval ?? null
    } catch (error) {
      if (this.isNotFoundError(error)) {
        return null
      }
      throw error
    }
  }

  /**
   * Approve a request
   */
  async approve(id: string, decidedBy: string): Promise<ApprovalResponse> {
    return this.decide(id, {
      decided_by: decidedBy,
      approved: true,
    })
  }

  /**
   * Reject a request
   */
  async reject(id: string, decidedBy: string, reason?: string): Promise<ApprovalResponse> {
    return this.decide(id, {
      decided_by: decidedBy,
      approved: false,
      reason,
    })
  }

  /**
   * Make a decision on an approval request
   */
  async decide(id: string, request: DecideRequest): Promise<ApprovalResponse> {
    const response = await this.fetch(`/api/v1/hitl/${id}/decide`, {
      method: "POST",
      body: JSON.stringify(request),
    })
    return response as ApprovalResponse
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
        throw new HitLApiError(response.status, body, url)
      }

      return response.json()
    } finally {
      clearTimeout(timeoutId)
    }
  }

  private isNotFoundError(error: unknown): boolean {
    return error instanceof HitLApiError && error.status === 404
  }
}

/**
 * HITL API Error
 */
export class HitLApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly body: string,
    public readonly url: string,
  ) {
    super(`HITL API error ${status} for ${url}: ${body}`)
    this.name = "HitLApiError"
  }
}

// ============================================================================
// Singleton and Convenience Functions
// ============================================================================

let clientInstance: HitLClient | null = null

/**
 * Get the global client instance
 */
export function getHitLClient(config?: Partial<HitLClientConfig>): HitLClient {
  if (!clientInstance) {
    clientInstance = new HitLClient(config)
  }
  return clientInstance
}

/**
 * Create a new client instance
 */
export function createHitLClient(config?: Partial<HitLClientConfig>): HitLClient {
  return new HitLClient(config)
}

/**
 * List pending approvals (convenience function)
 */
export async function listPendingApprovals(approverId?: string): Promise<ApprovalRequest[]> {
  const response = await getHitLClient().listPending(approverId)
  return response.requests
}

/**
 * Approve a request (convenience function)
 */
export async function approveRequest(id: string, decidedBy: string): Promise<ApprovalResponse> {
  return getHitLClient().approve(id, decidedBy)
}

/**
 * Reject a request (convenience function)
 */
export async function rejectRequest(id: string, decidedBy: string, reason?: string): Promise<ApprovalResponse> {
  return getHitLClient().reject(id, decidedBy, reason)
}

/**
 * Check HITL service health (convenience function)
 */
export async function isHitLServiceHealthy(): Promise<boolean> {
  return getHitLClient().health()
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Get display name for approval type
 */
export function getApprovalTypeName(type: ApprovalType): string {
  switch (type.type) {
    case "merge_request":
      return "合并请求"
    case "trading_command":
      return "交易命令"
    case "config_change":
      return "配置变更"
    case "high_cost_operation":
      return "高成本操作"
    case "risk_operation":
      return "风险操作"
    case "tool_execution":
      return "工具执行"
  }
}

/**
 * Get risk level color
 */
export function getRiskLevelColor(level: RiskLevel): string {
  switch (level) {
    case "Low":
      return "green"
    case "Medium":
      return "yellow"
    case "High":
      return "orange"
    case "Critical":
      return "red"
  }
}

/**
 * Get risk level icon
 */
export function getRiskLevelIcon(level: RiskLevel): string {
  switch (level) {
    case "Low":
      return "🟢"
    case "Medium":
      return "🟡"
    case "High":
      return "🟠"
    case "Critical":
      return "🔴"
  }
}

/**
 * Get status display text
 */
export function getStatusDisplay(status: ApprovalStatus): string {
  switch (status.status) {
    case "pending":
      return "⏳ 待审批"
    case "approved":
      return `✅ 已批准 by ${status.by}`
    case "rejected":
      return `❌ 已拒绝 by ${status.by}${status.reason ? `: ${status.reason}` : ""}`
    case "cancelled":
      return `⚪ 已取消: ${status.reason}`
  }
}

/**
 * Format approval request summary
 */
export function formatApprovalSummary(request: ApprovalRequest): string {
  const type = getApprovalTypeName(request.approval_type)
  const status = request.status.status === "pending" ? "⏳" : request.status.status === "approved" ? "✅" : "❌"

  let detail = ""
  switch (request.approval_type.type) {
    case "merge_request":
      detail = `${request.approval_type.repo}#${request.approval_type.mr_id}`
      break
    case "trading_command":
      detail = `${request.approval_type.action} ${request.approval_type.amount} ${request.approval_type.asset}`
      break
    case "config_change":
      detail = request.approval_type.key
      break
    case "high_cost_operation":
      detail = `$${request.approval_type.estimated_cost.toFixed(2)}`
      break
    case "risk_operation":
      detail = `${getRiskLevelIcon(request.approval_type.risk_level)} ${request.approval_type.risk_level}`
      break
    case "tool_execution":
      detail = `${getRiskLevelIcon(request.approval_type.risk_level)} ${request.approval_type.tool} (${request.approval_type.hand_id})`
      break
  }

  return `${status} [${type}] ${request.title} - ${detail}`
}
