import { Log } from "@/util/log"
import type { AutonomousDecisionCriteria, CLOSEScore, DecisionRecord, DecisionType } from "./criteria"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import {
  evaluateClose as nativeEvaluateClose,
  type NapiCLOSEInput,
  type NapiCLOSEWeights,
  type NapiCLOSEEvaluation,
} from "@codecoder-ai/core"

const log = Log.create({ service: "autonomous.decision.engine" })

export type AutonomyLevel = "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"

export interface DecisionEngineConfig {
  approvalThreshold: number
  cautionThreshold: number
  closeWeights: {
    convergence: number
    leverage: number
    optionality: number
    surplus: number
    evolution: number
  }
  autonomyLevel: AutonomyLevel
}

const DEFAULT_CLOSE_WEIGHTS = {
  convergence: 1.0,
  leverage: 1.2,
  optionality: 1.5,
  surplus: 1.3,
  evolution: 0.8,
}

const AUTONOMY_THRESHOLDS: Record<AutonomyLevel, { approval: number; caution: number }> = {
  lunatic: { approval: 5.0, caution: 3.0 },
  insane: { approval: 5.5, caution: 3.5 },
  crazy: { approval: 6.0, caution: 4.0 },
  wild: { approval: 6.5, caution: 4.5 },
  bold: { approval: 7.0, caution: 5.0 },
  timid: { approval: 8.0, caution: 6.0 },
}

export interface DecisionContext {
  sessionId: string
  currentState: string
  resourceUsage?: {
    tokensUsed: number
    costUSD: number
    durationMinutes: number
  }
  errorCount: number
  recentDecisions: string[]
}

export interface DecisionResult {
  approved: boolean
  action: "proceed" | "proceed_with_caution" | "pause" | "block" | "skip"
  score: CLOSEScore
  reasoning: string
  confidence: number
}

export class DecisionEngine {
  private config: DecisionEngineConfig
  private decisionHistory: Map<string, DecisionRecord> = new Map()

  constructor(config: Partial<DecisionEngineConfig> = {}) {
    const autonomyLevel = config.autonomyLevel ?? "crazy"
    const thresholds = AUTONOMY_THRESHOLDS[autonomyLevel]

    this.config = {
      autonomyLevel,
      approvalThreshold: config.approvalThreshold ?? thresholds.approval,
      cautionThreshold: config.cautionThreshold ?? thresholds.caution,
      closeWeights: { ...DEFAULT_CLOSE_WEIGHTS, ...config.closeWeights },
    }
  }

  async evaluate(criteria: AutonomousDecisionCriteria, context: DecisionContext): Promise<DecisionResult> {
    const score = this.calculateCLOSE(criteria)

    log.info("Evaluating decision", {
      description: criteria.description,
      score: score.total,
      threshold: this.config.approvalThreshold,
    })

    const result = this.makeDecision(score, criteria, context)

    const record: DecisionRecord = {
      id: this.generateDecisionId(),
      type: criteria.type,
      description: criteria.description,
      context: context.currentState,
      score,
      result: result.action,
      reasoning: result.reasoning,
      timestamp: Date.now(),
      sessionId: context.sessionId,
      criteria,
    }
    this.decisionHistory.set(record.id, record)

    await Bus.publish(AutonomousEvent.DecisionMade, {
      sessionId: context.sessionId,
      decisionId: record.id,
      type: criteria.type,
      description: criteria.description,
      score: score.total,
      approved: result.approved,
      closeScores: score,
    })

    return result
  }

  private calculateCLOSE(criteria: AutonomousDecisionCriteria): CLOSEScore {
    const { closeWeights } = this.config

    const convergence = criteria.convergence * closeWeights.convergence
    const leverage = criteria.leverage * closeWeights.leverage
    const optionality = criteria.optionality * closeWeights.optionality
    const surplus = criteria.surplus * closeWeights.surplus
    const evolution = criteria.evolution * closeWeights.evolution

    // maxScore is the maximum possible weighted sum (when all criteria are 10)
    const maxScore =
      10 *
      (closeWeights.convergence +
        closeWeights.leverage +
        closeWeights.optionality +
        closeWeights.surplus +
        closeWeights.evolution)
    // Normalize to 0-10 range
    const total = ((convergence + leverage + optionality + surplus + evolution) / maxScore) * 10

    return {
      convergence: criteria.convergence,
      leverage: criteria.leverage,
      optionality: criteria.optionality,
      surplus: criteria.surplus,
      evolution: criteria.evolution,
      total: Math.round(total * 100) / 100,
    }
  }

