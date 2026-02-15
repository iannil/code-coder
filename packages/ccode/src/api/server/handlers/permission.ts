/**
 * Permission API Handler
 * Handles /api/permissions/* endpoints
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
 * GET /api/permissions
 * List all pending permissions
 */
export async function listPermissions(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { LocalPermission } = await import("../../../api")
    const permissions = await LocalPermission.list()

    return jsonResponse({
      success: true,
      data: permissions,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/permissions/:id/respond
 * Respond to a permission request
 */
export async function respondPermission(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Permission ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as {
      reply: "once" | "always" | "reject"
      message?: string
    }

    if (!input.reply || !["once", "always", "reject"].includes(input.reply)) {
      return errorResponse("reply must be one of: once, always, reject", 400)
    }

    const { LocalPermission } = await import("../../../api")
    await LocalPermission.respond({
      sessionID: "", // Will be derived from the permission lookup
      permissionID: id,
      response: input.reply,
      message: input.message,
    })

    return jsonResponse({
      success: true,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/permissions/:id/reply
 * Reply to a next-generation permission request
 */
export async function replyPermission(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { id } = params

    if (!id) {
      return errorResponse("Permission ID is required", 400)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as {
      reply: string
      message?: string
    }

    const { LocalPermission } = await import("../../../api")
    await LocalPermission.reply({
      requestID: id,
      reply: input.reply as "once" | "always" | "reject",
      message: input.message,
    })

    return jsonResponse({
      success: true,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
