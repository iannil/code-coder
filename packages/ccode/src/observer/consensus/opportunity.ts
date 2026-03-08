/**
 * Opportunity Identification
 *
 * Identifies opportunities from observation streams including:
 * - Optimization opportunities
 * - Automation possibilities
 * - Learning opportunities
 * - Market opportunities
 * - Timing windows
 *
 * @module observer/consensus/opportunity
 */

import { Log } from "@/util/log"
import type { Observation, Opportunity, EmergentPattern, Anomaly } from "../types"

const log = Log.create({ service: "observer.consensus.opportunity" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface OpportunityConfig {
  /** Minimum confidence for opportunity */
  minConfidence: number
  /** Enable specific opportunity types */
  enabledTypes: Opportunity["type"][]
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: OpportunityConfig = {
  minConfidence: 0.4,
  enabledTypes: ["optimization", "automation", "learning", "improvement", "market", "timing"],
}

// ─────────────────────────────────────────────────────────────────────────────
// Opportunity Identifier
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Identifies opportunities from observations, patterns, and anomalies.
 */
export class OpportunityIdentifier {
  private config: OpportunityConfig
  private activeOpportunities: Map<string, Opportunity> = new Map()
  private idCounter = 0

  constructor(config: Partial<OpportunityConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Identify opportunities from observations and patterns.
   */
  identify(
    observations: Observation[],
    patterns: EmergentPattern[],
    anomalies: Anomaly[],
  ): Opportunity[] {
    const opportunities: Opportunity[] = []

    // From observations
    if (this.config.enabledTypes.includes("optimization")) {
      opportunities.push(...this.identifyOptimizations(observations))
    }

    // From patterns
    if (this.config.enabledTypes.includes("automation")) {
      opportunities.push(...this.identifyAutomation(patterns))
    }

    if (this.config.enabledTypes.includes("learning")) {
      opportunities.push(...this.identifyLearning(patterns, anomalies))
    }

    // From anomalies
    if (this.config.enabledTypes.includes("improvement")) {
      opportunities.push(...this.identifyImprovements(anomalies))
    }

    // Filter by confidence
    const valid = opportunities.filter((o) => o.confidence >= this.config.minConfidence)

    // Update active opportunities
    for (const opp of valid) {
      const existing = this.findSimilarOpportunity(opp)
      if (!existing) {
        this.activeOpportunities.set(opp.id, opp)
      }
    }

    return valid
  }

  /**
   * Get active opportunities.
   */
  getActive(): Opportunity[] {
    return Array.from(this.activeOpportunities.values())
  }

  /**
   * Dismiss an opportunity.
   */
  dismiss(opportunityId: string): boolean {
    return this.activeOpportunities.delete(opportunityId)
  }

  /**
   * Expire old opportunities.
   */
  expireOpportunities(maxAge: number): string[] {
    const now = Date.now()
    const expired: string[] = []

    for (const [id, opp] of this.activeOpportunities.entries()) {
      const age = now - opp.detectedAt.getTime()
      if (age > maxAge) {
        this.activeOpportunities.delete(id)
        expired.push(id)
      }
    }

    return expired
  }

  /**
   * Clear all state.
   */
  clear(): void {
    this.activeOpportunities.clear()
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Identification Algorithms
  // ─────────────────────────────────────────────────────────────────────────────

  private identifyOptimizations(observations: Observation[]): Opportunity[] {
    const opportunities: Opportunity[] = []

    // Look for self-observations with low efficiency
    const selfObs = observations.filter((o) => o.watcherType === "self")
    const resourceObs = selfObs.filter((o) => (o as any).type === "resource_usage")

    if (resourceObs.length >= 3) {
      // Calculate average efficiency
      const efficiencies = resourceObs
        .map((o) => (o as any).quality?.efficiency)
        .filter((e) => typeof e === "number") as number[]

      if (efficiencies.length > 0) {
        const avgEfficiency = efficiencies.reduce((a, b) => a + b, 0) / efficiencies.length

        if (avgEfficiency < 0.5) {
          opportunities.push(
            this.createOpportunity("optimization", resourceObs.map((o) => o.id), {
              description: `Resource efficiency is low (${(avgEfficiency * 100).toFixed(0)}%). Consider optimization.`,
              impact: avgEfficiency < 0.3 ? "high" : "medium",
              urgency: "medium",
              confidence: 0.7,
              actions: [
                "Review token usage patterns",
                "Identify redundant operations",
                "Consider caching strategies",
              ],
            }),
          )
        }
      }
    }

    // Look for code observations with tech debt
    const codeObs = observations.filter((o) => o.watcherType === "code")
    const techDebtObs = codeObs.filter((o) => (o as any).type === "tech_debt")

    if (techDebtObs.length > 0) {
      opportunities.push(
        this.createOpportunity("optimization", techDebtObs.map((o) => o.id), {
          description: "Technical debt detected. Consider refactoring.",
          impact: "medium",
          urgency: "low",
          confidence: 0.6,
          actions: [
            "Prioritize high-impact debt items",
            "Schedule refactoring sprints",
            "Update documentation",
          ],
        }),
      )
    }

    return opportunities
  }

  private identifyAutomation(patterns: EmergentPattern[]): Opportunity[] {
    const opportunities: Opportunity[] = []

    // Look for repeating sequences that could be automated
    const sequencePatterns = patterns.filter((p) => p.type === "sequence")

    for (const pattern of sequencePatterns) {
      if (pattern.strength >= 0.6) {
        opportunities.push(
          this.createOpportunity("automation", pattern.observationIds, {
            description: `Repeating sequence detected: "${pattern.name}". Consider automation.`,
            impact: pattern.strength > 0.8 ? "high" : "medium",
            urgency: "low",
            confidence: pattern.confidence,
            actions: [
              "Analyze sequence trigger conditions",
              "Design automated workflow",
              "Implement automation with appropriate safeguards",
              ...pattern.suggestedActions,
            ],
          }),
        )
      }
    }

    // Look for strong correlations that suggest causal relationships
    const correlationPatterns = patterns.filter((p) => p.type === "correlation")

    for (const pattern of correlationPatterns) {
      if (pattern.strength >= 0.7) {
        opportunities.push(
          this.createOpportunity("automation", pattern.observationIds, {
            description: `Strong correlation detected: "${pattern.name}". Potential for predictive automation.`,
            impact: "medium",
            urgency: "low",
            confidence: pattern.confidence * 0.8,
            actions: [
              "Validate causal relationship",
              "Design predictive trigger",
              "Test automated response",
            ],
          }),
        )
      }
    }

    return opportunities
  }

  private identifyLearning(
    patterns: EmergentPattern[],
    anomalies: Anomaly[],
  ): Opportunity[] {
    const opportunities: Opportunity[] = []

    // Unresolved anomalies are learning opportunities
    const unresolvedAnomalies = anomalies.filter((a) => a.status === "suspected")

    if (unresolvedAnomalies.length >= 2) {
      const ids = unresolvedAnomalies.flatMap((a) => a.observationIds)
      opportunities.push(
        this.createOpportunity("learning", ids, {
          description: `${unresolvedAnomalies.length} unresolved anomalies detected. Investigation may reveal system insights.`,
          impact: "medium",
          urgency: "medium",
          confidence: 0.6,
          actions: [
            "Investigate anomaly root causes",
            "Document findings",
            "Update detection thresholds if needed",
          ],
        }),
      )
    }

    // New patterns are learning opportunities
    const recentPatterns = patterns.filter((p) => {
      const age = Date.now() - p.detectedAt.getTime()
      return age < 300000 // Less than 5 minutes old
    })

    if (recentPatterns.length > 0) {
      const ids = recentPatterns.flatMap((p) => p.observationIds)
      opportunities.push(
        this.createOpportunity("learning", ids, {
          description: `${recentPatterns.length} new patterns detected. System behavior may be changing.`,
          impact: "low",
          urgency: "low",
          confidence: 0.5,
          actions: [
            "Monitor pattern evolution",
            "Consider updating baseline models",
            "Document new patterns",
          ],
        }),
      )
    }

    return opportunities
  }

  private identifyImprovements(anomalies: Anomaly[]): Opportunity[] {
    const opportunities: Opportunity[] = []

    // High-severity anomalies indicate improvement needs
    const highSeverity = anomalies.filter(
      (a) => a.severity === "high" || a.severity === "critical",
    )

    for (const anomaly of highSeverity) {
      opportunities.push(
        this.createOpportunity("improvement", anomaly.observationIds, {
          description: `High-severity anomaly: ${anomaly.description}. System improvement needed.`,
          impact: anomaly.severity === "critical" ? "high" : "medium",
          urgency: anomaly.severity === "critical" ? "high" : "medium",
          confidence: anomaly.confidence,
          actions: [
            "Investigate root cause",
            "Implement fix or mitigation",
            "Add monitoring for similar issues",
          ],
        }),
      )
    }

    // Timing anomalies suggest reliability improvements
    const timingAnomalies = anomalies.filter((a) => a.type === "timing")

    if (timingAnomalies.length >= 2) {
      const ids = timingAnomalies.flatMap((a) => a.observationIds)
      opportunities.push(
        this.createOpportunity("improvement", ids, {
          description: "Multiple timing anomalies detected. System reliability may need improvement.",
          impact: "medium",
          urgency: "medium",
          confidence: 0.7,
          actions: [
            "Review watcher configurations",
            "Add redundancy where needed",
            "Implement health checks",
          ],
        }),
      )
    }

    return opportunities
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Helper Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private createOpportunity(
    type: Opportunity["type"],
    observationIds: string[],
    details: {
      description: string
      impact: Opportunity["impact"]
      urgency: Opportunity["urgency"]
      confidence: number
      actions: string[]
    },
  ): Opportunity {
    return {
      id: `opp_${Date.now()}_${++this.idCounter}`,
      type,
      description: details.description,
      impact: details.impact,
      urgency: details.urgency,
      observationIds,
      detectedAt: new Date(),
      confidence: details.confidence,
      suggestedActions: details.actions,
    }
  }

  private findSimilarOpportunity(opportunity: Opportunity): Opportunity | null {
    for (const existing of this.activeOpportunities.values()) {
      if (existing.type !== opportunity.type) continue

      // Check description similarity
      if (existing.description === opportunity.description) {
        return existing
      }

      // Check observation overlap
      const overlap = opportunity.observationIds.filter((id) =>
        existing.observationIds.includes(id),
      )
      if (overlap.length > opportunity.observationIds.length * 0.5) {
        return existing
      }
    }
    return null
  }
}

/**
 * Create an opportunity identifier.
 */
export function createOpportunityIdentifier(
  config?: Partial<OpportunityConfig>,
): OpportunityIdentifier {
  return new OpportunityIdentifier(config)
}
