/**
 * Task Event Types for Event Sourcing
 *
 * Defines all event types that flow through the task execution system.
 * These events are stored in Redis Streams and can be replayed for state reconstruction.
 *
 * This file extends the existing types.ts with stream-specific types.
 */

import z from "zod"
import type { TaskEvent as BaseTaskEvent } from "./types"

// ============================================================================
// Extended Task Event Types (for Stream)
// ============================================================================

/** Task created event (entry point). */
export const TaskCreatedEvent = z.object({
  type: z.literal("task_created"),
  data: z.object({
    taskId: z.string(),
    userId: z.string(),
    channel: z.string(),
    channelId: z.string(),
    prompt: z.string(),
    agent: z.string(),
    traceId: z.string(),
    chatHistory: z.array(z.any()).optional(),
  }),
})
export type TaskCreatedEvent = z.infer<typeof TaskCreatedEvent>

/** Task started event. */
export const TaskStartedEvent = z.object({
  type: z.literal("task_started"),
  data: z.object({
    agent: z.string(),
    sessionId: z.string(),
    traceId: z.string(),
  }),
})
export type TaskStartedEvent = z.infer<typeof TaskStartedEvent>

/** Heartbeat event (for timeout management). */
export const HeartbeatEvent = z.object({
  type: z.literal("heartbeat"),
  data: z.object({
    stage: z.string().optional(),
    elapsedMs: z.number(),
  }),
})
export type HeartbeatEvent = z.infer<typeof HeartbeatEvent>

/** Agent switch event. */
export const AgentSwitchEvent = z.object({
  type: z.literal("agent_switch"),
  data: z.object({
    from: z.string(),
    to: z.string(),
    reason: z.string().optional(),
  }),
})
export type AgentSwitchEvent = z.infer<typeof AgentSwitchEvent>

/** Task completed event. */
export const TaskCompletedEvent = z.object({
  type: z.literal("task_completed"),
  data: z.object({
    output: z.string(),
    summary: z.string().optional(),
    usage: z
      .object({
        inputTokens: z.number(),
        outputTokens: z.number(),
        durationMs: z.number(),
      })
      .optional(),
  }),
})
export type TaskCompletedEvent = z.infer<typeof TaskCompletedEvent>

/** Task failed event. */
export const TaskFailedEvent = z.object({
  type: z.literal("task_failed"),
  data: z.object({
    error: z.string(),
    recoverable: z.boolean().default(false),
    code: z.string().optional(),
  }),
})
export type TaskFailedEvent = z.infer<typeof TaskFailedEvent>

// ============================================================================
// Stream Event (All Types)
// ============================================================================

/**
 * Extended task event types for Redis Streams.
 * Includes all base events plus stream-specific events.
 */
export const StreamTaskEvent = z.discriminatedUnion("type", [
  // Lifecycle events
  TaskCreatedEvent,
  TaskStartedEvent,
  TaskCompletedEvent,
  TaskFailedEvent,

  // Execution events (imported from types.ts structure)
  z.object({
    type: z.literal("thought"),
    data: z.string(),
  }),
  z.object({
    type: z.literal("tool_use"),
    data: z.object({
      tool: z.string(),
      args: z.any(),
      result: z.any().optional(),
      durationMs: z.number().optional(),
      isResultTool: z.boolean().optional(),
    }),
  }),
  z.object({
    type: z.literal("progress"),
    data: z.object({
      stage: z.string(),
      message: z.string(),
      percentage: z.number().min(0).max(100).optional(),
    }),
  }),
  z.object({
    type: z.literal("output"),
    data: z.union([
      z.string(),
      z.object({
        content: z.string(),
        isPartial: z.boolean().optional(),
      }),
    ]),
  }),
  z.object({
    type: z.literal("confirmation"),
    data: z.object({
      requestID: z.string(),
      tool: z.string(),
      description: z.string(),
      args: z.any(),
      actions: z.array(z.string()),
    }),
  }),
  z.object({
    type: z.literal("debug_info"),
    data: z.object({
      model: z.string().optional(),
      provider: z.string().optional(),
      input_tokens: z.number().optional(),
      output_tokens: z.number().optional(),
      total_tokens: z.number().optional(),
      duration_ms: z.number().optional(),
      request_bytes: z.number().optional(),
      response_bytes: z.number().optional(),
    }),
  }),
  z.object({
    type: z.literal("agent_info"),
    data: z.object({
      agent: z.string(),
      display_name: z.string().optional(),
      is_primary: z.boolean().optional(),
      duration_ms: z.number().optional(),
    }),
  }),
  z.object({
    type: z.literal("skill_use"),
    data: z.object({
      skill: z.string(),
      args: z.string().optional(),
      duration_ms: z.number().optional(),
    }),
  }),
  z.object({
    type: z.literal("finish"),
    data: z.object({
      success: z.boolean(),
      output: z.string().optional(),
      error: z.string().optional(),
    }),
  }),

  // Stream-specific events
  HeartbeatEvent,
  AgentSwitchEvent,
])
export type StreamTaskEvent = z.infer<typeof StreamTaskEvent>

