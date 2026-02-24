import z from "zod"

export type DecisionType =
  | "architecture"
  | "implementation"
  | "refactor"
  | "bugfix"
  | "feature"
  | "test"
  | "rollback"
  | "checkpoint"
  | "resource"
  | "other"

export type RiskLevel = "low" | "medium" | "high" | "critical"

export interface CLOSEScore {
  convergence: number
  leverage: number
  optionality: number
  surplus: number
  evolution: number
  total: number
}

export interface AutonomousDecisionCriteria {
  type: DecisionType
  description: string
  riskLevel: RiskLevel
  convergence: number
  leverage: number
  optionality: number
  surplus: number
  evolution: number
  alternatives?: string[]
  resources?: {
    estimatedTokens: number
    estimatedCostUSD: number
    estimatedDurationMinutes: number
  }
  impacts?: {
    files?: string[]
    tests?: boolean
    documentation?: boolean
  }
  metadata?: Record<string, unknown>
}

export const AutonomousDecisionCriteriaSchema = z.object({
  type: z.enum([
    "architecture",
    "implementation",
    "refactor",
    "bugfix",
    "feature",
    "test",
    "rollback",
    "checkpoint",
    "resource",
    "other",
  ]),
  description: z.string(),
  riskLevel: z.enum(["low", "medium", "high", "critical"]),
  convergence: z.number().min(0).max(10),
  leverage: z.number().min(0).max(10),
  optionality: z.number().min(0).max(10),
  surplus: z.number().min(0).max(10),
  evolution: z.number().min(0).max(10),
  alternatives: z.array(z.string()).optional(),
  resources: z
    .object({
      estimatedTokens: z.number().min(0),
      estimatedCostUSD: z.number().min(0),
      estimatedDurationMinutes: z.number().min(0),
    })
    .optional(),
  impacts: z
    .object({
      files: z.array(z.string()).optional(),
      tests: z.boolean().optional(),
      documentation: z.boolean().optional(),
    })
    .optional(),
  metadata: z.record(z.string(), z.any()).optional(),
})

export interface DecisionRecord {
  id: string
  type: DecisionType
  description: string
  context: string
  score: CLOSEScore
  result: "proceed" | "proceed_with_caution" | "pause" | "block" | "skip"
  reasoning: string
  timestamp: number
  sessionId: string
  criteria: AutonomousDecisionCriteria
}

export const DecisionTemplates = {
  lowRiskImplementation: (description: string): Partial<AutonomousDecisionCriteria> => ({
    type: "implementation",
    description,
    riskLevel: "low",
    convergence: 3,
    leverage: 7,
    optionality: 8,
    surplus: 7,
    evolution: 5,
  }),

  highRiskArchitecture: (description: string): Partial<AutonomousDecisionCriteria> => ({
    type: "architecture",
    description,
    riskLevel: "high",
    convergence: 8,
    leverage: 6,
    optionality: 3,
    surplus: 4,
    evolution: 7,
  }),

  testWriting: (description: string): Partial<AutonomousDecisionCriteria> => ({
    type: "test",
    description,
    riskLevel: "low",
    convergence: 2,
    leverage: 8,
    optionality: 9,
    surplus: 8,
    evolution: 6,
  }),

  rollback: (reason: string): Partial<AutonomousDecisionCriteria> => ({
    type: "rollback",
    description: `Rollback: ${reason}`,
    riskLevel: "medium",
    convergence: 5,
    leverage: 7,
    optionality: 10,
    surplus: 9,
    evolution: 4,
  }),

  checkpoint: (description: string): Partial<AutonomousDecisionCriteria> => ({
    type: "checkpoint",
    description,
    riskLevel: "low",
    convergence: 1,
    leverage: 6,
    optionality: 10,
    surplus: 9,
    evolution: 3,
  }),

  resourceLimit: (resource: string, current: number, limit: number): Partial<AutonomousDecisionCriteria> => ({
    type: "resource",
    description: `Resource limit approaching: ${resource} (${current}/${limit})`,
    riskLevel: "medium",
    convergence: 2,
    leverage: 5,
    optionality: 7,
    surplus: 3,
    evolution: 4,
  }),
}

export function buildCriteria(template: Partial<AutonomousDecisionCriteria>): AutonomousDecisionCriteria {
  return {
    type: template.type ?? "other",
    description: template.description ?? "",
    riskLevel: template.riskLevel ?? "medium",
    convergence: template.convergence ?? 5,
    leverage: template.leverage ?? 5,
    optionality: template.optionality ?? 5,
    surplus: template.surplus ?? 5,
    evolution: template.evolution ?? 5,
    alternatives: template.alternatives,
    resources: template.resources,
    impacts: template.impacts,
    metadata: template.metadata,
  }
}

export function calculateCLOSEFromContext(input: {
  reversibility: "fully" | "partially" | "not"
  riskReward: number
  futureOptions: number
  resourceMargin: number
  learningValue: number
}): CLOSEScore {
  const convergence = input.reversibility === "fully" ? 2 : input.reversibility === "partially" ? 5 : 8

  const leverage = Math.min(10, input.riskReward * 2)

  const optionality = input.futureOptions

  const surplus = (input.resourceMargin / 100) * 10

  const evolution = input.learningValue

  const weights = { convergence: 1.0, leverage: 1.2, optionality: 1.5, surplus: 1.3, evolution: 0.8 }
  // maxScore is the maximum possible weighted sum (when all criteria are 10)
  const maxScore = 10 * (weights.convergence + weights.leverage + weights.optionality + weights.surplus + weights.evolution)
  const total =
    ((convergence * weights.convergence +
      leverage * weights.leverage +
      optionality * weights.optionality +
      surplus * weights.surplus +
      evolution * weights.evolution) /
      maxScore) *
    10

  return {
    convergence,
    leverage,
    optionality,
    surplus,
    evolution,
    total: Math.round(total * 100) / 100,
  }
}

export function validateCLOSEScore(score: CLOSEScore): boolean {
  return (
    score.convergence >= 0 &&
    score.convergence <= 10 &&
    score.leverage >= 0 &&
    score.leverage <= 10 &&
    score.optionality >= 0 &&
    score.optionality <= 10 &&
    score.surplus >= 0 &&
    score.surplus <= 10 &&
    score.evolution >= 0 &&
    score.evolution <= 10 &&
    score.total >= 0 &&
    score.total <= 10
  )
}
