/**
 * Gap Detector
 *
 * Detects capability gaps that can be addressed by building new concepts.
 * Sources: task failures, search misses, pattern analysis, user requests.
 *
 * Integrates with the CLOSE framework for decision scoring.
 *
 * @package autonomous/builder
 */

import { Log } from "@/util/log"
import z from "zod"
import { nanoid } from "nanoid"

import { getConceptInventory, type SearchResult } from "./concept-inventory"
import {
  type ConceptType,
  type GapDetectionResult,
  type GapEvidence,
  CONCEPT_METADATA,
  createSelfBuildingCriteria,
} from "./types"
import { calculateCLOSEFromContext, type CLOSEScore } from "../decision/criteria"
import { DecisionEngine, createDecisionEngine } from "../decision/engine"
import type { EvolutionResult, AutonomousProblem } from "../execution/evolution-loop"

const log = Log.create({ service: "autonomous.builder.gap-detector" })

// ============================================================================
// Types
// ============================================================================

/**
 * Task failure information for gap detection
 */
export interface TaskFailure {
  /** Session ID */
  sessionId: string
  /** Problem description */
  description: string
  /** Error message */
  errorMessage?: string
  /** Technology context */
  technology?: string
  /** Attempts made */
  attempts: number
  /** Whether web search was used */
  webSearchUsed: boolean
  /** Whether existing tools were searched */
  toolSearchUsed: boolean
  /** Evolution result if available */
  evolutionResult?: EvolutionResult
}

/**
 * Pattern analysis result
 */
export interface FailurePattern {
  /** Pattern description */
  pattern: string
  /** Number of occurrences */
  occurrences: number
  /** Technology context */
  technology?: string
  /** Whether it requires long-running execution */
  requiresLongRunningExecution: boolean
  /** Whether it requires code execution */
  requiresCodeExecution: boolean
  /** Whether it requires workflow orchestration */
  requiresWorkflowOrchestration: boolean
  /** Whether it's a user-facing capability */
  requiresUserFacingCapability: boolean
  /** Sample error messages */
  sampleErrors: string[]
}

/**
 * Gap detector configuration
 */
export interface GapDetectorConfig {
  /** Minimum confidence to report a gap (0-1) */
  minConfidence: number
  /** Minimum occurrences for pattern detection */
  minPatternOccurrences: number
  /** Maximum gaps to track */
  maxTrackedGaps: number
  /** Enable LLM-based analysis */
  enableLLMAnalysis: boolean
  /** CLOSE threshold for gap acceptance */
  closeThreshold: number
}

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_CONFIG: GapDetectorConfig = {
  minConfidence: 0.6,
  minPatternOccurrences: 2,
  maxTrackedGaps: 100,
  enableLLMAnalysis: true,
  closeThreshold: 5.5,
}

// ============================================================================
// Gap Detector
// ============================================================================

export class GapDetector {
  private config: GapDetectorConfig
  private failureHistory: TaskFailure[] = []
  private detectedGaps: Map<string, GapDetectionResult> = new Map()
  private decisionEngine: DecisionEngine | null = null

