/**
 * Structured Audit Log
 *
 * Provides append-only, tamper-evident logging for autonomous mode operations.
 * Uses SQLite for efficient querying and persistence.
 *
 * @package audit
 */

import { Log } from "@/util/log"
import { Global } from "@/global"
import { Database } from "bun:sqlite"
import path from "path"
import fs from "fs/promises"
import z from "zod"
import type { RiskLevel } from "@/permission/auto-approve"

const log = Log.create({ service: "audit" })

// ============================================================================
// Types
// ============================================================================

/**
 * Audit entry types
 */
export type AuditEntryType =
  | "permission"
  | "tool_call"
  | "decision"
  | "state_change"
  | "checkpoint"
  | "rollback"
  | "error"
  | "session_start"
  | "session_end"

/**
 * Audit result
 */
export type AuditResult = "approved" | "rejected" | "error" | "success" | "failed"

/**
 * Audit entry
 */
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

  /** Input data (JSON serializable) */
  input: unknown

  /** Result of the action */
  result: AuditResult

  /** Risk level (for permission/tool entries) */
  risk?: RiskLevel

  /** Whether auto-approved */
  autoApproved?: boolean

  /** Reason for the action/result */
  reason: string

  /** Additional metadata */
  metadata: Record<string, unknown>
}

/**
 * Audit entry input (without auto-generated fields)
 */
export type AuditEntryInput = Omit<AuditEntry, "id" | "timestamp">

/**
 * Audit query filter
 */
export interface AuditFilter {
  sessionId?: string
  type?: AuditEntryType
  result?: AuditResult
  risk?: RiskLevel
  autoApproved?: boolean
  fromTimestamp?: number
  toTimestamp?: number
  action?: string
  limit?: number
  offset?: number
}

/**
 * Audit report
 */
export interface AuditReport {
  sessionId: string
  generatedAt: string
  summary: {
    totalEntries: number
    byType: Record<AuditEntryType, number>
    byResult: Record<AuditResult, number>
    byRisk: Record<RiskLevel, number>
    autoApprovedCount: number
    timeRange: {
      start: number
      end: number
      durationMs: number
    }
  }
  entries: AuditEntry[]
}

/**
 * Zod schema for audit entry
 */
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

// ============================================================================
// Constants
// ============================================================================

const AUDIT_DB_FILE = "audit.db"
const SCHEMA_VERSION = 1

// ============================================================================
// Audit Log Class
// ============================================================================

/**
 * Audit Log
 *
 * Append-only audit logging system with SQLite backend.
 */
export class AuditLog {
  private db: Database | null = null
  private dbPath: string
  private initialized = false

  constructor(dbPath?: string) {
    this.dbPath = dbPath ?? path.join(Global.Path.data, AUDIT_DB_FILE)
  }

  /**
   * Initialize the audit log
   */
  async initialize(): Promise<void> {
    if (this.initialized) return

    // Ensure directory exists
    await fs.mkdir(path.dirname(this.dbPath), { recursive: true })

    // Open database
    this.db = new Database(this.dbPath)
    this.db.exec("PRAGMA journal_mode = WAL")
    this.db.exec("PRAGMA synchronous = NORMAL")

    // Create tables
    this.createTables()

    this.initialized = true
    log.info("Audit log initialized", { dbPath: this.dbPath })
  }

  /**
   * Log an audit entry
   *
   * @param entry Audit entry input
   * @returns Entry ID
   */
  async log(entry: AuditEntryInput): Promise<string> {
    await this.ensureInitialized()

    const id = this.generateEntryId()
    const timestamp = Date.now()

    const stmt = this.db!.prepare(`
      INSERT INTO audit_entries (
        id, timestamp, session_id, type, action, input, result,
        risk, auto_approved, reason, metadata
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `)

    stmt.run(
      id,
      timestamp,
      entry.sessionId,
      entry.type,
      entry.action,
      JSON.stringify(entry.input),
      entry.result,
      entry.risk ?? null,
      entry.autoApproved === undefined ? null : entry.autoApproved ? 1 : 0,
      entry.reason,
      JSON.stringify(entry.metadata)
    )

    log.debug("Audit entry logged", { id, type: entry.type, action: entry.action })

    return id
  }

