/**
 * Anomaly Detection
 *
 * Detects anomalies in observation streams including:
 * - Outliers (values outside normal range)
 * - Sudden changes (rapid shifts in patterns)
 * - Missing expected observations
 * - Unexpected presence of observations
 * - Timing anomalies
 *
 * @module observer/consensus/anomaly
 */

import { Log } from "@/util/log"
import type { Observation, Anomaly } from "../types"
import { generateAnomalyId } from "../types"

const log = Log.create({ service: "observer.consensus.anomaly" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface AnomalyConfig {
  /** Standard deviations for outlier detection */
  outlierThreshold: number
  /** Maximum expected gap between observations (ms) */
  maxObservationGap: number
  /** Minimum confidence for anomaly detection */
  minConfidence: number
  /** History window for baseline calculation (ms) */
  historyWindowMs: number
}

interface StatisticalBaseline {
  mean: number
  stdDev: number
  min: number
  max: number
  count: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: AnomalyConfig = {
  outlierThreshold: 2.5,
  maxObservationGap: 120000, // 2 minutes
  minConfidence: 0.5,
  historyWindowMs: 600000, // 10 minutes
}

// ─────────────────────────────────────────────────────────────────────────────
// Anomaly Detector
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Detects anomalies in observation streams.
 */
export class AnomalyDetector {
  private config: AnomalyConfig
  private activeAnomalies: Map<string, Anomaly> = new Map()
  private baselines: Map<string, StatisticalBaseline> = new Map()
  private lastObservationByWatcher: Map<string, Date> = new Map()

  constructor(config: Partial<AnomalyConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Detect anomalies in observations.
   */
  detect(observations: Observation[], history?: Observation[]): Anomaly[] {
    const anomalies: Anomaly[] = []

    // Update baselines from history
    if (history && history.length > 0) {
      this.updateBaselines(history)
    }

    // Check for outliers
    anomalies.push(...this.detectOutliers(observations))

    // Check for sudden changes
    anomalies.push(...this.detectSuddenChanges(observations))

    // Check for timing anomalies
    anomalies.push(...this.detectTimingAnomalies(observations))

    // Filter by confidence
    const confirmed = anomalies.filter((a) => a.confidence >= this.config.minConfidence)

    // Update active anomalies
    for (const anomaly of confirmed) {
      const existing = this.findSimilarAnomaly(anomaly)
      if (existing) {
        // Update existing
        existing.observationIds = [
          ...new Set([...existing.observationIds, ...anomaly.observationIds]),
        ]
        if (existing.status === "suspected" && anomaly.confidence > 0.8) {
          existing.status = "confirmed"
        }
      } else {
        this.activeAnomalies.set(anomaly.id, anomaly)
      }
    }

    return confirmed
  }

  /**
   * Get active anomalies.
   */
  getActive(): Anomaly[] {
    return Array.from(this.activeAnomalies.values())
  }

  /**
   * Update anomaly status.
   */
  updateStatus(
    anomalyId: string,
    status: Anomaly["status"],
  ): Anomaly | undefined {
    const anomaly = this.activeAnomalies.get(anomalyId)
    if (anomaly) {
      anomaly.status = status
      if (status === "dismissed") {
        this.activeAnomalies.delete(anomalyId)
      }
    }
    return anomaly
  }

  /**
   * Clear old anomalies.
   */
  expireAnomalies(maxAge: number): string[] {
    const now = Date.now()
    const expired: string[] = []

    for (const [id, anomaly] of this.activeAnomalies.entries()) {
      const age = now - anomaly.detectedAt.getTime()
      if (age > maxAge) {
        this.activeAnomalies.delete(id)
        expired.push(id)
      }
    }

    return expired
  }

  /**
   * Clear all state.
   */
  clear(): void {
    this.activeAnomalies.clear()
    this.baselines.clear()
    this.lastObservationByWatcher.clear()
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Detection Algorithms
  // ─────────────────────────────────────────────────────────────────────────────

  private detectOutliers(observations: Observation[]): Anomaly[] {
    const anomalies: Anomaly[] = []

    for (const obs of observations) {
      const baselineKey = `${obs.watcherType}:confidence`
      const baseline = this.baselines.get(baselineKey)

      if (!baseline || baseline.count < 10) continue

      // Check confidence as outlier
      const zScore = Math.abs(obs.confidence - baseline.mean) / (baseline.stdDev || 1)

      if (zScore > this.config.outlierThreshold) {
        anomalies.push(
          this.createAnomaly("outlier", [obs.id], {
            description: `Confidence ${obs.confidence.toFixed(2)} is ${zScore.toFixed(1)} standard deviations from mean (${baseline.mean.toFixed(2)})`,
            severity: zScore > 4 ? "high" : zScore > 3 ? "medium" : "low",
            confidence: Math.min(zScore / this.config.outlierThreshold / 2, 1),
          }),
        )
      }
    }

    return anomalies
  }

  private detectSuddenChanges(observations: Observation[]): Anomaly[] {
    const anomalies: Anomaly[] = []

    // Sort by time
    const sorted = [...observations].sort(
      (a, b) => a.timestamp.getTime() - b.timestamp.getTime(),
    )

    // Look for sudden changes in confidence
    for (let i = 1; i < sorted.length; i++) {
      const prev = sorted[i - 1]
      const curr = sorted[i]

      // Only compare same watcher type
      if (prev.watcherType !== curr.watcherType) continue

      const confidenceChange = Math.abs(curr.confidence - prev.confidence)
      const timeDelta = curr.timestamp.getTime() - prev.timestamp.getTime()

      // Sudden change: large confidence change in short time
      if (confidenceChange > 0.5 && timeDelta < 30000) {
        anomalies.push(
          this.createAnomaly("sudden_change", [prev.id, curr.id], {
            description: `Sudden confidence change from ${prev.confidence.toFixed(2)} to ${curr.confidence.toFixed(2)} in ${(timeDelta / 1000).toFixed(0)}s`,
            severity: confidenceChange > 0.7 ? "high" : "medium",
            confidence: Math.min(confidenceChange, 1),
          }),
        )
      }
    }

    return anomalies
  }

  private detectTimingAnomalies(observations: Observation[]): Anomaly[] {
    const anomalies: Anomaly[] = []
    const now = new Date()

    // Group by watcher
    const byWatcher = new Map<string, Observation[]>()
    for (const obs of observations) {
      const list = byWatcher.get(obs.watcherId) ?? []
      list.push(obs)
      byWatcher.set(obs.watcherId, list)
    }

    for (const [watcherId, obs] of byWatcher.entries()) {
      const lastKnown = this.lastObservationByWatcher.get(watcherId)
      const latestObs = obs.reduce((latest, o) =>
        o.timestamp > latest.timestamp ? o : latest,
      )

      // Update last known
      this.lastObservationByWatcher.set(watcherId, latestObs.timestamp)

      // Check for gap
      if (lastKnown) {
        const gap = latestObs.timestamp.getTime() - lastKnown.getTime()
        if (gap > this.config.maxObservationGap) {
          anomalies.push(
            this.createAnomaly("timing", [latestObs.id], {
              description: `Observation gap of ${(gap / 1000 / 60).toFixed(1)} minutes from watcher ${watcherId}`,
              severity: gap > this.config.maxObservationGap * 2 ? "high" : "medium",
              confidence: Math.min(gap / this.config.maxObservationGap / 2, 1),
            }),
          )
        }
      }
    }

    return anomalies
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Helper Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private createAnomaly(
    type: Anomaly["type"],
    observationIds: string[],
    details: {
      description: string
      severity: Anomaly["severity"]
      confidence: number
    },
  ): Anomaly {
    return {
      id: generateAnomalyId(),
      type,
      description: details.description,
      severity: details.severity,
      observationIds,
      detectedAt: new Date(),
      status: "suspected",
      confidence: details.confidence,
    }
  }

  private findSimilarAnomaly(anomaly: Anomaly): Anomaly | null {
    for (const existing of this.activeAnomalies.values()) {
      if (existing.type !== anomaly.type) continue

      // Check observation overlap
      const overlap = anomaly.observationIds.filter((id) =>
        existing.observationIds.includes(id),
      )
      if (overlap.length > 0) {
        return existing
      }
    }
    return null
  }

  private updateBaselines(history: Observation[]): void {
    // Group by watcher type
    const byWatcher = new Map<string, number[]>()

    const windowStart = Date.now() - this.config.historyWindowMs

    for (const obs of history) {
      if (obs.timestamp.getTime() < windowStart) continue

      const key = `${obs.watcherType}:confidence`
      const values = byWatcher.get(key) ?? []
      values.push(obs.confidence)
      byWatcher.set(key, values)
    }

    // Calculate statistics
    for (const [key, values] of byWatcher.entries()) {
      if (values.length < 5) continue

      const mean = values.reduce((a, b) => a + b, 0) / values.length
      const variance = values.reduce((sum, v) => sum + (v - mean) ** 2, 0) / values.length
      const stdDev = Math.sqrt(variance)

      this.baselines.set(key, {
        mean,
        stdDev,
        min: Math.min(...values),
        max: Math.max(...values),
        count: values.length,
      })
    }
  }
}

/**
 * Create an anomaly detector.
 */
export function createAnomalyDetector(config?: Partial<AnomalyConfig>): AnomalyDetector {
  return new AnomalyDetector(config)
}
