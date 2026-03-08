/**
 * Event Stream Management
 *
 * Manages the flow of observation events through the Observer Network.
 * Provides buffering, routing, and aggregation of observations.
 *
 * Design principles:
 * - All observations are immutable
 * - Events are processed in order but may be aggregated
 * - Time-based windowing for consensus formation
 *
 * @module observer/event-stream
 */

import { Log } from "@/util/log"
import type {
  Observation,
  CodeObservation,
  WorldObservation,
  SelfObservation,
  MetaObservation,
  AttentionWeights,
  DEFAULT_ATTENTION_WEIGHTS,
} from "./types"
import { ObserverEvent } from "./events"

const log = Log.create({ service: "observer.event-stream" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface EventStreamConfig {
  /** Maximum number of observations to keep in memory */
  maxObservations: number
  /** Time window for consensus in ms */
  consensusWindowMs: number
  /** Attention weights for prioritization */
  attention: AttentionWeights
  /** Enable buffering (false = immediate emission) */
  buffered: boolean
  /** Buffer flush interval in ms */
  bufferFlushMs: number
}

export interface ObservationWindow {
  /** Start time of window */
  startTime: Date
  /** End time of window */
  endTime: Date
  /** Observations in this window */
  observations: Observation[]
  /** Aggregated metrics */
  metrics: {
    totalCount: number
    byType: Record<string, number>
    avgConfidence: number
  }
}

export type ObservationHandler = (observation: Observation) => void | Promise<void>

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: EventStreamConfig = {
  maxObservations: 1000,
  consensusWindowMs: 5000,
  attention: {
    byWatcher: { code: 0.3, world: 0.2, self: 0.3, meta: 0.2 },
    byType: {},
    timeDecay: 0.1,
    recencyBias: 0.7,
  },
  buffered: true,
  bufferFlushMs: 1000,
}

// ─────────────────────────────────────────────────────────────────────────────
// Event Stream Class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * EventStream manages observation flow through the Observer Network.
 */
export class EventStream {
  private config: EventStreamConfig
  private observations: Observation[] = []
  private buffer: Observation[] = []
  private handlers: Set<ObservationHandler> = new Set()
  private flushTimer: ReturnType<typeof setInterval> | null = null
  private running = false
  private stats = {
    received: 0,
    emitted: 0,
    dropped: 0,
  }

  constructor(config: Partial<EventStreamConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Start the event stream.
   */
  start(): void {
    if (this.running) return

    this.running = true

    if (this.config.buffered && this.config.bufferFlushMs > 0) {
      this.flushTimer = setInterval(() => {
        this.flushBuffer()
      }, this.config.bufferFlushMs)
    }

    log.info("Event stream started", {
      maxObservations: this.config.maxObservations,
      consensusWindowMs: this.config.consensusWindowMs,
      buffered: this.config.buffered,
    })
  }

  /**
   * Stop the event stream.
   */
  stop(): void {
    if (!this.running) return

    this.running = false

    if (this.flushTimer) {
      clearInterval(this.flushTimer)
      this.flushTimer = null
    }

    // Flush remaining buffer
    this.flushBuffer()

    log.info("Event stream stopped", {
      stats: this.stats,
    })
  }

  /**
   * Ingest a new observation.
   */
  async ingest(observation: Observation): Promise<void> {
    if (!this.running) {
      log.warn("Event stream not running, dropping observation", {
        observationId: observation.id,
      })
      this.stats.dropped++
      return
    }

    this.stats.received++

    // Add to observations list (with size limit)
    this.observations.push(observation)
    if (this.observations.length > this.config.maxObservations) {
      this.observations.shift()
      this.stats.dropped++
    }

    // Buffer or emit immediately
    if (this.config.buffered) {
      this.buffer.push(observation)
    } else {
      await this.emit(observation)
    }
  }

  /**
   * Subscribe to observations.
   */
  subscribe(handler: ObservationHandler): () => void {
    this.handlers.add(handler)
    return () => {
      this.handlers.delete(handler)
    }
  }

  /**
   * Get observations within a time window.
   */
  getWindow(windowMs?: number): ObservationWindow {
    const windowDuration = windowMs ?? this.config.consensusWindowMs
    const now = new Date()
    const startTime = new Date(now.getTime() - windowDuration)

    const windowObservations = this.observations.filter(
      (o) => o.timestamp >= startTime,
    )

    // Calculate metrics
    const byType: Record<string, number> = {}
    let totalConfidence = 0

    for (const obs of windowObservations) {
      const typeKey = `${obs.watcherType}:${(obs as any).type || "unknown"}`
      byType[typeKey] = (byType[typeKey] || 0) + 1
      totalConfidence += obs.confidence
    }

    return {
      startTime,
      endTime: now,
      observations: windowObservations,
      metrics: {
        totalCount: windowObservations.length,
        byType,
        avgConfidence:
          windowObservations.length > 0
            ? totalConfidence / windowObservations.length
            : 0,
      },
    }
  }

  /**
   * Get observations by watcher type.
   */
  getByWatcher(
    watcherType: "code" | "world" | "self" | "meta",
    limit?: number,
  ): Observation[] {
    const filtered = this.observations
      .filter((o) => o.watcherType === watcherType)
      .slice(-(limit ?? 100))
    return filtered
  }

  /**
   * Get weighted observations based on attention weights.
   */
  getWeighted(limit?: number): Array<{ observation: Observation; weight: number }> {
    const { attention } = this.config
    const now = Date.now()

    const weighted = this.observations.map((obs) => {
      // Base weight from watcher type
      const watcherWeight = attention.byWatcher[obs.watcherType] ?? 0.25

      // Type weight
      const typeKey = (obs as any).type || "unknown"
      const typeWeight = attention.byType[typeKey] ?? 1.0

      // Time decay
      const ageMs = now - obs.timestamp.getTime()
      const timeWeight = Math.exp(-attention.timeDecay * (ageMs / 1000))

      // Recency bonus
      const recencyBonus = attention.recencyBias * timeWeight

      // Combined weight
      const weight =
        obs.confidence * watcherWeight * typeWeight * (1 + recencyBonus)

      return { observation: obs, weight }
    })

    // Sort by weight descending
    weighted.sort((a, b) => b.weight - a.weight)

    return weighted.slice(0, limit ?? 100)
  }

  /**
   * Get recent observations.
   */
  getRecent(limit?: number): Observation[] {
    return this.observations.slice(-(limit ?? 50))
  }

  /**
   * Get all observations.
   */
  getAll(): Observation[] {
    return [...this.observations]
  }

  /**
   * Get stream statistics.
   */
  getStats(): typeof this.stats & { current: number; bufferSize: number } {
    return {
      ...this.stats,
      current: this.observations.length,
      bufferSize: this.buffer.length,
    }
  }

  /**
   * Clear all observations.
   */
  clear(): void {
    this.observations = []
    this.buffer = []
    this.stats = { received: 0, emitted: 0, dropped: 0 }
  }

  /**
   * Update configuration.
   */
  updateConfig(config: Partial<EventStreamConfig>): void {
    const wasBuffered = this.config.buffered
    this.config = { ...this.config, ...config }

    // Handle buffer mode changes
    if (wasBuffered && !this.config.buffered) {
      this.flushBuffer()
      if (this.flushTimer) {
        clearInterval(this.flushTimer)
        this.flushTimer = null
      }
    } else if (!wasBuffered && this.config.buffered && this.running) {
      this.flushTimer = setInterval(() => {
        this.flushBuffer()
      }, this.config.bufferFlushMs)
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async emit(observation: Observation): Promise<void> {
    this.stats.emitted++

    // Publish to bus based on watcher type
    const BusModule = await import("@/bus")
    const Bus = BusModule.Bus

    switch (observation.watcherType) {
      case "code":
        await Bus.publish(
          ObserverEvent.CodeObserved,
          observation as CodeObservation,
        )
        break
      case "world":
        await Bus.publish(
          ObserverEvent.WorldObserved,
          observation as WorldObservation,
        )
        break
      case "self":
        await Bus.publish(
          ObserverEvent.SelfObserved,
          observation as SelfObservation,
        )
        break
      case "meta":
        await Bus.publish(
          ObserverEvent.MetaObserved,
          observation as MetaObservation,
        )
        break
    }

    // Notify handlers
    for (const handler of this.handlers) {
      try {
        await handler(observation)
      } catch (error) {
        log.error("Handler error", {
          observationId: observation.id,
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }
  }

  private async flushBuffer(): Promise<void> {
    if (this.buffer.length === 0) return

    const toEmit = [...this.buffer]
    this.buffer = []

    for (const observation of toEmit) {
      await this.emit(observation)
    }

    log.debug("Buffer flushed", { count: toEmit.length })
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let streamInstance: EventStream | null = null

/**
 * Get the global event stream instance.
 */
export function getEventStream(config?: Partial<EventStreamConfig>): EventStream {
  if (!streamInstance) {
    streamInstance = new EventStream(config)
  }
  return streamInstance
}

/**
 * Reset the event stream (for testing).
 */
export function resetEventStream(): void {
  if (streamInstance) {
    streamInstance.stop()
    streamInstance = null
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create a new event stream instance.
 */
export function createEventStream(
  config?: Partial<EventStreamConfig>,
): EventStream {
  return new EventStream(config)
}
