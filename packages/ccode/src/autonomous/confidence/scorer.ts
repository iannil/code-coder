/**
 * Confidence Scoring System
 *
 * Based on the MiroFish OASIS pattern, this module provides multi-dimensional
 * confidence scoring for agent outputs. Unlike the CLOSE framework (which
 * evaluates *decisions*), confidence scoring evaluates *outputs*.
 *
 * # Dimensions
 *
 * 1. **Factual Accuracy**: Are claims verifiable? Based on evidence?
 * 2. **Completeness**: Does it answer all aspects of the query?
 * 3. **Coherence**: Is the logic internally consistent?
 * 4. **Relevance**: Is the output on-topic and useful?
 *
 * # Design Principles
 *
 * This is a **non-deterministic** task (requires judgment), so it's
 * implemented entirely in TypeScript with potential LLM assistance.
 *
 * @example
 * ```typescript
 * import { ConfidenceScorer, scoreOutput } from './scorer'
 *
 * const scorer = new ConfidenceScorer()
 * const score = await scorer.score(agentOutput, context)
 *
 * console.log(`Overall confidence: ${score.overall}`)
 * console.log(`Sources: ${score.sources.join(', ')}`)
 * console.log(`Uncertainties: ${score.uncertainties.join(', ')}`)
 * ```
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "autonomous.confidence" })

// ============================================================================
// Types
// ============================================================================

/**
 * Multi-dimensional confidence score
 */
export interface ConfidenceScore {
  /** Overall confidence (0.0 - 1.0) */
  overall: number

  /** Dimension-specific scores */
  dimensions: {
    /** Are claims verifiable and evidence-based? */
    factualAccuracy: number
    /** Does it address all aspects of the query? */
    completeness: number
    /** Is the logic internally consistent? */
    coherence: number
    /** Is the output on-topic and useful? */
    relevance: number
  }

  /** Evidence sources cited or used */
  sources: string[]

  /** Identified uncertainties ("known unknowns") */
  uncertainties: string[]

  /** Optional detailed breakdown */
  details?: ConfidenceDetails
}

/**
 * Detailed confidence breakdown for debugging
 */
export interface ConfidenceDetails {
  /** Specific factual claims and their verification status */
  factualClaims: Array<{
    claim: string
    verified: boolean
    source?: string
  }>

  /** Coverage of query aspects */
  queryCoverage: Array<{
    aspect: string
    addressed: boolean
  }>

  /** Logical consistency analysis */
  logicalAnalysis: {
    hasContradictions: boolean
    contradictions: string[]
  }

  /** Relevance signals */
  relevanceSignals: string[]
}

/**
 * Agent output to be scored
 */
export interface AgentOutput {
  /** The output content */
  content: string

  /** Type of output */
  type: "text" | "code" | "analysis" | "decision" | "plan"

  /** Tool calls made to generate this output */
  toolCalls?: Array<{
    tool: string
    args: Record<string, unknown>
    result?: string
  }>

  /** Sources referenced */
  citedSources?: string[]

  /** Duration to generate (ms) */
  generationTime?: number
}

/**
 * Context for confidence scoring
 */
export interface AgentContext {
  /** Original user query or task */
  query: string

  /** Agent type */
  agent: string

  /** Session ID for tracking */
  sessionId?: string

  /** Previous outputs in the conversation */
  previousOutputs?: AgentOutput[]

  /** Expected output characteristics */
  expectations?: {
    requiresSources?: boolean
    expectedLength?: "short" | "medium" | "long"
    expectedType?: "factual" | "analytical" | "creative"
  }
}

// ============================================================================
// Scoring Heuristics
// ============================================================================

/**
 * Patterns indicating factual claims
 */
const FACTUAL_CLAIM_PATTERNS = [
  /according to/i,
  /studies show/i,
  /research indicates/i,
  /data suggests/i,
  /\d+%/,
  /\d+ (percent|million|billion|thousand)/i,
  /in \d{4}/,
  /official(ly)?/i,
]

/**
 * Patterns indicating uncertainty
 */
const UNCERTAINTY_PATTERNS = [
  /may|might|could|possibly/i,
  /I'm not (sure|certain)/i,
  /it's unclear/i,
  /uncertain(ty)?/i,
  /approximately|about|around/i,
  /I don't have (access|information)/i,
  /as of my (knowledge|training)/i,
]

/**
 * Patterns indicating hedging
 */
const HEDGING_PATTERNS = [
  /I think/i,
  /I believe/i,
  /it seems/i,
  /appears to/i,
  /generally/i,
  /typically/i,
  /often/i,
  /usually/i,
]

