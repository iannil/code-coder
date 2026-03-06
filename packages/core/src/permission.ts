/**
 * Permission Module - Auto-Approve Engine wrapper
 *
 * Provides a TypeScript interface to the Rust-native auto-approve engine
 * for risk-based automatic approval of tool operations.
 *
 * @example
 * ```typescript
 * import { AutoApproveEngine, AutoApproveConfig } from '@codecoder-ai/core'
 *
 * // Create an engine with custom configuration
 * const engine = AutoApproveEngine.create({
 *   enabled: true,
 *   allowedTools: ['Read', 'Glob', 'Grep'],
 *   riskThreshold: 'low',
 *   timeoutMs: 0,
 *   unattended: false,
 * })
 *
 * // Evaluate a tool operation
 * const decision = engine.evaluate('Read', { inputType: 'none' })
 * if (decision.approved) {
 *   console.log('Auto-approved:', decision.reason)
 * }
 *
 * // Quick check if a tool can be auto-approved
 * if (engine.canAutoApprove('Bash')) {
 *   // ...
 * }
 * ```
 *
 * @example Safe-only configuration
 * ```typescript
 * const engine = AutoApproveEngine.safeOnly(true) // unattended mode
 *
 * // Only Read, Glob, Grep, LS, WebFetch, WebSearch are auto-approved
 * engine.evaluate('Write') // returns { approved: false }
 * ```
 *
 * @example Adaptive risk assessment
 * ```typescript
 * const engine = AutoApproveEngine.permissive(true)
 *
 * const ctx = {
 *   sessionId: 'session-123',
 *   iteration: 5,
 *   errors: 2,
 *   successes: 10,
 *   projectPath: '/projects/my-app',
 *   isProduction: false,
 * }
 *
 * // Risk level adjusts based on session context
 * const decision = engine.evaluateAdaptive('Write', null, ctx)
 * ```
 */

// Import native bindings - fail-fast if not available
import {
  createAutoApproveEngine,
  createSafeOnlyEngine,
  createPermissiveEngine,
  evaluateAutoApprove,
  evaluateAdaptiveAutoApprove,
  canSafeAutoApprove,
  type AutoApproveEngineHandle,
  type AutoApproveConfig as NapiAutoApproveConfig,
  type ToolInput as NapiToolInput,
  type ApprovalDecision as NapiApprovalDecision,
  type ExecutionContext as NapiExecutionContext,
  type AdaptiveRiskResult as NapiAdaptiveRiskResult,
  type RiskResult as NapiRiskResult,
  type AuditEntry as NapiAuditEntry,
} from './binding.js'

// ============================================================================
// Types
// ============================================================================

/**
 * Risk levels for tool operations
 */
export type RiskLevel = 'safe' | 'low' | 'medium' | 'high' | 'critical'

/** Alias for RiskLevel to avoid conflicts when re-exported */
export type PermissionRiskLevel = RiskLevel

/**
 * Auto-approve configuration
 */
export interface AutoApproveConfig {
  /** Enable auto-approval */
  enabled: boolean

  /** Tools allowed for auto-approval (whitelist). Empty = use risk-based evaluation only */
  allowedTools: string[]

  /** Maximum risk level for auto-approval */
  riskThreshold: RiskLevel

  /** Timeout in milliseconds before auto-approving (for unattended mode) */
  timeoutMs: number

  /** Whether running in unattended mode */
  unattended: boolean
}

/**
 * Tool input for risk assessment
 */
export interface ToolInput {
  /** Input type: 'bash', 'file', 'json', or 'none' */
  inputType: 'bash' | 'file' | 'json' | 'none'

  /** Command string (for bash type) */
  command?: string

  /** File path (for file type) */
  path?: string

  /** JSON data string (for json type) */
  json?: string
}

/**
 * Approval decision result
 */
export interface ApprovalDecision {
  /** Whether the operation is approved */
  approved: boolean

  /** Risk level of the operation */
  risk: RiskLevel

  /** Reason for the decision */
  reason: string

  /** Whether this was a timeout-based approval */
  timeoutApproved: boolean

  /** Whether the operation can potentially be auto-approved (false for critical) */
  autoApprovable: boolean
}

/**
 * Execution context for adaptive risk assessment
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
  isProduction: boolean
}

/**
 * Adaptive risk assessment result
 */
export interface AdaptiveRiskResult {
  /** Base risk level */
  baseRisk: RiskLevel

  /** Adjusted risk level */
  adjustedRisk: RiskLevel

