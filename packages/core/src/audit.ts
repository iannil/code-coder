/**
 * Audit module - Structured audit logging
 *
 * Provides append-only, tamper-evident logging for autonomous mode operations.
 * Uses native Rust implementation - no JavaScript fallback.
 *
 * @example
 * ```typescript
 * import { AuditLog, logPermission, logToolCall } from '@codecoder-ai/core/audit'
 *
 * // Create audit log
 * const audit = new AuditLog()
 *
 * // Log permission check
 * await logPermission({
 *   sessionId: 'session-123',
 *   tool: 'file',
 *   action: 'read',
 *   result: 'approved',
 *   risk: 'safe',
 *   autoApproved: true,
 *   reason: 'Safe read operation'
 * })
 *
 * // Query entries
 * const entries = await audit.query({ sessionId: 'session-123' })
 *
 * // Export report
 * const report = await audit.exportReport('session-123')
 * ```
 */

// Import native audit log directly - fail-fast if not available
// Note: const enum types are imported as type-only to avoid verbatimModuleSyntax issues
import { NapiAuditLog, type NapiAuditEntryType, type NapiAuditResult, type NapiRiskLevel } from './binding.js'

// ============================================================================
// Types
// ============================================================================

/** Audit entry types */
export type AuditEntryType =
  | 'permission'
  | 'tool_call'
  | 'decision'
  | 'state_change'
  | 'checkpoint'
  | 'rollback'
  | 'error'
  | 'session_start'
  | 'session_end'

/** Audit result */
export type AuditResultType = 'approved' | 'rejected' | 'error' | 'success' | 'failed'

/** Risk level */
export type RiskLevel = 'safe' | 'low' | 'medium' | 'high' | 'critical'

/** Audit entry */
export interface AuditEntry {
  /** Unique entry ID */
  id: string
  /** Timestamp (Unix ms) */
  timestamp: number
  /** Session ID */
  sessionId: string
  /** Entry type */
  type: AuditEntryType
  /** Action performed */
  action: string
  /** Input data */
  input: unknown
  /** Result of the action */
  result: AuditResultType
  /** Risk level */
  risk?: RiskLevel
  /** Whether auto-approved */
  autoApproved?: boolean
  /** Reason for the action/result */
  reason: string
  /** Additional metadata */
  metadata: Record<string, unknown>
}

/** Audit entry input (without auto-generated fields) */
export interface AuditEntryInput {
  sessionId: string
  type: AuditEntryType
  action: string
  input: unknown
  result: AuditResultType
  risk?: RiskLevel
  autoApproved?: boolean
  reason: string
  metadata?: Record<string, unknown>
}

/** Audit query filter */
export interface AuditFilter {
  sessionId?: string
  type?: AuditEntryType
  result?: AuditResultType
  risk?: RiskLevel
  autoApproved?: boolean
  fromTimestamp?: number
  toTimestamp?: number
  action?: string
  limit?: number
  offset?: number
}

/** Time range */
export interface TimeRange {
  start: number
  end: number
  durationMs: number
}

/** Summary by type */
export interface TypeSummary {
  permission: number
  tool_call: number
  decision: number
  state_change: number
  checkpoint: number
  rollback: number
  error: number
  session_start: number
  session_end: number
}

/** Summary by result */
export interface ResultSummary {
  approved: number
  rejected: number
  error: number
  success: number
  failed: number
}

/** Summary by risk */
export interface RiskSummary {
  safe: number
  low: number
  medium: number
  high: number
  critical: number
}

/** Audit summary */
export interface AuditSummary {
  totalEntries: number
  byType: TypeSummary
  byResult: ResultSummary
  byRisk: RiskSummary
  autoApprovedCount: number
  timeRange: TimeRange
}

/** Audit report */
export interface AuditReport {
  sessionId: string
  generatedAt: string
  summary: AuditSummary
  entries: AuditEntry[]
}

// ============================================================================
// Export Native Audit Log
// ============================================================================

/** Audit log class (native Rust implementation) */
export const AuditLog = NapiAuditLog

// ============================================================================
// Type Conversion Helpers
// ============================================================================

