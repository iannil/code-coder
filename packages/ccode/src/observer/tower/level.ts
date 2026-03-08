/**
 * Observer Level Base Class
 *
 * Abstract base class for all observation levels in the tower.
 * Each level observes the output of previous levels.
 *
 * @module observer/tower/level
 */

import { Log } from "@/util/log"
import type { Observation, EmergentPattern, Anomaly, Opportunity } from "../types"
import { ObserverEvent } from "../events"

const log = Log.create({ service: "observer.tower.level" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Level output type - what a level produces.
 */
export interface LevelOutput {
  /** Level that produced this output */
  level: number
  /** Timestamp of production */
  timestamp: Date
  /** The output data */
  data: LevelData
}

/**
 * Discriminated union of level data types.
 */
export type LevelData =
  | { type: "observation"; observation: Observation }
  | { type: "pattern"; pattern: EmergentPattern }
  | { type: "anomaly"; anomaly: Anomaly }
  | { type: "opportunity"; opportunity: Opportunity }
  | { type: "insight"; insight: LevelInsight }

/**
 * Higher-order insight from emergent layers.
 */
export interface LevelInsight {
  id: string
  name: string
  description: string
  confidence: number
  sources: string[] // IDs of contributing observations/patterns
  suggestedActions: string[]
}

/**
 * Level status for monitoring.
 */
export interface LevelStatus {
  level: number
  name: string
  running: boolean
  health: "healthy" | "degraded" | "failing"
  outputCount: number
  lastOutput: Date | null
  avgLatencyMs: number
}

/**
 * Level configuration.
 */
export interface LevelConfig {
  /** Level number (0, 1, 2, ...) */
  level: number
  /** Human-readable name */
  name: string
  /** Is level enabled */
  enabled: boolean
  /** Processing interval in ms (0 = event-driven) */
  intervalMs: number
  /** Maximum outputs to buffer */
  maxBuffer: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Abstract Base Class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Abstract base class for observation levels.
 */
export abstract class ObserverLevel {
  protected config: LevelConfig
  protected running = false
  protected outputBuffer: LevelOutput[] = []
  protected outputCount = 0
  protected lastOutput: Date | null = null
  protected totalLatencyMs = 0
  protected latencyCount = 0
  protected timer: ReturnType<typeof setInterval> | null = null
  protected handlers: ((output: LevelOutput) => void)[] = []

  constructor(config: Partial<LevelConfig> & { level: number; name: string }) {
    this.config = {
      enabled: true,
      intervalMs: 0,
      maxBuffer: 1000,
      ...config,
    }
  }

  /**
   * Get level number.
   */
  get level(): number {
    return this.config.level
  }

  /**
   * Get level name.
   */
  get name(): string {
    return this.config.name
  }

  /**
   * Start the level.
   */
  async start(): Promise<void> {
    if (this.running) return
    if (!this.config.enabled) {
      log.info("Level disabled, not starting", { level: this.level, name: this.name })
      return
    }

    this.running = true

    // Start periodic processing if configured
    if (this.config.intervalMs > 0) {
      this.timer = setInterval(() => {
        this.tick()
      }, this.config.intervalMs)
    }

    await this.onStart()

    log.info("Level started", { level: this.level, name: this.name })
  }

  /**
   * Stop the level.
   */
  async stop(): Promise<void> {
    if (!this.running) return

    this.running = false

    if (this.timer) {
      clearInterval(this.timer)
      this.timer = null
    }

    await this.onStop()

    log.info("Level stopped", { level: this.level, name: this.name })
  }

  /**
   * Process input from previous levels.
   */
  async process(input: LevelOutput): Promise<void> {
    if (!this.running) return

    const startTime = Date.now()

    try {
      const outputs = await this.onProcess(input)

      for (const output of outputs) {
        this.emit(output)
      }

      const latency = Date.now() - startTime
      this.totalLatencyMs += latency
      this.latencyCount++
    } catch (error) {
      log.error("Level processing error", {
        level: this.level,
        error: error instanceof Error ? error.message : String(error),
      })
    }
  }

  /**
   * Subscribe to outputs from this level.
   */
  onOutput(handler: (output: LevelOutput) => void): () => void {
    this.handlers.push(handler)
    return () => {
      const idx = this.handlers.indexOf(handler)
      if (idx >= 0) this.handlers.splice(idx, 1)
    }
  }

  /**
   * Get recent outputs.
   */
  getOutputs(limit = 100): LevelOutput[] {
    return this.outputBuffer.slice(-limit)
  }

  /**
   * Get level status.
   */
  getStatus(): LevelStatus {
    return {
      level: this.level,
      name: this.name,
      running: this.running,
      health: this.running ? "healthy" : "failing",
      outputCount: this.outputCount,
      lastOutput: this.lastOutput,
      avgLatencyMs: this.latencyCount > 0 ? this.totalLatencyMs / this.latencyCount : 0,
    }
  }

  /**
   * Check if running.
   */
  isRunning(): boolean {
    return this.running
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Protected Methods
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Emit an output to subscribers.
   */
  protected emit(output: LevelOutput): void {
    this.outputBuffer.push(output)
    if (this.outputBuffer.length > this.config.maxBuffer) {
      this.outputBuffer.shift()
    }

    this.outputCount++
    this.lastOutput = new Date()

    for (const handler of this.handlers) {
      try {
        handler(output)
      } catch (error) {
        log.error("Output handler error", {
          level: this.level,
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }
  }

  /**
   * Create a level output.
   */
  protected createOutput(data: LevelData): LevelOutput {
    return {
      level: this.level,
      timestamp: new Date(),
      data,
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Abstract Methods
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Called when level starts.
   */
  protected abstract onStart(): Promise<void>

  /**
   * Called when level stops.
   */
  protected abstract onStop(): Promise<void>

  /**
   * Process input and produce outputs.
   */
  protected abstract onProcess(input: LevelOutput): Promise<LevelOutput[]>

  /**
   * Called on each tick (if intervalMs > 0).
   */
  protected tick(): void {
    // Override in subclasses if needed
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Check if output is an observation.
 */
export function isObservationOutput(output: LevelOutput): output is LevelOutput & { data: { type: "observation" } } {
  return output.data.type === "observation"
}

/**
 * Check if output is a pattern.
 */
export function isPatternOutput(output: LevelOutput): output is LevelOutput & { data: { type: "pattern" } } {
  return output.data.type === "pattern"
}

/**
 * Check if output is an anomaly.
 */
export function isAnomalyOutput(output: LevelOutput): output is LevelOutput & { data: { type: "anomaly" } } {
  return output.data.type === "anomaly"
}

/**
 * Check if output is an opportunity.
 */
export function isOpportunityOutput(output: LevelOutput): output is LevelOutput & { data: { type: "opportunity" } } {
  return output.data.type === "opportunity"
}

/**
 * Check if output is an insight.
 */
export function isInsightOutput(output: LevelOutput): output is LevelOutput & { data: { type: "insight" } } {
  return output.data.type === "insight"
}
