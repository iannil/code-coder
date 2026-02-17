/**
 * Session API Handler
 * Handles /api/sessions/* endpoints
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
// Handler Functions
// ============================================================================

/**
 * GET /api/sessions
 * List all sessions with optional filtering
 */
export async function listSessions(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url
    const limit = url.searchParams.get("limit")
    const search = url.searchParams.get("search")

    // Import Session API dynamically to avoid circular dependencies
    const { LocalSession } = await import("../../../api")

    const input: {
      directory?: string
      roots?: boolean
      search?: string
      limit?: number
    } = {}

    if (search) input.search = search
    if (limit) input.limit = Number.parseInt(limit, 10)

    const sessions = await LocalSession.list(input)

    return jsonResponse({
      success: true,
      data: sessions,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/sessions/:id
 * Get a specific session by ID
 */
export async function getSession(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Session ID is required", 400)
    }

    const { LocalSession } = await import("../../../api")
    const session = await LocalSession.get(id)

    if (!session) {
      return errorResponse("Session not found", 404)
    }

    return jsonResponse({
      success: true,
      data: session,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/sessions
 * Create a new session
 */
export async function createSession(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as { title?: string; parentID?: string }

    const { LocalSession } = await import("../../../api")
    const session = await LocalSession.create({
      title: input.title,
      parentID: input.parentID,
    })

    return jsonResponse(
      {
        success: true,
        data: session,
      },
      201,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/sessions/:id
 * Delete a session
 */
export async function deleteSession(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Session ID is required", 400)
    }

    const { LocalSession } = await import("../../../api")
    await LocalSession.remove(id)

    return jsonResponse(
      {
        success: true,
      },
      200,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/sessions/:id/messages
 * Get messages for a session
 */
export async function getSessionMessages(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Session ID is required", 400)
    }

    const url = req.url
    const limit = url.searchParams.get("limit")

    const { LocalSession } = await import("../../../api")
    const messages = await LocalSession.messages({
      sessionID: id,
      limit: limit ? Number.parseInt(limit, 10) : undefined,
    })

    return jsonResponse({
      success: true,
      data: messages,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/sessions/:id/messages
 * Send a message to a session
 */
export async function sendSessionMessage(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Session ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as {
      agent?: string
      model?: string
      variant?: string
      parts: Array<{ type: string; text?: string; url?: string; filename?: string; mime?: string }>
    }

    if (!input.parts || !Array.isArray(input.parts)) {
      return errorResponse("parts array is required", 400)
    }

    const { LocalSession } = await import("../../../api")
    const result = await LocalSession.prompt({
      sessionID: id,
      agent: input.agent,
      model: input.model,
      variant: input.variant,
      parts: input.parts,
    })

    return jsonResponse(
      {
        success: true,
        data: { messageID: result.messageID },
      },
      201,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/sessions/:id/children
 * Get child sessions
 */
export async function getSessionChildren(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Session ID is required", 400)
    }

    const { LocalSession } = await import("../../../api")
    const children = await LocalSession.children(id)

    return jsonResponse({
      success: true,
      data: children,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/sessions/:id/fork
 * Fork a session
 */
export async function forkSession(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Session ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = body ? (JSON.parse(body) as { messageID?: string }) : {}

    const { LocalSession } = await import("../../../api")
    const forked = await LocalSession.fork({
      sessionID: id,
      messageID: input.messageID,
    })

    return jsonResponse(
      {
        success: true,
        data: forked,
      },
      201,
    )
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
