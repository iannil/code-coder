/**
 * Task Event Emitter
 * Per-task SSE event streaming for ZeroBot integration
 */

import { Bus } from "@/bus"
import { BusEvent } from "@/bus/bus-event"
import { Instance } from "@/project/instance"
import { Log } from "@/util/log"
import z from "zod"
import type { TaskEvent } from "./types"

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
  }

  const state = Instance.state((): TaskEmitterState => {
    return {
      streams: new Map(),
    }
  })

  // ============================================================================
  // Event Emission
  // ============================================================================

  /**
   * Emit an event for a specific task
   * This will be received by all SSE subscribers for that task
   */
  export function emit(taskID: string, event: TaskEvent): void {
    const s = state()
    const controllers = s.streams.get(taskID)

    log.info("emitting task event", { taskID, type: event.type })

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
   * Emit a finish event (success or failure)
   */
  export function finish(taskID: string, success: boolean, output?: string, error?: string): void {
    emit(taskID, { type: "finish", data: { success, output, error } })

    // Close all streams for this task
    closeAllStreams(taskID)

    // Publish completion event
    Bus.publish(Event.TaskCompleted, { taskID, success, output, error })
  }

  // ============================================================================
  // SSE Subscription
  // ============================================================================

  /**
   * Subscribe to events for a specific task
   * Returns a ReadableStream that can be piped to SSE response
   */
  export function subscribe(taskID: string): ReadableStream<TaskEvent> {
    const s = state()

    return new ReadableStream<TaskEvent>({
      start(controller) {
        log.info("new SSE subscriber for task", { taskID })

        const streamController: StreamController = {
          controller,
          taskID,
        }

        // Add to streams map
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
}
