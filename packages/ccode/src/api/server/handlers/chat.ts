/**
 * Chat API Handler
 *
 * Provides a unified chat endpoint for IM channels via ZeroBot bridge.
 * Handles intent detection, agent routing, and message processing.
 *
 * POST /api/v1/chat - Send a message and receive a response
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { ConversationStore } from "../store/conversation"

// ============================================================================
// Types
// ============================================================================

interface ChatRequest {
  /** User message content */
  message: string
  /** Optional conversation ID for context continuity */
  conversation_id?: string
  /** Optional agent to use (auto-detected if not specified) */
  agent?: string
  /** User identifier */
  user_id: string
  /** Channel type (telegram, slack, discord, etc.) */
  channel: string
}

interface ChatResponse {
  /** Response message content */
  message: string
  /** Conversation ID for follow-up messages */
  conversation_id: string
  /** Agent used for this response */
  agent: string
  /** Token usage information */
  usage?: {
    input_tokens: number
    output_tokens: number
    total_tokens: number
  }
}

/** Tracing context extracted from HTTP headers */
interface TracingContext {
  traceId: string
  spanId: string
  parentSpanId?: string
  userId?: string
}

/** Lifecycle event for structured logging (ODD compliance) */
interface LifecycleEvent {
  timestamp: string
  trace_id: string
  span_id: string
  parent_span_id?: string
  event_type: "function_start" | "function_end" | "error" | "http_request" | "http_response"
  service: string
  payload: Record<string, unknown>
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
 * Extract text content from message parts.
 */
function extractTextFromParts(parts: Array<{ type: string; text?: string }>): string {
  return parts
    .filter((p) => p.type === "text" && p.text)
    .map((p) => p.text)
    .join("\n\n")
}

/**
 * Generate a unique span ID (8 character UUID prefix)
 */
function generateSpanId(): string {
  return crypto.randomUUID().slice(0, 8)
}

/**
 * Generate a unique trace ID (full UUID)
 */
function generateTraceId(): string {
  return crypto.randomUUID()
}

/**
 * Extract tracing context from HTTP headers
 */
function extractTracingContext(req: HttpRequest): TracingContext {
  const headers = req.headers
  const traceId = headers.get("X-Trace-Id") ?? generateTraceId()
  const parentSpanId = headers.get("X-Span-Id") ?? undefined
  const userId = headers.get("X-User-Id") ?? undefined

  return {
    traceId,
    spanId: generateSpanId(),
    parentSpanId,
    userId,
  }
}

/**
 * Log a lifecycle event in structured JSON format (ODD compliance)
 */
function logLifecycleEvent(ctx: TracingContext, eventType: LifecycleEvent["event_type"], payload: Record<string, unknown>) {
  const event: LifecycleEvent = {
    timestamp: new Date().toISOString(),
    trace_id: ctx.traceId,
    span_id: ctx.spanId,
    parent_span_id: ctx.parentSpanId,
    event_type: eventType,
    service: "codecoder-api",
    payload,
  }
  console.log(JSON.stringify(event))
}

// ============================================================================
// Session Management
// ============================================================================

async function getOrCreateSession(conversationId: string | undefined): Promise<string> {
  const { LocalSession } = await import("../../../api")

  // If we have a conversation_id, check Redis for existing session
  if (conversationId && ConversationStore.isInitialized()) {
    try {
      const existingSessionId = await ConversationStore.get(conversationId)
      if (existingSessionId) {
        // Verify session still exists
        try {
          await LocalSession.get(existingSessionId)
          return existingSessionId
        } catch {
          // Session doesn't exist anymore, delete stale mapping
          await ConversationStore.delete_(conversationId)
        }
      }
    } catch (redisError) {
      // Redis unavailable - log and continue to create new session
      console.error(
        JSON.stringify({
          timestamp: new Date().toISOString(),
          event: "redis_error",
          function: "getOrCreateSession",
          error: redisError instanceof Error ? redisError.message : String(redisError),
        }),
      )
    }
  }

  // Create a new session
  const session = await LocalSession.create({
    title: `Chat: ${new Date().toISOString()}`,
  })

  // Map conversation_id if provided and Redis is available
  if (conversationId && ConversationStore.isInitialized()) {
    try {
      await ConversationStore.set(conversationId, session.id)
    } catch (redisError) {
      console.error(
        JSON.stringify({
          timestamp: new Date().toISOString(),
          event: "redis_error",
          function: "getOrCreateSession.set",
          error: redisError instanceof Error ? redisError.message : String(redisError),
        }),
      )
    }
  }

  return session.id
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * POST /api/v1/chat
 *
 * Process a chat message and return a response.
 * This endpoint:
 * 1. Accepts a message from an IM channel
 * 2. Detects intent and routes to the appropriate agent
 * 3. Waits for the full response
 * 4. Returns the response with usage statistics
 */
export async function chat(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const startTime = performance.now()

  // Extract tracing context from headers
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", {
    function: "chat",
    method: req.method,
    url: req.url,
  })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as ChatRequest

