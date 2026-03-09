/**
 * Lightweight Emit API (Agent Lightning style)
 *
 * Provides a simple, ergonomic API for emitting observability events.
 * This is a thin wrapper around the existing observability system,
 * designed for quick instrumentation with minimal boilerplate.
 *
 * @example
 * ```typescript
 * import { emit } from './observability/emit'
 *
 * // Tool execution tracking
 * const span = emit.toolStart('grep', { pattern: 'TODO' })
 * const result = await executeGrep()
 * emit.toolEnd(span, { matches: 42 }, performance.now() - span.startTime)
 *
 * // State transitions
 * emit.stateTransition('idle', 'executing', 'User initiated')
 *
 * // Agent decisions
 * emit.agentDecision('macro', 'proceed', 0.85)
 * ```
 */

import z from "zod"
import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { BusEvent } from "@/bus/bus-event"

// Lazy logger to avoid circular dependency with Log module
let _log: Log.Logger | null = null
const getLog = () => {
  if (!_log) _log = Log.create({ service: "emit" })
  return _log
}

// ============================================================================
// Bus Event Definition
// ============================================================================

/**
 * Schema for emit events published to the bus
 */
const EmitEventSchema = z.object({
  type: z.string(),
  timestamp: z.number(),
  spanId: z.string().optional(),
  tool: z.string().optional(),
  args: z.record(z.string(), z.unknown()).optional(),
  result: z.record(z.string(), z.unknown()).optional(),
  durationMs: z.number().optional(),
  success: z.boolean().optional(),
  from: z.string().optional(),
  to: z.string().optional(),
  reason: z.string().optional(),
  agent: z.string().optional(),
  decision: z.string().optional(),
  confidence: z.number().optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
  component: z.string().optional(),
  message: z.string().optional(),
  context: z.record(z.string(), z.unknown()).optional(),
  name: z.string().optional(),
  payload: z.record(z.string(), z.unknown()).optional(),
})

/**
 * Bus event definition for emit events
 */
const EmitBusEvent = BusEvent.define("emit.event", EmitEventSchema)

// ============================================================================
// Types
// ============================================================================

export interface SpanId {
  id: string
  tool: string
  startTime: number
  args: Record<string, unknown>
}

export interface EmitEvent {
  type: string
  timestamp: number
  [key: string]: unknown
}

export interface ToolStartEvent extends EmitEvent {
  type: "tool_start"
  spanId: string
  tool: string
  args: Record<string, unknown>
}

export interface ToolEndEvent extends EmitEvent {
  type: "tool_end"
  spanId: string
  tool: string
  result: Record<string, unknown>
  durationMs: number
  success: boolean
}

export interface StateTransitionEvent extends EmitEvent {
  type: "state_transition"
  from: string
  to: string
  reason?: string
}

export interface AgentDecisionEvent extends EmitEvent {
  type: "agent_decision"
  agent: string
  decision: string
  confidence: number
  metadata?: Record<string, unknown>
}

export interface ErrorEvent extends EmitEvent {
  type: "error"
  component: string
  message: string
  context?: Record<string, unknown>
}

export interface CustomEvent extends EmitEvent {
  type: "custom"
  name: string
  payload: Record<string, unknown>
}

export type AnyEmitEvent =
  | ToolStartEvent
  | ToolEndEvent
  | StateTransitionEvent
  | AgentDecisionEvent
  | ErrorEvent
  | CustomEvent

// ============================================================================
// Event Buffer
// ============================================================================

const eventBuffer: AnyEmitEvent[] = []
const MAX_BUFFER_SIZE = 1000
let autoFlush = true

function generateSpanId(): string {
  return `span_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`
}

function now(): number {
  return Date.now()
}

function emitEvent(event: AnyEmitEvent): void {
  eventBuffer.push(event)

  // Log as JSON for downstream processing
  getLog().debug("emit", { event })

  // Publish to event bus for real-time consumers
  Bus.publish(EmitBusEvent, event).catch(() => {
    // Ignore bus errors - observability shouldn't break the app
  })

  if (autoFlush && eventBuffer.length >= MAX_BUFFER_SIZE) {
    flush()
  }
}

// ============================================================================
// Emit API
// ============================================================================

/**
 * Record a tool execution start
 * @returns SpanId for correlation with toolEnd
 */
function toolStart(tool: string, args: Record<string, unknown> = {}): SpanId {
  const spanId = generateSpanId()
  const startTime = now()

  const event: ToolStartEvent = {
    type: "tool_start",
    timestamp: startTime,
    spanId,
    tool,
    args,
  }

  emitEvent(event)

  return {
    id: spanId,
    tool,
    startTime,
    args,
  }
}

