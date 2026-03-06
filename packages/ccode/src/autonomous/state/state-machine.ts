/**
 * State Machine for Autonomous Mode
 *
 * This module provides a TypeScript wrapper around the native Rust state machine implementation.
 * The native implementation offers:
 * - Compile-time state transition validation where possible
 * - Efficient state history tracking with bounded memory
 * - Loop detection for preventing infinite state cycles
 * - Time tracking for performance analysis
 *
 * @package autonomous
 * @see services/zero-core/src/autonomous/state.rs - Rust implementation
 * @see services/zero-core/src/napi/autonomous.rs - NAPI bindings
 */

import { Log } from "@/util/log"
import { AutonomousState, isValidTransition, StateMetadata, getStateCategory } from "./states"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { createStateMachine as rawNativeCreateStateMachine } from "@codecoder-ai/core"

const log = Log.create({ service: "autonomous.state-machine" })

// ============================================================================
// Native Type Definitions
// ============================================================================

interface NativeStateMachineConfig {
  maxHistory?: number
}

interface NativeStateMetadata {
  state: string
  enteredAt: number
  previousState?: string
  reason?: string
}

interface NativeTransitionResult {
  success: boolean
  fromState: string
  toState: string
  metadata?: NativeStateMetadata
  error?: string
}

interface NativeStateMachineHandle {
  state(): string
  category(): string
  isState(state: string): boolean
  isTerminal(): boolean
  isRecoverable(): boolean
  validTransitions(): string[]
  canTransitionTo(target: string): boolean
  transition(to: string, reason?: string): NativeTransitionResult
  forceTransition(to: string, reason?: string): NativeTransitionResult
  history(): NativeStateMetadata[]
  previousState(): string | null
  stateVisitCount(state: string): number
  detectLoop(state: string, threshold: number): boolean
  reset(): void
  timeInCurrentState(): number
  totalTimeInState(state: string): number
  serialize(): string
}

type NativeCreateStateMachine = (config?: NativeStateMachineConfig) => NativeStateMachineHandle

// Cast the raw import to our properly typed function
const nativeCreateStateMachine = rawNativeCreateStateMachine as unknown as NativeCreateStateMachine | undefined

// ============================================================================
// Native Binding Validation
// ============================================================================

if (!nativeCreateStateMachine) {
  throw new Error(
    "Native state machine bindings not available. Ensure @codecoder-ai/core is built with 'bun run build' in packages/core",
  )
}

// ============================================================================
// Configuration
// ============================================================================

export interface TransitionOptions {
  reason?: string
  metadata?: Record<string, unknown>
}

export interface StateMachineConfig {
  onStateChange?: (from: AutonomousState, to: AutonomousState, metadata: StateMetadata) => void | Promise<void>
  onInvalidTransition?: (from: AutonomousState, to: AutonomousState) => void | Promise<void>
  maxStateHistory?: number
}

// ============================================================================
// State Conversion Helpers
// ============================================================================

// State mapping (TS snake_case → Native PascalCase)
function toNativeState(state: AutonomousState): string {
  return state
    .split("_")
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1).toLowerCase())
    .join("")
}

// State mapping (Native PascalCase → TS snake_case)
function fromNativeState(state: string): AutonomousState {
  const snakeCase = state.replace(/([A-Z])/g, "_$1").toLowerCase().slice(1)
  return (snakeCase as AutonomousState) || AutonomousState.IDLE
}

// ============================================================================
// State Machine Class
// ============================================================================

/**
 * State machine for autonomous mode execution.
 *
 * Manages state transitions with validation, history tracking, and event publishing.
 * Uses native Rust implementation via NAPI bindings for performance.
 */
export class StateMachine {
  private native: NativeStateMachineHandle
  private config: StateMachineConfig

  constructor(config: StateMachineConfig = {}) {
    this.config = {
      maxStateHistory: 100,
      ...config,
    }

    // Create native state machine (fail-fast if unavailable)
    // Note: nativeCreateStateMachine is validated as non-null at module load
    this.native = nativeCreateStateMachine!({
      maxHistory: this.config.maxStateHistory,
    })

    log.debug("StateMachine created", { maxHistory: this.config.maxStateHistory })
  }

  /**
   * Get the current state
   */
  getState(): AutonomousState {
    return fromNativeState(this.native.state())
  }

  /**
   * Check if in a specific state
   */
  is(state: AutonomousState): boolean {
    return this.getState() === state
  }

  /**
   * Check if in any of the given states
   */
  isIn(states: AutonomousState[]): boolean {
    return states.includes(this.getState())
  }

