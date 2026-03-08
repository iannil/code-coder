/**
 * Consensus detection for forum discussions.
 *
 * Provides specialized consensus detection for structured multi-agent
 * forum discussions, building on the general aggregator consensus utilities.
 *
 * ## Design Principle
 *
 * Forum consensus detection requires understanding the flow of discussion
 * across rounds and tracking how positions evolve. This is an uncertain
 * task that benefits from heuristic analysis with optional LLM enhancement.
 */

import type { ForumTurn, ForumConsensusResult, ConsensusPoint } from './moderator'

/**
 * Position tracking for a speaker across rounds.
 */
export interface PositionTracker {
  /** Speaker identifier */
  speaker: string
  /** Positions by round */
  positionsByRound: Map<number, string[]>
  /** Whether position has shifted */
  hasShifted: boolean
  /** Direction of shift if any */
  shiftDirection?: 'converging' | 'diverging' | 'stable'
}

/**
 * Analysis of how consensus evolved across rounds.
 */
export interface ConsensusEvolution {
  /** Consensus state after each round */
  byRound: Array<{
    round: number
    consensusStrength: number
    consensusPoints: number
    dissentingCount: number
  }>
  /** Overall trend */
  trend: 'converging' | 'diverging' | 'stable' | 'fluctuating'
  /** Rounds where significant changes occurred */
  pivotRounds: number[]
}

/**
 * Track positions for all speakers across the discussion.
 */
export function trackPositions(history: ForumTurn[]): Map<string, PositionTracker> {
  const trackers = new Map<string, PositionTracker>()

  for (const turn of history) {
    let tracker = trackers.get(turn.speaker)
    if (!tracker) {
      tracker = {
        speaker: turn.speaker,
        positionsByRound: new Map(),
        hasShifted: false,
      }
      trackers.set(turn.speaker, tracker)
    }

    const positions = extractPositions(turn.content)
    tracker.positionsByRound.set(turn.round, positions)
  }

  // Analyze shifts
  for (const tracker of trackers.values()) {
    tracker.shiftDirection = analyzeShiftDirection(tracker)
    tracker.hasShifted = tracker.shiftDirection !== 'stable'
  }

  return trackers
}

/**
 * Analyze how consensus evolved across rounds.
 */
export function analyzeConsensusEvolution(
  history: ForumTurn[],
  participants: string[]
): ConsensusEvolution {
  const byRound: ConsensusEvolution['byRound'] = []
  const maxRound = Math.max(...history.map((t) => t.round), 0)

  let previousStrength = 0
  const pivotRounds: number[] = []

  for (let round = 1; round <= maxRound; round++) {
    const roundTurns = history.filter((t) => t.round <= round)
    const result = calculateConsensusForTurns(roundTurns, participants)

    const strengthChange = Math.abs(result.strength - previousStrength)
    if (strengthChange > 0.2) {
      pivotRounds.push(round)
    }

    byRound.push({
      round,
      consensusStrength: result.strength,
      consensusPoints: result.points,
      dissentingCount: result.dissentingCount,
    })

    previousStrength = result.strength
  }

  // Determine overall trend
  const trend = determineTrend(byRound)

  return {
    byRound,
    trend,
    pivotRounds,
  }
}

/**
 * Identify key turning points in the discussion.
 */
export function identifyTurningPoints(history: ForumTurn[]): Array<{
  turn: ForumTurn
  reason: string
  impact: 'high' | 'medium' | 'low'
}> {
  const turningPoints: Array<{
    turn: ForumTurn
    reason: string
    impact: 'high' | 'medium' | 'low'
  }> = []

  // Identify turns that introduce new topics or shift consensus
  const topicsByTurn = history.map((turn) => ({
    turn,
    topics: extractTopics(turn.content),
  }))

  const allTopicsBefore = new Set<string>()

  for (const { turn, topics } of topicsByTurn) {
    const newTopics = topics.filter((t) => !allTopicsBefore.has(t))

    if (newTopics.length > 0) {
      turningPoints.push({
        turn,
        reason: `Introduced new topic(s): ${newTopics.slice(0, 2).join(', ')}`,
        impact: newTopics.length > 2 ? 'high' : 'medium',
      })
    }

    // Check for concession language
    if (hasConcessionLanguage(turn.content)) {
      turningPoints.push({
        turn,
        reason: 'Made a concession or changed position',
        impact: 'high',
      })
    }

    // Check for strong disagreement
    if (hasStrongDisagreement(turn.content)) {
      turningPoints.push({
        turn,
        reason: 'Expressed strong disagreement',
        impact: 'medium',
      })
    }

    topics.forEach((t) => allTopicsBefore.add(t))
  }

  return turningPoints
}

