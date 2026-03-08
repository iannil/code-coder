/**
 * OASIS (Optimal Agent Synthesis and Integration System) Aggregator.
 *
 * Aggregates outputs from multiple parallel agents into a unified result.
 * Supports multiple aggregation modes:
 * - concatenate: Simple concatenation of outputs
 * - vote: Majority voting for discrete choices
 * - weighted: Confidence-weighted combination
 * - oasis: Full OASIS synthesis with conflict resolution
 *
 * ## Design Principle
 *
 * Aggregation requires **understanding** and **judgment** to properly
 * synthesize diverse agent outputs. This is an inherently uncertain task
 * that benefits from LLM reasoning.
 */

/**
 * Mode of aggregation.
 */
export interface AggregationMode {
  type: 'concatenate' | 'vote' | 'weighted' | 'oasis'
  /** For weighted mode: custom weight function */
  weightFn?: (result: AgentResult) => number
  /** For oasis mode: synthesis prompt */
  synthesisPrompt?: string
}

/**
 * Result from a single agent execution.
 */
export interface AgentResult {
  /** Agent identifier */
  agent: string
  /** Raw output from agent */
  output: string
  /** Confidence score (0-1) if available */
  confidence?: number
  /** Execution time in milliseconds */
  durationMs?: number
  /** Additional metadata */
  metadata?: Record<string, unknown>
}

/**
 * Aggregated result from multiple agents.
 */
export interface AggregatedResult {
  /** Final synthesized output */
  output: string
  /** Aggregation mode used */
  mode: AggregationMode['type']
  /** Individual results that were aggregated */
  sources: AgentResult[]
  /** Consensus information if available */
  consensus?: ConsensusInfo
  /** Conflicts detected during aggregation */
  conflicts?: ConflictInfo[]
  /** Aggregation metadata */
  metadata: {
    totalAgents: number
    successfulAgents: number
    averageConfidence?: number
    totalDurationMs?: number
  }
}

/**
 * Consensus information across agent results.
 */
export interface ConsensusInfo {
  /** Whether consensus was reached */
  reached: boolean
  /** Consensus strength (0-1) */
  strength: number
  /** Points of agreement */
  agreements: string[]
  /** Number of agents in consensus */
  consensusCount: number
  /** Total number of agents */
  totalCount: number
}

/**
 * Information about a conflict between agent outputs.
 */
export interface ConflictInfo {
  /** Topic or area of conflict */
  topic: string
  /** Conflicting positions */
  positions: Array<{
    agent: string
    stance: string
    confidence?: number
  }>
  /** Suggested resolution if available */
  suggestedResolution?: string
}

/**
 * OASIS Aggregator for multi-agent output synthesis.
 */
export class OASISAggregator {
  /**
   * Aggregate multiple agent results into a single output.
   */
  async aggregate(
    results: AgentResult[],
    mode: AggregationMode
  ): Promise<AggregatedResult> {
    const successfulResults = results.filter((r) => r.output)

    const baseMetadata = {
      totalAgents: results.length,
      successfulAgents: successfulResults.length,
      averageConfidence: this.calculateAverageConfidence(successfulResults),
      totalDurationMs: this.calculateTotalDuration(successfulResults),
    }

    switch (mode.type) {
      case 'concatenate':
        return this.concatenateResults(successfulResults, baseMetadata)
      case 'vote':
        return this.voteResults(successfulResults, baseMetadata)
      case 'weighted':
        return this.weightedResults(successfulResults, mode.weightFn, baseMetadata)
      case 'oasis':
        return this.oasisSynthesize(successfulResults, mode.synthesisPrompt, baseMetadata)
      default:
        return this.concatenateResults(successfulResults, baseMetadata)
    }
  }

  /**
   * Detect consensus across agent results.
   */
  async detectConsensus(results: AgentResult[]): Promise<ConsensusInfo> {
    if (results.length === 0) {
      return {
        reached: false,
        strength: 0,
        agreements: [],
        consensusCount: 0,
        totalCount: 0,
      }
    }

    if (results.length === 1) {
      return {
        reached: true,
        strength: 1,
        agreements: ['Single agent - full consensus'],
        consensusCount: 1,
        totalCount: 1,
      }
    }

    // Extract key points from each result
    const keyPoints = results.map((r) => this.extractKeyPoints(r.output))

    // Find common points
    const commonPoints = this.findCommonPoints(keyPoints)

    // Calculate consensus strength
    const strength = commonPoints.length > 0
      ? commonPoints.length / Math.max(...keyPoints.map((kp) => kp.length))
      : 0

    return {
      reached: strength >= 0.5,
      strength,
      agreements: commonPoints,
      consensusCount: results.filter((_, i) =>
        commonPoints.some((cp) => keyPoints[i].includes(cp))
      ).length,
      totalCount: results.length,
    }
  }

