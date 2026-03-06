/**
 * Trace Profiler (Native-Only)
 *
 * Provides performance analysis from trace logs using native Rust implementation.
 * Throws error if native store is unavailable - no fallback.
 *
 * @package trace
 */

import { Log } from "@/util/log"
import { getGlobalTraceStore, isNativeAvailable, type NapiProfileResult } from "./native"

const log = Log.create({ service: "trace.profiler" })

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
// Native Result Conversion
// ============================================================================

function convertNapiProfileResult(napi: NapiProfileResult): ProfileResult {
  const byService: Record<string, ServiceStats> = {}
  for (const s of napi.byService) {
    byService[s.service] = {
      service: s.service,
      eventCount: s.eventCount,
      errorCount: s.errorCount,
      avgDurationMs: s.avgDurationMs,
      p50DurationMs: s.p50DurationMs,
      p95DurationMs: s.p95DurationMs,
      p99DurationMs: s.p99DurationMs,
    }
  }

  const byFunction: Record<string, FunctionStats> = {}
  for (const f of napi.byFunction) {
    byFunction[f.function] = {
      function: f.function,
      callCount: f.callCount,
      avgDurationMs: f.avgDurationMs,
      maxDurationMs: f.maxDurationMs,
      minDurationMs: f.minDurationMs,
    }
  }

  return {
    totalTraces: napi.totalTraces,
    totalEvents: napi.totalEvents,
    avgDurationMs: napi.avgDurationMs,
    maxDurationMs: napi.maxDurationMs,
    minDurationMs: napi.minDurationMs,
    slowest: napi.slowest.map((op) => ({
      function: op.function,
      service: op.service,
      durationMs: op.durationMs,
      traceId: op.traceId,
      timestamp: op.timestamp,
    })),
    byService,
    byFunction,
  }
}

// ============================================================================
// Profile Functions
// ============================================================================

/**
 * Profile traces within a time range.
 * @throws Error if native bindings unavailable
 */
export async function profileTraces(fromDate: Date, topN: number = 10): Promise<ProfileResult> {
  const store = await getGlobalTraceStore()
  const fromTs = fromDate.toISOString()
  const result = store.profile(fromTs, topN)
  return convertNapiProfileResult(result)
}

/**
 * Generate a detailed profile report.
 * @throws Error if native bindings unavailable
 */
export async function generateDetailedReport(fromDate: Date, topN: number = 20): Promise<string> {
  const profile = await profileTraces(fromDate, topN)

  const lines: string[] = []

  lines.push("=".repeat(80))
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

  const sortedServices = Object.values(profile.byService).sort((a, b) => b.avgDurationMs - a.avgDurationMs)

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
  lines.push(`TOP ${topN} SLOWEST OPERATIONS`)
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
 * Compare two time periods.
 * @throws Error if native bindings unavailable
 */
export async function comparePeriods(
  period1Start: Date,
  period1End: Date,
  period2Start: Date,
  period2End: Date,
): Promise<string> {
  const store = await getGlobalTraceStore()
  return store.generateComparisonReport(
    period1Start.toISOString(),
    period1End.toISOString(),
    period2Start.toISOString(),
    period2End.toISOString(),
  )
}

/**
 * Check if native profiler is available
 */
export { isNativeAvailable as isProfilerNativeAvailable }
