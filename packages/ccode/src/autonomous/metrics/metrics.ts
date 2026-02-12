import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { AutonomousState } from "../state/states"
import z from "zod"

const log = Log.create({ service: "autonomous.metrics" })

/**
 * Metric types
 */
export type MetricType =
  | "autonomy"
  | "efficiency"
  | "quality"
  | "decision"
  | "resource"
  | "task"
  | "safety"

/**
 * Metric data point
 */
export interface MetricData {
  type: MetricType
  name: string
  value: number
  unit: string
  timestamp: number
  metadata?: Record<string, unknown>
}

/**
 * Session metrics summary
 */
export interface SessionMetrics {
  sessionId: string
  startTime: number
  endTime?: number
  duration: number
  tasks: {
    total: number
    completed: number
    failed: number
    skipped: number
  }
  decisions: {
    total: number
    approved: number
    paused: number
    blocked: number
    averageScore: number
  }
  resources: {
    tokensUsed: number
    costUSD: number
    filesChanged: number
  }
  tests: {
    run: number
    passed: number
    failed: number
    passRate: number
  }
  tdd: {
    cycles: number
    redPassed: number
    greenPassed: number
    refactorPassed: number
  }
  safety: {
    rollbacks: number
    loopsDetected: number
    warnings: number
  }
  states: {
    transitions: number
    finalState: AutonomousState
  }
}

/**
 * Metrics collector
 *
 * Collects and aggregates metrics throughout a Autonomous Mode session
 */
export class MetricsCollector {
  private sessionId: string
  private metrics: Map<string, MetricData[]> = new Map()
  private startTime: number
  private storageKey: string[]

  // Counter for quick increments
  private counters: Map<string, number> = new Map()

  constructor(sessionId: string) {
    this.sessionId = sessionId
    this.startTime = Date.now()
    const projectID = Instance.project.id
    this.storageKey = ["autonomous", "metrics", projectID, sessionId]

    // Initialize counters
    this.initializeCounters()
  }

  /**
   * Record a metric
   */
  record(metric: Omit<MetricData, "timestamp">): void {
    const data: MetricData = {
      ...metric,
      timestamp: Date.now(),
    }

    const key = `${metric.type}:${metric.name}`
    const existing = this.metrics.get(key) ?? []
    existing.push(data)
    this.metrics.set(key, existing)

    // Update counter
    const counterKey = key
    this.counters.set(counterKey, (this.counters.get(counterKey) ?? 0) + data.value)

    log.debug("Metric recorded", { type: metric.type, name: metric.name, value: metric.value })
  }

  /**
   * Increment a counter metric
   */
  increment(type: MetricType, name: string, delta = 1): void {
    const key = `${type}:${name}`
    const current = this.counters.get(key) ?? 0
    this.counters.set(key, current + delta)

    this.record({
      type,
      name,
      value: delta,
      unit: "count",
    })
  }

  /**
   * Get a metric value by type and name
   */
  get(type: MetricType, name: string): number {
    const key = `${type}:${name}`
    return this.counters.get(key) ?? 0
  }

  /**
   * Get all metrics for a type
   */
  getByType(type: MetricType): MetricData[] {
    const result: MetricData[] = []

    for (const [key, data] of this.metrics) {
      if (key.startsWith(`${type}:`)) {
        result.push(...data)
      }
    }

    return result.sort((a, b) => a.timestamp - b.timestamp)
  }

  /**
   * Get metric series over time
   */
  getSeries(type: MetricType, name: string): Array<{ timestamp: number; value: number }> {
    const key = `${type}:${name}`
    const data = this.metrics.get(key) ?? []

    return data.map((d) => ({
      timestamp: d.timestamp,
      value: d.value,
    }))
  }

  /**
   * Get session summary
   */
  getSummary(): SessionMetrics {
    const duration = Date.now() - this.startTime

    return {
      sessionId: this.sessionId,
      startTime: this.startTime,
      duration,
      tasks: {
        total: this.get("task", "total"),
        completed: this.get("task", "completed"),
        failed: this.get("task", "failed"),
        skipped: this.get("task", "skipped"),
      },
      decisions: {
        total: this.get("decision", "total"),
        approved: this.get("decision", "approved"),
        paused: this.get("decision", "paused"),
        blocked: this.get("decision", "blocked"),
        averageScore: this.get("decision", "score_sum") / Math.max(1, this.get("decision", "total")),
      },
      resources: {
        tokensUsed: this.get("resource", "tokens"),
        costUSD: this.get("resource", "cost_usd"),
        filesChanged: this.get("resource", "files_changed"),
      },
      tests: {
        run: this.get("task", "test_run"),
        passed: this.get("task", "test_passed"),
        failed: this.get("task", "test_failed"),
        passRate: this.get("task", "test_passed") / Math.max(1, this.get("task", "test_run")),
      },
      tdd: {
        cycles: this.get("task", "tdd_cycle"),
        redPassed: this.get("task", "tdd_red_success"),
        greenPassed: this.get("task", "tdd_green_success"),
        refactorPassed: this.get("task", "tdd_refactor_success"),
      },
      safety: {
        rollbacks: this.get("safety", "rollback"),
        loopsDetected: this.get("safety", "loop_detected"),
        warnings: this.get("safety", "warning"),
      },
      states: {
        transitions: this.get("autonomy", "state_transition"),
        finalState: AutonomousState.IDLE, // Will be updated by session
      },
    }
  }

