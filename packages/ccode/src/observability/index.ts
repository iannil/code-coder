/**
 * Observability Module
 *
 * Provides lightweight, zero-intrusion execution tracking for CodeCoder.
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

export {
  Tracer,
  getGlobalTracer,
  createTracer,
  type EventType,
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
