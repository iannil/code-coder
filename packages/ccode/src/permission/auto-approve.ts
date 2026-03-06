/**
 * Auto-Approve Permission Handler
 *
 * Provides risk-based automatic approval for tool calls in autonomous/unattended mode.
 * Uses the native Rust AutoApproveEngine from @codecoder-ai/core for performance.
 *
 * @package permission
 */

import { Log } from "@/util/log"
import z from "zod"
import type { Permission } from "./index"

// Import native AutoApproveEngine from @codecoder-ai/core
import {
  AutoApproveEngine,
  type AutoApproveConfig as NativeAutoApproveConfig,
  type ToolInput as NativeToolInput,
  type ApprovalDecision as NativeApprovalDecision,
  type ExecutionContext as NativeExecutionContext,
  type AdaptiveRiskResult as NativeAdaptiveRiskResult,
  type PermissionRiskLevel,
} from "@codecoder-ai/core"

const log = Log.create({ service: "permission.auto-approve" })

// ============================================================================
// Types (Re-export for backward compatibility)
// ============================================================================

/**
 * Risk levels for tool operations
 */
export type RiskLevel = PermissionRiskLevel

/**
 * Auto-approve configuration
 */
export interface AutoApproveConfig {
  /** Enable auto-approval */
  enabled: boolean

  /** Tools allowed for auto-approval (whitelist) */
  allowedTools: string[]

  /** Maximum risk level for auto-approval */
  riskThreshold: RiskLevel

  /** Timeout in milliseconds before auto-approving */
  timeoutMs: number

  /** Whether running in unattended mode */
  unattended: boolean
}

// ============================================================================
// Zod Schemas (for config integration)
// ============================================================================

/**
 * Risk level schema for configuration
 */
export const RiskLevelSchema = z.enum(["safe", "low", "medium", "high"]).meta({
  ref: "RiskLevel",
})

/**
 * Auto-approve configuration schema for use in Config and Agent definitions
 */
export const AutoApproveConfigSchema = z
  .object({
    /** Enable auto-approval */
    enabled: z.boolean().optional().describe("Enable auto-approval for tools"),

    /** Tools allowed for auto-approval (whitelist). Empty = use risk-based evaluation only */
    allowedTools: z.array(z.string()).optional().describe("Tools allowed for auto-approval"),

    /** Maximum risk level for auto-approval */
    riskThreshold: RiskLevelSchema.optional().describe("Maximum risk level for auto-approval"),

    /** Timeout in milliseconds before auto-approving non-critical operations */
    timeoutMs: z.number().int().positive().optional().describe("Timeout before auto-approving"),
  })
  .strict()
  .meta({
    ref: "AutoApproveConfig",
  })
export type AutoApproveConfigInput = z.infer<typeof AutoApproveConfigSchema>

/**
 * Tool risk assessment result
 */
interface ToolRiskAssessment {
  tool: string
  risk: RiskLevel
  reason: string
  autoApprovable: boolean
}

/**
 * Auto-approve audit entry
 */
export interface AutoApproveAudit {
  timestamp: string
  permissionId: string
  tool: string
  pattern?: string | string[]
  risk: RiskLevel
  decision: "approved" | "rejected" | "timeout_approved"
  reason: string
}

// ============================================================================
// Native Engine Instance
// ============================================================================

/** Cached native engine instance */
let nativeEngine: AutoApproveEngine | null = null

/**
 * Get or create the native auto-approve engine
 */
function getNativeEngine(config: AutoApproveConfig): AutoApproveEngine {
  // Create engine with config
  return AutoApproveEngine.create({
    enabled: config.enabled,
    allowedTools: config.allowedTools,
    riskThreshold: config.riskThreshold,
    timeoutMs: config.timeoutMs,
    unattended: config.unattended,
  })
}

// ============================================================================
// Risk Level Utilities
// ============================================================================

const RISK_VALUES: Record<RiskLevel, number> = {
  safe: 0,
  low: 1,
  medium: 2,
  high: 3,
  critical: 4,
}

/**
 * Compare risk levels
 */
export function compareRisk(a: RiskLevel, b: RiskLevel): number {
  return RISK_VALUES[a] - RISK_VALUES[b]
}

/**
 * Check if risk is at or below threshold
 */
export function riskAtOrBelowThreshold(risk: RiskLevel, threshold: RiskLevel): boolean {
  return RISK_VALUES[risk] <= RISK_VALUES[threshold]
}

/**
 * Parse risk level from string
 */
