/**
 * Observability Module
 *
 * Provides lightweight, zero-intrusion execution tracking for CodeCoder.
 *
 * This module exports three layers:
 * 1. **Types** (types.ts) - Zod schemas for log entries and configuration
 * 2. **Trace Context** (trace-context.ts) - AsyncLocalStorage-based context propagation
 * 3. **Structured Logging** (structured-log.ts) - JSONL output compatible with zero-* services
 * 4. **Tracer** (tracer.ts) - High-level tracer wrapper (requires NAPI bindings)
 *
 * @example
 * ```typescript
 * import { getGlobalTracer, createTracer } from './observability'
 *
 * // Use the global tracer
 * const tracer = getGlobalTracer()
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
 * // Get metrics
 * const metrics = tracer.getMetrics({ hours: 24 })
 * console.log(`Total cost: $${metrics.llm.totalCostUsd.toFixed(4)}`)
 * ```
 */

// ============================================================================
// Types (types.ts)
// ============================================================================

export {
  EventType,
  LogLevel,
  LogEntry,
  LegacyLogEntry,
  TraceHeaders,
  ObservabilityConfig,
  type TraceContext,
  type TrackerOptions,
} from './types.js'

// ============================================================================
// Trace Context (trace-context.ts)
// ============================================================================

export {
  getContext,
  getTraceId,
  getSpanId,
  getEntries,
  addEntry,
  createContext,
  runWithContext,
  runWithNewContext,
  runWithChildSpan,
  runWithContextAsync,
  runWithNewContextAsync,
  runWithChildSpanAsync,
  fromHeaders,
  toHeaders,
  runWithHeaderContext,
  runWithHeaderContextAsync,
} from './trace-context.js'

// ============================================================================
// Structured Logging (structured-log.ts)
// ============================================================================

export {
  isEnabled,
  configure,
  getConfig,
  init,
  getLogPath,
  log,
  functionStart,
  functionEnd,
  functionError,
  httpRequest,
  httpResponse,
  // Note: apiCall from structured-log.ts is not exported here because
  // point.ts exports a more feature-rich apiCall that returns ApiCallHandle.
  // The structured-log.ts apiCall is a simple one-shot logger.
  apiCall as logApiCall,
  type StructuredLogOptions,
} from './structured-log.js'

// Convenience aliases for backwards compatibility
export { configure as configureObservability } from './structured-log.js'
export { init as initObservability } from './structured-log.js'

// ============================================================================
// Point Functions (point.ts) - High-level observability helpers
// ============================================================================

export {
  point,
  branch,
  loop,
  apiCall,
  type ApiCallHandle,
} from './point.js'

// Legacy alias for backwards compatibility
// Some files import apiCallPoint instead of apiCall
export { apiCall as apiCallPoint } from './point.js'

// ============================================================================
// Tracer (tracer.ts) - Requires NAPI bindings
// ============================================================================

export {
  Tracer,
  getGlobalTracer,
  createTracer,
  type EventType as TracerEventType,
  type ToolStatus,
  type AgentLifecycleType,
  type SpanKind,
  type LlmCallEvent,
  type ToolExecutionEvent,
  type AgentLifecycleEvent,
  type SpanEvent,
  type LlmMetrics,
  type ToolMetrics,
  type AgentMetrics,
  type MetricsSummary,
  type StoreStats,
  type MetricsOptions,
} from './tracer.js'
