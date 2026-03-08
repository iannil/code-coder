/**
 * CLOSE Evaluator for Mode Decisions
 *
 * Evaluates observation data using the CLOSE framework to determine
 * appropriate operating mode.
 *
 * CLOSE Framework:
 * - Convergence: How well do observations agree?
 * - Leverage: What's the potential impact?
 * - Optionality: How many valid choices exist?
 * - Surplus: Do we have resources/margin?
 * - Evolution: Is the situation improving?
 *
 * @module observer/controller/close-evaluator
 */

import { Log } from "@/util/log"
import type { WorldModel, Anomaly, EmergentPattern, Opportunity } from "../types"
import type { ConsensusSnapshot } from "../consensus"

const log = Log.create({ service: "observer.controller.close-evaluator" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface CLOSEDimension {
  score: number // 0-10
  confidence: number // 0-1
  factors: string[]
}

export interface CLOSEEvaluation {
  convergence: CLOSEDimension
  leverage: CLOSEDimension
  optionality: CLOSEDimension
  surplus: CLOSEDimension
  evolution: CLOSEDimension
  total: number // Weighted sum, 0-10
  risk: number // Computed risk score, 0-10
  confidence: number // Overall confidence, 0-1
  timestamp: Date
}

export interface CLOSEWeights {
  convergence: number
  leverage: number
  optionality: number
  surplus: number
  evolution: number
}

export interface CLOSEEvaluatorConfig {
  weights: CLOSEWeights
  /** How much to weight recent history vs current snapshot */
  historyWeight: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_WEIGHTS: CLOSEWeights = {
  convergence: 1.0,
  leverage: 1.2,
  optionality: 1.5, // Higher weight: optionality is key to "再来一次"
  surplus: 1.3,
  evolution: 0.8,
}

const DEFAULT_CONFIG: CLOSEEvaluatorConfig = {
  weights: DEFAULT_WEIGHTS,
  historyWeight: 0.3,
}

// ─────────────────────────────────────────────────────────────────────────────
// CLOSE Evaluator
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Evaluates observation data using the CLOSE framework.
 */
export class CLOSEEvaluator {
  private config: CLOSEEvaluatorConfig
  private evaluationHistory: CLOSEEvaluation[] = []
  private maxHistory = 100

  constructor(config: Partial<CLOSEEvaluatorConfig> = {}) {
    this.config = {
      ...DEFAULT_CONFIG,
      ...config,
      weights: { ...DEFAULT_WEIGHTS, ...config.weights },
    }
  }

  /**
   * Evaluate the current snapshot.
   */
  evaluate(snapshot: ConsensusSnapshot): CLOSEEvaluation {
    const convergence = this.evaluateConvergence(snapshot)
    const leverage = this.evaluateLeverage(snapshot)
    const optionality = this.evaluateOptionality(snapshot)
    const surplus = this.evaluateSurplus(snapshot)
    const evolution = this.evaluateEvolution(snapshot)

    // Calculate weighted total
    const { weights } = this.config
    const weightSum =
      weights.convergence + weights.leverage + weights.optionality + weights.surplus + weights.evolution

    const rawTotal =
      convergence.score * weights.convergence +
      leverage.score * weights.leverage +
      optionality.score * weights.optionality +
      surplus.score * weights.surplus +
      evolution.score * weights.evolution

    const total = (rawTotal / weightSum / 10) * 10 // Normalize to 0-10

    // Calculate risk as inverse of safety factors
    const risk = this.calculateRisk(convergence, leverage, optionality, surplus, evolution, snapshot)

    // Calculate overall confidence
    const confidence = this.calculateOverallConfidence(convergence, leverage, optionality, surplus, evolution)

    const evaluation: CLOSEEvaluation = {
      convergence,
      leverage,
      optionality,
      surplus,
      evolution,
      total: Math.round(total * 100) / 100,
      risk: Math.round(risk * 100) / 100,
      confidence: Math.round(confidence * 100) / 100,
      timestamp: new Date(),
    }

    // Update history
    this.evaluationHistory.push(evaluation)
    if (this.evaluationHistory.length > this.maxHistory) {
      this.evaluationHistory.shift()
    }

    log.debug("CLOSE evaluation complete", {
      total: evaluation.total,
      risk: evaluation.risk,
      confidence: evaluation.confidence,
    })

    return evaluation
  }

  /**
   * Get evaluation history.
   */
  getHistory(limit?: number): CLOSEEvaluation[] {
    return this.evaluationHistory.slice(-(limit ?? 20))
  }

  /**
   * Get trend of CLOSE scores.
   */
  getTrend(): "improving" | "declining" | "stable" {
    const history = this.getHistory(10)
    if (history.length < 3) return "stable"

    const recent = history.slice(-3)
    const older = history.slice(0, Math.min(3, history.length - 3))

    const recentAvg = recent.reduce((sum, e) => sum + e.total, 0) / recent.length
    const olderAvg = older.reduce((sum, e) => sum + e.total, 0) / older.length

    if (recentAvg > olderAvg + 0.5) return "improving"
    if (recentAvg < olderAvg - 0.5) return "declining"
    return "stable"
  }

  /**
   * Clear evaluation history.
   */
  clear(): void {
    this.evaluationHistory = []
  }

  /**
   * Update configuration.
   */
  updateConfig(config: Partial<CLOSEEvaluatorConfig>): void {
    if (config.weights) {
      this.config.weights = { ...this.config.weights, ...config.weights }
    }
    if (config.historyWeight !== undefined) {
      this.config.historyWeight = config.historyWeight
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Dimension Evaluators
  // ─────────────────────────────────────────────────────────────────────────────

  private evaluateConvergence(snapshot: ConsensusSnapshot): CLOSEDimension {
    const factors: string[] = []
    let score = snapshot.confidence * 10 // Start from snapshot confidence

    // Check world model
    if (snapshot.worldModel) {
      const wm = snapshot.worldModel

      // Code state convergence
      if (wm.code.buildStatus === "passing") {
        score += 1
        factors.push("Build passing")
      } else if (wm.code.buildStatus === "failing") {
        score -= 2
        factors.push("Build failing")
      }

      // Self state convergence
      if (wm.self.sessionHealth === "healthy") {
        score += 1
        factors.push("Session healthy")
      } else if (wm.self.sessionHealth === "critical") {
        score -= 2
        factors.push("Session critical")
      }

      // Meta state convergence
      if (wm.meta.observerHealth === "healthy") {
        score += 0.5
        factors.push("Observers healthy")
      }
    }

    // Anomaly impact on convergence
    const criticalAnomalies = snapshot.anomalies.filter((a) => a.severity === "critical")
    if (criticalAnomalies.length > 0) {
      score -= criticalAnomalies.length * 1.5
      factors.push(`${criticalAnomalies.length} critical anomalies`)
    }

    // Pattern support for convergence
    const strongPatterns = snapshot.patterns.filter((p) => p.strength > 0.7)
    if (strongPatterns.length > 0) {
      score += strongPatterns.length * 0.3
      factors.push(`${strongPatterns.length} strong patterns`)
    }

    return {
      score: Math.max(0, Math.min(10, score)),
      confidence: snapshot.confidence,
      factors,
    }
  }

  private evaluateLeverage(snapshot: ConsensusSnapshot): CLOSEDimension {
    const factors: string[] = []
    let score = 5 // Neutral starting point

    // High-impact opportunities increase leverage
    const highImpactOpps = snapshot.opportunities.filter((o) => o.impact === "high")
    score += highImpactOpps.length * 1.5
    if (highImpactOpps.length > 0) {
      factors.push(`${highImpactOpps.length} high-impact opportunities`)
    }

    // Medium-impact opportunities
    const mediumImpactOpps = snapshot.opportunities.filter((o) => o.impact === "medium")
    score += mediumImpactOpps.length * 0.5
    if (mediumImpactOpps.length > 0) {
      factors.push(`${mediumImpactOpps.length} medium-impact opportunities`)
    }

    // World state leverage
    if (snapshot.worldModel?.world) {
      const world = snapshot.worldModel.world

      // Market opportunities
      if (world.opportunities.length > 0) {
        score += Math.min(world.opportunities.length * 0.5, 2)
        factors.push(`${world.opportunities.length} external opportunities`)
      }

      // External risks reduce leverage
      if (world.externalRisks.length > 0) {
        score -= Math.min(world.externalRisks.length * 0.5, 2)
        factors.push(`${world.externalRisks.length} external risks`)
      }
    }

    // Trend patterns increase leverage
    const trends = snapshot.patterns.filter((p) => p.type === "trend")
    score += trends.length * 0.3

    return {
      score: Math.max(0, Math.min(10, score)),
      confidence: snapshot.confidence * 0.9,
      factors,
    }
  }

  private evaluateOptionality(snapshot: ConsensusSnapshot): CLOSEDimension {
    const factors: string[] = []
    let score = 5 // Neutral starting point

    // More opportunities = more options
    const totalOpps = snapshot.opportunities.length
    score += Math.min(totalOpps * 0.5, 3)
    if (totalOpps > 0) {
      factors.push(`${totalOpps} opportunities available`)
    }

    // Diverse patterns suggest more paths
    const patternTypes = new Set(snapshot.patterns.map((p) => p.type))
    score += Math.min(patternTypes.size * 0.5, 2)
    if (patternTypes.size > 1) {
      factors.push(`${patternTypes.size} pattern types`)
    }

    // High anomaly count reduces options
    const anomalyCount = snapshot.anomalies.length
    if (anomalyCount > 5) {
      score -= 2
      factors.push("Many anomalies limiting options")
    } else if (anomalyCount > 2) {
      score -= 1
      factors.push("Some anomalies limiting options")
    }

    // Low optionality indicators
    if (snapshot.worldModel?.self) {
      const self = snapshot.worldModel.self

      // Low decision quality suggests fewer good options
      if (self.decisionQuality !== undefined && self.decisionQuality < 0.5) {
        score -= 1
        factors.push("Low decision quality")
      }

      // Many recent errors suggest constrained options
      if (self.recentErrors > 3) {
        score -= 1.5
        factors.push("Frequent errors")
      }
    }

    return {
      score: Math.max(0, Math.min(10, score)),
      confidence: snapshot.confidence * 0.85,
      factors,
    }
  }

  private evaluateSurplus(snapshot: ConsensusSnapshot): CLOSEDimension {
    const factors: string[] = []
    let score = 5 // Neutral starting point

    // Check resource margins
    if (snapshot.worldModel?.self) {
      const self = snapshot.worldModel.self

      // Resource usage (lower is better for surplus)
      const usage = self.resourceUsage
      if (usage) {
        // Token efficiency
        if (usage.tokens < 50000) {
          score += 1
          factors.push("Low token usage")
        } else if (usage.tokens > 200000) {
          score -= 1
          factors.push("High token usage")
        }

        // Cost efficiency
        if (usage.cost < 1.0) {
          score += 1
          factors.push("Low cost")
        } else if (usage.cost > 10.0) {
          score -= 1.5
          factors.push("High cost")
        }
      }

      // Session health
      if (self.sessionHealth === "healthy") {
        score += 1.5
        factors.push("Healthy session margin")
      } else if (self.sessionHealth === "degraded") {
        score -= 1
        factors.push("Degraded session")
      } else if (self.sessionHealth === "critical") {
        score -= 2
        factors.push("Critical session")
      }

      // Error margin
      if (self.recentErrors === 0) {
        score += 1
        factors.push("No recent errors")
      }
    }

    // Meta health
    if (snapshot.worldModel?.meta) {
      const meta = snapshot.worldModel.meta

      if (meta.consensusStrength > 0.7) {
        score += 1
        factors.push("Strong consensus")
      } else if (meta.consensusStrength < 0.4) {
        score -= 1
        factors.push("Weak consensus")
      }

      // Coverage gaps reduce surplus
      if (meta.coverageGaps.length > 0) {
        score -= meta.coverageGaps.length * 0.3
        factors.push(`${meta.coverageGaps.length} coverage gaps`)
      }
    }

    return {
      score: Math.max(0, Math.min(10, score)),
      confidence: snapshot.confidence * 0.9,
      factors,
    }
  }

  private evaluateEvolution(snapshot: ConsensusSnapshot): CLOSEDimension {
    const factors: string[] = []
    let score = 5 // Neutral starting point

    // Historical trend
    const trend = this.getTrend()
    if (trend === "improving") {
      score += 2
      factors.push("Improving trend")
    } else if (trend === "declining") {
      score -= 2
      factors.push("Declining trend")
    }

    // Learning opportunities suggest evolution potential
    const learningOpps = snapshot.opportunities.filter((o) => o.type === "learning")
    if (learningOpps.length > 0) {
      score += learningOpps.length * 0.5
      factors.push(`${learningOpps.length} learning opportunities`)
    }

    // Code evolution
    if (snapshot.worldModel?.code) {
      const code = snapshot.worldModel.code

      // Active development
      if (code.recentChanges > 0) {
        score += Math.min(code.recentChanges * 0.2, 1.5)
        factors.push(`${code.recentChanges} recent changes`)
      }

      // Tech debt reduction
      if (code.techDebtLevel === "low") {
        score += 1
        factors.push("Low tech debt")
      } else if (code.techDebtLevel === "high") {
        score -= 1
        factors.push("High tech debt")
      }
    }

    // Anomaly resolution shows evolution
    const dismissedAnomalies = snapshot.anomalies.filter((a) => a.status === "dismissed")
    const activeAnomalies = snapshot.anomalies.filter((a) => a.status !== "dismissed")
    if (dismissedAnomalies.length > activeAnomalies.length) {
      score += 1
      factors.push("Anomalies being resolved")
    } else if (activeAnomalies.length > dismissedAnomalies.length * 2) {
      score -= 1
      factors.push("Anomalies accumulating")
    }

    return {
      score: Math.max(0, Math.min(10, score)),
      confidence: snapshot.confidence * 0.8,
      factors,
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Risk and Confidence Calculation
  // ─────────────────────────────────────────────────────────────────────────────

  private calculateRisk(
    convergence: CLOSEDimension,
    leverage: CLOSEDimension,
    optionality: CLOSEDimension,
    surplus: CLOSEDimension,
    evolution: CLOSEDimension,
    snapshot: ConsensusSnapshot,
  ): number {
    // Risk is high when:
    // - Convergence is low (disagreement)
    // - Optionality is low (few choices)
    // - Surplus is low (no margin)
    // - Many anomalies present

    let risk = 5 // Neutral

    // Low convergence = high risk
    risk += (10 - convergence.score) * 0.3

    // Low optionality = high risk (can't "再来一次")
    risk += (10 - optionality.score) * 0.4

    // Low surplus = high risk (no margin for error)
    risk += (10 - surplus.score) * 0.3

    // Critical anomalies are direct risk factors
    const criticalAnomalies = snapshot.anomalies.filter((a) => a.severity === "critical")
    const highAnomalies = snapshot.anomalies.filter((a) => a.severity === "high")
    risk += criticalAnomalies.length * 1.5
    risk += highAnomalies.length * 0.5

    // Declining trend increases risk
    if (evolution.score < 4) {
      risk += 1
    }

    return Math.max(0, Math.min(10, risk))
  }

  private calculateOverallConfidence(
    convergence: CLOSEDimension,
    leverage: CLOSEDimension,
    optionality: CLOSEDimension,
    surplus: CLOSEDimension,
    evolution: CLOSEDimension,
  ): number {
    // Average of dimension confidences
    const confidences = [
      convergence.confidence,
      leverage.confidence,
      optionality.confidence,
      surplus.confidence,
      evolution.confidence,
    ]
    return confidences.reduce((sum, c) => sum + c, 0) / confidences.length
  }
}

/**
 * Create a CLOSE evaluator.
 */
export function createCLOSEEvaluator(config?: Partial<CLOSEEvaluatorConfig>): CLOSEEvaluator {
  return new CLOSEEvaluator(config)
}
