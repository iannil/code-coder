/**
 * Escalation Manager Tests
 *
 * Tests for the Escalation Manager component of the Observer Network.
 *
 * @module test/observer/controller/escalation.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, spyOn } from "bun:test"
import { EscalationManager, createEscalationManager, type EscalationContext, type HumanDecision } from "@/observer/controller/escalation"

describe("EscalationManager", () => {
  let manager: EscalationManager
  let fetchSpy: ReturnType<typeof spyOn>

  beforeEach(() => {
    manager = createEscalationManager({
      timeoutMs: 60000,
      maxPending: 5,
      notificationChannel: "tui", // Use TUI to avoid actual notifications
    })
    manager.start()

    // Mock fetch for webhook/IM calls
    fetchSpy = spyOn(globalThis, "fetch")
    fetchSpy.mockResolvedValue({
      ok: true,
      json: async () => ({ success: true }),
    } as Response)
  })

  afterEach(() => {
    manager.stop()
    manager.clear()
    fetchSpy.mockRestore()
  })

  function createContext(overrides: Partial<EscalationContext> = {}): EscalationContext {
    return {
      currentMode: "AUTO",
      recommendedMode: "MANUAL",
      closeEvaluation: {
        total: 6.5,
        risk: 4.0,
        confidence: 0.75,
        dimensions: {
          convergence: { score: 7, weight: 0.25, contribution: 1.75, description: "" },
          leverage: { score: 6, weight: 0.2, contribution: 1.2, description: "" },
          optionality: { score: 5, weight: 0.2, contribution: 1.0, description: "" },
          surplus: { score: 7, weight: 0.15, contribution: 1.05, description: "" },
          evolution: { score: 8, weight: 0.2, contribution: 1.6, description: "" },
        },
      },
      anomalies: [],
      opportunities: [],
      trigger: "test",
      ...overrides,
    }
  }

  describe("escalate", () => {
    it("should create a new escalation", async () => {
      const escalation = await manager.escalate(
        "medium",
        "Test Escalation",
        "This is a test escalation",
        createContext(),
      )

      expect(escalation.id).toBeDefined()
      expect(escalation.id.startsWith("esc_")).toBe(true)
      expect(escalation.status).toBe("pending")
      expect(escalation.priority).toBe("medium")
      expect(escalation.title).toBe("Test Escalation")
    })

    it("should set correct expiration time", async () => {
      const before = Date.now()
      const escalation = await manager.escalate(
        "low",
        "Test",
        "Description",
        createContext(),
      )
      const after = Date.now()

      // Expiration should be ~60 seconds from now
      const expiresTime = escalation.expiresAt.getTime()
      expect(expiresTime).toBeGreaterThanOrEqual(before + 60000)
      expect(expiresTime).toBeLessThanOrEqual(after + 60000 + 100) // +100ms tolerance
    })

    it("should expire old escalations when maxPending reached", async () => {
      // Create 5 low priority escalations
      for (let i = 0; i < 5; i++) {
        await manager.escalate("low", `Test ${i}`, "Description", createContext())
      }

      // Create one more, should expire oldest
      const newEscalation = await manager.escalate(
        "medium",
        "New Escalation",
        "Description",
        createContext(),
      )

      const pending = manager.getPending()
      expect(pending.length).toBe(5)
      expect(pending.some((e) => e.id === newEscalation.id)).toBe(true)
    })
  })

  describe("resolve", () => {
    it("should resolve escalation with approve action", async () => {
      const escalation = await manager.escalate(
        "medium",
        "Test",
        "Description",
        createContext(),
      )

      const decision: HumanDecision = {
        action: "approve",
        reason: "Approved by user",
        timestamp: new Date(),
      }

      const resolved = await manager.resolve(escalation.id, decision)

      expect(resolved).not.toBeNull()
      expect(resolved!.status).toBe("resolved")
      expect(resolved!.resolution?.action).toBe("approve")
    })

    it("should resolve escalation with reject action", async () => {
      const escalation = await manager.escalate(
        "high",
        "Test",
        "Description",
        createContext(),
      )

      const resolved = await manager.resolve(escalation.id, {
        action: "reject",
        reason: "Rejected",
        timestamp: new Date(),
      })

      expect(resolved!.status).toBe("resolved")
      expect(resolved!.resolution?.action).toBe("reject")
    })

    it("should resolve escalation with modify action and chosen mode", async () => {
      const escalation = await manager.escalate(
        "medium",
        "Test",
        "Description",
        createContext(),
      )

      const resolved = await manager.resolve(escalation.id, {
        action: "modify",
        chosenMode: "HYBRID",
        timestamp: new Date(),
      })

      expect(resolved!.resolution?.chosenMode).toBe("HYBRID")
    })

    it("should return null for non-existent escalation", async () => {
      const resolved = await manager.resolve("non_existent_id", {
        action: "approve",
        timestamp: new Date(),
      })

      expect(resolved).toBeNull()
    })
  })

  describe("acknowledge", () => {
    it("should acknowledge pending escalation", async () => {
      const escalation = await manager.escalate(
        "medium",
        "Test",
        "Description",
        createContext(),
      )

      const acknowledged = manager.acknowledge(escalation.id)

      expect(acknowledged).not.toBeNull()
      expect(acknowledged!.status).toBe("acknowledged")
    })

    it("should return null if already resolved", async () => {
      const escalation = await manager.escalate(
        "medium",
        "Test",
        "Description",
        createContext(),
      )

      await manager.resolve(escalation.id, { action: "approve", timestamp: new Date() })
      const acknowledged = manager.acknowledge(escalation.id)

      expect(acknowledged).toBeNull()
    })
  })

  describe("dismiss", () => {
    it("should dismiss escalation", async () => {
      const escalation = await manager.escalate(
        "low",
        "Test",
        "Description",
        createContext(),
      )

      const dismissed = await manager.dismiss(escalation.id, "No longer needed")

      expect(dismissed!.status).toBe("dismissed")
      expect(dismissed!.resolution?.action).toBe("reject")
    })
  })

  describe("getPending", () => {
    it("should return pending escalations sorted by priority", async () => {
      await manager.escalate("low", "Low Priority", "Description", createContext())
      await manager.escalate("high", "High Priority", "Description", createContext())
      await manager.escalate("medium", "Medium Priority", "Description", createContext())
      await manager.escalate("critical", "Critical Priority", "Description", createContext())

      const pending = manager.getPending()

      expect(pending.length).toBe(4)
      expect(pending[0].priority).toBe("critical")
      expect(pending[1].priority).toBe("high")
      expect(pending[2].priority).toBe("medium")
      expect(pending[3].priority).toBe("low")
    })

    it("should include acknowledged escalations", async () => {
      const escalation = await manager.escalate(
        "medium",
        "Test",
        "Description",
        createContext(),
      )

      manager.acknowledge(escalation.id)
      const pending = manager.getPending()

      expect(pending.length).toBe(1)
      expect(pending[0].status).toBe("acknowledged")
    })
  })

  describe("get", () => {
    it("should get escalation by ID", async () => {
      const escalation = await manager.escalate(
        "medium",
        "Test",
        "Description",
        createContext(),
      )

      const retrieved = manager.get(escalation.id)

      expect(retrieved).not.toBeNull()
      expect(retrieved!.id).toBe(escalation.id)
    })

    it("should return null for non-existent ID", () => {
      const retrieved = manager.get("non_existent")
      expect(retrieved).toBeNull()
    })
  })

  describe("getAll", () => {
    it("should return all escalations sorted by creation time", async () => {
      await manager.escalate("low", "First", "Description", createContext())
      await manager.escalate("high", "Second", "Description", createContext())
      await manager.escalate("medium", "Third", "Description", createContext())

      const all = manager.getAll()

      expect(all.length).toBe(3)
      // Most recent first
      expect(all[0].title).toBe("Third")
      expect(all[2].title).toBe("First")
    })
  })

  describe("clear", () => {
    it("should clear all escalations", async () => {
      await manager.escalate("low", "Test 1", "Description", createContext())
      await manager.escalate("low", "Test 2", "Description", createContext())

      manager.clear()

      expect(manager.getAll().length).toBe(0)
    })
  })

  describe("notifications", () => {
    it("should send webhook notification when configured", async () => {
      const webhookManager = createEscalationManager({
        notificationChannel: "webhook",
        webhookUrl: "http://localhost:9999/webhook",
      })
      webhookManager.start()

      await webhookManager.escalate(
        "high",
        "Webhook Test",
        "Testing webhook notification",
        createContext(),
      )

      expect(fetchSpy).toHaveBeenCalled()

      webhookManager.stop()
      webhookManager.clear()
    })
  })
})
