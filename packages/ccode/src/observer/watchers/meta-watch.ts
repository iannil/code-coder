/**
 * Meta Watcher (MetaWatch)
 *
 * Observes the Observer Network itself including:
 * - Observation quality and coverage
 * - System health
 * - Blind spots in observation
 * - Consensus drift
 * - Watcher performance
 *
 * This implements the "元观察" (meta-observation) aspect,
 * enabling the system to observe its own observation process.
 *
 * @module observer/watchers/meta-watch
 */

import { Log } from "@/util/log"
import { BaseWatcher, type WatcherOptions } from "./base-watcher"
import type { MetaObservation, MetaObservationType, WatcherStatus } from "../types"
import { getEventStream } from "../event-stream"
import { ObserverEvent } from "../events"
import { Bus } from "@/bus"

const log = Log.create({ service: "observer.meta-watch" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface MetaWatchOptions extends WatcherOptions {
  /** Quality threshold for warnings (0-1) */
  qualityThreshold?: number
  /** Coverage threshold for warnings (0-1) */
  coverageThreshold?: number
  /** Maximum consensus drift before warning */
  maxConsensusDrift?: number
  /** Watched watcher IDs (if empty, watches all) */
  watchedWatchers?: string[]
  /** Latency threshold in ms (default: 1000) */
  latencyThreshold?: number
}

interface WatcherMetrics {
  watcherId: string
  watcherType: "code" | "world" | "self" | "meta"
  observationCount: number
  errorRate: number
  avgLatency: number
  lastObservation: Date | null
  health: "healthy" | "degraded" | "failing" | "stopped"
}

// ─────────────────────────────────────────────────────────────────────────────
// MetaWatch Implementation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Watcher that observes the Observer Network itself.
 */
export class MetaWatch extends BaseWatcher<MetaObservation> {
  private qualityThreshold: number
  private coverageThreshold: number
  private maxConsensusDrift: number
  private latencyThreshold: number
  private watchedWatchers: string[]
  private watcherMetrics: Map<string, WatcherMetrics> = new Map()
  private eventSubscriptions: Array<() => void> = []
  private lastConsensusStrength = 0.5
  private observationsByWatcher: Map<string, number> = new Map()

  constructor(options: MetaWatchOptions = {}) {
    super("meta", {
      intervalMs: 60000, // Check every minute
      ...options,
    })
    this.qualityThreshold = options.qualityThreshold ?? 0.7
    this.coverageThreshold = options.coverageThreshold ?? 0.6
    this.maxConsensusDrift = options.maxConsensusDrift ?? 0.3
    this.latencyThreshold = options.latencyThreshold ?? 1000
    this.watchedWatchers = options.watchedWatchers ?? []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────────

  protected override async onStart(): Promise<void> {
    // Subscribe to watcher events
    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.WatcherStarted, async (event) => {
        const props = event.properties as { watcherId: string; watcherType: string }
        if (!this.shouldWatch(props.watcherId)) return
        this.initWatcherMetrics(props.watcherId, props.watcherType as any)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.WatcherStopped, async (event) => {
        const props = event.properties as { watcherId: string }
        if (!this.shouldWatch(props.watcherId)) return
        this.updateWatcherHealth(props.watcherId, "stopped")
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.WatcherError, async (event) => {
        const props = event.properties as {
          watcherId: string
          error: string
          recoverable: boolean
        }
        if (!this.shouldWatch(props.watcherId)) return
        await this.handleWatcherError(props)
      }),
    )

    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.WatcherStatusChanged, async (event) => {
        const status = event.properties as WatcherStatus
        if (!this.shouldWatch(status.id)) return
        await this.handleStatusChange(status)
      }),
    )

    // Subscribe to observation events to track coverage
    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.CodeObserved, () => this.trackObservation("code")),
    )
    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.WorldObserved, () => this.trackObservation("world")),
    )
    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.SelfObserved, () => this.trackObservation("self")),
    )

    // Subscribe to consensus events
    this.eventSubscriptions.push(
      Bus.subscribe(ObserverEvent.ConsensusStrengthChanged, async (event) => {
        const props = event.properties as {
          previousStrength: number
          newStrength: number
        }
        await this.handleConsensusChange(props)
      }),
    )

    log.info("MetaWatch initialized", {
      qualityThreshold: this.qualityThreshold,
      coverageThreshold: this.coverageThreshold,
      maxConsensusDrift: this.maxConsensusDrift,
      latencyThreshold: this.latencyThreshold,
    })
  }

  protected override async onStop(): Promise<void> {
    for (const unsubscribe of this.eventSubscriptions) {
      unsubscribe()
    }
    this.eventSubscriptions = []
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Observation
  // ─────────────────────────────────────────────────────────────────────────────

  protected async observe(): Promise<MetaObservation | null> {
    // Perform periodic health check
    const healthCheck = await this.performHealthCheck()
    if (healthCheck) return healthCheck

    // Check for latency threshold breaches
    const latencyCheck = this.checkLatencyThreshold()
    if (latencyCheck) return latencyCheck

    // Check for coverage gaps
    const coverageCheck = this.checkCoverageGaps()
    if (coverageCheck) return coverageCheck

    // Check observation quality
    const qualityCheck = this.checkObservationQuality()
    if (qualityCheck) return qualityCheck

    return null
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Public Methods
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Get metrics for all watched watchers.
   */
  getWatcherMetrics(): WatcherMetrics[] {
    return Array.from(this.watcherMetrics.values())
  }

  /**
   * Get overall system health.
   */
  getSystemHealth(): {
    overall: "healthy" | "degraded" | "failing"
    watcherHealth: Record<string, "healthy" | "degraded" | "failing" | "stopped">
    coverage: number
    quality: number
  } {
    const watchers = this.getWatcherMetrics()

    const healthCounts = { healthy: 0, degraded: 0, failing: 0, stopped: 0 }
    const watcherHealth: Record<string, WatcherMetrics["health"]> = {}

    for (const w of watchers) {
      healthCounts[w.health]++
      watcherHealth[w.watcherId] = w.health
    }

    // Calculate overall health
    let overall: "healthy" | "degraded" | "failing" = "healthy"
    if (healthCounts.failing > 0) overall = "failing"
    else if (healthCounts.degraded > 0 || healthCounts.stopped > 0) overall = "degraded"

    // Calculate coverage
    const coverage = this.calculateCoverage()

    // Calculate quality
    const quality = this.calculateQuality()

    return { overall, watcherHealth, coverage, quality }
  }

  /**
   * Get latency status for all watchers.
   */
  getLatencyStatus(): {
    avgLatency: number
    threshold: number
    exceededCount: number
    watcherLatencies: Record<string, number>
  } {
    const watchers = this.getWatcherMetrics()
    const avgLatency = this.calculateAvgLatency()
    const exceededCount = watchers.filter(
      (w) => w.avgLatency > this.latencyThreshold && w.health !== "stopped",
    ).length

    const watcherLatencies: Record<string, number> = {}
    for (const w of watchers) {
      watcherLatencies[w.watcherId] = w.avgLatency
    }

    return {
      avgLatency,
      threshold: this.latencyThreshold,
      exceededCount,
      watcherLatencies,
    }
  }

  /**
   * Perform a manual calibration check.
   */
  async calibrate(): Promise<MetaObservation> {
    const health = this.getSystemHealth()

    const issues: MetaObservation["issues"] = []

    // Check for failing watchers
    for (const [id, status] of Object.entries(health.watcherHealth)) {
      if (status === "failing") {
        issues.push({
          type: "watcher_failing",
          severity: "high",
          description: `Watcher ${id} is failing`,
        })
      } else if (status === "stopped") {
        issues.push({
          type: "watcher_stopped",
          severity: "medium",
          description: `Watcher ${id} is stopped`,
        })
      }
    }

    // Check coverage
    if (health.coverage < this.coverageThreshold) {
      issues.push({
        type: "low_coverage",
        severity: "medium",
        description: `Observation coverage is ${(health.coverage * 100).toFixed(0)}% (threshold: ${(this.coverageThreshold * 100).toFixed(0)}%)`,
      })
    }

    // Check quality
    if (health.quality < this.qualityThreshold) {
      issues.push({
        type: "low_quality",
        severity: "medium",
        description: `Observation quality is ${(health.quality * 100).toFixed(0)}% (threshold: ${(this.qualityThreshold * 100).toFixed(0)}%)`,
      })
    }

    // Check latency
    const avgLatency = this.calculateAvgLatency()
    if (avgLatency > this.latencyThreshold) {
      issues.push({
        type: "latency_exceeded",
        severity: avgLatency > this.latencyThreshold * 2 ? "high" : "medium",
        description: `Average latency is ${avgLatency.toFixed(0)}ms (threshold: ${this.latencyThreshold}ms)`,
      })
    }

    const observation = this.createObservation("calibration", {
      health: health.overall,
      coverage: health.coverage,
      accuracy: health.quality,
      latency: this.calculateAvgLatency(),
    })

    observation.issues = issues
    observation.recommendations = this.generateRecommendations(issues)

    await this.emit(observation)
    return observation
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods - Checks
  // ─────────────────────────────────────────────────────────────────────────────

  private async performHealthCheck(): Promise<MetaObservation | null> {
    const watchers = this.getWatcherMetrics()
    const failing = watchers.filter((w) => w.health === "failing")

    if (failing.length > 0) {
      const observation = this.createObservation("system_health", {
        health: "failing",
        coverage: this.calculateCoverage(),
        accuracy: this.calculateQuality(),
        latency: this.calculateAvgLatency(),
      })

      observation.issues = failing.map((w) => ({
        type: "watcher_failing",
        severity: "high" as const,
        description: `Watcher ${w.watcherId} (${w.watcherType}) is failing with ${(w.errorRate * 100).toFixed(0)}% error rate`,
      }))

      observation.recommendations = [
        "Check watcher logs for errors",
        "Consider restarting failing watchers",
        "Review watcher configurations",
      ]

      return observation
    }

    return null
  }

  private checkCoverageGaps(): MetaObservation | null {
    const coverage = this.calculateCoverage()

    if (coverage < this.coverageThreshold) {
      const gaps = this.identifyGaps()

      const observation = this.createObservation("coverage_gap", {
        health: "degraded",
        coverage,
        accuracy: this.calculateQuality(),
        latency: this.calculateAvgLatency(),
      })

      observation.issues = gaps.map((gap) => ({
        type: "coverage_gap",
        severity: "medium" as const,
        description: gap,
      }))

      observation.recommendations = [
        "Start additional watchers for uncovered areas",
        "Increase observation frequency",
        "Review filter configurations",
      ]

      return observation
    }

    return null
  }

  private checkObservationQuality(): MetaObservation | null {
    const quality = this.calculateQuality()

    if (quality < this.qualityThreshold) {
      const observation = this.createObservation("observation_quality", {
        health: "degraded",
        coverage: this.calculateCoverage(),
        accuracy: quality,
        latency: this.calculateAvgLatency(),
      })

      observation.issues = [
        {
          type: "low_quality",
          severity: "medium",
          description: `Overall observation quality is ${(quality * 100).toFixed(0)}%`,
        },
      ]

      observation.recommendations = [
        "Review observation confidence thresholds",
        "Check data source reliability",
        "Investigate high-latency watchers",
      ]

      return observation
    }

    return null
  }

  private checkLatencyThreshold(): MetaObservation | null {
    const watchers = this.getWatcherMetrics()
    const highLatencyWatchers = watchers.filter(
      (w) => w.avgLatency > this.latencyThreshold && w.health !== "stopped",
    )

    if (highLatencyWatchers.length > 0) {
      const avgLatency = this.calculateAvgLatency()

      const observation = this.createObservation("observation_quality", {
        health: avgLatency > this.latencyThreshold * 2 ? "failing" : "degraded",
        coverage: this.calculateCoverage(),
        accuracy: this.calculateQuality(),
        latency: avgLatency,
      })

      observation.issues = highLatencyWatchers.map((w) => ({
        type: "latency_exceeded",
        severity: w.avgLatency > this.latencyThreshold * 2 ? "high" as const : "medium" as const,
        description: `Watcher ${w.watcherId} (${w.watcherType}) has latency ${w.avgLatency.toFixed(0)}ms (threshold: ${this.latencyThreshold}ms)`,
      }))

      observation.tags = ["latency_exceeded"]

      observation.recommendations = [
        "Check system load and available resources",
        "Consider reducing observation frequency",
        "Review watcher configurations for optimization",
        "Investigate slow data sources or API calls",
      ]

      log.warn("Latency threshold exceeded", {
        watcherCount: highLatencyWatchers.length,
        threshold: this.latencyThreshold,
        avgLatency,
      })

      return observation
    }

    return null
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods - Event Handlers
  // ─────────────────────────────────────────────────────────────────────────────

  private async handleWatcherError(props: {
    watcherId: string
    error: string
    recoverable: boolean
  }): Promise<void> {
    const metrics = this.watcherMetrics.get(props.watcherId)
    if (!metrics) return

    // Update error rate
    const totalObs = metrics.observationCount + 1
    metrics.errorRate = (metrics.errorRate * metrics.observationCount + 1) / totalObs
    metrics.observationCount = totalObs

    if (metrics.errorRate > 0.5) {
      metrics.health = "failing"
    } else if (metrics.errorRate > 0.1) {
      metrics.health = "degraded"
    }

    this.watcherMetrics.set(props.watcherId, metrics)

    if (!props.recoverable || metrics.health === "failing") {
      // Map "stopped" to "failing" for observation health field
      const observationHealth: "healthy" | "degraded" | "failing" =
        metrics.health === "stopped" ? "failing" : metrics.health
      const observation = this.createObservation("watcher_status", {
        health: observationHealth,
        coverage: this.calculateCoverage(),
        accuracy: this.calculateQuality(),
        latency: metrics.avgLatency,
      })

      observation.targetWatcherId = props.watcherId
      observation.issues = [
        {
          type: "watcher_error",
          severity: props.recoverable ? "medium" : "high",
          description: `Watcher ${props.watcherId} error: ${props.error}`,
        },
      ]

      await this.emit(observation)
    }
  }

  private async handleStatusChange(status: WatcherStatus): Promise<void> {
    this.watcherMetrics.set(status.id, {
      watcherId: status.id,
      watcherType: status.type,
      observationCount: status.observationCount,
      errorRate: status.observationCount > 0 ? status.errorCount / status.observationCount : 0,
      avgLatency: status.avgLatency,
      lastObservation: status.lastObservation ?? null,
      health: status.health,
    })
  }

  private async handleConsensusChange(props: {
    previousStrength: number
    newStrength: number
  }): Promise<void> {
    const drift = Math.abs(props.newStrength - props.previousStrength)

    if (drift > this.maxConsensusDrift) {
      const observation = this.createObservation("consensus_drift", {
        health: "degraded",
        coverage: this.calculateCoverage(),
        accuracy: this.calculateQuality(),
        latency: this.calculateAvgLatency(),
      })

      observation.issues = [
        {
          type: "consensus_drift",
          severity: drift > 0.5 ? "high" : "medium",
          description: `Consensus strength changed from ${(props.previousStrength * 100).toFixed(0)}% to ${(props.newStrength * 100).toFixed(0)}%`,
        },
      ]

      observation.recommendations = [
        "Review recent observations for anomalies",
        "Check for external events affecting consensus",
        "Consider adjusting attention weights",
      ]

      await this.emit(observation)
    }

    this.lastConsensusStrength = props.newStrength
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods - Calculations
  // ─────────────────────────────────────────────────────────────────────────────

  private createObservation(
    type: MetaObservationType,
    assessment: MetaObservation["assessment"],
  ): MetaObservation {
    const base = this.createBaseObservation()

    return {
      ...base,
      watcherType: "meta" as const,
      type,
      assessment,
      recommendations: [],
      issues: [],
    }
  }

  private shouldWatch(watcherId: string): boolean {
    if (this.watchedWatchers.length === 0) return true
    return this.watchedWatchers.includes(watcherId)
  }

  private initWatcherMetrics(watcherId: string, watcherType: "code" | "world" | "self" | "meta"): void {
    this.watcherMetrics.set(watcherId, {
      watcherId,
      watcherType,
      observationCount: 0,
      errorRate: 0,
      avgLatency: 0,
      lastObservation: null,
      health: "healthy",
    })
  }

  private updateWatcherHealth(watcherId: string, health: WatcherMetrics["health"]): void {
    const metrics = this.watcherMetrics.get(watcherId)
    if (metrics) {
      metrics.health = health
      this.watcherMetrics.set(watcherId, metrics)
    }
  }

  private trackObservation(watcherType: "code" | "world" | "self"): void {
    const count = this.observationsByWatcher.get(watcherType) ?? 0
    this.observationsByWatcher.set(watcherType, count + 1)
  }

  private calculateCoverage(): number {
    const types = ["code", "world", "self"]
    const covered = types.filter((t) => (this.observationsByWatcher.get(t) ?? 0) > 0)
    return covered.length / types.length
  }

  private calculateQuality(): number {
    const stream = getEventStream()
    const recent = stream.getRecent(100)

    if (recent.length === 0) return 1

    const avgConfidence = recent.reduce((sum, o) => sum + o.confidence, 0) / recent.length
    return avgConfidence
  }

  private calculateAvgLatency(): number {
    const metrics = Array.from(this.watcherMetrics.values())
    if (metrics.length === 0) return 0

    return metrics.reduce((sum, m) => sum + m.avgLatency, 0) / metrics.length
  }

  private identifyGaps(): string[] {
    const gaps: string[] = []
    const types = ["code", "world", "self"]

    for (const type of types) {
      const count = this.observationsByWatcher.get(type) ?? 0
      if (count === 0) {
        gaps.push(`No observations from ${type} watcher`)
      }
    }

    return gaps
  }

  private generateRecommendations(issues: MetaObservation["issues"]): string[] {
    const recommendations: string[] = []

    for (const issue of issues) {
      switch (issue.type) {
        case "watcher_failing":
          recommendations.push("Restart failing watcher")
          recommendations.push("Check watcher configuration")
          break
        case "coverage_gap":
          recommendations.push("Start additional watchers")
          break
        case "low_quality":
          recommendations.push("Review data sources")
          break
        case "consensus_drift":
          recommendations.push("Investigate recent changes")
          break
      }
    }

    return [...new Set(recommendations)]
  }
}

/**
 * Create a MetaWatch instance.
 */
export function createMetaWatch(options?: MetaWatchOptions): MetaWatch {
  return new MetaWatch(options)
}