export function parseRiskLevel(level: string): RiskLevel {
  const normalized = level.toLowerCase() as RiskLevel
  return RISK_VALUES[normalized] !== undefined ? normalized : "medium"
}

// ============================================================================
// Tool Risk Assessment (Delegates to Native Engine)
// ============================================================================

/**
 * Convert tool input to native format
 */
function toNativeToolInput(tool: string, input: unknown): NativeToolInput | null {
  if (!input || typeof input !== "object") {
    return null
  }

  if (tool === "Bash" && "command" in input) {
    return { inputType: "bash", command: String(input.command) }
  }

  if ((tool === "Write" || tool === "Edit") && "file_path" in input) {
    return { inputType: "file", path: String(input.file_path) }
  }

  // Try JSON for other inputs
  try {
    return { inputType: "json", json: JSON.stringify(input) }
  } catch {
    return null
  }
}

/**
 * Assess risk level for a tool operation
 *
 * Uses native Rust engine for performance.
 */
export function assessToolRisk(tool: string, input: unknown): ToolRiskAssessment {
  // Create a temporary safe-only engine for risk assessment
  const engine = AutoApproveEngine.safeOnly(false)
  const nativeInput = toNativeToolInput(tool, input)
  const result = engine.assessRisk(tool, nativeInput)

  return {
    tool,
    risk: result.risk,
    reason: result.reason,
    autoApprovable: result.autoApprovable,
  }
}

// ============================================================================
// Auto-Approve Handler
// ============================================================================

/**
 * Audit log for auto-approve decisions
 */
const auditLog: AutoApproveAudit[] = []

/**
 * Maximum audit log entries to keep
 */
const MAX_AUDIT_ENTRIES = 1000

/**
 * Record auto-approve decision to audit log
 */
function recordAudit(entry: AutoApproveAudit): void {
  auditLog.push(entry)
  if (auditLog.length > MAX_AUDIT_ENTRIES) {
    auditLog.shift()
  }
  log.info("Auto-approve decision", entry)
}

/**
 * Get audit log
 */
export function getAuditLog(): readonly AutoApproveAudit[] {
  return auditLog
}

/**
 * Clear audit log
 */
export function clearAuditLog(): void {
  auditLog.length = 0
}

/**
 * Create an auto-approve permission handler
 *
 * @param config - Auto-approve configuration
 * @returns Permission handler callback
 */
export function createAutoApproveHandler(config: AutoApproveConfig): Permission.AskCallback {
  const { enabled, timeoutMs, unattended } = config
  const engine = getNativeEngine(config)

  return async (info: Permission.Info): Promise<"once" | "always" | "reject"> => {
    const tool = info.type
    const input = info.metadata
    const nativeInput = toNativeToolInput(tool, input)

    // If disabled, always reject (fall back to manual approval)
    if (!enabled) {
      recordAudit({
        timestamp: new Date().toISOString(),
        permissionId: info.id,
        tool,
        pattern: info.pattern,
        risk: "medium",
        decision: "rejected",
        reason: "Auto-approve disabled",
      })
      return "reject"
    }

    // Use native engine for evaluation
    const decision = engine.evaluate(tool, nativeInput)

    // Critical operations are ALWAYS rejected
    if (decision.risk === "critical") {
      recordAudit({
        timestamp: new Date().toISOString(),
        permissionId: info.id,
        tool,
        pattern: info.pattern,
        risk: decision.risk,
        decision: "rejected",
        reason: `Critical operation blocked: ${decision.reason}`,
      })
      return "reject"
    }

    // Auto-approved
    if (decision.approved) {
      recordAudit({
        timestamp: new Date().toISOString(),
        permissionId: info.id,
        tool,
        pattern: info.pattern,
        risk: decision.risk,
        decision: "approved",
        reason: decision.reason,
      })
      return "once"
    }

    // If unattended mode and timeout is set, wait then approve
    if (unattended && timeoutMs > 0 && decision.autoApprovable) {
      await new Promise((resolve) => setTimeout(resolve, timeoutMs))

      recordAudit({
        timestamp: new Date().toISOString(),
        permissionId: info.id,
        tool,
        pattern: info.pattern,
        risk: decision.risk,
        decision: "timeout_approved",
        reason: `Timeout auto-approved after ${timeoutMs}ms: ${decision.reason}`,
      })
      return "once"
    }

    // Otherwise reject
    recordAudit({
      timestamp: new Date().toISOString(),
      permissionId: info.id,
      tool,
      pattern: info.pattern,
      risk: decision.risk,
      decision: "rejected",
      reason: decision.reason,
    })
    return "reject"
  }
}

/**
 * Create default safe-tools-only auto-approve config
 */
