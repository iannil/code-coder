/**
 * Causal Analysis
 *
 * Pattern recognition and suggestion generation for the Causal Graph.
 * Analyzes historical decision-outcome data to identify patterns
 * and provide recommendations.
 *
 * Part of Phase 16: 因果链图数据库 (Causal Graph)
 */

import { Log } from "@/util/log"
import { CausalGraph } from "./causal-graph"
import type {
  CausalPattern,
  CausalSuggestion,
  CausalChain,
  ActionType,
  OutcomeStatus,
} from "./causal-types"

const log = Log.create({ service: "memory.knowledge.causal-analysis" })

export namespace CausalAnalysis {
  // ============================================================================
  // Pattern Recognition
  // ============================================================================

  /**
   * Identify recurring decision-outcome patterns
   */
  export async function findPatterns(options?: {
    agentId?: string
    minOccurrences?: number
    limit?: number
  }): Promise<CausalPattern[]> {
    const minOccurrences = options?.minOccurrences ?? 2
    const limit = options?.limit ?? 20

    const chains = await CausalGraph.query({
      agentId: options?.agentId,
      limit: 1000, // Get all for pattern analysis
    })

    // Group by agent + action type combination
    const patternMap = new Map<
      string,
      {
        agentId: string
        actionType: ActionType
        occurrences: number
        successes: number
        confidenceSum: number
        decisionIds: string[]
      }
    >()

    for (const chain of chains) {
      for (const action of chain.actions) {
        const key = `${chain.decision.agentId}:${action.actionType}`
        const existing = patternMap.get(key) || {
          agentId: chain.decision.agentId,
          actionType: action.actionType as ActionType,
          occurrences: 0,
          successes: 0,
          confidenceSum: 0,
          decisionIds: [],
        }

        existing.occurrences++
        existing.confidenceSum += chain.decision.confidence
        existing.decisionIds.push(chain.decision.id)

        // Check if this action led to success
        const actionOutcomes = chain.outcomes.filter((o) => o.actionId === action.id)
        const hasSuccess = actionOutcomes.some((o) => o.status === "success")
        if (hasSuccess) existing.successes++

        patternMap.set(key, existing)
      }
    }

    // Convert to patterns and filter by minimum occurrences
    const patterns: CausalPattern[] = []
    let patternIndex = 0

    for (const [key, data] of patternMap) {
      if (data.occurrences < minOccurrences) continue

      const successRate = data.occurrences > 0 ? data.successes / data.occurrences : 0
      const avgConfidence = data.occurrences > 0 ? data.confidenceSum / data.occurrences : 0

      patterns.push({
        id: `pattern_${patternIndex++}`,
        name: `${data.agentId} ${data.actionType} pattern`,
        description: `Agent ${data.agentId} performing ${data.actionType} actions`,
        agentId: data.agentId,
        actionType: data.actionType,
        occurrences: data.occurrences,
        successRate,
        avgConfidence,
        examples: data.decisionIds.slice(0, 5),
      })
    }

    // Sort by occurrences and limit
    return patterns.sort((a, b) => b.occurrences - a.occurrences).slice(0, limit)
  }

  /**
   * Find failure patterns - recurring decisions that often fail
   */
  export async function findFailurePatterns(options?: {
    agentId?: string
    minOccurrences?: number
    maxSuccessRate?: number
  }): Promise<CausalPattern[]> {
    const maxSuccessRate = options?.maxSuccessRate ?? 0.3

    const allPatterns = await findPatterns({
      agentId: options?.agentId,
      minOccurrences: options?.minOccurrences ?? 2,
      limit: 100,
    })

    return allPatterns.filter((p) => p.successRate <= maxSuccessRate)
  }

