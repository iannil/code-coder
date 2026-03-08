/**
 * Attention Calculation
 *
 * Calculates attention weights for prioritizing observations.
 * Implements the core attention mechanism that determines which
 * observations receive focus in the consensus process.
 *
 * @module observer/consensus/attention
 */

import { Log } from "@/util/log"
import type {
  Observation,
  AttentionWeights,
  DEFAULT_ATTENTION_WEIGHTS,
} from "../types"

const log = Log.create({ service: "observer.consensus.attention" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface WeightedObservation {
  observation: Observation
  weight: number
  components: {
    base: number
    watcher: number
    type: number
    recency: number
    confidence: number
  }
}

export interface AttentionConfig {
  weights: AttentionWeights
  /** Maximum age for observations to receive full attention (ms) */
  maxAge: number
  /** Minimum weight threshold to include observation */
  minWeight: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: AttentionConfig = {
  weights: {
    byWatcher: { code: 0.3, world: 0.2, self: 0.3, meta: 0.2 },
    byType: {},
    timeDecay: 0.1,
    recencyBias: 0.7,
  },
  maxAge: 300000, // 5 minutes
  minWeight: 0.1,
}

// ─────────────────────────────────────────────────────────────────────────────
// Attention Calculator
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Calculates attention weights for observations.
 */
export class AttentionCalculator {
  private config: AttentionConfig

  constructor(config: Partial<AttentionConfig> = {}) {
    this.config = {
      ...DEFAULT_CONFIG,
      ...config,
      weights: {
        ...DEFAULT_CONFIG.weights,
        ...config.weights,
      },
    }
  }

  /**
   * Calculate weighted observations.
   */
  calculate(observations: Observation[]): WeightedObservation[] {
    const now = Date.now()
    const results: WeightedObservation[] = []

    for (const obs of observations) {
      const weighted = this.calculateSingle(obs, now)
      if (weighted.weight >= this.config.minWeight) {
        results.push(weighted)
      }
    }

    // Sort by weight descending
    results.sort((a, b) => b.weight - a.weight)

    return results
  }

  /**
   * Calculate weight for a single observation.
   */
  calculateSingle(observation: Observation, now?: number): WeightedObservation {
    const currentTime = now ?? Date.now()
    const { weights } = this.config

    // Base weight (all observations start with 1.0)
    const base = 1.0

    // Watcher type weight
    const watcher = weights.byWatcher[observation.watcherType] ?? 0.25

    // Observation type weight (use type from observation if available)
    const obsType = (observation as any).type as string | undefined
    const type = obsType ? (weights.byType[obsType] ?? 1.0) : 1.0

    // Recency weight (exponential decay)
    const ageMs = currentTime - observation.timestamp.getTime()
    const normalizedAge = Math.min(ageMs / this.config.maxAge, 1)
    const recency = Math.exp(-weights.timeDecay * normalizedAge * 10) *
      (1 + weights.recencyBias * (1 - normalizedAge))

    // Confidence weight
    const confidence = observation.confidence

    // Combined weight
    const weight = base * watcher * type * recency * confidence

    return {
      observation,
      weight,
      components: {
        base,
        watcher,
        type,
        recency,
        confidence,
      },
    }
  }

  /**
   * Get top N observations by weight.
   */
  getTop(observations: Observation[], n: number): WeightedObservation[] {
    return this.calculate(observations).slice(0, n)
  }

  /**
   * Get observations above a weight threshold.
   */
  getAboveThreshold(
    observations: Observation[],
    threshold: number,
  ): WeightedObservation[] {
    return this.calculate(observations).filter((w) => w.weight >= threshold)
  }

  /**
   * Update configuration.
   */
  updateConfig(config: Partial<AttentionConfig>): void {
    this.config = {
      ...this.config,
      ...config,
      weights: {
        ...this.config.weights,
        ...config.weights,
      },
    }
  }

  /**
   * Get current configuration.
   */
  getConfig(): AttentionConfig {
    return { ...this.config }
  }
}

/**
 * Create an attention calculator.
 */
export function createAttentionCalculator(
  config?: Partial<AttentionConfig>,
): AttentionCalculator {
  return new AttentionCalculator(config)
}
