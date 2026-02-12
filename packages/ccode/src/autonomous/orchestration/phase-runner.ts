import { Log } from "@/util/log"
import { AutonomousEvent } from "../events"
import { Bus } from "@/bus"
import { AutonomousState } from "../state/states"

const log = Log.create({ service: "autonomous.phase-runner" })

/**
 * Phase types in Autonomous Mode execution
 */
export type PhaseType =
  | "understand"
  | "plan"
  | "decide"
  | "execute"
  | "test"
  | "verify"
  | "evaluate"
  | "report"

/**
 * Phase result
 */
export type PhaseResult = "success" | "failure" | "partial" | "skipped"

/**
 * Phase context
 */
export interface PhaseContext {
  sessionId: string
  requestId: string
  input: Record<string, unknown>
  previousPhases: Map<PhaseType, PhaseResult>
  metadata: Record<string, unknown>
}

/**
 * Phase definition
 */
export interface Phase {
  type: PhaseType
  name: string
  description: string
  dependencies: PhaseType[]
  handler: (context: PhaseContext) => Promise<PhaseResult>
  timeout?: number
  retryable?: boolean
}

/**
 * Phase runner configuration
 */
export interface PhaseRunnerConfig {
  timeout: number
  retryAttempts: number
  continueOnFailure: boolean
}

/**
 * Phase runner for executing Autonomous Mode phases
 *
 * Manages the sequential execution of phases with dependency handling
 */
export class PhaseRunner {
  private phases: Map<PhaseType, Phase> = new Map()
  private results: Map<PhaseType, PhaseResult> = new Map()
  private config: PhaseRunnerConfig
  private sessionId: string

  constructor(sessionId: string, config: Partial<PhaseRunnerConfig> = {}) {
    this.sessionId = sessionId
    this.config = {
      timeout: 300000, // 5 minutes default
      retryAttempts: 2,
      continueOnFailure: false,
      ...config,
    }
  }

  /**
   * Register a phase
   */
  register(phase: Phase): void {
    this.phases.set(phase.type, phase)
    log.info("Phase registered", { type: phase.type, name: phase.name })
  }

  /**
   * Get a phase
   */
  getPhase(type: PhaseType): Phase | undefined {
    return this.phases.get(type)
  }

  /**
   * Get all phases
   */
  getAllPhases(): Phase[] {
    return Array.from(this.phases.values())
  }

  /**
   * Get phase result
   */
  getResult(type: PhaseType): PhaseResult | undefined {
    return this.results.get(type)
  }

  /**
   * Get all results
   */
  getAllResults(): Map<PhaseType, PhaseResult> {
    return new Map(this.results)
  }

  /**
   * Check if phase can run (dependencies satisfied)
   */
  canRun(phase: Phase): boolean {
    for (const dep of phase.dependencies) {
      const depResult = this.results.get(dep)
      if (depResult !== "success" && depResult !== "partial") {
        return false
      }
    }
    return true
  }

  /**
   * Run a single phase
   */
  async runPhase(type: PhaseType, context: Omit<PhaseContext, "previousPhases">): Promise<PhaseResult> {
    const phase = this.phases.get(type)
    if (!phase) {
      log.error("Phase not found", { type })
      return "failure"
    }

    // Check dependencies
    if (!this.canRun(phase)) {
      log.warn("Phase dependencies not satisfied", { type, dependencies: phase.dependencies })
      return "skipped"
    }

    const phaseContext: PhaseContext = {
      ...context,
      previousPhases: new Map(this.results),
    }

    log.info("Phase starting", { type, name: phase.name })

    await Bus.publish(AutonomousEvent.PhaseStarted, {
      sessionId: this.sessionId,
      phase: type,
      metadata: { name: phase.name },
    })

    const startTime = Date.now()
    let result: PhaseResult = "failure"

    try {
      // Run with timeout
      const timeout = phase.timeout ?? this.config.timeout
      result = await this.withTimeout(phase.handler(phaseContext), timeout)
    } catch (error) {
      log.error("Phase error", { type, error: error instanceof Error ? error.message : String(error) })
      result = "failure"
    }

    const duration = Date.now() - startTime
    this.results.set(type, result)

    log.info("Phase completed", { type, result, duration })

    await Bus.publish(AutonomousEvent.PhaseCompleted, {
      sessionId: this.sessionId,
      phase: type,
      duration,
      success: result === "success" || result === "partial",
      metadata: { result },
    })

    return result
  }

