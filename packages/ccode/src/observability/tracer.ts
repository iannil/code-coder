/**
 * Lightweight Observability Tracer
 *
 * Provides zero-intrusion execution tracking inspired by Agent Lightning.
 * Uses the native Rust observability store for high-performance storage.
 *
 * @example
 * ```typescript
 * import { Tracer, getGlobalTracer } from './observability/tracer'
 *
 * // Get or create the global tracer
 * const tracer = getGlobalTracer()
 *
 * // Emit an LLM call event
 * tracer.emitLlmCall({
 *   provider: 'anthropic',
 *   model: 'claude-opus-4-5',
 *   inputTokens: 1500,
 *   outputTokens: 500,
 *   latencyMs: 2500,
 *   costUsd: 0.03,
 *   success: true
 * })
 *
 * // Emit a tool execution event
 * tracer.emitToolExecution({
 *   toolName: 'Read',
 *   durationMs: 50,
 *   status: 'success'
 * })
 *
 * // Get metrics
 * const metrics = await tracer.getMetrics({ hours: 24 })
 * console.log(`Total cost: $${metrics.llm.totalCostUsd.toFixed(4)}`)
 * ```
 */

import * as native from '@codecoder-ai/core'
import * as path from 'node:path'
import * as os from 'node:os'

// ============================================================================
// Types
// ============================================================================

export type EventType = 'llm_call' | 'tool_execution' | 'agent_lifecycle' | 'span'
export type ToolStatus = 'success' | 'error' | 'cancelled' | 'timeout' | 'blocked'
export type AgentLifecycleType = 'start' | 'complete' | 'error' | 'fork' | 'resume' | 'pause' | 'cancel'
export type SpanKind = 'internal' | 'client' | 'server' | 'producer' | 'consumer'

// ============================================================================
// Native Store Stub Types (NAPI not yet implemented)
// ============================================================================

/**
 * Stub interface for the native observability store.
 * The actual NAPI bindings are not yet implemented in Rust.
 * This interface documents the expected API for when they are added.
 */
interface NativeObservabilityStore {
  emitLlmCall(event: {
    traceId?: string
    parentSpanId?: string
    sessionId?: string
    agentId?: string
    provider: string
    model: string
    inputTokens: number
    outputTokens: number
    cacheReadTokens?: number
    cacheWriteTokens?: number
    latencyMs: number
    costUsd: number
    success: boolean
    error?: string
    stopReason?: string
  }): void

  emitToolExecution(event: {
    traceId?: string
    parentSpanId?: string
    sessionId?: string
    agentId?: string
    toolName: string
    toolCallId?: string
    durationMs: number
    status: NapiToolStatus
    error?: string
    inputSizeBytes?: number
    outputSizeBytes?: number
  }): void

  emitAgentLifecycle(event: {
    traceId?: string
    parentSpanId?: string
    sessionId?: string
    agentId: string
    agentType: string
    lifecycleType: NapiAgentLifecycleType
    parentAgentId?: string
    durationMs?: number
    error?: string
    turnCount?: number
  }): void

  emitSpan(event: {
    traceId?: string
    parentSpanId?: string
    sessionId?: string
    agentId?: string
    name: string
    kind?: NapiSpanKind
    durationMs: number
    success: boolean
    error?: string
  }): void

  totalCost(from: string, to: string): number
  totalTokens(from: string, to: string): [number, number]
  aggregateMetrics(from: string, to: string): NativeMetricsSummary
  stats(): NativeStoreStats
  cleanup(): number
  compact(): void
  healthCheck(): boolean
}

interface NativeMetricsSummary {
  fromTs: string
  toTs: string
  totalEvents: number
  llm: NativeLlmMetrics
  tools: NativeToolMetrics
  agents: NativeAgentMetrics
}

interface NativeLlmMetrics {
  totalCalls: number
  successfulCalls: number
  failedCalls: number
  totalInputTokens: number
  totalOutputTokens: number
  totalCacheReadTokens: number
  totalCacheWriteTokens: number
  totalLatencyMs: number
  avgLatencyMs: number
  p50LatencyMs: number
  p95LatencyMs: number
  p99LatencyMs: number
  totalCostUsd: number
  avgCostPerCallUsd: number
  cacheHitRate: number
  successRate: number
}

