/**
 * Causal Analysis
 *
 * Pattern recognition and suggestion generation for the Causal Graph.
 * Analyzes historical decision-outcome data to identify patterns
 * and provide recommendations.
 *
 * Part of Phase 16: 因果链图数据库 (Causal Graph)
 *
 * NOTE: Core analysis algorithms are now implemented in Rust for performance.
 * This file provides thin wrappers and TypeScript-specific business logic.
 */

import { Log } from "@/util/log"
import { CausalGraph } from "./graph"
import type { CausalPattern, CausalSuggestion, ActionType, OutcomeStatus } from "./causal-types"
import type { NapiCausalChain, NapiCausalPattern, NapiSimilarDecision, NapiTrendAnalysis, NapiAgentInsights } from "@codecoder-ai/core"

const log = Log.create({ service: "memory.knowledge.causal-analysis" })

export namespace CausalAnalysis {
  // ============================================================================
  // Pattern Recognition (Native Rust)
  // ============================================================================

  /**
   * Identify recurring decision-outcome patterns
   * Delegates to native Rust implementation for O(N) performance
   */
  export async function findPatterns(options?: {
    agentId?: string
    minOccurrences?: number
    limit?: number
  }): Promise<CausalPattern[]> {
    const nativePatterns = await CausalGraph.findPatterns(options)

    // Convert NapiCausalPattern to CausalPattern for API compatibility
    return nativePatterns.map((p) => ({
      id: p.id,
      name: p.name,
      description: p.description,
      agentId: p.agentId,
      actionType: p.actionType as ActionType,
      occurrences: p.occurrences,
      successRate: p.successRate,
      avgConfidence: p.avgConfidence,
      examples: p.examples,
    }))
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

    // Find similar past decisions using native Rust implementation
    const similarDecisions = await CausalGraph.findSimilarDecisions(input.prompt, input.agentId, 3)

    for (const similar of similarDecisions) {
      const chain = await CausalGraph.getCausalChain(similar.decisionId)
      if (!chain) continue

      const successOutcomes = chain.outcomes.filter((o) => o.status === "success")
      const failureOutcomes = chain.outcomes.filter((o) => o.status === "failure")

      if (successOutcomes.length > failureOutcomes.length) {
        // Successful similar decision - recommend similar approach
        suggestions.push({
          id: `sug_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
          type: "similar_decision",
          confidence: similar.similarity * (successOutcomes.length / (chain.outcomes.length || 1)),
          reasoning: `Similar decision "${similar.prompt.slice(0, 50)}..." succeeded with ${chain.actions[0]?.actionType || "unknown"} approach`,
          basedOn: [similar.decisionId],
          suggestedAction: chain.actions[0]?.description,
        })
      } else if (failureOutcomes.length > 0) {
        // Failed similar decision - warn about pattern
        suggestions.push({
          id: `sug_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
          type: "avoid_pattern",
          confidence: similar.similarity * (failureOutcomes.length / (chain.outcomes.length || 1)),
          reasoning: `Similar decision "${similar.prompt.slice(0, 50)}..." failed with ${chain.actions[0]?.actionType || "unknown"} approach`,
          basedOn: [similar.decisionId],
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

  // ============================================================================
  // Trend Analysis (Native Rust)
  // ============================================================================

  /**
   * Analyze decision trends over time
   * Delegates to native Rust implementation for statistical analysis
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
    const nativeTrends = await CausalGraph.analyzeTrends(options)

    // Parse action type shifts from JSON string
    let actionTypeShifts: Record<string, { before: number; after: number }> = {}
    try {
      const shifts = JSON.parse(nativeTrends.actionTypeShifts) as Record<string, [number, number]>
      for (const [type, [before, after]] of Object.entries(shifts)) {
        actionTypeShifts[type] = { before, after }
      }
    } catch {
      // If parsing fails, return empty shifts
    }

    return {
      totalDecisions: nativeTrends.totalDecisions,
      successRateTrend: [nativeTrends.successRateBefore, nativeTrends.successRateAfter],
      confidenceTrend: [nativeTrends.confidenceBefore, nativeTrends.confidenceAfter],
      actionTypeShifts,
    }
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
      status: outcome.status as OutcomeStatus,
      confidence: decision.confidence,
      relatedDecisions,
    }
  }

  /**
   * Get aggregated insights for an agent
   * Delegates to native Rust implementation for comprehensive analysis
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
    const nativeInsights = await CausalGraph.getAgentInsights(agentId)

    return {
      totalDecisions: nativeInsights.totalDecisions,
      successRate: nativeInsights.successRate,
      avgConfidence: nativeInsights.avgConfidence,
      strongestActionType: nativeInsights.strongestActionType ?? null,
      weakestActionType: nativeInsights.weakestActionType ?? null,
      recentTrend: nativeInsights.recentTrend as "improving" | "declining" | "stable",
      suggestions: nativeInsights.suggestions,
    }
  }
}