/**
 * Calculate weighted consensus based on speaker expertise/confidence.
 */
export function calculateWeightedConsensus(
  history: ForumTurn[],
  weights: Map<string, number>
): ForumConsensusResult {
  const positionsBySpeaker = new Map<string, string[]>()

  // Extract positions for each speaker
  for (const turn of history) {
    const existing = positionsBySpeaker.get(turn.speaker) || []
    positionsBySpeaker.set(turn.speaker, [
      ...existing,
      ...extractPositions(turn.content),
    ])
  }

  // Count weighted support for each position
  const positionSupport = new Map<string, { weight: number; supporters: string[] }>()

  for (const [speaker, positions] of positionsBySpeaker.entries()) {
    const speakerWeight = weights.get(speaker) || 1

    for (const position of positions) {
      const normalized = normalizePosition(position)
      const existing = positionSupport.get(normalized) || {
        weight: 0,
        supporters: [],
      }

      if (!existing.supporters.includes(speaker)) {
        existing.weight += speakerWeight
        existing.supporters.push(speaker)
        positionSupport.set(normalized, existing)
      }
    }
  }

  // Calculate total weight
  let totalWeight = 0
  for (const speaker of positionsBySpeaker.keys()) {
    totalWeight += weights.get(speaker) || 1
  }

  // Find consensus points
  const consensusThreshold = totalWeight / 2
  const points: ConsensusPoint[] = []

  for (const [position, support] of positionSupport.entries()) {
    if (support.weight >= consensusThreshold) {
      points.push({
        statement: position,
        supporters: support.supporters,
        strength: support.weight / totalWeight,
      })
    }
  }

  // Sort by strength
  points.sort((a, b) => b.strength - a.strength)

  // Identify dissenting speakers
  const consensusSupporters = new Set(points.flatMap((p) => p.supporters))
  const dissenting = Array.from(positionsBySpeaker.keys()).filter(
    (s) => !consensusSupporters.has(s)
  )

  // Calculate overall confidence
  const confidence =
    points.length > 0
      ? points.reduce((sum, p) => sum + p.strength, 0) / points.length
      : 0

  return {
    reached: confidence >= 0.5,
    confidence,
    points,
    dissenting,
    summary: buildSummary(points, dissenting, confidence),
  }
}

// Private helper functions

function extractPositions(content: string): string[] {
  const positions: string[] = []
  const lines = content.split('\n')

  for (const line of lines) {
    const trimmed = line.trim()

    // Look for assertion patterns
    if (
      /^(I believe|I think|We should|The best|This is|In my view)/i.test(trimmed) ||
      /^[-*•]\s/.test(trimmed) ||
      /^\d+\.\s/.test(trimmed)
    ) {
      const cleaned = trimmed.replace(/^[-*•\d.]\s*/, '')
      if (cleaned.length > 15 && cleaned.length < 200) {
        positions.push(cleaned)
      }
    }
  }

  return positions
}