// ============================================================================
// Stream Event Envelope
// ============================================================================

/**
 * Event envelope for Redis Stream storage.
 * Contains the event plus metadata for stream operations.
 */
export const StreamEventEnvelope = z.object({
  /** Monotonically increasing sequence number (per task). */
  seq: z.number(),
  /** Event timestamp (ISO 8601). */
  timestamp: z.string().datetime(),
  /** The actual event. */
  event: StreamTaskEvent,
  /** Trace ID for distributed tracing. */
  traceId: z.string().optional(),
  /** Span ID. */
  spanId: z.string().optional(),
})
export type StreamEventEnvelope = z.infer<typeof StreamEventEnvelope>

// ============================================================================
// Task State Projection
// ============================================================================

/**
 * Task status values.
 */
export const TaskStatusEnum = z.enum([
  "pending",
  "running",
  "awaiting_approval",
  "completed",
  "failed",
])
export type TaskStatusEnum = z.infer<typeof TaskStatusEnum>

/**
 * Task state projection (materialized from events).
 * This is stored in Redis Hash for quick access.
 */
export const TaskStateProjection = z.object({
  /** Task ID. */
  taskId: z.string(),
  /** Current status. */
  status: TaskStatusEnum,
  /** Current agent. */
  currentAgent: z.string().optional(),
  /** Progress percentage (0-100). */
  progressPct: z.number().min(0).max(100).default(0),
  /** Last event sequence number. */
  lastEventSeq: z.number().default(0),
  /** Partial output buffer. */
  outputBuffer: z.string().optional(),
  /** Task start time (ISO 8601). */
  startedAt: z.string().datetime().optional(),
  /** Last update time (ISO 8601). */
  updatedAt: z.string().datetime(),
  /** Last heartbeat time (ISO 8601). */
  lastHeartbeat: z.string().datetime().optional(),
  /** Error message (if failed). */
  error: z.string().optional(),
})
export type TaskStateProjection = z.infer<typeof TaskStateProjection>

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Create a new stream event envelope.
 */
export function createStreamEvent(
  seq: number,
  event: StreamTaskEvent,
  traceId?: string,
  spanId?: string,
): StreamEventEnvelope {
  return {
    seq,
    timestamp: new Date().toISOString(),
    event,
    traceId,
    spanId,
  }
}

/**
 * Check if an event is a terminal event (task_completed or task_failed).
 */
export function isTerminalEvent(event: StreamTaskEvent): boolean {
  return event.type === "task_completed" || event.type === "task_failed"
}

/**
 * Check if an event is a lifecycle event.
 */
export function isLifecycleEvent(event: StreamTaskEvent): boolean {
  return ["task_created", "task_started", "task_completed", "task_failed"].includes(
    event.type,
  )
}

/**
 * Apply an event to a task state projection.
 * Returns a new state object (immutable).
 */
export function applyEventToState(
  state: TaskStateProjection,
  envelope: StreamEventEnvelope,
): TaskStateProjection {
  const { event, seq, timestamp } = envelope

  const newState: TaskStateProjection = {
    ...state,
    lastEventSeq: seq,
    updatedAt: timestamp,
  }

  switch (event.type) {
    case "task_started":
      return {
        ...newState,
        status: "running",
        currentAgent: event.data.agent,
        startedAt: timestamp,
      }

    case "progress":
      return {
        ...newState,
        progressPct: event.data.percentage ?? newState.progressPct,
      }

    case "output": {
      const content =
        typeof event.data === "string"
          ? event.data
          : event.data.content
      const isPartial =
        typeof event.data === "string"
          ? false
          : event.data.isPartial ?? false

      if (isPartial) {
        return {
          ...newState,
          outputBuffer: (newState.outputBuffer ?? "") + content,
        }
      }
      return {
        ...newState,
        outputBuffer: content,
      }
    }

    case "confirmation":
      return {
        ...newState,
        status: "awaiting_approval",
      }

    case "agent_switch":
      return {
        ...newState,
        currentAgent: event.data.to,
      }

    case "heartbeat":
      return {
        ...newState,
        lastHeartbeat: timestamp,
      }

    case "task_completed":
      return {
        ...newState,
        status: "completed",
        progressPct: 100,
      }

    case "task_failed":
      return {
        ...newState,
        status: "failed",
        error: event.data.error,
      }

    default:
      return newState
  }
}

