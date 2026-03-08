/**
 * Level 2: Meta Observation
 *
 * Observes all lower levels and provides insights about
 * the observation system itself (coverage, health, blind spots).
 *
 * @module observer/tower/levels/meta
 */

import { Log } from "@/util/log"
import { ObserverLevel, type LevelOutput, type LevelConfig, type LevelInsight, isPatternOutput, isAnomalyOutput, isObservationOutput } from "../level"
import type { MetaObservation } from "../../types"

const log = Log.create({ service: "observer.tower.level2" })

/**
 * Level 2 configuration.
 */
export interface Level2Config extends Partial<LevelConfig> {
  /** Minimum observation rate per minute before warning */
  minObservationRate?: number
  /** Maximum anomaly rate per minute before warning */
  maxAnomalyRate?: number
  /** Time window for rate calculations (ms) */
  rateWindowMs?: number
}

/**
 * Observation quality metrics.
 */
interface QualityMetrics {
  observationRate: number // per minute
  patternRate: number
  anomalyRate: number
  coverageByType: Record<string, number>
  avgConfidence: number
}

/**
 * Level 2: Meta observation layer.
 *
 * Monitors the health and quality of the observation system,
 * detecting blind spots, coverage gaps, and calibration issues.
 */
export class MetaObservationLevel extends ObserverLevel {
  private minObservationRate: number
  private maxAnomalyRate: number
  private rateWindowMs: number

  // Rolling window for rate calculation
  private recentOutputs: Array<{ time: number; type: string; watcherType?: string; confidence: number }> = []
  private lastMetaCheck: number = 0
  private metaCheckIntervalMs = 30000 // Check every 30 seconds

  constructor(config: Level2Config = {}) {
    super({
      level: 2,
      name: "Meta Observation",
      intervalMs: 30000, // Tick every 30 seconds
      ...config,
    })

    this.minObservationRate = config.minObservationRate ?? 1 // At least 1 per minute
    this.maxAnomalyRate = config.maxAnomalyRate ?? 10 // No more than 10 anomalies per minute
    this.rateWindowMs = config.rateWindowMs ?? 60000 // 1 minute window
  }

  protected async onStart(): Promise<void> {
    this.lastMetaCheck = Date.now()
    log.debug("Level 2 meta observation started")
  }

  protected async onStop(): Promise<void> {
    this.recentOutputs = []
    log.debug("Level 2 meta observation stopped")
  }

  protected async onProcess(input: LevelOutput): Promise<LevelOutput[]> {
    // Track all outputs from lower levels
    const now = Date.now()

    // Extract info for tracking
    let watcherType: string | undefined
    let confidence = 1.0

    if (isObservationOutput(input)) {
      watcherType = input.data.observation.watcherType
      confidence = input.data.observation.confidence
    } else if (isPatternOutput(input)) {
      confidence = input.data.pattern.confidence
    } else if (isAnomalyOutput(input)) {
      confidence = input.data.anomaly.confidence
    }

    this.recentOutputs.push({
      time: now,
      type: input.data.type,
      watcherType,
      confidence,
    })

    // Clean old entries
    const cutoff = now - this.rateWindowMs
    this.recentOutputs = this.recentOutputs.filter((o) => o.time >= cutoff)

    return []
  }

  protected override tick(): void {
    const now = Date.now()

    // Only run meta check periodically
    if (now - this.lastMetaCheck < this.metaCheckIntervalMs) {
      return
    }

    this.lastMetaCheck = now
    this.runMetaCheck()
  }

  /**
   * Run a meta observation check.
   */
  private runMetaCheck(): void {
    const metrics = this.calculateMetrics()
    const insights = this.generateInsights(metrics)

    for (const insight of insights) {
      this.emit(this.createOutput({ type: "insight", insight }))
    }

    if (insights.length > 0) {
      log.debug("Level 2 generated insights", { count: insights.length })
    }
  }