// NAPI enum values as string literals (matching binding.d.ts const enum definitions)
// Using string literals to avoid const enum import issues with verbatimModuleSyntax
const EntryType = {
  Permission: 'Permission' as NapiAuditEntryType,
  ToolCall: 'ToolCall' as NapiAuditEntryType,
  Decision: 'Decision' as NapiAuditEntryType,
  StateChange: 'StateChange' as NapiAuditEntryType,
  Checkpoint: 'Checkpoint' as NapiAuditEntryType,
  Rollback: 'Rollback' as NapiAuditEntryType,
  Error: 'Error' as NapiAuditEntryType,
  SessionStart: 'SessionStart' as NapiAuditEntryType,
  SessionEnd: 'SessionEnd' as NapiAuditEntryType,
} as const

const Result = {
  Approved: 'Approved' as NapiAuditResult,
  Rejected: 'Rejected' as NapiAuditResult,
  Error: 'Error' as NapiAuditResult,
  Success: 'Success' as NapiAuditResult,
  Failed: 'Failed' as NapiAuditResult,
} as const

const Risk = {
  Safe: 'Safe' as NapiRiskLevel,
  Low: 'Low' as NapiRiskLevel,
  Medium: 'Medium' as NapiRiskLevel,
  High: 'High' as NapiRiskLevel,
  Critical: 'Critical' as NapiRiskLevel,
} as const

function toNapiRiskLevel(risk: RiskLevel): NapiRiskLevel {
  const map: Record<RiskLevel, NapiRiskLevel> = {
    safe: Risk.Safe,
    low: Risk.Low,
    medium: Risk.Medium,
    high: Risk.High,
    critical: Risk.Critical,
  }
  return map[risk]
}

// ============================================================================
// Singleton and Convenience Functions
// ============================================================================

let auditLogInstance: InstanceType<typeof NapiAuditLog> | null = null

/** Get the global audit log instance */
export function getAuditLog(): InstanceType<typeof NapiAuditLog> {
  if (!auditLogInstance) {
    auditLogInstance = new NapiAuditLog()
  }
  return auditLogInstance
}

/** Create a new audit log instance */
export function createAuditLog(): InstanceType<typeof NapiAuditLog> {
  return new NapiAuditLog()
}

/** Log a permission audit entry */
export async function logPermission(entry: {
  sessionId: string
  tool: string
  action: string
  result: 'approved' | 'rejected'
  risk?: RiskLevel
  autoApproved?: boolean
  reason: string
  metadata?: Record<string, unknown>
}): Promise<string> {
  const auditLog = getAuditLog()
  return auditLog.log({
    sessionId: entry.sessionId,
    entryType: EntryType.Permission,
    action: `${entry.tool}:${entry.action}`,
    input: JSON.stringify({ tool: entry.tool }),
    result: entry.result === 'approved' ? Result.Approved : Result.Rejected,
    risk: entry.risk ? toNapiRiskLevel(entry.risk) : undefined,
    autoApproved: entry.autoApproved,
    reason: entry.reason,
    metadata: JSON.stringify(entry.metadata ?? {}),
  })
}

/** Log a tool call audit entry */
export async function logToolCall(entry: {
  sessionId: string
  tool: string
  input: unknown
  result: 'success' | 'error'
  reason: string
  metadata?: Record<string, unknown>
}): Promise<string> {
  const auditLog = getAuditLog()
  return auditLog.log({
    sessionId: entry.sessionId,
    entryType: EntryType.ToolCall,
    action: entry.tool,
    input: JSON.stringify(entry.input),
    result: entry.result === 'success' ? Result.Success : Result.Error,
    reason: entry.reason,
    metadata: JSON.stringify(entry.metadata ?? {}),
  })
}

/** Log a session event */
export async function logSession(entry: {
  sessionId: string
  event: 'start' | 'end'
  reason: string
  metadata?: Record<string, unknown>
}): Promise<string> {
  const auditLog = getAuditLog()
  return auditLog.log({
    sessionId: entry.sessionId,
    entryType: entry.event === 'start' ? EntryType.SessionStart : EntryType.SessionEnd,
    action: entry.event,
    input: JSON.stringify(null),
    result: Result.Success,
    reason: entry.reason,
    metadata: JSON.stringify(entry.metadata ?? {}),
  })
}