interface NativeToolMetrics {
  totalExecutions: number
  successfulExecutions: number
  failedExecutions: number
  blockedExecutions: number
  timeoutExecutions: number
  cancelledExecutions: number
  totalDurationMs: number
  avgDurationMs: number
  p50DurationMs: number
  p95DurationMs: number
  totalInputBytes: number
  totalOutputBytes: number
  successRate: number
}

interface NativeAgentMetrics {
  totalStarts: number
  totalCompletions: number
  totalErrors: number
  totalForks: number
  avgTurns: number
  avgDurationMs: number
  completionRate: number
}

interface NativeStoreStats {
  totalEvents: number
  llmCalls: number
  toolExecutions: number
  agentEvents: number
  totalCostUsd: number
  totalInputTokens: number
  totalOutputTokens: number
  oldestTs?: string
  newestTs?: string
}

export interface LlmCallEvent {
  /** Trace ID for correlation (auto-generated if not provided) */
  traceId?: string
  /** Parent span ID (if nested) */
  parentSpanId?: string
  /** Session ID */
  sessionId?: string
  /** Agent ID that made the call */
  agentId?: string
  /** LLM provider */
  provider: string
  /** Model ID */
  model: string
  /** Input tokens */
  inputTokens: number
  /** Output tokens */
  outputTokens: number
  /** Cache read tokens */
  cacheReadTokens?: number
  /** Cache write tokens */
  cacheWriteTokens?: number
  /** Latency in milliseconds */
  latencyMs: number
  /** Cost in USD */
  costUsd: number
  /** Whether the call succeeded */
  success: boolean
  /** Error message if failed */
  error?: string
  /** Stop reason */
  stopReason?: string
}

export interface ToolExecutionEvent {
  /** Trace ID for correlation */
  traceId?: string
  /** Parent span ID */
  parentSpanId?: string
  /** Session ID */
  sessionId?: string
  /** Agent ID */
  agentId?: string
  /** Tool name */
  toolName: string
  /** Tool call ID from LLM */
  toolCallId?: string
  /** Duration in milliseconds */
  durationMs: number
  /** Tool status */
  status: ToolStatus
  /** Error message if failed */
  error?: string
  /** Input size in bytes */
  inputSizeBytes?: number
  /** Output size in bytes */
  outputSizeBytes?: number
}

export interface AgentLifecycleEvent {
  /** Trace ID for correlation */
  traceId?: string
  /** Parent span ID */
  parentSpanId?: string
  /** Session ID */
  sessionId?: string
  /** Agent ID */
  agentId: string
  /** Agent type (build, plan, etc.) */
  agentType: string
  /** Lifecycle type */
  lifecycleType: AgentLifecycleType
  /** Parent agent ID (for Fork events) */
  parentAgentId?: string
  /** Duration in milliseconds (for Complete/Error events) */
  durationMs?: number
  /** Error message (for Error events) */
  error?: string
  /** Turn count (for Complete events) */
  turnCount?: number
}

export interface SpanEvent {
  /** Trace ID for correlation */
  traceId?: string
  /** Parent span ID */
  parentSpanId?: string
  /** Session ID */
  sessionId?: string
  /** Agent ID */
  agentId?: string
  /** Span name/operation */
  name: string
  /** Span kind */
  kind?: SpanKind
  /** Duration in milliseconds */
  durationMs: number
  /** Whether the span succeeded */
  success: boolean
  /** Error message if failed */
  error?: string
}

export interface LlmMetrics {
  totalCalls: number
  successfulCalls: number
  failedCalls: number
  totalInputTokens: number
  totalOutputTokens: number
  totalCacheReadTokens: number
  totalCacheWriteTokens: number
  totalLatencyMs: number
  avgLatencyMs: number
  p50LatencyMs: number
  p95LatencyMs: number
  p99LatencyMs: number
  totalCostUsd: number
  avgCostPerCallUsd: number
  cacheHitRate: number
  successRate: number
}