  private makeDecision(
    score: CLOSEScore,
    criteria: AutonomousDecisionCriteria,
    context: DecisionContext,
  ): DecisionResult {
    const { approvalThreshold, cautionThreshold } = this.config

    if (score.total >= approvalThreshold) {
      return {
        approved: true,
        action: "proceed",
        score,
        reasoning: this.buildReasoning("proceed", score),
        confidence: this.calculateConfidence(score, approvalThreshold),
      }
    }

    if (score.total >= cautionThreshold) {
      return {
        approved: true,
        action: "proceed_with_caution",
        score,
        reasoning: this.buildReasoning("proceed_with_caution", score),
        confidence: this.calculateConfidence(score, approvalThreshold),
      }
    }

    if (criteria.riskLevel === "low" && context.errorCount < 3) {
      return {
        approved: true,
        action: "proceed_with_caution",
        score,
        reasoning: this.buildReasoning("proceed_with_caution", score),
        confidence: this.calculateConfidence(score, approvalThreshold),
      }
    }

    const errorCount = context?.errorCount ?? 0
    if (criteria.riskLevel === "high" || errorCount >= 5) {
      return {
        approved: false,
        action: "pause",
        score,
        reasoning: this.buildReasoning("pause", score, context ?? {}),
        confidence: this.calculateConfidence(score, approvalThreshold),
      }
    }

    if (criteria.riskLevel === "medium") {
      return {
        approved: false,
        action: this.config.autonomyLevel === "timid" ? "block" : "pause",
        score,
        reasoning: this.buildReasoning("pause", score, context ?? {}),
        confidence: this.calculateConfidence(score, approvalThreshold),
      }
    }

    return {
      approved: false,
      action: "skip",
      score,
      reasoning: this.buildReasoning("skip", score),
      confidence: this.calculateConfidence(score, approvalThreshold),
    }
  }

  private calculateConfidence(score: CLOSEScore, threshold: number): number {
    const normalized = Math.max(0, Math.min(10, score.total))
    const confidence = (normalized / 10) * 100
    return Math.round(confidence)
  }

  private buildReasoning(
    action: "proceed" | "proceed_with_caution" | "pause" | "block" | "skip",
    score: CLOSEScore,
    context?: DecisionContext,
  ): string {
    const parts: string[] = []

    parts.push(`CLOSE Score: ${score.total.toFixed(2)}/10`)
    parts.push(`  Convergence: ${score.convergence.toFixed(1)}/10`)
    parts.push(`  Leverage: ${score.leverage.toFixed(1)}/10`)
    parts.push(`  Optionality: ${score.optionality.toFixed(1)}/10`)
    parts.push(`  Surplus: ${score.surplus.toFixed(1)}/10`)
    parts.push(`  Evolution: ${score.evolution.toFixed(1)}/10`)

    switch (action) {
      case "proceed":
        parts.push("\nDecision: PROCEED - High confidence, all dimensions favorable")
        break
      case "proceed_with_caution":
        parts.push("\nDecision: PROCEED WITH CAUTION - Moderate score, increase monitoring")
        break
      case "pause":
        parts.push("\nDecision: PAUSE - Score below threshold, awaiting review")
        if (context && context.errorCount > 0) {
          parts.push(`  Reason: ${context.errorCount} recent errors`)
        }
        break
      case "block":
        parts.push("\nDecision: BLOCK - High risk operation requires approval")
        break
      case "skip":
        parts.push("\nDecision: SKIP - Insufficient confidence, skipping this action")
        break
    }

    return parts.join("\n")
  }

  private generateDecisionId(): string {
    return `decision_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`
  }

  getHistory(): DecisionRecord[] {
    return Array.from(this.decisionHistory.values()).sort((a, b) => b.timestamp - a.timestamp)
  }