function normalizePosition(position: string): string {
  return position
    .toLowerCase()
    .replace(/^(i believe|i think|we should|the best is|this is|in my view)\s*/i, '')
    .replace(/[^\w\s]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

function analyzeShiftDirection(
  tracker: PositionTracker
): 'converging' | 'diverging' | 'stable' {
  const rounds = Array.from(tracker.positionsByRound.keys()).sort()
  if (rounds.length < 2) return 'stable'

  const firstPositions = tracker.positionsByRound.get(rounds[0]) || []
  const lastPositions = tracker.positionsByRound.get(rounds[rounds.length - 1]) || []

  // Check for convergence (using more agreeable language)
  const agreeablePatterns = ['agree', 'accept', 'understand', 'can work with']
  const firstAgreeCount = firstPositions.filter((p) =>
    agreeablePatterns.some((pat) => p.toLowerCase().includes(pat))
  ).length
  const lastAgreeCount = lastPositions.filter((p) =>
    agreeablePatterns.some((pat) => p.toLowerCase().includes(pat))
  ).length

  if (lastAgreeCount > firstAgreeCount) return 'converging'
  if (lastAgreeCount < firstAgreeCount) return 'diverging'
  return 'stable'
}

function calculateConsensusForTurns(
  turns: ForumTurn[],
  participants: string[]
): { strength: number; points: number; dissentingCount: number } {
  const positionsBySpeaker = new Map<string, Set<string>>()

  for (const turn of turns) {
    const positions = extractPositions(turn.content)
    const existing = positionsBySpeaker.get(turn.speaker) || new Set()
    positions.forEach((p) => existing.add(normalizePosition(p)))
    positionsBySpeaker.set(turn.speaker, existing)
  }

  // Count position frequency
  const positionCounts = new Map<string, number>()
  for (const positions of positionsBySpeaker.values()) {
    for (const pos of positions) {
      positionCounts.set(pos, (positionCounts.get(pos) || 0) + 1)
    }
  }

  // Count consensus points
  const threshold = Math.ceil(participants.length / 2)
  let consensusPoints = 0
  for (const count of positionCounts.values()) {
    if (count >= threshold) consensusPoints++
  }

  // Calculate strength
  const strength =
    positionCounts.size > 0 ? consensusPoints / positionCounts.size : 0

  // Count dissenting
  const speakersWithConsensusPositions = new Set<string>()
  for (const [speaker, positions] of positionsBySpeaker.entries()) {
    for (const pos of positions) {
      if ((positionCounts.get(pos) || 0) >= threshold) {
        speakersWithConsensusPositions.add(speaker)
        break
      }
    }
  }
  const dissentingCount = participants.length - speakersWithConsensusPositions.size

  return { strength, points: consensusPoints, dissentingCount }
}

function determineTrend(
  byRound: ConsensusEvolution['byRound']
): ConsensusEvolution['trend'] {
  if (byRound.length < 2) return 'stable'

  const strengths = byRound.map((r) => r.consensusStrength)
  const first = strengths[0]
  const last = strengths[strengths.length - 1]

  // Check for fluctuation
  let changes = 0
  let lastDirection: 'up' | 'down' | null = null
  for (let i = 1; i < strengths.length; i++) {
    const direction = strengths[i] > strengths[i - 1] ? 'up' : 'down'
    if (lastDirection && direction !== lastDirection) changes++
    lastDirection = direction
  }

  if (changes > strengths.length / 2) return 'fluctuating'

  const diff = last - first
  if (diff > 0.15) return 'converging'
  if (diff < -0.15) return 'diverging'
  return 'stable'
}

function extractTopics(content: string): string[] {
  const topics: string[] = []
  const lines = content.split('\n')

  for (const line of lines) {
    const trimmed = line.trim()

    // Look for headers
    if (/^#+\s/.test(trimmed)) {
      topics.push(trimmed.replace(/^#+\s*/, '').toLowerCase())
    }

    // Look for emphasized terms
    const emphMatches = trimmed.match(/\*\*([^*]+)\*\*/g)
    if (emphMatches) {
      for (const match of emphMatches) {
        topics.push(match.replace(/\*+/g, '').toLowerCase())
      }
    }
  }

  return topics
}

function hasConcessionLanguage(content: string): boolean {
  const concessionPatterns = [
    'you make a good point',
    'i agree with',
    'that\'s fair',
    'you\'re right',
    'i can see',
    'i concede',
    'on reflection',
    'actually',
    'i\'ve changed my mind',
  ]

  const lower = content.toLowerCase()
  return concessionPatterns.some((p) => lower.includes(p))
}

function hasStrongDisagreement(content: string): boolean {
  const disagreementPatterns = [
    'strongly disagree',
    'completely wrong',
    'cannot accept',
    'fundamentally flawed',
    'that\'s not accurate',
    'i must object',
    'absolutely not',
  ]

  const lower = content.toLowerCase()
  return disagreementPatterns.some((p) => lower.includes(p))
}

function buildSummary(
  points: ConsensusPoint[],
  dissenting: string[],
  confidence: number
): string {
  if (points.length === 0) {
    return 'No consensus points emerged from the discussion.'
  }

  let summary = `Consensus strength: ${(confidence * 100).toFixed(0)}%\n`
  summary += `${points.length} point(s) of agreement:\n`

  for (const point of points.slice(0, 3)) {
    summary += `- ${point.statement}\n`
  }

  if (dissenting.length > 0) {
    summary += `\nDissenting: ${dissenting.join(', ')}`
  }

  return summary
}
