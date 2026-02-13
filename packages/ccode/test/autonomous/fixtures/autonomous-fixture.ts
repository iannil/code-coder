import { expect } from "bun:test"
import { tmpdir } from "../../fixture/fixture"
import { Instance } from "@/project/instance"
import { AutonomousState } from "@/autonomous/state/states"
import { StateMachine } from "@/autonomous/state/state-machine"
import { Orchestrator, createOrchestrator, type OrchestratorConfig } from "@/autonomous/orchestration/orchestrator"
import { Executor, createExecutor, type TDDCycleResult } from "@/autonomous/execution/executor"
import { SafetyGuard, parseResourceBudget, type ResourceBudget, type ResourceUsage } from "@/autonomous/safety/constraints"
import {
  SafetyIntegration,
  createSafetyIntegration,
} from "@/autonomous/safety/integration"
import { MetricsCollector, createMetricsCollector } from "@/autonomous/metrics/metrics"
import { Scorer, createScorer } from "@/autonomous/metrics/scorer"

/**
 * Autonomous Mode test fixtures
 *
 * Provides reusable test utilities, mock agents, and test scenarios
 * for autonomous mode testing.
 *
 * IMPORTANT: All autonomous tests must be run within Instance.provide() context.
 * Use createTestAutonomousSession() which handles this automatically.
 */

/**
 * Mock TDD Guide agent response
 */
export interface MockAgentResponse {
  success: boolean
  output: string
  changes: string[]
}

/**
 * Create mock TDD guide agent
 */
export function createMockTDDGuideAgent() {
  return {
    async red(requirement: string): Promise<MockAgentResponse> {
      return {
        success: true,
        output: `Test File: ${requirement.replace(/\s+/g, "-")}.test.ts\nTest: Write failing test for "${requirement}"`,
        changes: [`${requirement.replace(/\s+/g, "-")}.test.ts`],
      }
    },

    async green(testFile: string): Promise<MockAgentResponse> {
      const implFile = testFile.replace(".test.ts", ".ts")
      return {
        success: true,
        output: `Implementation File: ${implFile}\nCode: // Minimal implementation`,
        changes: [implFile],
      }
    },

    async refactor(files: string[]): Promise<MockAgentResponse> {
      return {
        success: true,
        output: "Refactor suggestions applied",
        changes: files,
      }
    },
  }
}

/**
 * Create mock code reviewer agent
 */
export function createMockCodeReviewer() {
  return {
    async review(files: string[]): Promise<MockAgentResponse> {
      return {
        success: true,
        output: `### Suggestions\n- Improve code quality\n- Add documentation\n- Fix potential issues`,
        changes: files,
      }
    },
  }
}

/**
 * Create mock security reviewer agent
 */
export function createMockSecurityReviewer() {
  return {
    async review(code: string): Promise<MockAgentResponse> {
      return {
        success: true,
        output: "Security review passed",
        changes: [],
      }
    },
  }
}

/**
 * Mock agents collection
 */
export const mockAgents = {
  tddGuide: createMockTDDGuideAgent(),
  codeReviewer: createMockCodeReviewer(),
  securityReviewer: createMockSecurityReviewer(),
}

/**
 * Test scenarios for autonomous mode
 */
export const testScenarios = {
  simpleFeature: {
    request: "实现一个简单的计算器",
    expectedPhases: ["red", "green", "refactor"],
    expectedTests: 4,
  },

  multiStepFeature: {
    request: "实现用户认证，包含注册、登录和登出功能",
    expectedPhases: ["red", "green", "refactor"],
    expectedIterations: 3,
  },

  apiFeature: {
    request: "为用户创建 REST API CRUD 操作",
    expectedPhases: ["red", "green", "refactor"],
    expectedEndpoints: 5,
  },

  bugFix: {
    request: "修复用户登录时的验证错误",
    expectedPhases: ["red", "green"],
  },
}

/**
 * Default autonomous configuration for testing
 */
