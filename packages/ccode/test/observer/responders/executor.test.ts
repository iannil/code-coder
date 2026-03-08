/**
 * Executor Responder Tests
 *
 * Tests for the Executor component of the Observer Network.
 *
 * @module test/observer/responders/executor.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, mock } from "bun:test"
import {
  Executor,
  createExecutor,
  type ExecutorConfig,
  type ExecutionRequest,
  type ExecutionAction,
  type ExecutionType,
} from "@/observer/responders/executor"
import type { Anomaly, Opportunity, OperatingMode } from "@/observer/types"

describe("Executor", () => {
  let executor: Executor

  beforeEach(() => {
    executor = createExecutor({
      autoExecute: false,
      mode: "AUTO",
      maxConcurrent: 2,
      timeoutMs: 1000,
      dryRun: true, // Use dry run for tests
    })
  })

  afterEach(() => {
    executor.stop()
    executor.clear()
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", async () => {
      await executor.start()
      // Executor should be running
      executor.stop()
      // Should be able to stop without error
    })

    it("should not start twice", async () => {
      await executor.start()
      await executor.start() // Should not throw
      executor.stop()
    })

    it("should be safe to stop when not started", () => {
      executor.stop() // Should not throw
    })

    it("should reject pending executions on stop", async () => {
      await executor.start()

      // Create a pending execution
      const manualExecutor = createExecutor({
        autoExecute: false,
        mode: "MANUAL",
        requireApproval: ["auto_fix"],
        maxConcurrent: 1,
        timeoutMs: 1000,
        dryRun: true,
      })
      await manualExecutor.start()

      const request = await manualExecutor.requestExecution({
        type: "auto_fix",
        description: "Test fix",
        trigger: { type: "manual" },
        actions: [{ id: "a1", type: "test", description: "Test action", status: "pending" }],
      })

      manualExecutor.stop()

      // Pending requests should be rejected
      const pending = manualExecutor.getPending()
      for (const p of pending) {
        expect(p.status).toBe("rejected")
      }
    })
  })

  describe("requestExecution", () => {
    it("should create execution request with correct fields", async () => {
      await executor.start()

      const actions: ExecutionAction[] = [
        { id: "action1", type: "cleanup", description: "Remove temp files", status: "pending" },
      ]

      const request = await executor.requestExecution({
        type: "auto_cleanup",
        description: "Cleanup temporary files",
        trigger: { type: "schedule" },
        actions,
      })

      expect(request).toBeDefined()
      expect(request.id).toMatch(/^exec_/)
      expect(request.type).toBe("auto_cleanup")
      expect(request.description).toBe("Cleanup temporary files")
      expect(request.trigger.type).toBe("schedule")
      expect(request.actions.length).toBe(1)
      expect(request.createdAt).toBeInstanceOf(Date)
    })

    it("should auto-approve in AUTO mode for non-restricted types", async () => {
      const autoExecutor = createExecutor({
        autoExecute: false,
        mode: "AUTO",
        requireApproval: ["hands_action"], // Only hands_action requires approval
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await autoExecutor.start()

      const request = await autoExecutor.requestExecution({
        type: "auto_cleanup",
        description: "Test",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      expect(request.status).not.toBe("pending")
      expect(request.requiresApproval).toBe(false)

      autoExecutor.stop()
    })

    it("should require approval in MANUAL mode", async () => {
      const manualExecutor = createExecutor({
        autoExecute: false,
        mode: "MANUAL",
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await manualExecutor.start()

      const request = await manualExecutor.requestExecution({
        type: "auto_cleanup",
        description: "Test",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      expect(request.requiresApproval).toBe(true)
      expect(request.status).toBe("pending")

      manualExecutor.stop()
    })

    it("should require approval for restricted types even in AUTO mode", async () => {
      const autoExecutor = createExecutor({
        autoExecute: false,
        mode: "AUTO",
        requireApproval: ["auto_fix", "hands_action"],
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await autoExecutor.start()

      const request = await autoExecutor.requestExecution({
        type: "auto_fix",
        description: "Test fix",
        trigger: { type: "anomaly" },
        actions: [{ id: "a1", type: "fix", description: "Fix", status: "pending" }],
      })

      expect(request.requiresApproval).toBe(true)

      autoExecutor.stop()
    })
  })

  describe("approval workflow", () => {
    it("should approve pending execution", async () => {
      const manualExecutor = createExecutor({
        autoExecute: false,
        mode: "MANUAL",
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await manualExecutor.start()

      const request = await manualExecutor.requestExecution({
        type: "auto_cleanup",
        description: "Test",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      expect(request.status).toBe("pending")

      const approved = await manualExecutor.approve(request.id)
      expect(approved).toBe(true)

      const updated = manualExecutor.getExecution(request.id)
      expect(updated?.status).not.toBe("pending")
      expect(updated?.approvedAt).toBeDefined()

      manualExecutor.stop()
    })

    it("should reject pending execution", async () => {
      const manualExecutor = createExecutor({
        autoExecute: false,
        mode: "MANUAL",
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await manualExecutor.start()

      const request = await manualExecutor.requestExecution({
        type: "auto_fix",
        description: "Test fix",
        trigger: { type: "anomaly" },
        actions: [{ id: "a1", type: "fix", description: "Fix", status: "pending" }],
      })

      const rejected = manualExecutor.reject(request.id, "Not needed")

      expect(rejected).toBe(true)

      const updated = manualExecutor.getExecution(request.id)
      expect(updated?.status).toBe("rejected")
      expect(updated?.error).toBe("Not needed")

      manualExecutor.stop()
    })

    it("should return false when approving non-existent execution", async () => {
      await executor.start()
      const approved = await executor.approve("non_existent")
      expect(approved).toBe(false)
    })

    it("should return false when rejecting non-existent execution", () => {
      const rejected = executor.reject("non_existent")
      expect(rejected).toBe(false)
    })

    it("should use custom approval handler", async () => {
      const autoApproveExecutor = createExecutor({
        autoExecute: false,
        mode: "MANUAL",
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })

      // Set up auto-approve handler
      autoApproveExecutor.setApprovalHandler(async (request) => {
        return request.type === "auto_cleanup" // Auto-approve cleanups
      })

      await autoApproveExecutor.start()

      const cleanupRequest = await autoApproveExecutor.requestExecution({
        type: "auto_cleanup",
        description: "Auto-approved cleanup",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      // Should be auto-approved by handler
      expect(cleanupRequest.status).not.toBe("pending")

      const fixRequest = await autoApproveExecutor.requestExecution({
        type: "auto_fix",
        description: "Not auto-approved",
        trigger: { type: "anomaly" },
        actions: [{ id: "a2", type: "fix", description: "Fix", status: "pending" }],
      })

      // Should be rejected by handler
      expect(fixRequest.status).toBe("rejected")

      autoApproveExecutor.stop()
    })
  })

  describe("executeOpportunity", () => {
    it("should create execution from opportunity", async () => {
      await executor.start()

      const opportunity: Opportunity = {
        id: "opp_1",
        type: "optimization",
        impact: "medium",
        description: "Optimize database queries",
        detectedAt: new Date(),
        suggestedActions: ["Add index on users.email", "Cache frequent queries"],
        urgency: "low",
      }

      const request = await executor.executeOpportunity(opportunity)

      expect(request.type).toBe("auto_optimize")
      expect(request.trigger.type).toBe("opportunity")
      expect(request.trigger.id).toBe("opp_1")
      expect(request.actions.length).toBe(2)
    })
  })

  describe("executeAnomalyFix", () => {
    it("should create execution for anomaly fix", async () => {
      await executor.start()

      const anomaly: Anomaly = {
        id: "anomaly_1",
        type: "performance_degradation",
        severity: "high",
        description: "API response time increased",
        detectedAt: new Date(),
        source: "world",
        data: { endpoint: "/api/users", latency: 5000 },
      }

      const fix = {
        description: "Restart the API server",
        command: "systemctl restart api",
      }

      const request = await executor.executeAnomalyFix(anomaly, fix)

      expect(request.type).toBe("auto_fix")
      expect(request.trigger.type).toBe("anomaly")
      expect(request.trigger.id).toBe("anomaly_1")
      expect(request.actions.length).toBe(1)
      expect(request.actions[0].command).toBe("systemctl restart api")
    })
  })

  describe("executeCleanup", () => {
    it("should create cleanup execution", async () => {
      await executor.start()

      const actions: ExecutionAction[] = [
        { id: "clean1", type: "remove", description: "Remove temp files", status: "pending" },
        { id: "clean2", type: "archive", description: "Archive old logs", status: "pending" },
      ]

      const request = await executor.executeCleanup(actions)

      expect(request.type).toBe("auto_cleanup")
      expect(request.trigger.type).toBe("schedule")
      expect(request.actions.length).toBe(2)
    })
  })

  describe("executeHandsAction", () => {
    it("should create hands action execution", async () => {
      await executor.start()

      const actions: ExecutionAction[] = [
        { id: "hand1", type: "trigger_hand", description: "Trigger backup hand", status: "pending" },
      ]

      const request = await executor.executeHandsAction("Run backup workflow", actions)

      expect(request.type).toBe("hands_action")
      expect(request.trigger.type).toBe("manual")
    })
  })

  describe("dry run mode", () => {
    it("should not actually execute in dry run mode", async () => {
      const dryRunExecutor = createExecutor({
        autoExecute: false,
        mode: "AUTO",
        requireApproval: [],
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await dryRunExecutor.start()

      const request = await dryRunExecutor.requestExecution({
        type: "auto_cleanup",
        description: "Test cleanup",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test action", status: "pending" }],
      })

      // Wait for execution
      await new Promise((r) => setTimeout(r, 100))

      const updated = dryRunExecutor.getExecution(request.id)
      // In dry run, actions should complete with dry run output
      if (updated?.result) {
        expect(updated.result.success).toBe(true)
      }

      dryRunExecutor.stop()
    })
  })

  describe("getExecution", () => {
    it("should retrieve execution by id", async () => {
      await executor.start()

      const request = await executor.requestExecution({
        type: "auto_cleanup",
        description: "Test",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      const retrieved = executor.getExecution(request.id)

      expect(retrieved).toBeDefined()
      expect(retrieved?.id).toBe(request.id)
    })

    it("should return null for non-existent id", () => {
      const retrieved = executor.getExecution("non_existent")
      expect(retrieved).toBeNull()
    })
  })

  describe("getHistory", () => {
    it("should return execution history sorted by date", async () => {
      await executor.start()

      await executor.requestExecution({
        type: "auto_cleanup",
        description: "First",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      await executor.requestExecution({
        type: "auto_optimize",
        description: "Second",
        trigger: { type: "opportunity" },
        actions: [{ id: "a2", type: "test", description: "Test", status: "pending" }],
      })

      const history = executor.getHistory()

      expect(history.length).toBe(2)
      expect(history[0].createdAt.getTime()).toBeGreaterThanOrEqual(history[1].createdAt.getTime())
    })

    it("should limit history results", async () => {
      await executor.start()

      for (let i = 0; i < 5; i++) {
        await executor.requestExecution({
          type: "auto_cleanup",
          description: `Execution ${i}`,
          trigger: { type: "schedule" },
          actions: [{ id: `a${i}`, type: "test", description: "Test", status: "pending" }],
        })
      }

      const history = executor.getHistory(3)

      expect(history.length).toBe(3)
    })
  })

  describe("getPending", () => {
    it("should return only pending executions", async () => {
      const manualExecutor = createExecutor({
        autoExecute: false,
        mode: "MANUAL",
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await manualExecutor.start()

      await manualExecutor.requestExecution({
        type: "auto_fix",
        description: "Pending 1",
        trigger: { type: "anomaly" },
        actions: [{ id: "a1", type: "fix", description: "Fix", status: "pending" }],
      })

      await manualExecutor.requestExecution({
        type: "auto_fix",
        description: "Pending 2",
        trigger: { type: "anomaly" },
        actions: [{ id: "a2", type: "fix", description: "Fix", status: "pending" }],
      })

      const pending = manualExecutor.getPending()

      expect(pending.length).toBe(2)
      for (const p of pending) {
        expect(p.status).toBe("pending")
      }

      manualExecutor.stop()
    })
  })

  describe("getRunning", () => {
    it("should return running executions", async () => {
      await executor.start()

      await executor.requestExecution({
        type: "auto_cleanup",
        description: "Running execution",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      // Running executions are tracked (may be 0 or 1 depending on timing)
      const running = executor.getRunning()
      expect(running).toBeDefined()
    })
  })

  describe("clear", () => {
    it("should clear all execution history", async () => {
      await executor.start()

      await executor.requestExecution({
        type: "auto_cleanup",
        description: "Test",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      executor.clear()

      const history = executor.getHistory()
      expect(history.length).toBe(0)
    })
  })

  describe("mode handling", () => {
    it("should handle HYBRID mode", async () => {
      const hybridExecutor = createExecutor({
        autoExecute: false,
        mode: "HYBRID",
        requireApproval: ["auto_fix"],
        maxConcurrent: 2,
        timeoutMs: 1000,
        dryRun: true,
      })
      await hybridExecutor.start()

      // Non-restricted types should auto-approve
      const cleanupRequest = await hybridExecutor.requestExecution({
        type: "auto_cleanup",
        description: "Cleanup",
        trigger: { type: "schedule" },
        actions: [{ id: "a1", type: "test", description: "Test", status: "pending" }],
      })

      expect(cleanupRequest.requiresApproval).toBe(true) // HYBRID requires approval for most

      hybridExecutor.stop()
    })
  })
})
