/**
 * Trace Profiler
 * Provides performance analysis from trace logs
 */

import fs from "fs/promises"
import path from "path"
import type { LogEntry } from "../observability"

// ============================================================================
// Types
// ============================================================================

export interface SlowOperation {
  function: string
  service: string
  durationMs: number
  traceId: string
  timestamp: string
}

export interface ProfileResult {
  totalTraces: number
  totalEvents: number
  avgDurationMs: number
  maxDurationMs: number
  minDurationMs: number
  slowest: SlowOperation[]
  byService: Record<string, ServiceStats>
  byFunction: Record<string, FunctionStats>
}

export interface ServiceStats {
  service: string
  eventCount: number
  errorCount: number
  avgDurationMs: number
  p50DurationMs: number
  p95DurationMs: number
  p99DurationMs: number
}

export interface FunctionStats {
  function: string
  callCount: number
  avgDurationMs: number
  maxDurationMs: number
  minDurationMs: number
}

// ============================================================================
// File Utilities
// ============================================================================

async function getLogFiles(logDir: string): Promise<string[]> {
  const files = await fs.readdir(logDir).catch(() => [])
  return files
    .filter((f) => f.startsWith("trace-") && f.endsWith(".jsonl"))
    .sort()
    .reverse()
    .map((f) => path.join(logDir, f))
}

async function* parseLogFileInRange(
  filePath: string,
  fromDate: Date,
): AsyncGenerator<LogEntry> {
  const content = await fs.readFile(filePath, "utf-8").catch(() => "")
  for (const line of content.split("\n")) {
    if (!line.trim()) continue
    try {
      const entry = JSON.parse(line) as LogEntry
      const entryDate = new Date(entry.ts)
      if (entryDate >= fromDate) {
        yield entry
      }
    } catch {
      // Skip malformed
    }
  }
}

// ============================================================================
// Profiling Functions
// ============================================================================

/**
 * Calculate percentile from sorted array
 */
function percentile(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0
  const idx = Math.ceil((p / 100) * sorted.length) - 1
  return sorted[Math.max(0, Math.min(idx, sorted.length - 1))]
}

/**
 * Profile traces within a time range
 */
