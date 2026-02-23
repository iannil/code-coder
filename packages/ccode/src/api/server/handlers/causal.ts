/**
 * Causal Graph Handler
 *
 * API handlers for the Causal Graph system.
 * Provides endpoints for recording and querying causal chains
 * (Decision → Action → Outcome).
 *
 * Part of Phase 16: 因果链图数据库 (Causal Graph)
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { z } from "zod"
import { CausalGraph } from "@/memory/knowledge/causal-graph"
import { CausalAnalysis } from "@/memory/knowledge/causal-analysis"
import {
  RecordDecisionRequestSchema,
  RecordActionRequestSchema,
  RecordOutcomeRequestSchema,
  CausalQuerySchema,
} from "@/memory/knowledge/causal-types"

// ============================================================================
// Request Body Helper
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

// ============================================================================
// Decision Endpoints
// ============================================================================

/**
 * POST /api/v1/causal/decisions
 * Record a new decision
 */
export async function recordDecision(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = RecordDecisionRequestSchema.parse(JSON.parse(body))

    const decision = await CausalGraph.recordDecision(input)

    return jsonResponse({ success: true, data: decision }, 201)
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/causal/decisions/:id
 * Get a specific decision
 */
export async function getDecision(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Decision ID is required", 400)
  }

  const decision = await CausalGraph.getDecision(id)

  if (!decision) {
    return errorResponse(`Decision "${id}" not found`, 404)
  }

  return jsonResponse({ success: true, data: decision })
}

// ============================================================================
// Action Endpoints
// ============================================================================

/**
 * POST /api/v1/causal/actions
 * Record a new action
 */
