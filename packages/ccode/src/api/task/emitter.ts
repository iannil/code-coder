/**
 * Task Event Emitter
 * Per-task SSE event streaming for ZeroBot integration
 *
 * ## Dual Output Mode
 *
 * The emitter supports two output modes:
 * 1. **SSE (default)**: Direct streaming to connected clients
 * 2. **Redis Streams**: Persistent event log for reliable delivery
 *
 * When Redis Streams is enabled, events are written to both:
 * - `tasks:events:{task_id}` - Per-task event stream
 * - `tasks:state:{task_id}` - State projection (Redis Hash)
 */

import { Bus } from "@/bus"
import { BusEvent } from "@/bus/bus-event"
import { Instance } from "@/project/instance"
import { Log } from "@/util/log"
import z from "zod"
import type { TaskEvent } from "./types"
import {
  StreamTaskEvent,
  StreamEventEnvelope,
  createStreamEvent,
  toStreamEvent,
  applyEventToState,
  stateToHashFields,
  createInitialState,
} from "./events"
import {
  RedisStreamClient,
  streamKeys,
  isRedisStreamsAvailable,
} from "@/infrastructure/redis"

export namespace TaskEmitter {
  const log = Log.create({ service: "task-emitter" })

  // ============================================================================
  // Bus Events for Task System
  // ============================================================================

  export const Event = {
    /** Emitted when a task event occurs */
    TaskEvent: BusEvent.define(
      "task.event",
      z.object({
        taskID: z.string(),
        event: z.any(), // TaskEvent
      }),
    ),
    /** Emitted when a task starts */
    TaskStarted: BusEvent.define(
      "task.started",
      z.object({
        taskID: z.string(),
        agent: z.string(),
      }),
    ),
    /** Emitted when a task completes */
    TaskCompleted: BusEvent.define(
      "task.completed",
      z.object({
        taskID: z.string(),
        success: z.boolean(),
        output: z.string().optional(),
        error: z.string().optional(),
      }),
    ),
  }

  // ============================================================================
  // SSE Stream Management
  // ============================================================================

  interface StreamController {
    controller: ReadableStreamDefaultController<TaskEvent>
    taskID: string
  }

  interface TaskEmitterState {
    /** Map of taskID to set of stream controllers */
    streams: Map<string, Set<StreamController>>
    /** Map of taskID to cached events (for late subscribers) */
    eventCache: Map<string, TaskEvent[]>
    /** Map of taskID to event sequence number */
    eventSeq: Map<string, number>
    /** Redis client for stream publishing */
    redisClient: RedisStreamClient | null
    /** Whether Redis Streams is available */
    redisAvailable: boolean
  }

  const state = Instance.state((): TaskEmitterState => {
    return {
      streams: new Map(),
      eventCache: new Map(),
      eventSeq: new Map(),
      redisClient: null,
      redisAvailable: false,
    }
  })

  // ============================================================================
  // Initialization
  // ============================================================================

