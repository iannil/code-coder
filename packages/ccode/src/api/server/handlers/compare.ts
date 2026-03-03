/**
 * Compare API Handler
 *
 * Provides multi-model A/B testing endpoint for parallel model comparison.
 * Calls multiple AI models in parallel and returns all responses for comparison.
 *
 * POST /api/v1/compare - Compare responses from multiple models
 * GET /api/v1/compare/history - Get comparison history
 * POST /api/v1/compare/:id/vote - Vote for a model response
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import { Provider } from "../../../provider/provider"
import { generateText } from "ai"
import { randomUUID } from "node:crypto"
import os from "os"
import path from "path"
import { Log } from "@/util/log"

const log = Log.create({ service: "api.compare" })

// ============================================================================
// Types
// ============================================================================

interface CompareRequest {
  /** Models to query (e.g., ["anthropic/claude-sonnet-4", "openai/gpt-4o"]) */
  models: string[]
  /** The prompt to send to all models */
  prompt: string
  /** Optional system prompt */
  system?: string
  /** Maximum tokens to generate (default: 4096) */
  max_tokens?: number
  /** Temperature (0.0 - 1.0, default: 0.7) */
  temperature?: number
}

interface ModelResult {
  /** Full model identifier (provider/model) */
  model: string
  /** Provider name */
  provider: string
  /** Model ID within provider */
  model_id: string
  /** Response content */
  content: string
  /** Token usage */
  tokens: {
    input: number
    output: number
    total: number
  }
  /** Response latency in milliseconds */
  latency_ms: number
  /** Error message if failed */
  error?: string
}

interface CompareResponse {
  /** Results from each model */
  results: ModelResult[]
  /** Total tokens used across all models */
  total_tokens: number
  /** Total latency in milliseconds (max of all models) */
  total_latency_ms: number
}

// ============================================================================
// Comparison History Types
// ============================================================================

interface ComparisonHistoryEntry {
  /** Unique identifier */
  id: string
  /** Timestamp of comparison */
  timestamp: number
  /** The prompt sent to all models */
  prompt: string
  /** System prompt if provided */
  system?: string
  /** Models used in this comparison */
  models: string[]
  /** Results from each model */
  results: ModelResult[]
  /** Total tokens used */
  total_tokens: number
  /** Total latency in milliseconds */
  total_latency_ms: number
  /** Votes for each model (model -> vote count) */
  votes: Record<string, number>
  /** User ratings for each model (model -> rating 1-5) */
  ratings: Record<string, number[]>
  /** User ID who created this comparison */
  user_id?: string
  /** Optional tags for organization */
  tags?: string[]
}

interface VoteRequest {
  /** Model to vote for */
  model: string
  /** Rating (1-5, optional) */
  rating?: number
  /** User ID */
  user_id?: string
}

// ============================================================================
// File-backed History Storage
// ============================================================================

const MAX_HISTORY_SIZE = 100
const HISTORY_FILE_NAME = "compare-history.json"

/** Get the path to the history file */
function getHistoryFilePath(): string {
  const dataDir = path.join(os.homedir(), ".codecoder", "data")
  return path.join(dataDir, HISTORY_FILE_NAME)
}

/** In-memory cache of comparison history */
let comparisonHistoryCache: Map<string, ComparisonHistoryEntry> | null = null
let historyLoadPromise: Promise<void> | null = null

/** Load history from disk (lazy, cached) */
async function loadHistory(): Promise<Map<string, ComparisonHistoryEntry>> {
  if (comparisonHistoryCache !== null) {
    return comparisonHistoryCache
  }

  // Avoid concurrent loads
  if (historyLoadPromise) {
    await historyLoadPromise
    return comparisonHistoryCache!
  }

  historyLoadPromise = (async () => {
    comparisonHistoryCache = new Map()
    const filePath = getHistoryFilePath()

    try {
      const file = Bun.file(filePath)
      if (await file.exists()) {
        const data = await file.json()
        if (Array.isArray(data)) {
          for (const entry of data) {
            comparisonHistoryCache.set(entry.id, entry)
          }
        }
      }
    } catch (error) {
      log.warn("Failed to load comparison history", { error: error instanceof Error ? error.message : String(error) })
    }
  })()

  await historyLoadPromise
  historyLoadPromise = null
  return comparisonHistoryCache!
}

/** Save history to disk */
async function saveHistory(): Promise<void> {
  const history = await loadHistory()
  const filePath = getHistoryFilePath()
  const dataDir = path.dirname(filePath)

  try {
    // Ensure data directory exists
    await Bun.write(
      path.join(dataDir, ".keep"),
      ""
    ).catch(() => {})

    // Convert Map to array for JSON serialization
    const entries = Array.from(history.values())
    await Bun.write(filePath, JSON.stringify(entries, null, 2))
  } catch (error) {
    log.error("Failed to save comparison history", { error: error instanceof Error ? error.message : String(error) })
  }
}

