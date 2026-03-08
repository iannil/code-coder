/**
 * Level 0: Raw Observations
 *
 * The base level of the Observer Tower that wraps existing watchers
 * and forwards their observations as Level 0 outputs.
 *
 * @module observer/tower/levels/raw
 */

import { Log } from "@/util/log"
import type { Observation } from "../../types"
import { ObserverLevel, type LevelOutput, type LevelConfig } from "../level"
import { getEventStream } from "../../event-stream"

const log = Log.create({ service: "observer.tower.level0" })

/**
 * Level 0 configuration.
 */
export interface Level0Config extends Partial<LevelConfig> {
  /** Filter by watcher types */
  watcherTypes?: ("code" | "world" | "self" | "meta")[]
  /** Minimum confidence threshold */
  minConfidence?: number
}

/**
 * Level 0: Raw observation layer.
 *
 * This level wraps the existing watcher system and forwards
 * raw observations to higher levels in the tower.
 */
export class RawObservationLevel extends ObserverLevel {
  private streamUnsubscribe: (() => void) | null = null
  private watcherTypes: Set<string>
  private minConfidence: number

  constructor(config: Level0Config = {}) {
    super({
      level: 0,
      name: "Raw Observations",
      ...config,
    })

    this.watcherTypes = new Set(config.watcherTypes ?? ["code", "world", "self", "meta"])
    this.minConfidence = config.minConfidence ?? 0
  }

  protected async onStart(): Promise<void> {
    // Subscribe to the event stream
    const stream = getEventStream()

    this.streamUnsubscribe = stream.subscribe((observation) => {
      this.handleObservation(observation)
    })

    log.debug("Level 0 subscribed to event stream")
  }

  protected async onStop(): Promise<void> {
    if (this.streamUnsubscribe) {
      this.streamUnsubscribe()
      this.streamUnsubscribe = null
    }

    log.debug("Level 0 unsubscribed from event stream")
  }

  protected async onProcess(input: LevelOutput): Promise<LevelOutput[]> {
    // Level 0 doesn't process inputs from other levels
    // It only forwards observations from the event stream
    return []
  }

  /**
   * Handle an observation from the event stream.
   */
  private handleObservation(observation: Observation): void {
    // Filter by watcher type
    if (!this.watcherTypes.has(observation.watcherType)) {
      return
    }

    // Filter by confidence
    if (observation.confidence < this.minConfidence) {
      return
    }

    // Emit as Level 0 output
    const output = this.createOutput({
      type: "observation",
      observation,
    })

    this.emit(output)
  }

  /**
   * Get observations of a specific type.
   */
  getObservationsByType(watcherType: string): Observation[] {
    return this.getOutputs()
      .filter((o) => o.data.type === "observation")
      .map((o) => (o.data as { type: "observation"; observation: Observation }).observation)
      .filter((obs) => obs.watcherType === watcherType)
  }

  /**
   * Get observation statistics.
   */
  getStats(): { byType: Record<string, number>; total: number } {
    const outputs = this.getOutputs()
    const byType: Record<string, number> = {}
    let total = 0

    for (const output of outputs) {
      if (output.data.type === "observation") {
        const type = output.data.observation.watcherType
        byType[type] = (byType[type] ?? 0) + 1
        total++
      }
    }

    return { byType, total }
  }
}

/**
 * Create a Level 0 instance.
 */
export function createRawObservationLevel(config?: Level0Config): RawObservationLevel {
  return new RawObservationLevel(config)
}
