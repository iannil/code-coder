/**
 * Dial Panel
 *
 * User interface for controlling the three dials and gear presets.
 * Connects the dial system to the observer tower and response layer.
 *
 * @module observer/panel
 */

import { Log } from "@/util/log"
import { ThreeDials, type GearPreset, type DialName, type DialValues, GEAR_PRESETS } from "../dial"
import { ObserverEvent } from "../events"
import { gearToOperatingMode } from "../types"
import { GEAR_PRESET_DETAILS, getGearPresetDetail, validateGearTransition, type GearPresetDetail } from "./presets"

const log = Log.create({ service: "observer.panel" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Panel configuration.
 */
export interface DialPanelConfig {
  /** Initial gear */
  initialGear?: GearPreset
  /** Initial custom dials (for Manual mode) */
  initialDials?: DialValues
  /** Whether to emit events on changes */
  emitEvents?: boolean
  /** Require confirmation for gear upgrades */
  requireConfirmation?: boolean
}

/**
 * Panel state.
 */
export interface PanelState {
  gear: GearPreset
  dials: DialValues
  previousGear: GearPreset | null
  lastChange: Date | null
  changeCount: number
}

/**
 * Dial change event.
 */
export interface DialChangeEvent {
  type: "gear" | "dial"
  gear: GearPreset
  dials: DialValues
  previousGear?: GearPreset
  changedDial?: DialName
  timestamp: Date
  reason?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Dial Panel
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Dial Panel - control interface for the three-dial system.
 */
export class DialPanel {
  private dials: ThreeDials
  private previousGear: GearPreset | null = null
  private lastChange: Date | null = null
  private changeCount = 0
  private emitEvents: boolean
  private requireConfirmation: boolean
  private handlers: ((event: DialChangeEvent) => void)[] = []

  constructor(config: DialPanelConfig = {}) {
    const gear = config.initialGear ?? "D"
    this.emitEvents = config.emitEvents ?? true
    this.requireConfirmation = config.requireConfirmation ?? true

    if (gear === "M" && config.initialDials) {
      this.dials = ThreeDials.custom(
        config.initialDials.observe,
        config.initialDials.decide,
        config.initialDials.act,
      )
    } else {
      this.dials = ThreeDials.fromGear(gear)
    }

    log.debug("DialPanel initialized", { gear, dials: this.dials.values() })
  }

  /**
   * Get current gear.
   */
  get gear(): GearPreset {
    return this.dials.gear ?? "M"
  }

  /**
   * Get current state.
   */
  getState(): PanelState {
    return {
      gear: this.gear,
      dials: this.dials.values(),
      previousGear: this.previousGear,
      lastChange: this.lastChange,
      changeCount: this.changeCount,
    }
  }

  /**
   * Get current dial values.
   */
  getDials(): DialValues {
    return this.dials.values()
  }

  /**
   * Get a specific dial value.
   */
  getDial(name: DialName): number {
    return this.dials.getDial(name).value
  }

  /**
   * Set gear preset.
   */
  async setGear(gear: GearPreset, customDials?: DialValues, reason?: string): Promise<boolean> {
    const currentGear = this.gear

    // Validate transition
    if (this.requireConfirmation) {
      const validation = validateGearTransition(currentGear, gear)
      if (validation.requiresConfirmation) {
        log.warn("Gear transition requires confirmation", {
          from: currentGear,
          to: gear,
          reason: validation.reason,
        })
        // In a real implementation, this would prompt the user
        // For now, we proceed but log the warning
      }
    }

    this.previousGear = currentGear
    this.dials.setGear(gear, customDials)
    this.lastChange = new Date()
    this.changeCount++

    const event: DialChangeEvent = {
      type: "gear",
      gear,
      dials: this.dials.values(),
      previousGear: currentGear,
      timestamp: this.lastChange,
      reason,
    }

    await this.emitChange(event)

    log.info("Gear changed", {
      from: currentGear,
      to: gear,
      dials: this.dials.values(),
      reason,
    })

    return true
  }

  /**
   * Set a single dial value.
   */
  async setDial(name: DialName, value: number, reason?: string): Promise<boolean> {
    const previousGear = this.gear
    this.dials.setDial(name, value)
    this.lastChange = new Date()
    this.changeCount++

    const event: DialChangeEvent = {
      type: "dial",
      gear: "M", // Always Manual when individual dial is set
      dials: this.dials.values(),
      previousGear,
      changedDial: name,
      timestamp: this.lastChange,
      reason,
    }

    await this.emitChange(event)

    log.info("Dial changed", {
      dial: name,
      value,
      gear: "M",
      reason,
    })

    return true
  }

  /**
   * Adjust a dial by delta.
   */
  async adjustDial(name: DialName, delta: number, reason?: string): Promise<boolean> {
    const current = this.getDial(name)
    return this.setDial(name, current + delta, reason)
  }

  /**
   * Reset to default (Drive mode).
   */
  async reset(reason?: string): Promise<void> {
    await this.setGear("D", undefined, reason ?? "Reset to default")
  }

  /**
   * Check if observation should be active.
   */
  shouldObserve(): boolean {
    return this.dials.shouldObserve()
  }

  /**
   * Check if decisions should be autonomous.
   */
  shouldDecideAutonomously(): boolean {
    return this.dials.shouldDecideAutonomously()
  }

  /**
   * Check if actions should be immediate.
   */
  shouldActImmediately(): boolean {
    return this.dials.shouldActImmediately()
  }

  /**
   * Get autonomy score (0-100).
   */
  autonomyScore(): number {
    return this.dials.autonomyScore()
  }

  /**
   * Subscribe to changes.
   */
  onChange(handler: (event: DialChangeEvent) => void): () => void {
    this.handlers.push(handler)
    return () => {
      const idx = this.handlers.indexOf(handler)
      if (idx >= 0) this.handlers.splice(idx, 1)
    }
  }

  /**
   * Get gear preset details.
   */
  getGearDetails(): GearPresetDetail {
    return getGearPresetDetail(this.gear)
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private
  // ─────────────────────────────────────────────────────────────────────────────

  private async emitChange(event: DialChangeEvent): Promise<void> {
    // Notify handlers
    for (const handler of this.handlers) {
      try {
        handler(event)
      } catch (error) {
        log.error("Change handler error", {
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }

    // Emit bus event if enabled (using legacy OperatingMode for backward compatibility)
    if (this.emitEvents) {
      try {
        const BusModule = await import("@/bus")
        const currentMode = gearToOperatingMode(event.gear)
        const previousMode = event.previousGear ? gearToOperatingMode(event.previousGear) : currentMode

        if (event.type === "gear") {
          await BusModule.Bus.publish(ObserverEvent.ModeSwitched, {
            previousMode,
            newMode: currentMode,
            reason: event.reason ?? "User action",
            timestamp: event.timestamp,
          })
        } else {
          await BusModule.Bus.publish(ObserverEvent.ModeEvaluated, {
            currentMode,
            recommendedMode: currentMode,
            shouldSwitch: false,
            reason: event.reason ?? "Dial adjusted",
            closeScore: this.autonomyScore() / 10, // Scale to 0-10
            risk: (100 - this.autonomyScore()) / 10, // Inverse of autonomy
            confidence: 1.0,
          })
        }
      } catch (error) {
        log.error("Failed to publish event", {
          error: error instanceof Error ? error.message : String(error),
        })
      }
    }
  }
}

/**
 * Create a dial panel.
 */
export function createDialPanel(config?: DialPanelConfig): DialPanel {
  return new DialPanel(config)
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton
// ─────────────────────────────────────────────────────────────────────────────

let panelInstance: DialPanel | null = null

/**
 * Get the global dial panel instance.
 */
export function getDialPanel(config?: DialPanelConfig): DialPanel {
  if (!panelInstance) {
    panelInstance = new DialPanel(config)
  }
  return panelInstance
}

/**
 * Reset the dial panel singleton.
 */
export function resetDialPanel(): void {
  panelInstance = null
}

// ─────────────────────────────────────────────────────────────────────────────
// Exports
// ─────────────────────────────────────────────────────────────────────────────

export { GEAR_PRESET_DETAILS, getGearPresetDetail, getAllGearPresets, suggestGear, getGearRiskLevel, validateGearTransition, type GearPresetDetail } from "./presets"
