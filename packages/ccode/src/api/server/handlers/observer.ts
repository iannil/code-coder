/**
 * Observer API Handlers
 *
 * HTTP API handlers for Observer Network integration.
 * Provides endpoints for:
 * - Escalation callbacks from IM channels
 * - Observer Network status and control
 * - Pending escalation management
 *
 * @module api/server/handlers/observer
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { Log } from "@/util/log"
import { ObserverNetwork, type HumanDecision, type OperatingMode } from "@/observer"

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

// ============================================================================
// Handlers
// ============================================================================

/**
 * POST /api/v1/observer/escalations/:id/callback
 * Handle escalation decision callback from IM or webhook.
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

  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(503, { success: false, error: "Observer Network not running" })
  }

  // Convert action to HumanDecision
  const decision: HumanDecision = {
    action: body.action === "manual" ? "modify" : body.action,
    chosenMode: body.action === "manual" ? "MANUAL" : undefined,
    reason: body.reason ?? `Via ${body.source ?? "API"} by ${body.userId ?? "unknown"}`,
    timestamp: new Date(),
  }

  try {
    await network.handleHumanDecision(escalationId, decision)

    log.info("Escalation callback processed", {
      escalationId,
      action: body.action,
      source: body.source,
    })

    return jsonResponse(200, {
      success: true,
      escalationId,
      action: body.action,
    })
  } catch (error) {
    log.error("Escalation callback failed", {
      escalationId,
      error: error instanceof Error ? error.message : String(error),
    })

    return jsonResponse(500, {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    })
  }
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

  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(503, { success: false, error: "Observer Network not running" })
  }

  const escalations = network.getPendingEscalations()
  const escalation = escalations.find((e) => e.id === escalationId)

  if (!escalation) {
    return jsonResponse(404, { success: false, error: "Escalation not found" })
  }

  return jsonResponse(200, {
    success: true,
    escalation,
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
  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(503, { success: false, error: "Observer Network not running" })
  }

  const escalations = network.getPendingEscalations()

  return jsonResponse(200, {
    success: true,
    escalations,
    count: escalations.length,
  })
}

/**
 * GET /api/v1/observer/status
 * Get Observer Network status.
 */
export async function getObserverStatus(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(200, {
      running: false,
    })
  }

  const stats = network.getStats()
  const watchers = network.getWatcherStatuses()
  const mode = network.getMode()
  const controllerStats = network.getModeControllerStats()

  return jsonResponse(200, {
    running: true,
    mode,
    watchers: watchers.map((w) => ({
      id: w.id,
      type: w.type,
      running: w.running,
      health: w.health,
      observationCount: w.observationCount,
      avgLatency: w.avgLatency,
    })),
    stats,
    controller: controllerStats,
  })
}

/**
 * POST /api/v1/observer/start
 * Start the Observer Network.
 */
export async function startObserver(
  req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const body = await parseJsonBody<StartObserverBody>(req)

  // Check if already running
  const existing = ObserverNetwork.getInstance()
  if (existing?.isRunning()) {
    return jsonResponse(200, {
      success: true,
      message: "Observer Network already running",
      mode: existing.getMode(),
    })
  }

  try {
    const network = await ObserverNetwork.start({
      mode: body?.mode ?? "HYBRID",
      riskTolerance: body?.riskTolerance ?? "balanced",
      autoModeSwitch: body?.autoModeSwitch ?? true,
      watchers: body?.watchers,
    })

    log.info("Observer Network started via API", {
      mode: network.getMode(),
    })

    return jsonResponse(200, {
      success: true,
      mode: network.getMode(),
    })
  } catch (error) {
    log.error("Failed to start Observer Network", {
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
 */
export async function stopObserver(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const network = ObserverNetwork.getInstance()

  if (!network || !network.isRunning()) {
    return jsonResponse(200, {
      success: true,
      message: "Observer Network not running",
    })
  }

  try {
    await network.stop()

    log.info("Observer Network stopped via API")

    return jsonResponse(200, {
      success: true,
    })
  } catch (error) {
    log.error("Failed to stop Observer Network", {
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
 */
export async function switchObserverMode(
  req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const body = await parseJsonBody<{ mode: OperatingMode; reason?: string }>(req)

  if (!body || !body.mode) {
    return jsonResponse(400, { success: false, error: "Missing mode in request body" })
  }

  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(503, { success: false, error: "Observer Network not running" })
  }

  const validModes: OperatingMode[] = ["AUTO", "MANUAL", "HYBRID"]
  if (!validModes.includes(body.mode)) {
    return jsonResponse(400, {
      success: false,
      error: `Invalid mode: ${body.mode}. Must be one of: ${validModes.join(", ")}`,
    })
  }

  try {
    await network.switchMode(body.mode, body.reason ?? "API request")

    log.info("Observer mode switched via API", {
      mode: body.mode,
      reason: body.reason,
    })

    return jsonResponse(200, {
      success: true,
      mode: body.mode,
    })
  } catch (error) {
    log.error("Failed to switch Observer mode", {
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
 */
export async function getWorldModel(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(503, { success: false, error: "Observer Network not running" })
  }

  const worldModel = await network.getWorldModel()

  if (!worldModel) {
    return jsonResponse(200, {
      success: true,
      worldModel: null,
    })
  }

  return jsonResponse(200, {
    success: true,
    worldModel,
  })
}

/**
 * GET /api/v1/observer/snapshot
 * Get the consensus snapshot.
 */
export async function getConsensusSnapshot(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(503, { success: false, error: "Observer Network not running" })
  }

  const snapshot = network.getSnapshot()

  return jsonResponse(200, {
    success: true,
    snapshot,
  })
}

/**
 * GET /api/v1/observer/opportunities
 * Get active opportunities.
 */
export async function getOpportunities(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const network = ObserverNetwork.getInstance()

  if (!network) {
    return jsonResponse(503, { success: false, error: "Observer Network not running" })
  }

  const opportunities = network.getOpportunities()

  return jsonResponse(200, {
    success: true,
    opportunities,
    count: opportunities.length,
  })
}

/**
 * GET /api/v1/observer/health
 * Health check for Observer API.
 */
export async function observerHealth(
  _req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const network = ObserverNetwork.getInstance()

  return jsonResponse(200, {
    healthy: true,
    observerRunning: network?.isRunning() ?? false,
    mode: network?.getMode() ?? null,
    timestamp: new Date().toISOString(),
  })
}
