/**
 * SelfWatch Tests
 *
 * Tests for the Self Watcher component of the Observer Network.
 *
 * @module test/observer/watchers/self-watch.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import { SelfWatch, createSelfWatch } from "@/observer/watchers/self-watch"
import { resetEventStream } from "@/observer"

describe("SelfWatch", () => {
  let watcher: SelfWatch

  beforeEach(() => {
    resetEventStream()
    watcher = createSelfWatch({
      intervalMs: 0,
      costSpikeThreshold: 2.0,
      costHistorySize: 10,
    })
  })

  afterEach(async () => {
    if (watcher.isRunning()) {
      await watcher.stop()
    }
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", async () => {
      expect(watcher.isRunning()).toBe(false)

      await watcher.start()
      expect(watcher.isRunning()).toBe(true)

      await watcher.stop()
      expect(watcher.isRunning()).toBe(false)
    })
  })

  describe("getStatus", () => {
    it("should return correct status when stopped", () => {
      const status = watcher.getStatus()
      expect(status.type).toBe("self")
      expect(status.running).toBe(false)
      expect(status.health).toBe("stopped")
    })

    it("should return correct status when running", async () => {
      await watcher.start()
      const status = watcher.getStatus()

      expect(status.type).toBe("self")
      expect(status.running).toBe(true)
      expect(status.health).toBe("healthy")
    })
  })

  describe("observeAgentAction", () => {
    it("should emit observation for agent actions", async () => {
      await watcher.start()

      await watcher.observeAgentAction({
        agentId: "test-agent",
        action: "code_review",
        duration: 1000,
        success: true,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should track errors in recent actions", async () => {
      await watcher.start()

      await watcher.observeAgentAction({
        agentId: "test-agent",
        action: "failed_action",
        duration: 500,
        success: false,
        error: "Something went wrong",
      })

      const errors = watcher.getRecentErrors()
      expect(errors.length).toBe(1)
      expect(errors[0].error).toBe("Something went wrong")
    })

    it("should not observe when stopped", async () => {
      await watcher.observeAgentAction({
        agentId: "test-agent",
        action: "test",
        duration: 100,
        success: true,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(0)
    })
  })

  describe("observeResourceUsage", () => {
    it("should emit observation for resource usage", async () => {
      await watcher.start()

      await watcher.observeResourceUsage({
        agentId: "test-agent",
        tokens: 5000,
        cost: 0.01,
        duration: 2000,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should calculate resource summary", async () => {
      await watcher.start()

      await watcher.observeResourceUsage({
        agentId: "agent1",
        tokens: 1000,
        cost: 0.01,
        duration: 1000,
      })

      await watcher.observeResourceUsage({
        agentId: "agent2",
        tokens: 2000,
        cost: 0.02,
        duration: 2000,
      })

      const summary = watcher.getResourceSummary()
      expect(summary.totalTokens).toBe(3000)
      expect(summary.totalCost).toBeCloseTo(0.03)
      expect(summary.totalDuration).toBe(3000)
    })
  })

  describe("observeToolInvocation", () => {
    it("should emit observation for tool invocation", async () => {
      await watcher.start()

      await watcher.observeToolInvocation({
        toolName: "read_file",
        agentId: "test-agent",
        input: { path: "/test/file.ts" },
        output: "file content",
        duration: 50,
        success: true,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("observeQualityMetric", () => {
    it("should emit observation for quality metrics", async () => {
      await watcher.start()

      await watcher.observeQualityMetric({
        agentId: "test-agent",
        metricName: "code_quality",
        value: 0.85,
        closeScore: 7.5,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("session cost tracking", () => {
    it("should return empty cost history initially", () => {
      const history = watcher.getSessionCostHistory()
      expect(history).toEqual([])
    })

    it("should return cost statistics", () => {
      const stats = watcher.getCostStatistics()
      expect(stats.totalCost).toBe(0)
      expect(stats.avgCost).toBe(0)
      expect(stats.sessionCount).toBe(0)
    })
  })

  describe("recent actions tracking", () => {
    it("should return recent actions with limit", async () => {
      await watcher.start()

      for (let i = 0; i < 5; i++) {
        await watcher.observeAgentAction({
          agentId: `agent-${i}`,
          action: `action-${i}`,
          duration: 100,
          success: true,
        })
      }

      const actions = watcher.getRecentActions(3)
      expect(actions.length).toBe(3)
    })
  })
})
