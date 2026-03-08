/**
 * Dial Control Types for Observer Network
 *
 * Provides independent control over three dimensions:
 * - Observe: 0% = passive wait, 100% = active scanning
 * - Decide: 0% = suggest only, 100% = autonomous decision
 * - Act: 0% = wait for confirmation, 100% = immediate execution
 *
 * @module observer/dial
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "observer.dial" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Dial operating mode.
 */
export type DialMode = "manual" | "adaptive"

/**
 * Gear presets for intuitive control.
 *
 * Like a car's gear selector:
 * - P (Park): Everything off, no resource consumption
 * - N (Neutral): Observe only, no intervention
 * - D (Drive): Balanced autonomy for daily use
 * - S (Sport): High autonomy, aggressive mode
 * - M (Manual): Full user control over each dial
 */
export type GearPreset = "P" | "N" | "D" | "S" | "M"

/**
 * Dial names for type safety.
 */
export type DialName = "observe" | "decide" | "act"

/**
 * Configuration for a single dial.
 */
export interface DialConfig {
  /** Current value (0-100) */
  value: number
  /** Operating mode */
  mode: DialMode
  /** Bounds for adaptive mode */
  bounds: {
    min: number
    max: number
  }
}

/**
 * Three dials configuration.
 */
export interface ThreeDialsConfig {
  observe: DialConfig
  decide: DialConfig
  act: DialConfig
}

/**
 * Simple dial values (without mode and bounds).
 */
export interface DialValues {
  observe: number
  decide: number
  act: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Gear Presets
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Preset dial values for each gear.
 */
export const GEAR_PRESETS: Record<GearPreset, DialValues> = {
  P: { observe: 0, decide: 0, act: 0 },     // Park: everything off
  N: { observe: 50, decide: 0, act: 0 },    // Neutral: observe only
  D: { observe: 70, decide: 60, act: 40 },  // Drive: balanced autonomy
  S: { observe: 90, decide: 80, act: 70 },  // Sport: high autonomy
  M: { observe: 50, decide: 50, act: 50 },  // Manual: neutral starting point
}

/**
 * Gear metadata for display.
 */
export const GEAR_INFO: Record<GearPreset, { name: string; description: string }> = {
  P: { name: "Park", description: "System inactive, no resource consumption" },
  N: { name: "Neutral", description: "Observe and record only, no intervention" },
  D: { name: "Drive", description: "Balanced autonomy for daily operation" },
  S: { name: "Sport", description: "High autonomy, aggressive mode" },
  M: { name: "Manual", description: "Full manual control over each dial" },
}

// ─────────────────────────────────────────────────────────────────────────────
// Dial Class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A single dial with value, mode, and adaptive bounds.
 */
export class Dial {
  private _value: number
  private _mode: DialMode
  private _min: number
  private _max: number

  constructor(value = 50, mode: DialMode = "manual", min = 0, max = 100) {
    this._value = this.clamp(value, 0, 100)
    this._mode = mode
    this._min = this.clamp(min, 0, 100)
    this._max = this.clamp(max, this._min, 100)
  }

  /** Get current value. */
  get value(): number {
    return this._value
  }

  /** Set value, respecting bounds in adaptive mode. */
  set value(v: number) {
    const clamped = this.clamp(v, 0, 100)
    this._value = this._mode === "adaptive"
      ? this.clamp(clamped, this._min, this._max)
      : clamped
  }

  /** Get operating mode. */
  get mode(): DialMode {
    return this._mode
  }

  /** Set operating mode. */
  set mode(m: DialMode) {
    this._mode = m
    if (m === "adaptive") {
      this._value = this.clamp(this._value, this._min, this._max)
    }
  }

  /** Get bounds. */
  get bounds(): { min: number; max: number } {
    return { min: this._min, max: this._max }
  }

  /** Set bounds (for adaptive mode). */
  setBounds(min: number, max: number): void {
    this._min = this.clamp(min, 0, 100)
    this._max = this.clamp(max, this._min, 100)
    if (this._mode === "adaptive") {
      this._value = this.clamp(this._value, this._min, this._max)
    }
  }

  /** Adjust value by delta. */
  adjust(delta: number): void {
    this.value = this._value + delta
  }

  /** Check if dial is active (value > 0). */
  isActive(): boolean {
    return this._value > 0
  }

  /** Check if dial is above threshold (default 50). */
  isHigh(threshold = 50): boolean {
    return this._value > threshold
  }

  /** Get value as float (0.0 - 1.0). */
  asFloat(): number {
    return this._value / 100
  }

  /** Switch to manual mode. */
  toManual(): void {
    this._mode = "manual"
  }

  /** Switch to adaptive mode with bounds. */
  toAdaptive(min: number, max: number): void {
    this._mode = "adaptive"
    this.setBounds(min, max)
  }

  /** Get config object. */
  toConfig(): DialConfig {
    return {
      value: this._value,
      mode: this._mode,
      bounds: { min: this._min, max: this._max },
    }
  }

  /** Create from config. */
  static fromConfig(config: DialConfig): Dial {
    return new Dial(config.value, config.mode, config.bounds.min, config.bounds.max)
  }