  /**
   * Persist metrics to storage
   */
  async persist(): Promise<void> {
    try {
      const data = {
        sessionId: this.sessionId,
        summary: this.getSummary(),
        metrics: Array.from(this.metrics.entries()).flatMap(([key, data]) =>
          data.map((d) => ({
            ...d,
            key,
          }))
        ),
      }

      await Storage.write(this.storageKey, data)
      log.info("Metrics persisted", { sessionId: this.sessionId })
    } catch (error) {
      log.error("Failed to persist metrics", {
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  /**
   * Load metrics from storage
   */
  async load(): Promise<boolean> {
    try {
      const data = await Storage.read<{
        metrics: Array<MetricData & { key: string }>
      }>(this.storageKey)

      if (!data?.metrics) {
        return false
      }

      this.metrics.clear()
      this.counters.clear()

      for (const item of data.metrics) {
        const { key, ...metric } = item
        const existing = this.metrics.get(key) ?? []
        existing.push(metric)
        this.metrics.set(key, existing)

        // Rebuild counters
        this.counters.set(key, (this.counters.get(key) ?? 0) + metric.value)
      }

      log.info("Metrics loaded", { sessionId: this.sessionId, count: data.metrics.length })
      return true
    } catch {
      return false
    }
  }

  /**
   * Clear all metrics
   */
  clear(): void {
    this.metrics.clear()
    this.initializeCounters()
  }

  /**
   * Initialize counters
   */
  private initializeCounters(): void {
    this.counters = new Map(
      Object.entries({
        "task:total": 0,
        "task:completed": 0,
        "task:failed": 0,
        "task:skipped": 0,
        "task:test_run": 0,
        "task:test_passed": 0,
        "task:test_failed": 0,
        "task:tdd_cycle": 0,
        "task:tdd_red_success": 0,
        "task:tdd_green_success": 0,
        "task:tdd_refactor_success": 0,
        "decision:total": 0,
        "decision:approved": 0,
        "decision:paused": 0,
        "decision:blocked": 0,
        "decision:score_sum": 0,
        "resource:tokens": 0,
        "resource:cost_usd": 0,
        "resource:files_changed": 0,
        "safety:rollback": 0,
        "safety:loop_detected": 0,
        "safety:warning": 0,
        "autonomy:state_transition": 0,
      }),
    )
  }

  /**
   * Serialize
   */
  serialize(): {
    metrics: Array<{ key: string } & MetricData>
    counters: Record<string, number>
  } {
    return {
      metrics: Array.from(this.metrics.entries()).flatMap(([key, data]) =>
        data.map((d) => ({ ...d, key }))
      ),
      counters: Object.fromEntries(this.counters),
    }
  }

  /**
   * Deserialize
   */
  static deserialize(
    data: {
      metrics: Array<{ key: string } & MetricData>
      counters: Record<string, number>
    },
    sessionId: string,
  ): MetricsCollector {
    const collector = new MetricsCollector(sessionId)

    collector.metrics.clear()
    collector.counters = new Map(Object.entries(data.counters))

    for (const item of data.metrics) {
      const { key, ...metric } = item
      const existing = collector.metrics.get(key) ?? []
      existing.push(metric)
      collector.metrics.set(key, existing)
    }

    return collector
  }
}

/**
 * Create a metrics collector
 */
export function createMetricsCollector(sessionId: string): MetricsCollector {
  return new MetricsCollector(sessionId)
}

/**
 * Stored session metrics schema
 */
const StoredSessionMetrics = z.object({
  sessionId: z.string(),
  summary: z.object({
    sessionId: z.string(),
    startTime: z.number(),
    endTime: z.number().optional(),
    duration: z.number(),
    tasks: z.object({
      total: z.number(),
      completed: z.number(),
      failed: z.number(),
      skipped: z.number(),
    }),
    decisions: z.object({
      total: z.number(),
      approved: z.number(),
      paused: z.number(),
      blocked: z.number(),
      averageScore: z.number(),
    }),
    resources: z.object({
      tokensUsed: z.number(),
      costUSD: z.number(),
      filesChanged: z.number(),
    }),
    tests: z.object({
      run: z.number(),
      passed: z.number(),
      failed: z.number(),
      passRate: z.number(),
    }),
    tdd: z.object({
      cycles: z.number(),
      redPassed: z.number(),
      greenPassed: z.number(),
      refactorPassed: z.number(),
    }),
    safety: z.object({
      rollbacks: z.number(),
      loopsDetected: z.number(),
      warnings: z.number(),
    }),
    states: z.object({
      transitions: z.number(),
      finalState: z.nativeEnum(AutonomousState),
    }),
  }),
  metrics: z.array(z.object({
    key: z.string(),
    type: z.enum(["autonomy", "efficiency", "quality", "decision", "resource", "task", "safety"]),
    name: z.string(),
    value: z.number(),
    unit: z.string(),
    timestamp: z.number(),
    metadata: z.record(z.string(), z.any()).optional(),
  })),
})

export type StoredSessionMetrics = z.infer<typeof StoredSessionMetrics>

/**
 * Get metrics for a session
 */
export async function getSessionMetrics(sessionId: string): Promise<SessionMetrics | undefined> {
  const projectID = Instance.project.id

  try {
    const data = await Storage.read<StoredSessionMetrics>([
      "autonomous",
      "metrics",
      projectID,
      sessionId,
    ])

    return data?.summary
  } catch {
    return undefined
  }
}

/**
 * Get all session metrics
 */
export async function getAllSessionMetrics(): Promise<SessionMetrics[]> {
  const projectID = Instance.project.id

  try {
    const keys = await Storage.list(["autonomous", "metrics", projectID])
    const summaries: SessionMetrics[] = []

    for (const key of keys) {
      try {
        const data = await Storage.read<StoredSessionMetrics>(key)
        if (data?.summary) {
          summaries.push(data.summary)
        }
      } catch {
        // Skip invalid entries
      }
    }

    return summaries.sort((a, b) => b.startTime - a.startTime)
  } catch {
    return []
  }
}
