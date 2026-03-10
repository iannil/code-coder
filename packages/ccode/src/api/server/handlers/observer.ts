/**
 * Observer API Handlers
 *
 * HTTP API handlers that proxy requests to the Rust Observer Network API.
 *
 * ## Architecture (Phase 5)
 *
 * These handlers are now thin proxies that forward requests to the Rust daemon:
 *
 * ```
 * Client Request → TS API Handler → ObserverApiClient → Rust Daemon :4402
 * ```
 *
 * The Rust daemon (zero-cli) manages all Observer state including:
 * - Observer Network lifecycle (start/stop)
 * - Consensus Engine
 * - World Model
 * - SSE Event Stream
 *
 * @module api/server/handlers/observer
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { Log } from "@/util/log"
import {
  getObserverClient,
  type ApiResponse,
  type ObserverStatus,
  type GearStatus,
} from "@/observer/client"
import type { OperatingMode, HumanDecision } from "@/observer"

const log = Log.create({ service: "api.observer" })

// ============================================================================
// Types
// ============================================================================

interface EscalationCallbackBody {
  action: "approve" | "reject" | "defer" | "manual"
  userId?: string
  source?: string
  reason?: string
}

interface StartObserverBody {
  mode?: OperatingMode
  riskTolerance?: "conservative" | "balanced" | "aggressive"
  autoModeSwitch?: boolean
  watchers?: {
    code?: boolean
    world?: boolean
    self?: boolean
    meta?: boolean
  }
}

// ============================================================================
// Helpers
// ============================================================================

function jsonResponse(status: number, body: unknown): HttpResponse {
  return {
    status,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  }
}

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    return "{}"
  }
  const reader = body.getReader()
  const chunks: Uint8Array[] = []

  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    if (value) chunks.push(value)
  }

  const totalLength = chunks.reduce((acc, chunk) => acc + chunk.length, 0)
  const result = new Uint8Array(totalLength)
  let offset = 0
  for (const chunk of chunks) {
    result.set(chunk, offset)
    offset += chunk.length
  }

  return new TextDecoder().decode(result)
}

async function parseJsonBody<T>(req: HttpRequest): Promise<T | null> {
  try {
    const body = await readRequestBody(req.body)
    return JSON.parse(body) as T
  } catch {
    return null
  }
}

/**
 * Get the Observer API client.
 * The client connects to the Rust daemon for all operations.
 */
function getClient() {
  return getObserverClient()
}

/**
 * Convert API response to HTTP response.
 * Ensures consistent response format between TS and Rust layers.
 */
function apiToHttpResponse<T>(response: ApiResponse<T>, successStatus = 200): HttpResponse {
  if (response.success) {
    return jsonResponse(successStatus, response)
  }
  // Determine appropriate error status
  const errorStatus = response.error?.includes("not running") ? 503 : 500
  return jsonResponse(errorStatus, response)
}

// ============================================================================
// Handlers
// ============================================================================

/**
 * POST /api/v1/observer/escalations/:id/callback
 * Handle escalation decision callback from IM or webhook.
 *
 * Note: Escalations are managed by the Rust daemon but this endpoint
 * provides a TS-layer interface for legacy compatibility.
 */
export async function handleEscalationCallback(
  req: HttpRequest,
  params: RouteParams,
): Promise<HttpResponse> {
  const escalationId = params.id

  if (!escalationId) {
    return jsonResponse(400, { success: false, error: "Missing escalation ID" })
  }

  const body = await parseJsonBody<EscalationCallbackBody>(req)

  if (!body || !body.action) {
    return jsonResponse(400, { success: false, error: "Missing action in request body" })
  }

  // Note: Escalation handling is still managed by the local ModeController
  // until full migration. For now, return not implemented.
  log.warn("Escalation callback received but not yet proxied to Rust", {
    escalationId,
    action: body.action,
  })

  return jsonResponse(501, {
    success: false,
    error: "Escalation handling migration pending. Use Rust API directly.",
  })
}

/**
 * GET /api/v1/observer/escalations/:id
 * Get a specific escalation by ID.
 */
export async function getEscalation(
  _req: HttpRequest,
  params: RouteParams,
): Promise<HttpResponse> {
  const escalationId = params.id

  if (!escalationId) {
    return jsonResponse(400, { success: false, error: "Missing escalation ID" })
  }

  // Escalation management migration pending
  return jsonResponse(501, {
    success: false,
    error: "Escalation retrieval migration pending. Use Rust API directly.",
  })
}

/**
 * GET /api/v1/observer/escalations
 * List all pending escalations.
 */
export async function listEscalations(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  // Escalation management migration pending
  return jsonResponse(501, {
    success: false,
    error: "Escalation listing migration pending. Use Rust API directly.",
  })
}

/**
 * GET /api/v1/observer/status
 * Get Observer Network status.
 *
 * Proxies to Rust: GET /api/v1/observer/status
 */