export function createSafeOnlyConfig(unattended = true): AutoApproveConfig {
  return {
    enabled: true,
    allowedTools: ["Read", "Glob", "Grep", "LS", "WebFetch", "WebSearch"],
    riskThreshold: "low",
    timeoutMs: 0,
    unattended,
  }
}

/**
 * Create permissive auto-approve config for trusted environments
 */
export function createPermissiveConfig(unattended = true): AutoApproveConfig {
  return {
    enabled: true,
    allowedTools: [], // Use risk-based evaluation only
    riskThreshold: "medium",
    timeoutMs: 30000,
    unattended,
  }
}

/**
 * Resolve partial config input to full AutoApproveConfig with defaults
 */
export function resolveAutoApproveConfig(
  input: AutoApproveConfigInput | undefined,
  unattended = false,
): AutoApproveConfig | undefined {
  if (!input || !input.enabled) return undefined

  return {
    enabled: true,
    allowedTools: input.allowedTools ?? [],
    riskThreshold: input.riskThreshold ?? "low",
    timeoutMs: input.timeoutMs ?? 0,
    unattended,
  }
}

/**
 * Check if a permission request should be auto-approved
 *
 * Returns the approval decision if auto-approve applies, or undefined to defer to manual approval
 */
export function shouldAutoApprove(
  config: AutoApproveConfig,
  tool: string,
  input: unknown,
): "once" | "reject" | undefined {
  if (!config.enabled) return undefined

  const engine = getNativeEngine(config)
  const nativeInput = toNativeToolInput(tool, input)
  const decision = engine.evaluate(tool, nativeInput)

  // Critical operations NEVER auto-approve - defer to manual
  if (decision.risk === "critical") return undefined

  return decision.approved ? "once" : undefined
}

// ============================================================================
// Environment Variable Support
// ============================================================================

/**
 * Environment variable names for auto-approve configuration
 */
export const ENV_VARS = {
  ENABLED: "CODECODER_AUTO_APPROVE",
  THRESHOLD: "CODECODER_AUTO_APPROVE_THRESHOLD",
  TOOLS: "CODECODER_AUTO_APPROVE_TOOLS",
  TIMEOUT: "CODECODER_AUTO_APPROVE_TIMEOUT",
} as const

/**
 * Get auto-approve configuration from environment variables
 *
 * Environment variables:
 * - CODECODER_AUTO_APPROVE: "true" or "1" to enable
 * - CODECODER_AUTO_APPROVE_THRESHOLD: "safe" | "low" | "medium" | "high"
 * - CODECODER_AUTO_APPROVE_TOOLS: comma-separated tool names (e.g., "Read,Glob,Grep")
 * - CODECODER_AUTO_APPROVE_TIMEOUT: timeout in milliseconds
 *
 * @returns Auto-approve config input or undefined if not configured
 */
export function getAutoApproveFromEnv(): AutoApproveConfigInput | undefined {
  const envEnabled = process.env[ENV_VARS.ENABLED]
  if (!envEnabled) return undefined

  const enabled = envEnabled === "true" || envEnabled === "1"
  if (!enabled) return undefined

  const thresholdStr = process.env[ENV_VARS.THRESHOLD]
  const riskThreshold = thresholdStr ? parseRiskLevel(thresholdStr) : "low"

  // Exclude 'critical' from env-based config for safety
  if (riskThreshold === "critical") {
    log.warn("Environment variable CODECODER_AUTO_APPROVE_THRESHOLD cannot be 'critical', defaulting to 'high'")
  }
  const safeThreshold = riskThreshold === "critical" ? "high" : riskThreshold

  const toolsStr = process.env[ENV_VARS.TOOLS]
  const allowedTools = toolsStr ? toolsStr.split(",").map((t) => t.trim()).filter(Boolean) : undefined

  const timeoutStr = process.env[ENV_VARS.TIMEOUT]
  const timeoutMs = timeoutStr ? parseInt(timeoutStr, 10) : undefined

  log.info("Auto-approve config from environment", {
    enabled: true,
    riskThreshold: safeThreshold,
    allowedTools,
    timeoutMs,
  })

  return {
    enabled: true,
    riskThreshold: safeThreshold,
    allowedTools,
    timeoutMs: timeoutMs && !isNaN(timeoutMs) ? timeoutMs : undefined,
  }
}

// ============================================================================
// Adaptive Risk Assessment (Delegates to Native Engine)
// ============================================================================

/**
 * Project sensitivity levels
 */
export type ProjectSensitivity = "low" | "medium" | "high"

