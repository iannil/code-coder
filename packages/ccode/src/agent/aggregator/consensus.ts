/**
 * Consensus detection utilities for multi-agent aggregation.
 *
 * Provides functions for analyzing agreement, extracting common themes,
 * and identifying dissenting views across multiple agent outputs.
 *
 * ## Design Principle
 *
 * Consensus detection requires **understanding** semantics and context.
 * These utilities provide heuristic-based analysis that can be enhanced
 * with LLM reasoning when needed.
 */

import type { AgentResult, ConsensusInfo, ConflictInfo } from './oasis'

/**
 * Agreement score between agent outputs.
 */
export interface AgreementScore {
  /** Overall agreement score (0-1) */
  score: number
  /** Number of agents that agree */
  agreeingAgents: number
  /** Total number of agents */
  totalAgents: number
  /** Specific areas of agreement */
  agreementAreas: string[]
  /** Breakdown by topic */
  topicScores?: Record<string, number>
}

/**
 * Information about a dissenting view.
 */
export interface DissentInfo {
  /** Agent with the dissenting view */
  agent: string
  /** The dissenting point */
  point: string
  /** How different from consensus (0-1) */
  divergence: number
  /** Suggested explanation for dissent */
  possibleReason?: string
}

/**
 * Analyze agreement level across agent results.
 */
export function analyzeAgreement(results: AgentResult[]): AgreementScore {
  if (results.length === 0) {
    return {
      score: 0,
      agreeingAgents: 0,
      totalAgents: 0,
      agreementAreas: [],
    }
  }

  if (results.length === 1) {
    return {
      score: 1,
      agreeingAgents: 1,
      totalAgents: 1,
      agreementAreas: ['Single agent - full agreement'],
    }
  }

  // Extract themes from each result
  const themes = results.map((r) => extractThemesFromOutput(r.output))

  // Find overlapping themes
  const themeFrequency = new Map<string, number>()
  for (const agentThemes of themes) {
    const unique = new Set(agentThemes)
    for (const theme of unique) {
      themeFrequency.set(theme, (themeFrequency.get(theme) || 0) + 1)
    }
  }

  // Calculate agreement
  const totalUniqueThemes = themeFrequency.size
  const commonThemes: string[] = []

  for (const [theme, count] of themeFrequency.entries()) {
    if (count >= Math.ceil(results.length / 2)) {
      commonThemes.push(theme)
    }
  }

  const score = totalUniqueThemes > 0 ? commonThemes.length / totalUniqueThemes : 0

  // Count agreeing agents (those that share at least one common theme)
  const agreeingAgents = themes.filter((agentThemes) =>
    commonThemes.some((ct) => agentThemes.includes(ct))
  ).length

  return {
    score,
    agreeingAgents,
    totalAgents: results.length,
    agreementAreas: commonThemes,
  }
}

/**
 * Extract common themes across all agent outputs.
 */
export function extractCommonThemes(results: AgentResult[]): string[] {
  if (results.length === 0) return []

  const allThemes = results.map((r) => extractThemesFromOutput(r.output))

  // Count theme frequency
  const frequency = new Map<string, number>()
  for (const themes of allThemes) {
    const unique = new Set(themes)
    for (const theme of unique) {
      frequency.set(theme, (frequency.get(theme) || 0) + 1)
    }
  }

  // Return themes that appear in at least half of results
  const threshold = Math.ceil(results.length / 2)
  const common: string[] = []

  for (const [theme, count] of frequency.entries()) {
    if (count >= threshold) {
      common.push(theme)
    }
  }

  // Sort by frequency (most common first)
  common.sort((a, b) => (frequency.get(b) || 0) - (frequency.get(a) || 0))

  return common
}

/**
 * Identify dissenting views from the consensus.
 */
export function identifyDissentingViews(results: AgentResult[]): DissentInfo[] {
  if (results.length < 2) return []

  const commonThemes = extractCommonThemes(results)
  const dissenters: DissentInfo[] = []

  for (const result of results) {
    const agentThemes = extractThemesFromOutput(result.output)

    // Find themes unique to this agent
    const uniqueThemes = agentThemes.filter(
      (theme) => !commonThemes.includes(theme) && !isRelatedToCommon(theme, commonThemes)
    )

    // Calculate divergence based on unique themes ratio
    const divergence = agentThemes.length > 0
      ? uniqueThemes.length / agentThemes.length
      : 0

    // Report significant dissent
    for (const uniqueTheme of uniqueThemes) {
      if (divergence > 0.3) {
        dissenters.push({
          agent: result.agent,
          point: uniqueTheme,
          divergence,
          possibleReason: determineDissentReason(uniqueTheme, commonThemes),
        })
      }
    }
  }

  return dissenters
}

/**
 * Build consensus from multiple agent results.
 */
