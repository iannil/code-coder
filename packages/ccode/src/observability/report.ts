import type { LogEntry } from "./types"
import { getContext, getEntries } from "./trace-context"

export interface ExecutionReport {
  traceId: string
  service: string
  startTime: string
  endTime: string
  durationMs: number
  summary: {
    totalEntries: number
    functionCalls: number
    apiCalls: number
    errors: number
    branches: number
    loops: number
  }
  timeline: TimelineEntry[]
  errors: ErrorEntry[]
  apiCalls: ApiCallEntry[]
}

export interface TimelineEntry {
  timestamp: string
  eventType: string
  functionName?: string
  durationMs?: number
  depth: number
}

export interface ErrorEntry {
  timestamp: string
  functionName?: string
  message: string
  stackTrace?: string
}

export interface ApiCallEntry {
  functionName: string
  startTime: string
  endTime?: string
  durationMs?: number
  success?: boolean
}

export function generateReport(entries?: LogEntry[]): ExecutionReport | null {
  const ctx = getContext()
  const logEntries = entries ?? getEntries()

  if (logEntries.length === 0) {
    return null
  }

  const firstEntry = logEntries[0]
  const lastEntry = logEntries[logEntries.length - 1]

  const startTime = new Date(firstEntry.timestamp)
  const endTime = new Date(lastEntry.timestamp)
  const durationMs = endTime.getTime() - startTime.getTime()

  const summary = {
    totalEntries: logEntries.length,
    functionCalls: logEntries.filter((e) => e.event_type === "Function_Start").length,
    apiCalls: logEntries.filter((e) => e.event_type === "API_Call_Start").length,
    errors: logEntries.filter((e) => e.event_type === "Error").length,
    branches: logEntries.filter((e) => e.event_type === "Branch").length,
    loops: logEntries.filter((e) => e.event_type === "Loop").length,
  }

  const spanStack: string[] = []
  const timeline: TimelineEntry[] = []

  for (const entry of logEntries) {
    if (entry.event_type === "Function_Start") {
      spanStack.push(entry.span_id)
    }

    timeline.push({
      timestamp: entry.timestamp,
      eventType: entry.event_type,
      functionName: entry.function_name,
      durationMs: entry.duration_ms,
      depth: spanStack.length,
    })

    if (entry.event_type === "Function_End" || entry.event_type === "Error") {
      spanStack.pop()
    }
  }

  const errors: ErrorEntry[] = logEntries
    .filter((e) => e.event_type === "Error")
    .map((e) => ({
      timestamp: e.timestamp,
      functionName: e.function_name,
      message: String(e.payload.error ?? "Unknown error"),
      stackTrace: e.stack_trace,
    }))

  const apiCallStarts = new Map<string, LogEntry>()
  const apiCalls: ApiCallEntry[] = []

  for (const entry of logEntries) {
    if (entry.event_type === "API_Call_Start" && entry.function_name) {
      apiCallStarts.set(entry.function_name + entry.timestamp, entry)
    }
    if (entry.event_type === "API_Call_End" && entry.function_name) {
      const startKey = Array.from(apiCallStarts.keys()).find((k) => k.startsWith(entry.function_name!))
      const startEntry = startKey ? apiCallStarts.get(startKey) : undefined

      apiCalls.push({
        functionName: entry.function_name,
        startTime: startEntry?.timestamp ?? entry.timestamp,
        endTime: entry.timestamp,
        durationMs: entry.duration_ms,
        success: entry.payload.success as boolean | undefined,
      })

      if (startKey) {
        apiCallStarts.delete(startKey)
      }
    }
  }

  for (const [, entry] of apiCallStarts) {
    apiCalls.push({
      functionName: entry.function_name!,
      startTime: entry.timestamp,
    })
  }

  return {
    traceId: ctx?.traceId ?? firstEntry.trace_id,
    service: ctx?.service ?? firstEntry.service,
    startTime: firstEntry.timestamp,
    endTime: lastEntry.timestamp,
    durationMs,
    summary,
    timeline,
    errors,
    apiCalls,
  }
}

export function formatReportAsText(report: ExecutionReport): string {
  const lines: string[] = []

  lines.push("=".repeat(60))
  lines.push("EXECUTION TRACE REPORT")
  lines.push("=".repeat(60))
  lines.push("")
  lines.push(`Trace ID: ${report.traceId}`)
  lines.push(`Service: ${report.service}`)
  lines.push(`Duration: ${report.durationMs}ms`)
  lines.push(`Start: ${report.startTime}`)
  lines.push(`End: ${report.endTime}`)
  lines.push("")

  lines.push("-".repeat(40))
  lines.push("SUMMARY")
  lines.push("-".repeat(40))
  lines.push(`Total Entries: ${report.summary.totalEntries}`)
  lines.push(`Function Calls: ${report.summary.functionCalls}`)
  lines.push(`API Calls: ${report.summary.apiCalls}`)
  lines.push(`Errors: ${report.summary.errors}`)
  lines.push(`Branches: ${report.summary.branches}`)
  lines.push(`Loops: ${report.summary.loops}`)
  lines.push("")

  if (report.errors.length > 0) {
    lines.push("-".repeat(40))
    lines.push("ERRORS")
    lines.push("-".repeat(40))
    for (const error of report.errors) {
      lines.push(`[${error.timestamp}] ${error.functionName ?? "unknown"}`)
      lines.push(`  ${error.message}`)
      if (error.stackTrace) {
        lines.push(`  Stack: ${error.stackTrace.split("\n")[0]}`)
      }
      lines.push("")
    }
  }

  if (report.apiCalls.length > 0) {
    lines.push("-".repeat(40))
    lines.push("API CALLS")
    lines.push("-".repeat(40))
    for (const call of report.apiCalls) {
      const status = call.success === undefined ? "?" : call.success ? "OK" : "FAIL"
      const duration = call.durationMs !== undefined ? `${call.durationMs}ms` : "pending"
      lines.push(`[${status}] ${call.functionName} (${duration})`)
    }
    lines.push("")
  }

  lines.push("-".repeat(40))
  lines.push("TIMELINE (first 50 entries)")
  lines.push("-".repeat(40))
  for (const entry of report.timeline.slice(0, 50)) {
    const indent = "  ".repeat(entry.depth)
    const duration = entry.durationMs !== undefined ? ` (${entry.durationMs}ms)` : ""
    lines.push(`${indent}${entry.eventType}: ${entry.functionName ?? "-"}${duration}`)
  }

  if (report.timeline.length > 50) {
    lines.push(`  ... and ${report.timeline.length - 50} more entries`)
  }

  lines.push("")
  lines.push("=".repeat(60))

  return lines.join("\n")
}

export function formatReportAsJson(report: ExecutionReport): string {
  return JSON.stringify(report, null, 2)
}