    // Validate required fields
    if (!input.message) {
      logLifecycleEvent(ctx, "error", { function: "chat", error: "message is required" })
      return errorResponse("message is required", 400)
    }
    if (!input.user_id) {
      logLifecycleEvent(ctx, "error", { function: "chat", error: "user_id is required" })
      return errorResponse("user_id is required", 400)
    }
    if (!input.channel) {
      logLifecycleEvent(ctx, "error", { function: "chat", error: "channel is required" })
      return errorResponse("channel is required", 400)
    }

    // Import dependencies
    const { getRegistry } = await import("../../../agent/registry")
    const { SessionPrompt } = await import("../../../session/prompt")
    const { MessageV2 } = await import("../../../session/message-v2")

    // Get or create session
    const sessionId = await getOrCreateSession(input.conversation_id)

    // Determine which agent to use
    let agentName = input.agent
    if (!agentName) {
      // Use agent registry to recommend an agent based on intent
      const registry = await getRegistry()
      const recommended = registry.recommend(input.message)
      agentName = recommended?.name ?? "general"
    }

    // Validate agent exists
    const { Agent } = await import("../../../agent/agent")
    const agents = await Agent.list()
    const agentExists = agents.some((a) => a.name === agentName)

    if (!agentExists) {
      // Fall back to general if agent not found
      agentName = "general"
    }

    logLifecycleEvent(ctx, "http_request", {
      function: "chat",
      user_id: input.user_id,
      channel: input.channel,
      agent: agentName,
      session_id: sessionId,
    })

    // Send the message and wait for response
    const result = await SessionPrompt.prompt({
      sessionID: sessionId,
      agent: agentName,
      parts: [{ type: "text", text: input.message }],
    })

    // Extract text content from assistant message
    let responseText = ""
    let usage = {
      input_tokens: 0,
      output_tokens: 0,
      total_tokens: 0,
    }

    if (typeof result === "object" && "info" in result && "parts" in result) {
      // Full response with parts
      const assistantMsg = result as { info: { role: string; tokens?: { input: number; output: number } }; parts: Array<{ type: string; text?: string }> }
      responseText = extractTextFromParts(assistantMsg.parts)

      // Extract token usage from assistant message
      if (assistantMsg.info.tokens) {
        usage.input_tokens = assistantMsg.info.tokens.input
        usage.output_tokens = assistantMsg.info.tokens.output
        usage.total_tokens = usage.input_tokens + usage.output_tokens
      }
    } else if (typeof result === "string") {
      // Just a message ID - need to fetch the message
      const parts = await MessageV2.parts(result)
      responseText = extractTextFromParts(parts)
    }

    // If no text was extracted, provide a default
    if (!responseText.trim()) {
      responseText = "I processed your request but have no text response to provide."
    }

    const response: ChatResponse = {
      message: responseText,
      conversation_id: input.conversation_id ?? sessionId,
      agent: agentName,
      usage,
    }

    const durationMs = Math.round(performance.now() - startTime)
    logLifecycleEvent(ctx, "function_end", {
      function: "chat",
      duration_ms: durationMs,
      success: true,
      agent: agentName,
      tokens: usage.total_tokens,
    })

    return jsonResponse({
      success: true,
      data: response,
    })
  } catch (error) {
    const durationMs = Math.round(performance.now() - startTime)
    logLifecycleEvent(ctx, "error", {
      function: "chat",
      duration_ms: durationMs,
      error: error instanceof Error ? error.message : String(error),
    })

    console.error("Chat API error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/chat/health
 *
 * Health check endpoint for the chat service.
 */
export async function chatHealth(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "chatHealth" })

  const response = jsonResponse({
    success: true,
    data: {
      status: "healthy",
      timestamp: new Date().toISOString(),
    },
  })

  logLifecycleEvent(ctx, "function_end", { function: "chatHealth", success: true })

  return response
}

// ============================================================================
// Session Control Commands
// ============================================================================

interface ClearRequest {
  /** Conversation ID to clear */
  conversation_id: string
  /** User identifier */
  user_id: string
  /** Channel type */
  channel: string
}

interface CompactRequest {
  /** Conversation ID to compact */
  conversation_id: string
  /** User identifier */
  user_id: string
  /** Channel type */
  channel: string
}

/**
 * POST /api/v1/chat/clear
 *
 * Clear the conversation context (start fresh).
 * This removes the session mapping so next message creates a new session.
 */
