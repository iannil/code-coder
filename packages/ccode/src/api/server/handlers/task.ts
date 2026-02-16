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
import { shouldRequireApproval } from "@/security/remote-policy"

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

    // Create or reuse session
    const { LocalSession } = await import("@/api")

    let sessionID = input.sessionID
    if (!sessionID) {
      const session = await LocalSession.create({
        title: `[Remote] ${input.agent}: ${input.prompt.slice(0, 50)}...`,
      })
      sessionID = session.id
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

  // Mark task as running
  TaskStore.setRunning(taskID)
  TaskEmitter.progress(taskID, "starting", `Starting ${agent} agent...`)

  // Subscribe to permission requests for this session
  const unsubscribePermission = Bus.subscribe(PermissionNext.Event.Asked, (event) => {
    if (event.properties.sessionID !== sessionID) return

    // Check if this is a remote context that needs approval
    if (context.source === "remote") {
      if (shouldRequireApproval(event.properties.permission, context)) {
        // Emit confirmation event
        TaskStore.setAwaitingApproval(taskID, event.properties.id)
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
    // Send prompt to agent
    TaskEmitter.progress(taskID, "processing", `Processing with ${agent}...`)

    const result = await LocalSession.prompt({
      sessionID,
      agent,
      model,
      parts: [{ type: "text", text: prompt }],
    })

    // Get the response
    const messages = await LocalSession.messages({ sessionID })
    const lastAssistantMessage = messages.findLast((m) => m.info.role === "assistant")
    const output = lastAssistantMessage
      ? lastAssistantMessage.parts
          .filter((c): c is { type: "text"; text: string; id: string; sessionID: string; messageID: string } => c.type === "text")
          .map((c) => c.text)
          .join("\n")
      : "Task completed"

    // Mark task as completed
    TaskStore.complete(taskID, output)
    TaskEmitter.finish(taskID, true, output)
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error)
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

    const transformStream = new TransformStream({
      transform(chunk, controller) {
        try {
          eventCounter++
          const data = JSON.stringify(chunk)
          const sseEvent = formatSSEEvent("message", data, String(eventCounter))
          controller.enqueue(encoder.encode(sseEvent))
        } catch (error) {
          const errorMsg = formatSSEEvent(
            "error",
            JSON.stringify({
              message: error instanceof Error ? error.message : String(error),
            }),
          )
          controller.enqueue(encoder.encode(errorMsg))
        }
      },
    })

    const readable = eventStream.pipeThrough(transformStream)

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

    // Reply to permission request
    if (input.action === "approve") {
      const reply = input.reply ?? "once"
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
