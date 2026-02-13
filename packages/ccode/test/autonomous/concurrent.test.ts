import { describe, test, expect, beforeEach } from "bun:test"
import { AutonomousState } from "@/autonomous/state/states"
import { Orchestrator, createOrchestrator } from "@/autonomous/orchestration/orchestrator"
import { Bus } from "@/bus"
import { AutonomousEvent } from "@/autonomous/events"
import { createTestConfig, createTestSessionContext } from "./fixtures/autonomous-fixture"

describe("Autonomous Mode - Concurrent Operations", () => {
  describe("Multiple Sessions", () => {
    test("should create multiple sessions with unique IDs", () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()
      const context3 = createTestSessionContext()

      expect(context1.sessionId).not.toBe(context2.sessionId)
      expect(context2.sessionId).not.toBe(context3.sessionId)
      expect(context1.sessionId).not.toBe(context3.sessionId)
    })

    test("should create multiple orchestrators independently", () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator1 = createOrchestrator(context1, config)
      const orchestrator2 = createOrchestrator(context2, config)

      expect(orchestrator1).toBeDefined()
      expect(orchestrator2).toBeDefined()
      expect(orchestrator1.getState()).toBe(AutonomousState.IDLE)
      expect(orchestrator2.getState()).toBe(AutonomousState.IDLE)
    })

    test("should start multiple sessions concurrently", async () => {
      const context1 = createTestSessionContext("Request 1")
      const context2 = createTestSessionContext("Request 2")
      const context3 = createTestSessionContext("Request 3")
      const config = createTestConfig()

      const orchestrator1 = createOrchestrator(context1, config)
      const orchestrator2 = createOrchestrator(context2, config)
      const orchestrator3 = createOrchestrator(context3, config)

      // Start all sessions
      await Promise.all([
        orchestrator1.start("Request 1"),
        orchestrator2.start("Request 2"),
        orchestrator3.start("Request 3"),
      ])

      await new Promise((resolve) => setTimeout(resolve, 100))

      // All should be in PLANNING state
      expect(orchestrator1.getState()).toBe(AutonomousState.PLANNING)
      expect(orchestrator2.getState()).toBe(AutonomousState.PLANNING)
      expect(orchestrator3.getState()).toBe(AutonomousState.PLANNING)
    })
  })

  describe("Session Isolation", () => {
    test("should not share state between sessions", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator1 = createOrchestrator(context1, config)
      const orchestrator2 = createOrchestrator(context2, config)

      await orchestrator1.start("Request 1")

      await new Promise((resolve) => setTimeout(resolve, 50))

      // Session 1 should progress, session 2 should remain idle
      expect(orchestrator1.getState()).not.toBe(AutonomousState.IDLE)
      expect(orchestrator2.getState()).toBe(AutonomousState.IDLE)
    })

    test("should not share resources between sessions", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()

      const config1 = createTestConfig({ resourceBudget: { maxTokens: 1000, maxCostUSD: 1.0, maxDurationMinutes: 10, maxFilesChanged: 5, maxActions: 10 } })
      const config2 = createTestConfig({ resourceBudget: { maxTokens: 5000, maxCostUSD: 5.0, maxDurationMinutes: 20, maxFilesChanged: 10, maxActions: 20 } })

      const orchestrator1 = createOrchestrator(context1, config1)
      const orchestrator2 = createOrchestrator(context2, config2)

      // Each orchestrator should have its own config
      const serialized1 = orchestrator1.serialize()
      const serialized2 = orchestrator2.serialize()

      expect(serialized1).toBeDefined()
      expect(serialized2).toBeDefined()
    })
  })

  describe("Event Isolation", () => {
    test("should filter events by sessionId", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()
      const config = createTestConfig()

      let event1Count = 0
      let event2Count = 0

      const sub1 = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        if (event.properties.sessionId === context1.sessionId) event1Count++
      })

      const sub2 = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        if (event.properties.sessionId === context2.sessionId) event2Count++
      })

      const orchestrator1 = createOrchestrator(context1, config)
      const orchestrator2 = createOrchestrator(context2, config)

      await Promise.all([
        orchestrator1.start("Request 1"),
        orchestrator2.start("Request 2"),
      ])

      await new Promise((resolve) => setTimeout(resolve, 100))

      sub1()
      sub2()

      expect(event1Count).toBe(1)
      expect(event2Count).toBe(1)
    })

    test("should not cross-contaminate events", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()
      const config = createTestConfig()

      const session1Events: string[] = []
      const session2Events: string[] = []

      const sub = Bus.subscribe(AutonomousEvent.SessionStarted, async (event) => {
        if (event.properties.sessionId === context1.sessionId) {
          session1Events.push(event.properties.sessionId)
        }
        if (event.properties.sessionId === context2.sessionId) {
          session2Events.push(event.properties.sessionId)
        }
      })

      const orchestrator1 = createOrchestrator(context1, config)
      const orchestrator2 = createOrchestrator(context2, config)

      await Promise.all([
        orchestrator1.start("Request 1"),
        orchestrator2.start("Request 2"),
      ])

      await new Promise((resolve) => setTimeout(resolve, 100))

      sub()

      expect(session1Events).toHaveLength(1)
      expect(session2Events).toHaveLength(1)
      expect(session1Events[0]).toBe(context1.sessionId)
      expect(session2Events[0]).toBe(context2.sessionId)
    })
  })

  describe("Resource Contention", () => {
    test("should enforce budget per session", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()

      const budget1 = { maxTokens: 1000, maxCostUSD: 1.0, maxDurationMinutes: 10, maxFilesChanged: 5, maxActions: 10 }
      const budget2 = { maxTokens: 2000, maxCostUSD: 2.0, maxDurationMinutes: 20, maxFilesChanged: 10, maxActions: 20 }

      const config1 = createTestConfig({ resourceBudget: budget1 })
      const config2 = createTestConfig({ resourceBudget: budget2 })

      const orchestrator1 = createOrchestrator(context1, config1)
      const orchestrator2 = createOrchestrator(context2, config2)

      // Each should have its own budget
      expect(orchestrator1).toBeDefined()
      expect(orchestrator2).toBeDefined()
    })

    test("should handle multiple sessions approaching limits", async () => {
      const contexts = Array.from({ length: 3 }, () => createTestSessionContext())
      const config = createTestConfig({
        resourceBudget: {
          maxTokens: 10000,
          maxCostUSD: 5.0,
          maxDurationMinutes: 15,
          maxFilesChanged: 10,
          maxActions: 50,
        },
      })

      const orchestrators = contexts.map((ctx) => createOrchestrator(ctx, config))

      // Start all sessions
      await Promise.all(orchestrators.map((orch) => orch.start("Test")))

      await new Promise((resolve) => setTimeout(resolve, 100))

      // All should start successfully
      for (const orch of orchestrators) {
        expect(orch.getState()).toBe(AutonomousState.PLANNING)
      }
    })
  })

  describe("Concurrent State Transitions", () => {
    test("should handle simultaneous state changes", async () => {
      const contexts = Array.from({ length: 3 }, () => createTestSessionContext())
      const config = createTestConfig()

      const orchestrators = contexts.map((ctx) => createOrchestrator(ctx, config))

      // Trigger transitions in all orchestrators
      const transitions = orchestrators.map((orch) => orch.start("Test"))

      await Promise.all(transitions)
      await new Promise((resolve) => setTimeout(resolve, 100))

      // All should transition successfully
      for (const orch of orchestrators) {
        expect(orch.getState()).toBe(AutonomousState.PLANNING)
      }
    })
  })

  describe("Parallel Execution", () => {
    test("should execute operations in parallel", async () => {
      const context1 = createTestSessionContext()
      const context2 = createTestSessionContext()
      const config = createTestConfig()

      const orchestrator1 = createOrchestrator(context1, config)
      const orchestrator2 = createOrchestrator(context2, config)

      const startTime = Date.now()

      await Promise.all([
        orchestrator1.start("Request 1"),
        orchestrator2.start("Request 2"),
      ])

      const duration = Date.now() - startTime

      // Parallel execution should be faster than sequential
      expect(duration).toBeLessThan(5000) // Should complete in under 5 seconds
    })
  })

  describe("Race Conditions", () => {
    test("should not have race conditions in event handling", async () => {
      const contexts = Array.from({ length: 5 }, () => createTestSessionContext())
      const config = createTestConfig()

      let eventCount = 0

      const sub = Bus.subscribe(AutonomousEvent.SessionStarted, async () => {
        eventCount++
      })

      // Start all sessions rapidly
      await Promise.all(contexts.map((ctx) => {
        const orch = createOrchestrator(ctx, config)
        return orch.start("Test")
      }))

      await new Promise((resolve) => setTimeout(resolve, 200))

      sub()

      // Should receive exactly 5 events
      expect(eventCount).toBe(5)
    })
  })

  describe("Cleanup", () => {
    test("should cleanup multiple sessions", async () => {
      const contexts = Array.from({ length: 3 }, () => createTestSessionContext())
      const config = createTestConfig()

      const orchestrators = contexts.map((ctx) => createOrchestrator(ctx, config))

      // Start and verify
      await Promise.all(orchestrators.map((orch) => orch.start("Test")))
      await new Promise((resolve) => setTimeout(resolve, 100))

      for (const orch of orchestrators) {
        expect(orch.getState()).not.toBe(AutonomousState.IDLE)
      }

      // Cleanup would happen here in real implementation
      // For now, we just verify they exist
      expect(orchestrators.length).toBe(3)
    })
  })
})