/**
 * Record a tool execution end
 */
function toolEnd(
  span: SpanId,
  result: Record<string, unknown> = {},
  durationMs?: number
): void {
  const endTime = now()
  const duration = durationMs ?? endTime - span.startTime

  const event: ToolEndEvent = {
    type: "tool_end",
    timestamp: endTime,
    spanId: span.id,
    tool: span.tool,
    result,
    durationMs: duration,
    success: true,
  }

  emitEvent(event)
}

/**
 * Record a tool execution error
 */
function toolError(
  span: SpanId,
  error: Error | string,
  durationMs?: number
): void {
  const endTime = now()
  const duration = durationMs ?? endTime - span.startTime
  const errorMessage = error instanceof Error ? error.message : error

  const event: ToolEndEvent = {
    type: "tool_end",
    timestamp: endTime,
    spanId: span.id,
    tool: span.tool,
    result: { error: errorMessage },
    durationMs: duration,
    success: false,
  }

  emitEvent(event)
}

/**
 * Record a state machine transition
 */
function stateTransition(from: string, to: string, reason?: string): void {
  const event: StateTransitionEvent = {
    type: "state_transition",
    timestamp: now(),
    from,
    to,
    reason,
  }

  emitEvent(event)
}

/**
 * Record an agent decision
 */
function agentDecision(
  agent: string,
  decision: string,
  confidence: number,
  metadata?: Record<string, unknown>
): void {
  const event: AgentDecisionEvent = {
    type: "agent_decision",
    timestamp: now(),
    agent,
    decision,
    confidence,
    metadata,
  }

  emitEvent(event)
}

/**
 * Record an error
 */
function error(
  component: string,
  message: string,
  context?: Record<string, unknown>
): void {
  const event: ErrorEvent = {
    type: "error",
    timestamp: now(),
    component,
    message,
    context,
  }

  emitEvent(event)
}

/**
 * Record a custom event
 */
function custom(name: string, payload: Record<string, unknown> = {}): void {
  const event: CustomEvent = {
    type: "custom",
    timestamp: now(),
    name,
    payload,
  }

  emitEvent(event)
}

/**
 * Flush all buffered events
 */
function flush(): AnyEmitEvent[] {
  const events = [...eventBuffer]
  eventBuffer.length = 0
  return events
}

/**
 * Get buffered events (for testing/debugging)
 */
function getBuffer(): AnyEmitEvent[] {
  return [...eventBuffer]
}

/**
 * Clear the buffer without returning events
 */
function clear(): void {
  eventBuffer.length = 0
}

/**
 * Configure auto-flush behavior
 */
function setAutoFlush(enabled: boolean): void {
  autoFlush = enabled
}

// ============================================================================
// Convenience Wrapper - Higher-order function for tool execution
// ============================================================================

/**
 * Wrap a function with automatic tool start/end tracking
 */
function withToolTracking<T extends (...args: unknown[]) => Promise<unknown>>(
  tool: string,
  fn: T
): T {
  return (async (...args: unknown[]) => {
    const span = toolStart(tool, { args })
    try {
      const result = await fn(...args)
      toolEnd(span, { result })
      return result
    } catch (err) {
      toolError(span, err instanceof Error ? err : String(err))
      throw err
    }
  }) as T
}

/**
 * Create a scoped emitter for a specific agent/component
 */
function scoped(component: string) {
  return {
    toolStart: (tool: string, args?: Record<string, unknown>) =>
      toolStart(`${component}:${tool}`, args),
    error: (message: string, context?: Record<string, unknown>) =>
      error(component, message, context),
    custom: (name: string, payload?: Record<string, unknown>) =>
      custom(`${component}:${name}`, payload),
  }
}

// ============================================================================
// Export
// ============================================================================

export const emit = {
  // Core API
  toolStart,
  toolEnd,
  toolError,
  stateTransition,
  agentDecision,
  error,
  custom,

  // Buffer management
  flush,
  getBuffer,
  clear,
  setAutoFlush,

  // Utilities
  withToolTracking,
  scoped,
}

// Also export individual functions for tree-shaking
export {
  toolStart,
  toolEnd,
  toolError,
  stateTransition,
  agentDecision,
  error,
  custom,
  flush,
  getBuffer,
  clear,
  setAutoFlush,
  withToolTracking,
  scoped,
}
