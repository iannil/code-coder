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

// ============================================================================
// Session Management
// ============================================================================

/** Maps conversation_id to session_id */
const conversationToSession = new Map<string, string>()

async function getOrCreateSession(conversationId: string | undefined): Promise<string> {
  const { LocalSession } = await import("../../../api")

  // If we have a conversation_id, check if we have a mapped session
  if (conversationId) {
    const existingSessionId = conversationToSession.get(conversationId)
    if (existingSessionId) {
      // Verify session still exists
      try {
        await LocalSession.get(existingSessionId)
        return existingSessionId
      } catch {
        // Session doesn't exist anymore, create a new one
        conversationToSession.delete(conversationId)
      }
    }
  }

  // Create a new session
  const session = await LocalSession.create({
    title: `Chat: ${new Date().toISOString()}`,
  })

  // Map conversation_id if provided
  if (conversationId) {
    conversationToSession.set(conversationId, session.id)
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
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as ChatRequest

    // Validate required fields
    if (!input.message) {
      return errorResponse("message is required", 400)
    }
    if (!input.user_id) {
      return errorResponse("user_id is required", 400)
    }
    if (!input.channel) {
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

    return jsonResponse({
      success: true,
      data: response,
    })
  } catch (error) {
    console.error("Chat API error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/chat/health
 *
 * Health check endpoint for the chat service.
 */
export async function chatHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  return jsonResponse({
    success: true,
    data: {
      status: "healthy",
      timestamp: new Date().toISOString(),
    },
  })
}