  /**
   * Find success patterns - recurring decisions that often succeed
   */
  export async function findSuccessPatterns(options?: {
    agentId?: string
    minOccurrences?: number
    minSuccessRate?: number
  }): Promise<CausalPattern[]> {
    const minSuccessRate = options?.minSuccessRate ?? 0.7

    const allPatterns = await findPatterns({
      agentId: options?.agentId,
      minOccurrences: options?.minOccurrences ?? 2,
      limit: 100,
    })

    return allPatterns.filter((p) => p.successRate >= minSuccessRate)
  }

  // ============================================================================
  // Suggestion Generation
  // ============================================================================

  /**
   * Generate suggestions based on historical causal data
   */
  export async function suggestFromHistory(input: {
    prompt: string
    agentId: string
    context?: { files?: string[]; tools?: string[] }
  }): Promise<CausalSuggestion[]> {
    const suggestions: CausalSuggestion[] = []

    // Find similar past decisions
    const similarDecisions = await findSimilarDecisions(input.prompt, input.agentId)

    for (const similar of similarDecisions.slice(0, 3)) {
      const chain = await CausalGraph.getCausalChain(similar.decision.id)
      if (!chain) continue

      const successOutcomes = chain.outcomes.filter((o) => o.status === "success")
      const failureOutcomes = chain.outcomes.filter((o) => o.status === "failure")

      if (successOutcomes.length > failureOutcomes.length) {
        // Successful similar decision - recommend similar approach
        suggestions.push({
          id: `sug_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
          type: "similar_decision",
          confidence: similar.similarity * (successOutcomes.length / (chain.outcomes.length || 1)),
          reasoning: `Similar decision "${similar.decision.prompt.slice(0, 50)}..." succeeded with ${chain.actions[0]?.actionType || "unknown"} approach`,
          basedOn: [similar.decision.id],
          suggestedAction: chain.actions[0]?.description,
        })
      } else if (failureOutcomes.length > 0) {
        // Failed similar decision - warn about pattern
        suggestions.push({
          id: `sug_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
          type: "avoid_pattern",
          confidence: similar.similarity * (failureOutcomes.length / (chain.outcomes.length || 1)),
          reasoning: `Similar decision "${similar.decision.prompt.slice(0, 50)}..." failed with ${chain.actions[0]?.actionType || "unknown"} approach`,
          basedOn: [similar.decision.id],
        })
      }
    }

    // Add recommendations based on agent success patterns
    const successPatterns = await findSuccessPatterns({ agentId: input.agentId, minOccurrences: 3 })
    if (successPatterns.length > 0) {
      const topPattern = successPatterns[0]
      suggestions.push({
        id: `sug_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        type: "recommended_action",
        confidence: topPattern.successRate * topPattern.avgConfidence,
        reasoning: `${topPattern.actionType} actions have ${Math.round(topPattern.successRate * 100)}% success rate for this agent`,
        basedOn: topPattern.examples,
        suggestedAction: `Consider using ${topPattern.actionType} approach`,
      })
    }

    // Sort by confidence
    return suggestions.sort((a, b) => b.confidence - a.confidence)
  }

  /**
   * Find decisions similar to the given prompt
   */
  async function findSimilarDecisions(
    prompt: string,
    agentId: string,
  ): Promise<Array<{ decision: CausalChain["decision"]; similarity: number }>> {
    const chains = await CausalGraph.query({ agentId, limit: 100 })
    const results: Array<{ decision: CausalChain["decision"]; similarity: number }> = []

    const promptWords = extractKeywords(prompt)

    for (const chain of chains) {
      const decisionWords = extractKeywords(chain.decision.prompt)
      const similarity = calculateSimilarity(promptWords, decisionWords)

      if (similarity > 0.2) {
        results.push({ decision: chain.decision, similarity })
      }
    }

    return results.sort((a, b) => b.similarity - a.similarity)
  }

  /**
   * Extract keywords from text for similarity comparison
   */
  function extractKeywords(text: string): Set<string> {
    const stopWords = new Set([
      "the", "a", "an", "is", "are", "was", "were", "be", "been",
      "being", "have", "has", "had", "do", "does", "did", "will",
      "would", "could", "should", "may", "might", "must", "shall",
      "can", "to", "of", "in", "for", "on", "with", "at", "by",
      "from", "or", "and", "not", "this", "that", "these", "those",
      "it", "its", "i", "me", "my", "we", "our", "you", "your",
    ])

    const words = text
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, " ")
      .split(/\s+/)
      .filter((w) => w.length > 2 && !stopWords.has(w))

    return new Set(words)
  }

  /**
   * Calculate Jaccard similarity between two keyword sets
   */
  function calculateSimilarity(set1: Set<string>, set2: Set<string>): number {
    if (set1.size === 0 && set2.size === 0) return 0

    const intersection = new Set([...set1].filter((x) => set2.has(x)))
    const union = new Set([...set1, ...set2])

    return intersection.size / union.size
  }

  // ============================================================================
  // Trend Analysis
  // ============================================================================

  /**
   * Analyze decision trends over time
   */
  export async function analyzeTrends(options?: {
    agentId?: string
    periodDays?: number
  }): Promise<{
    totalDecisions: number
    successRateTrend: number[]
    confidenceTrend: number[]
    actionTypeShifts: Record<string, { before: number; after: number }>
  }> {
    const periodDays = options?.periodDays ?? 7
    const now = new Date()
    const periodStart = new Date(now.getTime() - periodDays * 24 * 60 * 60 * 1000)

    const chains = await CausalGraph.query({
      agentId: options?.agentId,
      dateFrom: new Date(now.getTime() - 2 * periodDays * 24 * 60 * 60 * 1000).toISOString(),
      limit: 1000,
    })

    // Split into before and after periods
    const beforePeriod: CausalChain[] = []
    const afterPeriod: CausalChain[] = []

    for (const chain of chains) {
      const decisionDate = new Date(chain.decision.timestamp)
      if (decisionDate >= periodStart) {
        afterPeriod.push(chain)
      } else {
        beforePeriod.push(chain)
      }
    }

    // Calculate success rate trend
    const beforeSuccessRate = calculatePeriodSuccessRate(beforePeriod)
    const afterSuccessRate = calculatePeriodSuccessRate(afterPeriod)

    // Calculate confidence trend
    const beforeConfidence = calculatePeriodConfidence(beforePeriod)
    const afterConfidence = calculatePeriodConfidence(afterPeriod)

    // Calculate action type shifts
    const beforeActionTypes = countActionTypes(beforePeriod)
    const afterActionTypes = countActionTypes(afterPeriod)
    const allTypes = new Set([...Object.keys(beforeActionTypes), ...Object.keys(afterActionTypes)])

    const actionTypeShifts: Record<string, { before: number; after: number }> = {}
    for (const type of allTypes) {
      actionTypeShifts[type] = {
        before: beforeActionTypes[type] || 0,
        after: afterActionTypes[type] || 0,
      }
    }

    return {
      totalDecisions: chains.length,
      successRateTrend: [beforeSuccessRate, afterSuccessRate],
      confidenceTrend: [beforeConfidence, afterConfidence],
      actionTypeShifts,
    }
  }

  function calculatePeriodSuccessRate(chains: CausalChain[]): number {
    let totalOutcomes = 0
    let successOutcomes = 0

    for (const chain of chains) {
      totalOutcomes += chain.outcomes.length
      successOutcomes += chain.outcomes.filter((o) => o.status === "success").length
    }

    return totalOutcomes > 0 ? successOutcomes / totalOutcomes : 0
  }

  function calculatePeriodConfidence(chains: CausalChain[]): number {
    if (chains.length === 0) return 0
    return chains.reduce((sum, c) => sum + c.decision.confidence, 0) / chains.length
  }

  function countActionTypes(chains: CausalChain[]): Record<string, number> {
    const counts: Record<string, number> = {}
    for (const chain of chains) {
      for (const action of chain.actions) {
        counts[action.actionType] = (counts[action.actionType] || 0) + 1
      }
    }
    return counts
  }

  // ============================================================================
  // Learning from Outcomes
  // ============================================================================

  /**
   * Extract lessons learned from a specific outcome
   */
  export async function extractLessons(outcomeId: string): Promise<{
    lesson: string
    actionType: ActionType
    status: OutcomeStatus
    confidence: number
    relatedDecisions: string[]
  } | null> {
    const outcome = await CausalGraph.getOutcome(outcomeId)
    if (!outcome) return null

    const action = await CausalGraph.getAction(outcome.actionId)
    if (!action) return null

    const decision = await CausalGraph.getDecision(action.decisionId)
    if (!decision) return null

    // Find similar decisions
    const similarChains = await CausalGraph.query({
      agentId: decision.agentId,
      actionType: action.actionType as ActionType,
      limit: 10,
    })

    const relatedDecisions = similarChains
      .filter((c) => c.decision.id !== decision.id)
      .map((c) => c.decision.id)

    let lesson: string
    if (outcome.status === "success") {
      lesson = `${action.actionType} action succeeded: ${outcome.description}`
    } else if (outcome.status === "failure") {
      lesson = `${action.actionType} action failed: ${outcome.description}. Consider alternative approaches.`
    } else {
      lesson = `${action.actionType} action partially succeeded: ${outcome.description}. May need refinement.`
    }

    return {
      lesson,
      actionType: action.actionType as ActionType,
      status: outcome.status,
      confidence: decision.confidence,
      relatedDecisions,
    }
  }

  /**
   * Get aggregated insights for an agent
   */
  export async function getAgentInsights(agentId: string): Promise<{
    totalDecisions: number
    successRate: number
    avgConfidence: number
    strongestActionType: string | null
    weakestActionType: string | null
    recentTrend: "improving" | "declining" | "stable"
    suggestions: string[]
  }> {
    const stats = await CausalGraph.getStats()
    const agentStats = stats.topAgents.find((a) => a.agentId === agentId)

    if (!agentStats) {
      return {
        totalDecisions: 0,
        successRate: 0,
        avgConfidence: 0,
        strongestActionType: null,
        weakestActionType: null,
        recentTrend: "stable",
        suggestions: ["No historical data available for this agent"],
      }
    }

    const patterns = await findPatterns({ agentId, minOccurrences: 2 })

    const strongestPattern = patterns
      .filter((p) => p.successRate >= 0.7)
      .sort((a, b) => b.successRate - a.successRate)[0]

    const weakestPattern = patterns
      .filter((p) => p.successRate <= 0.3)
      .sort((a, b) => a.successRate - b.successRate)[0]

    const trends = await analyzeTrends({ agentId, periodDays: 7 })
    const [beforeSuccess, afterSuccess] = trends.successRateTrend
    const recentTrend =
      afterSuccess > beforeSuccess + 0.1
        ? "improving"
        : afterSuccess < beforeSuccess - 0.1
          ? "declining"
          : "stable"

    const suggestions: string[] = []
    if (weakestPattern) {
      suggestions.push(`Consider reviewing ${weakestPattern.actionType} approach - only ${Math.round(weakestPattern.successRate * 100)}% success rate`)
    }
    if (strongestPattern) {
      suggestions.push(`${strongestPattern.actionType} is working well (${Math.round(strongestPattern.successRate * 100)}% success) - consider using more`)
    }
    if (recentTrend === "declining") {
      suggestions.push("Performance declining - review recent failures for patterns")
    }

    return {
      totalDecisions: agentStats.decisionCount,
      successRate: agentStats.successRate,
      avgConfidence: stats.avgConfidence,
      strongestActionType: strongestPattern?.actionType ?? null,
      weakestActionType: weakestPattern?.actionType ?? null,
      recentTrend,
      suggestions,
    }
  }
}