export async function profileTraces(
  logDir: string,
  fromDate: Date,
  topN: number = 10,
): Promise<ProfileResult> {
  const files = await getLogFiles(logDir)

  const traceIds = new Set<string>()
  const slowOperations: SlowOperation[] = []
  const serviceData: Record<string, { durations: number[]; errors: number }> = {}
  const functionData: Record<string, { durations: number[] }> = {}

  let totalEvents = 0
  let totalDuration = 0
  let maxDuration = 0
  let minDuration = Infinity

  for (const file of files) {
    // Check if file date is before our range
    const fileMatch = path.basename(file).match(/trace-(\d{4}-\d{2}-\d{2})\.jsonl/)
    if (fileMatch) {
      const fileDate = new Date(fileMatch[1])
      if (fileDate < new Date(fromDate.toISOString().split("T")[0])) {
        continue
      }
    }

    for await (const entry of parseLogFileInRange(file, fromDate)) {
      totalEvents++
      traceIds.add(entry.trace_id)

      // Track service stats
      if (!serviceData[entry.service]) {
        serviceData[entry.service] = { durations: [], errors: 0 }
      }
      if (entry.event_type === "error") {
        serviceData[entry.service].errors++
      }

      // Track function_end events with duration
      if (entry.event_type === "function_end" && entry.payload?.duration_ms !== undefined) {
        const duration = entry.payload.duration_ms as number
        const funcName = entry.payload?.function as string ?? "unknown"

        totalDuration += duration
        maxDuration = Math.max(maxDuration, duration)
        minDuration = Math.min(minDuration, duration)

        serviceData[entry.service].durations.push(duration)

        if (!functionData[funcName]) {
          functionData[funcName] = { durations: [] }
        }
        functionData[funcName].durations.push(duration)

        slowOperations.push({
          function: funcName,
          service: entry.service,
          durationMs: duration,
          traceId: entry.trace_id,
          timestamp: entry.ts,
        })
      }

      // Track http_response events with duration
      if (entry.event_type === "http_response" && entry.payload?.duration_ms !== undefined) {
        const duration = entry.payload.duration_ms as number
        const funcName = `${entry.payload.method} ${entry.payload.path}`

        totalDuration += duration
        maxDuration = Math.max(maxDuration, duration)
        minDuration = Math.min(minDuration, duration)

        serviceData[entry.service].durations.push(duration)

        if (!functionData[funcName]) {
          functionData[funcName] = { durations: [] }
        }
        functionData[funcName].durations.push(duration)

        slowOperations.push({
          function: funcName,
          service: entry.service,
          durationMs: duration,
          traceId: entry.trace_id,
          timestamp: entry.ts,
        })
      }
    }
  }

  // Sort and get top N slowest
  slowOperations.sort((a, b) => b.durationMs - a.durationMs)
  const topSlowest = slowOperations.slice(0, topN)

  // Calculate service stats
  const byService: Record<string, ServiceStats> = {}
  for (const [service, data] of Object.entries(serviceData)) {
    const sorted = data.durations.sort((a, b) => a - b)
    const total = sorted.reduce((a, b) => a + b, 0)

    byService[service] = {
      service,
      eventCount: sorted.length,
      errorCount: data.errors,
      avgDurationMs: sorted.length > 0 ? total / sorted.length : 0,
      p50DurationMs: percentile(sorted, 50),
      p95DurationMs: percentile(sorted, 95),
      p99DurationMs: percentile(sorted, 99),
    }
  }

  // Calculate function stats
  const byFunction: Record<string, FunctionStats> = {}
  for (const [func, data] of Object.entries(functionData)) {
    const sorted = data.durations.sort((a, b) => a - b)
    const total = sorted.reduce((a, b) => a + b, 0)

    byFunction[func] = {
      function: func,
      callCount: sorted.length,
      avgDurationMs: sorted.length > 0 ? total / sorted.length : 0,
      maxDurationMs: sorted.length > 0 ? sorted[sorted.length - 1] : 0,
      minDurationMs: sorted.length > 0 ? sorted[0] : 0,
    }
  }

  const eventCount = slowOperations.length
  return {
    totalTraces: traceIds.size,
    totalEvents,
    avgDurationMs: eventCount > 0 ? totalDuration / eventCount : 0,
    maxDurationMs: maxDuration === 0 ? 0 : maxDuration,
    minDurationMs: minDuration === Infinity ? 0 : minDuration,
    slowest: topSlowest,
    byService,
    byFunction,
  }
}

/**
 * Generate a detailed profile report
 */
export async function generateDetailedReport(
  logDir: string,
  fromDate: Date,
): Promise<string> {
  const profile = await profileTraces(logDir, fromDate, 20)
  const lines: string[] = []

  lines.push("=" .repeat(80))
  lines.push("                         PERFORMANCE PROFILE REPORT")
  lines.push("=".repeat(80))
  lines.push("")
  lines.push(`Time Range: ${fromDate.toISOString()} - ${new Date().toISOString()}`)
  lines.push("")

  // Summary
  lines.push("SUMMARY")
  lines.push("-".repeat(40))
  lines.push(`Total Traces:    ${profile.totalTraces}`)
  lines.push(`Total Events:    ${profile.totalEvents}`)
  lines.push(`Avg Duration:    ${profile.avgDurationMs.toFixed(2)}ms`)
  lines.push(`Min Duration:    ${profile.minDurationMs.toFixed(2)}ms`)
  lines.push(`Max Duration:    ${profile.maxDurationMs.toFixed(2)}ms`)
  lines.push("")

  // Service breakdown
  lines.push("BY SERVICE")
  lines.push("-".repeat(80))
  lines.push(
    "Service".padEnd(20) +
      "Events".padStart(10) +
      "Errors".padStart(10) +
      "Avg(ms)".padStart(12) +
      "P50(ms)".padStart(12) +
      "P95(ms)".padStart(12),
  )
  lines.push("-".repeat(80))

  const sortedServices = Object.values(profile.byService).sort(
    (a, b) => b.avgDurationMs - a.avgDurationMs,
  )

  for (const stats of sortedServices) {
    lines.push(
      stats.service.padEnd(20) +
        stats.eventCount.toString().padStart(10) +
        stats.errorCount.toString().padStart(10) +
        stats.avgDurationMs.toFixed(2).padStart(12) +
        stats.p50DurationMs.toFixed(2).padStart(12) +
        stats.p95DurationMs.toFixed(2).padStart(12),
    )
  }
  lines.push("")

  // Top slowest operations
  lines.push("TOP 20 SLOWEST OPERATIONS")
  lines.push("-".repeat(80))

  for (let i = 0; i < profile.slowest.length; i++) {
    const op = profile.slowest[i]
    lines.push(`${(i + 1).toString().padStart(2)}. ${op.function}`)
    lines.push(`    Service: ${op.service}`)
    lines.push(`    Duration: ${op.durationMs}ms`)
    lines.push(`    Trace ID: ${op.traceId}`)
    lines.push(`    Time: ${op.timestamp}`)
    lines.push("")
  }

  lines.push("=".repeat(80))

  return lines.join("\n")
}