  private clamp(value: number, min: number, max: number): number {
    return Math.max(min, Math.min(max, Math.round(value)))
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// ThreeDials Class
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Three independent dials for observe/decide/act control.
 */
export class ThreeDials {
  /** Observation intensity: 0% = passive, 100% = active scanning */
  readonly observe: Dial
  /** Decision autonomy: 0% = suggest only, 100% = autonomous */
  readonly decide: Dial
  /** Execution autonomy: 0% = wait for confirmation, 100% = immediate */
  readonly act: Dial

  /** Current gear (null if custom values don't match any preset) */
  private _gear: GearPreset | null

  constructor(observe = 70, decide = 60, act = 40, gear: GearPreset | null = null) {
    this.observe = new Dial(observe)
    this.decide = new Dial(decide)
    this.act = new Dial(act)
    this._gear = gear ?? this.detectGear()
  }

  /** Get current gear. */
  get gear(): GearPreset | null {
    return this._gear
  }

  /** Create from a gear preset. */
  static fromGear(gear: GearPreset): ThreeDials {
    const values = GEAR_PRESETS[gear]
    return new ThreeDials(values.observe, values.decide, values.act, gear)
  }

  /** Create with custom values. */
  static custom(observe: number, decide: number, act: number): ThreeDials {
    return new ThreeDials(observe, decide, act, "M")
  }

  /** Create from config. */
  static fromConfig(config: ThreeDialsConfig): ThreeDials {
    const dials = new ThreeDials()
    Object.assign(dials.observe, Dial.fromConfig(config.observe))
    Object.assign(dials.decide, Dial.fromConfig(config.decide))
    Object.assign(dials.act, Dial.fromConfig(config.act))
    return dials
  }

  /** Set gear preset. */
  setGear(gear: GearPreset, customValues?: DialValues): void {
    if (gear === "M" && customValues) {
      this.observe.value = customValues.observe
      this.decide.value = customValues.decide
      this.act.value = customValues.act
    } else {
      const values = GEAR_PRESETS[gear]
      this.observe.value = values.observe
      this.decide.value = values.decide
      this.act.value = values.act
    }
    this._gear = gear

    log.debug("Gear set", { gear, values: this.values() })
  }

  /** Set a single dial and switch to Manual mode. */
  setDial(name: DialName, value: number): void {
    switch (name) {
      case "observe":
        this.observe.value = value
        break
      case "decide":
        this.decide.value = value
        break
      case "act":
        this.act.value = value
        break
    }
    this._gear = "M" // Switch to manual when individual dial is adjusted

    log.debug("Dial set", { dial: name, value, gear: this._gear })
  }

  /** Get dial by name. */
  getDial(name: DialName): Dial {
    switch (name) {
      case "observe":
        return this.observe
      case "decide":
        return this.decide
      case "act":
        return this.act
    }
  }

  /** Check if observation should be active. */
  shouldObserve(): boolean {
    return this.observe.isActive()
  }

  /** Check if observation is in high/active mode. */
  isObservingActively(): boolean {
    return this.observe.isHigh()
  }

  /** Check if decisions should be made autonomously. */
  shouldDecideAutonomously(): boolean {
    return this.decide.isHigh()
  }

  /** Check if actions should be executed immediately. */
  shouldActImmediately(): boolean {
    return this.act.isHigh()
  }

  /** Check if all dials are off (Park mode). */
  isParked(): boolean {
    return !this.observe.isActive() && !this.decide.isActive() && !this.act.isActive()
  }

  /** Get all values as object. */
  values(): DialValues {
    return {
      observe: this.observe.value,
      decide: this.decide.value,
      act: this.act.value,
    }
  }

  /** Get combined autonomy score (average of all dials). */
  autonomyScore(): number {
    return Math.round((this.observe.value + this.decide.value + this.act.value) / 3)
  }

  /** Get config object. */
  toConfig(): ThreeDialsConfig {
    return {
      observe: this.observe.toConfig(),
      decide: this.decide.toConfig(),
      act: this.act.toConfig(),
    }
  }

  /** Detect which gear preset matches current values (if any). */
  private detectGear(): GearPreset | null {
    const v = this.values()
    for (const [gear, preset] of Object.entries(GEAR_PRESETS) as [GearPreset, DialValues][]) {
      if (v.observe === preset.observe && v.decide === preset.decide && v.act === preset.act) {
        return gear
      }
    }
    return null
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create a ThreeDials instance from a gear preset.
 */
export function createDialsFromGear(gear: GearPreset): ThreeDials {
  return ThreeDials.fromGear(gear)
}

/**
 * Create a ThreeDials instance with custom values.
 */
export function createCustomDials(observe: number, decide: number, act: number): ThreeDials {
  return ThreeDials.custom(observe, decide, act)
}

/**
 * Parse a gear string to GearPreset.
 */
export function parseGear(input: string): GearPreset | null {
  const normalized = input.toUpperCase().trim()
  switch (normalized) {
    case "P":
    case "PARK":
      return "P"
    case "N":
    case "NEUTRAL":
      return "N"
    case "D":
    case "DRIVE":
      return "D"
    case "S":
    case "SPORT":
      return "S"
    case "M":
    case "MANUAL":
      return "M"
    default:
      return null
  }
}

/**
 * Check if a gear allows autonomous decisions.
 */
export function gearAllowsAutonomousDecisions(gear: GearPreset): boolean {
  return gear !== "P" && gear !== "N"
}

/**
 * Check if a gear allows autonomous actions.
 */
export function gearAllowsAutonomousActions(gear: GearPreset): boolean {
  return gear === "S"
}
