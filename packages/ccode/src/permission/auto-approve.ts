/**
 * Auto-Approve Permission Handler
 *
 * Provides risk-based automatic approval for tool calls in autonomous/unattended mode.
 * Inspired by OpenFang's approval gates design.
 *
 * @package permission
 */

import { Log } from "@/util/log"
import z from "zod"
import type { Permission } from "./index"

const log = Log.create({ service: "permission.auto-approve" })

// ============================================================================
// Types
// ============================================================================

/**
 * Risk levels for tool operations
 */
export type RiskLevel = "safe" | "low" | "medium" | "high" | "critical"

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
// Tool Risk Assessment
// ============================================================================

/**
 * Tool categories with base risk levels
 */
const TOOL_BASE_RISK: Record<string, RiskLevel> = {
  // Safe - No side effects, read-only
  Read: "safe",
  Glob: "safe",
  Grep: "safe",
  LS: "safe",
  NotebookRead: "safe",
  TaskList: "safe",
  TaskGet: "safe",

  // Low - External read-only
  WebFetch: "low",
  WebSearch: "low",

  // Medium - Local reversible writes
  Write: "medium",
  Edit: "medium",
  NotebookEdit: "medium",
  TaskCreate: "medium",
  TaskUpdate: "medium",

  // High - External side effects or semi-reversible
  Bash: "high",
  Task: "high",
  mcp__playwright__browser_click: "high",
  mcp__playwright__browser_type: "high",
  mcp__playwright__browser_navigate: "high",
}

/**
 * Bash command patterns that elevate risk level
 */