export async function recordAction(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = RecordActionRequestSchema.parse(JSON.parse(body))

    const action = await CausalGraph.recordAction(input)

    return jsonResponse({ success: true, data: action }, 201)
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    if (error instanceof Error && error.message.includes("not found")) {
      return errorResponse(error.message, 404)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/causal/actions/:id
 * Get a specific action
 */
export async function getAction(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Action ID is required", 400)
  }

  const action = await CausalGraph.getAction(id)

  if (!action) {
    return errorResponse(`Action "${id}" not found`, 404)
  }

  return jsonResponse({ success: true, data: action })
}

// ============================================================================
// Outcome Endpoints
// ============================================================================

/**
 * POST /api/v1/causal/outcomes
 * Record a new outcome
 */
export async function recordOutcome(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = RecordOutcomeRequestSchema.parse(JSON.parse(body))

    const outcome = await CausalGraph.recordOutcome(input)

    return jsonResponse({ success: true, data: outcome }, 201)
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    if (error instanceof Error && error.message.includes("not found")) {
      return errorResponse(error.message, 404)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/causal/outcomes/:id
 * Get a specific outcome
 */
export async function getOutcome(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Outcome ID is required", 400)
  }

  const outcome = await CausalGraph.getOutcome(id)

  if (!outcome) {
    return errorResponse(`Outcome "${id}" not found`, 404)
  }

  return jsonResponse({ success: true, data: outcome })
}

// ============================================================================
// Chain Endpoints
// ============================================================================

/**
 * GET /api/v1/causal/chain/:id
 * Get complete causal chain for a decision
 */
export async function getCausalChain(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { id } = params

  if (!id) {
    return errorResponse("Decision ID is required", 400)
  }

  const chain = await CausalGraph.getCausalChain(id)

  if (!chain) {
    return errorResponse(`Decision "${id}" not found`, 404)
  }

  return jsonResponse({ success: true, data: chain })
}

/**
 * GET /api/v1/causal/chains
 * Get causal chains for a session
 */
export async function getCausalChains(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const url = req.url
  const sessionId = url.searchParams.get("sessionId")

  if (!sessionId) {
    return errorResponse("sessionId query parameter is required", 400)
  }

  const chains = await CausalGraph.getCausalChainsForSession(sessionId)

  return jsonResponse({
    success: true,
    data: chains,
    meta: { count: chains.length },
  })
}

// ============================================================================
// Query & Analysis Endpoints
// ============================================================================

/**
 * POST /api/v1/causal/query
 * Complex query against the causal graph
 */
export async function queryCausalGraph(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const query = CausalQuerySchema.parse(JSON.parse(body))

    const chains = await CausalGraph.query(query)

    return jsonResponse({
      success: true,
      data: chains,
      meta: { count: chains.length, query },
    })
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/causal/patterns
 * Get recurring causal patterns
 */
export async function getCausalPatterns(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const url = req.url
  const agentId = url.searchParams.get("agentId") ?? undefined
  const minOccurrences = parseInt(url.searchParams.get("minOccurrences") || "2", 10)
  const limit = parseInt(url.searchParams.get("limit") || "20", 10)

  const patterns = await CausalAnalysis.findPatterns({
    agentId,
    minOccurrences,
    limit,
  })

  return jsonResponse({
    success: true,
    data: patterns,
    meta: { count: patterns.length },
  })
}

/**
 * GET /api/v1/causal/patterns/success
 * Get successful patterns
 */
export async function getSuccessPatterns(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const url = req.url
  const agentId = url.searchParams.get("agentId") ?? undefined
  const minSuccessRate = parseFloat(url.searchParams.get("minSuccessRate") || "0.7")

  const patterns = await CausalAnalysis.findSuccessPatterns({
    agentId,
    minSuccessRate,
  })

  return jsonResponse({
    success: true,
    data: patterns,
    meta: { count: patterns.length },
  })
}

/**
 * GET /api/v1/causal/patterns/failure
 * Get failure patterns
 */
export async function getFailurePatterns(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const url = req.url
  const agentId = url.searchParams.get("agentId") ?? undefined
  const maxSuccessRate = parseFloat(url.searchParams.get("maxSuccessRate") || "0.3")

  const patterns = await CausalAnalysis.findFailurePatterns({
    agentId,
    maxSuccessRate,
  })

  return jsonResponse({
    success: true,
    data: patterns,
    meta: { count: patterns.length },
  })
}

/**
 * GET /api/v1/causal/stats
 * Get comprehensive statistics
 */
export async function getCausalStats(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const stats = await CausalGraph.getStats()

  return jsonResponse({ success: true, data: stats })
}

/**
 * POST /api/v1/causal/suggest
 * Get suggestions based on historical data
 */
export async function getCausalSuggestions(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = z
      .object({
        prompt: z.string(),
        agentId: z.string(),
        context: z
          .object({
            files: z.array(z.string()).optional(),
            tools: z.array(z.string()).optional(),
          })
          .optional(),
      })
      .parse(JSON.parse(body))

    const suggestions = await CausalAnalysis.suggestFromHistory(input)

    return jsonResponse({
      success: true,
      data: suggestions,
      meta: { count: suggestions.length },
    })
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Validation error: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/causal/trends
 * Get trend analysis
 */
export async function getCausalTrends(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const url = req.url
  const agentId = url.searchParams.get("agentId") ?? undefined
  const periodDays = parseInt(url.searchParams.get("periodDays") || "7", 10)

  const trends = await CausalAnalysis.analyzeTrends({ agentId, periodDays })

  return jsonResponse({ success: true, data: trends })
}

/**
 * GET /api/v1/causal/insights/:agentId
 * Get aggregated insights for an agent
 */
export async function getAgentInsights(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { agentId } = params

  if (!agentId) {
    return errorResponse("Agent ID is required", 400)
  }

  const insights = await CausalAnalysis.getAgentInsights(agentId)

  return jsonResponse({ success: true, data: insights })
}

/**
 * GET /api/v1/causal/lessons/:outcomeId
 * Extract lessons from an outcome
 */
export async function getLessons(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  const { outcomeId } = params

  if (!outcomeId) {
    return errorResponse("Outcome ID is required", 400)
  }

  const lesson = await CausalAnalysis.extractLessons(outcomeId)

  if (!lesson) {
    return errorResponse(`Outcome "${outcomeId}" not found`, 404)
  }

  return jsonResponse({ success: true, data: lesson })
}

// ============================================================================
// Visualization Endpoints
// ============================================================================

/**
 * GET /api/v1/causal/graph
 * Get full graph data for visualization
 */
export async function getCausalGraphData(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const graph = await CausalGraph.load()

  return jsonResponse({
    success: true,
    data: {
      nodes: {
        decisions: graph.nodes.decisions.length,
        actions: graph.nodes.actions.length,
        outcomes: graph.nodes.outcomes.length,
      },
      edges: graph.edges.length,
      time: graph.time,
    },
  })
}

/**
 * GET /api/v1/causal/mermaid
 * Get Mermaid diagram representation
 */
export async function getCausalMermaid(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  const url = req.url
  const maxNodes = parseInt(url.searchParams.get("maxNodes") || "50", 10)
  const decisionId = url.searchParams.get("decisionId") ?? undefined

  const mermaid = await CausalGraph.toMermaid({ maxNodes, decisionId })

  return jsonResponse({
    success: true,
    data: { mermaid },
  })
}

// ============================================================================
// Health Check
// ============================================================================

/**
 * GET /api/v1/causal/health
 * Health check for causal graph service
 */
export async function causalHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const graph = await CausalGraph.load()

    return jsonResponse({
      success: true,
      data: {
        status: "healthy",
        projectId: graph.projectId,
        counts: {
          decisions: graph.nodes.decisions.length,
          actions: graph.nodes.actions.length,
          outcomes: graph.nodes.outcomes.length,
          edges: graph.edges.length,
        },
        lastUpdated: new Date(graph.time.updated).toISOString(),
      },
    })
  } catch (error) {
    return jsonResponse({
      success: true,
      data: {
        status: "degraded",
        error: error instanceof Error ? error.message : String(error),
      },
    })
  }
}