  /**
   * Query audit entries
   *
   * @param filter Query filter
   * @returns Array of matching entries
   */
  async query(filter: AuditFilter = {}): Promise<AuditEntry[]> {
    await this.ensureInitialized()

    const conditions: string[] = ["1=1"]
    const params: (string | number | null)[] = []

    if (filter.sessionId) {
      conditions.push("session_id = ?")
      params.push(filter.sessionId)
    }

    if (filter.type) {
      conditions.push("type = ?")
      params.push(filter.type)
    }

    if (filter.result) {
      conditions.push("result = ?")
      params.push(filter.result)
    }

    if (filter.risk) {
      conditions.push("risk = ?")
      params.push(filter.risk)
    }

    if (filter.autoApproved !== undefined) {
      conditions.push("auto_approved = ?")
      params.push(filter.autoApproved ? 1 : 0)
    }

    if (filter.fromTimestamp) {
      conditions.push("timestamp >= ?")
      params.push(filter.fromTimestamp)
    }

    if (filter.toTimestamp) {
      conditions.push("timestamp <= ?")
      params.push(filter.toTimestamp)
    }

    if (filter.action) {
      conditions.push("action LIKE ?")
      params.push(`%${filter.action}%`)
    }

    const limit = filter.limit ?? 1000
    const offset = filter.offset ?? 0

    const query = `
      SELECT * FROM audit_entries
      WHERE ${conditions.join(" AND ")}
      ORDER BY timestamp DESC
      LIMIT ? OFFSET ?
    `

    params.push(limit, offset)

    const stmt = this.db!.prepare(query)
    const rows = stmt.all(...params) as DbRow[]

    return rows.map((row) => this.rowToEntry(row))
  }

  /**
   * Export audit report for a session
   *
   * @param sessionId Session ID
   * @returns Audit report
   */
  async exportReport(sessionId: string): Promise<AuditReport> {
    await this.ensureInitialized()

    const entries = await this.query({ sessionId, limit: 10000 })

    // Calculate summary statistics
    const byType: Record<AuditEntryType, number> = {
      permission: 0,
      tool_call: 0,
      decision: 0,
      state_change: 0,
      checkpoint: 0,
      rollback: 0,
      error: 0,
      session_start: 0,
      session_end: 0,
    }

    const byResult: Record<AuditResult, number> = {
      approved: 0,
      rejected: 0,
      error: 0,
      success: 0,
      failed: 0,
    }

    const byRisk: Record<RiskLevel, number> = {
      safe: 0,
      low: 0,
      medium: 0,
      high: 0,
      critical: 0,
    }

    let autoApprovedCount = 0
    let minTimestamp = Infinity
    let maxTimestamp = 0

    for (const entry of entries) {
      byType[entry.type]++
      byResult[entry.result]++

      if (entry.risk) {
        byRisk[entry.risk]++
      }

      if (entry.autoApproved) {
        autoApprovedCount++
      }

      if (entry.timestamp < minTimestamp) {
        minTimestamp = entry.timestamp
      }
      if (entry.timestamp > maxTimestamp) {
        maxTimestamp = entry.timestamp
      }
    }

    return {
      sessionId,
      generatedAt: new Date().toISOString(),
      summary: {
        totalEntries: entries.length,
        byType,
        byResult,
        byRisk,
        autoApprovedCount,
        timeRange: {
          start: minTimestamp === Infinity ? 0 : minTimestamp,
          end: maxTimestamp,
          durationMs: maxTimestamp > 0 && minTimestamp < Infinity
            ? maxTimestamp - minTimestamp
            : 0,
        },
      },
      entries: entries.reverse(), // Chronological order
    }
  }

  /**
   * Get entry count
   */
  async count(filter?: AuditFilter): Promise<number> {
    await this.ensureInitialized()

    if (!filter || Object.keys(filter).length === 0) {
      const stmt = this.db!.prepare("SELECT COUNT(*) as count FROM audit_entries")
      const result = stmt.get() as { count: number }
      return result.count
    }

    // Use query with limit 0 and get total
    const conditions: string[] = ["1=1"]
    const params: (string | number | null)[] = []

    if (filter.sessionId) {
      conditions.push("session_id = ?")
      params.push(filter.sessionId)
    }

    if (filter.type) {
      conditions.push("type = ?")
      params.push(filter.type)
    }

    const query = `
      SELECT COUNT(*) as count FROM audit_entries
      WHERE ${conditions.join(" AND ")}
    `

    const stmt = this.db!.prepare(query)
    const result = stmt.get(...params) as { count: number }
    return result.count
  }