  /**
   * Identify conflicts between agent outputs.
   */
  async identifyConflicts(results: AgentResult[]): Promise<ConflictInfo[]> {
    if (results.length < 2) {
      return []
    }

    const conflicts: ConflictInfo[] = []

    // Compare each pair of results
    for (let i = 0; i < results.length; i++) {
      for (let j = i + 1; j < results.length; j++) {
        const pairConflicts = this.detectPairConflicts(results[i], results[j])
        conflicts.push(...pairConflicts)
      }
    }

    // Merge conflicts on the same topic
    return this.mergeConflicts(conflicts)
  }

  // Private helper methods

  private calculateAverageConfidence(results: AgentResult[]): number | undefined {
    const withConfidence = results.filter((r) => r.confidence !== undefined)
    if (withConfidence.length === 0) return undefined
    return (
      withConfidence.reduce((sum, r) => sum + (r.confidence || 0), 0) /
      withConfidence.length
    )
  }

  private calculateTotalDuration(results: AgentResult[]): number | undefined {
    const withDuration = results.filter((r) => r.durationMs !== undefined)
    if (withDuration.length === 0) return undefined
    return Math.max(...withDuration.map((r) => r.durationMs || 0))
  }

  private concatenateResults(
    results: AgentResult[],
    metadata: AggregatedResult['metadata']
  ): AggregatedResult {
    const output = results
      .map((r) => `## ${r.agent}\n\n${r.output}`)
      .join('\n\n---\n\n')

    return {
      output,
      mode: 'concatenate',
      sources: results,
      metadata,
    }
  }

  private voteResults(
    results: AgentResult[],
    metadata: AggregatedResult['metadata']
  ): AggregatedResult {
    // For voting, we need discrete choices - use first line as vote
    const votes = new Map<string, number>()
    for (const result of results) {
      const vote = result.output.split('\n')[0].trim()
      votes.set(vote, (votes.get(vote) || 0) + 1)
    }

    // Find majority
    let maxVotes = 0
    let winner = ''
    for (const [vote, count] of votes.entries()) {
      if (count > maxVotes) {
        maxVotes = count
        winner = vote
      }
    }

    return {
      output: winner,
      mode: 'vote',
      sources: results,
      metadata: {
        ...metadata,
        voteDistribution: Object.fromEntries(votes),
        majorityPercentage: maxVotes / results.length,
      } as AggregatedResult['metadata'],
    }
  }

  private weightedResults(
    results: AgentResult[],
    weightFn: ((result: AgentResult) => number) | undefined,
    metadata: AggregatedResult['metadata']
  ): AggregatedResult {
    // Default weight is confidence, or 1 if no confidence
    const defaultWeightFn = (r: AgentResult) => r.confidence ?? 1

    const fn = weightFn || defaultWeightFn
    const weights = results.map(fn)
    const totalWeight = weights.reduce((a, b) => a + b, 0)

    if (totalWeight === 0) {
      return this.concatenateResults(results, metadata)
    }

    // Normalize weights
    const normalizedWeights = weights.map((w) => w / totalWeight)

    // Build weighted output (highest weight first)
    const indexed = results.map((r, i) => ({ result: r, weight: normalizedWeights[i] }))
    indexed.sort((a, b) => b.weight - a.weight)

    const output = indexed
      .map(({ result, weight }) =>
        `## ${result.agent} (weight: ${(weight * 100).toFixed(1)}%)\n\n${result.output}`
      )
      .join('\n\n---\n\n')

    return {
      output,
      mode: 'weighted',
      sources: results,
      metadata: {
        ...metadata,
        weights: Object.fromEntries(
          results.map((r, i) => [r.agent, normalizedWeights[i]])
        ),
      } as AggregatedResult['metadata'],
    }
  }