  /**
   * Get state history
   */
  getHistory(): StateMetadata[] {
    const history = this.native.history()
    return history.map((h) => ({
      state: fromNativeState(h.state),
      enteredAt: h.enteredAt,
      previousState: h.previousState ? fromNativeState(h.previousState) : undefined,
      reason: h.reason,
    }))
  }

  /**
   * Get previous state
   */
  getPreviousState(): AutonomousState | undefined {
    const prev = this.native.previousState()
    return prev ? fromNativeState(prev) : undefined
  }

  /**
   * Count how many times a state has been visited
   */
  getStateVisitCount(state: AutonomousState): number {
    return this.native.stateVisitCount(toNativeState(state))
  }

  /**
   * Detect if we're in a loop (state visited threshold times in recent history)
   */
  detectLoop(state: AutonomousState, threshold = 3): boolean {
    return this.native.detectLoop(toNativeState(state), threshold)
  }

  /**
   * Attempt a state transition
   *
   * @returns true if transition succeeded, false if invalid
   */
  async transition(to: AutonomousState, options: TransitionOptions = {}): Promise<boolean> {
    const from = this.getState()

    // Validate transition using TS rules (matches Rust implementation)
    if (!isValidTransition(from, to)) {
      log.warn("Invalid state transition", { from, to })
      await this.config.onInvalidTransition?.(from, to)
      await Bus.publish(AutonomousEvent.InvalidTransition, {
        from,
        to,
        reason: options.reason ?? "Invalid state transition",
      })
      return false
    }

    // Execute transition in native
    const result = this.native.transition(toNativeState(to), options.reason)
    if (!result.success) {
      log.warn("Native transition failed", { error: result.error })
      return false
    }

    // Create metadata for callbacks
    const metadata: StateMetadata = {
      state: to,
      enteredAt: Date.now(),
      previousState: from,
      reason: options.reason,
    }

    log.info("State transition", {
      from,
      to,
      category: getStateCategory(to),
      reason: options.reason,
    })

    // Fire callbacks and events
    await this.config.onStateChange?.(from, to, metadata)
    await Bus.publish(AutonomousEvent.StateChanged, {
      from,
      to,
      metadata: options.metadata ?? {},
    })

    return true
  }

  /**
   * Force a transition (bypasses validation)
   */
  async forceTransition(to: AutonomousState, options: TransitionOptions = {}): Promise<void> {
    const from = this.getState()

    this.native.forceTransition(toNativeState(to), options.reason ?? "Forced transition")

    const metadata: StateMetadata = {
      state: to,
      enteredAt: Date.now(),
      previousState: from,
      reason: options.reason ?? "Forced transition",
    }

    log.warn("Forced state transition", { from, to })

    await this.config.onStateChange?.(from, to, metadata)
    await Bus.publish(AutonomousEvent.StateChanged, {
      from,
      to,
      metadata: { forced: true, ...options.metadata },
    })
  }

  /**
   * Reset to initial state
   */
  async reset(reason = "Reset to initial state"): Promise<void> {
    const from = this.getState()

    this.native.reset()

    log.info("State machine reset", { from, to: AutonomousState.IDLE, reason })

    await Bus.publish(AutonomousEvent.StateChanged, {
      from,
      to: AutonomousState.IDLE,
      metadata: { reset: true, reason },
    })
  }

  /**
   * Get time spent in current state (ms)
   */
  getTimeInCurrentState(): number {
    return this.native.timeInCurrentState()
  }

  /**
   * Get total time spent in a specific state (ms)
   */
  getTotalTimeInState(state: AutonomousState): number {
    return this.native.totalTimeInState(toNativeState(state))
  }

  /**
   * Serialize state machine for persistence
   */
  serialize(): { currentState: AutonomousState; stateHistory: StateMetadata[] } {
    return {
      currentState: this.getState(),
      stateHistory: this.getHistory(),
    }
  }

  /**
   * Restore state machine from serialized data
   */
  static deserialize(data: { currentState: AutonomousState; stateHistory: StateMetadata[] }): StateMachine {
    const sm = new StateMachine()
    // Force transition to the saved state
    if (data.currentState !== AutonomousState.IDLE) {
      sm.native.forceTransition(toNativeState(data.currentState), "Restored from serialized state")
    }
    return sm
  }
}

/**
 * Create a new state machine
 */
export function createStateMachine(config?: StateMachineConfig): StateMachine {
  return new StateMachine(config)
}