export async function clearConversation(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "clearConversation" })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as ClearRequest

    if (!input.conversation_id) {
      return errorResponse("conversation_id is required", 400)
    }

    // Remove the session mapping
    let hadMapping = false
    let redisError: Error | null = null
    if (ConversationStore.isInitialized()) {
      try {
        hadMapping = await ConversationStore.delete_(input.conversation_id)
      } catch (err) {
        redisError = err instanceof Error ? err : new Error(String(err))
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "clearConversation",
            error: redisError.message,
          }),
        )
      }
    }

    logLifecycleEvent(ctx, "function_end", {
      function: "clearConversation",
      success: true,
      had_mapping: hadMapping,
      conversation_id: input.conversation_id,
      redis_error: redisError?.message,
    })

    // Return different messages based on actual result
    const message = redisError
      ? "⚠️ 清空上下文时出现错误，请重试。"
      : hadMapping
        ? "✨ 上下文已清空，下一条消息将开始新对话。"
        : "✨ 已准备开始新对话。"

    return jsonResponse({
      success: true,
      data: {
        message,
        message_en: redisError
          ? "Error clearing context, please retry."
          : hadMapping
            ? "Context cleared. Next message will start a new conversation."
            : "Ready to start a new conversation.",
        conversation_id: input.conversation_id,
        cleared: hadMapping,
        redis_error: redisError ? true : undefined,
      },
    })
  } catch (error) {
    logLifecycleEvent(ctx, "error", {
      function: "clearConversation",
      error: error instanceof Error ? error.message : String(error),
    })

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/chat/compact
 *
 * Compact the conversation context by summarizing the history.
 * This creates a new session with a summary of the previous conversation.
 */
export async function compactConversation(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "compactConversation" })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as CompactRequest

    if (!input.conversation_id) {
      return errorResponse("conversation_id is required", 400)
    }

    // Get the current session
    let sessionId: string | null = null
    if (ConversationStore.isInitialized()) {
      try {
        sessionId = await ConversationStore.get(input.conversation_id)
      } catch (redisError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "compactConversation.get",
            error: redisError instanceof Error ? redisError.message : String(redisError),
          }),
        )
      }
    }
    if (!sessionId) {
      return jsonResponse({
        success: true,
        data: {
          message: "没有活跃的会话需要压缩。",
          message_en: "No active session to compact.",
          conversation_id: input.conversation_id,
          compacted: false,
        },
      })
    }

    // Import dependencies
    const { LocalSession } = await import("../../../api")
    const { SessionPrompt } = await import("../../../session/prompt")

    // Get the current session messages
    const messages = await LocalSession.messages({
      sessionID: sessionId,
    })

    if (!messages || messages.length < 3) {
      return jsonResponse({
        success: true,
        data: {
          message: "会话消息太少，无需压缩。",
          message_en: "Session has too few messages to compact.",
          conversation_id: input.conversation_id,
          compacted: false,
          message_count: messages?.length ?? 0,
        },
      })
    }

    // Create a summary prompt
    const summaryPrompt = `请用中文简洁地总结以下对话的关键信息和上下文，以便继续对话时保持连贯性。只输出总结，不要其他内容。

对话历史：
${messages.map((m: { info: { role: string }; parts: Array<{ type: string; text?: string }> }) =>
  `${m.info.role === "user" ? "用户" : "助手"}: ${extractTextFromParts(m.parts).slice(0, 500)}`
).join("\n\n")}`

    // Get summary using a quick model
    const summaryResult = await SessionPrompt.prompt({
      sessionID: sessionId,
      agent: "general",
      parts: [{ type: "text", text: summaryPrompt }],
    })

    // Extract summary text
    let summaryText = ""
    if (typeof summaryResult === "object" && "parts" in summaryResult) {
      summaryText = extractTextFromParts(summaryResult.parts as Array<{ type: string; text?: string }>)
    }

    // Create a new session with the summary as initial context
    const newSession = await LocalSession.create({
      title: `Compacted: ${new Date().toISOString()}`,
    })

    // Send the summary as the first message to establish context
    await SessionPrompt.prompt({
      sessionID: newSession.id,
      agent: "general",
      parts: [{ type: "text", text: `[上下文摘要]\n${summaryText}\n\n请基于以上上下文继续对话。` }],
    })

    // Update the mapping
    if (ConversationStore.isInitialized()) {
      try {
        await ConversationStore.set(input.conversation_id, newSession.id)
      } catch (redisError) {
        console.error(
          JSON.stringify({
            timestamp: new Date().toISOString(),
            event: "redis_error",
            function: "compactConversation.set",
            error: redisError instanceof Error ? redisError.message : String(redisError),
          }),
        )
      }
    }

    const originalMessageCount = messages.length

    logLifecycleEvent(ctx, "function_end", {
      function: "compactConversation",
      success: true,
      conversation_id: input.conversation_id,
      original_messages: originalMessageCount,
      new_session_id: newSession.id,
    })

    return jsonResponse({
      success: true,
      data: {
        message: `上下文已压缩，从 ${originalMessageCount} 条消息精简为摘要。`,
        message_en: `Context compacted from ${originalMessageCount} messages to a summary.`,
        conversation_id: input.conversation_id,
        compacted: true,
        original_message_count: originalMessageCount,
        new_session_id: newSession.id,
      },
    })
  } catch (error) {
    logLifecycleEvent(ctx, "error", {
      function: "compactConversation",
      error: error instanceof Error ? error.message : String(error),
    })

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
