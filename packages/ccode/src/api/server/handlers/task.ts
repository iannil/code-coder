/**
 * Task API Handler
 * Handles /api/v1/tasks endpoints for async task management
 *
 * Endpoints:
 * - POST /api/v1/tasks - Submit a new task
 * - GET /api/v1/tasks - List all tasks
 * - GET /api/v1/tasks/:id - Get task status
 * - GET /api/v1/tasks/:id/events - SSE event stream for task
 * - POST /api/v1/tasks/:id/interact - Human interaction (approve/reject)
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { TaskStore, TaskEmitter } from "@/api/task"
import { CreateTaskRequest, InteractTaskRequest, type TaskContext } from "@/api/task/types"
import { PermissionNext } from "@/permission/next"
import { Bus } from "@/bus"
import { Log } from "@/util/log"
import { shouldRequireApproval, allowForUser, loadAllowlists } from "@/security/remote-policy"
import { ConversationStore } from "../store/conversation"

const log = Log.create({ service: "task-handler" })

// ============================================================================
// Constants
// ============================================================================

/** Task execution timeout in milliseconds (5 minutes) */
const TASK_TIMEOUT_MS = 5 * 60 * 1000

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

function formatSSEEvent(event: string, data: string, id?: string): string {
  let output = ""
  if (id) {
    output += `id: ${id}\n`
  }
  output += `event: ${event}\n`
  output += `data: ${data}\n`
  output += "\n"
  return output
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * POST /api/v1/tasks
 * Submit a new task for execution
 */
export async function createTask(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    // Ensure allowlists are loaded from disk
    await loadAllowlists()

    const body = await readRequestBody(req.body)
    const input = CreateTaskRequest.parse(JSON.parse(body))

    // Validate agent exists
    const { Agent } = await import("@/agent/agent")
    const agents = await Agent.list()
    const agentExists = agents.some((a) => a.name === input.agent)

    if (!agentExists) {
      return errorResponse(
        `Agent "${input.agent}" not found. Available: ${agents.map((a) => a.name).join(", ")}`,
        400,
      )
    }

    // Create or reuse session (with ConversationStore integration for context continuity)
    const { LocalSession } = await import("@/api")

    let sessionID = input.sessionID
    const conversationId = input.context.conversationId

    // Try to look up existing session from ConversationStore
    if (!sessionID && conversationId && ConversationStore.isInitialized()) {
      try {
        const existingSessionId = await ConversationStore.get(conversationId)
        if (existingSessionId) {
          // Verify session still exists
          try {
            await LocalSession.get(existingSessionId)
            sessionID = existingSessionId
            log.info("reusing existing session from ConversationStore", {
              conversationId,
              sessionID,
            })
          } catch {
            // Session doesn't exist anymore, delete stale mapping
            await ConversationStore.delete_(conversationId)
            log.info("deleted stale session mapping", { conversationId, existingSessionId })
          }
        }
      } catch (redisError) {
        log.error("redis error in getOrCreateSession", {
          error: redisError instanceof Error ? redisError.message : String(redisError),
        })
      }
    }

    // Create new session if not found
    if (!sessionID) {
      const session = await LocalSession.create({
        title: `[Remote] ${input.agent}: ${input.prompt.slice(0, 50)}...`,
      })
      sessionID = session.id

      // Save mapping to ConversationStore
      if (conversationId && ConversationStore.isInitialized()) {
        try {
          await ConversationStore.set(conversationId, sessionID)
          log.info("saved session mapping to ConversationStore", { conversationId, sessionID })
        } catch (redisError) {
          log.error("redis error saving session mapping", {
            error: redisError instanceof Error ? redisError.message : String(redisError),
          })
        }
      }
    }

    // Create task
    const task = TaskStore.create({
      agent: input.agent,
      prompt: input.prompt,
      context: input.context,
      sessionID,
    })

    // Start task execution asynchronously
    executeTask(task.id, input.agent, input.prompt, sessionID, input.context, input.model).catch((error) => {
      TaskStore.fail(task.id, error instanceof Error ? error.message : String(error))
      TaskEmitter.finish(task.id, false, undefined, error instanceof Error ? error.message : String(error))
    })

    return jsonResponse(
      {
        success: true,
        data: task,
      },
      201,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Execute a task asynchronously
 */
async function executeTask(
  taskID: string,
  agent: string,
  prompt: string,
  sessionID: string,
  context: TaskContext,
  model?: string,
): Promise<void> {
  const { LocalSession } = await import("@/api")
  const startTime = Date.now()

  log.info("starting task execution", {
    taskID,
    agent,
    sessionID,
    userID: context.userID,
    platform: context.platform,
    promptLength: prompt.length,
  })

  // Mark task as running
  TaskStore.setRunning(taskID)
  TaskEmitter.progress(taskID, "starting", `Starting ${agent} agent...`)

  // Subscribe to permission requests for this session
  const unsubscribePermission = Bus.subscribe(PermissionNext.Event.Asked, (event) => {
    if (event.properties.sessionID !== sessionID) return

    // Check if this is a remote context that needs approval
    if (context.source === "remote") {
      if (shouldRequireApproval(event.properties.permission, context)) {
        log.info("permission requested", {
          taskID,
          requestID: event.properties.id,
          permission: event.properties.permission,
          patterns: event.properties.patterns,
        })
        // Emit confirmation event
        TaskStore.setAwaitingApproval(taskID, event.properties.id, event.properties.permission)
        TaskEmitter.confirmation(
          taskID,
          event.properties.id,
          event.properties.permission,
          `Permission requested: ${event.properties.permission} for patterns: ${event.properties.patterns.join(", ")}`,
          event.properties.metadata,
        )
      }
    }
  })

  try {
    // Send prompt to agent with timeout
    TaskEmitter.progress(taskID, "processing", `Processing with ${agent}...`)

    // Create timeout promise
    const timeoutPromise = new Promise<never>((_, reject) => {
      setTimeout(() => {
        log.warn("task execution timeout", {
          taskID,
          agent,
          timeoutMs: TASK_TIMEOUT_MS,
          elapsedMs: Date.now() - startTime,
        })
        reject(new Error(`Task execution timeout after ${TASK_TIMEOUT_MS / 1000} seconds`))
      }, TASK_TIMEOUT_MS)
    })

    // Race between prompt execution and timeout
    await Promise.race([
      LocalSession.prompt({
        sessionID,
        agent,
        model,
        parts: [{ type: "text", text: prompt }],
      }),
      timeoutPromise,
    ])

    // Get the response
    const messages = await LocalSession.messages({ sessionID })
    const lastAssistantMessage = messages.findLast((m) => m.info.role === "assistant")
    const output = lastAssistantMessage
      ? lastAssistantMessage.parts
          .filter((c): c is { type: "text"; text: string; id: string; sessionID: string; messageID: string } => c.type === "text")
          .map((c) => c.text)
          .join("\n")
      : "Task completed"

    const elapsedMs = Date.now() - startTime
    log.info("task completed successfully", {
      taskID,
      agent,
      elapsedMs,
      outputLength: output.length,
    })

    // Mark task as completed
    TaskStore.complete(taskID, output)
    TaskEmitter.finish(taskID, true, output)
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    const elapsedMs = Date.now() - startTime
    const isTimeout = errorMsg.includes("timeout")

    log.error("task execution failed", {
      taskID,
      agent,
      elapsedMs,
      isTimeout,
      error: errorMsg,
    })

    TaskStore.fail(taskID, errorMsg)
    TaskEmitter.finish(taskID, false, undefined, errorMsg)
  } finally {
    unsubscribePermission()
  }
}

/**
 * GET /api/v1/tasks
 * List all tasks
 */
export async function listTasks(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const tasks = TaskStore.list()
    return jsonResponse({
      success: true,
      data: tasks,
      meta: TaskStore.stats(),
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/tasks/:id
 * Get task status
 */
export async function getTask(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Task ID is required", 400)
    }

    const task = TaskStore.get(id)

    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: task,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/** SSE heartbeat interval in milliseconds (15 seconds) */
const SSE_HEARTBEAT_INTERVAL_MS = 15_000

/**
 * GET /api/v1/tasks/:id/events
 * SSE event stream for a specific task
 */
export async function streamTaskEvents(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Task ID is required", 400)
    }

    const task = TaskStore.get(id)

    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    // If task is already completed/failed, return immediately
    if (task.status === "completed" || task.status === "failed") {
      const encoder = new TextEncoder()
      const finishEvent = formatSSEEvent(
        "message",
        JSON.stringify({
          type: "finish",
          data: {
            success: task.status === "completed",
            output: task.output,
            error: task.error,
          },
        }),
      )

      return {
        status: 200,
        headers: {
          "Content-Type": "text/event-stream",
          "Cache-Control": "no-cache",
          Connection: "keep-alive",
          "X-Accel-Buffering": "no",
        },
        body: new ReadableStream({
          start(controller) {
            controller.enqueue(encoder.encode(finishEvent))
            controller.close()
          },
        }),
      }
    }

    // Subscribe to task events
    const eventStream = TaskEmitter.subscribe(id)
    const encoder = new TextEncoder()
    let eventCounter = 0
    let heartbeatInterval: ReturnType<typeof setInterval> | null = null
    let streamClosed = false

    // Create a readable stream that merges task events with heartbeats
    const readable = new ReadableStream({
      start(controller) {
        // Start heartbeat interval to keep connection alive
        heartbeatInterval = setInterval(() => {
          if (streamClosed) {
            if (heartbeatInterval) clearInterval(heartbeatInterval)
            return
          }
          try {
            // SSE comment line for keep-alive (clients ignore these)
            controller.enqueue(encoder.encode(": heartbeat\n\n"))
          } catch {
            // Stream may have been closed
            if (heartbeatInterval) clearInterval(heartbeatInterval)
          }
        }, SSE_HEARTBEAT_INTERVAL_MS)

        // Pipe task events to the stream
        const reader = eventStream.getReader()
        const pump = async () => {
          try {
            while (true) {
              const { done, value } = await reader.read()
              if (done || streamClosed) {
                break
              }
              try {
                eventCounter++
                const data = JSON.stringify(value)
                const sseEvent = formatSSEEvent("message", data, String(eventCounter))
                controller.enqueue(encoder.encode(sseEvent))

                // Check if this is a finish event
                if (value && typeof value === "object" && "type" in value && value.type === "finish") {
                  break
                }
              } catch (error) {
                const errorMsg = formatSSEEvent(
                  "error",
                  JSON.stringify({
                    message: error instanceof Error ? error.message : String(error),
                  }),
                )
                controller.enqueue(encoder.encode(errorMsg))
              }
            }
          } finally {
            streamClosed = true
            if (heartbeatInterval) clearInterval(heartbeatInterval)
            reader.releaseLock()
            controller.close()
          }
        }
        pump()
      },
      cancel() {
        streamClosed = true
        if (heartbeatInterval) clearInterval(heartbeatInterval)
      },
    })

    return {
      status: 200,
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
        "X-Accel-Buffering": "no",
      },
      body: readable,
    }
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/tasks/:id/interact
 * Human interaction endpoint (approve/reject)
 */
export async function interactTask(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Task ID is required", 400)
    }

    const task = TaskStore.get(id)

    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    if (task.status !== "awaiting_approval") {
      return errorResponse(`Task "${id}" is not awaiting approval. Current status: ${task.status}`, 400)
    }

    const body = await readRequestBody(req.body)
    const input = InteractTaskRequest.parse(JSON.parse(body))

    const pendingRequestID = TaskStore.getPendingConfirmation(id)

    if (!pendingRequestID) {
      return errorResponse(`No pending confirmation for task "${id}"`, 400)
    }

    // Get the full confirmation info including permission name
    const confirmationInfo = TaskStore.getPendingConfirmationInfo(id)

    // Reply to permission request
    if (input.action === "approve") {
      const reply = input.reply ?? "once"

      // If user chose "always", add the tool to their allowlist
      if (reply === "always" && confirmationInfo) {
        const userID = task.context.userID
        const permission = confirmationInfo.permission
        await allowForUser(userID, permission)
      }

      await PermissionNext.reply({
        requestID: pendingRequestID,
        reply: reply === "reject" ? "reject" : reply,
      })
      TaskStore.clearPendingConfirmation(id)
      TaskEmitter.progress(id, "approved", "Permission approved, continuing execution...")
    } else {
      await PermissionNext.reply({
        requestID: pendingRequestID,
        reply: "reject",
        message: input.reason,
      })
      TaskStore.clearPendingConfirmation(id)
      // Task will be marked as failed by the rejection error handler
    }

    const updatedTask = TaskStore.get(id)

    return jsonResponse({
      success: true,
      data: updatedTask,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/tasks/:id
 * Cancel/remove a task
 */
export async function deleteTask(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Task ID is required", 400)
    }

    const task = TaskStore.get(id)

    if (!task) {
      return errorResponse(`Task "${id}" not found`, 404)
    }

    // If task is running, we can't cancel it (for now)
    if (task.status === "running") {
      return errorResponse(`Cannot delete running task "${id}"`, 400)
    }

    TaskStore.remove(id)

    return jsonResponse({
      success: true,
      data: { deleted: true },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