/**
 * Compare two time periods
 *
 * Analyzes performance metrics from two time periods and generates
 * a comparison report highlighting regressions and improvements.
 */
export async function comparePeriods(
  logDir: string,
  period1Start: Date,
  period1End: Date,
  period2Start: Date,
  period2End: Date,
): Promise<string> {
  // Profile both periods
  const profile1 = await profileTraces(logDir, period1Start, 10)
  const profile2 = await profileTraces(logDir, period2Start, 10)

  const lines: string[] = []

  lines.push("=".repeat(80))
  lines.push("                      PERFORMANCE COMPARISON REPORT")
  lines.push("=".repeat(80))
  lines.push("")

  // Period labels
  const formatDate = (d: Date) => d.toISOString().split("T")[0]
  const period1Label = `Period 1: ${formatDate(period1Start)} - ${formatDate(period1End)}`
  const period2Label = `Period 2: ${formatDate(period2Start)} - ${formatDate(period2End)}`

  lines.push(period1Label)
  lines.push(period2Label)
  lines.push("")

  // Summary comparison
  lines.push("OVERALL SUMMARY")
  lines.push("-".repeat(60))
  lines.push(
    "Metric".padEnd(25) +
      "Period 1".padStart(15) +
      "Period 2".padStart(15) +
      "Change".padStart(15),
  )
  lines.push("-".repeat(60))

  const formatChange = (p1: number, p2: number): string => {
    if (p1 === 0 && p2 === 0) return "N/A"
    if (p1 === 0) return "+∞"
    const pct = ((p2 - p1) / p1) * 100
    const sign = pct >= 0 ? "+" : ""
    const indicator = pct > 10 ? " 🔴" : pct < -10 ? " 🟢" : ""
    return `${sign}${pct.toFixed(1)}%${indicator}`
  }

  lines.push(
    "Total Traces".padEnd(25) +
      profile1.totalTraces.toString().padStart(15) +
      profile2.totalTraces.toString().padStart(15) +
      formatChange(profile1.totalTraces, profile2.totalTraces).padStart(15),
  )

  lines.push(
    "Total Events".padEnd(25) +
      profile1.totalEvents.toString().padStart(15) +
      profile2.totalEvents.toString().padStart(15) +
      formatChange(profile1.totalEvents, profile2.totalEvents).padStart(15),
  )

  lines.push(
    "Avg Duration (ms)".padEnd(25) +
      profile1.avgDurationMs.toFixed(2).padStart(15) +
      profile2.avgDurationMs.toFixed(2).padStart(15) +
      formatChange(profile1.avgDurationMs, profile2.avgDurationMs).padStart(15),
  )

  lines.push(
    "Max Duration (ms)".padEnd(25) +
      profile1.maxDurationMs.toFixed(2).padStart(15) +
      profile2.maxDurationMs.toFixed(2).padStart(15) +
      formatChange(profile1.maxDurationMs, profile2.maxDurationMs).padStart(15),
  )
  lines.push("")

  // Service-level comparison
  lines.push("BY SERVICE COMPARISON")
  lines.push("-".repeat(80))
  lines.push(
    "Service".padEnd(20) +
      "Avg P1 (ms)".padStart(12) +
      "Avg P2 (ms)".padStart(12) +
      "Change".padStart(12) +
      "Errors P1".padStart(12) +
      "Errors P2".padStart(12),
  )
  lines.push("-".repeat(80))

  // Get all services from both periods
  const allServices = new Set([
    ...Object.keys(profile1.byService),
    ...Object.keys(profile2.byService),
  ])

  const serviceChanges: { service: string; changePct: number }[] = []

  for (const service of Array.from(allServices).sort()) {
    const s1 = profile1.byService[service]
    const s2 = profile2.byService[service]

    const avg1 = s1?.avgDurationMs ?? 0
    const avg2 = s2?.avgDurationMs ?? 0
    const err1 = s1?.errorCount ?? 0
    const err2 = s2?.errorCount ?? 0

    const change = formatChange(avg1, avg2)
    const changePct = avg1 > 0 ? ((avg2 - avg1) / avg1) * 100 : 0
    serviceChanges.push({ service, changePct })

    lines.push(
      service.slice(0, 19).padEnd(20) +
        avg1.toFixed(2).padStart(12) +
        avg2.toFixed(2).padStart(12) +
        change.padStart(12) +
        err1.toString().padStart(12) +
        err2.toString().padStart(12),
    )
  }
  lines.push("")

  // Highlight regressions (services with >10% slowdown)
  const regressions = serviceChanges.filter((s) => s.changePct > 10)
  if (regressions.length > 0) {
    lines.push("⚠️  REGRESSIONS DETECTED (>10% slower)")
    lines.push("-".repeat(40))
    for (const r of regressions.sort((a, b) => b.changePct - a.changePct)) {
      lines.push(`  ${r.service}: +${r.changePct.toFixed(1)}%`)
    }
    lines.push("")
  }

  // Highlight improvements (services with >10% speedup)
  const improvements = serviceChanges.filter((s) => s.changePct < -10)
  if (improvements.length > 0) {
    lines.push("✅ IMPROVEMENTS DETECTED (>10% faster)")
    lines.push("-".repeat(40))
    for (const r of improvements.sort((a, b) => a.changePct - b.changePct)) {
      lines.push(`  ${r.service}: ${r.changePct.toFixed(1)}%`)
    }
    lines.push("")
  }

  // Function-level comparison for top functions
  lines.push("TOP FUNCTIONS COMPARISON")
  lines.push("-".repeat(60))

  const allFunctions = new Set([
    ...Object.keys(profile1.byFunction),
    ...Object.keys(profile2.byFunction),
  ])

  const funcComparisons: { func: string; avg1: number; avg2: number; change: number }[] = []

  for (const func of allFunctions) {
    const f1 = profile1.byFunction[func]
    const f2 = profile2.byFunction[func]
    const avg1 = f1?.avgDurationMs ?? 0
    const avg2 = f2?.avgDurationMs ?? 0
    const change = avg1 > 0 ? ((avg2 - avg1) / avg1) * 100 : 0
    funcComparisons.push({ func, avg1, avg2, change })
  }

  // Sort by absolute change and take top 10
  const topChanges = funcComparisons
    .sort((a, b) => Math.abs(b.change) - Math.abs(a.change))
    .slice(0, 10)

  lines.push(
    "Function".padEnd(30) +
      "Avg P1".padStart(10) +
      "Avg P2".padStart(10) +
      "Change".padStart(12),
  )
  lines.push("-".repeat(60))

  for (const f of topChanges) {
    const changeStr = formatChange(f.avg1, f.avg2)
    lines.push(
      f.func.slice(0, 29).padEnd(30) +
        f.avg1.toFixed(2).padStart(10) +
        f.avg2.toFixed(2).padStart(10) +
        changeStr.padStart(12),
    )
  }

  lines.push("")
  lines.push("=".repeat(80))
  lines.push("")
  lines.push("Legend: 🔴 Regression (>10% slower)  🟢 Improvement (>10% faster)")

  return lines.join("\n")
}
