/**
 * Pattern Recognition
 *
 * Detects emergent patterns from observation streams.
 * Identifies trends, correlations, cycles, and sequences.
 *
 * @module observer/consensus/patterns
 */

import { Log } from "@/util/log"
import type {
  Observation,
  EmergentPattern,
} from "../types"
import { generatePatternId } from "../types"

const log = Log.create({ service: "observer.consensus.patterns" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface PatternConfig {
  /** Minimum observations to form a pattern */
  minObservations: number
  /** Time window for pattern detection (ms) */
  windowMs: number
  /** Minimum confidence for pattern */
  minConfidence: number
  /** Pattern types to detect */
  enabledTypes: EmergentPattern["type"][]
}

interface PatternCandidate {
  type: EmergentPattern["type"]
  observationIds: string[]
  confidence: number
  metadata: Record<string, unknown>
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: PatternConfig = {
  minObservations: 3,
  windowMs: 300000, // 5 minutes
  minConfidence: 0.5,
  enabledTypes: ["trend", "anomaly", "correlation", "cycle", "threshold", "sequence"],
}

// ─────────────────────────────────────────────────────────────────────────────
// Pattern Detector
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Detects patterns in observation streams.
 */
export class PatternDetector {
  private config: PatternConfig
  private activePatterns: Map<string, EmergentPattern> = new Map()
  private patternHistory: EmergentPattern[] = []

  constructor(config: Partial<PatternConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Detect patterns in observations.
   */
  detect(observations: Observation[]): EmergentPattern[] {
    const candidates: PatternCandidate[] = []

    // Filter to window
    const now = Date.now()
    const windowStart = now - this.config.windowMs
    const recent = observations.filter((o) => o.timestamp.getTime() > windowStart)

    if (recent.length < this.config.minObservations) {
      return []
    }

    // Detect different pattern types
    if (this.config.enabledTypes.includes("trend")) {
      candidates.push(...this.detectTrends(recent))
    }

    if (this.config.enabledTypes.includes("correlation")) {
      candidates.push(...this.detectCorrelations(recent))
    }

    if (this.config.enabledTypes.includes("sequence")) {
      candidates.push(...this.detectSequences(recent))
    }

    if (this.config.enabledTypes.includes("threshold")) {
      candidates.push(...this.detectThresholds(recent))
    }

    // Convert candidates to patterns
    const patterns = candidates
      .filter((c) => c.confidence >= this.config.minConfidence)
      .map((c) => this.createPattern(c))

    // Update active patterns
    for (const pattern of patterns) {
      const existing = this.findSimilarPattern(pattern)
      if (existing) {
        // Update existing pattern
        existing.lastSeenAt = new Date()
        existing.observationIds = [
          ...new Set([...existing.observationIds, ...pattern.observationIds]),
        ]
        existing.strength = Math.max(existing.strength, pattern.strength)
      } else {
        // Add new pattern
        this.activePatterns.set(pattern.id, pattern)
        this.patternHistory.push(pattern)
      }
    }

    return patterns
  }

  /**
   * Get active patterns.
   */
  getActive(): EmergentPattern[] {
    return Array.from(this.activePatterns.values())
  }

  /**
   * Get pattern history.
   */
  getHistory(limit?: number): EmergentPattern[] {
    return this.patternHistory.slice(-(limit ?? 100))
  }

  /**
   * Expire old patterns.
   */
  expirePatterns(maxAge: number = this.config.windowMs * 2): string[] {
    const now = Date.now()
    const expired: string[] = []

    for (const [id, pattern] of this.activePatterns.entries()) {
      const age = now - pattern.lastSeenAt.getTime()
      if (age > maxAge) {
        this.activePatterns.delete(id)
        expired.push(id)
      }
    }

    return expired
  }

  /**
   * Clear all patterns.
   */
  clear(): void {
    this.activePatterns.clear()
    this.patternHistory = []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Pattern Detection Algorithms
  // ─────────────────────────────────────────────────────────────────────────────

  private detectTrends(observations: Observation[]): PatternCandidate[] {
    const candidates: PatternCandidate[] = []

    // Group by watcher type
    const byWatcher = this.groupByWatcher(observations)

    for (const [watcherType, obs] of byWatcher.entries()) {
      if (obs.length < this.config.minObservations) continue

      // Sort by time
      const sorted = [...obs].sort(
        (a, b) => a.timestamp.getTime() - b.timestamp.getTime(),
      )

      // Check for increasing/decreasing confidence trend
      const confidences = sorted.map((o) => o.confidence)
      const trend = this.calculateTrend(confidences)

      if (Math.abs(trend.slope) > 0.1) {
        candidates.push({
          type: "trend",
          observationIds: sorted.map((o) => o.id),
          confidence: Math.min(Math.abs(trend.slope) * 2, 1),
          metadata: {
            watcherType,
            direction: trend.slope > 0 ? "increasing" : "decreasing",
            slope: trend.slope,
            r2: trend.r2,
          },
        })
      }
    }

    return candidates
  }

  private detectCorrelations(observations: Observation[]): PatternCandidate[] {
    const candidates: PatternCandidate[] = []

    // Group by observation type
    const byType = this.groupByType(observations)
    const types = Array.from(byType.keys())

    // Check pairs of types for correlation
    for (let i = 0; i < types.length; i++) {
      for (let j = i + 1; j < types.length; j++) {
        const type1 = types[i]
        const type2 = types[j]
        const obs1 = byType.get(type1)!
        const obs2 = byType.get(type2)!

        // Find temporally close observations
        const correlated = this.findTemporalCorrelation(obs1, obs2, 60000) // 1 minute window

        if (correlated.pairs.length >= this.config.minObservations) {
          candidates.push({
            type: "correlation",
            observationIds: correlated.observationIds,
            confidence: correlated.strength,
            metadata: {
              type1,
              type2,
              pairCount: correlated.pairs.length,
              avgTimeDelta: correlated.avgTimeDelta,
            },
          })
        }
      }
    }

    return candidates
  }

  private detectSequences(observations: Observation[]): PatternCandidate[] {
    const candidates: PatternCandidate[] = []

    // Sort by time
    const sorted = [...observations].sort(
      (a, b) => a.timestamp.getTime() - b.timestamp.getTime(),
    )

    // Look for repeating type sequences
    const typeSequence = sorted.map((o) => `${o.watcherType}:${(o as any).type || "unknown"}`)
    const sequences = this.findRepeatingSequences(typeSequence, sorted.map((o) => o.id))

    for (const seq of sequences) {
      if (seq.count >= 2 && seq.pattern.length >= 2) {
        candidates.push({
          type: "sequence",
          observationIds: seq.observationIds,
          confidence: Math.min(seq.count / 3, 1),
          metadata: {
            pattern: seq.pattern,
            repeatCount: seq.count,
            patternLength: seq.pattern.length,
          },
        })
      }
    }

    return candidates
  }

  private detectThresholds(observations: Observation[]): PatternCandidate[] {
    const candidates: PatternCandidate[] = []

    // Look for observations with extreme confidence values
    const highConfidence = observations.filter((o) => o.confidence > 0.9)
    const lowConfidence = observations.filter((o) => o.confidence < 0.3)

    if (highConfidence.length >= this.config.minObservations) {
      candidates.push({
        type: "threshold",
        observationIds: highConfidence.map((o) => o.id),
        confidence: 0.8,
        metadata: {
          threshold: "high_confidence",
          count: highConfidence.length,
          avgConfidence: highConfidence.reduce((s, o) => s + o.confidence, 0) / highConfidence.length,
        },
      })
    }

    if (lowConfidence.length >= this.config.minObservations) {
      candidates.push({
        type: "threshold",
        observationIds: lowConfidence.map((o) => o.id),
        confidence: 0.7,
        metadata: {
          threshold: "low_confidence",
          count: lowConfidence.length,
          avgConfidence: lowConfidence.reduce((s, o) => s + o.confidence, 0) / lowConfidence.length,
        },
      })
    }

    return candidates
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Helper Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private createPattern(candidate: PatternCandidate): EmergentPattern {
    const now = new Date()

    return {
      id: generatePatternId(),
      name: this.generatePatternName(candidate),
      description: this.generatePatternDescription(candidate),
      type: candidate.type,
      observationIds: candidate.observationIds,
      confidence: candidate.confidence,
      detectedAt: now,
      lastSeenAt: now,
      strength: candidate.confidence,
      suggestedActions: this.generateSuggestedActions(candidate),
    }
  }

  private generatePatternName(candidate: PatternCandidate): string {
    const meta = candidate.metadata
    switch (candidate.type) {
      case "trend":
        return `${meta.direction} ${meta.watcherType} trend`
      case "correlation":
        return `${meta.type1} - ${meta.type2} correlation`
      case "sequence":
        return `repeating sequence (${(meta.pattern as string[]).length} steps)`
      case "threshold":
        return `${meta.threshold} cluster`
      default:
        return `${candidate.type} pattern`
    }
  }

  private generatePatternDescription(candidate: PatternCandidate): string {
    const meta = candidate.metadata
    switch (candidate.type) {
      case "trend":
        return `${meta.watcherType} observations showing ${meta.direction} confidence trend (slope: ${(meta.slope as number).toFixed(3)})`
      case "correlation":
        return `${meta.type1} and ${meta.type2} observations occurring within close time proximity (${meta.pairCount} pairs)`
      case "sequence":
        return `Repeating observation sequence detected ${meta.repeatCount} times`
      case "threshold":
        return `${meta.count} observations clustered at ${meta.threshold} level`
      default:
        return `Pattern of type ${candidate.type}`
    }
  }

  private generateSuggestedActions(candidate: PatternCandidate): string[] {
    const actions: string[] = []
    const meta = candidate.metadata

    switch (candidate.type) {
      case "trend":
        if (meta.direction === "decreasing") {
          actions.push("Investigate declining confidence")
          actions.push("Check data source reliability")
        } else {
          actions.push("Monitor trend continuation")
        }
        break
      case "correlation":
        actions.push(`Monitor ${meta.type1} for predictive signals`)
        actions.push("Consider combining observations")
        break
      case "sequence":
        actions.push("Analyze sequence trigger conditions")
        actions.push("Consider automation of sequence handling")
        break
      case "threshold":
        if (meta.threshold === "low_confidence") {
          actions.push("Improve observation quality")
          actions.push("Add additional data sources")
        }
        break
    }

    return actions
  }

  private findSimilarPattern(pattern: EmergentPattern): EmergentPattern | null {
    for (const existing of this.activePatterns.values()) {
      if (existing.type !== pattern.type) continue
      if (existing.name === pattern.name) return existing

      // Check observation overlap
      const overlap = pattern.observationIds.filter((id) =>
        existing.observationIds.includes(id),
      )
      if (overlap.length > pattern.observationIds.length * 0.5) {
        return existing
      }
    }
    return null
  }

  private groupByWatcher(
    observations: Observation[],
  ): Map<string, Observation[]> {
    const groups = new Map<string, Observation[]>()
    for (const obs of observations) {
      const list = groups.get(obs.watcherType) ?? []
      list.push(obs)
      groups.set(obs.watcherType, list)
    }
    return groups
  }

  private groupByType(observations: Observation[]): Map<string, Observation[]> {
    const groups = new Map<string, Observation[]>()
    for (const obs of observations) {
      const type = `${obs.watcherType}:${(obs as any).type || "unknown"}`
      const list = groups.get(type) ?? []
      list.push(obs)
      groups.set(type, list)
    }
    return groups
  }

  private calculateTrend(values: number[]): { slope: number; r2: number } {
    const n = values.length
    if (n < 2) return { slope: 0, r2: 0 }

    // Simple linear regression
    let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0
    for (let i = 0; i < n; i++) {
      sumX += i
      sumY += values[i]
      sumXY += i * values[i]
      sumX2 += i * i
    }

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX)
    const intercept = (sumY - slope * sumX) / n

    // Calculate R²
    const meanY = sumY / n
    let ssRes = 0, ssTot = 0
    for (let i = 0; i < n; i++) {
      const predicted = slope * i + intercept
      ssRes += (values[i] - predicted) ** 2
      ssTot += (values[i] - meanY) ** 2
    }
    const r2 = ssTot > 0 ? 1 - ssRes / ssTot : 0

    return { slope, r2 }
  }

  private findTemporalCorrelation(
    obs1: Observation[],
    obs2: Observation[],
    maxDelta: number,
  ): { pairs: Array<[string, string]>; observationIds: string[]; strength: number; avgTimeDelta: number } {
    const pairs: Array<[string, string]> = []
    const observationIds = new Set<string>()
    let totalDelta = 0

    for (const a of obs1) {
      for (const b of obs2) {
        const delta = Math.abs(a.timestamp.getTime() - b.timestamp.getTime())
        if (delta <= maxDelta) {
          pairs.push([a.id, b.id])
          observationIds.add(a.id)
          observationIds.add(b.id)
          totalDelta += delta
        }
      }
    }

    return {
      pairs,
      observationIds: Array.from(observationIds),
      strength: Math.min(pairs.length / Math.min(obs1.length, obs2.length), 1),
      avgTimeDelta: pairs.length > 0 ? totalDelta / pairs.length : 0,
    }
  }

  private findRepeatingSequences(
    types: string[],
    ids: string[],
  ): Array<{ pattern: string[]; count: number; observationIds: string[] }> {
    const sequences: Array<{ pattern: string[]; count: number; observationIds: string[] }> = []

    // Try different pattern lengths
    for (let len = 2; len <= Math.min(5, Math.floor(types.length / 2)); len++) {
      for (let start = 0; start <= types.length - len * 2; start++) {
        const pattern = types.slice(start, start + len)
        let count = 1
        const obsIds = [...ids.slice(start, start + len)]

        // Look for repeats
        for (let pos = start + len; pos <= types.length - len; pos++) {
          const candidate = types.slice(pos, pos + len)
          if (JSON.stringify(pattern) === JSON.stringify(candidate)) {
            count++
            obsIds.push(...ids.slice(pos, pos + len))
            pos += len - 1 // Skip matched sequence
          }
        }

        if (count >= 2) {
          sequences.push({ pattern, count, observationIds: obsIds })
        }
      }
    }

    // Deduplicate and keep strongest
    const unique = new Map<string, { pattern: string[]; count: number; observationIds: string[] }>()
    for (const seq of sequences) {
      const key = seq.pattern.join("|")
      const existing = unique.get(key)
      if (!existing || existing.count < seq.count) {
        unique.set(key, seq)
      }
    }

    return Array.from(unique.values())
  }
}

/**
 * Create a pattern detector.
 */
export function createPatternDetector(config?: Partial<PatternConfig>): PatternDetector {
  return new PatternDetector(config)
}
