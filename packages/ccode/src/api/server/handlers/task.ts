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
import { TaskStore, TaskEmitter, TaskContextRegistry } from "@/api/task"
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
// Type Definitions
// ============================================================================

/**
 * Error information from assistant message info object.
 */
interface AssistantError {
  name?: string
  message?: string
  data?: { message?: string }
}

/**
 * Tool invocation structure in message parts.
 */
interface ToolInvocationPart {
  type: "tool"
  toolInvocation: {
    state: string
    toolName: string
    result: string
  }
}

/**
 * Type guard to check if a message part is a tool invocation.
 */
function isToolInvocationPart(part: unknown): part is ToolInvocationPart {
  if (typeof part !== "object" || part === null) return false
  const p = part as Record<string, unknown>
  if (p.type !== "tool") return false
  if (typeof p.toolInvocation !== "object" || p.toolInvocation === null) return false
  return true
}

/**
 * Type guard to check if a value is an AssistantError.
 */
function isAssistantError(value: unknown): value is AssistantError {
  if (typeof value !== "object" || value === null) return false
  return true
}

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

/**
 * Get the display name for an agent.
 */
function getAgentDisplayName(agent: string): string | undefined {
  const displayNames: Record<string, string> = {
    "build": "构建工程师",
    "plan": "规划专家",
    "code-reviewer": "代码审查员",
    "security-reviewer": "安全审计员",
    "tdd-guide": "TDD指导",
    "architect": "架构师",
    "writer": "写作助手",
    "proofreader": "校对员",
    "code-reverse": "代码逆向工程师",
    "jar-code-reverse": "JAR逆向工程师",
    "observer": "观察者",
    "decision": "决策顾问",
    "macro": "宏观经济分析师",
    "trader": "交易顾问",
    "picker": "选品专家",
    "miniproduct": "极小产品教练",
    "ai-engineer": "AI工程师导师",
    "general": "通用助手",
    "autonomous": "自主代理"
  }
  return displayNames[agent]
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

/**
 * Check if a tool is a "result tool" - one whose output is the user's final goal.
 * These results should be accumulated to the final IM message.
 *
 * Examples: WebSearch (user wants search results), WebFetch (user wants page content)
 */
function isResultTool(toolName: string): boolean {
  const lower = toolName.toLowerCase()

  // Web search and content fetching tools
  if (lower.includes("websearch") || lower.includes("web_search")
    || lower.includes("webfetch") || lower.includes("web_fetch")
    || lower.includes("mcp__web_search")) {
    return true
  }

  // Content reaching tools (YouTube, Bilibili, RSS, etc.)
  if (lower.includes("reach_youtube") || lower.includes("reach_bilibili")
    || lower.includes("reach_rss") || lower.includes("network_analyzer")) {
    return true
  }

  return false
}

/**
 * Check if a tool is an "intermediate tool" - AI uses these for processing.
 * Results should NOT be accumulated; they're just for AI's internal work.
 *
 * Examples: Read (AI reads to analyze), Bash (AI runs commands to check things)
 */
function isIntermediateTool(toolName: string): boolean {
  const lower = toolName.toLowerCase()

  // File operations
  if (lower.includes("read") || lower.includes("write")
    || lower.includes("edit") || lower.includes("multiedit")
    || lower.includes("apply_patch") || lower === "ls") {
    return true
  }

  // Code search tools
  if (lower.includes("grep") || lower.includes("glob")
    || lower.includes("codesearch") || lower.includes("code_search")) {
    return true
  }

  // System/execution tools
  if (lower.includes("bash") || lower.includes("batch")
    || lower.includes("lsp") || lower.includes("language_server")) {
    return true
  }

  // Task/agent management
  if (lower.includes("task") || lower.includes("plan")
    || lower.includes("todo") || lower.includes("skill")) {
    return true
  }

  return false
}

/**
 * Check if a tool is "sensitive" - results should NEVER be shown in IM.
 *
 * Examples: credential (passwords, tokens), secret management
 */
function isSensitiveTool(toolName: string): boolean {
  const lower = toolName.toLowerCase()
  return lower.includes("credential")
    || lower.includes("secret")
    || lower.includes("password")
    || lower.includes("token")
}

/**
 * Format tool result for inclusion in task output.
 * Used when AI only uses tools without generating text.
 *
 * Only accumulates results from "result tools" - tools whose output
 * is the user's actual goal. Intermediate tools (Read, Bash, etc.)
 * are not accumulated since their output is just for AI processing.
 */
function formatToolResult(toolName: string, result: string): string | null {
  // Skip sensitive tools entirely
  if (isSensitiveTool(toolName)) {
    return null
  }

  // Skip intermediate tools - their results are for AI, not user
  if (isIntermediateTool(toolName)) {
    return null
  }

  const normalizedTool = toolName.toLowerCase()
  const MAX_OUTPUT_LENGTH = 2000

  // For WebSearch results, try to parse and format nicely
  if (normalizedTool.includes("search")) {
    try {
      const parsed = JSON.parse(result)
      if (parsed.results && Array.isArray(parsed.results)) {
        if (parsed.results.length === 0) {
          return "🌐 搜索完成，未找到结果"
        }
        let formatted = "## 搜索结果\n\n"
        for (let i = 0; i < parsed.results.length && i < 10; i++) {
          const item = parsed.results[i]
          formatted += `${i + 1}. [${item.title || "无标题"}]`
          if (item.url) formatted += `(${item.url})`
          formatted += "\n"
          if (item.snippet) formatted += `   ${item.snippet}\n`
        }
        return formatted
      }
    } catch {
      // Not JSON, continue to general formatting
    }
  }

  // For WebFetch results
  if (normalizedTool.includes("fetch")) {
    try {
      const parsed = JSON.parse(result)
      if (parsed.content) {
        return `## 网页内容\n\n${parsed.content}`
      }
      if (parsed.url) {
        return `## 已获取网页\n\nURL: ${parsed.url}`
      }
    } catch {
      // Not JSON, continue to general formatting
    }
  }

  // For result tools, format with truncation for large outputs
  const toolDisplay = formatToolDisplayName(toolName)
  if (result.length > MAX_OUTPUT_LENGTH) {
    return `${toolDisplay} ${toolName}\n\n\`\`\`\n${result.slice(0, MAX_OUTPUT_LENGTH)}...\n\`\`\`\n[输出已截断，共 ${result.length} 字符]`
  } else if (result.length === 0) {
    return `${toolDisplay} ${toolName}\n[无输出]`
  } else {
    return `${toolDisplay} ${toolName}\n\n\`\`\`\n${result}\n\`\`\``
  }
}

/**
 * Format tool display name from tool name.
 */
function formatToolDisplayName(toolName: string): string {
  const displayNames: Record<string, string> = {
    read: "📄 读取文件",
    write: "✏️ 写入文件",
    edit: "🔧 编辑文件",
    bash: "💻 执行命令",
    grep: "🔍 搜索代码",
    glob: "📁 查找文件",
    websearch: "🌐 网络搜索",
    webfetch: "🌐 获取网页",
    task: "🤖 启动子任务",
  }
  const normalized = toolName.toLowerCase()
  return displayNames[normalized] || "⚡ 执行工具"
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

    // Register session -> task mapping for SSE event routing
    TaskContextRegistry.register(sessionID, task.id)

    // Start task execution asynchronously
    executeTask(task.id, input.agent, input.prompt, sessionID, input.context, input.model)
      .finally(() => {
        // Cleanup registration after task completes
        TaskContextRegistry.unregister(sessionID)
      })
      .catch((error) => {
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
    log.info("task: retrieved messages for output", {
      taskID,
      sessionID,
      totalMessages: messages.length,
      roles: messages.map((m) => m.info.role),
    })

    const lastAssistantMessage = messages.findLast((m) => m.info.role === "assistant")

    // Emit agent info event with the primary agent used
    TaskEmitter.agentInfo(taskID, {
      agent,
      display_name: getAgentDisplayName(agent),
      is_primary: true,
      duration_ms: Date.now() - startTime,
    })

    // Emit debug info event if usage information is available
    if (lastAssistantMessage && lastAssistantMessage.info.role === "assistant") {
      const assistantInfo = lastAssistantMessage.info as { modelID?: string; providerID?: string; tokens?: { input: number; output: number; reasoning?: number; cache?: { read: number; write: number } } }

      if (assistantInfo.tokens) {
        const totalTokens = assistantInfo.tokens.input +
          assistantInfo.tokens.output +
          (assistantInfo.tokens.reasoning || 0)

        TaskEmitter.debugInfo(taskID, {
          model: model || assistantInfo.modelID,
          provider: assistantInfo.providerID,
          input_tokens: assistantInfo.tokens.input,
          output_tokens: assistantInfo.tokens.output,
          total_tokens: totalTokens,
        })
      }
    }

    if (lastAssistantMessage) {
      log.info("task: found assistant message", {
        taskID,
        messageID: lastAssistantMessage.info.id,
        partsCount: lastAssistantMessage.parts.length,
        partsTypes: lastAssistantMessage.parts.map((p) => p.type),
        partsDetail: lastAssistantMessage.parts.map((p) => ({
          type: p.type,
          hasText: "text" in p && !!p.text,
          textLength: "text" in p ? p.text?.length ?? 0 : 0,
          isSynthetic: "synthetic" in p ? p.synthetic : undefined,
          isIgnored: "ignored" in p ? p.ignored : undefined,
        })),
      })
    } else {
      log.warn("task: no assistant message found", { taskID, messagesCount: messages.length })
    }

    // Check for errors in assistant message (error is in info object)
    const assistantInfo = lastAssistantMessage?.info as { error?: unknown } | undefined
    const assistantError = isAssistantError(assistantInfo?.error) ? assistantInfo.error : undefined
    const hasError = assistantError !== undefined

    // Filter text parts, excluding synthetic ones (like tool call summaries)
    const textParts = lastAssistantMessage
      ? lastAssistantMessage.parts.filter((c) => {
          if (c.type !== "text") return false
          // Skip synthetic parts (system reminders, tool call descriptions, etc.)
          if ("synthetic" in c && c.synthetic) return false
          return true
        })
      : []

    // Filter tool result parts (for cases where AI only used tools without text output)
    const toolResultParts = lastAssistantMessage
      ? lastAssistantMessage.parts.filter((c) => {
          if (!isToolInvocationPart(c)) return false
          return c.toolInvocation.state === "result"
        })
      : []

    log.info("task: filtered text and tool parts", {
      taskID,
      textPartsCount: textParts.length,
      toolResultPartsCount: toolResultParts.length,
      hasError,
      errorName: assistantError?.name,
    })

    // Generate output from text parts, tool results, or error information
    let output: string
    if (textParts.length > 0) {
      // Prefer text parts if available
      output = textParts.map((c) => (c as { text: string }).text).join("\n")
    } else if (hasError && assistantError) {
      // Extract error information using typed interface
      const errorMsg =
        assistantError.data?.message ||
        assistantError.message ||
        assistantError.name ||
        "Unknown error"
      output = `❌ 处理失败: ${errorMsg}`
      log.warn("task: using error message as output", { taskID, error: errorMsg })
    } else if (toolResultParts.length > 0) {
      // Scheme 2: If no text parts but tool results exist, format tool results
      // This handles cases where AI only uses tools (e.g., WebSearch) without text output
      const formattedToolResults = toolResultParts
        .map((part) => {
          // Type guard ensures part has toolInvocation
          if (!isToolInvocationPart(part)) return null
          return formatToolResult(part.toolInvocation.toolName, part.toolInvocation.result)
        })
        .filter(Boolean)
        .join("\n\n")

      output = formattedToolResults || "✅ 处理完成"
      log.info("task: using tool results as output", {
        taskID,
        toolCount: toolResultParts.length,
        outputLength: output.length,
      })
    } else {
      output = "✅ 处理完成"
    }

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
