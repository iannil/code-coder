/**
 * Observer Network Unified API
 *
 * Simplified user API for the Observer Network with gear presets
 * and three-dial control.
 *
 * @module observer/api
 */

import { Log } from "@/util/log"
import type { GearPreset, DialValues, DialName } from "./dial"
import type { OperatingMode } from "./types"
import { operatingModeToGear, gearToOperatingMode } from "./types"
import { DialPanel, getDialPanel, resetDialPanel, type PanelState, type DialChangeEvent } from "./panel"
import { ObserverTower, createObserverTower, type TowerStatus, type LevelOutput } from "./tower"
import {
  createNotifier,
  createAnalyzer,
  createExecutor,
  type Notifier,
  type Analyzer,
  type Executor,
} from "./responders"

const log = Log.create({ service: "observer.api" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Network configuration.
 */
export interface NetworkConfig {
  /** Initial gear (new API) */
  gear?: GearPreset
  /** Initial mode (legacy API, maps to gear) */
  mode?: OperatingMode
  /** Custom dial values for Manual mode */
  customDials?: DialValues
  /** Tower configuration */
  tower?: {
    level0?: boolean
    level1?: boolean
    level2?: boolean
  }
  /** Responder configuration */
  responders?: {
    notifier?: boolean
    analyzer?: boolean
    executor?: boolean
  }
  /** Auto-start the network */
  autoStart?: boolean
}

/**
 * Network state.
 */
export interface NetworkState {
  /** Whether network is running */
  running: boolean
  /** Current gear */
  gear: GearPreset
  /** Current dial values */
  dials: DialValues
  /** Legacy operating mode */
  mode: OperatingMode
  /** Autonomy score (0-100) */
  autonomy: number
  /** Tower status */
  tower: TowerStatus | null
  /** Panel state */
  panel: PanelState
}

// ─────────────────────────────────────────────────────────────────────────────
// Observer Network
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Observer Network - unified control interface.
 *
 * @example
 * ```typescript
 * import { ObserverNetwork } from "@/observer"
 *
 * // Start with gear preset
 * const network = await ObserverNetwork.start({ gear: 'D' })
 *
 * // Switch gears
 * await network.setGear('S')
 * await network.setGear('M', { observe: 80, decide: 50, act: 30 })
 *
 * // Single dial adjustment
 * await network.setDial('observe', 90)
 *
 * // Get state
 * const state = network.getState()
 *
 * // Stop
 * await network.stop()
 * ```
 */
export class ObserverNetwork {
  private panel: DialPanel
  private tower: ObserverTower | null = null
  private notifier: Notifier | null = null
  private analyzer: Analyzer | null = null
  private executor: Executor | null = null
  private running = false
  private startTime: Date | null = null
  private changeHandlers: ((event: DialChangeEvent) => void)[] = []
  private outputHandlers: ((output: LevelOutput) => void)[] = []

  private constructor(config: NetworkConfig = {}) {
    // Determine initial gear from config
    const gear = config.gear ?? (config.mode ? operatingModeToGear(config.mode) : "D")

    // Reset singleton and create panel with config
    resetDialPanel()
    this.panel = getDialPanel({
      initialGear: gear,
      initialDials: config.customDials,
      emitEvents: true,
    })

    // Subscribe to panel changes
    this.panel.onChange((event) => {
      for (const handler of this.changeHandlers) {
        handler(event)
      }
    })
  }

  /**
   * Create and start an observer network.
   */
  static async start(config: NetworkConfig = {}): Promise<ObserverNetwork> {
    const network = new ObserverNetwork(config)
    if (config.autoStart !== false) {
      await network.start(config)
    }
    return network
  }

  /**
   * Start the network.
   */
  async start(config: NetworkConfig = {}): Promise<void> {
    if (this.running) {
      log.warn("Network already running")
      return
    }

    this.startTime = new Date()

    // Create and start tower
    const towerConfig = config.tower ?? {}
    this.tower = createObserverTower({
      level0: towerConfig.level0 !== false ? {} : false,
      level1: towerConfig.level1 !== false ? {} : false,
      level2: towerConfig.level2 !== false ? {} : false,
    })

    // Subscribe to tower outputs
    this.tower.onOutput((output) => {
      for (const handler of this.outputHandlers) {
        handler(output)
      }
    })

    await this.tower.start()

    // Create and start responders
    const responderConfig = config.responders ?? {}

    if (responderConfig.notifier !== false) {
      this.notifier = createNotifier()
      await this.notifier.start()
    }

    if (responderConfig.analyzer !== false) {
      this.analyzer = createAnalyzer()
      await this.analyzer.start()
    }

    if (responderConfig.executor !== false) {
      this.executor = createExecutor()
      await this.executor.start()
    }

    this.running = true

    log.info("Observer Network started", {
      gear: this.panel.gear,
      dials: this.panel.getDials(),
      tower: this.tower.isRunning(),
    })
  }

  /**
   * Stop the network.
   */
  async stop(): Promise<void> {
    if (!this.running) return

    // Stop responders
    this.notifier?.stop()
    this.analyzer?.stop()
    this.executor?.stop()

    // Stop tower
    await this.tower?.stop()

    this.running = false

    log.info("Observer Network stopped")
  }

  /**
   * Set gear preset.
   *
   * @param gear - Gear preset (P/N/D/S/M)
   * @param customDials - Custom dial values for Manual mode
   * @param reason - Optional reason for the change
   */
  async setGear(gear: GearPreset, customDials?: DialValues, reason?: string): Promise<void> {
    await this.panel.setGear(gear, customDials, reason)
    log.info("Gear changed", { gear, dials: this.panel.getDials(), reason })
  }

  /**
   * Set a single dial value.
   *
   * @param name - Dial name (observe/decide/act)
   * @param value - Value (0-100)
   * @param reason - Optional reason for the change
   */
  async setDial(name: DialName, value: number, reason?: string): Promise<void> {
    await this.panel.setDial(name, value, reason)
    log.info("Dial changed", { dial: name, value, reason })
  }

  /**
   * Adjust a dial by delta.
   *
   * @param name - Dial name (observe/decide/act)
   * @param delta - Change amount
   * @param reason - Optional reason for the change
   */
  async adjustDial(name: DialName, delta: number, reason?: string): Promise<void> {
    await this.panel.adjustDial(name, delta, reason)
  }

  /**
   * Reset to default (Drive mode).
   */
  async reset(reason?: string): Promise<void> {
    await this.panel.reset(reason)
  }

  /**
   * Get current network state.
   */
  getState(): NetworkState {
    const panelState = this.panel.getState()
    return {
      running: this.running,
      gear: panelState.gear,
      dials: panelState.dials,
      mode: gearToOperatingMode(panelState.gear),
      autonomy: this.panel.autonomyScore(),
      tower: this.tower?.getStatus() ?? null,
      panel: panelState,
    }
  }

  /**
   * Get current gear.
   */
  get gear(): GearPreset {
    return this.panel.gear
  }

  /**
   * Get current dials.
   */
  get dials(): DialValues {
    return this.panel.getDials()
  }

  /**
   * Get legacy operating mode.
   */
  get mode(): OperatingMode {
    return gearToOperatingMode(this.panel.gear)
  }

  /**
   * Get autonomy score (0-100).
   */
  get autonomy(): number {
    return this.panel.autonomyScore()
  }

  /**
   * Get the observer tower.
   */
  getTower(): ObserverTower | null {
    return this.tower
  }

  /**
   * Get the dial panel.
   */
  getPanel(): DialPanel {
    return this.panel
  }

  /**
   * Get the notifier.
   */
  getNotifier(): Notifier | null {
    return this.notifier
  }

  /**
   * Get the analyzer.
   */
  getAnalyzer(): Analyzer | null {
    return this.analyzer
  }

  /**
   * Get the executor.
   */
  getExecutor(): Executor | null {
    return this.executor
  }

  /**
   * Check if network is running.
   */
  isRunning(): boolean {
    return this.running
  }

  /**
   * Get uptime in milliseconds.
   */
  uptime(): number {
    return this.startTime ? Date.now() - this.startTime.getTime() : 0
  }

  /**
   * Subscribe to gear/dial changes.
   */
  onChange(handler: (event: DialChangeEvent) => void): () => void {
    this.changeHandlers.push(handler)
    return () => {
      const idx = this.changeHandlers.indexOf(handler)
      if (idx >= 0) this.changeHandlers.splice(idx, 1)
    }
  }

  /**
   * Subscribe to tower outputs.
   */
  onOutput(handler: (output: LevelOutput) => void): () => void {
    this.outputHandlers.push(handler)
    return () => {
      const idx = this.outputHandlers.indexOf(handler)
      if (idx >= 0) this.outputHandlers.splice(idx, 1)
    }
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Shorthand Methods
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Park mode - stop all observation and action.
   */
  async park(reason?: string): Promise<void> {
    await this.setGear("P", undefined, reason ?? "Entering park mode")
  }

  /**
   * Neutral mode - observe only, no action.
   */
  async neutral(reason?: string): Promise<void> {
    await this.setGear("N", undefined, reason ?? "Entering neutral mode")
  }

  /**
   * Drive mode - balanced daily operation.
   */
  async drive(reason?: string): Promise<void> {
    await this.setGear("D", undefined, reason ?? "Entering drive mode")
  }

  /**
   * Sport mode - aggressive, high autonomy.
   */
  async sport(reason?: string): Promise<void> {
    await this.setGear("S", undefined, reason ?? "Entering sport mode")
  }

  /**
   * Manual mode - custom dial control.
   */
  async manual(dials: DialValues, reason?: string): Promise<void> {
    await this.setGear("M", dials, reason ?? "Entering manual mode")
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Exports
// ─────────────────────────────────────────────────────────────────────────────

export { type GearPreset, type DialValues, type DialName } from "./dial"
export { type DialChangeEvent, type PanelState } from "./panel"
export { type LevelOutput, type TowerStatus } from "./tower"
