/**
 * Trace Query Engine (Native-Only)
 *
 * Provides functions for querying and filtering trace logs using native Rust implementation.
 * Throws error if native store is unavailable - no fallback.
 *
 * @package trace
 */

import { Log } from "@/util/log"
import type { LogEntry } from "../observability"
import {
  getGlobalTraceStore,
  isNativeAvailable,
  fromNapiTraceEntry,
  type NapiErrorSummary,
  type NapiTraceFilter,
} from "./native"

const log = Log.create({ service: "trace.query" })

// ============================================================================
// Types
// ============================================================================

export interface WatchOptions {
  service?: string
  level?: string
  follow?: boolean
}

export interface ErrorGroup {
  key: string
  count: number
  samples: Array<{
    error: string
    timestamp: string
    traceId: string
  }>
}

export interface ErrorSummary {
  total: number
  groups: ErrorGroup[]
}

// ============================================================================
// Native Result Conversion
// ============================================================================

function convertNapiErrorSummary(napi: NapiErrorSummary): ErrorSummary {
  return {
    total: napi.total,
    groups: napi.groups.map((g) => ({
      key: g.key,
      count: g.count,
      samples: g.samples.map((s) => ({
        error: s.error,
        timestamp: s.timestamp,
        traceId: s.traceId,
      })),
    })),
  }
}

// ============================================================================
// Query Functions
// ============================================================================

/**
 * Query all entries for a specific trace ID.
 * @throws Error if native bindings unavailable
 */
export async function queryTrace(traceId: string): Promise<LogEntry[]> {
  const store = await getGlobalTraceStore()
  const entries = store.queryByTraceId(traceId)
  return entries.map((e) => fromNapiTraceEntry(e) as LogEntry)
}

/**
 * Query traces by service.
 * @throws Error if native bindings unavailable
 */
export async function queryByService(service: string, fromTs?: string, limit?: number): Promise<LogEntry[]> {
  const store = await getGlobalTraceStore()
  const entries = store.queryByService(service, fromTs, limit)
  return entries.map((e) => fromNapiTraceEntry(e) as LogEntry)
}

/**
 * Query traces with flexible filter.
 * @throws Error if native bindings unavailable
 */
export async function queryTraces(filter: {
  traceId?: string
  service?: string
  eventType?: string
  level?: string
  fromTs?: string
  toTs?: string
  limit?: number
  offset?: number
}): Promise<LogEntry[]> {
  const store = await getGlobalTraceStore()

  const napiFilter: NapiTraceFilter = {
    traceId: filter.traceId,
    service: filter.service,
    eventType: filter.eventType,
    level: filter.level,
    fromTs: filter.fromTs,
    toTs: filter.toTs,
    limit: filter.limit,
    offset: filter.offset,
  }
  const entries = store.query(napiFilter)
  return entries.map((e) => fromNapiTraceEntry(e) as LogEntry)
}

/**
 * Get distinct trace IDs within a time range.
 * @throws Error if native bindings unavailable
 */
export async function getTraceIds(fromTs: string, limit?: number): Promise<string[]> {
  const store = await getGlobalTraceStore()
  return store.getTraceIds(fromTs, limit)
}

/**
 * Get all distinct services.
 * @throws Error if native bindings unavailable
 */
export async function getServices(): Promise<string[]> {
  const store = await getGlobalTraceStore()
  return store.getServices()
}

/**
 * Aggregate errors from trace logs.
 * @throws Error if native bindings unavailable
 */
export async function aggregateErrors(
  fromDate: Date,
  groupBy: "service" | "function" | "error" = "service",
): Promise<ErrorSummary> {
  const store = await getGlobalTraceStore()
  const fromTs = fromDate.toISOString()
  const result = store.aggregateErrors(fromTs, groupBy)
  return convertNapiErrorSummary(result)
}

/**
 * Get error rates by service.
 * @throws Error if native bindings unavailable
 */
export async function getErrorRates(fromDate: Date): Promise<Record<string, number>> {
  const store = await getGlobalTraceStore()
  const fromTs = fromDate.toISOString()
  return store.errorRates(fromTs)
}

/**
 * Get recent errors.
 * @throws Error if native bindings unavailable
 */
export async function getRecentErrors(limit: number = 10): Promise<Array<{ error: string; timestamp: string; traceId: string }>> {
  const store = await getGlobalTraceStore()
  const samples = store.recentErrors(limit)
  return samples.map((s) => ({
    error: s.error,
    timestamp: s.timestamp,
    traceId: s.traceId,
  }))
}

/**
 * Count traces matching a filter.
 * @throws Error if native bindings unavailable
 */
export async function countTraces(filter: {
  traceId?: string
  service?: string
  eventType?: string
  level?: string
  fromTs?: string
  toTs?: string
}): Promise<number> {
  const store = await getGlobalTraceStore()

  const napiFilter: NapiTraceFilter = {
    traceId: filter.traceId,
    service: filter.service,
    eventType: filter.eventType,
    level: filter.level,
    fromTs: filter.fromTs,
    toTs: filter.toTs,
  }
  return store.count(napiFilter)
}

/**
 * Check if native query is available
 */
export { isNativeAvailable as isQueryNativeAvailable }

// ============================================================================
// Legacy API Compatibility
// ============================================================================

/**
 * Watch logs is not supported in native-only mode.
 * @deprecated Use native trace store queries instead
 */
export async function watchLogs(_logDir: string, _options: WatchOptions): Promise<void> {
  console.warn("watchLogs is deprecated - use native trace store queries instead")
}
