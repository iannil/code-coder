import { AutonomousState } from "./states"

/**
 * Transition guard function type
 * Returns true if transition is allowed
 */
export type TransitionGuard = (from: AutonomousState, to: AutonomousState, context?: TransitionContext) => boolean | Promise<boolean>

/**
 * Transition context for guards
 */
export interface TransitionContext {
  resourceUsage?: {
    tokensUsed: number
    costUSD: number
    durationMinutes: number
  }
  taskProgress?: {
    completed: number
    total: number
    failed: number
  }
  errorCount?: number
  lastError?: string
}

/**
 * Predefined transition guards
 */

/**
 * Guard: Only allow transition if under resource limit
 */
export function resourceGuard(limit: { maxTokens?: number; maxCostUSD?: number; maxDurationMinutes?: number }): TransitionGuard {
  return (_from, _to, context?) => {
    if (!context?.resourceUsage) return true

    if (limit.maxTokens && context.resourceUsage.tokensUsed > limit.maxTokens) {
      return false
    }
    if (limit.maxCostUSD && context.resourceUsage.costUSD > limit.maxCostUSD) {
      return false
    }
    if (limit.maxDurationMinutes && context.resourceUsage.durationMinutes > limit.maxDurationMinutes) {
      return false
    }
    return true
  }
}

/**
 * Guard: Only allow transition if error count is below threshold
 */
export function errorGuard(maxErrors: number): TransitionGuard {
  return (_from, _to, context?) => {
    const errors = context?.errorCount ?? 0
    return errors < maxErrors
  }
}

/**
 * Guard: Only allow transition if progress is acceptable
 */
export function progressGuard(minProgressRatio = 0.1): TransitionGuard {
  return (_from, _to, context?) => {
    if (!context?.taskProgress) return true
    if (context.taskProgress.total === 0) return true
    const ratio = context.taskProgress.completed / context.taskProgress.total
    return ratio >= minProgressRatio
  }
}

/**
 * Guard: Prevent rapid state oscillation
 */
export class OscillationGuard {
  private recentTransitions: Array<{ from: AutonomousState; to: AutonomousState; time: number }> = []
  private windowMs: number
  private maxOscillations: number

  constructor(windowMs = 60000, maxOscillations = 5) {
    this.windowMs = windowMs
    this.maxOscillations = maxOscillations
  }

  private cleanOldEntries(): void {
    const now = Date.now()
    this.recentTransitions = this.recentTransitions.filter((t) => now - t.time < this.windowMs)
  }

  private countOscillations(from: AutonomousState, to: AutonomousState): number {
    return this.recentTransitions.filter(
      (t) => (t.from === to && t.to === from) || (t.from === from && t.to === to),
    ).length
  }

  guard(): TransitionGuard {
    return (from, to) => {
      this.cleanOldEntries()

      // Check for oscillation between the same two states
      const oscillations = this.countOscillations(from, to)
      if (oscillations >= this.maxOscillations) {
        return false
      }

      // Record this transition
      this.recentTransitions.push({ from, to, time: Date.now() })
      return true
    }
  }

  reset(): void {
    this.recentTransitions = []
  }
}

/**
 * Composite guard: All guards must pass
 */
export function andGuard(...guards: TransitionGuard[]): TransitionGuard {
  return async (from, to, context) => {
    for (const guard of guards) {
      const result = await guard(from, to, context)
      if (!result) return false
    }
    return true
  }
}

/**
 * Composite guard: At least one guard must pass
 */
export function orGuard(...guards: TransitionGuard[]): TransitionGuard {
  return async (from, to, context) => {
    for (const guard of guards) {
      const result = await guard(from, to, context)
      if (result) return true
    }
    return false
  }
}

/**
 * Negate a guard
 */
export function notGuard(guard: TransitionGuard): TransitionGuard {
  return async (from, to, context) => {
    return !(await guard(from, to, context))
  }
}

/**
 * State transition recipes - common transition patterns
 */
export const TransitionRecipes = {
  /**
   * Normal execution flow
   */
  execute: async (transition: (to: AutonomousState) => Promise<boolean>) => {
    await transition(AutonomousState.EXECUTING)
  },

  /**
   * Error recovery flow
   */
  recover: async (transition: (to: AutonomousState) => Promise<boolean>) => {
    await transition(AutonomousState.FIXING)
  },

  /**
   * Decision point flow
   */
  decide: async (transition: (to: AutonomousState) => Promise<boolean>) => {
    await transition(AutonomousState.DECIDING)
  },

  /**
   * Complete flow
   */
  complete: async (transition: (to: AutonomousState) => Promise<boolean>) => {
    await transition(AutonomousState.COMPLETED)
  },

  /**
   * Fail flow
   */
  fail: async (transition: (to: AutonomousState) => Promise<boolean>) => {
    await transition(AutonomousState.FAILED)
  },

  /**
   * Pause flow
   */
  pause: async (transition: (to: AutonomousState) => Promise<boolean>) => {
    await transition(AutonomousState.PAUSED)
  },
}
