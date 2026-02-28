/**
 * Autonomous Mode API Handler
 *
 * Provides HTTP endpoints for integrating with the Autonomous Orchestrator.
 * Designed for Hands system (zero-workflow) to invoke autonomous decision-making.
 *
 * POST /api/v1/autonomous/execute - Execute a task with autonomous capabilities
 * GET  /api/v1/autonomous/health  - Health check
 *
 * @package autonomous
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import {
  Permission,
  createAutoApproveHandler,
  parseRiskLevel,
  type RiskLevel,
  type AutoApproveConfig,
} from "../../../permission"

// ============================================================================
// Types
// ============================================================================

/**
 * Autonomy levels matching the CLOSE decision framework thresholds
 */
export type AutonomyLevel = "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"

/**
 * Resource budget for autonomous execution
 */
export interface ResourceBudget {
  /** Maximum tokens to consume */
  maxTokens: number
  /** Maximum cost in USD */
  maxCostUSD: number
  /** Maximum duration in seconds */
  maxDurationSec: number
}

/**
 * Autonomous execution configuration
 */
export interface AutonomousConfig {
  /** Autonomy level (affects CLOSE thresholds) */
  autonomyLevel: AutonomyLevel
  /** Enable unattended mode (no human interaction) */
  unattended: boolean
  /** Resource budget */
  resourceBudget: ResourceBudget
  /** Enable evolution loop for problem solving */
  enableEvolutionLoop?: boolean
  /** Enable web search for solutions */
  enableWebSearch?: boolean
  /** Maximum iterations */
  maxIterations?: number
}

/**
 * Auto-approve configuration from Hands
 */
export interface AutoApproveConfigInput {
  /** Enable auto-approval */
  enabled: boolean
  /** Tools allowed for auto-approval (whitelist) */
  allowedTools: string[]
  /** Maximum risk level for auto-approval */
  riskThreshold: string
  /** Timeout in milliseconds before auto-approving */
  timeoutMs: number
  /** Whether running in unattended mode */
  unattended: boolean
}

/**
 * Context from Hands system
 */
export interface HandsContext {
  /** Hand ID invoking autonomous */
  handId: string
  /** Hand name */
  handName?: string
  /** Previous execution results for context */
  previousResults?: Array<{
    timestamp: string
    output: string
    success: boolean
  }>
  /** Custom state from previous runs */
  customState?: Record<string, unknown>
}

/**
 * Request to execute autonomous task
 */
export interface AutonomousExecuteRequest {
  /** Task request/description */
  request: string
  /** Autonomous configuration */
  config: AutonomousConfig
  /** Optional context from Hands */
  context?: HandsContext
  /** Optional session ID (for resuming) */
  sessionId?: string
  /** Optional auto-approve configuration */
  autoApproveConfig?: AutoApproveConfigInput
}

/**
 * CLOSE decision score result
 */
export interface CLOSEScoreResult {
  /** Convergence: How focused the decision is (0-10) */
  convergence: number
  /** Leverage: Impact vs effort ratio (0-10) */
  leverage: number
  /** Optionality: Flexibility and reversibility (0-10) */
  optionality: number
  /** Surplus: Resource availability (0-10) */
  surplus: number
  /** Evolution: Learning value (0-10) */
  evolution: number
  /** Total weighted score (0-10) */
  total: number
}

/**
 * Autonomous execution result
 */
export interface AutonomousExecutionResult {
  /** Whether execution completed successfully */
  success: boolean
  /** Output content */
  output: string
  /** Quality score (0-100) */
  qualityScore: number
  /** Craziness score (0-100) */
  crazinessScore: number
  /** Duration in milliseconds */
  duration: number
  /** Tokens used */
  tokensUsed: number
  /** Cost in USD */
  costUSD: number
  /** Iterations completed */
  iterationsCompleted: number
  /** CLOSE decision scores */
  closeScores?: CLOSEScoreResult[]
  /** Whether execution was paused (can be resumed) */
  paused: boolean
  /** Pause reason if paused */
  pauseReason?: string
  /** Error message if failed */
  error?: string
}

/**
 * Response from autonomous execute endpoint
 */
export interface AutonomousExecuteResponse {
  /** Request success */
  success: boolean
  /** Session ID for tracking/resuming */
  sessionId: string
  /** Execution result (if completed) */
  result?: AutonomousExecutionResult
  /** Error message if failed */
  error?: string
}

