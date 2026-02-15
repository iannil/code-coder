/**
 * Config API Handler
 * Handles /api/config endpoints
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
 * GET /api/config
 * Get current configuration
 */
export async function getConfig(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { LocalConfig } = await import("../../../api")
    const config = await LocalConfig.get()

    return jsonResponse({
      success: true,
      data: config,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * PUT /api/config
 * Update configuration
 */
export async function updateConfig(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const updates = JSON.parse(body) as Record<string, unknown>

    const { LocalConfig } = await import("../../../api")
    await LocalConfig.update(updates)

    return jsonResponse({
      success: true,
      data: updates,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}