export async function getObserverStatus(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const client = getClient()

  try {
    const response = await client.getStatus()

    if (response.success && response.data) {
      // Transform to match legacy format expected by TUI
      return jsonResponse(200, {
        running: response.data.running,
        mode: "HYBRID", // Mode is managed by Gear system now
        watchers: [], // Watcher details are internal to Rust now
        stats: {
          observations: response.data.streamStats.received,
          patterns: response.data.activePatterns,
          anomalies: response.data.activeAnomalies,
        },
        controller: {
          opportunities: response.data.activeOpportunities,
        },
      })
    }

    return apiToHttpResponse(response)
  } catch (error) {
    log.error("Failed to get observer status from Rust API", {
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(503, {
      success: false,
      error: "Observer daemon not available",
    })
  }
}

/**
 * POST /api/v1/observer/start
 * Start the Observer Network.
 *
 * Proxies to Rust: POST /api/v1/observer/start
 */
export async function startObserver(
  req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const body = await parseJsonBody<StartObserverBody>(req)
  const client = getClient()

  try {
    const response = await client.start()

    if (response.success) {
      log.info("Observer Network started via Rust API")

      return jsonResponse(200, {
        success: true,
        mode: body?.mode ?? "HYBRID",
      })
    }

    return apiToHttpResponse(response)
  } catch (error) {
    log.error("Failed to start Observer Network via Rust API", {
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(500, {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}

/**
 * POST /api/v1/observer/stop
 * Stop the Observer Network.
 *
 * Proxies to Rust: POST /api/v1/observer/stop
 */
export async function stopObserver(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const client = getClient()

  try {
    const response = await client.stop()

    if (response.success) {
      log.info("Observer Network stopped via Rust API")

      return jsonResponse(200, {
        success: true,
      })
    }

    return apiToHttpResponse(response)
  } catch (error) {
    log.error("Failed to stop Observer Network via Rust API", {
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(500, {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}

/**
 * POST /api/v1/observer/mode
 * Switch Observer Network operating mode.
 *
 * Note: Mode is now controlled via Gear system.
 * Proxies to Rust: POST /api/v1/gear/switch
 */
export async function switchObserverMode(
  req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const body = await parseJsonBody<{ mode: OperatingMode; reason?: string }>(req)

  if (!body || !body.mode) {
    return jsonResponse(400, { success: false, error: "Missing mode in request body" })
  }

  const validModes: OperatingMode[] = ["AUTO", "MANUAL", "HYBRID"]
  if (!validModes.includes(body.mode)) {
    return jsonResponse(400, {
      success: false,
      error: `Invalid mode: ${body.mode}. Must be one of: ${validModes.join(", ")}`,
    })
  }

  // Map mode to gear
  const modeToGear: Record<OperatingMode, string> = {
    AUTO: "S",
    MANUAL: "N",
    HYBRID: "D",
  }

  const client = getClient()

  try {
    const gear = modeToGear[body.mode]
    const response = await client.switchGear(gear as any, body.reason ?? "Mode switch via API")

    if (response.success) {
      log.info("Observer mode switched via Rust API", {
        mode: body.mode,
        gear,
        reason: body.reason,
      })

      return jsonResponse(200, {
        success: true,
        mode: body.mode,
      })
    }

    return apiToHttpResponse(response)
  } catch (error) {
    log.error("Failed to switch Observer mode via Rust API", {
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(500, {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
}

/**
 * GET /api/v1/observer/world-model
 * Get the current world model.
 *
 * Proxies to Rust: GET /api/v1/observer/world-model
 */
export async function getWorldModel(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const client = getClient()

  try {
    const response = await client.getWorldModel()

    return jsonResponse(200, {
      success: true,
      worldModel: response.data ?? null,
    })
  } catch (error) {
    log.error("Failed to get world model from Rust API", {
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(503, {
      success: false,
      error: "Observer daemon not available",
    })
  }
}

/**
 * GET /api/v1/observer/snapshot
 * Get the consensus snapshot.
 *
 * Proxies to Rust: GET /api/v1/observer/consensus
 */
export async function getConsensusSnapshot(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const client = getClient()

  try {
    const response = await client.getConsensus()

    return jsonResponse(200, {
      success: true,
      snapshot: response.data ?? null,
    })
  } catch (error) {
    log.error("Failed to get consensus snapshot from Rust API", {
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(503, {
      success: false,
      error: "Observer daemon not available",
    })
  }
}

/**
 * GET /api/v1/observer/opportunities
 * Get active opportunities.
 *
 * Proxies to Rust: GET /api/v1/observer/opportunities
 */
export async function getOpportunities(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const client = getClient()

  try {
    const response = await client.getOpportunities()

    return jsonResponse(200, {
      success: true,
      opportunities: response.data ?? [],
      count: response.data?.length ?? 0,
    })
  } catch (error) {
    log.error("Failed to get opportunities from Rust API", {
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(503, {
      success: false,
      error: "Observer daemon not available",
    })
  }
}

/**
 * GET /api/v1/observer/health
 * Health check for Observer API.
 */
export async function observerHealth(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const client = getClient()

  try {
    const response = await client.getStatus()

    return jsonResponse(200, {
      healthy: true,
      observerRunning: response.success && response.data?.running === true,
      mode: response.success ? "HYBRID" : null,
      timestamp: new Date().toISOString(),
    })
  } catch {
    return jsonResponse(200, {
      healthy: true,
      observerRunning: false,
      mode: null,
      timestamp: new Date().toISOString(),
    })
  }
}