// ============================================================================
// Constants - Autonomy Thresholds
// ============================================================================

/**
 * CLOSE decision thresholds for each autonomy level
 * Lower values = more permissive (higher autonomy)
 */
const AUTONOMY_THRESHOLDS: Record<
  AutonomyLevel,
  { approval: number; caution: number; description: string }
> = {
  lunatic: {
    approval: 5.0,
    caution: 3.0,
    description: "完全自主 - 无需人工干预",
  },
  insane: {
    approval: 5.5,
    caution: 3.5,
    description: "高度自主 - 关键决策前通知",
  },
  crazy: {
    approval: 6.0,
    caution: 4.0,
    description: "显著自主 - 半自动执行",
  },
  wild: {
    approval: 6.5,
    caution: 4.5,
    description: "部分自主 - 仅执行简单任务",
  },
  bold: {
    approval: 7.0,
    caution: 5.0,
    description: "谨慎自主 - 仅执行已定义步骤",
  },
  timid: {
    approval: 8.0,
    caution: 6.0,
    description: "基本不自主 - 仅收集信息",
  },
}

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) {
    throw new Error("Request body is empty")
  }
  return await new Response(body).text()
}

function generateSpanId(): string {
  return crypto.randomUUID().slice(0, 8)
}

function generateTraceId(): string {
  return crypto.randomUUID()
}

interface TracingContext {
  traceId: string
  spanId: string
  parentSpanId?: string
}

function extractTracingContext(req: HttpRequest): TracingContext {
  const headers = req.headers
  const traceId = headers.get("X-Trace-Id") ?? generateTraceId()
  const parentSpanId = headers.get("X-Span-Id") ?? undefined

  return {
    traceId,
    spanId: generateSpanId(),
    parentSpanId,
  }
}

interface LifecycleEvent {
  timestamp: string
  trace_id: string
  span_id: string
  parent_span_id?: string
  event_type: "function_start" | "function_end" | "error"
  service: string
  payload: Record<string, unknown>
}

function logLifecycleEvent(
  ctx: TracingContext,
  eventType: LifecycleEvent["event_type"],
  payload: Record<string, unknown>,
) {
  const event: LifecycleEvent = {
    timestamp: new Date().toISOString(),
    trace_id: ctx.traceId,
    span_id: ctx.spanId,
    parent_span_id: ctx.parentSpanId,
    event_type: eventType,
    service: "autonomous-api",
    payload,
  }
  console.log(JSON.stringify(event))
}

/**
 * Validate autonomy level
 */
function isValidAutonomyLevel(level: string): level is AutonomyLevel {
  return ["lunatic", "insane", "crazy", "wild", "bold", "timid"].includes(level)
}

/**
 * Get thresholds for autonomy level
 */
function getThresholds(level: AutonomyLevel) {
  return AUTONOMY_THRESHOLDS[level]
}

// ============================================================================
// Autonomous Executor (Lightweight Non-Session Mode)
// ============================================================================

/**
 * Execute autonomous task without full session management.
 * This is a simplified path for Hands integration.
 */
