/**
 * Observer Tower
 *
 * Multi-layer observation structure where each level observes
 * the output of previous levels.
 *
 * Architecture:
 * - Level 0: Raw Observations (wraps watchers)
 * - Level 1: Pattern Recognition (patterns, anomalies, opportunities)
 * - Level 2: Meta Observation (system health, coverage, blind spots)
 * - Level N: Emergent (future: higher-order insights)
 *
 * @module observer/tower
 */

import { Log } from "@/util/log"
import { ObserverLevel, type LevelOutput, type LevelStatus } from "./level"
import { RawObservationLevel, type Level0Config, createRawObservationLevel } from "./levels/raw"
import { PatternRecognitionLevel, type Level1Config, createPatternRecognitionLevel } from "./levels/pattern"
import { MetaObservationLevel, type Level2Config, createMetaObservationLevel } from "./levels/meta"

const log = Log.create({ service: "observer.tower" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Tower configuration.
 */
export interface ObserverTowerConfig {
  /** Level 0 configuration */
  level0?: Level0Config | false
  /** Level 1 configuration */
  level1?: Level1Config | false
  /** Level 2 configuration */
  level2?: Level2Config | false
  /** Whether to auto-wire levels */
  autoWire?: boolean
}

/**
 * Tower status.
 */
export interface TowerStatus {
  running: boolean
  levels: LevelStatus[]
  totalOutputs: number
  uptime: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Observer Tower
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Observer Tower - multi-layer observation structure.
 */
export class ObserverTower {
  private levels: Map<number, ObserverLevel> = new Map()
  private running = false
  private startTime: Date | null = null
  private outputHandlers: ((output: LevelOutput) => void)[] = []

  constructor(config: ObserverTowerConfig = {}) {
    // Create levels based on config
    if (config.level0 !== false) {
      const level0 = createRawObservationLevel(typeof config.level0 === "object" ? config.level0 : {})
      this.addLevel(level0)
    }

    if (config.level1 !== false) {
      const level1 = createPatternRecognitionLevel(typeof config.level1 === "object" ? config.level1 : {})
      this.addLevel(level1)
    }

    if (config.level2 !== false) {
      const level2 = createMetaObservationLevel(typeof config.level2 === "object" ? config.level2 : {})
      this.addLevel(level2)
    }

    // Auto-wire levels
    if (config.autoWire !== false) {
      this.wirelevels()
    }
  }

  /**
   * Add a level to the tower.
   */
  addLevel(level: ObserverLevel): void {
    if (this.levels.has(level.level)) {
      log.warn("Replacing existing level", { level: level.level })
    }
    this.levels.set(level.level, level)
  }

  /**
   * Get a level by number.
   */
  getLevel(levelNum: number): ObserverLevel | undefined {
    return this.levels.get(levelNum)
  }

  /**
   * Get Level 0 (raw observations).
   */
  get level0(): RawObservationLevel | undefined {
    return this.levels.get(0) as RawObservationLevel | undefined
  }

  /**
   * Get Level 1 (pattern recognition).
   */
  get level1(): PatternRecognitionLevel | undefined {
    return this.levels.get(1) as PatternRecognitionLevel | undefined
  }

  /**
   * Get Level 2 (meta observation).
   */
  get level2(): MetaObservationLevel | undefined {
    return this.levels.get(2) as MetaObservationLevel | undefined
  }

  /**
   * Wire levels together (each level feeds the next).
   */
  private wirelevels(): void {
    const sortedLevels = [...this.levels.entries()].sort((a, b) => a[0] - b[0])

    for (let i = 0; i < sortedLevels.length; i++) {
      const [levelNum, level] = sortedLevels[i]

      // Each level's output goes to all higher levels
      level.onOutput((output) => {
        // Forward to all handlers
        for (const handler of this.outputHandlers) {
          handler(output)
        }

        // Forward to higher levels
        for (let j = i + 1; j < sortedLevels.length; j++) {
          const [, higherLevel] = sortedLevels[j]
          higherLevel.process(output).catch((err) => {
            log.error("Level processing error", {
              fromLevel: levelNum,
              toLevel: sortedLevels[j][0],
              error: err instanceof Error ? err.message : String(err),
            })
          })
        }
      })
    }

    log.debug("Levels wired", { count: this.levels.size })
  }

  /**
   * Start all levels.
   */
  async start(): Promise<void> {
    if (this.running) {
      log.warn("Tower already running")
      return
    }

    this.startTime = new Date()

    // Start levels in order
    const sortedLevels = [...this.levels.entries()].sort((a, b) => a[0] - b[0])

    for (const [levelNum, level] of sortedLevels) {
      await level.start()
    }

    this.running = true

    log.info("Observer Tower started", { levels: this.levels.size })
  }

  /**
   * Stop all levels.
   */
  async stop(): Promise<void> {
    if (!this.running) return

    // Stop levels in reverse order
    const sortedLevels = [...this.levels.entries()].sort((a, b) => b[0] - a[0])

    for (const [levelNum, level] of sortedLevels) {
      await level.stop()
    }

    this.running = false

    log.info("Observer Tower stopped")
  }

  /**
   * Subscribe to all outputs from the tower.
   */
  onOutput(handler: (output: LevelOutput) => void): () => void {
    this.outputHandlers.push(handler)
    return () => {
      const idx = this.outputHandlers.indexOf(handler)
      if (idx >= 0) this.outputHandlers.splice(idx, 1)
    }
  }

  /**
   * Get outputs from a specific level.
   */
  getLevelOutputs(levelNum: number, limit = 100): LevelOutput[] {
    const level = this.levels.get(levelNum)
    return level?.getOutputs(limit) ?? []
  }

  /**
   * Get all recent outputs.
   */
  getAllOutputs(limit = 100): LevelOutput[] {
    const all: LevelOutput[] = []

    for (const level of this.levels.values()) {
      all.push(...level.getOutputs(limit))
    }

    return all.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime()).slice(0, limit)
  }

  /**
   * Get tower status.
   */
  getStatus(): TowerStatus {
    const levels = [...this.levels.values()].map((l) => l.getStatus())
    const totalOutputs = levels.reduce((sum, l) => sum + l.outputCount, 0)

    return {
      running: this.running,
      levels,
      totalOutputs,
      uptime: this.startTime ? Date.now() - this.startTime.getTime() : 0,
    }
  }

  /**
   * Check if tower is running.
   */
  isRunning(): boolean {
    return this.running
  }
}

/**
 * Create an Observer Tower.
 */
export function createObserverTower(config?: ObserverTowerConfig): ObserverTower {
  return new ObserverTower(config)
}

// ─────────────────────────────────────────────────────────────────────────────
// Exports
// ─────────────────────────────────────────────────────────────────────────────

export { ObserverLevel, type LevelOutput, type LevelStatus, type LevelConfig, type LevelData, type LevelInsight } from "./level"
export { RawObservationLevel, createRawObservationLevel, type Level0Config } from "./levels/raw"
export { PatternRecognitionLevel, createPatternRecognitionLevel, type Level1Config } from "./levels/pattern"
export { MetaObservationLevel, createMetaObservationLevel, type Level2Config } from "./levels/meta"