  /** Adjustment applied (+N = increased, -N = decreased) */
  adjustment: number

  /** Reason for adjustment */
  adjustmentReason: string
}

/**
 * Risk assessment result
 */
export interface RiskResult {
  /** Risk level */
  risk: RiskLevel

  /** Reason for the assessment */
  reason: string

  /** Whether the operation can be auto-approved (not critical) */
  autoApprovable: boolean
}

/**
 * Audit entry for auto-approve decisions
 */
export interface AuditEntry {
  /** ISO 8601 timestamp */
  timestamp: string

  /** Permission request ID */
  permissionId?: string

  /** Tool name */
  tool: string

  /** Associated patterns */
  pattern?: string[]

  /** Risk level */
  risk: RiskLevel

  /** Decision made */
  decision: string

  /** Reason for decision */
  reason: string
}

// ============================================================================
// Type Conversions
// ============================================================================

function toNapiConfig(config: AutoApproveConfig): NapiAutoApproveConfig {
  return {
    enabled: config.enabled,
    allowedTools: config.allowedTools,
    riskThreshold: config.riskThreshold,
    timeoutMs: config.timeoutMs,
    unattended: config.unattended,
  }
}

function fromNapiConfig(config: NapiAutoApproveConfig): AutoApproveConfig {
  return {
    enabled: config.enabled,
    allowedTools: config.allowedTools,
    riskThreshold: config.riskThreshold as RiskLevel,
    timeoutMs: config.timeoutMs,
    unattended: config.unattended,
  }
}

function toNapiToolInput(input: ToolInput | null | undefined): NapiToolInput | undefined {
  if (!input) return undefined
  return {
    inputType: input.inputType,
    command: input.command,
    path: input.path,
    json: input.json,
  }
}

function toNapiExecutionContext(ctx: ExecutionContext): NapiExecutionContext {
  return {
    sessionId: ctx.sessionId,
    iteration: ctx.iteration,
    errors: ctx.errors,
    successes: ctx.successes,
    projectPath: ctx.projectPath,
    isProduction: ctx.isProduction,
  }
}

function fromNapiDecision(decision: NapiApprovalDecision): ApprovalDecision {
  return {
    approved: decision.approved,
    risk: decision.risk as RiskLevel,
    reason: decision.reason,
    timeoutApproved: decision.timeoutApproved,
    autoApprovable: decision.autoApprovable,
  }
}

function fromNapiAdaptiveResult(result: NapiAdaptiveRiskResult): AdaptiveRiskResult {
  return {
    baseRisk: result.baseRisk as RiskLevel,
    adjustedRisk: result.adjustedRisk as RiskLevel,
    adjustment: result.adjustment,
    adjustmentReason: result.adjustmentReason,
  }
}

function fromNapiRiskResult(result: NapiRiskResult): RiskResult {
  return {
    risk: result.risk as RiskLevel,
    reason: result.reason,
    autoApprovable: result.autoApprovable,
  }
}

function fromNapiAuditEntry(entry: NapiAuditEntry): AuditEntry {
  return {
    timestamp: entry.timestamp,
    permissionId: entry.permissionId ?? undefined,
    tool: entry.tool,
    pattern: entry.pattern ?? undefined,
    risk: entry.risk as RiskLevel,
    decision: entry.decision,
    reason: entry.reason,
  }
}

// ============================================================================
// AutoApproveEngine Class
// ============================================================================

/**
 * Auto-approve permission engine
 *
 * Evaluates tool operations for automatic approval based on configuration
 * and risk assessment.
 */
export class AutoApproveEngine {
  private handle: AutoApproveEngineHandle

  private constructor(handle: AutoApproveEngineHandle) {
    this.handle = handle
  }

  /**
   * Create a new auto-approve engine with custom configuration
   */
  static create(config: AutoApproveConfig): AutoApproveEngine {
    return new AutoApproveEngine(createAutoApproveEngine(toNapiConfig(config)))
  }

  /**
   * Create an engine with safe-only configuration
   *
   * Only allows: Read, Glob, Grep, LS, WebFetch, WebSearch
   * Risk threshold: Low
   */
  static safeOnly(unattended = false): AutoApproveEngine {
    return new AutoApproveEngine(createSafeOnlyEngine(unattended))
  }

  /**
   * Create an engine with permissive configuration
   *
   * Uses risk-based evaluation only (no whitelist)
   * Risk threshold: Medium
   * Timeout: 30000ms
   */
  static permissive(unattended = false): AutoApproveEngine {
    return new AutoApproveEngine(createPermissiveEngine(unattended))
  }

