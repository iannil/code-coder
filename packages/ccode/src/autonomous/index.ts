/**
 * Autonomous System Type Stubs
 * @deprecated Autonomous functionality is being migrated to Rust.
 */

import { z } from "zod"
import { BusEvent } from "@/bus/bus-event"

// Resource budget configuration
export interface ResourceBudget {
  maxTokens: number
  maxCostUSD: number
  maxDurationMinutes: number
  maxFilesChanged: number
  maxActions: number
}

// Session context for autonomous operations
export interface SessionContext {
  sessionId: string
  requestId: string
  request: string
  startTime: number
}

// Orchestrator configuration
export interface OrchestratorConfig {
  mode: "code" | "research" | "decision" | "auto"
  autonomyLevel: string
  resourceBudget: ResourceBudget
  unattended: boolean
}

// Orchestrator result
export interface OrchestratorResult {
  success: boolean
  result?: {
    success: boolean
    mode?: "code" | "research" | "decision"
    qualityScore?: number
    crazinessScore?: number
    iterationsCompleted?: number
    topic?: string
    report?: string
    sources?: Array<{ url: string; title: string }>
    insights?: string[]
    research?: string
    closeScore?: {
      convergence: number
      leverage: number
      optionality: number
      surplus: number
      evolution: number
      overall: number
    }
    recommendation?: string
    alternatives?: string[]
    duration: number
    tokensUsed: number
    costUSD: number
  }
}

// Orchestrator interface
export interface Orchestrator {
  start(request: string): Promise<void>
  process(request: string): Promise<OrchestratorResult>
  getState(): string
}

// Parse resource budget from string
export function parseResourceBudget(budget: string): ResourceBudget {
  const result: ResourceBudget = {
    maxTokens: 500000,
    maxCostUSD: 5.0,
    maxDurationMinutes: 60,
    maxFilesChanged: 50,
    maxActions: 1000,
  }

  const parts = budget.split(",")
  for (const part of parts) {
    const [key, value] = part.split(":")
    if (key === "tokens") result.maxTokens = parseInt(value, 10)
    else if (key === "cost") result.maxCostUSD = parseFloat(value)
    else if (key === "time") result.maxDurationMinutes = parseInt(value, 10) / 60
  }

  return result
}

// Create orchestrator stub
export function createOrchestrator(_context: SessionContext, _config: OrchestratorConfig): Orchestrator {
  console.warn("Autonomous orchestrator is deprecated. Use Rust implementation.")
  return {
    async start(_request: string): Promise<void> {
      throw new Error("Deprecated: use Rust autonomous implementation")
    },
    async process(_request: string): Promise<OrchestratorResult> {
      throw new Error("Deprecated: use Rust autonomous implementation")
    },
    getState(): string {
      return "DEPRECATED"
    },
  }
}

// Autonomous event definitions using BusEvent.define
export const AutonomousEvent = {
  StateChanged: BusEvent.define(
    "autonomous:state:changed",
    z.object({
      sessionId: z.string().optional(),
      from: z.string().optional(),
      to: z.string().optional(),
    }),
  ),
  IterationStarted: BusEvent.define(
    "autonomous:iteration:started",
    z.object({
      sessionId: z.string(),
      iteration: z.number(),
    }),
  ),
  TaskStarted: BusEvent.define(
    "autonomous:task:started",
    z.object({
      sessionId: z.string(),
      taskId: z.string(),
    }),
  ),
  TaskCompleted: BusEvent.define(
    "autonomous:task:completed",
    z.object({
      sessionId: z.string(),
      taskId: z.string(),
      success: z.boolean(),
    }),
  ),
  DecisionMade: BusEvent.define(
    "autonomous:decision:made",
    z.object({
      sessionId: z.string(),
      type: z.string(),
      approved: z.boolean(),
    }),
  ),
  ResourceWarning: BusEvent.define(
    "autonomous:resource:warning",
    z.object({
      sessionId: z.string(),
      resource: z.string(),
      percentage: z.number(),
    }),
  ),
  SessionStarted: BusEvent.define(
    "autonomous:session:started",
    z.object({
      sessionId: z.string(),
      request: z.string().optional(),
      mode: z.string().optional(),
      autonomyLevel: z.string().optional(),
    }),
  ),
  SessionCompleted: BusEvent.define(
    "autonomous:session:completed",
    z.object({
      sessionId: z.string(),
      success: z.boolean(),
      duration: z.number().optional(),
    }),
  ),
  SessionFailed: BusEvent.define(
    "autonomous:session:failed",
    z.object({
      sessionId: z.string(),
      error: z.string(),
    }),
  ),
  MetricsUpdated: BusEvent.define(
    "autonomous:metrics:updated",
    z.object({
      sessionId: z.string(),
      tokensUsed: z.number().optional(),
      costUSD: z.number().optional(),
      filesChanged: z.number().optional(),
      metrics: z
        .object({
          tokensUsed: z.number().optional(),
          costUSD: z.number().optional(),
          qualityScore: z.number().optional(),
          crazinessScore: z.number().optional(),
          tasksCompleted: z.number().optional(),
          tasksTotal: z.number().optional(),
        })
        .optional(),
    }),
  ),
  SafetyTriggered: BusEvent.define(
    "autonomous:safety:triggered",
    z.object({
      sessionId: z.string(),
      trigger: z.string(),
      action: z.string().optional(),
      severity: z.enum(["low", "medium", "high", "critical"]).optional(),
    }),
  ),
  KnowledgeConsolidated: BusEvent.define(
    "autonomous:knowledge:consolidated",
    z.object({
      sessionId: z.string().optional(),
      topic: z.string().optional(),
      entries: z.number().optional(),
    }),
  ),
} as const

export type AutonomousEventType = keyof typeof AutonomousEvent

// Event helper
export const AutonomousEventHelper = {
  isStateChanged: (type: string) => type === AutonomousEvent.StateChanged.type,
  isIterationStarted: (type: string) => type === AutonomousEvent.IterationStarted.type,
  isTaskEvent: (type: string) =>
    type === AutonomousEvent.TaskStarted.type || type === AutonomousEvent.TaskCompleted.type,
}
