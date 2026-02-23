/**
 * Compliance and Audit API Handler
 *
 * Provides audit log viewing and compliance report generation for regulatory requirements.
 * Supports 5-year retention for financial compliance.
 *
 * GET  /api/v1/compliance/logs - List audit log entries
 * GET  /api/v1/compliance/logs/:id - Get specific audit entry
 * POST /api/v1/compliance/report - Generate compliance report
 * GET  /api/v1/compliance/reports - List generated reports
 * GET  /api/v1/compliance/reports/:id - Get specific report
 * GET  /api/v1/compliance/status - Get compliance status
 * POST /api/v1/compliance/export - Export audit data
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { createHash } from "crypto"

// ============================================================================
// Types
// ============================================================================

export type AuditEventType =
  | "prompt_submitted"
  | "response_generated"
  | "tool_invoked"
  | "decision_made"
  | "config_changed"
  | "user_authenticated"
  | "session_started"
  | "session_ended"
  | "data_exported"
  | "error_occurred"

export interface AuditEntry {
  id: string
  timestamp: string
  event_type: AuditEventType
  user_id: string
  session_id?: string
  request_id?: string
  description: string
  content_hash?: string
  model?: string
  tokens?: {
    input: number
    output: number
    total: number
  }
  metadata: Record<string, unknown>
  prev_hash?: string
  entry_hash: string
}

export interface ComplianceReport {
  id: string
  generated_at: string
  period_start: string
  period_end: string
  total_entries: number
  entries_by_type: Record<string, number>
  entries_by_user: Record<string, number>
  chain_integrity_valid: boolean
  broken_chain_entries: string[]
  compliance_standard: string
  retention_compliant: boolean
  oldest_entry?: string
  statistics: AuditStatistics
}

export interface AuditStatistics {
  total_prompts: number
  total_responses: number
  total_tool_invocations: number
  total_tokens_input: number
  total_tokens_output: number
  unique_users: number
  unique_sessions: number
  error_count: number
}

export interface ComplianceStatus {
  audit_enabled: boolean
  retention_years: number
  compliance_standard: string
  chain_integrity_enabled: boolean
  total_entries: number
  oldest_entry?: string
  storage_size_mb: number
  last_cleanup?: string
  issues: ComplianceIssue[]
}

export interface ComplianceIssue {
  severity: "critical" | "warning" | "info"
  type: string
  description: string
  recommendation: string
}

// ============================================================================
// In-Memory Store (Production would use SQLite/Postgres)
// ============================================================================

const auditEntries: Map<string, AuditEntry> = new Map()
const complianceReports: Map<string, ComplianceReport> = new Map()
let lastHash: string | undefined

// ============================================================================
// Helper Functions
// ============================================================================

function generateId(): string {
  return `audit_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`
}

function hashContent(content: string): string {
  return createHash("sha256").update(content).digest("hex")
}

function calculateEntryHash(entry: Omit<AuditEntry, "entry_hash">): string {
  const data = `${entry.id}${entry.timestamp}${entry.event_type}${entry.user_id}${entry.description}${entry.content_hash ?? ""}${entry.prev_hash ?? ""}`
  return createHash("sha256").update(data).digest("hex")
}

// ============================================================================
// Audit Logging Function (for internal use)
// ============================================================================

export function logAuditEvent(event: {
  event_type: AuditEventType
  user_id: string
  description: string
  session_id?: string
  request_id?: string
  content?: string
  model?: string
  tokens?: { input: number; output: number }
  metadata?: Record<string, unknown>
}): string {
  const id = generateId()
  const timestamp = new Date().toISOString()

  const entry: AuditEntry = {
    id,
    timestamp,
    event_type: event.event_type,
    user_id: event.user_id,
    session_id: event.session_id,
    request_id: event.request_id,
    description: event.description,
    content_hash: event.content ? hashContent(event.content) : undefined,
    model: event.model,
    tokens: event.tokens
      ? {
          input: event.tokens.input,
          output: event.tokens.output,
          total: event.tokens.input + event.tokens.output,
        }
      : undefined,
    metadata: event.metadata ?? {},
    prev_hash: lastHash,
    entry_hash: "",
  }

  entry.entry_hash = calculateEntryHash(entry)
  lastHash = entry.entry_hash

  auditEntries.set(id, entry)

  return id
}

// ============================================================================
// Handlers
// ============================================================================

export async function handleListLogs(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const limit = parseInt(url.searchParams.get("limit") ?? "100", 10)
    const offset = parseInt(url.searchParams.get("offset") ?? "0", 10)
    const startDate = url.searchParams.get("start_date")
    const endDate = url.searchParams.get("end_date")
    const eventType = url.searchParams.get("event_type") as AuditEventType | null
    const userId = url.searchParams.get("user_id")

    let entries = Array.from(auditEntries.values())

    // Apply filters
    if (startDate) {
      const start = new Date(startDate)
      entries = entries.filter((e) => new Date(e.timestamp) >= start)
    }
    if (endDate) {
      const end = new Date(endDate)
      entries = entries.filter((e) => new Date(e.timestamp) <= end)
    }
    if (eventType) {
      entries = entries.filter((e) => e.event_type === eventType)
    }
    if (userId) {
      entries = entries.filter((e) => e.user_id === userId)
    }

    // Sort by timestamp descending
    entries.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())

    // Paginate
    const paginated = entries.slice(offset, offset + limit)

    return jsonResponse({
      success: true,
      entries: paginated,
      total: entries.length,
      limit,
      offset,
    })
  } catch (error) {
    return errorResponse(`Failed to list audit logs: ${error}`, 500)
  }
}

export async function handleGetLog(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const id = params.id
  if (!id) return errorResponse("Audit entry ID required", 400)

  const entry = auditEntries.get(id)
  if (!entry) return errorResponse("Audit entry not found", 404)

  return jsonResponse({ success: true, entry })
}

export async function handleGenerateReport(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = (await req.json()) as {
      start_date: string
      end_date: string
      compliance_standard?: string
    }

    if (!body.start_date || !body.end_date) {
      return errorResponse("Missing required fields: start_date, end_date", 400)
    }

    const start = new Date(body.start_date)
    const end = new Date(body.end_date)

    // Filter entries in range
    const entries = Array.from(auditEntries.values()).filter((e) => {
      const ts = new Date(e.timestamp)
      return ts >= start && ts <= end
    })

    // Calculate statistics
    const entriesByType: Record<string, number> = {}
    const entriesByUser: Record<string, number> = {}
    const sessions = new Set<string>()
    const brokenChain: string[] = []

    const stats: AuditStatistics = {
      total_prompts: 0,
      total_responses: 0,
      total_tool_invocations: 0,
      total_tokens_input: 0,
      total_tokens_output: 0,
      unique_users: 0,
      unique_sessions: 0,
      error_count: 0,
    }

    let prevHash: string | undefined

    for (const entry of entries.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())) {
      // Count by type
      entriesByType[entry.event_type] = (entriesByType[entry.event_type] ?? 0) + 1

      // Count by user
      entriesByUser[entry.user_id] = (entriesByUser[entry.user_id] ?? 0) + 1

      // Track sessions
      if (entry.session_id) sessions.add(entry.session_id)

      // Update stats
      switch (entry.event_type) {
        case "prompt_submitted":
          stats.total_prompts++
          break
        case "response_generated":
          stats.total_responses++
          break
        case "tool_invoked":
          stats.total_tool_invocations++
          break
        case "error_occurred":
          stats.error_count++
          break
      }

      if (entry.tokens) {
        stats.total_tokens_input += entry.tokens.input
        stats.total_tokens_output += entry.tokens.output
      }

      // Verify chain integrity
      if (entry.prev_hash !== prevHash) {
        brokenChain.push(entry.id)
      }
      prevHash = entry.entry_hash
    }

    stats.unique_users = Object.keys(entriesByUser).length
    stats.unique_sessions = sessions.size

    const oldestEntry = entries[0]?.timestamp

    // Check retention compliance (5 years for financial)
    const retentionYears = body.compliance_standard === "financial" ? 5 : 3
    const retentionCutoff = new Date()
    retentionCutoff.setFullYear(retentionCutoff.getFullYear() - retentionYears)
    const retentionCompliant = !oldestEntry || new Date(oldestEntry) >= retentionCutoff

    const report: ComplianceReport = {
      id: `report_${Date.now()}`,
      generated_at: new Date().toISOString(),
      period_start: body.start_date,
      period_end: body.end_date,
      total_entries: entries.length,
      entries_by_type: entriesByType,
      entries_by_user: entriesByUser,
      chain_integrity_valid: brokenChain.length === 0,
      broken_chain_entries: brokenChain,
      compliance_standard: body.compliance_standard ?? "general",
      retention_compliant: retentionCompliant,
      oldest_entry: oldestEntry,
      statistics: stats,
    }

    complianceReports.set(report.id, report)

    return jsonResponse({ success: true, report })
  } catch (error) {
    return errorResponse(`Failed to generate report: ${error}`, 500)
  }
}

export async function handleListReports(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const limit = parseInt(url.searchParams.get("limit") ?? "20", 10)

    const reports = Array.from(complianceReports.values())
      .sort((a, b) => new Date(b.generated_at).getTime() - new Date(a.generated_at).getTime())
      .slice(0, limit)

    return jsonResponse({
      success: true,
      reports: reports.map((r) => ({
        id: r.id,
        generated_at: r.generated_at,
        period_start: r.period_start,
        period_end: r.period_end,
        total_entries: r.total_entries,
        chain_integrity_valid: r.chain_integrity_valid,
        compliance_standard: r.compliance_standard,
      })),
      total: complianceReports.size,
    })
  } catch (error) {
    return errorResponse(`Failed to list reports: ${error}`, 500)
  }
}

export async function handleGetReport(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const id = params.id
  if (!id) return errorResponse("Report ID required", 400)

  const report = complianceReports.get(id)
  if (!report) return errorResponse("Report not found", 404)

  return jsonResponse({ success: true, report })
}

export async function handleGetStatus(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const entries = Array.from(auditEntries.values())
    const oldestEntry = entries.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())[0]

    const issues: ComplianceIssue[] = []

    // Check for common compliance issues
    if (entries.length === 0) {
      issues.push({
        severity: "warning",
        type: "no_audit_data",
        description: "No audit data found",
        recommendation: "Enable audit logging for all user interactions",
      })
    }

    // Estimate storage size (rough estimate based on entry count)
    const avgEntrySizeBytes = 500
    const storageSizeMb = (entries.length * avgEntrySizeBytes) / (1024 * 1024)

    const status: ComplianceStatus = {
      audit_enabled: true,
      retention_years: 5,
      compliance_standard: "financial",
      chain_integrity_enabled: true,
      total_entries: entries.length,
      oldest_entry: oldestEntry?.timestamp,
      storage_size_mb: Math.round(storageSizeMb * 100) / 100,
      issues,
    }

    return jsonResponse({ success: true, status })
  } catch (error) {
    return errorResponse(`Failed to get status: ${error}`, 500)
  }
}

export async function handleExport(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = (await req.json()) as {
      start_date: string
      end_date: string
      format?: "json" | "csv"
      include_content_hashes?: boolean
    }

    if (!body.start_date || !body.end_date) {
      return errorResponse("Missing required fields: start_date, end_date", 400)
    }

    const start = new Date(body.start_date)
    const end = new Date(body.end_date)
    const format = body.format ?? "json"

    const entries = Array.from(auditEntries.values())
      .filter((e) => {
        const ts = new Date(e.timestamp)
        return ts >= start && ts <= end
      })
      .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())

    // Log the export action
    logAuditEvent({
      event_type: "data_exported",
      user_id: "system",
      description: `Exported ${entries.length} audit entries from ${body.start_date} to ${body.end_date}`,
      metadata: { format, entry_count: entries.length },
    })

    if (format === "csv") {
      const headers = ["id", "timestamp", "event_type", "user_id", "session_id", "description", "model", "entry_hash"]
      const rows = entries.map((e) =>
        [e.id, e.timestamp, e.event_type, e.user_id, e.session_id ?? "", e.description, e.model ?? "", e.entry_hash].join(","),
      )
      const csv = [headers.join(","), ...rows].join("\n")

      return {
        status: 200,
        headers: {
          "Content-Type": "text/csv",
          "Content-Disposition": `attachment; filename="audit-export-${body.start_date}-${body.end_date}.csv"`,
        },
        body: csv,
      }
    }

    return jsonResponse({
      success: true,
      export: {
        period_start: body.start_date,
        period_end: body.end_date,
        entry_count: entries.length,
        entries: body.include_content_hashes ? entries : entries.map(({ content_hash: _, ...rest }) => rest),
        exported_at: new Date().toISOString(),
      },
    })
  } catch (error) {
    return errorResponse(`Failed to export: ${error}`, 500)
  }
}

// ============================================================================
// Route Registration Helper
// ============================================================================

export const complianceRoutes = {
  "GET /api/v1/compliance/logs": handleListLogs,
  "GET /api/v1/compliance/logs/:id": handleGetLog,
  "POST /api/v1/compliance/report": handleGenerateReport,
  "GET /api/v1/compliance/reports": handleListReports,
  "GET /api/v1/compliance/reports/:id": handleGetReport,
  "GET /api/v1/compliance/status": handleGetStatus,
  "POST /api/v1/compliance/export": handleExport,
}