export interface ToolMetrics {
  totalExecutions: number
  successfulExecutions: number
  failedExecutions: number
  blockedExecutions: number
  timeoutExecutions: number
  cancelledExecutions: number
  totalDurationMs: number
  avgDurationMs: number
  p50DurationMs: number
  p95DurationMs: number
  totalInputBytes: number
  totalOutputBytes: number
  successRate: number
}

export interface AgentMetrics {
  totalStarts: number
  totalCompletions: number
  totalErrors: number
  totalForks: number
  avgTurns: number
  avgDurationMs: number
  completionRate: number
}

export interface MetricsSummary {
  fromTs: string
  toTs: string
  totalEvents: number
  llm: LlmMetrics
  tools: ToolMetrics
  agents: AgentMetrics
}

export interface StoreStats {
  totalEvents: number
  llmCalls: number
  toolExecutions: number
  agentEvents: number
  totalCostUsd: number
  totalInputTokens: number
  totalOutputTokens: number
  oldestTs?: string
  newestTs?: string
}

export interface MetricsOptions {
  /** Hours to look back (default: 24) */
  hours?: number
  /** From timestamp (ISO 8601) */
  from?: string
  /** To timestamp (ISO 8601) */
  to?: string
}

// ============================================================================
// Tracer Implementation
// ============================================================================

/**
 * Observability Tracer
 *
 * Provides a thin TypeScript wrapper around the native Rust observability store.
 * All storage and aggregation happens in Rust for maximum performance.
 *
 * NOTE: The native NAPI bindings are not yet implemented. All Tracer methods
 * will throw "Native observability bindings not available" until they are.
 */
export class Tracer {
  private store: NativeObservabilityStore | null = null
  private readonly dbPath: string
  private currentTraceId: string | null = null
  private currentSessionId: string | null = null
  private currentAgentId: string | null = null

  constructor(dbPath?: string) {
    this.dbPath = dbPath ?? path.join(os.homedir(), '.codecoder', 'observability.db')
  }

  /**
   * Initialize the tracer (lazy initialization)
   */
  private ensureStore(): NativeObservabilityStore {
    if (this.store === null) {
      // NOTE: native.openObservabilityStore is undefined until NAPI bindings are implemented
      const openStore = native.openObservabilityStore as ((path: string) => NativeObservabilityStore) | undefined
      if (!openStore) {
        throw new Error('Native observability bindings not available')
      }
      this.store = openStore(this.dbPath)
    }
    return this.store
  }

  /**
   * Set the current trace context
   */
  setContext(options: { traceId?: string; sessionId?: string; agentId?: string }): void {
    if (options.traceId !== undefined) this.currentTraceId = options.traceId
    if (options.sessionId !== undefined) this.currentSessionId = options.sessionId
    if (options.agentId !== undefined) this.currentAgentId = options.agentId
  }

  /**
   * Clear the current trace context
   */
  clearContext(): void {
    this.currentTraceId = null
    this.currentSessionId = null
    this.currentAgentId = null
  }

  /**
   * Get the current trace ID
   */
  getTraceId(): string | null {
    return this.currentTraceId
  }

  /**
   * Emit an LLM call event
   */
  emitLlmCall(event: LlmCallEvent): void {
    const store = this.ensureStore()
    store.emitLlmCall({
      traceId: event.traceId ?? this.currentTraceId ?? undefined,
      parentSpanId: event.parentSpanId,
      sessionId: event.sessionId ?? this.currentSessionId ?? undefined,
      agentId: event.agentId ?? this.currentAgentId ?? undefined,
      provider: event.provider,
      model: event.model,
      inputTokens: event.inputTokens,
      outputTokens: event.outputTokens,
      cacheReadTokens: event.cacheReadTokens,
      cacheWriteTokens: event.cacheWriteTokens,
      latencyMs: event.latencyMs,
      costUsd: event.costUsd,
      success: event.success,
      error: event.error,
      stopReason: event.stopReason,
    })
  }

  /**
   * Emit a tool execution event
   */
  emitToolExecution(event: ToolExecutionEvent): void {
    const store = this.ensureStore()
    store.emitToolExecution({
      traceId: event.traceId ?? this.currentTraceId ?? undefined,
      parentSpanId: event.parentSpanId,
      sessionId: event.sessionId ?? this.currentSessionId ?? undefined,
      agentId: event.agentId ?? this.currentAgentId ?? undefined,
      toolName: event.toolName,
      toolCallId: event.toolCallId,
      durationMs: event.durationMs,
      status: mapToolStatus(event.status),
      error: event.error,
      inputSizeBytes: event.inputSizeBytes,
      outputSizeBytes: event.outputSizeBytes,
    })
  }