/**
 * Time of day categories
 */
export type TimeOfDay = "business" | "after_hours"

/**
 * Context factors for adaptive risk assessment
 */
export interface ContextFactors {
  /** Project sensitivity (production = high) */
  projectSensitivity: ProjectSensitivity

  /** Time of day (after hours = stricter) */
  timeOfDay: TimeOfDay

  /** Historical success rate (0-1) */
  successRate: number

  /** Current session error count */
  sessionErrorCount: number

  /** Current session iteration count */
  sessionIterations: number

  /** Whether in unattended mode */
  unattended: boolean
}

/**
 * Adaptive risk configuration result
 */
export interface AdaptiveRiskConfig {
  /** Base risk level for the tool */
  baseRisk: RiskLevel

  /** Context factors used */
  contextFactors: ContextFactors

  /** Adjusted risk level after applying factors */
  adjustedRisk: RiskLevel

  /** Adjustment applied (+N = risk increased, -N = risk decreased) */
  adjustment: number

  /** Reason for adjustment */
  adjustmentReason: string
}

/**
 * Execution context for adaptive assessment
 */
export interface ExecutionContext {
  /** Session ID */
  sessionId: string

  /** Current iteration */
  iteration: number

  /** Errors in this session */
  errors: number

  /** Successful operations in session */
  successes: number

  /** Project path */
  projectPath?: string

  /** Is production environment */
  isProduction?: boolean
}

/**
 * Convert to native execution context
 */
function toNativeExecutionContext(ctx: ExecutionContext): NativeExecutionContext {
  return {
    sessionId: ctx.sessionId,
    iteration: ctx.iteration,
    errors: ctx.errors,
    successes: ctx.successes,
    projectPath: ctx.projectPath,
    isProduction: ctx.isProduction ?? false,
  }
}

/**
 * Evaluate adaptive risk for a tool operation
 *
 * Uses native Rust engine for performance.
 *
 * @param tool Tool name
 * @param input Tool input
 * @param ctx Execution context
 * @returns Adaptive risk configuration
 */
export function evaluateAdaptiveRisk(
  tool: string,
  input: unknown,
  ctx: ExecutionContext,
): AdaptiveRiskConfig {
  const engine = AutoApproveEngine.permissive(false)
  const nativeInput = toNativeToolInput(tool, input)
  const nativeCtx = toNativeExecutionContext(ctx)
  const result = engine.evaluateAdaptiveRisk(tool, nativeInput, nativeCtx)

  // Build context factors from native result
  const contextFactors: ContextFactors = {
    projectSensitivity: "low", // Simplified - native doesn't expose this directly
    timeOfDay: "business",
    successRate: ctx.successes / (ctx.successes + ctx.errors || 1),
    sessionErrorCount: ctx.errors,
    sessionIterations: ctx.iteration,
    unattended: false,
  }

  return {
    baseRisk: result.baseRisk,
    contextFactors,
    adjustedRisk: result.adjustedRisk,
    adjustment: result.adjustment,
    adjustmentReason: result.adjustmentReason,
  }
}

/**
 * Check if adaptive risk allows auto-approval
 *
 * @param config Adaptive risk config
 * @param threshold Risk threshold for auto-approval
 * @returns Whether auto-approval is allowed
 */
export function adaptiveRiskAllowsApproval(
  config: AdaptiveRiskConfig,
  threshold: RiskLevel,
): boolean {
  // Critical is never auto-approved
  if (config.adjustedRisk === "critical") return false

  return riskAtOrBelowThreshold(config.adjustedRisk, threshold)
}

/**
 * Create adaptive auto-approve handler
 *
 * Uses native engine with adaptive risk assessment.
 */
export function createAdaptiveAutoApproveHandler(
  baseConfig: AutoApproveConfig,
  ctx: ExecutionContext,
): (tool: string, input: unknown) => "once" | "reject" | undefined {
  const engine = getNativeEngine(baseConfig)
  const nativeCtx = toNativeExecutionContext(ctx)

  return (tool: string, input: unknown) => {
    if (!baseConfig.enabled) return undefined

    const nativeInput = toNativeToolInput(tool, input)
    const decision = engine.evaluateAdaptive(tool, nativeInput, nativeCtx)

    if (!decision.approved) {
      log.info("Adaptive risk rejected auto-approval", {
        tool,
        risk: decision.risk,
        reason: decision.reason,
      })
      return undefined // Defer to manual approval
    }

    log.info("Adaptive risk approved", {
      tool,
      risk: decision.risk,
      reason: decision.reason,
    })

    return "once"
  }
}
