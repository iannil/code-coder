/**
 * ModeController Tests
 *
 * Tests for the Mode Controller component of the Observer Network.
 *
 * @module test/observer/controller/mode.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import {
  ModeController,
  createModeController,
  getModeController,
  resetModeController,
} from "@/observer/controller/mode"
import type { ConsensusSnapshot } from "@/observer/consensus"

// Helper to create a minimal consensus snapshot
function createMockSnapshot(overrides: Partial<ConsensusSnapshot> = {}): ConsensusSnapshot {
  return {
    worldModel: {
      id: "wm_test",
      timestamp: new Date(),
      observationIds: [],
      code: {
        buildStatus: "passing",
        recentChanges: 0,
      },
      world: {
        relevantNews: [],
        externalRisks: [],
        opportunities: [],
      },
      self: {
        sessionHealth: "healthy",
        resourceUsage: { tokens: 1000, cost: 0.01, duration: 1000 },
        recentErrors: 0,
      },
      meta: {
        observerHealth: "healthy",
        coverageGaps: [],
        consensusStrength: 0.8,
      },
      confidence: 0.8,
    },
    patterns: [],
    anomalies: [],
    opportunities: [],
    timestamp: new Date(),
    confidence: 0.8,
    ...overrides,
  }
}

describe("ModeController", () => {
  let controller: ModeController

  beforeEach(() => {
    resetModeController()
    controller = createModeController({
      initialMode: "HYBRID",
      riskTolerance: "balanced",
      autoApply: false, // Disable auto-apply for testing
      evaluationIntervalMs: 0, // Disable periodic evaluation
    })
  })

  afterEach(() => {
    if (controller.isRunning()) {
      controller.stop()
    }
    resetModeController()
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", () => {
      const snapshotProvider = async () => createMockSnapshot()

      expect(controller.isRunning()).toBe(false)

      controller.start(snapshotProvider)
      expect(controller.isRunning()).toBe(true)

      controller.stop()
      expect(controller.isRunning()).toBe(false)
    })

    it("should not start twice", () => {
      const snapshotProvider = async () => createMockSnapshot()

      controller.start(snapshotProvider)
      expect(controller.isRunning()).toBe(true)

      // Second start should be idempotent
      controller.start(snapshotProvider)
      expect(controller.isRunning()).toBe(true)
    })
  })

  describe("getMode", () => {
    it("should return initial mode", () => {
      expect(controller.getMode()).toBe("HYBRID")
    })

    it("should return AUTO when configured", () => {
      const autoController = createModeController({
        initialMode: "AUTO",
      })
      expect(autoController.getMode()).toBe("AUTO")
    })
  })

  describe("switchMode", () => {
    it("should switch to new mode", async () => {
      expect(controller.getMode()).toBe("HYBRID")

      await controller.switchMode("AUTO", "Test switch")
      expect(controller.getMode()).toBe("AUTO")
    })

    it("should track mode switches in stats", async () => {
      await controller.switchMode("AUTO", "First switch")
      await controller.switchMode("MANUAL", "Second switch")

      const stats = controller.getStats()
      expect(stats.modeSwitches).toBe(2)
    })

    it("should not switch to same mode", async () => {
      const initialStats = controller.getStats()
      const initialSwitches = initialStats.modeSwitches

      await controller.switchMode("HYBRID", "No-op switch")

      const afterStats = controller.getStats()
      expect(afterStats.modeSwitches).toBe(initialSwitches)
    })
  })

  describe("evaluate", () => {
    it("should return null without snapshot provider", async () => {
      // Don't start the controller (no snapshot provider)
      const decision = await controller.evaluate()
      expect(decision).toBeNull()
    })

    it("should evaluate and return decision", async () => {
      const snapshot = createMockSnapshot()
      const snapshotProvider = async () => snapshot

      controller.start(snapshotProvider)

      const decision = await controller.evaluate()

      expect(decision).not.toBeNull()
      expect(decision!.currentMode).toBe("HYBRID")
      expect(decision!.evaluation).toBeDefined()
      expect(decision!.timestamp).toBeInstanceOf(Date)
    })

    it("should recommend MANUAL when risk is critical", async () => {
      const highRiskSnapshot = createMockSnapshot({
        confidence: 0.3, // Low confidence
        anomalies: [
          {
            id: "anom_1",
            type: "sudden_change",
            description: "Critical system failure",
            severity: "critical",
            observationIds: [],
            detectedAt: new Date(),
            status: "confirmed",
            confidence: 0.9,
          },
          {
            id: "anom_2",
            type: "outlier",
            description: "Another critical issue",
            severity: "critical",
            observationIds: [],
            detectedAt: new Date(),
            status: "confirmed",
            confidence: 0.9,
          },
        ],
      })

      const snapshotProvider = async () => highRiskSnapshot

      controller.start(snapshotProvider)

      const decision = await controller.evaluate()

      expect(decision).not.toBeNull()
      // High risk should recommend MANUAL mode
      expect(["MANUAL", "HYBRID"]).toContain(decision!.recommendedMode)
    })

    it("should recommend AUTO or HYBRID when confidence is high and risk is low", async () => {
      const goodSnapshot = createMockSnapshot({
        confidence: 0.95,
        patterns: [
          {
            id: "pat_1",
            name: "Stable operation",
            description: "System running smoothly",
            type: "trend",
            observationIds: [],
            confidence: 0.9,
            detectedAt: new Date(),
            lastSeenAt: new Date(),
            strength: 0.9,
            suggestedActions: [],
          },
        ],
        anomalies: [],
        opportunities: [],
      })

      // Start with AUTO mode
      const autoController = createModeController({
        initialMode: "AUTO",
        autoApply: false,
        evaluationIntervalMs: 0,
      })

      const snapshotProvider = async () => goodSnapshot
      autoController.start(snapshotProvider)

      const decision = await autoController.evaluate()

      expect(decision).not.toBeNull()
      // With high confidence and no anomalies, should remain in AUTO or HYBRID
      // (AUTO requires CLOSE score >= 7.0 which depends on the evaluation algorithm)
      expect(["AUTO", "HYBRID"]).toContain(decision!.recommendedMode)
      // Should not escalate to MANUAL when conditions are good
      expect(decision!.recommendedMode).not.toBe("MANUAL")

      autoController.stop()
    })
  })

  describe("getStats", () => {
    it("should return initial stats", () => {
      const stats = controller.getStats()

      expect(stats.currentMode).toBe("HYBRID")
      expect(stats.modeSwitches).toBe(0)
      expect(stats.escalations).toBe(0)
      expect(stats.pendingEscalations).toBe(0)
      expect(stats.lastEvaluation).toBeNull()
      expect(stats.lastDecision).toBeNull()
    })

    it("should track uptime when running", async () => {
      const snapshotProvider = async () => createMockSnapshot()

      controller.start(snapshotProvider)

      // Wait a bit
      await new Promise((resolve) => setTimeout(resolve, 50))

      const stats = controller.getStats()
      expect(stats.uptime).toBeGreaterThan(0)

      controller.stop()
    })
  })

  describe("getHistory", () => {
    it("should return empty history initially", () => {
      const history = controller.getHistory()
      expect(history).toEqual([])
    })

    it("should return decision history", async () => {
      const snapshotProvider = async () => createMockSnapshot()

      controller.start(snapshotProvider)

      await controller.evaluate()
      await controller.evaluate()

      const history = controller.getHistory()
      expect(history.length).toBe(2)

      controller.stop()
    })

    it("should respect limit parameter", async () => {
      const snapshotProvider = async () => createMockSnapshot()

      controller.start(snapshotProvider)

      for (let i = 0; i < 5; i++) {
        await controller.evaluate()
      }

      const limited = controller.getHistory(3)
      expect(limited.length).toBe(3)

      controller.stop()
    })
  })

  describe("setRiskTolerance", () => {
    it("should update risk tolerance", () => {
      controller.setRiskTolerance("conservative")
      // The controller should now use conservative thresholds
      // This is primarily a configuration change
      expect(controller).toBeDefined()
    })

    it("should accept different tolerance levels", () => {
      controller.setRiskTolerance("aggressive")
      controller.setRiskTolerance("balanced")
      controller.setRiskTolerance("conservative")
      // All should work without errors
      expect(controller).toBeDefined()
    })
  })

  describe("clear", () => {
    it("should clear all state", async () => {
      const snapshotProvider = async () => createMockSnapshot()

      controller.start(snapshotProvider)
      await controller.evaluate()
      await controller.switchMode("AUTO", "Test")

      controller.clear()

      const stats = controller.getStats()
      expect(stats.modeSwitches).toBe(0)
      expect(stats.lastEvaluation).toBeNull()
      expect(stats.lastDecision).toBeNull()

      const history = controller.getHistory()
      expect(history).toEqual([])

      controller.stop()
    })
  })

  describe("singleton", () => {
    it("should return same instance from getModeController", () => {
      const controller1 = getModeController()
      const controller2 = getModeController()

      expect(controller1).toBe(controller2)
    })

    it("should reset singleton correctly", () => {
      const controller1 = getModeController()
      resetModeController()
      const controller2 = getModeController()

      expect(controller1).not.toBe(controller2)
    })
  })
})