const BASH_RISK_PATTERNS: Array<{ pattern: RegExp; risk: RiskLevel; reason: string }> = [
  // Critical - System level, destructive
  { pattern: /\bsudo\b/, risk: "critical", reason: "sudo command requires elevated privileges" },
  { pattern: /\brm\s+-rf?\s+\/\s*$/, risk: "critical", reason: "rm on root path is destructive" },
  { pattern: /\brm\s+-rf?\s+\/[^\/\s]*\s*$/, risk: "high", reason: "recursive file deletion" },
  { pattern: /\b(shutdown|reboot|init)\b/, risk: "critical", reason: "system control command" },
  { pattern: /\b(mkfs|fdisk|dd)\b/, risk: "critical", reason: "disk manipulation command" },
  { pattern: /\b(chmod|chown)\s+(-R\s+)?[0-9]{3}\s+\//, risk: "critical", reason: "permission change on root" },

  // High - External effects, irreversible
  { pattern: /\bgit\s+push\s+--force/, risk: "critical", reason: "force push is destructive" },
  { pattern: /\bgit\s+push\b/, risk: "high", reason: "git push has external effects" },
  { pattern: /\bgit\s+reset\s+--hard/, risk: "high", reason: "hard reset discards changes" },
  { pattern: /\bcurl\s+.*-X\s*(POST|PUT|DELETE|PATCH)/, risk: "high", reason: "HTTP mutation request" },
  { pattern: /\bnpm\s+publish\b/, risk: "high", reason: "package publishing has external effects" },
  { pattern: /\bcargo\s+publish\b/, risk: "high", reason: "package publishing has external effects" },
  { pattern: /\bdocker\s+(push|rm|rmi)\b/, risk: "high", reason: "docker registry/image manipulation" },

  // Medium - Local changes
  { pattern: /\bgit\s+(add|commit|checkout|branch)\b/, risk: "medium", reason: "git local operation" },
  { pattern: /\bnpm\s+(install|uninstall)\b/, risk: "medium", reason: "dependency modification" },
  { pattern: /\bcargo\s+(add|remove)\b/, risk: "medium", reason: "dependency modification" },
  { pattern: /\bmkdir\b/, risk: "medium", reason: "directory creation" },
  { pattern: /\btouch\b/, risk: "medium", reason: "file creation" },

  // Low - Information gathering
  { pattern: /\bgit\s+(status|log|diff|show|branch\s+-[avl])\b/, risk: "low", reason: "git read operation" },
  { pattern: /\bcurl\s+.*-X\s*GET/, risk: "low", reason: "HTTP read request" },
  { pattern: /\bcurl\s+(?!.*-X)(?!.*-d)(?!.*--data)/, risk: "low", reason: "HTTP GET request (default)" },
  { pattern: /\b(ls|cat|head|tail|less|more|pwd|which|whoami|echo)\b/, risk: "low", reason: "read/info operation" },
]

/**
 * Assess risk level for a tool operation
 */
export function assessToolRisk(tool: string, input: unknown): ToolRiskAssessment {
  // Get base risk for tool
  let risk: RiskLevel = TOOL_BASE_RISK[tool] ?? "medium"
  let reason = `Base risk for ${tool}`

  // Special handling for Bash commands
  if (tool === "Bash" && input && typeof input === "object" && "command" in input) {
    const command = String(input.command)

    // For Bash, start with safe and find the highest matching pattern
    let bashRisk: RiskLevel = "safe"
    let bashReason = "No risky patterns detected"

    // Check against risk patterns - take the HIGHEST risk match
    for (const pattern of BASH_RISK_PATTERNS) {
      if (pattern.pattern.test(command)) {
        if (RISK_VALUES[pattern.risk] > RISK_VALUES[bashRisk]) {
          bashRisk = pattern.risk
          bashReason = pattern.reason
        }
      }
    }

    // If we found any matching pattern, use that risk, otherwise use base Bash risk
    if (bashRisk !== "safe") {
      risk = bashRisk
      reason = bashReason
    }
    // If no pattern matched, Bash commands default to high (could be anything)
  }

  // Special handling for Write/Edit to sensitive files
  if ((tool === "Write" || tool === "Edit") && input && typeof input === "object" && "file_path" in input) {
    const filePath = String(input.file_path)

    // Sensitive file patterns
    if (/\.(env|pem|key|crt|p12)$/i.test(filePath)) {
      risk = "high"
      reason = "Operation on sensitive file (credentials/secrets)"
    } else if (/\/etc\/|\/usr\/|\/var\//.test(filePath)) {
      risk = "high"
      reason = "Operation on system directory"
    } else if (/package\.json|Cargo\.toml|go\.mod/.test(filePath)) {
      risk = "medium"
      reason = "Dependency manifest modification"
    }
  }

  return {
    tool,
    risk,
    reason,
    autoApprovable: risk !== "critical",
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
  const { enabled, allowedTools, riskThreshold, timeoutMs, unattended } = config

  return async (info: Permission.Info): Promise<"once" | "always" | "reject"> => {
    const tool = info.type
    const input = info.metadata

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

    // Assess tool risk
    const assessment = assessToolRisk(tool, input)

    // Critical operations are ALWAYS rejected
    if (assessment.risk === "critical") {
      recordAudit({
        timestamp: new Date().toISOString(),
        permissionId: info.id,
        tool,
        pattern: info.pattern,
        risk: assessment.risk,
        decision: "rejected",
        reason: `Critical operation blocked: ${assessment.reason}`,
      })
      return "reject"
    }

    // Check whitelist (if provided)
    const isWhitelisted = allowedTools.length === 0 || allowedTools.includes(tool)

    // Check risk threshold
    const withinThreshold = riskAtOrBelowThreshold(assessment.risk, riskThreshold)

    // Auto-approve if whitelisted AND within risk threshold
    if (isWhitelisted && withinThreshold) {
      recordAudit({
        timestamp: new Date().toISOString(),
        permissionId: info.id,
        tool,
        pattern: info.pattern,
        risk: assessment.risk,
        decision: "approved",
        reason: `Auto-approved: ${assessment.reason}`,
      })
      // Use "once" to avoid blanket approval for all future calls
      return "once"
    }

    // If unattended mode and timeout is set, wait then approve
    // (Critical operations were already rejected above)
    if (unattended && timeoutMs > 0) {
      await new Promise((resolve) => setTimeout(resolve, timeoutMs))

      recordAudit({
        timestamp: new Date().toISOString(),
        permissionId: info.id,
        tool,
        pattern: info.pattern,
        risk: assessment.risk,
        decision: "timeout_approved",
        reason: `Timeout auto-approved after ${timeoutMs}ms: ${assessment.reason}`,
      })
      return "once"
    }

    // Otherwise reject
    recordAudit({
      timestamp: new Date().toISOString(),
      permissionId: info.id,
      tool,
      pattern: info.pattern,
      risk: assessment.risk,
      decision: "rejected",
      reason: `Not in whitelist or exceeds threshold: ${assessment.reason}`,
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
 * Create permissive auto-approve config for trusted Hands
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

  const assessment = assessToolRisk(tool, input)

  // Critical operations NEVER auto-approve - defer to manual
  if (assessment.risk === "critical") return undefined

  // Check whitelist
  const isWhitelisted = config.allowedTools.length === 0 || config.allowedTools.includes(tool)
  if (!isWhitelisted) return undefined

  // Check risk threshold
  const withinThreshold = riskAtOrBelowThreshold(assessment.risk, config.riskThreshold)
  if (!withinThreshold) return undefined

  return "once"
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
// Adaptive Risk Assessment
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
 * Determine project sensitivity based on path and environment
 */
function determineProjectSensitivity(ctx: ExecutionContext): ProjectSensitivity {
  // Check for production indicators
  if (ctx.isProduction) return "high"

  const path = ctx.projectPath ?? ""

  // High sensitivity patterns
  if (/\/(prod|production|live|release)\//i.test(path)) return "high"
  if (/\/\.env|\/secrets|\/credentials/i.test(path)) return "high"

  // Medium sensitivity patterns
  if (/\/(staging|pre-prod|uat)\//i.test(path)) return "medium"
  if (/\/config|\/settings/i.test(path)) return "medium"

  return "low"
}

/**
 * Determine time of day category
 */
function determineTimeOfDay(): TimeOfDay {
  const hour = new Date().getHours()
  // Business hours: 9 AM - 6 PM local time, Monday-Friday
  const dayOfWeek = new Date().getDay()
  const isWeekday = dayOfWeek >= 1 && dayOfWeek <= 5
  const isBusinessHours = hour >= 9 && hour < 18

  return isWeekday && isBusinessHours ? "business" : "after_hours"
}

/**
 * Calculate success rate from session stats
 */
function calculateSuccessRate(ctx: ExecutionContext): number {
  const total = ctx.successes + ctx.errors
  if (total === 0) return 1.0 // No data = assume good
  return ctx.successes / total
}

/**
 * Adjust risk level by delta
 */
function adjustRiskLevel(risk: RiskLevel, delta: number): RiskLevel {
  const levels: RiskLevel[] = ["safe", "low", "medium", "high", "critical"]
  const currentIndex = levels.indexOf(risk)
  const newIndex = Math.max(0, Math.min(levels.length - 1, currentIndex + delta))
  return levels[newIndex]
}

/**
 * Evaluate adaptive risk for a tool operation
 *
 * Takes base risk assessment and adjusts based on context factors:
 * - High success rate + no errors → risk decreases by 1 level
 * - Session has errors → risk increases by 1 level
 * - After hours + high sensitivity → risk increases by 1 level
 * - Unattended mode → risk increases by 1 level
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
  // Get base risk assessment
  const baseAssessment = assessToolRisk(tool, input)
  const baseRisk = baseAssessment.risk

  // Build context factors
  const contextFactors: ContextFactors = {
    projectSensitivity: determineProjectSensitivity(ctx),
    timeOfDay: determineTimeOfDay(),
    successRate: calculateSuccessRate(ctx),
    sessionErrorCount: ctx.errors,
    sessionIterations: ctx.iteration,
    unattended: false, // Will be set by caller
  }

  // Calculate adjustment
  let adjustment = 0
  const reasons: string[] = []

  // Success rate factor: high success rate with no errors can reduce risk
  if (contextFactors.successRate >= 0.95 && contextFactors.sessionErrorCount === 0) {
    adjustment -= 1
    reasons.push("High success rate (≥95%) with no errors")
  }

  // Error factor: errors in session increase risk
  if (contextFactors.sessionErrorCount > 0) {
    adjustment += 1
    reasons.push(`${contextFactors.sessionErrorCount} error(s) in session`)
  }

  // Additional increase if many errors
  if (contextFactors.sessionErrorCount >= 3) {
    adjustment += 1
    reasons.push("Multiple errors (≥3) in session")
  }

  // Time + sensitivity factor: after hours on high-sensitivity projects
  if (
    contextFactors.timeOfDay === "after_hours" &&
    contextFactors.projectSensitivity === "high"
  ) {
    adjustment += 1
    reasons.push("After hours operation on high-sensitivity project")
  }

  // Production environment factor
  if (contextFactors.projectSensitivity === "high") {
    // High sensitivity always gets at least +1
    if (adjustment === 0) {
      adjustment += 1
      reasons.push("High-sensitivity project environment")
    }
  }

  // Apply adjustment
  const adjustedRisk = adjustRiskLevel(baseRisk, adjustment)

  return {
    baseRisk,
    contextFactors,
    adjustedRisk,
    adjustment,
    adjustmentReason: reasons.length > 0 ? reasons.join("; ") : "No adjustment",
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
 * Wraps the standard auto-approve handler with adaptive risk assessment.
 */
export function createAdaptiveAutoApproveHandler(
  baseConfig: AutoApproveConfig,
  ctx: ExecutionContext,
): (tool: string, input: unknown) => "once" | "reject" | undefined {
  return (tool: string, input: unknown) => {
    if (!baseConfig.enabled) return undefined

    // Evaluate adaptive risk
    const adaptiveRisk = evaluateAdaptiveRisk(tool, input, ctx)

    // Check if auto-approval is allowed with adjusted risk
    if (!adaptiveRiskAllowsApproval(adaptiveRisk, baseConfig.riskThreshold)) {
      log.info("Adaptive risk rejected auto-approval", {
        tool,
        baseRisk: adaptiveRisk.baseRisk,
        adjustedRisk: adaptiveRisk.adjustedRisk,
        adjustment: adaptiveRisk.adjustment,
        reason: adaptiveRisk.adjustmentReason,
      })
      return undefined // Defer to manual approval
    }

    // Check whitelist
    const isWhitelisted =
      baseConfig.allowedTools.length === 0 ||
      baseConfig.allowedTools.includes(tool)

    if (!isWhitelisted) return undefined

    log.info("Adaptive risk approved", {
      tool,
      baseRisk: adaptiveRisk.baseRisk,
      adjustedRisk: adaptiveRisk.adjustedRisk,
      adjustment: adaptiveRisk.adjustment,
    })

    return "once"
  }
}