  /**
   * Emit an agent lifecycle event
   */
  emitAgentLifecycle(event: AgentLifecycleEvent): void {
    const store = this.ensureStore()
    store.emitAgentLifecycle({
      traceId: event.traceId ?? this.currentTraceId ?? undefined,
      parentSpanId: event.parentSpanId,
      sessionId: event.sessionId ?? this.currentSessionId ?? undefined,
      agentId: event.agentId,
      agentType: event.agentType,
      lifecycleType: mapAgentLifecycleType(event.lifecycleType),
      parentAgentId: event.parentAgentId,
      durationMs: event.durationMs,
      error: event.error,
      turnCount: event.turnCount,
    })
  }

  /**
   * Emit a span event
   */
  emitSpan(event: SpanEvent): void {
    const store = this.ensureStore()
    store.emitSpan({
      traceId: event.traceId ?? this.currentTraceId ?? undefined,
      parentSpanId: event.parentSpanId,
      sessionId: event.sessionId ?? this.currentSessionId ?? undefined,
      agentId: event.agentId ?? this.currentAgentId ?? undefined,
      name: event.name,
      kind: event.kind ? mapSpanKind(event.kind) : undefined,
      durationMs: event.durationMs,
      success: event.success,
      error: event.error,
    })
  }

  /**
   * Get total cost for a time period
   */
  getTotalCost(options?: MetricsOptions): number {
    const store = this.ensureStore()
    const { from, to } = resolveTimeRange(options)
    return store.totalCost(from, to)
  }

  /**
   * Get total tokens for a time period
   */
  getTotalTokens(options?: MetricsOptions): { input: number; output: number } {
    const store = this.ensureStore()
    const { from, to } = resolveTimeRange(options)
    const [input, output] = store.totalTokens(from, to)
    return { input, output }
  }

  /**
   * Get aggregated metrics for a time period
   */
  getMetrics(options?: MetricsOptions): MetricsSummary {
    const store = this.ensureStore()
    const { from, to } = resolveTimeRange(options)
    const metrics = store.aggregateMetrics(from, to)
    return {
      fromTs: metrics.fromTs,
      toTs: metrics.toTs,
      totalEvents: metrics.totalEvents,
      llm: {
        totalCalls: metrics.llm.totalCalls,
        successfulCalls: metrics.llm.successfulCalls,
        failedCalls: metrics.llm.failedCalls,
        totalInputTokens: metrics.llm.totalInputTokens,
        totalOutputTokens: metrics.llm.totalOutputTokens,
        totalCacheReadTokens: metrics.llm.totalCacheReadTokens,
        totalCacheWriteTokens: metrics.llm.totalCacheWriteTokens,
        totalLatencyMs: metrics.llm.totalLatencyMs,
        avgLatencyMs: metrics.llm.avgLatencyMs,
        p50LatencyMs: metrics.llm.p50LatencyMs,
        p95LatencyMs: metrics.llm.p95LatencyMs,
        p99LatencyMs: metrics.llm.p99LatencyMs,
        totalCostUsd: metrics.llm.totalCostUsd,
        avgCostPerCallUsd: metrics.llm.avgCostPerCallUsd,
        cacheHitRate: metrics.llm.cacheHitRate,
        successRate: metrics.llm.successRate,
      },
      tools: {
        totalExecutions: metrics.tools.totalExecutions,
        successfulExecutions: metrics.tools.successfulExecutions,
        failedExecutions: metrics.tools.failedExecutions,
        blockedExecutions: metrics.tools.blockedExecutions,
        timeoutExecutions: metrics.tools.timeoutExecutions,
        cancelledExecutions: metrics.tools.cancelledExecutions,
        totalDurationMs: metrics.tools.totalDurationMs,
        avgDurationMs: metrics.tools.avgDurationMs,
        p50DurationMs: metrics.tools.p50DurationMs,
        p95DurationMs: metrics.tools.p95DurationMs,
        totalInputBytes: metrics.tools.totalInputBytes,
        totalOutputBytes: metrics.tools.totalOutputBytes,
        successRate: metrics.tools.successRate,
      },
      agents: {
        totalStarts: metrics.agents.totalStarts,
        totalCompletions: metrics.agents.totalCompletions,
        totalErrors: metrics.agents.totalErrors,
        totalForks: metrics.agents.totalForks,
        avgTurns: metrics.agents.avgTurns,
        avgDurationMs: metrics.agents.avgDurationMs,
        completionRate: metrics.agents.completionRate,
      },
    }
  }