  /**
   * Initialize Redis Streams publishing (called during server startup).
   */
  export async function initRedisStreams(): Promise<void> {
    const s = state()

    try {
      const available = await isRedisStreamsAvailable()
      if (available) {
        const { getRedisStreamClient } = await import("@/infrastructure/redis")
        s.redisClient = await getRedisStreamClient()
        s.redisAvailable = true
        log.info("TaskEmitter: Redis Streams enabled")
      } else {
        log.info("TaskEmitter: Redis Streams not available, using SSE only")
      }
    } catch (error) {
      log.warn("TaskEmitter: Failed to initialize Redis Streams", {
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  // ============================================================================
  // Event Emission
  // ============================================================================

  /**
   * Emit an event for a specific task
   * This will be received by all SSE subscribers for that task
   * and optionally published to Redis Streams
   */
  export async function emit(taskID: string, event: TaskEvent): Promise<void> {
    const s = state()
    const controllers = s.streams.get(taskID)

    log.info("emitting task event", { taskID, type: event.type })

    // Cache the event for late subscribers
    const cached = s.eventCache.get(taskID) ?? []
    cached.push(event)
    s.eventCache.set(taskID, cached)

    // Publish to Bus for other listeners
    Bus.publish(Event.TaskEvent, { taskID, event })

    // Send to SSE streams
    if (controllers && controllers.size > 0) {
      for (const { controller } of controllers) {
        try {
          controller.enqueue(event)
        } catch (error) {
          log.warn("failed to enqueue event to stream", {
            taskID,
            error: error instanceof Error ? error.message : String(error),
          })
        }
      }
    }

    // Publish to Redis Streams if available
    if (s.redisAvailable && s.redisClient) {
      try {
        await publishToStream(taskID, event)
      } catch (error) {
        log.warn("failed to publish to Redis Stream", {
          taskID,
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }
  }

  /**
   * Publish event to Redis Stream.
   */
  async function publishToStream(
    taskID: string,
    event: TaskEvent,
  ): Promise<void> {
    const s = state()
    if (!s.redisClient) return

    // Get next sequence number
    const seq = (s.eventSeq.get(taskID) ?? 0) + 1
    s.eventSeq.set(taskID, seq)

    // Convert to stream event format
    const streamEvent = toStreamEvent(event)
    const envelope = createStreamEvent(seq, streamEvent)

    // Add to event stream
    await s.redisClient.xadd(streamKeys.taskEvents(taskID), envelope)

    // Update state projection
    const stateKey = streamKeys.taskState(taskID)
    const currentFields = await s.redisClient.hgetall(stateKey)

    const currentState =
      Object.keys(currentFields).length > 0
        ? {
            taskId: currentFields.taskId ?? taskID,
            status: (currentFields.status as any) ?? "running",
            currentAgent: currentFields.currentAgent,
            progressPct: parseInt(currentFields.progressPct ?? "0", 10),
            lastEventSeq: parseInt(currentFields.lastEventSeq ?? "0", 10),
            outputBuffer: currentFields.outputBuffer,
            startedAt: currentFields.startedAt,
            updatedAt: currentFields.updatedAt ?? new Date().toISOString(),
            lastHeartbeat: currentFields.lastHeartbeat,
            error: currentFields.error,
          }
        : createInitialState(taskID)

    const newState = applyEventToState(currentState, envelope)
    const newFields = stateToHashFields(newState)

    await s.redisClient.hset(stateKey, newFields)
  }

  /**
   * Emit a thought event
   */
  export function thought(taskID: string, data: string): void {
    emit(taskID, { type: "thought", data })
  }

  /**
   * Emit a tool use event
   */
  export function toolUse(taskID: string, tool: string, args: unknown, result?: unknown): void {
    emit(taskID, { type: "tool_use", data: { tool, args, result } })
  }

  /**
   * Emit an output event
   */
  export function output(taskID: string, data: string): void {
    emit(taskID, { type: "output", data })
  }

  /**
   * Emit a confirmation request event
   */
  export function confirmation(
    taskID: string,
    requestID: string,
    tool: string,
    description: string,
    args: unknown,
  ): void {
    emit(taskID, {
      type: "confirmation",
      data: {
        requestID,
        tool,
        description,
        args,
        actions: ["once", "always", "reject"],
      },
    })
  }

  /**
   * Emit a progress event
   */
  export function progress(taskID: string, stage: string, message: string, percentage?: number): void {
    emit(taskID, { type: "progress", data: { stage, message, percentage } })
  }

  /**
   * Emit a debug info event
   */
  export function debugInfo(taskID: string, data: {
    model?: string
    provider?: string
    input_tokens?: number
    output_tokens?: number
    total_tokens?: number
    duration_ms?: number
    request_bytes?: number
    response_bytes?: number
  }): void {
    emit(taskID, { type: "debug_info", data })
  }

  /**
   * Emit an agent info event
   */
  export function agentInfo(taskID: string, data: {
    agent: string
    display_name?: string
    is_primary?: boolean
    duration_ms?: number
  }): void {
    emit(taskID, { type: "agent_info", data })
  }

  /**
   * Emit a skill use event
   */
  export function skillUse(taskID: string, skill: string, args?: string, duration_ms?: number): void {
    emit(taskID, { type: "skill_use", data: { skill, args, duration_ms } })
  }

  /**
   * Emit a heartbeat event (for timeout management)
   */
  export function heartbeat(taskID: string, elapsedMs: number, stage?: string): void {
    const s = state()

    // Only publish to Redis Streams, not SSE
    if (s.redisAvailable && s.redisClient) {
      const seq = (s.eventSeq.get(taskID) ?? 0) + 1
      s.eventSeq.set(taskID, seq)

      const envelope = createStreamEvent(seq, {
        type: "heartbeat",
        data: { elapsedMs, stage },
      })

      s.redisClient.xadd(streamKeys.taskEvents(taskID), envelope).catch((error) => {
        log.warn("failed to publish heartbeat", {
          taskID,
          error: error instanceof Error ? error.message : String(error),
        })
      })
    }
  }

  /**
   * Emit a finish event (success or failure)
   */
  export function finish(taskID: string, success: boolean, output?: string, error?: string): void {
    emit(taskID, { type: "finish", data: { success, output, error } })

    // Close all streams for this task
    closeAllStreams(taskID)

    // Publish completion event
    Bus.publish(Event.TaskCompleted, { taskID, success, output, error })

    // Clean up sequence tracking
    const s = state()
    setTimeout(() => {
      s.eventSeq.delete(taskID)
    }, 30000)
  }

  // ============================================================================
  // SSE Subscription
  // ============================================================================

  /**
   * Subscribe to events for a specific task
   * Returns a ReadableStream that can be piped to SSE response
   * Late subscribers will receive cached events first
   */
  export function subscribe(taskID: string): ReadableStream<TaskEvent> {
    const s = state()

    return new ReadableStream<TaskEvent>({
      start(controller) {
        log.info("new SSE subscriber for task", { taskID })

        // First, send any cached events to catch up
        const cachedEvents = s.eventCache.get(taskID)
        if (cachedEvents && cachedEvents.length > 0) {
          log.info("sending cached events to late subscriber", {
            taskID,
            cachedCount: cachedEvents.length,
          })
          for (const event of cachedEvents) {
            try {
              controller.enqueue(event)
            } catch (error) {
              log.warn("failed to send cached event", {
                taskID,
                error: error instanceof Error ? error.message : String(error),
              })
            }
          }

          // If the last cached event is a finish event, close the stream
          const lastEvent = cachedEvents[cachedEvents.length - 1]
          if (lastEvent && lastEvent.type === "finish") {
            log.info("task already finished, closing stream after cached events", { taskID })
            controller.close()
            return
          }
        }

        const streamController: StreamController = {
          controller,
          taskID,
        }

        // Add to streams map for future events
        const existing = s.streams.get(taskID) ?? new Set()
        existing.add(streamController)
        s.streams.set(taskID, existing)
      },

      cancel() {
        log.info("SSE subscriber cancelled", { taskID })
        // Cleanup is handled by unsubscribe or closeAllStreams
      },
    })
  }

  /**
   * Subscribe to events from Redis Stream (for zero-channels).
   * Returns events starting from the given sequence number.
   */
  export async function subscribeFromStream(
    taskID: string,
    fromSeq: number = 0,
  ): Promise<StreamEventEnvelope[]> {
    const s = state()
    if (!s.redisClient) {
      return []
    }

    // Read all events from the task's stream
    const messages = await s.redisClient.xread(
      streamKeys.taskEvents(taskID),
      "0", // From beginning
      1000, // Max 1000 events
    )

    const events: StreamEventEnvelope[] = []
    for (const message of messages) {
      try {
        const envelope = s.redisClient.parsePayload<StreamEventEnvelope>(message)
        if (envelope.seq > fromSeq) {
          events.push(envelope)
        }
      } catch (error) {
        log.warn("failed to parse stream event", {
          taskID,
          messageId: message.id,
        })
      }
    }

    return events
  }

  /**
   * Unsubscribe a specific controller
   */
  export function unsubscribe(taskID: string, controller: ReadableStreamDefaultController<TaskEvent>): void {
    const s = state()
    const controllers = s.streams.get(taskID)

    if (controllers) {
      for (const sc of controllers) {
        if (sc.controller === controller) {
          controllers.delete(sc)
          break
        }
      }

      if (controllers.size === 0) {
        s.streams.delete(taskID)
      }
    }

    log.info("SSE subscriber unsubscribed", { taskID })
  }

  /**
   * Close all streams for a task
   */
  export function closeAllStreams(taskID: string): void {
    const s = state()
    const controllers = s.streams.get(taskID)

    if (controllers) {
      for (const { controller } of controllers) {
        try {
          controller.close()
        } catch (error) {
          // Stream may already be closed
          log.debug("stream already closed", { taskID })
        }
      }
      s.streams.delete(taskID)
      log.info("closed all SSE streams for task", { taskID, count: controllers.size })
    }

    // Delay cleanup of event cache to allow late subscribers to catch up
    // Clean up after 30 seconds
    setTimeout(() => {
      const currentState = state()
      currentState.eventCache.delete(taskID)
      log.debug("cleaned up event cache for task", { taskID })
    }, 30000)
  }

  /**
   * Get the number of active subscribers for a task
   */
  export function subscriberCount(taskID: string): number {
    return state().streams.get(taskID)?.size ?? 0
  }

  /**
   * Check if a task has any active subscribers
   */
  export function hasSubscribers(taskID: string): boolean {
    return subscriberCount(taskID) > 0
  }

  /**
   * Check if Redis Streams is available
   */
  export function isRedisEnabled(): boolean {
    return state().redisAvailable
  }
}