export function createTestConfig(overrides?: Partial<OrchestratorConfig>): OrchestratorConfig {
  return {
    autonomyLevel: "wild",
    resourceBudget: {
      maxTokens: 100000,
      maxCostUSD: 10.0,
      maxDurationMinutes: 30,
      maxFilesChanged: 20,
      maxActions: 100,
    },
    unattended: false,
    ...overrides,
  }
}

/**
 * Create a mock autonomous session with proper Instance context
 *
 * IMPORTANT: This function creates a temporary directory and provides Instance context.
 * The returned session must be properly disposed to clean up.
 */
export async function createMockAutonomousSession(config?: Partial<OrchestratorConfig>) {
  const sessionId = `test_session_${Date.now()}_${Math.random().toString(36).slice(2)}`
  const requestId = `test_req_${Date.now()}`
  const request = "Test request"
  const startTime = Date.now()

  let tmpDir: Awaited<ReturnType<typeof tmpdir>> | null = null

  try {
    tmpDir = await tmpdir({ git: true })

    const orchestrator = await Instance.provide({
      directory: tmpDir.path,
      fn: async () => {
        return createOrchestrator(
          { sessionId, requestId, request, startTime },
          createTestConfig(config),
        )
      },
    })

    return {
      sessionId,
      requestId,
      orchestrator,
      async start() {
        await Instance.provide({
          directory: tmpDir!.path,
          fn: async () => orchestrator.start(request),
        })
      },
      async dispose() {
        if (tmpDir) {
          await tmpDir.cleanup()
        }
      },
    }
  } catch (error) {
    if (tmpDir) {
      await tmpDir.cleanup()
    }
    throw error
  }
}

/**
 * Wait for state transition
 */
export async function waitForState(
  orchestrator: Orchestrator,
  targetState: AutonomousState,
  timeout = 5000,
): Promise<boolean> {
  const startTime = Date.now()

  while (Date.now() - startTime < timeout) {
    if (orchestrator.getState() === targetState) {
      return true
    }
    await new Promise((resolve) => setTimeout(resolve, 50))
  }

  return false
}

/**
 * Wait for predicate
 */
export async function waitFor(
  predicate: () => boolean,
  timeout = 5000,
): Promise<boolean> {
  const startTime = Date.now()

  while (Date.now() - startTime < timeout) {
    if (predicate()) {
      return true
    }
    await new Promise((resolve) => setTimeout(resolve, 50))
  }

  return false
}

/**
 * Create mock TDD cycle result
 */
export function createMockTDDCycleResult(
  phase: "red" | "green" | "refactor",
  overrides?: Partial<TDDCycleResult>,
): TDDCycleResult {
  return {
    phase,
    success: true,
    duration: 1000,
    changes: [`mock-${phase}.ts`],
    ...overrides,
  }
}

/**
 * Create mock resource budget
 */
export function createMockResourceBudget(overrides?: Partial<ResourceBudget>): ResourceBudget {
  return {
    maxTokens: 100000,
    maxCostUSD: 10.0,
    maxDurationMinutes: 30,
    maxFilesChanged: 20,
    maxActions: 100,
    ...overrides,
  }
}

/**
 * Parse and validate resource budget
 */
export function parseTestBudget(budget: string): ResourceBudget {
  return parseResourceBudget(budget)
}

/**
 * Create test session context
 *
 * NOTE: This only creates a context object. For tests that require storage access,
 * use withTestInstance() wrapper or createMockAutonomousSession().
 */
export interface TestSessionContext {
  sessionId: string
  requestId: string
  request: string
  startTime: number
}

export function createTestSessionContext(request = "Test request"): TestSessionContext {
  return {
    sessionId: `test_session_${Date.now()}_${Math.random().toString(36).slice(2)}`,
    requestId: `test_req_${Date.now()}_${Math.random().toString(36).slice(2)}`,
    request,
    startTime: Date.now(),
  }
}