  getDecision(id: string): DecisionRecord | undefined {
    return this.decisionHistory.get(id)
  }

  getSessionDecisions(sessionId: string): DecisionRecord[] {
    return this.getHistory().filter((d) => d.sessionId === sessionId)
  }

  getDecisionsByType(type: DecisionType, limit = 10): DecisionRecord[] {
    return this.getHistory()
      .filter((d) => d.type === type)
      .slice(0, limit)
  }

  analyzePatterns(): {
    averageScore: number
    approvalRate: number
    mostCommonType: DecisionType | undefined
    recentTrend: "improving" | "declining" | "stable"
  } {
    const history = this.getHistory()
    if (history.length === 0) {
      return {
        averageScore: 0,
        approvalRate: 0,
        mostCommonType: undefined,
        recentTrend: "stable",
      }
    }

    const totalScore = history.reduce((sum, d) => sum + d.score.total, 0)
    const averageScore = totalScore / history.length

    const approved = history.filter((d) => d.result === "proceed" || d.result === "proceed_with_caution").length
    const approvalRate = approved / history.length

    const typeCounts = new Map<DecisionType, number>()
    for (const decision of history) {
      typeCounts.set(decision.type, (typeCounts.get(decision.type) ?? 0) + 1)
    }
    const mostCommonType = Array.from(typeCounts.entries()).sort((a, b) => b[1] - a[1])[0]?.[0]

    const recent = history.slice(0, Math.min(10, history.length))
    const older = history.slice(recent.length, recent.length * 2)
    const recentAvg = recent.reduce((sum, d) => sum + d.score.total, 0) / recent.length
    const olderAvg = older.length > 0 ? older.reduce((sum, d) => sum + d.score.total, 0) / older.length : recentAvg

    let recentTrend: "improving" | "declining" | "stable" = "stable"
    if (recentAvg > olderAvg + 0.5) recentTrend = "improving"
    else if (recentAvg < olderAvg - 0.5) recentTrend = "declining"

    return {
      averageScore: Math.round(averageScore * 100) / 100,
      approvalRate: Math.round(approvalRate * 100) / 100,
      mostCommonType,
      recentTrend,
    }
  }

  clearHistory(): void {
    this.decisionHistory.clear()
  }

  updateConfig(config: Partial<DecisionEngineConfig>): void {
    if (config.approvalThreshold) this.config.approvalThreshold = config.approvalThreshold
    if (config.cautionThreshold) this.config.cautionThreshold = config.cautionThreshold
    if (config.closeWeights) this.config.closeWeights = { ...this.config.closeWeights, ...config.closeWeights }
    if (config.autonomyLevel) {
      this.config.autonomyLevel = config.autonomyLevel
      const thresholds = AUTONOMY_THRESHOLDS[config.autonomyLevel]
      this.config.approvalThreshold = thresholds.approval
      this.config.cautionThreshold = thresholds.caution
    }
  }

  getConfig(): DecisionEngineConfig {
    return { ...this.config }
  }
}

export function createDecisionEngine(config?: Partial<DecisionEngineConfig>): DecisionEngine {
  return new DecisionEngine(config)
}

export async function evaluateDecision(
  criteria: AutonomousDecisionCriteria,
  context: DecisionContext,
  config?: Partial<DecisionEngineConfig>,
): Promise<DecisionResult> {
  const engine = createDecisionEngine(config)
  return engine.evaluate(criteria, context)
}

// ============================================================================
// Native CLOSE Evaluation (Rust-backed)
// ============================================================================

/**
 * Native CLOSE evaluation input - matches Rust CLOSEInput
 */
export interface NativeCLOSEInput {
  // Convergence factors
  snapshotConfidence?: number
  buildStatus?: "green" | "yellow" | "red"
  sessionHealth?: "healthy" | "degraded" | "critical"
  criticalAnomalies?: number

  // Leverage factors
  strongPatterns?: number
  highImpactOpportunities?: number
  mediumImpactOpportunities?: number
  externalOpportunities?: number
  externalRisks?: number

  // Optionality factors
  totalOpportunities?: number
  patternTypes?: number
  anomalyCount?: number
  decisionQuality?: number

