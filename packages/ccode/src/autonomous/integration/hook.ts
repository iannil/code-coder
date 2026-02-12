/**
 * Autonomous Mode Hook Integration
 *
 * Integrates the CLOSE decision framework into the tool execution flow via PreToolUse hooks.
 * This module manages the lifecycle of DecisionEngine instances per session.
 *
 * @package autonomous/integration
 */

import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { DecisionEngine, type DecisionContext, type DecisionResult, type AutonomyLevel } from "../decision/engine"
import { AutonomousConfig } from "../config/config"
import { buildCriteria, type AutonomousDecisionCriteria, type RiskLevel, type DecisionType } from "../decision/criteria"

const log = Log.create({ service: "autonomous.integration.hook" })

const engineCache = new Map<string, DecisionEngine>()

const sessionTracking = new Map<
  string,
  {
    startTime: number
    errorCount: number
    recentDecisions: string[]
  }
>()

const TOOL_RISK_LEVELS: Record<string, RiskLevel> = {
  write: "low",
  edit: "low",
  read: "low",
  glob: "low",
  grep: "low",
  bash: "medium",
  mcp__browser__browser_navigate: "low",
  mcp__browser__browser_click: "low",
  mcp__browser__browser_type: "low",
  websearch: "low",
  webfetch: "low",
}

const DESTRUCTIVE_PATTERNS = [
  /\brm\s+-rf?\b/,
  /\bdel\s+\//,
  /\bdelete\s+.*\*/,
  /\btruncate\b/,
  /\bdrop\s+table\b/,
  /\bdrop\s+database\b/,
]

function isDestructiveOperation(toolName: string, toolInput: Record<string, unknown>): boolean {
  if (toolName === "bash" && typeof toolInput.command === "string") {
    const command = toolInput.command.toLowerCase()
    return DESTRUCTIVE_PATTERNS.some((pattern) => pattern.test(command))
  }
  return false
}

function getDecisionType(toolName: string, toolInput: Record<string, unknown>): DecisionType {
  if (toolName === "bash") {
    const command = typeof toolInput.command === "string" ? toolInput.command : ""
    if (command.includes("test") || command.includes("jest") || command.includes("bun test")) {
      return "test"
    }
    if (command.includes("git")) {
      return "checkpoint"
    }
    return "implementation"
  }
  if (toolName === "write" || toolName === "edit") {
    return "implementation"
  }
  if (toolName === "read" || toolName === "glob" || toolName === "grep") {
    return "other"
  }
  return "implementation"
}

function buildToolDecisionCriteria(
  toolName: string,
  toolInput: Record<string, unknown>,
): Partial<AutonomousDecisionCriteria> {
  const isDestructive = isDestructiveOperation(toolName, toolInput)
  const baseRiskLevel = TOOL_RISK_LEVELS[toolName] ?? "medium"
  const riskLevel: RiskLevel = isDestructive ? "high" : baseRiskLevel

  const description =
    `Execute tool: ${toolName}` +
    (typeof toolInput.command === "string" ? ` (${toolInput.command.slice(0, 50)}...)` : "")

  const type = getDecisionType(toolName, toolInput)

  const criteria: Partial<AutonomousDecisionCriteria> = {
    type,
    description,
    riskLevel,
    convergence: isDestructive ? 8 : 3,
    leverage: toolName.startsWith("read") || toolName === "grep" || toolName === "glob" ? 8 : 6,
    optionality: isDestructive ? 3 : 7,
    surplus: 7,
    evolution: type === "test" ? 7 : 5,
  }

  return criteria
}

