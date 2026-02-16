/**
 * Agent API Handler
 * Handles /api/agent endpoints for direct agent invocation
 *
 * This provides a simplified interface for invoking CodeCoder agents,
 * useful for integration with external systems like ZeroBot.
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

// ============================================================================
// Types
// ============================================================================

interface InvokeAgentRequest {
  /** Agent name: build, plan, decision, macro, trader, etc. */
  agent: string
  /** Prompt/message to send to the agent */
  prompt: string
  /** Optional session ID for conversation continuity */
  sessionId?: string
  /** Optional model override */
  model?: string
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/agents
 * List available agents
 */
export async function listAgents(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    // Import Agent dynamically to avoid circular dependencies
    const { Agent } = await import("../../../agent/agent")
    const agents = await Agent.list()

    const agentList = agents.map((agent) => ({
      id: agent.name,
      name: agent.name,
      description: agent.description,
    }))

    return jsonResponse({
      success: true,
      data: agentList,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/agent/invoke
 * Invoke an agent with a prompt
 *
 * This is a convenience endpoint that:
 * 1. Creates a session if sessionId not provided
 * 2. Sends the prompt to the specified agent
 * 3. Returns the session ID and message ID
 *
 * For real-time responses, use SSE endpoint /api/events
 */
export async function invokeAgent(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as InvokeAgentRequest

    if (!input.agent) {
      return errorResponse("agent is required", 400)
    }
    if (!input.prompt) {
      return errorResponse("prompt is required", 400)
    }

    // Validate agent exists
    const { Agent } = await import("../../../agent/agent")
    const agents = await Agent.list()
    const agentExists = agents.some((a) => a.name === input.agent)

    if (!agentExists) {
      return errorResponse(
        `Agent "${input.agent}" not found. Available: ${agents.map((a) => a.name).join(", ")}`,
        400,
      )
    }

    const { LocalSession } = await import("../../../api")

    // Create or reuse session
    let sessionId = input.sessionId
    if (!sessionId) {
      const session = await LocalSession.create({
        title: `Agent: ${input.agent}`,
      })
      sessionId = session.id
    }

    // Send message to agent
    const result = await LocalSession.prompt({
      sessionID: sessionId,
      agent: input.agent,
      model: input.model,
      parts: [{ type: "text", text: input.prompt }],
    })

    return jsonResponse(
      {
        success: true,
        data: {
          sessionId,
          messageId: result.messageID,
          agent: input.agent,
        },
      },
      201,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/agent/:agentId
 * Get information about a specific agent
 */
export async function getAgent(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { agentId } = params

    if (!agentId) {
      return errorResponse("Agent ID is required", 400)
    }

    const { Agent } = await import("../../../agent/agent")
    const agents = await Agent.list()
    const agent = agents.find((a) => a.name === agentId)

    if (!agent) {
      return errorResponse(`Agent "${agentId}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: {
        id: agent.name,
        name: agent.name,
        description: agent.description,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