/** Add entry to history with persistence */
async function addToHistory(entry: ComparisonHistoryEntry): Promise<void> {
  const history = await loadHistory()

  // Enforce max history size (remove oldest entries)
  if (history.size >= MAX_HISTORY_SIZE) {
    const oldestKey = history.keys().next().value
    if (oldestKey) history.delete(oldestKey)
  }

  history.set(entry.id, entry)
  await saveHistory()
}

// ============================================================================
// Request Body Reader
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) return ""
  const reader = body.getReader()
  const chunks: Uint8Array[] = []
  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    if (value) chunks.push(value)
  }
  return new TextDecoder().decode(Buffer.concat(chunks))
}

// ============================================================================
// Helper Functions
// ============================================================================

async function callModel(
  modelSpec: string,
  prompt: string,
  options: { system?: string; max_tokens?: number; temperature?: number },
): Promise<ModelResult> {
  const startTime = performance.now()
  const parsed = Provider.parseModel(modelSpec)

  try {
    // Get model info and language model instance
    const modelInfo = await Provider.getModel(parsed.providerID, parsed.modelID)
    const language = await Provider.getLanguage(modelInfo)

    // Generate text using AI SDK
    const result = await generateText({
      model: language,
      prompt,
      system: options.system,
      maxOutputTokens: options.max_tokens ?? 4096,
      temperature: options.temperature ?? 0.7,
    })

    const latencyMs = Math.round(performance.now() - startTime)

    return {
      model: modelSpec,
      provider: parsed.providerID,
      model_id: parsed.modelID,
      content: result.text,
      tokens: {
        input: result.usage?.inputTokens ?? 0,
        output: result.usage?.outputTokens ?? 0,
        total: (result.usage?.inputTokens ?? 0) + (result.usage?.outputTokens ?? 0),
      },
      latency_ms: latencyMs,
    }
  } catch (error) {
    const latencyMs = Math.round(performance.now() - startTime)
    return {
      model: modelSpec,
      provider: parsed.providerID,
      model_id: parsed.modelID,
      content: "",
      tokens: { input: 0, output: 0, total: 0 },
      latency_ms: latencyMs,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * POST /api/v1/compare
 *
 * Compare responses from multiple AI models.
 * This endpoint:
 * 1. Accepts a list of models and a prompt
 * 2. Calls each model in parallel
 * 3. Returns all responses with timing and usage statistics
 */
export async function compare(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as CompareRequest

    // Validate required fields
    if (!input.models || !Array.isArray(input.models) || input.models.length === 0) {
      return errorResponse("models array is required and must not be empty", 400)
    }
    if (!input.prompt) {
      return errorResponse("prompt is required", 400)
    }
    if (input.models.length > 5) {
      return errorResponse("Maximum 5 models allowed per request", 400)
    }

    // Validate model format
    for (const model of input.models) {
      if (!model.includes("/")) {
        return errorResponse(`Invalid model format: ${model}. Expected format: provider/model`, 400)
      }
    }

    // Call all models in parallel
    const results = await Promise.all(
      input.models.map((model) =>
        callModel(model, input.prompt, {
          system: input.system,
          max_tokens: input.max_tokens,
          temperature: input.temperature,
        }),
      ),
    )

    // Calculate totals
    const totalTokens = results.reduce((sum, r) => sum + r.tokens.total, 0)
    const maxLatency = Math.max(...results.map((r) => r.latency_ms))

    const response: CompareResponse = {
      results,
      total_tokens: totalTokens,
      total_latency_ms: maxLatency,
    }

    // Store in history
    const historyEntry: ComparisonHistoryEntry = {
      id: randomUUID(),
      timestamp: Date.now(),
      prompt: input.prompt,
      system: input.system,
      models: input.models,
      results,
      total_tokens: totalTokens,
      total_latency_ms: maxLatency,
      votes: {},
      ratings: {},
    }

    // Add to history with persistence
    await addToHistory(historyEntry)

    return jsonResponse({
      success: true,
      data: {
        ...response,
        id: historyEntry.id,
      },
    })
  } catch (error) {
    log.error("Compare API error", { error: error instanceof Error ? error.message : String(error) })
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/compare/health
 *
 * Health check endpoint for the compare service.
 */
export async function compareHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  return jsonResponse({
    success: true,
    data: {
      status: "healthy",
      timestamp: new Date().toISOString(),
      max_models: 5,
    },
  })
}

/**
 * GET /api/v1/compare/models
 *
 * List available models for comparison.
 * Returns connected providers with their models.
 */
export async function listCompareModels(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const providers = await Provider.list()

    const models: Array<{
      id: string
      provider: string
      name: string
      capabilities: {
        reasoning: boolean
        toolcall: boolean
      }
    }> = []

    for (const [providerID, provider] of Object.entries(providers)) {
      for (const [modelID, model] of Object.entries(provider.models)) {
        models.push({
          id: `${providerID}/${modelID}`,
          provider: providerID,
          name: model.name,
          capabilities: {
            reasoning: model.capabilities.reasoning,
            toolcall: model.capabilities.toolcall,
          },
        })
      }
    }

    return jsonResponse({
      success: true,
      data: {
        models,
        total: models.length,
      },
    })
  } catch (error) {
    log.error("List compare models error", { error: error instanceof Error ? error.message : String(error) })
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/compare/history
 *
 * Get comparison history with optional filtering.
 * Query params:
 *   - limit: Maximum number of entries to return (default: 20)
 *   - offset: Number of entries to skip (default: 0)
 */
export async function getCompareHistory(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const url = new URL(req.url)
    const limit = Math.min(parseInt(url.searchParams.get("limit") ?? "20", 10), 100)
    const offset = parseInt(url.searchParams.get("offset") ?? "0", 10)

    const history = await loadHistory()

    // Convert to array and sort by timestamp (newest first)
    const entries = Array.from(history.values())
      .sort((a, b) => b.timestamp - a.timestamp)
      .slice(offset, offset + limit)

    // Map to response format (omit full results for list view)
    const items = entries.map((entry) => ({
      id: entry.id,
      timestamp: entry.timestamp,
      prompt: entry.prompt.slice(0, 200) + (entry.prompt.length > 200 ? "..." : ""),
      models: entry.models,
      total_tokens: entry.total_tokens,
      total_latency_ms: entry.total_latency_ms,
      votes: entry.votes,
      vote_count: Object.values(entry.votes).reduce((a, b) => a + b, 0),
      avg_rating: calculateAverageRating(entry.ratings),
    }))

    return jsonResponse({
      success: true,
      data: {
        items,
        total: history.size,
        limit,
        offset,
      },
    })
  } catch (error) {
    log.error("Get compare history error", { error: error instanceof Error ? error.message : String(error) })
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/compare/history/:id
 *
 * Get a specific comparison entry with full details.
 */
export async function getCompareEntry(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Comparison ID is required", 400)
    }

    const history = await loadHistory()
    const entry = history.get(id)
    if (!entry) {
      return errorResponse("Comparison not found", 404)
    }

    return jsonResponse({
      success: true,
      data: {
        ...entry,
        avg_rating: calculateAverageRating(entry.ratings),
      },
    })
  } catch (error) {
    log.error("Get compare entry error", { error: error instanceof Error ? error.message : String(error) })
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/compare/:id/vote
 *
 * Vote for a model response in a comparison.
 * Body: { model: string, rating?: number (1-5), user_id?: string }
 */
export async function voteForModel(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Comparison ID is required", 400)
    }

    const history = await loadHistory()
    const entry = history.get(id)
    if (!entry) {
      return errorResponse("Comparison not found", 404)
    }

    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as VoteRequest

    if (!input.model) {
      return errorResponse("model is required", 400)
    }

    // Verify the model was part of this comparison
    if (!entry.models.includes(input.model)) {
      return errorResponse(`Model ${input.model} was not part of this comparison`, 400)
    }

    // Increment vote count
    entry.votes[input.model] = (entry.votes[input.model] ?? 0) + 1

    // Add rating if provided
    if (input.rating !== undefined) {
      const rating = Math.max(1, Math.min(5, Math.round(input.rating)))
      if (!entry.ratings[input.model]) {
        entry.ratings[input.model] = []
      }
      entry.ratings[input.model].push(rating)
    }

    // Update the entry and persist
    history.set(id, entry)
    await saveHistory()

    return jsonResponse({
      success: true,
      data: {
        id: entry.id,
        votes: entry.votes,
        avg_rating: calculateAverageRating(entry.ratings),
        message: `Vote recorded for ${input.model}`,
      },
    })
  } catch (error) {
    log.error("Vote error", { error: error instanceof Error ? error.message : String(error) })
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/compare/history/:id
 *
 * Delete a comparison entry from history.
 */
export async function deleteCompareEntry(req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const id = params.id
    if (!id) {
      return errorResponse("Comparison ID is required", 400)
    }

    const history = await loadHistory()
    if (!history.has(id)) {
      return errorResponse("Comparison not found", 404)
    }

    history.delete(id)
    await saveHistory()

    return jsonResponse({
      success: true,
      data: {
        message: "Comparison deleted successfully",
      },
    })
  } catch (error) {
    log.error("Delete compare entry error", { error: error instanceof Error ? error.message : String(error) })
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * Helper function to calculate average rating across all models
 */
function calculateAverageRating(ratings: Record<string, number[]>): Record<string, number> {
  const result: Record<string, number> = {}
  for (const [model, modelRatings] of Object.entries(ratings)) {
    if (modelRatings.length > 0) {
      result[model] = modelRatings.reduce((a, b) => a + b, 0) / modelRatings.length
    }
  }
  return result
}