async function executeAutonomousTask(
  request: string,
  config: AutonomousConfig,
  context?: HandsContext,
): Promise<AutonomousExecutionResult> {
  const startTime = Date.now()

  // Import autonomous components dynamically
  const { buildCriteria } = await import("../../../autonomous/decision/criteria")
  const { DecisionEngine } = await import("../../../autonomous/decision/engine")

  // Create decision engine with autonomy level
  const decisionEngine = new DecisionEngine({
    autonomyLevel: config.autonomyLevel,
  })

  // Evaluate task using CLOSE framework
  const criteria = buildCriteria({
    type: "implementation",
    description: request,
    riskLevel: config.autonomyLevel === "lunatic" || config.autonomyLevel === "insane" ? "medium" : "low",
    convergence: 6,
    leverage: 7,
    optionality: 8,
    surplus: 7,
    evolution: 6,
  })

  const decisionContext = {
    sessionId: `hands-${context?.handId ?? "unknown"}-${Date.now()}`,
    currentState: "executing",
    resourceUsage: {
      tokensUsed: 0,
      costUSD: 0,
      durationMinutes: 0,
    },
    errorCount: 0,
    recentDecisions: [],
  }

  const decision = await decisionEngine.evaluate(criteria, decisionContext)

  // If decision not approved, return early
  if (!decision.approved) {
    return {
      success: false,
      output: "",
      qualityScore: 0,
      crazinessScore: 0,
      duration: Date.now() - startTime,
      tokensUsed: 0,
      costUSD: 0,
      iterationsCompleted: 0,
      closeScores: [
        {
          convergence: criteria.convergence,
          leverage: criteria.leverage,
          optionality: criteria.optionality,
          surplus: criteria.surplus,
          evolution: criteria.evolution,
          total: decision.score.total,
        },
      ],
      paused: true,
      pauseReason: `Decision blocked: ${decision.reasoning}`,
    }
  }

  // Import agent invocation
  const { Agent } = await import("../../../agent/agent")
  const { SessionPrompt } = await import("../../../session/prompt")

  // Find appropriate agent
  const agents = await Agent.list()
  let agentName = "general"

  // Try to determine agent from request or use general
  const lowerRequest = request.toLowerCase()
  if (lowerRequest.includes("macro") || lowerRequest.includes("economic")) {
    agentName = "macro"
  } else if (lowerRequest.includes("trading") || lowerRequest.includes("market")) {
    agentName = "trader"
  } else if (lowerRequest.includes("code") || lowerRequest.includes("implement")) {
    agentName = "code-reviewer"
  }

  const agentExists = agents.some((a) => a.name === agentName)
  if (!agentExists) {
    agentName = "general"
  }

  // Create a temporary session for this execution
  const { LocalSession } = await import("../../../api")
  const session = await LocalSession.create({
    title: `Hands: ${context?.handName ?? context?.handId ?? "Autonomous Task"}`,
  })

  // Build prompt with context
  let prompt = request
  if (context?.previousResults && context.previousResults.length > 0) {
    const lastResult = context.previousResults[context.previousResults.length - 1]
    prompt += `\n\n**Previous output (${lastResult.timestamp}):**\n${lastResult.output.slice(0, 500)}`
  }

  if (context?.customState) {
    prompt += `\n\n**Custom state:**\n${JSON.stringify(context.customState, null, 2)}`
  }

  // Execute agent
  const result = await SessionPrompt.prompt({
    sessionID: session.id,
    agent: agentName,
    parts: [{ type: "text", text: prompt }],
  })

  // Extract response text
  let responseText = ""
  let tokensUsed = 0
  let costUSD = 0

  if (typeof result === "object" && "info" in result && "parts" in result) {
    const assistantMsg = result as {
      info: { role: string; tokens?: { input: number; output: number } }
      parts: Array<{ type: string; text?: string }>
    }
    responseText = assistantMsg.parts
      .filter((p) => p.type === "text" && p.text)
      .map((p) => p.text)
      .join("\n\n")

    if (assistantMsg.info.tokens) {
      tokensUsed = assistantMsg.info.tokens.input + assistantMsg.info.tokens.output
      // Rough cost estimation (Claude Opus pricing)
      costUSD = (assistantMsg.info.tokens.input * 0.003 + assistantMsg.info.tokens.output * 0.015) / 1000
    }
  } else if (typeof result === "string") {
    const { MessageV2 } = await import("../../../session/message-v2")
    const parts = await MessageV2.parts(result)
    responseText = parts
      .map((p) => ("text" in p ? p.text : ""))
      .filter(Boolean)
      .join("\n\n")
  }

  const duration = Date.now() - startTime

  // Calculate scores
  const qualityScore = decision.approved ? 85 : 50
  const autonomyScores = { lunatic: 95, insane: 85, crazy: 75, wild: 60, bold: 40, timid: 15 }
  const crazinessScore = autonomyScores[config.autonomyLevel]

  return {
    success: true,
    output: responseText,
    qualityScore,
    crazinessScore,
    duration,
    tokensUsed,
    costUSD,
    iterationsCompleted: 1,
    closeScores: [
      {
        convergence: criteria.convergence,
        leverage: criteria.leverage,
        optionality: criteria.optionality,
        surplus: criteria.surplus,
        evolution: criteria.evolution,
        total: decision.score.total,
      },
    ],
    paused: false,
  }
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * POST /api/v1/autonomous/execute
 *
 * Execute a task with autonomous decision-making capabilities.
 * This endpoint:
 * 1. Sets up auto-approve handler if configured
 * 2. Evaluates the task using CLOSE decision framework
 * 3. Invokes appropriate agent based on autonomy level
 * 4. Returns result with quality and craziness scores
 *
 * Designed for Hands system integration but usable by any client.
 */
export async function executeAutonomous(
  req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const startTime = performance.now()
  const ctx = extractTracingContext(req)
  let previousHandler: Permission.AskCallback | undefined

  logLifecycleEvent(ctx, "function_start", {
    function: "executeAutonomous",
    method: req.method,
    url: req.url.pathname,
  })

  try {
    const body = await readRequestBody(req.body)
    const input = JSON.parse(body) as AutonomousExecuteRequest

    // Validate required fields
    if (!input.request) {
      logLifecycleEvent(ctx, "error", { function: "executeAutonomous", error: "request is required" })
      return errorResponse("request is required", 400)
    }

    if (!input.config) {
      logLifecycleEvent(ctx, "error", { function: "executeAutonomous", error: "config is required" })
      return errorResponse("config is required", 400)
    }

    const { autonomyLevel, unattended, resourceBudget } = input.config

    // Validate autonomy level
    if (!autonomyLevel || !isValidAutonomyLevel(autonomyLevel)) {
      return errorResponse(
        `Invalid autonomyLevel: ${autonomyLevel}. Must be one of: lunatic, insane, crazy, wild, bold, timid`,
        400,
      )
    }

    // Validate resource budget
    if (!resourceBudget || resourceBudget.maxTokens <= 0 || resourceBudget.maxCostUSD <= 0) {
      return errorResponse("Invalid resourceBudget: maxTokens and maxCostUSD must be positive", 400)
    }

    const thresholds = getThresholds(autonomyLevel)

    // Set up auto-approve handler if configured
    if (input.autoApproveConfig?.enabled) {
      const autoApproveConfig: AutoApproveConfig = {
        enabled: true,
        allowedTools: input.autoApproveConfig.allowedTools ?? [],
        riskThreshold: parseRiskLevel(input.autoApproveConfig.riskThreshold ?? "medium"),
        timeoutMs: input.autoApproveConfig.timeoutMs ?? 30000,
        unattended: input.autoApproveConfig.unattended ?? unattended,
      }

      logLifecycleEvent(ctx, "function_start", {
        function: "setupAutoApprove",
        config: autoApproveConfig,
      })

      // Set the auto-approve handler
      Permission.setHandler(createAutoApproveHandler(autoApproveConfig))
    }

    // Execute autonomous task
    const result = await executeAutonomousTask(input.request, input.config, input.context)

    const sessionId = input.sessionId ?? `hands-${input.context?.handId ?? "auto"}-${Date.now()}`

    const durationMs = Math.round(performance.now() - startTime)

    logLifecycleEvent(ctx, "function_end", {
      function: "executeAutonomous",
      duration_ms: durationMs,
      success: result.success,
      qualityScore: result.qualityScore,
      crazinessScore: result.crazinessScore,
      tokensUsed: result.tokensUsed,
      costUSD: result.costUSD,
      paused: result.paused,
      autoApproveEnabled: input.autoApproveConfig?.enabled ?? false,
    })

    const response: AutonomousExecuteResponse = {
      success: true,
      sessionId,
      result,
    }

    return jsonResponse(response)
  } catch (error) {
    const durationMs = Math.round(performance.now() - startTime)

    logLifecycleEvent(ctx, "error", {
      function: "executeAutonomous",
      duration_ms: durationMs,
      error: error instanceof Error ? error.message : String(error),
    })

    console.error("Autonomous API error:", error)

    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/autonomous/health
 *
 * Health check endpoint for the autonomous service.
 */
export async function autonomousHealth(
  req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "autonomousHealth" })

  const response = jsonResponse({
    success: true,
    data: {
      status: "healthy",
      timestamp: new Date().toISOString(),
      autonomyLevels: Object.keys(AUTONOMY_THRESHOLDS),
    },
  })

  logLifecycleEvent(ctx, "function_end", { function: "autonomousHealth", success: true })

  return response
}

/**
 * GET /api/v1/autonomous/thresholds
 *
 * Get CLOSE decision thresholds for each autonomy level.
 */
export async function getThresholdsInfo(
  req: HttpRequest,
  _params: RouteParams,
): Promise<HttpResponse> {
  const ctx = extractTracingContext(req)

  logLifecycleEvent(ctx, "function_start", { function: "getThresholdsInfo" })

  const response = jsonResponse({
    success: true,
    data: {
      thresholds: AUTONOMY_THRESHOLDS,
    },
  })

  logLifecycleEvent(ctx, "function_end", { function: "getThresholdsInfo", success: true })

  return response
}