  /**
   * Run all phases in dependency order
   */
  async runAll(context: Omit<PhaseContext, "previousPhases">): Promise<Map<PhaseType, PhaseResult>> {
    const sorted = this.topologicalSort()

    for (const phase of sorted) {
      // Skip if already run
      if (this.results.has(phase.type)) {
        continue
      }

      // Check if can run
      if (!this.canRun(phase)) {
        log.warn("Skipping phase due to failed dependencies", { type: phase.type })
        this.results.set(phase.type, "skipped")
        continue
      }

      const result = await this.runPhase(phase.type, context)

      // Stop on failure if not continuing
      if (result === "failure" && !this.config.continueOnFailure) {
        log.warn("Stopping phase execution due to failure", { type: phase.type })
        break
      }
    }

    return this.getAllResults()
  }

  /**
   * Run phases up to a specific phase
   */
  async runUpTo(
    targetPhase: PhaseType,
    context: Omit<PhaseContext, "previousPhases">,
  ): Promise<Map<PhaseType, PhaseResult>> {
    const sorted = this.topologicalSort()
    const targetIndex = sorted.findIndex((p) => p.type === targetPhase)

    if (targetIndex === -1) {
      log.error("Target phase not found", { targetPhase })
      return this.getAllResults()
    }

    for (let i = 0; i <= targetIndex; i++) {
      const phase = sorted[i]

      // Skip if already run
      if (this.results.has(phase.type)) {
        continue
      }

      // Check if can run
      if (!this.canRun(phase)) {
        log.warn("Skipping phase due to failed dependencies", { type: phase.type })
        this.results.set(phase.type, "skipped")
        continue
      }

      const result = await this.runPhase(phase.type, context)

      // Stop on failure if not continuing
      if (result === "failure" && !this.config.continueOnFailure) {
        log.warn("Stopping phase execution due to failure", { type: phase.type })
        break
      }
    }

    return this.getAllResults()
  }

  /**
   * Clear results
   */
  clearResults(): void {
    this.results.clear()
  }

  /**
   * Clear everything
   */
  clear(): void {
    this.phases.clear()
    this.results.clear()
  }

  /**
   * Topological sort of phases by dependency
   */
  private topologicalSort(): Phase[] {
    const sorted: Phase[] = []
    const visited = new Set<PhaseType>()
    const visiting = new Set<PhaseType>()

    const visit = (phase: Phase) => {
      if (visited.has(phase.type)) return
      if (visiting.has(phase.type)) {
        throw new Error(`Circular dependency detected involving phase: ${phase.type}`)
      }

      visiting.add(phase.type)

      for (const depType of phase.dependencies) {
        const dep = this.phases.get(depType)
        if (dep) {
          visit(dep)
        }
      }

      visiting.delete(phase.type)
      visited.add(phase.type)
      sorted.push(phase)
    }

    for (const phase of this.phases.values()) {
      visit(phase)
    }

    return sorted
  }

  /**
   * Run promise with timeout
   */
  private async withTimeout<T>(promise: Promise<T>, timeout: number): Promise<T> {
    let timeoutHandle: ReturnType<typeof setTimeout> | undefined

    const timeoutPromise = new Promise<never>((_, reject) => {
      timeoutHandle = setTimeout(() => reject(new Error(`Phase timeout after ${timeout}ms`)), timeout).unref()
    })

    try {
      return await Promise.race([promise, timeoutPromise])
    } finally {
      if (timeoutHandle) {
        clearTimeout(timeoutHandle)
      }
    }
  }
}

/**
 * Create a phase runner with default Autonomous Mode phases
 */
export function createPhaseRunner(sessionId: string, config?: Partial<PhaseRunnerConfig>): PhaseRunner {
  const runner = new PhaseRunner(sessionId, config)

  // Default phases will be registered by the orchestrator
  // This is just the factory function

  return runner
}

/**
 * Phase templates for common Autonomous Mode phases
 */
export const PhaseTemplates = {
  understand: {
    type: "understand" as const,
    name: "Understand",
    description: "Parse and understand the user request",
    dependencies: [],
  },

  plan: {
    type: "plan" as const,
    name: "Plan",
    description: "Generate execution plan",
    dependencies: ["understand" as const],
  },

  decide: {
    type: "decide" as const,
    name: "Decide",
    description: "Evaluate plan using CLOSE framework",
    dependencies: ["plan" as const],
  },

  execute: {
    type: "execute" as const,
    name: "Execute",
    description: "Execute the plan using TDD methodology",
    dependencies: ["decide" as const],
  },

  test: {
    type: "test" as const,
    name: "Test",
    description: "Run tests and verify functionality",
    dependencies: ["execute" as const],
  },

  verify: {
    type: "verify" as const,
    name: "Verify",
    description: "Verify code quality and run checks",
    dependencies: ["test" as const],
  },

  evaluate: {
    type: "evaluate" as const,
    name: "Evaluate",
    description: "Evaluate results and calculate scores",
    dependencies: ["verify" as const],
  },

  report: {
    type: "report" as const,
    name: "Report",
    description: "Generate execution report",
    dependencies: ["evaluate" as const],
  },
}