export async function getEngine(sessionId: string): Promise<DecisionEngine> {
  const existing = engineCache.get(sessionId)
  if (existing) {
    return existing
  }

  if (!sessionTracking.has(sessionId)) {
    sessionTracking.set(sessionId, {
      startTime: Date.now(),
      errorCount: 0,
      recentDecisions: [],
    })
  }

  const config = await AutonomousConfig.get()

  const engine = new DecisionEngine({
    autonomyLevel: config.autonomyLevel as AutonomyLevel,
    approvalThreshold: config.decisionThreshold,
    cautionThreshold: config.decisionThreshold - 2,
    closeWeights: config.closeWeights,
  })

  engineCache.set(sessionId, engine)

  log.info("CLOSE Decision Engine created", { sessionId, autonomyLevel: config.autonomyLevel })

  await Bus.publish(AutonomousEvent.SessionStarted, {
    sessionId,
    requestId: sessionId,
    autonomyLevel: config.autonomyLevel,
    config: {
      decisionThreshold: config.decisionThreshold,
      riskTolerance: config.riskTolerance,
    },
  })

  return engine
}

export async function evaluateToolCall(ctx: {
  sessionId: string
  toolName: string
  toolInput: Record<string, unknown>
}): Promise<{
  allowed: boolean
  decision?: DecisionResult
}> {
  const { sessionId, toolName, toolInput } = ctx

  const engine = await getEngine(sessionId)

  const tracking = sessionTracking.get(sessionId)
  if (!tracking) {
    log.error("Session tracking not found", { sessionId })
    return { allowed: true }
  }

  const criteriaPartial = buildToolDecisionCriteria(toolName, toolInput)
  const criteria = buildCriteria(criteriaPartial)

  const decisionContext: DecisionContext = {
    sessionId,
    currentState: `Executing tool: ${toolName}`,
    resourceUsage: {
      tokensUsed: 0,
      costUSD: 0,
      durationMinutes: (Date.now() - tracking.startTime) / 60000,
    },
    errorCount: tracking.errorCount,
    recentDecisions: tracking.recentDecisions,
  }

  log.info("CLOSE Decision evaluating", {
    sessionId,
    tool: toolName,
    description: criteria.description,
    riskLevel: criteria.riskLevel,
  })

  const decision = await engine.evaluate(criteria, decisionContext)

  tracking.recentDecisions.push(`${toolName}:${decision.action}`)

  const score = decision.score
  log.info(`CLOSE Decision: ${decision.action.toUpperCase()}`, {
    sessionId,
    tool: toolName,
    score: score.total.toFixed(2),
    breakdown: {
      C: score.convergence.toFixed(1),
      L: score.leverage.toFixed(1),
      O: score.optionality.toFixed(1),
      S: score.surplus.toFixed(1),
      E: score.evolution.toFixed(1),
    },
    approved: decision.approved,
    reasoning: decision.reasoning.split("\n")[0],
  })

  return {
    allowed: decision.approved,
    decision,
  }
}

export function recordError(sessionId: string): void {
  const tracking = sessionTracking.get(sessionId)
  if (tracking) {
    tracking.errorCount++
  }
}

export async function cleanup(sessionId: string): Promise<void> {
  log.info("Cleaning up Autonomous Mode session", { sessionId })

  engineCache.delete(sessionId)

  sessionTracking.delete(sessionId)

  await Bus.publish(AutonomousEvent.SessionCompleted, {
    sessionId,
    requestId: sessionId,
    result: {
      success: true,
      qualityScore: 0,
      crazinessScore: 0,
      duration: 0,
      tokensUsed: 0,
      costUSD: 0,
    },
  })
}

export function getSessionStats(sessionId: string): {
  errorCount: number
  recentDecisions: string[]
  durationMinutes: number
} | null {
  const tracking = sessionTracking.get(sessionId)
  if (!tracking) return null

  return {
    errorCount: tracking.errorCount,
    recentDecisions: [...tracking.recentDecisions],
    durationMinutes: (Date.now() - tracking.startTime) / 60000,
  }
}

export function hasActiveSession(sessionId: string): boolean {
  return engineCache.has(sessionId)
}

export function clearAllSessions(): void {
  engineCache.clear()
  sessionTracking.clear()
}

export type { DecisionContext, DecisionResult, AutonomyLevel } from "../decision/engine"
export type { AutonomousDecisionCriteria, CLOSEScore, DecisionType, RiskLevel } from "../decision/criteria"

export const AutonomousModeHook = {
  getEngine,
  evaluateToolCall,
  recordError,
  cleanup,
  getSessionStats,
  hasActiveSession,
  clearAllSessions,
} as const