  /**
   * Get the current configuration
   */
  config(): AutoApproveConfig {
    return fromNapiConfig(this.handle.config())
  }

  /**
   * Update configuration
   */
  setConfig(config: AutoApproveConfig): void {
    this.handle.setConfig(toNapiConfig(config))
  }

  /**
   * Evaluate a tool operation for auto-approval
   *
   * @param tool - Tool name (e.g., 'Read', 'Bash', 'Write')
   * @param input - Optional tool input for detailed risk assessment
   * @returns Approval decision
   */
  evaluate(tool: string, input?: ToolInput | null): ApprovalDecision {
    return fromNapiDecision(this.handle.evaluate(tool, toNapiToolInput(input)))
  }

  /**
   * Evaluate with adaptive risk assessment
   *
   * Adjusts risk level based on session context:
   * - High success rate with no errors -> decreases risk
   * - Errors in session -> increases risk
   * - High-sensitivity project -> increases risk
   *
   * @param tool - Tool name
   * @param input - Optional tool input
   * @param ctx - Execution context
   * @returns Approval decision
   */
  evaluateAdaptive(tool: string, input: ToolInput | null, ctx: ExecutionContext): ApprovalDecision {
    return fromNapiDecision(
      this.handle.evaluateAdaptive(tool, toNapiToolInput(input), toNapiExecutionContext(ctx)),
    )
  }

  /**
   * Quick check if a tool can be auto-approved
   *
   * Does not perform full risk assessment, only checks:
   * - If enabled
   * - If tool is whitelisted
   * - If base risk is within threshold
   *
   * @param tool - Tool name
   * @returns Whether the tool can potentially be auto-approved
   */
  canAutoApprove(tool: string): boolean {
    return this.handle.canAutoApprove(tool)
  }

  /**
   * Assess risk for a tool operation
   *
   * @param tool - Tool name
   * @param input - Optional tool input
   * @returns Risk assessment result
   */
  assessRisk(tool: string, input?: ToolInput | null): RiskResult {
    return fromNapiRiskResult(this.handle.assessRisk(tool, toNapiToolInput(input)))
  }

  /**
   * Evaluate adaptive risk without making an approval decision
   *
   * @param tool - Tool name
   * @param input - Optional tool input
   * @param ctx - Execution context
   * @returns Adaptive risk result
   */
  evaluateAdaptiveRisk(tool: string, input: ToolInput | null, ctx: ExecutionContext): AdaptiveRiskResult {
    return fromNapiAdaptiveResult(
      this.handle.evaluateAdaptiveRisk(tool, toNapiToolInput(input), toNapiExecutionContext(ctx)),
    )
  }

  /**
   * Get the audit log
   */
  auditLog(): AuditEntry[] {
    return this.handle.auditLog().map(fromNapiAuditEntry)
  }

  /**
   * Clear the audit log
   */
  clearAuditLog(): void {
    this.handle.clearAuditLog()
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Evaluate a tool operation for auto-approval (stateless)
 *
 * Creates a temporary engine with the given config and evaluates.
 *
 * @param config - Auto-approve configuration
 * @param tool - Tool name
 * @param input - Optional tool input
 * @returns Approval decision
 */
export function evaluateToolApproval(
  config: AutoApproveConfig,
  tool: string,
  input?: ToolInput | null,
): ApprovalDecision {
  return fromNapiDecision(evaluateAutoApprove(toNapiConfig(config), tool, toNapiToolInput(input)))
}

/**
 * Evaluate with adaptive risk (stateless)
 *
 * @param config - Auto-approve configuration
 * @param tool - Tool name
 * @param input - Optional tool input
 * @param ctx - Execution context
 * @returns Approval decision
 */
export function evaluateAdaptiveToolApproval(
  config: AutoApproveConfig,
  tool: string,
  input: ToolInput | null,
  ctx: ExecutionContext,
): ApprovalDecision {
  return fromNapiDecision(
    evaluateAdaptiveAutoApprove(toNapiConfig(config), tool, toNapiToolInput(input), toNapiExecutionContext(ctx)),
  )
}

/**
 * Quick check if a tool can be auto-approved with safe-only config
 *
 * @param tool - Tool name
 * @returns Whether the tool can be auto-approved
 */
export function canToolBeSafeAutoApproved(tool: string): boolean {
  return canSafeAutoApprove(tool)
}

// Native bindings are always available
export const isPermissionNative = true
