/**
 * Structured Audit Log
 *
 * Re-exports the audit functionality from @codecoder-ai/core.
 * The core package handles native/fallback implementation internally.
 *
 * @package audit
 */

// Re-export everything from @codecoder-ai/core/audit
export {
  AuditLog,
  createAuditLog,
  getAuditLog,
  logPermission,
  logToolCall,
  logSession,
  type AuditEntry,
  type AuditEntryInput,
  type AuditEntryType,
  type AuditFilter,
  type AuditReport,
  type AuditSummary,
  type AuditResultType as AuditResult,
  type RiskLevel,
  type TypeSummary,
  type ResultSummary,
  type RiskSummary,
  type TimeRange,
} from "@codecoder-ai/core"

// Re-export with alias for backward compatibility
import { AuditLog } from "@codecoder-ai/core"
export { AuditLog as AuditLogClass }

// Re-export schema for validation
import z from "zod"

export const AuditEntrySchema = z.object({
  id: z.string(),
  timestamp: z.number(),
  sessionId: z.string(),
  type: z.enum([
    "permission",
    "tool_call",
    "decision",
    "state_change",
    "checkpoint",
    "rollback",
    "error",
    "session_start",
    "session_end",
  ]),
  action: z.string(),
  input: z.unknown(),
  result: z.enum(["approved", "rejected", "error", "success", "failed"]),
  risk: z.enum(["safe", "low", "medium", "high", "critical"]).optional(),
  autoApproved: z.boolean().optional(),
  reason: z.string(),
  metadata: z.record(z.string(), z.unknown()),
})