  /**
   * Calculate quality metrics from recent outputs.
   */
  private calculateMetrics(): QualityMetrics {
    const now = Date.now()
    const windowMinutes = this.rateWindowMs / 60000

    const observations = this.recentOutputs.filter((o) => o.type === "observation")
    const patterns = this.recentOutputs.filter((o) => o.type === "pattern")
    const anomalies = this.recentOutputs.filter((o) => o.type === "anomaly")

    // Coverage by watcher type
    const coverageByType: Record<string, number> = {}
    for (const obs of observations) {
      if (obs.watcherType) {
        coverageByType[obs.watcherType] = (coverageByType[obs.watcherType] ?? 0) + 1
      }
    }

    // Average confidence
    const allConfidences = this.recentOutputs.map((o) => o.confidence)
    const avgConfidence = allConfidences.length > 0
      ? allConfidences.reduce((a, b) => a + b, 0) / allConfidences.length
      : 1.0

    return {
      observationRate: observations.length / windowMinutes,
      patternRate: patterns.length / windowMinutes,
      anomalyRate: anomalies.length / windowMinutes,
      coverageByType,
      avgConfidence,
    }
  }

  /**
   * Generate insights from metrics.
   */
  private generateInsights(metrics: QualityMetrics): LevelInsight[] {
    const insights: LevelInsight[] = []
    const timestamp = Date.now()

    // Low observation rate
    if (metrics.observationRate < this.minObservationRate) {
      insights.push({
        id: `meta_low_obs_${timestamp}`,
        name: "Low Observation Rate",
        description: `Observation rate (${metrics.observationRate.toFixed(1)}/min) is below minimum (${this.minObservationRate}/min)`,
        confidence: 0.9,
        sources: [],
        suggestedActions: [
          "Check if watchers are running",
          "Verify event stream is connected",
          "Review watcher configurations",
        ],
      })
    }

    // High anomaly rate
    if (metrics.anomalyRate > this.maxAnomalyRate) {
      insights.push({
        id: `meta_high_anom_${timestamp}`,
        name: "High Anomaly Rate",
        description: `Anomaly rate (${metrics.anomalyRate.toFixed(1)}/min) exceeds threshold (${this.maxAnomalyRate}/min)`,
        confidence: 0.85,
        sources: [],
        suggestedActions: [
          "Review anomaly detector thresholds",
          "Check for systemic issues",
          "Consider switching to more conservative mode",
        ],
      })
    }

    // Coverage gaps
    const expectedTypes = ["code", "world", "self", "meta"]
    const missingTypes = expectedTypes.filter((t) => !(t in metrics.coverageByType))

    if (missingTypes.length > 0) {
      insights.push({
        id: `meta_coverage_gap_${timestamp}`,
        name: "Coverage Gap Detected",
        description: `No observations from: ${missingTypes.join(", ")}`,
        confidence: 0.8,
        sources: [],
        suggestedActions: missingTypes.map((t) => `Enable or check ${t} watcher`),
      })
    }

    // Low confidence average
    if (metrics.avgConfidence < 0.5) {
      insights.push({
        id: `meta_low_conf_${timestamp}`,
        name: "Low Average Confidence",
        description: `Average observation confidence (${(metrics.avgConfidence * 100).toFixed(0)}%) is below 50%`,
        confidence: 0.7,
        sources: [],
        suggestedActions: [
          "Review data sources",
          "Check for noise in observations",
          "Calibrate watcher sensitivity",
        ],
      })
    }

    return insights
  }

  /**
   * Get current quality metrics.
   */
  getMetrics(): QualityMetrics {
    return this.calculateMetrics()
  }

  /**
   * Get recent insights.
   */
  getInsights(): LevelInsight[] {
    return this.getOutputs()
      .filter((o) => o.data.type === "insight")
      .map((o) => (o.data as { type: "insight"; insight: LevelInsight }).insight)
  }
}

/**
 * Create a Level 2 instance.
 */
export function createMetaObservationLevel(config?: Level2Config): MetaObservationLevel {
  return new MetaObservationLevel(config)
}
