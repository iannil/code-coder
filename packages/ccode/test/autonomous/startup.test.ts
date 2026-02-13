import { describe, test, expect, beforeEach } from "bun:test"
import { AutonomousState } from "@/autonomous/state/states"
import { StateMachine } from "@/autonomous/state/state-machine"
import { DecisionEngine } from "@/autonomous/decision/engine"
import { Orchestrator, createOrchestrator, type OrchestratorConfig } from "@/autonomous/orchestration/orchestrator"
import { SafetyGuard } from "@/autonomous/safety/constraints"
import {
  SafetyIntegration,
  createSafetyIntegration,
} from "@/autonomous/safety/integration"
import { Bus } from "@/bus"
import { AutonomousEvent } from "@/autonomous/events"
import {
  createTestConfig,
  createTestSessionContext,
  assert,
  verify,
  type StateTransitionTracker,
} from "./fixtures/autonomous-fixture"

describe("Autonomous Mode - Startup Verification", () => {
  describe("Session Initialization", () => {
    test("should create unique sessionId", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()

      expect(context1.sessionId).toBeDefined()
      expect(context2.sessionId).toBeDefined()
      expect(context1.sessionId).not.toBe(context2.sessionId)
    })

    test("should generate unique requestId", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()

      expect(context1.requestId).toBeDefined()
      expect(context2.requestId).toBeDefined()
      expect(context1.requestId).not.toBe(context2.requestId)
    })

    test("should initialize in IDLE state", async () => {
      const stateMachine = new StateMachine({ onStateChange: async () => {} })

      assert.initialState(stateMachine.getState(), AutonomousState.IDLE)
    })

    test("should set startTime on context", async () => {
      const beforeTime = Date.now()
      const context = createTestSessionContext()

      expect(context.startTime).toBeGreaterThanOrEqual(beforeTime)
      expect(context.startTime).toBeLessThanOrEqual(Date.now())
    })
  })

  describe("Configuration Loading", () => {
    test("should load autonomyLevel from config", async () => {
      const config = createTestConfig({ autonomyLevel: "lunatic" })
      expect(config.autonomyLevel).toBe("lunatic")

      const timidConfig = createTestConfig({ autonomyLevel: "timid" })
      expect(timidConfig.autonomyLevel).toBe("timid")
    })

    test("should apply resourceBudget correctly", async () => {
      const customBudget = {
        maxTokens: 50000,
        maxCostUSD: 5.0,
        maxDurationMinutes: 15,
        maxFilesChanged: 10,
        maxActions: 50,
      }

      const config = createTestConfig({ resourceBudget: customBudget })

      expect(config.resourceBudget.maxTokens).toBe(50000)
      expect(config.resourceBudget.maxCostUSD).toBe(5.0)
      expect(config.resourceBudget.maxDurationMinutes).toBe(15)
      expect(config.resourceBudget.maxFilesChanged).toBe(10)
      expect(config.resourceBudget.maxActions).toBe(50)
    })

    test("should set unattended mode from config", async () => {
      const attendedConfig = createTestConfig({ unattended: false })
      expect(attendedConfig.unattended).toBe(false)

      const unattendedConfig = createTestConfig({ unattended: true })
      expect(unattendedConfig.unattended).toBe(true)
    })

    test("should use default config when overrides not provided", async () => {
      const config = createTestConfig()

      expect(config.autonomyLevel).toBe("wild")
      expect(config.resourceBudget.maxTokens).toBe(100000)
      expect(config.resourceBudget.maxCostUSD).toBe(10.0)
      expect(config.unattended).toBe(false)
    })
  })

  describe("Component Initialization", () => {
    let sessionId: string
    let config: OrchestratorConfig

    beforeEach(() => {
      const context = createTestSessionContext()
      sessionId = context.sessionId
      config = createTestConfig()
    })

    test("should create StateMachine", async () => {
      const stateMachine = new StateMachine({ onStateChange: async () => {} })

      expect(stateMachine).toBeDefined()
      expect(stateMachine.getState()).toBe(AutonomousState.IDLE)
    })

    test("should create DecisionEngine", async () => {
      const decisionEngine = new DecisionEngine({ autonomyLevel: "wild" })

      expect(decisionEngine).toBeDefined()
      expect(decisionEngine.getHistory()).toEqual([])
    })

    test("should initialize DecisionEngine with config autonomy level", async () => {
      const crazyEngine = new DecisionEngine({ autonomyLevel: "crazy" })
      const timidEngine = new DecisionEngine({ autonomyLevel: "timid" })

      expect(crazyEngine).toBeDefined()
      expect(timidEngine).toBeDefined()
    })

    test("should create SafetyGuard with budget", async () => {
      const guard = new SafetyGuard(sessionId, config.resourceBudget)

      expect(guard).toBeDefined()

      const remaining = guard.getRemaining()
      expect(remaining.maxTokens).toBe(config.resourceBudget.maxTokens)
      expect(remaining.maxCostUSD).toBe(config.resourceBudget.maxCostUSD)
    })

    test("should create SafetyIntegration", async () => {
      const safetyIntegration = createSafetyIntegration(sessionId, {
        enableDoomLoopBridge: true,
        enableDestructiveProtection: true,
        autoRollbackOnFailure: false,
      })

      expect(safetyIntegration).toBeDefined()

      const status = safetyIntegration.getStatus()
      expect(status.safe).toBe(true)
    })
  })

  describe("Orchestrator Creation", () => {
    test("should create orchestrator with context", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator = createOrchestrator(context, config)

      expect(orchestrator).toBeDefined()
      expect(orchestrator.getState()).toBe(AutonomousState.IDLE)
    })

    test("should initialize orchestrator components", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator = createOrchestrator(context, config)

      // Get stats to verify components are initialized
      const stats = orchestrator.getTaskStats()
      expect(stats).toBeDefined()

      const history = orchestrator.getDecisionHistory()
      expect(history).toEqual([])
    })

    test("should serialize orchestrator state", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator = createOrchestrator(context, config)

      const serialized = orchestrator.serialize()
      expect(serialized.state).toBe(AutonomousState.IDLE)
      expect(serialized.tasks).toBeDefined()
      expect(serialized.decisions).toBeDefined()
    })
  })

  describe("Event Publishing", () => {
    test("should publish SessionStarted event on start", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      const publishedEvents: string[] = []

      // Subscribe to SessionStarted event
      const subscription = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        if (event.properties.sessionId === context.sessionId) {
          publishedEvents.push("SessionStarted")
        }
      })

      await orchestrator.start("Test request")

      // Give event time to propagate
      await new Promise((resolve) => setTimeout(resolve, 100))

      subscription()

      expect(publishedEvents).toContain("SessionStarted")
    })

    test("should include correct sessionId in event", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      let capturedSessionId: string | undefined

      const subscription = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        if (event.properties.sessionId === context.sessionId) {
          capturedSessionId = event.properties.sessionId
        }
      })

      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      subscription()

      expect(capturedSessionId).toBe(context.sessionId)
    })

    test("should include autonomyLevel in event", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig({ autonomyLevel: "insane" })
      const orchestrator = createOrchestrator(context, config)

      let capturedLevel: string | undefined

      const subscription = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        if (event.properties.sessionId === context.sessionId) {
          capturedLevel = event.properties.autonomyLevel
        }
      })

      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      subscription()

      expect(capturedLevel).toBe("insane")
    })
  })

  describe("State Transitions", () => {
    test("should transition from IDLE to PLANNING on start", async () => {
      const context = createTestSessionContext()
      const config = createTestConfig()
      const orchestrator = createOrchestrator(context, config)

      expect(orchestrator.getState()).toBe(AutonomousState.IDLE)

      await orchestrator.start("Test request")

      await new Promise((resolve) => setTimeout(resolve, 100))

      expect(orchestrator.getState()).toBe(AutonomousState.PLANNING)
    })

    test("should reject invalid state transitions", async () => {
      const stateMachine = new StateMachine({ onStateChange: async () => {} })

      // Try to transition from IDLE directly to COMPLETED (not valid)
      const success = await stateMachine.transition(AutonomousState.COMPLETED, {
        reason: "Invalid transition",
      })

      expect(success).toBe(false)
      expect(stateMachine.getState()).toBe(AutonomousState.IDLE)
    })

    test("should accept valid state transitions", async () => {
      const stateMachine = new StateMachine({ onStateChange: async () => {} })

      const success = await stateMachine.transition(AutonomousState.PLANNING, {
        reason: "Starting to plan",
      })

      expect(success).toBe(true)
      expect(stateMachine.getState()).toBe(AutonomousState.PLANNING)
    })
  })

  describe("Safety System", () => {
    test("should initialize safety integration correctly", async () => {
      const context = createTestSessionContext()
      const safetyIntegration = createSafetyIntegration(context.sessionId, {
        enableDoomLoopBridge: true,
        enableDestructiveProtection: true,
        autoRollbackOnFailure: false,
      })

      await safetyIntegration.initialize()

      const status = safetyIntegration.getStatus()
      expect(status.safe).toBe(true)
      expect(status.resources).toBeDefined()
      expect(status.loops).toBeDefined()
    })

    test("should track resource usage from start", async () => {
      const context = createTestSessionContext()
      const budget = {
        maxTokens: 100000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 20,
        maxActions: 100,
      }
      const guard = new SafetyGuard(context.sessionId, budget)

      const usage = guard.getCurrentUsage()

      expect(usage.tokensUsed).toBe(0)
      expect(usage.costUSD).toBe(0)
      expect(usage.actionsPerformed).toBe(0)
      expect(usage.filesChanged).toBe(0)
    })
  })

  describe("Configuration Edge Cases", () => {
    test("should handle minimum budget values", async () => {
      const minBudget = {
        maxTokens: 1,
        maxCostUSD: 0.01,
        maxDurationMinutes: 1,
        maxFilesChanged: 1,
        maxActions: 1,
      }

      const config = createTestConfig({ resourceBudget: minBudget })
      expect(config.resourceBudget.maxTokens).toBe(1)
      expect(config.resourceBudget.maxCostUSD).toBe(0.01)
    })

    test("should handle maximum budget values", async () => {
      const maxBudget = {
        maxTokens: Number.MAX_SAFE_INTEGER,
        maxCostUSD: Number.MAX_SAFE_INTEGER,
        maxDurationMinutes: Number.MAX_SAFE_INTEGER,
        maxFilesChanged: Number.MAX_SAFE_INTEGER,
        maxActions: Number.MAX_SAFE_INTEGER,
      }

      const config = createTestConfig({ resourceBudget: maxBudget })
      expect(config.resourceBudget.maxTokens).toBe(Number.MAX_SAFE_INTEGER)
    })

    test("should handle all autonomy levels", async () => {
      const levels = ["lunatic", "insane", "crazy", "wild", "bold", "timid"] as const

      for (const level of levels) {
        const config = createTestConfig({ autonomyLevel: level })
        expect(config.autonomyLevel).toBe(level)
      }
    })
  })

  describe("Multiple Sessions", () => {
    test("should create multiple independent sessions", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()
      const context3 = createTestSessionContext()

      const orchestrator1 = createOrchestrator(context1, createTestConfig())
      const orchestrator2 = createOrchestrator(context2, createTestConfig())
      const orchestrator3 = createOrchestrator(context3, createTestConfig())

      expect(orchestrator1).toBeDefined()
      expect(orchestrator2).toBeDefined()
      expect(orchestrator3).toBeDefined()

      // Verify session IDs are unique
      expect(context1.sessionId).not.toBe(context2.sessionId)
      expect(context2.sessionId).not.toBe(context3.sessionId)
      expect(context1.sessionId).not.toBe(context3.sessionId)
    })

    test("should maintain independent state across sessions", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()

      const orchestrator1 = createOrchestrator(context1, createTestConfig())
      const orchestrator2 = createOrchestrator(context2, createTestConfig())

      await orchestrator1.start("Request 1")
      await new Promise((resolve) => setTimeout(resolve, 50))

      expect(orchestrator1.getState()).toBe(AutonomousState.PLANNING)
      expect(orchestrator2.getState()).toBe(AutonomousState.IDLE)

      await orchestrator2.start("Request 2")
      await new Promise((resolve) => setTimeout(resolve, 50))

      expect(orchestrator1.getState()).toBe(AutonomousState.PLANNING)
      expect(orchestrator2.getState()).toBe(AutonomousState.PLANNING)
    })
  })
})
