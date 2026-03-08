/**
 * Level 1: Pattern Recognition
 *
 * Observes Level 0 outputs and detects patterns, anomalies, and opportunities.
 *
 * @module observer/tower/levels/pattern
 */

import { Log } from "@/util/log"
import type { Observation, EmergentPattern, Anomaly, Opportunity } from "../../types"
import { ObserverLevel, type LevelOutput, type LevelConfig, isObservationOutput } from "../level"
import {
  PatternDetector,
  createPatternDetector,
  AnomalyDetector,
  createAnomalyDetector,
  OpportunityIdentifier,
  createOpportunityIdentifier,
} from "../../consensus"

const log = Log.create({ service: "observer.tower.level1" })

/**
 * Level 1 configuration.
 */
export interface Level1Config extends Partial<LevelConfig> {
  /** Enable pattern detection */
  detectPatterns?: boolean
  /** Enable anomaly detection */
  detectAnomalies?: boolean
  /** Enable opportunity identification */
  identifyOpportunities?: boolean
  /** Minimum pattern confidence to emit */
  minPatternConfidence?: number
  /** Minimum anomaly confidence to emit */
  minAnomalyConfidence?: number
  /** Minimum opportunity confidence to emit */
  minOpportunityConfidence?: number
}

/**
 * Level 1: Pattern recognition layer.
 *
 * Uses the existing consensus components to detect patterns,
 * anomalies, and opportunities from raw observations.
 */
export class PatternRecognitionLevel extends ObserverLevel {
  private patternDetector: PatternDetector | null = null
  private anomalyDetector: AnomalyDetector | null = null
  private opportunityIdentifier: OpportunityIdentifier | null = null

  private detectPatterns: boolean
  private detectAnomalies: boolean
  private identifyOpportunities: boolean
  private minPatternConfidence: number
  private minAnomalyConfidence: number
  private minOpportunityConfidence: number

  // Buffer for batch processing
  private observationBuffer: Observation[] = []
  private batchSize = 10
  private lastProcessTime: number = 0
  private processIntervalMs = 5000 // Process every 5 seconds

  constructor(config: Level1Config = {}) {
    super({
      level: 1,
      name: "Pattern Recognition",
      intervalMs: 5000, // Tick every 5 seconds
      ...config,
    })

    this.detectPatterns = config.detectPatterns ?? true
    this.detectAnomalies = config.detectAnomalies ?? true
    this.identifyOpportunities = config.identifyOpportunities ?? true
    this.minPatternConfidence = config.minPatternConfidence ?? 0.6
    this.minAnomalyConfidence = config.minAnomalyConfidence ?? 0.7
    this.minOpportunityConfidence = config.minOpportunityConfidence ?? 0.5
  }

  protected async onStart(): Promise<void> {
    // Initialize detectors
    if (this.detectPatterns) {
      this.patternDetector = createPatternDetector()
    }

    if (this.detectAnomalies) {
      this.anomalyDetector = createAnomalyDetector()
    }

    if (this.identifyOpportunities) {
      this.opportunityIdentifier = createOpportunityIdentifier()
    }

    this.lastProcessTime = Date.now()

    log.debug("Level 1 detectors initialized", {
      patterns: this.detectPatterns,
      anomalies: this.detectAnomalies,
      opportunities: this.identifyOpportunities,
    })
  }

  protected async onStop(): Promise<void> {
    // Process remaining buffer
    if (this.observationBuffer.length > 0) {
      await this.processBatch()
    }

    this.patternDetector = null
    this.anomalyDetector = null
    this.opportunityIdentifier = null
    this.observationBuffer = []

    log.debug("Level 1 detectors stopped")
  }

  protected async onProcess(input: LevelOutput): Promise<LevelOutput[]> {
    // Only process Level 0 observation outputs
    if (!isObservationOutput(input) || input.level !== 0) {
      return []
    }

    // Add to buffer
    this.observationBuffer.push(input.data.observation)

    // Process if batch is full or interval elapsed
    const shouldProcess =
      this.observationBuffer.length >= this.batchSize ||
      Date.now() - this.lastProcessTime >= this.processIntervalMs

    if (shouldProcess) {
      return this.processBatch()
    }

    return []
  }

  protected override tick(): void {
    // Process buffer on tick if not empty
    if (this.observationBuffer.length > 0) {
      this.processBatch().catch((err) => {
        log.error("Tick processing error", { error: err instanceof Error ? err.message : String(err) })
      })
    }
  }

  /**
   * Process buffered observations in batch.
   */
  private async processBatch(): Promise<LevelOutput[]> {
    const observations = this.observationBuffer.splice(0)
    this.lastProcessTime = Date.now()

    if (observations.length === 0) {
      return []
    }

    const outputs: LevelOutput[] = []
    const detectedPatterns: import("../../types").EmergentPattern[] = []
    const detectedAnomalies: import("../../types").Anomaly[] = []

    // Detect patterns
    if (this.patternDetector) {
      const patterns = this.patternDetector.detect(observations)
      for (const pattern of patterns) {
        if (pattern.confidence >= this.minPatternConfidence) {
          outputs.push(this.createOutput({ type: "pattern", pattern }))
          detectedPatterns.push(pattern)
        }
      }
    }

    // Detect anomalies
    if (this.anomalyDetector) {
      const anomalies = this.anomalyDetector.detect(observations)
      for (const anomaly of anomalies) {
        if (anomaly.confidence >= this.minAnomalyConfidence) {
          outputs.push(this.createOutput({ type: "anomaly", anomaly }))
          detectedAnomalies.push(anomaly)
        }
      }
    }

    // Identify opportunities (requires patterns and anomalies)
    if (this.opportunityIdentifier) {
      const opportunities = this.opportunityIdentifier.identify(observations, detectedPatterns, detectedAnomalies)
      for (const opportunity of opportunities) {
        if (opportunity.confidence >= this.minOpportunityConfidence) {
          outputs.push(this.createOutput({ type: "opportunity", opportunity }))
        }
      }
    }

    if (outputs.length > 0) {
      log.debug("Level 1 batch processed", {
        observations: observations.length,
        outputs: outputs.length,
      })
    }

    return outputs
  }

  /**
   * Get detected patterns.
   */
  getPatterns(): EmergentPattern[] {
    return this.getOutputs()
      .filter((o) => o.data.type === "pattern")
      .map((o) => (o.data as { type: "pattern"; pattern: EmergentPattern }).pattern)
  }

  /**
   * Get detected anomalies.
   */
  getAnomalies(): Anomaly[] {
    return this.getOutputs()
      .filter((o) => o.data.type === "anomaly")
      .map((o) => (o.data as { type: "anomaly"; anomaly: Anomaly }).anomaly)
  }

  /**
   * Get identified opportunities.
   */
  getOpportunities(): Opportunity[] {
    return this.getOutputs()
      .filter((o) => o.data.type === "opportunity")
      .map((o) => (o.data as { type: "opportunity"; opportunity: Opportunity }).opportunity)
  }
}

/**
 * Create a Level 1 instance.
 */
export function createPatternRecognitionLevel(config?: Level1Config): PatternRecognitionLevel {
  return new PatternRecognitionLevel(config)
}
