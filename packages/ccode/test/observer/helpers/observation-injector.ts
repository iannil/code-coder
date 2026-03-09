/**
 * Observation Injector Helper
 *
 * Utility for injecting test observations into the Observer Network event stream.
 * Used by E2E tests to simulate real-world scenarios.
 *
 * @module test/observer/helpers/observation-injector
 */

import type { EventStream } from "@/observer/event-stream"
import type {
  Observation,
  CodeObservation,
  WorldObservation,
  SelfObservation,
  MetaObservation,
} from "@/observer/types"

export interface InjectionOptions {
  /** Delay between injections in batch mode (ms) */
  delayMs?: number
  /** Whether to wait for event stream processing */
  waitForProcessing?: boolean
}

/**
 * Observation injector for testing purposes.
 */
export class ObservationInjector {
  private stream: EventStream
  private injectionCount = 0

  constructor(stream: EventStream) {
    this.stream = stream
  }

  /**
   * Inject a single observation into the event stream.
   */
  async inject(observation: Observation): Promise<void> {
    await this.stream.ingest(observation)
    this.injectionCount++
  }

  /**
   * Inject multiple observations sequentially.
   */
  async injectBatch(
    observations: Observation[],
    options: InjectionOptions = {},
  ): Promise<void> {
    const { delayMs = 0, waitForProcessing = false } = options

    for (const obs of observations) {
      await this.inject(obs)

      if (delayMs > 0) {
        await Bun.sleep(delayMs)
      }
    }

    if (waitForProcessing) {
      // Allow event loop to process
      await Bun.sleep(100)
    }
  }

  /**
   * Inject observations with escalating severity for crisis simulation.
   */
  async injectCrisisSequence(
    phases: Array<{
      phase: string
      observations: Observation[]
      delayMs?: number
    }>,
  ): Promise<Map<string, number>> {
    const phaseInjections = new Map<string, number>()

    for (const { phase, observations, delayMs = 50 } of phases) {
      const startCount = this.injectionCount
      await this.injectBatch(observations, { delayMs })
      phaseInjections.set(phase, this.injectionCount - startCount)
    }

    return phaseInjections
  }

  /**
   * Create a code observation for testing.
   */
  static createCodeObservation(overrides: Partial<CodeObservation> = {}): CodeObservation {
    const timestamp = new Date()
    return {
      id: `obs_code_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      timestamp,
      watcherId: "code_watch_test",
      watcherType: "code",
      confidence: 0.8,
      tags: [],
      type: "build_status",
      source: "ci/test",
      change: {
        action: "modify",
        before: null,
        after: null,
      },
      impact: {
        scope: "file",
        severity: "low",
        affectedFiles: [],
      },
      ...overrides,
    }
  }

  /**
   * Create a world observation for testing.
   */
  static createWorldObservation(overrides: Partial<WorldObservation> = {}): WorldObservation {
    const timestamp = new Date()
    return {
      id: `obs_world_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      timestamp,
      watcherId: "world_watch_test",
      watcherType: "world",
      confidence: 0.75,
      tags: [],
      type: "api_change",
      source: "external/api",
      data: {
        title: "Test world observation",
        summary: "A test world observation",
        content: {},
      },
      relevance: 0.7,
      ...overrides,
    }
  }

  /**
   * Create a self observation for testing.
   */
  static createSelfObservation(overrides: Partial<SelfObservation> = {}): SelfObservation {
    const timestamp = new Date()
    return {
      id: `obs_self_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      timestamp,
      watcherId: "self_watch_test",
      watcherType: "self",
      confidence: 0.85,
      tags: [],
      type: "agent_behavior",
      agentId: "test_agent",
      observation: {
        action: "test_action",
        success: true,
        duration: 100,
      },
      quality: {
        closeScore: 7.0,
        accuracy: 0.9,
        efficiency: 0.85,
      },
      ...overrides,
    }
  }

  /**
   * Create a meta observation for testing.
   */
  static createMetaObservation(overrides: Partial<MetaObservation> = {}): MetaObservation {
    const timestamp = new Date()
    return {
      id: `obs_meta_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
      timestamp,
      watcherId: "meta_watch_test",
      watcherType: "meta",
      confidence: 0.9,
      tags: [],
      type: "system_health",
      assessment: {
        health: "healthy",
        coverage: 0.8,
        accuracy: 0.85,
        latency: 50,
      },
      recommendations: [],
      issues: [],
      ...overrides,
    }
  }

  /**
   * Get injection statistics.
   */
  getStats(): { injectionCount: number } {
    return {
      injectionCount: this.injectionCount,
    }
  }

  /**
   * Reset injection counter.
   */
  reset(): void {
    this.injectionCount = 0
  }
}
