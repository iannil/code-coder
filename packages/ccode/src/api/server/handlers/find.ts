/**
 * Find API Handler
 * Handles /api/files endpoint for file search
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/files
 * Search for files
 */
export async function findFiles(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = req.url
    const query = url.searchParams.get("q")

    if (!query) {
      return jsonResponse({
        success: true,
        data: [],
      })
    }

    const { LocalFind } = await import("../../../api")
    const files = await LocalFind.files({ query })

    return jsonResponse({
      success: true,
      data: files,
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/files/cache
 * Search files using cache (same as /api/files)
 */
export async function findFilesCache(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  // For now, this is the same as findFiles
  return findFiles(req, _params)
}
