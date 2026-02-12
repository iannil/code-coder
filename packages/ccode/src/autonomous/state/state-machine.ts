import { Log } from "@/util/log"
import { AutonomousState, isValidTransition, isTerminal, StateMetadata, getStateCategory } from "./states"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"

const log = Log.create({ service: "autonomous.state-machine" })

/**
 * State transition options
 */
export interface TransitionOptions {
  reason?: string
  metadata?: Record<string, unknown>
}

/**
 * State machine configuration
 */
export interface StateMachineConfig {
  onStateChange?: (from: AutonomousState, to: AutonomousState, metadata: StateMetadata) => void | Promise<void>
  onInvalidTransition?: (from: AutonomousState, to: AutonomousState) => void | Promise<void>
  maxStateHistory?: number
}

/**
 * State machine for Autonomous Mode
 *
 * Manages state transitions with validation and history tracking
 */
export class StateMachine {
  private currentState: AutonomousState = AutonomousState.IDLE
  private stateHistory: StateMetadata[] = []
  private config: StateMachineConfig

  constructor(config: StateMachineConfig = {}) {
    this.config = {
      maxStateHistory: 100,
      ...config,
    }
  }

  /**
   * Get current state
   */
  getState(): AutonomousState {
    return this.currentState
  }

  /**
   * Check if in a specific state
   */
  is(state: AutonomousState): boolean {
    return this.currentState === state
  }

  /**
   * Check if in any of the given states
   */
  isIn(states: AutonomousState[]): boolean {
    return states.includes(this.currentState)
  }

  /**
   * Get state history
   */
  getHistory(): StateMetadata[] {
    return [...this.stateHistory]
  }

  /**
   * Get last state (before current)
   */
  getPreviousState(): AutonomousState | undefined {
    if (this.stateHistory.length < 2) return undefined
    return this.stateHistory[this.stateHistory.length - 2]?.state
  }

  /**
   * Get count of visits to a state
   */
  getStateVisitCount(state: AutonomousState): number {
    return this.stateHistory.filter((s) => s.state === state).length
  }

  /**
   * Check if we're in a loop (same state visited multiple times)
   */
  detectLoop(state: AutonomousState, threshold = 3): boolean {
    const recent = this.stateHistory.slice(-threshold * 2)
    const count = recent.filter((s) => s.state === state).length
    return count >= threshold
  }

  /**
   * Attempt a state transition
   */
  async transition(to: AutonomousState, options: TransitionOptions = {}): Promise<boolean> {
    const from = this.currentState

    // Validate transition
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

    // Record state history
    const metadata: StateMetadata = {
      state: to,
      enteredAt: Date.now(),
      previousState: from,
      reason: options.reason,
    }

    this.stateHistory.push(metadata)

    // Trim history if needed
    if (this.config.maxStateHistory && this.stateHistory.length > this.config.maxStateHistory) {
      this.stateHistory = this.stateHistory.slice(-this.config.maxStateHistory)
    }

    // Update state
    const previous = this.currentState
    this.currentState = to

    log.info("State transition", {
      from,
      to,
      category: getStateCategory(to),
      reason: options.reason,
    })

    // Notify listeners
    await this.config.onStateChange?.(from, to, metadata)
    await Bus.publish(AutonomousEvent.StateChanged, {
      from,
      to,
      metadata: options.metadata ?? {},
    })

    return true
  }

  /**
   * Force transition (bypasses validation - use with caution)
   */
  async forceTransition(to: AutonomousState, options: TransitionOptions = {}): Promise<void> {
    const from = this.currentState

    const metadata: StateMetadata = {
      state: to,
      enteredAt: Date.now(),
      previousState: from,
      reason: options.reason ?? "Forced transition",
    }

    this.stateHistory.push(metadata)
    this.currentState = to

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
    const from = this.currentState
    this.currentState = AutonomousState.IDLE
    this.stateHistory = []

    log.info("State machine reset", { from, to: AutonomousState.IDLE, reason })

    await Bus.publish(AutonomousEvent.StateChanged, {
      from,
      to: AutonomousState.IDLE,
      metadata: { reset: true, reason },
    })
  }

  /**
   * Get time spent in current state
   */
  getTimeInCurrentState(): number {
    const current = this.stateHistory[this.stateHistory.length - 1]
    if (!current) return 0
    return Date.now() - current.enteredAt
  }

  /**
   * Get total time in a specific state across history
   */
  getTotalTimeInState(state: AutonomousState): number {
    let total = 0
    for (let i = 0; i < this.stateHistory.length; i++) {
      if (this.stateHistory[i]?.state !== state) continue
      const entered = this.stateHistory[i]?.enteredAt ?? 0
      const exited = this.stateHistory[i + 1]?.enteredAt ?? Date.now()
      total += exited - entered
    }
    return total
  }

  /**
   * Serialize state machine for persistence
   */
  serialize(): {
    currentState: AutonomousState
    stateHistory: StateMetadata[]
  } {
    return {
      currentState: this.currentState,
      stateHistory: this.stateHistory,
    }
  }

  /**
   * Restore state machine from serialized data
   */
  static deserialize(data: {
    currentState: AutonomousState
    stateHistory: StateMetadata[]
  }): StateMachine {
    const sm = new StateMachine()
    sm.currentState = data.currentState
    sm.stateHistory = data.stateHistory
    return sm
  }
}

/**
 * Create a new state machine instance
 */
export function createStateMachine(config?: StateMachineConfig): StateMachine {
  return new StateMachine(config)
}
