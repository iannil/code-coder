/**
 * Registry API Handler
 *
 * Provides agent registry endpoints for the Chat page and other clients.
 * Enables agent discovery, recommendations, and metadata retrieval.
 *
 * GET /api/v1/registry/agents - List all agents with metadata
 * GET /api/v1/registry/agents/:name - Get specific agent metadata
 * POST /api/v1/registry/recommend - Get recommended agent for user input
 * GET /api/v1/registry/search - Search agents by query
 * GET /api/v1/registry/categories - List available categories
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

function parseQueryParams(url: string | URL): URLSearchParams {
  try {
    const urlStr = typeof url === "string" ? url : url.toString()
    const urlObj = new URL(urlStr, "http://localhost")
    return urlObj.searchParams
  } catch {
    return new URLSearchParams()
  }
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * GET /api/v1/registry/agents
 *
 * List all registered agents with their metadata.
 * Optionally filter by category.
 */
export async function listAgents(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { getRegistry } = await import("../../../agent/registry")
    const registry = await getRegistry()

    const params = parseQueryParams(req.url)
    const category = params.get("category")

    const agents = category ? registry.listByCategory(category as any) : registry.list()

    // Sort by recommended first, then by name
    const sortedAgents = [...agents].sort((a, b) => {
      if (a.recommended && !b.recommended) return -1
      if (!a.recommended && b.recommended) return 1
      return a.name.localeCompare(b.name)
    })

    return jsonResponse({
      success: true,
      data: sortedAgents,
    })
  } catch (error) {
    console.error("Registry list error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/registry/agents/:name
 *
 * Get metadata for a specific agent.
 */
export async function getAgent(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const { name } = params

    if (!name) {
      return errorResponse("Agent name is required", 400)
    }

    const { getRegistry } = await import("../../../agent/registry")
    const registry = await getRegistry()
    const agent = registry.get(name)

    if (!agent) {
      return errorResponse(`Agent "${name}" not found`, 404)
    }

    return jsonResponse({
      success: true,
      data: agent,
    })
  } catch (error) {
    console.error("Registry get error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/registry/recommend
 *
 * Get recommended agent based on user intent.
 * Body: { intent: string }
 */
export async function recommendAgent(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as { intent: string }

    if (!input.intent) {
      return errorResponse("intent is required", 400)
    }

    const { getRegistry } = await import("../../../agent/registry")
    const registry = await getRegistry()

    const recommended = registry.recommend(input.intent)
    const alternates = registry.findByTrigger(input.intent).slice(0, 5)

    return jsonResponse({
      success: true,
      data: {
        recommended: recommended ?? null,
        alternates: alternates.filter((a) => a.name !== recommended?.name),
      },
    })
  } catch (error) {
    console.error("Registry recommend error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/registry/search
 *
 * Search agents by query string.
 * Query params: q (search query), limit (max results)
 */
export async function searchAgents(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const params = parseQueryParams(req.url)
    const query = params.get("q") ?? ""
    const limit = parseInt(params.get("limit") ?? "10", 10)

    if (!query.trim()) {
      return errorResponse("Search query (q) is required", 400)
    }

    const { getRegistry } = await import("../../../agent/registry")
    const registry = await getRegistry()
    const results = registry.search(query, { limit })

    return jsonResponse({
      success: true,
      data: results,
    })
  } catch (error) {
    console.error("Registry search error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/registry/categories
 *
 * List available agent categories with counts.
 */
export async function listCategories(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { getRegistry } = await import("../../../agent/registry")
    const registry = await getRegistry()
    const agents = registry.list()

    // Count agents per category
    const categoryCounts: Record<string, number> = {}
    for (const agent of agents) {
      const cat = agent.category || "custom"
      categoryCounts[cat] = (categoryCounts[cat] || 0) + 1
    }

    const categories = Object.entries(categoryCounts).map(([name, count]) => ({
      name,
      count,
      label: getCategoryLabel(name),
    }))

    return jsonResponse({
      success: true,
      data: categories,
    })
  } catch (error) {
    console.error("Registry categories error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/registry/recommended
 *
 * List recommended agents for new users.
 */
export async function listRecommended(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const { getRegistry } = await import("../../../agent/registry")
    const registry = await getRegistry()
    const recommended = registry.listRecommended()

    return jsonResponse({
      success: true,
      data: recommended,
    })
  } catch (error) {
    console.error("Registry recommended error:", error)
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

// ============================================================================
// Helper Functions
// ============================================================================

function getCategoryLabel(category: string): string {
  const labels: Record<string, string> = {
    engineering: "Engineering",
    content: "Content Creation",
    analysis: "Analysis",
    philosophy: "Philosophy",
    system: "System",
    custom: "Custom",
  }
  return labels[category] ?? category.charAt(0).toUpperCase() + category.slice(1)
}