/**
 * Create initial task state.
 */
export function createInitialState(taskId: string): TaskStateProjection {
  return {
    taskId,
    status: "pending",
    progressPct: 0,
    lastEventSeq: 0,
    updatedAt: new Date().toISOString(),
  }
}

/**
 * Convert task state to Redis hash fields.
 */
export function stateToHashFields(
  state: TaskStateProjection,
): Record<string, string> {
  const fields: Record<string, string> = {
    taskId: state.taskId,
    status: state.status,
    progressPct: state.progressPct.toString(),
    lastEventSeq: state.lastEventSeq.toString(),
    updatedAt: state.updatedAt,
  }

  if (state.currentAgent) fields.currentAgent = state.currentAgent
  if (state.outputBuffer) fields.outputBuffer = state.outputBuffer
  if (state.startedAt) fields.startedAt = state.startedAt
  if (state.lastHeartbeat) fields.lastHeartbeat = state.lastHeartbeat
  if (state.error) fields.error = state.error

  return fields
}

/**
 * Create task state from Redis hash fields.
 */
export function stateFromHashFields(
  fields: Record<string, string>,
): TaskStateProjection {
  return {
    taskId: fields.taskId ?? "",
    status: (fields.status as TaskStatusEnum) ?? "pending",
    currentAgent: fields.currentAgent,
    progressPct: parseInt(fields.progressPct ?? "0", 10),
    lastEventSeq: parseInt(fields.lastEventSeq ?? "0", 10),
    outputBuffer: fields.outputBuffer,
    startedAt: fields.startedAt,
    updatedAt: fields.updatedAt ?? new Date().toISOString(),
    lastHeartbeat: fields.lastHeartbeat,
    error: fields.error,
  }
}

// ============================================================================
// Type Conversions
// ============================================================================

/**
 * Convert a base TaskEvent to a StreamTaskEvent.
 * This allows existing code to work with the new stream system.
 */
export function toStreamEvent(event: BaseTaskEvent): StreamTaskEvent {
  switch (event.type) {
    case "thought":
      return { type: "thought", data: event.data }
    case "tool_use":
      return { type: "tool_use", data: event.data }
    case "output":
      return { type: "output", data: event.data }
    case "confirmation":
      return { type: "confirmation", data: event.data }
    case "progress":
      return { type: "progress", data: event.data }
    case "debug_info":
      return { type: "debug_info", data: event.data }
    case "agent_info":
      return { type: "agent_info", data: event.data }
    case "skill_use":
      return { type: "skill_use", data: event.data }
    case "finish":
      return event.data.success
        ? {
            type: "task_completed",
            data: {
              output: event.data.output ?? "",
            },
          }
        : {
            type: "task_failed",
            data: {
              error: event.data.error ?? "Unknown error",
              recoverable: false,
            },
          }
    default:
      // For any unknown type, wrap as thought
      return { type: "thought", data: JSON.stringify(event) }
  }
}

/**
 * Convert a StreamTaskEvent to a base TaskEvent (for SSE compatibility).
 */
export function fromStreamEvent(event: StreamTaskEvent): BaseTaskEvent | null {
  switch (event.type) {
    case "thought":
      return { type: "thought", data: event.data }
    case "tool_use":
      return { type: "tool_use", data: event.data }
    case "output":
      return {
        type: "output",
        data:
          typeof event.data === "string"
            ? event.data
            : event.data.content,
      }
    case "confirmation":
      return { type: "confirmation", data: event.data }
    case "progress":
      return { type: "progress", data: event.data }
    case "debug_info":
      return { type: "debug_info", data: event.data }
    case "agent_info":
      return { type: "agent_info", data: event.data }
    case "skill_use":
      return { type: "skill_use", data: event.data }
    case "task_completed":
      return {
        type: "finish",
        data: { success: true, output: event.data.output },
      }
    case "task_failed":
      return {
        type: "finish",
        data: { success: false, error: event.data.error },
      }
    // Events without SSE equivalent
    case "task_created":
    case "task_started":
    case "heartbeat":
    case "agent_switch":
      return null
    default:
      return null
  }
}