/**
 * Patterns indicating source citations
 */
const SOURCE_PATTERNS = [
  /\[(\d+|[a-z])\]/i,
  /according to ([^,]+)/i,
  /(source|reference|cited):\s*/i,
  /https?:\/\/[^\s]+/,
  /\((\d{4})\)/,
]

// ============================================================================
// Confidence Scorer
// ============================================================================

/**
 * Configuration for the confidence scorer
 */
export interface ConfidenceScorerConfig {
  /** Weight for factual accuracy (0-1) */
  factualWeight: number
  /** Weight for completeness (0-1) */
  completenessWeight: number
  /** Weight for coherence (0-1) */
  coherenceWeight: number
  /** Weight for relevance (0-1) */
  relevanceWeight: number
  /** Minimum overall score to consider "confident" */
  confidenceThreshold: number
}

const DEFAULT_CONFIG: ConfidenceScorerConfig = {
  factualWeight: 0.3,
  completenessWeight: 0.25,
  coherenceWeight: 0.2,
  relevanceWeight: 0.25,
  confidenceThreshold: 0.7,
}

/**
 * Confidence scorer for agent outputs
 */
export class ConfidenceScorer {
  private config: ConfidenceScorerConfig

  constructor(config: Partial<ConfidenceScorerConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Score an agent output for confidence
   */
  async score(output: AgentOutput, context: AgentContext): Promise<ConfidenceScore> {
    log.debug("Scoring output confidence", {
      agent: context.agent,
      type: output.type,
      contentLength: output.content.length,
    })

    // Calculate dimension scores
    const factualAccuracy = this.scoreFactualAccuracy(output, context)
    const completeness = this.scoreCompleteness(output, context)
    const coherence = this.scoreCoherence(output, context)
    const relevance = this.scoreRelevance(output, context)

    // Extract sources and uncertainties
    const sources = this.extractSources(output)
    const uncertainties = this.extractUncertainties(output)

    // Calculate weighted overall score
    const overall =
      factualAccuracy * this.config.factualWeight +
      completeness * this.config.completenessWeight +
      coherence * this.config.coherenceWeight +
      relevance * this.config.relevanceWeight

    const score: ConfidenceScore = {
      overall: Math.round(overall * 100) / 100,
      dimensions: {
        factualAccuracy: Math.round(factualAccuracy * 100) / 100,
        completeness: Math.round(completeness * 100) / 100,
        coherence: Math.round(coherence * 100) / 100,
        relevance: Math.round(relevance * 100) / 100,
      },
      sources,
      uncertainties,
    }

    log.info("Confidence score calculated", {
      overall: score.overall,
      isConfident: score.overall >= this.config.confidenceThreshold,
    })

    return score
  }

  /**
   * Score factual accuracy based on evidence and citations
   */
  private scoreFactualAccuracy(output: AgentOutput, context: AgentContext): number {
    const content = output.content
    let score = 0.5 // Start neutral

    // Check for factual claims
    const hasClaims = FACTUAL_CLAIM_PATTERNS.some((p) => p.test(content))

    // Check for source citations
    const sources = this.extractSources(output)
    const hasSources = sources.length > 0

    // Check for tool calls (evidence gathering)
    const hasToolEvidence = (output.toolCalls?.length ?? 0) > 0

    // Adjust score based on evidence
    if (hasClaims) {
      if (hasSources || hasToolEvidence) {
        score += 0.3 // Claims backed by evidence
      } else {
        score -= 0.2 // Unsupported claims
      }
    }

    // Bonus for explicit citations
    if (hasSources) {
      score += 0.1 * Math.min(sources.length, 3)
    }

    // Penalty for hedging without explanation
    const hedgingCount = HEDGING_PATTERNS.filter((p) => p.test(content)).length
    score -= 0.05 * Math.min(hedgingCount, 3)

    return Math.max(0, Math.min(1, score))
  }

  /**
   * Score completeness based on query coverage
   */
  private scoreCompleteness(output: AgentOutput, context: AgentContext): number {
    const content = output.content.toLowerCase()
    const query = context.query.toLowerCase()
    let score = 0.5

    // Check if key terms from query appear in output
    const queryTerms = this.extractKeyTerms(query)
    const coveredTerms = queryTerms.filter((term) => content.includes(term))
    const coverage = queryTerms.length > 0 ? coveredTerms.length / queryTerms.length : 0.5

    score = 0.3 + coverage * 0.4

    // Adjust based on output length vs expectations
    const expectedLength = context.expectations?.expectedLength ?? "medium"
    const actualLength = content.length

    const lengthRanges = {
      short: { min: 50, ideal: 200, max: 500 },
      medium: { min: 200, ideal: 800, max: 2000 },
      long: { min: 500, ideal: 2000, max: 5000 },
    }

    const range = lengthRanges[expectedLength]
    if (actualLength < range.min) {
      score -= 0.2 // Too short
    } else if (actualLength > range.max) {
      score -= 0.1 // Too verbose
    } else if (actualLength >= range.ideal * 0.5 && actualLength <= range.ideal * 1.5) {
      score += 0.1 // Good length
    }

    // Check for structured responses (lists, sections)
    if (/^#{1,3}\s/m.test(content) || /^\s*[-*]\s/m.test(content)) {
      score += 0.1 // Structured content
    }

    return Math.max(0, Math.min(1, score))
  }

  /**
   * Score coherence based on logical consistency
   */
  private scoreCoherence(output: AgentOutput, context: AgentContext): number {
    const content = output.content
    let score = 0.7 // Start optimistic

    // Check for contradictory patterns
    const contradictionPatterns = [
      /however.*but/i,
      /although.*nevertheless/i,
      /on one hand.*on the other hand/i,
    ]

    // Contradictions in exposition are fine, but excessive hedging suggests confusion
    const hedgingCount = HEDGING_PATTERNS.filter((p) => p.test(content)).length
    const uncertaintyCount = UNCERTAINTY_PATTERNS.filter((p) => p.test(content)).length

    // Excessive hedging suggests low coherence
    if (hedgingCount > 5) {
      score -= 0.15
    }

    // Some uncertainty is good (epistemic humility), too much suggests confusion
    if (uncertaintyCount > 5) {
      score -= 0.1
    } else if (uncertaintyCount >= 1 && uncertaintyCount <= 3) {
      score += 0.05 // Appropriate epistemic humility
    }

    // Check for logical connectors (good for coherence)
    const logicalConnectors = [
      /therefore/i,
      /because/i,
      /since/i,
      /thus/i,
      /as a result/i,
      /consequently/i,
      /this means/i,
    ]
    const hasLogicalFlow = logicalConnectors.some((p) => p.test(content))
    if (hasLogicalFlow) {
      score += 0.1
    }

    // Check for sentence structure variety
    const sentences = content.split(/[.!?]+/).filter((s) => s.trim().length > 0)
    if (sentences.length > 3) {
      const avgLength = sentences.reduce((a, s) => a + s.length, 0) / sentences.length
      const variance =
        sentences.reduce((a, s) => a + Math.abs(s.length - avgLength), 0) / sentences.length

      // Good variance = variety in sentence structure
      if (variance > 20 && variance < 60) {
        score += 0.05
      }
    }

    return Math.max(0, Math.min(1, score))
  }

  /**
   * Score relevance based on topicality
   */
  private scoreRelevance(output: AgentOutput, context: AgentContext): number {
    const content = output.content.toLowerCase()
    const query = context.query.toLowerCase()
    let score = 0.5

    // Direct keyword matching
    const queryTerms = this.extractKeyTerms(query)
    const contentTerms = this.extractKeyTerms(content)

    const overlap = queryTerms.filter((t) => contentTerms.includes(t))
    const overlapRatio = queryTerms.length > 0 ? overlap.length / queryTerms.length : 0

    score = 0.3 + overlapRatio * 0.4

    // Check for off-topic markers
    const offTopicPatterns = [
      /this is outside (my|the) scope/i,
      /I cannot help with/i,
      /not related to/i,
      /tangentially/i,
    ]
    if (offTopicPatterns.some((p) => p.test(content))) {
      score -= 0.2
    }

    // Bonus for actionable content (relevant outputs tend to be actionable)
    const actionPatterns = [
      /you (can|should|could)/i,
      /to do this/i,
      /here's how/i,
      /follow these steps/i,
      /the solution is/i,
    ]
    if (actionPatterns.some((p) => p.test(content))) {
      score += 0.15
    }

    // Penalty for excessive meta-commentary
    const metaPatterns = [/as an AI/i, /I'm a language model/i, /my training/i]
    const metaCount = metaPatterns.filter((p) => p.test(content)).length
    score -= 0.1 * metaCount

    return Math.max(0, Math.min(1, score))
  }

  /**
   * Extract key terms from text
   */
  private extractKeyTerms(text: string): string[] {
    // Remove common words and extract significant terms
    const stopWords = new Set([
      "the",
      "a",
      "an",
      "is",
      "are",
      "was",
      "were",
      "be",
      "been",
      "being",
      "have",
      "has",
      "had",
      "do",
      "does",
      "did",
      "will",
      "would",
      "could",
      "should",
      "may",
      "might",
      "can",
      "this",
      "that",
      "these",
      "those",
      "i",
      "you",
      "he",
      "she",
      "it",
      "we",
      "they",
      "what",
      "which",
      "who",
      "when",
      "where",
      "why",
      "how",
      "all",
      "each",
      "every",
      "both",
      "few",
      "more",
      "most",
      "other",
      "some",
      "such",
      "no",
      "not",
      "only",
      "same",
      "so",
      "than",
      "too",
      "very",
      "just",
      "and",
      "but",
      "if",
      "or",
      "as",
      "of",
      "at",
      "by",
      "for",
      "with",
      "about",
      "against",
      "between",
      "into",
      "through",
      "during",
      "before",
      "after",
      "above",
      "below",
      "to",
      "from",
      "up",
      "down",
      "in",
      "out",
      "on",
      "off",
      "over",
      "under",
      "again",
      "further",
      "then",
      "once",
    ])

    return text
      .split(/\W+/)
      .filter((word) => word.length > 2 && !stopWords.has(word.toLowerCase()))
      .map((word) => word.toLowerCase())
      .filter((word, index, arr) => arr.indexOf(word) === index) // Unique
      .slice(0, 20) // Limit to top 20
  }

  /**
   * Extract sources from output
   */
  private extractSources(output: AgentOutput): string[] {
    const sources: string[] = []

    // Add explicitly cited sources
    if (output.citedSources) {
      sources.push(...output.citedSources)
    }

    // Extract URLs
    const urlPattern = /https?:\/\/[^\s)]+/g
    const urls = output.content.match(urlPattern) ?? []
    sources.push(...urls)

    // Extract "according to" sources
    const accordingToPattern = /according to ([^,.\n]+)/gi
    let match
    while ((match = accordingToPattern.exec(output.content)) !== null) {
      sources.push(match[1].trim())
    }

    // Extract tool-based sources
    if (output.toolCalls) {
      for (const call of output.toolCalls) {
        if (call.tool === "WebFetch" || call.tool === "WebSearch") {
          const url = call.args.url as string | undefined
          if (url) sources.push(url)
        }
        if (call.tool === "Read") {
          const path = call.args.file_path as string | undefined
          if (path) sources.push(`file:${path}`)
        }
      }
    }

    // Deduplicate
    return [...new Set(sources)]
  }

  /**
   * Extract uncertainties from output
   */
  private extractUncertainties(output: AgentOutput): string[] {
    const uncertainties: string[] = []
    const content = output.content

    // Find sentences containing uncertainty patterns
    const sentences = content.split(/[.!?]+/)

    for (const sentence of sentences) {
      for (const pattern of UNCERTAINTY_PATTERNS) {
        if (pattern.test(sentence)) {
          const trimmed = sentence.trim()
          if (trimmed.length > 10 && trimmed.length < 200) {
            uncertainties.push(trimmed)
            break
          }
        }
      }
    }

    // Limit to most relevant uncertainties
    return uncertainties.slice(0, 5)
  }

  /**
   * Check if the score indicates confident output
   */
  isConfident(score: ConfidenceScore): boolean {
    return score.overall >= this.config.confidenceThreshold
  }

  /**
   * Get configuration
   */
  getConfig(): ConfidenceScorerConfig {
    return { ...this.config }
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<ConfidenceScorerConfig>): void {
    this.config = { ...this.config, ...config }
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Create a new confidence scorer
 */
export function createConfidenceScorer(
  config?: Partial<ConfidenceScorerConfig>
): ConfidenceScorer {
  return new ConfidenceScorer(config)
}

/**
 * Score an output using the default scorer
 */
export async function scoreOutput(
  output: AgentOutput,
  context: AgentContext
): Promise<ConfidenceScore> {
  const scorer = new ConfidenceScorer()
  return scorer.score(output, context)
}

/**
 * Quick confidence check
 */
export async function isOutputConfident(
  output: AgentOutput,
  context: AgentContext,
  threshold = 0.7
): Promise<boolean> {
  const scorer = new ConfidenceScorer({ confidenceThreshold: threshold })
  const score = await scorer.score(output, context)
  return scorer.isConfident(score)
}

// ============================================================================
// Exports
// ============================================================================

export const Confidence = {
  ConfidenceScorer,
  createConfidenceScorer,
  scoreOutput,
  isOutputConfident,
}