  /**
   * Get store statistics
   */
  getStats(): StoreStats {
    const store = this.ensureStore()
    const stats = store.stats()
    return {
      totalEvents: stats.totalEvents,
      llmCalls: stats.llmCalls,
      toolExecutions: stats.toolExecutions,
      agentEvents: stats.agentEvents,
      totalCostUsd: stats.totalCostUsd,
      totalInputTokens: stats.totalInputTokens,
      totalOutputTokens: stats.totalOutputTokens,
      oldestTs: stats.oldestTs,
      newestTs: stats.newestTs,
    }
  }

  /**
   * Clean up old events (based on retention period)
   */
  cleanup(): number {
    const store = this.ensureStore()
    return store.cleanup()
  }

  /**
   * Compact the database
   */
  compact(): void {
    const store = this.ensureStore()
    store.compact()
  }

  /**
   * Health check
   */
  healthCheck(): boolean {
    try {
      const store = this.ensureStore()
      return store.healthCheck()
    } catch {
      return false
    }
  }
}

// ============================================================================
// Helper Functions
// ============================================================================

// NAPI type stubs - These types are expected by the native Rust store but the
// observability NAPI bindings are not yet implemented. We define them locally
// so TypeScript compiles without errors. The values are cast as the store
// methods expect PascalCase enum variants.
type NapiToolStatus = 'Success' | 'Error' | 'Cancelled' | 'Timeout' | 'Blocked'
type NapiAgentLifecycleType = 'Start' | 'Complete' | 'Error' | 'Fork' | 'Resume' | 'Pause' | 'Cancel'
type NapiSpanKind = 'Internal' | 'Client' | 'Server' | 'Producer' | 'Consumer'

function mapToolStatus(status: ToolStatus): NapiToolStatus {
  const map: Record<ToolStatus, NapiToolStatus> = {
    success: 'Success',
    error: 'Error',
    cancelled: 'Cancelled',
    timeout: 'Timeout',
    blocked: 'Blocked',
  }
  return map[status]
}

function mapAgentLifecycleType(type: AgentLifecycleType): NapiAgentLifecycleType {
  const map: Record<AgentLifecycleType, NapiAgentLifecycleType> = {
    start: 'Start',
    complete: 'Complete',
    error: 'Error',
    fork: 'Fork',
    resume: 'Resume',
    pause: 'Pause',
    cancel: 'Cancel',
  }
  return map[type]
}

function mapSpanKind(kind: SpanKind): NapiSpanKind {
  const map: Record<SpanKind, NapiSpanKind> = {
    internal: 'Internal',
    client: 'Client',
    server: 'Server',
    producer: 'Producer',
    consumer: 'Consumer',
  }
  return map[kind]
}

function resolveTimeRange(options?: MetricsOptions): { from: string; to: string } {
  const now = new Date()
  const to = options?.to ?? now.toISOString()

  let from: string
  if (options?.from) {
    from = options.from
  } else {
    const hours = options?.hours ?? 24
    const fromDate = new Date(now.getTime() - hours * 60 * 60 * 1000)
    from = fromDate.toISOString()
  }

  return { from, to }
}

// ============================================================================
// Global Tracer
// ============================================================================

let globalTracer: Tracer | null = null

/**
 * Get or create the global tracer instance
 */
export function getGlobalTracer(): Tracer {
  if (globalTracer === null) {
    globalTracer = new Tracer()
  }
  return globalTracer
}

/**
 * Create a new tracer with a custom database path
 */
export function createTracer(dbPath: string): Tracer {
  return new Tracer(dbPath)
}

// Default export
export default Tracer