/**
 * Run a test function within a temporary Instance context
 *
 * Use this wrapper for tests that access storage or require Instance.project.id
 */
export async function withTestInstance<T>(
  fn: (tmpDir: string) => Promise<T>,
): Promise<T> {
  const tmp = await tmpdir({ git: true })
  try {
    return await Instance.provide({
      directory: tmp.path,
      fn: () => fn(tmp.path),
    })
  } finally {
    await tmp.cleanup()
  }
}

/**
 * Wraps a test function for use in describe() blocks that need Instance context
 *
 * Usage:
 * ```ts
 * describe("My Test", () => {
 *   const testWithInstance = withTestInstanceWrapper()
 *   test("should do something", testWithInstance(async () => {
 *     // orchestrator code here...
 *   }))
 * })
 * ```
 */
export function withTestInstanceWrapper() {
  return <T>(fn: () => Promise<T>) => {
    return () =>
      withTestInstance(async () => {
        await fn()
      })
  }
}

/**
 * State transition tracker
 */
export class StateTransitionTracker {
  private transitions: Array<{ from: AutonomousState; to: AutonomousState; timestamp: number }> =
    []

  record(from: AutonomousState, to: AutonomousState) {
    this.transitions.push({ from, to, timestamp: Date.now() })
  }

  getTransitions(): Array<{ from: AutonomousState; to: AutonomousState; timestamp: number }> {
    return [...this.transitions]
  }

  getLastTransition(): { from: AutonomousState; to: AutonomousState; timestamp: number } | undefined {
    return this.transitions[this.transitions.length - 1]
  }

  hasTransition(from: AutonomousState, to: AutonomousState): boolean {
    return this.transitions.some((t) => t.from === from && t.to === to)
  }

  countTransitions(to: AutonomousState): number {
    return this.transitions.filter((t) => t.to === to).length
  }

  clear() {
    this.transitions = []
  }
}

/**
 * Create a state machine with transition tracking
 */
export function createTrackedStateMachine(tracker: StateTransitionTracker): StateMachine {
  return new StateMachine({
    onStateChange: async (from, to) => {
      tracker.record(from, to)
    },
  })
}

/**
 * Verification helpers
 */
export const verify = {
  stateTransition: (from: AutonomousState, to: AutonomousState, tracker: StateTransitionTracker) => {
    expect(tracker.hasTransition(from, to)).toBe(true)
  },

  stateSequence: (
    sequence: AutonomousState[],
    tracker: StateTransitionTracker,
    allowGaps = true,
  ) => {
    if (allowGaps) {
      let lastIndex = -1
      for (const state of sequence) {
        const index = tracker.getTransitions().findIndex((t) => t.to === state)
        expect(index).toBeGreaterThan(lastIndex)
        lastIndex = index
      }
    } else {
      const actualSequence = tracker.getTransitions().map((t) => t.to)
      expect(actualSequence).toEqual(sequence)
    }
  },

  resourceBudget: (usage: ResourceUsage, budget: ResourceBudget) => {
    expect(usage.tokensUsed).toBeLessThanOrEqual(budget.maxTokens)
    expect(usage.costUSD).toBeLessThanOrEqual(budget.maxCostUSD)
  },

  eventPublished: (eventName: string, sessionEvents: string[]) => {
    expect(sessionEvents).toContain(eventName)
  },
}

/**
 * Test assertion helpers
 */
export const assert = {
  sessionCreated: (sessionId: string) => {
    expect(sessionId).toBeDefined()
    expect(sessionId.length).toBeGreaterThan(0)
    expect(sessionId).toStartWith("test_session_")
  },

  requestId: (requestId: string) => {
    expect(requestId).toBeDefined()
    expect(requestId.length).toBeGreaterThan(0)
  },

  initialState: (state: AutonomousState, expected = AutonomousState.IDLE) => {
    expect(state).toBe(expected)
  },

  validState: (state: AutonomousState) => {
    expect(Object.values(AutonomousState)).toContain(state)
  },
}
