/**
 * Native Trace Store Bindings (Fail-Fast Mode)
 *
 * Provides native Rust implementations for trace log storage and analysis.
 * Throws error if native bindings are unavailable - no fallback.
 *
 * @package trace
 */

import { Log } from "@/util/log"
import os from "os"
import path from "path"

const log = Log.create({ service: "trace.native" })

// ============================================================================
// Type Definitions (mirrors NAPI types from binding.d.ts)
// ============================================================================

export interface NapiTraceEntry {
  ts: string
  traceId: string
  spanId: string
  parentSpanId?: string
  service: string
  eventType: string
  level: string
  payload: string // JSON string
}

export interface NapiTraceFilter {
  traceId?: string
  service?: string
  eventType?: string
  level?: string
  fromTs?: string
  toTs?: string
  limit?: number
  offset?: number
}

export interface NapiTraceStoreStats {
  totalEntries: number
  totalSizeBytes: number
  oldestTs?: string
  newestTs?: string
  byService: Record<string, number>
  byEventType: Record<string, number>
}

export interface NapiSlowOperation {
  function: string
  service: string
  durationMs: number
  traceId: string
  timestamp: string
}

export interface NapiServiceStats {
  service: string
  eventCount: number
  errorCount: number
  avgDurationMs: number
  p50DurationMs: number
  p95DurationMs: number
  p99DurationMs: number
}

export interface NapiFunctionStats {
  function: string
  callCount: number
  avgDurationMs: number
  maxDurationMs: number
  minDurationMs: number
}

export interface NapiProfileResult {
  totalTraces: number
  totalEvents: number
  avgDurationMs: number
  maxDurationMs: number
  minDurationMs: number
  slowest: NapiSlowOperation[]
  byService: NapiServiceStats[]
  byFunction: NapiFunctionStats[]
}

export interface NapiErrorSample {
  error: string
  timestamp: string
  traceId: string
}

export interface NapiErrorGroup {
  key: string
  count: number
  samples: NapiErrorSample[]
}

export interface NapiErrorSummary {
  total: number
  groups: NapiErrorGroup[]
}

export interface TraceStoreHandle {
  append(entry: NapiTraceEntry): void
  appendBatch(entries: NapiTraceEntry[]): number
  queryByTraceId(traceId: string): NapiTraceEntry[]
  queryByService(service: string, fromTs?: string, limit?: number): NapiTraceEntry[]
  query(filter: NapiTraceFilter): NapiTraceEntry[]
  count(filter: NapiTraceFilter): number
  getTraceIds(fromTs: string, limit?: number): string[]
  getServices(): string[]
  profile(fromTs: string, topN: number): NapiProfileResult
  generateReport(fromTs: string, topN: number): string
  generateComparisonReport(period1Start: string, period1End: string, period2Start: string, period2End: string): string
  aggregateErrors(fromTs: string, groupBy: string): NapiErrorSummary
  errorRates(fromTs: string): Record<string, number>
  recentErrors(limit: number): NapiErrorSample[]
  cleanup(retentionDays: number): number
  compact(): void
  stats(): NapiTraceStoreStats
  healthCheck(): boolean
  path(): string
}

// ============================================================================
// Native Bindings Loader (Fail-Fast)
// ============================================================================

interface NativeTraceBindings {
  openTraceStore: (dbPath: string) => TraceStoreHandle
  createMemoryTraceStore: () => TraceStoreHandle
}

let nativeBindings: NativeTraceBindings | null = null
let loadAttempted = false

/**
 * Load native trace bindings. Throws if unavailable.
 * @throws Error if native bindings cannot be loaded
 */
async function loadNativeBindings(): Promise<NativeTraceBindings> {
  if (loadAttempted && nativeBindings) return nativeBindings

  try {
    const core = await import("@codecoder-ai/core")

    if (typeof core.openTraceStore === "function" && typeof core.createMemoryTraceStore === "function") {
      // Cast through unknown to work around outdated binding.d.ts types
      nativeBindings = {
        openTraceStore: core.openTraceStore as unknown as NativeTraceBindings["openTraceStore"],
        createMemoryTraceStore: core.createMemoryTraceStore as unknown as NativeTraceBindings["createMemoryTraceStore"],
      }
      log.debug("Loaded native trace bindings")
      loadAttempted = true
      return nativeBindings
    }
  } catch (e) {
    loadAttempted = true
    throw new Error(`Native bindings required: @codecoder-ai/core trace functions not available: ${e}`)
  }

  loadAttempted = true
  throw new Error("Native bindings required: @codecoder-ai/core trace functions not available")
}

// ============================================================================
// Public API (Fail-Fast)
// ============================================================================

export async function isNativeAvailable(): Promise<boolean> {
  try {
    await loadNativeBindings()
    return true
  } catch {
    return false
  }
}

export function isUsingNative(): boolean {
  return nativeBindings !== null
}

/**
 * Open a trace store at the given path.
 * @throws Error if native bindings unavailable
 */
export async function openTraceStore(dbPath: string): Promise<TraceStoreHandle> {
  const bindings = await loadNativeBindings()
  return bindings.openTraceStore(dbPath)
}

/**
 * Create an in-memory trace store.
 * @throws Error if native bindings unavailable
 */
export async function createMemoryTraceStore(): Promise<TraceStoreHandle> {
  const bindings = await loadNativeBindings()
  return bindings.createMemoryTraceStore()
}

// ============================================================================
// Utility Functions
// ============================================================================

export function toNapiTraceEntry(entry: {
  ts: string
  trace_id: string
  span_id: string
  parent_span_id?: string
  service: string
  event_type: string
  level: string
  payload: Record<string, unknown>
}): NapiTraceEntry {
  return {
    ts: entry.ts,
    traceId: entry.trace_id,
    spanId: entry.span_id,
    parentSpanId: entry.parent_span_id,
    service: entry.service,
    eventType: entry.event_type,
    level: entry.level,
    payload: JSON.stringify(entry.payload),
  }
}

export function fromNapiTraceEntry(entry: NapiTraceEntry): {
  ts: string
  trace_id: string
  span_id: string
  parent_span_id?: string
  service: string
  event_type: string
  level: string
  payload: Record<string, unknown>
} {
  return {
    ts: entry.ts,
    trace_id: entry.traceId,
    span_id: entry.spanId,
    parent_span_id: entry.parentSpanId,
    service: entry.service,
    event_type: entry.eventType,
    level: entry.level,
    payload: JSON.parse(entry.payload) as Record<string, unknown>,
  }
}

export function getDefaultDbPath(): string {
  return path.join(os.homedir(), ".codecoder", "traces.db")
}

// ============================================================================
// Singleton Store Instance
// ============================================================================

let globalStore: TraceStoreHandle | null = null

/**
 * Get or create the global trace store.
 * @throws Error if native bindings unavailable
 */
export async function getGlobalTraceStore(dbPath?: string): Promise<TraceStoreHandle> {
  if (globalStore) return globalStore

  globalStore = await openTraceStore(dbPath ?? getDefaultDbPath())
  return globalStore
}

export function resetGlobalStore(): void {
  globalStore = null
}