  constructor(config: Partial<GapDetectorConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Initialize the decision engine
   */
  async initialize(): Promise<void> {
    this.decisionEngine = createDecisionEngine()
  }

  /**
   * Detect a gap from a task failure
   */
  async detectFromFailure(failure: TaskFailure): Promise<GapDetectionResult | null> {
    log.info("Analyzing failure for capability gaps", {
      sessionId: failure.sessionId,
      descriptionPreview: failure.description.slice(0, 100),
    })

    // Record failure for pattern analysis
    this.failureHistory.push(failure)
    this.trimHistory()

    // Check if we have similar concepts that could help
    const inventory = getConceptInventory()
    const similarConcepts = await inventory.search(failure.description, {
      limit: 5,
      minScore: 0.3,
    })

    // If we found highly similar concepts, no gap detected
    if (similarConcepts.length > 0 && similarConcepts[0].score > 0.8) {
      log.debug("Similar concept exists, no gap detected", {
        similar: similarConcepts[0].concept.identifier,
        score: similarConcepts[0].score,
      })
      return null
    }

    // Infer concept type from failure characteristics
    const conceptType = this.inferConceptType(failure)
    const confidence = this.calculateConfidence(failure, similarConcepts)

    // Skip if confidence too low
    if (confidence < this.config.minConfidence) {
      log.debug("Confidence too low for gap detection", { confidence, threshold: this.config.minConfidence })
      return null
    }

    // Build evidence
    const evidence: GapEvidence[] = [
      {
        type: "task_failure",
        description: `Task failed after ${failure.attempts} attempts`,
        timestamp: Date.now(),
        source: failure.errorMessage,
        metadata: {
          webSearchUsed: failure.webSearchUsed,
          toolSearchUsed: failure.toolSearchUsed,
        },
      },
    ]

    // Calculate CLOSE score
    const closeScore = this.calculateCLOSEScore(conceptType, confidence, failure)

    // Skip if CLOSE score too low
    if (closeScore.total < this.config.closeThreshold) {
      log.debug("CLOSE score too low for gap detection", {
        score: closeScore.total,
        threshold: this.config.closeThreshold,
      })
      return null
    }

    const gap: GapDetectionResult = {
      id: `gap_${nanoid(10)}`,
      type: conceptType,
      description: this.generateGapDescription(failure, conceptType),
      confidence,
      evidence,
      closeScore,
      suggestedName: this.suggestName(failure, conceptType),
      technology: failure.technology,
      detectedAt: Date.now(),
    }

    // Store detected gap
    this.detectedGaps.set(gap.id, gap)
    this.trimGaps()

    log.info("Gap detected from failure", {
      gapId: gap.id,
      type: conceptType,
      confidence,
      closeScore: closeScore.total,
    })

    return gap
  }

  /**
   * Detect a gap from a search query that yielded no results
   */
  async detectFromQuery(query: string, context?: {
    sessionId?: string
    technology?: string
    isUserRequest?: boolean
  }): Promise<GapDetectionResult | null> {
    log.info("Analyzing query for capability gaps", {
      queryPreview: query.slice(0, 100),
    })

    const inventory = getConceptInventory()
    const searchResults = await inventory.search(query, {
      limit: 10,
      minScore: 0.2,
    })

    // If we found good matches, no gap
    if (searchResults.length > 0 && searchResults[0].score > 0.6) {
      return null
    }

    // Analyze the query to determine what kind of concept is needed
    const conceptType = this.inferConceptTypeFromQuery(query)
    const confidence = context?.isUserRequest ? 0.8 : 0.5

    if (confidence < this.config.minConfidence) {
      return null
    }

    const evidence: GapEvidence[] = [
      {
        type: context?.isUserRequest ? "user_request" : "search_miss",
        description: `Search for "${query}" found no matching concepts`,
        timestamp: Date.now(),
        source: query,
        metadata: {
          topResult: searchResults[0]?.concept.identifier,
          topScore: searchResults[0]?.score,
        },
      },
    ]

    const closeScore = this.calculateCLOSEScore(conceptType, confidence, { query })

    if (closeScore.total < this.config.closeThreshold) {
      return null
    }

    const gap: GapDetectionResult = {
      id: `gap_${nanoid(10)}`,
      type: conceptType,
      description: `Missing ${conceptType.toLowerCase()}: ${query}`,
      confidence,
      evidence,
      closeScore,
      suggestedName: this.normalizeForIdentifier(query),
      technology: context?.technology,
      detectedAt: Date.now(),
    }

    this.detectedGaps.set(gap.id, gap)
    this.trimGaps()

    log.info("Gap detected from query", {
      gapId: gap.id,
      type: conceptType,
      confidence,
    })

    return gap
  }

  /**
   * Analyze failure patterns to detect systematic gaps
   */
  async analyzePatterns(): Promise<GapDetectionResult[]> {
    if (this.failureHistory.length < this.config.minPatternOccurrences) {
      return []
    }

    log.info("Analyzing failure patterns", { historySize: this.failureHistory.length })

    const patterns = this.identifyPatterns()
    const gaps: GapDetectionResult[] = []

    for (const pattern of patterns) {
      if (pattern.occurrences < this.config.minPatternOccurrences) {
        continue
      }

      const conceptType = this.inferConceptTypeFromPattern(pattern)
      const confidence = Math.min(0.9, 0.5 + pattern.occurrences * 0.1)

      const evidence: GapEvidence[] = [
        {
          type: "pattern_detection",
          description: `Detected ${pattern.occurrences} occurrences of pattern: ${pattern.pattern}`,
          timestamp: Date.now(),
          metadata: {
            sampleErrors: pattern.sampleErrors.slice(0, 3),
          },
        },
      ]

      const closeScore = this.calculateCLOSEScore(conceptType, confidence, { pattern })

      if (closeScore.total < this.config.closeThreshold) {
        continue
      }

      const gap: GapDetectionResult = {
        id: `gap_${nanoid(10)}`,
        type: conceptType,
        description: `Pattern detected: ${pattern.pattern}`,
        confidence,
        evidence,
        closeScore,
        technology: pattern.technology,
        detectedAt: Date.now(),
      }

      gaps.push(gap)
      this.detectedGaps.set(gap.id, gap)
    }

    this.trimGaps()

    log.info("Pattern analysis complete", { patternsFound: patterns.length, gapsDetected: gaps.length })

    return gaps
  }

  /**
   * Get all detected gaps
   */
  getDetectedGaps(): GapDetectionResult[] {
    return Array.from(this.detectedGaps.values())
  }

  /**
   * Get a specific gap by ID
   */
  getGap(gapId: string): GapDetectionResult | null {
    return this.detectedGaps.get(gapId) ?? null
  }

  /**
   * Clear a resolved gap
   */
  clearGap(gapId: string): void {
    this.detectedGaps.delete(gapId)
  }

  /**
   * Clear all gaps
   */
  clearAllGaps(): void {
    this.detectedGaps.clear()
    this.failureHistory = []
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private inferConceptType(failure: TaskFailure): ConceptType {
    const desc = failure.description.toLowerCase()
    const error = (failure.errorMessage ?? "").toLowerCase()

    // Check for HAND indicators (scheduled, autonomous, recurring)
    if (
      desc.includes("schedule") ||
      desc.includes("cron") ||
      desc.includes("recurring") ||
      desc.includes("autonomous") ||
      desc.includes("background")
    ) {
      return "HAND"
    }

    // Check for WORKFLOW indicators (multi-step, pipeline, orchestrat)
    if (
      desc.includes("pipeline") ||
      desc.includes("orchestrat") ||
      desc.includes("multi-step") ||
      desc.includes("workflow")
    ) {
      return "WORKFLOW"
    }

    // Check for AGENT indicators (specialized persona, assistant, expert)
    if (
      desc.includes("expert") ||
      desc.includes("specialist") ||
      desc.includes("persona") ||
      (desc.includes("agent") && !desc.includes("user agent"))
    ) {
      return "AGENT"
    }

    // Check for SKILL indicators (command, action, user-facing)
    if (
      desc.includes("/") ||
      desc.includes("command") ||
      desc.includes("slash ") ||
      desc.includes("action")
    ) {
      return "SKILL"
    }

    // Check for MEMORY indicators (store, remember, persist)
    if (
      desc.includes("remember") ||
      desc.includes("store") ||
      desc.includes("persist") ||
      desc.includes("schema")
    ) {
      return "MEMORY"
    }

    // Check for PROMPT indicators (template, prompt, instruction)
    if (
      desc.includes("template") ||
      desc.includes("prompt") ||
      desc.includes("instruction")
    ) {
      return "PROMPT"
    }

    // Default to TOOL for general code execution needs
    return "TOOL"
  }

  private inferConceptTypeFromQuery(query: string): ConceptType {
    const queryLower = query.toLowerCase()

    if (queryLower.includes("agent") || queryLower.includes("persona")) return "AGENT"
    if (queryLower.includes("skill") || queryLower.includes("command")) return "SKILL"
    if (queryLower.includes("hand") || queryLower.includes("schedule")) return "HAND"
    if (queryLower.includes("workflow") || queryLower.includes("pipeline")) return "WORKFLOW"
    if (queryLower.includes("memory") || queryLower.includes("schema")) return "MEMORY"
    if (queryLower.includes("prompt") || queryLower.includes("template")) return "PROMPT"

    return "TOOL"
  }

  private inferConceptTypeFromPattern(pattern: FailurePattern): ConceptType {
    if (pattern.requiresLongRunningExecution) return "HAND"
    if (pattern.requiresWorkflowOrchestration) return "WORKFLOW"
    if (pattern.requiresUserFacingCapability) return "SKILL"
    if (pattern.requiresCodeExecution) return "TOOL"

    return "TOOL"
  }

  private calculateConfidence(failure: TaskFailure, similar: SearchResult[]): number {
    let confidence = 0.5

    // Higher confidence if multiple attempts
    if (failure.attempts >= 3) confidence += 0.2

    // Higher confidence if tools were searched
    if (failure.toolSearchUsed) confidence += 0.1

    // Higher confidence if web search was used
    if (failure.webSearchUsed) confidence += 0.1

    // Lower confidence if similar concepts exist
    if (similar.length > 0) {
      const topScore = similar[0].score
      confidence -= topScore * 0.3
    }

    return Math.max(0, Math.min(1, confidence))
  }

  private calculateCLOSEScore(
    conceptType: ConceptType,
    confidence: number,
    _context: unknown,
  ): CLOSEScore {
    const meta = CONCEPT_METADATA[conceptType]

    // Calculate reversibility based on concept type
    const reversibility = meta.riskLevel === "low" ? "fully" : meta.riskLevel === "medium" ? "partially" : "not"

    // Calculate resource margin (higher confidence = more surplus)
    const resourceMargin = Math.round(confidence * 100)

    // Calculate learning value (building new concepts always has learning value)
    const learningValue = 7

    // Calculate risk/reward ratio
    const riskReward = meta.autoApprovable ? 4 : 3

    // Calculate future options
    const futureOptions = meta.riskLevel === "low" ? 9 : meta.riskLevel === "medium" ? 7 : 5

    return calculateCLOSEFromContext({
      reversibility,
      riskReward,
      futureOptions,
      resourceMargin,
      learningValue,
    })
  }

  private generateGapDescription(failure: TaskFailure, conceptType: ConceptType): string {
    const action = this.getActionVerb(conceptType)
    const subject = this.extractSubject(failure.description)

    return `Need to ${action} ${subject}`
  }

  private getActionVerb(conceptType: ConceptType): string {
    switch (conceptType) {
      case "TOOL":
        return "automate"
      case "SKILL":
        return "enable"
      case "AGENT":
        return "specialize in"
      case "HAND":
        return "schedule"
      case "WORKFLOW":
        return "orchestrate"
      case "MEMORY":
        return "remember"
      case "PROMPT":
        return "instruct"
    }
  }

  private extractSubject(description: string): string {
    // Take first sentence or first 50 chars
    const firstSentence = description.split(/[.!?]/)[0]
    return firstSentence.length > 50 ? firstSentence.slice(0, 50) + "..." : firstSentence
  }

  private suggestName(failure: TaskFailure, conceptType: ConceptType): string {
    // Extract key terms from description
    const terms = failure.description
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, "")
      .split(/\s+/)
      .filter((t) => t.length > 3)
      .slice(0, 3)

    if (terms.length === 0) {
      return `${conceptType.toLowerCase()}_${Date.now()}`
    }

    return terms.join("_")
  }

  private normalizeForIdentifier(input: string): string {
    return input
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, "")
      .replace(/\s+/g, "_")
      .slice(0, 50)
  }

