/**
 * Historian Responder
 *
 * Records observation history for analysis and learning.
 * Integrates with the memory system for persistent storage.
 *
 * @module observer/responders/historian
 */

import { Log } from "@/util/log"
import type {
  Observation,
  WorldModel,
  EmergentPattern,
  Anomaly,
  Opportunity,
} from "../types"
import type { ConsensusSnapshot } from "../consensus"
import type { ModeDecision } from "../controller"
import { ObserverEvent } from "../events"
import {
  getMemoryClient,
  type ObserverHistoryEntry,
  type ObserverMemoryConfig,
} from "../integration/memory-client"

const log = Log.create({ service: "observer.responders.historian" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type HistoryEventType =
  | "observation"
  | "pattern"
  | "anomaly"
  | "opportunity"
  | "world_model"
  | "mode_decision"
  | "escalation"
  | "execution"

export interface HistoryEntry {
  id: string
  type: HistoryEventType
  timestamp: Date
  data: unknown
  tags: string[]
  sessionId?: string
}

export interface HistoryQuery {
  type?: HistoryEventType | HistoryEventType[]
  startTime?: Date
  endTime?: Date
  tags?: string[]
  sessionId?: string
  limit?: number
  offset?: number
}

export interface HistoryStats {
  totalEntries: number
  byType: Record<HistoryEventType, number>
  oldestEntry?: Date
  newestEntry?: Date
  sessionCount: number
}

export interface HistorianConfig {
  /** Enable automatic recording */
  autoRecord: boolean
  /** Storage path for persistent history */
  storagePath?: string
  /** Maximum in-memory entries */
  maxInMemory: number
  /** Flush to storage interval (ms) */
  flushIntervalMs: number
  /** Session ID for tagging */
  sessionId?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: HistorianConfig = {
  autoRecord: true,
  maxInMemory: 10000,
  flushIntervalMs: 60000, // 1 minute
}

// ─────────────────────────────────────────────────────────────────────────────
// Historian
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Records and manages observation history.
 */
export class Historian {
  private config: HistorianConfig
  private entries: Map<string, HistoryEntry> = new Map()
  private entriesByType: Map<HistoryEventType, string[]> = new Map()
  private idCounter = 0
  private running = false
  private flushTimer: ReturnType<typeof setInterval> | null = null
  private eventSubscriptions: Array<() => void> = []
  private pendingFlush: HistoryEntry[] = []

  constructor(config: Partial<HistorianConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }

    // Initialize type index
    const types: HistoryEventType[] = [
      "observation",
      "pattern",
      "anomaly",
      "opportunity",
      "world_model",
      "mode_decision",
      "escalation",
      "execution",
    ]
    for (const type of types) {
      this.entriesByType.set(type, [])
    }
  }

  /**
   * Start the historian.
   */
  async start(): Promise<void> {
    if (this.running) return

    this.running = true

    // Start flush timer - always enabled since we use memory-markdown
    this.flushTimer = setInterval(() => {
      void this.flush()
    }, this.config.flushIntervalMs)

    if (this.config.autoRecord) {
      const Bus = (await import("@/bus")).Bus

      // Record observations
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.CodeObserved, (event) => {
          void this.record("observation", event.properties, ["code"])
        }),
      )
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.WorldObserved, (event) => {
          void this.record("observation", event.properties, ["world"])
        }),
      )
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.SelfObserved, (event) => {
          void this.record("observation", event.properties, ["self"])
        }),
      )
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.MetaObserved, (event) => {
          void this.record("observation", event.properties, ["meta"])
        }),
      )

      // Record patterns
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.PatternDetected, (event) => {
          const pattern = event.properties as EmergentPattern
          void this.record("pattern", pattern, [pattern.type])
        }),
      )

      // Record anomalies
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.AnomalyDetected, (event) => {
          const anomaly = event.properties as Anomaly
          void this.record("anomaly", anomaly, [anomaly.type, anomaly.severity])
        }),
      )

      // Record opportunities
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.OpportunityIdentified, (event) => {
          const opp = event.properties as Opportunity
          void this.record("opportunity", opp, [opp.type, opp.impact])
        }),
      )

      // Record world model updates
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.WorldModelUpdated, (event) => {
          void this.record("world_model", event.properties, ["snapshot"])
        }),
      )

      // Record mode decisions
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.ModeEvaluated, (event) => {
          const decision = event.properties as unknown
          void this.record("mode_decision", decision, ["evaluation"])
        }),
      )
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.ModeSwitched, (event) => {
          void this.record("mode_decision", event.properties, ["switch"])
        }),
      )

      // Record escalations
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.EscalationCreated, (event) => {
          void this.record("escalation", event.properties, ["created"])
        }),
      )
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.EscalationResolved, (event) => {
          void this.record("escalation", event.properties, ["resolved"])
        }),
      )

      // Record executions
      this.eventSubscriptions.push(
        Bus.subscribe(ObserverEvent.ExecutionTriggered, (event) => {
          const exec = event.properties as { status: string }
          void this.record("execution", event.properties, [exec.status])
        }),
      )
    }

    log.info("Historian started", {
      autoRecord: this.config.autoRecord,
      storagePath: this.config.storagePath,
    })
  }

  /**
   * Stop the historian.
   */
  async stop(): Promise<void> {
    if (!this.running) return

    this.running = false

    if (this.flushTimer) {
      clearInterval(this.flushTimer)
      this.flushTimer = null
    }

    for (const unsubscribe of this.eventSubscriptions) {
      unsubscribe()
    }
    this.eventSubscriptions = []

    // Final flush
    await this.flush()

    log.info("Historian stopped", {
      entriesRecorded: this.entries.size,
    })
  }

  /**
   * Record an event.
   */
  async record(
    type: HistoryEventType,
    data: unknown,
    tags: string[] = [],
  ): Promise<HistoryEntry> {
    const entry: HistoryEntry = {
      id: `hist_${Date.now()}_${++this.idCounter}`,
      type,
      timestamp: new Date(),
      data,
      tags,
      sessionId: this.config.sessionId,
    }

    this.entries.set(entry.id, entry)
    this.entriesByType.get(type)?.push(entry.id)

    // Enforce max in-memory limit
    if (this.entries.size > this.config.maxInMemory) {
      this.evictOldest(this.entries.size - this.config.maxInMemory)
    }

    // Add to pending flush for persistent storage
    this.pendingFlush.push(entry)

    // Publish event
    const Bus = (await import("@/bus")).Bus
    await Bus.publish(ObserverEvent.HistoryRecorded, {
      eventType: type,
      eventId: entry.id,
      storagePath: this.config.storagePath,
    })

    return entry
  }

  /**
   * Query history.
   */
  query(query: HistoryQuery = {}): HistoryEntry[] {
    let results: HistoryEntry[]

    // Filter by type
    if (query.type) {
      const types = Array.isArray(query.type) ? query.type : [query.type]
      const ids = types.flatMap((t) => this.entriesByType.get(t) ?? [])
      results = ids.map((id) => this.entries.get(id)).filter(Boolean) as HistoryEntry[]
    } else {
      results = Array.from(this.entries.values())
    }

    // Filter by time range
    if (query.startTime) {
      results = results.filter((e) => e.timestamp >= query.startTime!)
    }
    if (query.endTime) {
      results = results.filter((e) => e.timestamp <= query.endTime!)
    }

    // Filter by tags
    if (query.tags && query.tags.length > 0) {
      results = results.filter((e) =>
        query.tags!.some((tag) => e.tags.includes(tag)),
      )
    }

    // Filter by session
    if (query.sessionId) {
      results = results.filter((e) => e.sessionId === query.sessionId)
    }

    // Sort by timestamp descending
    results.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())

    // Apply pagination
    const offset = query.offset ?? 0
    const limit = query.limit ?? 100
    return results.slice(offset, offset + limit)
  }

  /**
   * Get entry by ID.
   */
  get(id: string): HistoryEntry | null {
    return this.entries.get(id) ?? null
  }

  /**
   * Get statistics.
   */
  getStats(): HistoryStats {
    const byType: Record<HistoryEventType, number> = {
      observation: 0,
      pattern: 0,
      anomaly: 0,
      opportunity: 0,
      world_model: 0,
      mode_decision: 0,
      escalation: 0,
      execution: 0,
    }

    for (const [type, ids] of this.entriesByType) {
      byType[type] = ids.length
    }

    const entries = Array.from(this.entries.values())
    const sessions = new Set(entries.map((e) => e.sessionId).filter(Boolean))

    let oldest: Date | undefined
    let newest: Date | undefined

    if (entries.length > 0) {
      const sorted = entries.sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime())
      oldest = sorted[0].timestamp
      newest = sorted[sorted.length - 1].timestamp
    }

    return {
      totalEntries: this.entries.size,
      byType,
      oldestEntry: oldest,
      newestEntry: newest,
      sessionCount: sessions.size,
    }
  }

  /**
   * Get recent entries of a type.
   */
  getRecent(type: HistoryEventType, limit = 10): HistoryEntry[] {
    return this.query({ type, limit })
  }

  /**
   * Search entries by data content.
   */
  search(predicate: (entry: HistoryEntry) => boolean, limit = 100): HistoryEntry[] {
    const results: HistoryEntry[] = []
    for (const entry of this.entries.values()) {
      if (predicate(entry)) {
        results.push(entry)
        if (results.length >= limit) break
      }
    }
    return results.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
  }

  /**
   * Flush pending entries to storage.
   */
  async flush(): Promise<void> {
    if (this.pendingFlush.length === 0) return

    const toFlush = [...this.pendingFlush]
    this.pendingFlush = []

    try {
      // Convert to ObserverHistoryEntry format
      const memoryEntries: ObserverHistoryEntry[] = toFlush.map((entry) => ({
        id: entry.id,
        type: entry.type as ObserverHistoryEntry["type"],
        timestamp: entry.timestamp,
        data: entry.data,
        tags: entry.tags,
        sessionId: entry.sessionId,
      }))

      // Write to memory system
      const memoryClient = getMemoryClient({
        sessionId: this.config.sessionId,
      })

      await memoryClient.recordBatch(memoryEntries)

      log.debug("Flushed history entries to memory", {
        count: toFlush.length,
        path: memoryClient.getPath(),
      })
    } catch (error) {
      // Put entries back in queue
      this.pendingFlush.unshift(...toFlush)
      log.error("Failed to flush history", { error: String(error) })
    }
  }

  /**
   * Clear all history.
   */
  clear(): void {
    this.entries.clear()
    for (const ids of this.entriesByType.values()) {
      ids.length = 0
    }
    this.pendingFlush = []
  }

  /**
   * Export history.
   */
  export(query?: HistoryQuery): string {
    const entries = this.query(query)
    return JSON.stringify(entries, null, 2)
  }

  /**
   * Import history.
   */
  import(json: string): number {
    const entries = JSON.parse(json) as HistoryEntry[]
    let imported = 0

    for (const entry of entries) {
      // Restore dates
      entry.timestamp = new Date(entry.timestamp)

      if (!this.entries.has(entry.id)) {
        this.entries.set(entry.id, entry)
        this.entriesByType.get(entry.type)?.push(entry.id)
        imported++
      }
    }

    return imported
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private evictOldest(count: number): void {
    const sorted = Array.from(this.entries.entries())
      .sort(([, a], [, b]) => a.timestamp.getTime() - b.timestamp.getTime())

    for (let i = 0; i < count && i < sorted.length; i++) {
      const [id, entry] = sorted[i]
      this.entries.delete(id)

      const typeIds = this.entriesByType.get(entry.type)
      if (typeIds) {
        const idx = typeIds.indexOf(id)
        if (idx >= 0) typeIds.splice(idx, 1)
      }
    }
  }
}

/**
 * Create a historian.
 */
export function createHistorian(config?: Partial<HistorianConfig>): Historian {
  return new Historian(config)
}