export function buildConsensus(results: AgentResult[]): ConsensusInfo {
  const agreement = analyzeAgreement(results)

  return {
    reached: agreement.score >= 0.5,
    strength: agreement.score,
    agreements: agreement.agreementAreas,
    consensusCount: agreement.agreeingAgents,
    totalCount: agreement.totalAgents,
  }
}

/**
 * Resolve conflicts between agent outputs.
 */
export function resolveConflicts(conflicts: ConflictInfo[]): ConflictInfo[] {
  return conflicts.map((conflict) => ({
    ...conflict,
    suggestedResolution: suggestResolution(conflict),
  }))
}

// Private helper functions

function extractThemesFromOutput(output: string): string[] {
  const themes: string[] = []
  const lines = output.split('\n')

  for (const line of lines) {
    const trimmed = line.trim().toLowerCase()

    // Skip empty lines and very short content
    if (trimmed.length < 10) continue

    // Extract from headers
    if (/^#+\s/.test(trimmed)) {
      const header = trimmed.replace(/^#+\s*/, '')
      if (header.length > 3) themes.push(normalizeTheme(header))
      continue
    }

    // Extract from bullet points
    if (/^[-*•]\s/.test(trimmed)) {
      const point = trimmed.replace(/^[-*•]\s*/, '')
      if (point.length > 10 && point.length < 150) {
        themes.push(normalizeTheme(point))
      }
      continue
    }

    // Extract from numbered lists
    if (/^\d+\.\s/.test(trimmed)) {
      const point = trimmed.replace(/^\d+\.\s*/, '')
      if (point.length > 10 && point.length < 150) {
        themes.push(normalizeTheme(point))
      }
      continue
    }

    // Extract key phrases from sentences
    const keyPhrases = extractKeyPhrases(trimmed)
    themes.push(...keyPhrases)
  }

  return themes
}

function normalizeTheme(theme: string): string {
  return theme
    .toLowerCase()
    .replace(/[^\w\s]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

function extractKeyPhrases(text: string): string[] {
  const phrases: string[] = []

  // Look for common phrase patterns
  const patterns = [
    /(?:should|must|need to|recommend|suggest)\s+([^,.]+)/gi,
    /(?:important|key|critical|essential)\s+(?:to|that|is)\s+([^,.]+)/gi,
    /(?:because|since|due to)\s+([^,.]+)/gi,
  ]

  for (const pattern of patterns) {
    let match: RegExpExecArray | null = null
    const regex = new RegExp(pattern.source, pattern.flags)
    while ((match = regex.exec(text)) !== null) {
      if (match[1] && match[1].length > 5 && match[1].length < 100) {
        phrases.push(normalizeTheme(match[1]))
      }
    }
  }

  return phrases
}

function isRelatedToCommon(theme: string, commonThemes: string[]): boolean {
  // Check if theme shares significant words with common themes
  const themeWords = theme.split(' ').filter((w) => w.length > 3)

  for (const common of commonThemes) {
    const commonWords = common.split(' ').filter((w) => w.length > 3)
    const overlap = themeWords.filter((w) => commonWords.includes(w))

    // If more than 50% of words overlap, consider related
    if (overlap.length >= Math.ceil(themeWords.length / 2)) {
      return true
    }
  }

  return false
}

function determineDissentReason(
  uniqueTheme: string,
  commonThemes: string[]
): string | undefined {
  // Heuristic reasons for dissent
  const negativeIndicators = ['not', 'avoid', 'against', 'shouldn\'t', 'wrong', 'risk']
  const hasNegative = negativeIndicators.some((n) => uniqueTheme.includes(n))

  if (hasNegative) {
    return 'Expressing concern or caution not shared by other agents'
  }

  // Check if it's a specialized perspective
  const specializedTerms = ['technically', 'specifically', 'in this case', 'edge case']
  const hasSpecialized = specializedTerms.some((t) => uniqueTheme.includes(t))

  if (hasSpecialized) {
    return 'Providing specialized or edge-case analysis'
  }

  // Default: different perspective
  return 'Alternative perspective or approach'
}

function suggestResolution(conflict: ConflictInfo): string {
  const positions = conflict.positions

  // If one position has higher confidence, suggest that
  const withConfidence = positions.filter((p) => p.confidence !== undefined)
  if (withConfidence.length > 1) {
    const highest = withConfidence.reduce((a, b) =>
      (a.confidence || 0) > (b.confidence || 0) ? a : b
    )
    if ((highest.confidence || 0) > 0.7) {
      return `Consider ${highest.agent}'s position due to higher confidence (${((highest.confidence || 0) * 100).toFixed(0)}%)`
    }
  }

  // If equal confidence, suggest synthesis
  if (positions.length === 2) {
    return `Consider synthesizing both perspectives: ${positions[0].agent} emphasizes ${positions[0].stance}, while ${positions[1].agent} focuses on ${positions[1].stance}`
  }

  // Multiple positions: suggest seeking additional input
  return 'Consider gathering additional perspectives or conducting further analysis'
}