  private async oasisSynthesize(
    results: AgentResult[],
    synthesisPrompt: string | undefined,
    metadata: AggregatedResult['metadata']
  ): Promise<AggregatedResult> {
    // Full OASIS synthesis with consensus detection and conflict resolution
    const consensus = await this.detectConsensus(results)
    const conflicts = await this.identifyConflicts(results)

    // Build synthesis context
    let output: string

    if (consensus.reached && conflicts.length === 0) {
      // Strong consensus - use agreement points
      output = `## Consensus Synthesis\n\n`
      output += `**Consensus Strength:** ${(consensus.strength * 100).toFixed(0)}%\n\n`
      output += `### Key Agreements\n\n`
      output += consensus.agreements.map((a) => `- ${a}`).join('\n')
      output += `\n\n### Detailed Analysis\n\n`
      output += results.map((r) => `#### ${r.agent}\n${r.output}`).join('\n\n')
    } else if (conflicts.length > 0) {
      // Conflicts present - highlight them
      output = `## Analysis with Conflicts\n\n`
      output += `**Consensus Strength:** ${(consensus.strength * 100).toFixed(0)}%\n\n`
      output += `### Conflicts Identified\n\n`
      for (const conflict of conflicts) {
        output += `#### ${conflict.topic}\n`
        for (const pos of conflict.positions) {
          output += `- **${pos.agent}**: ${pos.stance}\n`
        }
        if (conflict.suggestedResolution) {
          output += `- *Suggested Resolution*: ${conflict.suggestedResolution}\n`
        }
        output += '\n'
      }
      output += `### Individual Perspectives\n\n`
      output += results.map((r) => `#### ${r.agent}\n${r.output}`).join('\n\n')
    } else {
      // No clear consensus or conflicts - synthesize
      output = `## Synthesized Analysis\n\n`
      output += results.map((r) => `### ${r.agent}\n${r.output}`).join('\n\n')
    }

    return {
      output,
      mode: 'oasis',
      sources: results,
      consensus,
      conflicts,
      metadata,
    }
  }

  private extractKeyPoints(output: string): string[] {
    // Simple extraction: split by newlines, filter bullets/numbered items
    const lines = output.split('\n')
    const keyPoints: string[] = []

    for (const line of lines) {
      const trimmed = line.trim()
      // Look for bullet points, numbered lists, or headers
      if (/^[-*•]\s/.test(trimmed) || /^\d+\.\s/.test(trimmed) || /^#+\s/.test(trimmed)) {
        const point = trimmed.replace(/^[-*•\d.#]+\s*/, '').trim()
        if (point.length > 10 && point.length < 200) {
          keyPoints.push(point.toLowerCase())
        }
      }
    }

    return keyPoints
  }

  private findCommonPoints(keyPointsPerAgent: string[][]): string[] {
    if (keyPointsPerAgent.length === 0) return []
    if (keyPointsPerAgent.length === 1) return keyPointsPerAgent[0]

    // Find points that appear in at least half of the agents
    const threshold = Math.ceil(keyPointsPerAgent.length / 2)
    const pointCounts = new Map<string, number>()

    for (const points of keyPointsPerAgent) {
      // Use Set to count each point once per agent
      const uniquePoints = new Set(points)
      for (const point of uniquePoints) {
        pointCounts.set(point, (pointCounts.get(point) || 0) + 1)
      }
    }

    const common: string[] = []
    for (const [point, count] of pointCounts.entries()) {
      if (count >= threshold) {
        common.push(point)
      }
    }

    return common
  }

  private detectPairConflicts(a: AgentResult, b: AgentResult): ConflictInfo[] {
    const conflicts: ConflictInfo[] = []

    // Simple heuristic: look for opposing sentiment indicators
    const negatives = ['not', "don't", 'avoid', 'against', 'shouldn\'t', 'wrong', 'bad', 'no']
    const positives = ['should', 'recommend', 'suggest', 'good', 'best', 'correct', 'yes']

    const aWords = a.output.toLowerCase().split(/\s+/)
    const bWords = b.output.toLowerCase().split(/\s+/)

    const aHasNeg = negatives.some((n) => aWords.includes(n))
    const aHasPos = positives.some((p) => aWords.includes(p))
    const bHasNeg = negatives.some((n) => bWords.includes(n))
    const bHasPos = positives.some((p) => bWords.includes(p))

    // If one is positive and other is negative
    if ((aHasPos && bHasNeg && !aHasNeg && !bHasPos) ||
        (aHasNeg && bHasPos && !aHasPos && !bHasNeg)) {
      conflicts.push({
        topic: 'Overall Sentiment',
        positions: [
          { agent: a.agent, stance: aHasPos ? 'Positive' : 'Negative', confidence: a.confidence },
          { agent: b.agent, stance: bHasPos ? 'Positive' : 'Negative', confidence: b.confidence },
        ],
      })
    }

    return conflicts
  }

  private mergeConflicts(conflicts: ConflictInfo[]): ConflictInfo[] {
    const byTopic = new Map<string, ConflictInfo>()

    for (const conflict of conflicts) {
      const existing = byTopic.get(conflict.topic)
      if (existing) {
        // Merge positions, avoiding duplicates
        for (const pos of conflict.positions) {
          if (!existing.positions.some((p) => p.agent === pos.agent)) {
            existing.positions.push(pos)
          }
        }
      } else {
        byTopic.set(conflict.topic, { ...conflict })
      }
    }

    return Array.from(byTopic.values())
  }
}

/**
 * Create an OASIS aggregator with default configuration.
 */
export function createAggregator(): OASISAggregator {
  return new OASISAggregator()
}