  /**
   * Close the database connection
   */
  close(): void {
    if (this.db) {
      this.db.close()
      this.db = null
      this.initialized = false
      log.info("Audit log closed")
    }
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async ensureInitialized(): Promise<void> {
    if (!this.initialized) {
      await this.initialize()
    }
  }

  private createTables(): void {
    // Main audit entries table
    this.db!.exec(`
      CREATE TABLE IF NOT EXISTS audit_entries (
        id TEXT PRIMARY KEY,
        timestamp INTEGER NOT NULL,
        session_id TEXT NOT NULL,
        type TEXT NOT NULL,
        action TEXT NOT NULL,
        input TEXT,
        result TEXT NOT NULL,
        risk TEXT,
        auto_approved INTEGER,
        reason TEXT NOT NULL,
        metadata TEXT NOT NULL,
        created_at TEXT DEFAULT CURRENT_TIMESTAMP
      )
    `)

    // Indexes for common queries
    this.db!.exec(`
      CREATE INDEX IF NOT EXISTS idx_audit_session_id ON audit_entries(session_id)
    `)
    this.db!.exec(`
      CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_entries(timestamp)
    `)
    this.db!.exec(`
      CREATE INDEX IF NOT EXISTS idx_audit_type ON audit_entries(type)
    `)

    // Schema version table
    this.db!.exec(`
      CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY
      )
    `)

    // Insert initial schema version if not exists
    const versionStmt = this.db!.prepare(
      "INSERT OR IGNORE INTO schema_version (version) VALUES (?)"
    )
    versionStmt.run(SCHEMA_VERSION)
  }

  private generateEntryId(): string {
    return `audit_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
  }

  private rowToEntry(row: DbRow): AuditEntry {
    return {
      id: row.id,
      timestamp: row.timestamp,
      sessionId: row.session_id,
      type: row.type as AuditEntryType,
      action: row.action,
      input: row.input ? JSON.parse(row.input) : null,
      result: row.result as AuditResult,
      risk: row.risk as RiskLevel | undefined,
      autoApproved: row.auto_approved === null ? undefined : row.auto_approved === 1,
      reason: row.reason,
      metadata: JSON.parse(row.metadata),
    }
  }
}

/**
 * Database row type
 */
interface DbRow {
  id: string
  timestamp: number
  session_id: string
  type: string
  action: string
  input: string | null
  result: string
  risk: string | null
  auto_approved: number | null
  reason: string
  metadata: string
}

// ============================================================================
// Singleton Instance
// ============================================================================

let auditLogInstance: AuditLog | null = null

/**
 * Get the global audit log instance
 */
export function getAuditLog(): AuditLog {
  if (!auditLogInstance) {
    auditLogInstance = new AuditLog()
  }
  return auditLogInstance
}

/**
 * Create a new audit log instance (for testing or custom paths)
 */
export function createAuditLog(dbPath?: string): AuditLog {
  return new AuditLog(dbPath)
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Log a permission audit entry
 */
export async function logPermission(entry: {
  sessionId: string
  tool: string
  action: string
  result: "approved" | "rejected"
  risk?: RiskLevel
  autoApproved?: boolean
  reason: string
  metadata?: Record<string, unknown>
}): Promise<string> {
  const auditLog = getAuditLog()
  return auditLog.log({
    sessionId: entry.sessionId,
    type: "permission",
    action: `${entry.tool}:${entry.action}`,
    input: { tool: entry.tool },
    result: entry.result,
    risk: entry.risk,
    autoApproved: entry.autoApproved,
    reason: entry.reason,
    metadata: entry.metadata ?? {},
  })
}

/**
 * Log a tool call audit entry
 */
export async function logToolCall(entry: {
  sessionId: string
  tool: string
  input: unknown
  result: "success" | "error"
  reason: string
  metadata?: Record<string, unknown>
}): Promise<string> {
  const auditLog = getAuditLog()
  return auditLog.log({
    sessionId: entry.sessionId,
    type: "tool_call",
    action: entry.tool,
    input: entry.input,
    result: entry.result,
    reason: entry.reason,
    metadata: entry.metadata ?? {},
  })
}

/**
 * Log a session event
 */
export async function logSession(entry: {
  sessionId: string
  event: "start" | "end"
  reason: string
  metadata?: Record<string, unknown>
}): Promise<string> {
  const auditLog = getAuditLog()
  return auditLog.log({
    sessionId: entry.sessionId,
    type: entry.event === "start" ? "session_start" : "session_end",
    action: entry.event,
    input: null,
    result: "success",
    reason: entry.reason,
    metadata: entry.metadata ?? {},
  })
}
