/**
 * Base Watcher
 *
 * Abstract base class for all observer watchers in the Observer Network.
 * Provides common functionality for lifecycle management, observation
 * emission, and status tracking.
 *
 * @module observer/watchers/base-watcher
 */

import { Log } from "@/util/log"
import type {
  Observation,
  WatcherConfig,
  WatcherStatus,
  BaseObservation,
} from "../types"
import { generateObservationId } from "../types"
import { getEventStream } from "../event-stream"
import { ObserverEvent } from "../events"

const log = Log.create({ service: "observer.watcher" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type WatcherType = "code" | "world" | "self" | "meta"

export interface WatcherOptions {
  /** Custom watcher ID (auto-generated if not provided) */
  id?: string
  /** Observation interval in ms (0 = event-driven only) */
  intervalMs?: number
  /** Filter patterns */
  filters?: string[]
  /** Priority (0-10) */
  priority?: number
  /** Additional options */
  [key: string]: unknown
}

// ─────────────────────────────────────────────────────────────────────────────
// Base Watcher Class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Abstract base class for watchers.
 *
 * Subclasses must implement:
 * - `observe()`: Perform one observation cycle
 * - `onStart()`: Optional setup when watcher starts
 * - `onStop()`: Optional cleanup when watcher stops
 */
export abstract class BaseWatcher<
  TObservation extends Observation = Observation,
> {
  protected readonly id: string
  protected readonly type: WatcherType
  protected readonly options: WatcherOptions

  protected running = false
  protected intervalTimer: ReturnType<typeof setInterval> | null = null
  protected observationCount = 0
  protected errorCount = 0
  protected lastObservation: Date | null = null
  protected latencies: number[] = []
  protected maxLatencySamples = 100

  constructor(type: WatcherType, options: WatcherOptions = {}) {
    this.type = type
    this.id = options.id ?? `${type}_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`
    this.options = {
      intervalMs: 0,
      filters: [],
      priority: 5,
      ...options,
    }
  }

  /**
   * Get watcher ID.
   */
  getId(): string {
    return this.id
  }

  /**
   * Get watcher type.
   */
  getType(): WatcherType {
    return this.type
  }

  /**
   * Check if watcher is running.
   */
  isRunning(): boolean {
    return this.running
  }

  /**
   * Start the watcher.
   */
  async start(): Promise<void> {
    if (this.running) {
      log.warn("Watcher already running", { watcherId: this.id })
      return
    }

    this.running = true

    try {
      await this.onStart()
    } catch (error) {
      log.error("Error in onStart", {
        watcherId: this.id,
        error: error instanceof Error ? error.message : String(error),
      })
    }

    // Set up interval if configured
    if (this.options.intervalMs && this.options.intervalMs > 0) {
      this.intervalTimer = setInterval(async () => {
        await this.runObservationCycle()
      }, this.options.intervalMs)
    }

    // Publish started event
    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.WatcherStarted, {
      watcherId: this.id,
      watcherType: this.type,
      config: this.options,
    })

    log.info("Watcher started", {
      watcherId: this.id,
      watcherType: this.type,
      intervalMs: this.options.intervalMs,
    })
  }

  /**
   * Stop the watcher.
   */
  async stop(reason?: string): Promise<void> {
    if (!this.running) {
      return
    }

    this.running = false

    // Clear interval
    if (this.intervalTimer) {
      clearInterval(this.intervalTimer)
      this.intervalTimer = null
    }

    try {
      await this.onStop()
    } catch (error) {
      log.error("Error in onStop", {
        watcherId: this.id,
        error: error instanceof Error ? error.message : String(error),
      })
    }

    // Publish stopped event
    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.WatcherStopped, {
      watcherId: this.id,
      watcherType: this.type,
      reason,
    })

    log.info("Watcher stopped", {
      watcherId: this.id,
      watcherType: this.type,
      reason,
      stats: this.getStats(),
    })
  }

  /**
   * Get watcher status.
   */
  getStatus(): WatcherStatus {
    const avgLatency =
      this.latencies.length > 0
        ? this.latencies.reduce((a, b) => a + b, 0) / this.latencies.length
        : 0

    return {
      id: this.id,
      type: this.type,
      running: this.running,
      health: this.calculateHealth(),
      lastObservation: this.lastObservation ?? undefined,
      observationCount: this.observationCount,
      errorCount: this.errorCount,
      avgLatency: Math.round(avgLatency),
    }
  }

  /**
   * Get watcher configuration.
   */
  getConfig(): WatcherConfig {
    return {
      id: this.id,
      type: this.type,
      enabled: this.running,
      intervalMs: this.options.intervalMs ?? 0,
      filters: this.options.filters ?? [],
      priority: this.options.priority ?? 5,
      options: this.options,
    }
  }

  /**
   * Trigger an immediate observation cycle.
   */
  async triggerObservation(): Promise<TObservation | null> {
    if (!this.running) {
      log.warn("Cannot trigger observation, watcher not running", {
        watcherId: this.id,
      })
      return null
    }

    return this.runObservationCycle()
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Abstract Methods (to be implemented by subclasses)
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Perform one observation cycle.
   * Called either by interval timer or triggered manually.
   */
  protected abstract observe(): Promise<TObservation | null>

  /**
   * Called when watcher starts.
   * Override to perform setup (e.g., subscribe to events).
   */
  protected async onStart(): Promise<void> {
    // Default: no-op
  }

  /**
   * Called when watcher stops.
   * Override to perform cleanup.
   */
  protected async onStop(): Promise<void> {
    // Default: no-op
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Protected Methods (for use by subclasses)
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Create a base observation with common fields.
   */
  protected createBaseObservation(): Omit<BaseObservation, "watcherType"> {
    return {
      id: generateObservationId(this.type),
      timestamp: new Date(),
      watcherId: this.id,
      confidence: 1.0,
      tags: [],
      metadata: {},
    }
  }

  /**
   * Emit an observation to the event stream.
   */
  protected async emit(observation: TObservation): Promise<void> {
    const stream = getEventStream()
    await stream.ingest(observation)

    this.observationCount++
    this.lastObservation = new Date()

    log.debug("Observation emitted", {
      watcherId: this.id,
      observationId: observation.id,
      type: (observation as any).type,
    })
  }

  /**
   * Report an error.
   */
  protected async reportError(error: Error, recoverable = true): Promise<void> {
    this.errorCount++

    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.WatcherError, {
      watcherId: this.id,
      watcherType: this.type,
      error: error.message,
      recoverable,
    })

    log.error("Watcher error", {
      watcherId: this.id,
      error: error.message,
      recoverable,
      errorCount: this.errorCount,
    })

    // Stop if too many errors
    if (this.errorCount > 10 && !recoverable) {
      await this.stop("Too many errors")
    }
  }

  /**
   * Check if a source matches the configured filters.
   * Returns true if no filters configured or if source matches any filter.
   */
  protected matchesFilters(source: string): boolean {
    const filters = this.options.filters ?? []
    if (filters.length === 0) return true

    return filters.some((filter) => {
      // Support glob-like patterns
      if (filter.includes("*")) {
        const regex = new RegExp(
          "^" + filter.replace(/\*/g, ".*").replace(/\?/g, ".") + "$",
        )
        return regex.test(source)
      }
      return source.includes(filter)
    })
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async runObservationCycle(): Promise<TObservation | null> {
    const startTime = Date.now()

    try {
      const observation = await this.observe()

      if (observation) {
        await this.emit(observation)
      }

      // Track latency
      const latency = Date.now() - startTime
      this.trackLatency(latency)

      return observation
    } catch (error) {
      await this.reportError(
        error instanceof Error ? error : new Error(String(error)),
        true,
      )
      return null
    }
  }

  private trackLatency(latency: number): void {
    this.latencies.push(latency)
    if (this.latencies.length > this.maxLatencySamples) {
      this.latencies.shift()
    }
  }

  private calculateHealth(): WatcherStatus["health"] {
    if (!this.running) return "stopped"

    const errorRate =
      this.observationCount > 0
        ? this.errorCount / this.observationCount
        : 0

    if (errorRate > 0.5) return "failing"
    if (errorRate > 0.1) return "degraded"
    return "healthy"
  }

  private getStats(): {
    observations: number
    errors: number
    avgLatency: number
  } {
    const avgLatency =
      this.latencies.length > 0
        ? this.latencies.reduce((a, b) => a + b, 0) / this.latencies.length
        : 0

    return {
      observations: this.observationCount,
      errors: this.errorCount,
      avgLatency: Math.round(avgLatency),
    }
  }
}
