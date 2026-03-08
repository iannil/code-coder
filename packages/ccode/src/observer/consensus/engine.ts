/**
 * Consensus Engine
 *
 * The core of the Observer Network that aggregates observations from
 * multiple watchers and forms a unified understanding of the world.
 *
 * Implements the "祝融说" philosophy of "观察共识" (observation consensus):
 * Multiple observers contribute to a shared understanding through
 * attention-weighted aggregation, pattern detection, and convergence.
 *
 * @module observer/consensus/engine
 */

import { Log } from "@/util/log"
import type {
  Observation,
  WorldModel,
  EmergentPattern,
  Anomaly,
  Opportunity,
  AttentionWeights,
} from "../types"
import { ObserverEvent } from "../events"
import { getEventStream } from "../event-stream"
import { AttentionCalculator, type AttentionConfig } from "./attention"
import { PatternDetector, type PatternConfig } from "./patterns"
import { AnomalyDetector, type AnomalyConfig } from "./anomaly"
import { OpportunityIdentifier, type OpportunityConfig } from "./opportunity"
import { WorldModelBuilder, type WorldModelConfig } from "./world-model"

const log = Log.create({ service: "observer.consensus.engine" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ConsensusEngineConfig {
  /** Consensus window in ms */
  windowMs: number
  /** Update interval in ms */
  updateIntervalMs: number
  /** Attention configuration */
  attention?: Partial<AttentionConfig>
  /** Pattern detection configuration */
  patterns?: Partial<PatternConfig>
  /** Anomaly detection configuration */
  anomaly?: Partial<AnomalyConfig>
  /** Opportunity identification configuration */
  opportunity?: Partial<OpportunityConfig>
  /** World model configuration */
  worldModel?: Partial<WorldModelConfig>
}

export interface ConsensusSnapshot {
  worldModel: WorldModel | null
  patterns: EmergentPattern[]
  anomalies: Anomaly[]
  opportunities: Opportunity[]
  timestamp: Date
  confidence: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: ConsensusEngineConfig = {
  windowMs: 60000, // 1 minute
  updateIntervalMs: 5000, // 5 seconds
}

// ─────────────────────────────────────────────────────────────────────────────
// Consensus Engine
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Aggregates observations and forms consensus understanding.
 */
export class ConsensusEngine {
  private config: ConsensusEngineConfig
  private attention: AttentionCalculator
  private patternDetector: PatternDetector
  private anomalyDetector: AnomalyDetector
  private opportunityIdentifier: OpportunityIdentifier
  private worldModelBuilder: WorldModelBuilder

  private running = false
  private updateTimer: ReturnType<typeof setInterval> | null = null
  private lastSnapshot: ConsensusSnapshot | null = null
  private lastConsensusStrength = 0.5

  constructor(config: Partial<ConsensusEngineConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }

    this.attention = new AttentionCalculator(config.attention)
    this.patternDetector = new PatternDetector(config.patterns)
    this.anomalyDetector = new AnomalyDetector(config.anomaly)
    this.opportunityIdentifier = new OpportunityIdentifier(config.opportunity)
    this.worldModelBuilder = new WorldModelBuilder(config.worldModel)
  }

  /**
   * Start the consensus engine.
   */
  start(): void {
    if (this.running) return

    this.running = true

    // Start periodic updates
    this.updateTimer = setInterval(async () => {
      await this.update()
    }, this.config.updateIntervalMs)

    log.info("Consensus engine started", {
      windowMs: this.config.windowMs,
      updateIntervalMs: this.config.updateIntervalMs,
    })
  }

  /**
   * Stop the consensus engine.
   */
  stop(): void {
    if (!this.running) return

    this.running = false

    if (this.updateTimer) {
      clearInterval(this.updateTimer)
      this.updateTimer = null
    }

    log.info("Consensus engine stopped")
  }

  /**
   * Perform a consensus update.
   */
  async update(): Promise<ConsensusSnapshot> {
    const stream = getEventStream()
    const window = stream.getWindow(this.config.windowMs)
    const observations = window.observations

    // Apply attention weighting
    const weighted = this.attention.calculate(observations)
    const weightedObs = weighted.map((w) => w.observation)

    // Detect patterns
    const patterns = this.patternDetector.detect(weightedObs)

    // Detect anomalies
    const allObs = stream.getAll()
    const anomalies = this.anomalyDetector.detect(weightedObs, allObs)

    // Identify opportunities
    const opportunities = this.opportunityIdentifier.identify(
      weightedObs,
      this.patternDetector.getActive(),
      this.anomalyDetector.getActive(),
    )

    // Build world model
    const worldModel = this.worldModelBuilder.build(weightedObs)

    // Calculate consensus strength
    const confidence = this.calculateConsensusStrength(
      weightedObs,
      patterns,
      anomalies,
    )

    // Publish events for significant changes
    await this.publishEvents(worldModel, patterns, anomalies, opportunities, confidence)

    // Create snapshot
    const snapshot: ConsensusSnapshot = {
      worldModel,
      patterns: this.patternDetector.getActive(),
      anomalies: this.anomalyDetector.getActive(),
      opportunities: this.opportunityIdentifier.getActive(),
      timestamp: new Date(),
      confidence,
    }

    this.lastSnapshot = snapshot
    this.lastConsensusStrength = confidence

    log.debug("Consensus updated", {
      observations: observations.length,
      patterns: patterns.length,
      anomalies: anomalies.length,
      opportunities: opportunities.length,
      confidence: confidence.toFixed(2),
    })

    return snapshot
  }

  /**
   * Get current snapshot.
   */
  getSnapshot(): ConsensusSnapshot | null {
    return this.lastSnapshot
  }

  /**
   * Get current world model.
   */
  getWorldModel(): WorldModel | null {
    return this.worldModelBuilder.getCurrent()
  }

  /**
   * Get active patterns.
   */
  getPatterns(): EmergentPattern[] {
    return this.patternDetector.getActive()
  }

  /**
   * Get active anomalies.
   */
  getAnomalies(): Anomaly[] {
    return this.anomalyDetector.getActive()
  }

  /**
   * Get active opportunities.
   */
  getOpportunities(): Opportunity[] {
    return this.opportunityIdentifier.getActive()
  }

  /**
   * Get attention weights.
   */
  getAttentionWeights(): AttentionWeights {
    return this.attention.getConfig().weights
  }

  /**
   * Update attention weights.
   */
  updateAttentionWeights(weights: Partial<AttentionWeights>): void {
    this.attention.updateConfig({ weights: weights as AttentionWeights })
  }

  /**
   * Clear all state.
   */
  clear(): void {
    this.patternDetector.clear()
    this.anomalyDetector.clear()
    this.opportunityIdentifier.clear()
    this.worldModelBuilder.clear()
    this.lastSnapshot = null
    this.lastConsensusStrength = 0.5
  }

  /**
   * Check if running.
   */
  isRunning(): boolean {
    return this.running
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private calculateConsensusStrength(
    observations: Observation[],
    patterns: EmergentPattern[],
    anomalies: Anomaly[],
  ): number {
    if (observations.length === 0) return 0

    // Base from observation confidence
    const avgConfidence =
      observations.reduce((sum, o) => sum + o.confidence, 0) / observations.length

    // Pattern strength bonus
    const patternBonus =
      patterns.length > 0
        ? patterns.reduce((sum, p) => sum + p.strength, 0) / patterns.length * 0.2
        : 0

    // Anomaly penalty
    const anomalyPenalty =
      anomalies.length > 0
        ? Math.min(anomalies.length * 0.05, 0.3)
        : 0

    // Coverage factor
    const watcherTypes = new Set(observations.map((o) => o.watcherType))
    const coverage = watcherTypes.size / 4

    return Math.max(
      0,
      Math.min(1, avgConfidence * 0.4 + patternBonus + coverage * 0.4 - anomalyPenalty),
    )
  }

  private async publishEvents(
    worldModel: WorldModel | null,
    newPatterns: EmergentPattern[],
    newAnomalies: Anomaly[],
    newOpportunities: Opportunity[],
    confidence: number,
  ): Promise<void> {
    const BusModule = await import("@/bus")
    const Bus = BusModule.Bus

    // Publish world model update
    if (worldModel) {
      await Bus.publish(ObserverEvent.WorldModelUpdated, worldModel)
    }

    // Publish new patterns
    for (const pattern of newPatterns) {
      await Bus.publish(ObserverEvent.PatternDetected, pattern)
    }

    // Publish new anomalies
    for (const anomaly of newAnomalies) {
      await Bus.publish(ObserverEvent.AnomalyDetected, anomaly)
    }

    // Publish new opportunities
    for (const opportunity of newOpportunities) {
      await Bus.publish(ObserverEvent.OpportunityIdentified, opportunity)
    }

    // Check for consensus strength change
    const strengthChange = Math.abs(confidence - this.lastConsensusStrength)
    if (strengthChange > 0.1) {
      await Bus.publish(ObserverEvent.ConsensusStrengthChanged, {
        previousStrength: this.lastConsensusStrength,
        newStrength: confidence,
        change: confidence > this.lastConsensusStrength ? "increased" : "decreased",
        trigger: strengthChange > 0.2 ? "significant_shift" : undefined,
      })
    }

    // Expire old items
    const expiredPatterns = this.patternDetector.expirePatterns()
    for (const patternId of expiredPatterns) {
      await Bus.publish(ObserverEvent.PatternExpired, {
        patternId,
        reason: "aged_out",
      })
    }

    this.anomalyDetector.expireAnomalies(this.config.windowMs * 5)
    this.opportunityIdentifier.expireOpportunities(this.config.windowMs * 10)
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let engineInstance: ConsensusEngine | null = null

/**
 * Get the global consensus engine instance.
 */
export function getConsensusEngine(
  config?: Partial<ConsensusEngineConfig>,
): ConsensusEngine {
  if (!engineInstance) {
    engineInstance = new ConsensusEngine(config)
  }
  return engineInstance
}

/**
 * Reset the consensus engine (for testing).
 */
export function resetConsensusEngine(): void {
  if (engineInstance) {
    engineInstance.stop()
    engineInstance.clear()
    engineInstance = null
  }
}

/**
 * Create a new consensus engine instance.
 */
export function createConsensusEngine(
  config?: Partial<ConsensusEngineConfig>,
): ConsensusEngine {
  return new ConsensusEngine(config)
}