  private identifyPatterns(): FailurePattern[] {
    const patternMap = new Map<string, {
      count: number
      technology?: string
      errors: string[]
    }>()

    for (const failure of this.failureHistory) {
      // Create pattern key from normalized description
      const key = failure.description
        .toLowerCase()
        .replace(/[^a-z\s]/g, "")
        .split(/\s+/)
        .slice(0, 5)
        .join(" ")

      const existing = patternMap.get(key) ?? { count: 0, errors: [] }
      existing.count++
      if (failure.errorMessage) {
        existing.errors.push(failure.errorMessage)
      }
      if (failure.technology) {
        existing.technology = failure.technology
      }
      patternMap.set(key, existing)
    }

    return Array.from(patternMap.entries()).map(([pattern, data]) => ({
      pattern,
      occurrences: data.count,
      technology: data.technology,
      requiresLongRunningExecution: pattern.includes("schedule") || pattern.includes("cron"),
      requiresCodeExecution: pattern.includes("run") || pattern.includes("execute"),
      requiresWorkflowOrchestration: pattern.includes("pipeline") || pattern.includes("workflow"),
      requiresUserFacingCapability: pattern.includes("command") || pattern.includes("action"),
      sampleErrors: data.errors,
    }))
  }

  private trimHistory(): void {
    while (this.failureHistory.length > this.config.maxTrackedGaps * 10) {
      this.failureHistory.shift()
    }
  }

  private trimGaps(): void {
    if (this.detectedGaps.size <= this.config.maxTrackedGaps) {
      return
    }

    // Remove oldest gaps
    const sorted = Array.from(this.detectedGaps.entries())
      .sort((a, b) => a[1].detectedAt - b[1].detectedAt)

    while (this.detectedGaps.size > this.config.maxTrackedGaps) {
      const oldest = sorted.shift()
      if (oldest) {
        this.detectedGaps.delete(oldest[0])
      }
    }
  }
}

// ============================================================================
// Factory Functions
// ============================================================================

let gapDetectorInstance: GapDetector | null = null

/**
 * Get the global gap detector instance
 */
export function getGapDetector(): GapDetector {
  if (!gapDetectorInstance) {
    gapDetectorInstance = new GapDetector()
  }
  return gapDetectorInstance
}

/**
 * Create a new gap detector instance
 */
export function createGapDetector(config?: Partial<GapDetectorConfig>): GapDetector {
  return new GapDetector(config)
}
