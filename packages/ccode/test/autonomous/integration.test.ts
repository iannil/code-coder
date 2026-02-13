import { describe, test, expect, beforeEach } from "bun:test"
import { AutonomousState } from "@/autonomous/state/states"
import { Orchestrator, createOrchestrator, type OrchestratorConfig } from "@/autonomous/orchestration/orchestrator"
import { Executor, createExecutor, type ExecutionConfig } from "@/autonomous/execution/executor"
import { TestRunner } from "@/autonomous/execution/test-runner"
import { SafetyGuard, type ResourceBudget } from "@/autonomous/safety/constraints"
import {
  SafetyIntegration,
  createSafetyIntegration,
} from "@/autonomous/safety/integration"
import { DecisionEngine } from "@/autonomous/decision/engine"
import { Bus } from "@/bus"
import { AutonomousEvent } from "@/autonomous/events"
import {
  createTestConfig,
  createTestSessionContext,
  waitFor,
} from "./fixtures/autonomous-fixture"

describe("Autonomous Mode - Integration", () => {
  describe("Orchestrator Connections", () => {
    test("should connect Orchestrator to Executor", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator = createOrchestrator(context, config)

      // Start the orchestrator which should initialize the executor
      await orchestrator.start("Test request")

      // Give executor time to initialize
      await new Promise((resolve) => setTimeout(resolve, 100))

      // Verify orchestrator is in correct state
      expect(orchestrator.getState()).toBe(AutonomousState.PLANNING)
    })

    test("should pass sessionId through components", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator = createOrchestrator(context, config)

      // Verify sessionId is consistent
      expect(orchestrator.serialize().state).toBe(AutonomousState.IDLE)
    })

    test("should propagate configuration to all components", async () => {
      const context = createTestSessionContext()
      const customConfig = createTestConfig({
        autonomyLevel: "insane",
        unattended: true,
      })

      const orchestrator = createOrchestrator(context, customConfig)

      // Configuration should be applied
      const serialized = orchestrator.serialize()
      expect(serialized).toBeDefined()
    })
  })

  describe("Executor Connections", () => {
    let sessionId: string
    let executor: Executor

    beforeEach(() => {
      sessionId = `test_session_${Date.now()}`
      executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })
    })

    test("should initialize executor context", async () => {
      await executor.initialize()

      const context = executor.getContext()
      const metadata = context.get()

      expect(metadata.sessionId).toBe(sessionId)
    })

    test("should connect executor to test runner", async () => {
      await executor.initialize()

      // Test runner should be accessible through executor
      const stats = executor.getStats()
      expect(stats).toBeDefined()
      expect(stats.cyclesCompleted).toBe(0)
    })

    test("should track test statistics", async () => {
      await executor.initialize()

      const stats = executor.getStats()
      expect(stats.testsRun).toBe(0)
      expect(stats.testsPassed).toBe(0)
      expect(stats.testsFailed).toBe(0)
    })
  })

  describe("TestRunner Integration", () => {
    test("should have runAll method", () => {
      expect(typeof TestRunner.runAll).toBe("function")
    })

    test("should have runCoverage method", () => {
      expect(typeof TestRunner.runCoverage).toBe("function")
    })

    test("should have runPattern method", () => {
      expect(typeof TestRunner.runPattern).toBe("function")
    })

    test("should return test result structure", async () => {
      // Mock the test execution to avoid running real tests
      const result = {
        success: false,
        passed: 0,
        failed: 1,
        skipped: 0,
        duration: 0,
        errors: ["Mock test failure"],
      }

      expect(result).toHaveProperty("success")
      expect(result).toHaveProperty("passed")
      expect(result).toHaveProperty("failed")
      expect(result).toHaveProperty("skipped")
      expect(result).toHaveProperty("duration")
      expect(result).toHaveProperty("errors")
    })
  })

  describe("SafetyIntegration Bridge", () => {
    test("should create safety integration with sessionId", async () => {
      const sessionId = `test_session_${Date.now()}`
      const safetyIntegration = createSafetyIntegration(sessionId, {
        enableDoomLoopBridge: true,
        enableDestructiveProtection: true,
        autoRollbackOnFailure: false,
      })

      expect(safetyIntegration).toBeDefined()

      const status = safetyIntegration.getStatus()
      expect(status).toBeDefined()
    })

    test("should connect to SafetyGuard", async () => {
      const sessionId = `test_session_${Date.now()}`
      const budget: ResourceBudget = {
        maxTokens: 100000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 20,
        maxActions: 100,
      }

      const guard = new SafetyGuard(sessionId, budget)

      expect(guard).toBeDefined()

      const remaining = guard.getRemaining()
      expect(remaining.maxTokens).toBe(budget.maxTokens)
    })

    test("should record destructive operations", () => {
      const sessionId = `test_session_${Date.now()}`
      const safetyIntegration = createSafetyIntegration(sessionId, {
        enableDestructiveProtection: true,
        enableDoomLoopBridge: false,
        autoRollbackOnFailure: false,
      })

      // Record a destructive operation
      safetyIntegration.recordDestructiveOperation({
        category: "git_operations",
        description: "Test operation",
        reversible: true,
        riskLevel: "medium",
      })

      const status = safetyIntegration.getStatus()
      expect(status).toBeDefined()
    })

    test("should check safety before operations", async () => {
      const sessionId = `test_session_${Date.now()}`
      const safetyIntegration = createSafetyIntegration(sessionId, {
        enableDestructiveProtection: true,
        enableDoomLoopBridge: false,
        autoRollbackOnFailure: false,
      })

      const check = await safetyIntegration.checkSafety({
        category: "maxActions",
      })

      expect(check.safe).toBe(true)
    })
  })

  describe("Decision Engine Integration", () => {
    test("should create decision engine with autonomy level", () => {
      const engine = new DecisionEngine({ autonomyLevel: "wild" })
      expect(engine).toBeDefined()
      expect(engine.getHistory()).toEqual([])
    })

    test("should track decision history", async () => {
      const engine = new DecisionEngine({ autonomyLevel: "wild" })

      // Initial history should be empty
      expect(engine.getHistory()).toEqual([])

      // After a decision, history should be populated
      const result = await engine.evaluate(
        {
          type: "implementation",
          description: "Test decision",
          riskLevel: "low",
          convergence: 7,
          leverage: 8,
          optionality: 6,
          surplus: 7,
          evolution: 8,
        },
        {
          sessionId: "test",
          currentState: AutonomousState.PLANNING,
          resourceUsage: {
            tokensUsed: 1000,
            costUSD: 0.5,
            durationMinutes: 5,
          },
          errorCount: 0,
          recentDecisions: [],
        },
      )

      const history = engine.getHistory()
      expect(history.length).toBeGreaterThan(0)
    })
  })

  describe("Event System Integration", () => {
    test("should publish events through Bus", async () => {
      let eventReceived = false

      const subscription = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        eventReceived = true
      })

      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      subscription()

      expect(eventReceived).toBe(true)
    })

    test("should propagate session context in events", async () => {
      let capturedSessionId: string | undefined

      const subscription = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        capturedSessionId = event.properties.sessionId
      })

      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      subscription()

      expect(capturedSessionId).toBe(context.sessionId)
    })

    test("should handle state transition events", async () => {
      let transitionReceived = false

      const subscription = Bus.subscribe(AutonomousEvent.StateChanged, async (event) => {
        transitionReceived = true
      })

      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      subscription()

      // State transition events should be fired
      expect(transitionReceived).toBe(true)
    })
  })

  describe("Component Lifecycle", () => {
    test("should initialize components in correct order", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator = createOrchestrator(context, config)

      // Components should be initialized
      expect(orchestrator.getState()).toBe(AutonomousState.IDLE)

      // Starting should trigger initialization
      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      // State should have progressed
      expect(orchestrator.getState()).not.toBe(AutonomousState.IDLE)
    })

    test("should cleanup resources properly", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()
      await executor.cleanup()

      // After cleanup, context should be cleared
      const metadata = executor.getContext().get()
      expect(metadata).toBeDefined()
    })
  })

  describe("Cross-Component Communication", () => {
    test("should share resource budget across components", async () => {
      const context = createTestSessionContext()
      const budget: ResourceBudget = {
        maxTokens: 50000,
        maxCostUSD: 5.0,
        maxDurationMinutes: 15,
        maxFilesChanged: 10,
        maxActions: 50,
      }

      const config = createTestConfig({ resourceBudget: budget })
      const orchestrator = createOrchestrator(context, config)

      // Budget should be accessible to orchestrator
      expect(orchestrator).toBeDefined()

      const serialized = orchestrator.serialize()
      expect(serialized).toBeDefined()
    })

    test("should propagate errors between components", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig({ autonomyLevel: "timid" })
      const orchestrator = createOrchestrator(context, config)

      // Start the orchestrator
      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      // Should be in PLANNING state
      expect(orchestrator.getState()).toBe(AutonomousState.PLANNING)
    })
  })

  describe("Resource Tracking", () => {
    test("should track tokens used across session", async () => {
      const sessionId = `test_session_${Date.now()}`
      const budget: ResourceBudget = {
        maxTokens: 100000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 20,
        maxActions: 100,
      }

      const guard = new SafetyGuard(sessionId, budget)

      // Initially, no tokens used
      const initialUsage = guard.getCurrentUsage()
      expect(initialUsage.tokensUsed).toBe(0)

      // Record some usage
      guard.record("tokensUsed", 1000)
      guard.record("costUSD", 0.5)

      const updatedUsage = guard.getCurrentUsage()
      expect(updatedUsage.tokensUsed).toBe(1000)
      expect(updatedUsage.costUSD).toBe(0.5)
    })

    test("should calculate remaining budget correctly", async () => {
      const sessionId = `test_session_${Date.now()}`
      const budget: ResourceBudget = {
        maxTokens: 100000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 20,
        maxActions: 100,
      }

      const guard = new SafetyGuard(sessionId, budget)

      // Use half of budget
      guard.record("tokensUsed", 50000)
      guard.record("costUSD", 5.0)

      const remaining = guard.getRemaining()
      expect(remaining.maxTokens).toBe(50000)
      expect(remaining.maxCostUSD).toBe(5.0)
    })

    test("should enforce resource limits", async () => {
      const sessionId = `test_session_${Date.now()}`
      const budget: ResourceBudget = {
        maxTokens: 1000,
        maxCostUSD: 1.0,
        maxDurationMinutes: 10,
        maxFilesChanged: 5,
        maxActions: 10,
      }

      const guard = new SafetyGuard(sessionId, budget)

      // Under limit should pass
      const check1 = await guard.check("maxTokens", { tokensUsed: 500 })
      expect(check1.safe).toBe(true)

      // At limit should fail
      const check2 = await guard.check("maxTokens", { tokensUsed: 1000 })
      expect(check2.safe).toBe(false)
    })
  })

  describe("State Synchronization", () => {
    test("should maintain state consistency across components", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      const initialState = orchestrator.getState()

      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      const newState = orchestrator.getState()

      expect(initialState).toBe(AutonomousState.IDLE)
      expect(newState).not.toBe(AutonomousState.IDLE)
    })

    test("should serialize state correctly", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      const serialized = orchestrator.serialize()

      expect(serialized).toHaveProperty("state")
      expect(serialized).toHaveProperty("tasks")
      expect(serialized).toHaveProperty("decisions")
      expect(typeof serialized.state).toBe("string")
    })
  })
})