  // Surplus factors
  recentErrors?: number
  tokenUsage?: number
  cost?: number
  consensusStrength?: number
  coverageGaps?: number

  // Evolution factors
  learningOpportunities?: number
  recentChanges?: number
  techDebtLevel?: "low" | "medium" | "high"
  dismissedAnomalies?: number
  activeAnomalies?: number

  // Trend data (for historical comparison)
  recentTrendAvg?: number | null
  olderTrendAvg?: number | null
}

/**
 * Native CLOSE dimension result
 */
export interface NativeCLOSEDimension {
  score: number
  confidence: number
  factors: string[]
}

/**
 * Native CLOSE evaluation result - matches Rust CLOSEEvaluation
 */
export interface NativeCLOSEEvaluation {
  convergence: NativeCLOSEDimension
  leverage: NativeCLOSEDimension
  optionality: NativeCLOSEDimension
  surplus: NativeCLOSEDimension
  evolution: NativeCLOSEDimension
  total: number
  risk: number
  confidence: number
  recommendedGear: "P" | "N" | "D" | "S"
  timestamp: number
}

/**
 * Native CLOSE weights configuration
 */
export interface NativeCLOSEWeights {
  convergence?: number
  leverage?: number
  optionality?: number
  surplus?: number
  evolution?: number
}

/**
 * Evaluate CLOSE decision framework using native Rust implementation
 *
 * This provides a more comprehensive evaluation than the TypeScript DecisionEngine,
 * including gear recommendations based on the Gear System (P/N/D/S).
 *
 * @param input - CLOSE evaluation input data
 * @param weights - Optional custom weights for each dimension
 * @returns Native CLOSE evaluation result with gear recommendation
 */
export function evaluateCLOSENative(
  input: NativeCLOSEInput,
  weights?: NativeCLOSEWeights,
): NativeCLOSEEvaluation {
  if (!nativeEvaluateClose) {
    throw new Error("Native CLOSE evaluator not available. Build native modules with `cargo build` in services/zero-core.")
  }

  // Convert input to NAPI format
  const napiInput: NapiCLOSEInput = {
    snapshotConfidence: input.snapshotConfidence,
    buildStatus: input.buildStatus,
    sessionHealth: input.sessionHealth,
    criticalAnomalies: input.criticalAnomalies,
    strongPatterns: input.strongPatterns,
    highImpactOpportunities: input.highImpactOpportunities,
    mediumImpactOpportunities: input.mediumImpactOpportunities,
    externalOpportunities: input.externalOpportunities,
    externalRisks: input.externalRisks,
    totalOpportunities: input.totalOpportunities,
    patternTypes: input.patternTypes,
    anomalyCount: input.anomalyCount,
    decisionQuality: input.decisionQuality,
    recentErrors: input.recentErrors,
    tokenUsage: input.tokenUsage,
    cost: input.cost,
    consensusStrength: input.consensusStrength,
    coverageGaps: input.coverageGaps,
    learningOpportunities: input.learningOpportunities,
    recentChanges: input.recentChanges,
    techDebtLevel: input.techDebtLevel,
    dismissedAnomalies: input.dismissedAnomalies,
    activeAnomalies: input.activeAnomalies,
    recentTrendAvg: input.recentTrendAvg ?? undefined,
    olderTrendAvg: input.olderTrendAvg ?? undefined,
  }

  const napiWeights: NapiCLOSEWeights | undefined = weights
    ? {
        convergence: weights.convergence,
        leverage: weights.leverage,
        optionality: weights.optionality,
        surplus: weights.surplus,
        evolution: weights.evolution,
      }
    : undefined

  const result = nativeEvaluateClose(napiInput, napiWeights)

  return {
    convergence: result.convergence,
    leverage: result.leverage,
    optionality: result.optionality,
    surplus: result.surplus,
    evolution: result.evolution,
    total: result.total,
    risk: result.risk,
    confidence: result.confidence,
    recommendedGear: result.recommendedGear as "P" | "N" | "D" | "S",
    timestamp: result.timestamp,
  }
}

/**
 * Check if native CLOSE evaluator is available
 */
export function isNativeCLOSEAvailable(): boolean {
  return nativeEvaluateClose !== undefined
}

