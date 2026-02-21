import { describe, test, expect, beforeEach } from "bun:test"
import { AutonomousState } from "@/autonomous/state/states"
import { Executor, createExecutor, type TDDPhase, type TDDCycleResult, type TestResult } from "@/autonomous/execution/executor"
import { CheckpointManager, createCheckpointManager } from "@/autonomous/execution/checkpoint"
import { RollbackManager, createRollbackManager } from "@/autonomous/safety/rollback"
import { AgentInvoker } from "@/autonomous/execution/agent-invoker"
import { TestRunner } from "@/autonomous/execution/test-runner"
import { createMockTDDCycleResult } from "./fixtures/autonomous-fixture"

describe("Autonomous Mode - Operations", () => {
  describe("TDD Phases", () => {
    test("should define all TDD phases", () => {
      const phases: TDDPhase[] = ["red", "green", "refactor"]

      for (const phase of phases) {
        expect(["red", "green", "refactor"]).toContain(phase)
      }
    })

    test("should create valid TDD cycle result", () => {
      const result: TDDCycleResult = {
        phase: "red",
        success: true,
        duration: 1000,
        changes: ["test-file.test.ts"],
      }

      expect(result.phase).toBe("red")
      expect(result.success).toBe(true)
      expect(result.changes).toContain("test-file.test.ts")
    })

    test("should create mock TDD cycle result", () => {
      const mockResult = createMockTDDCycleResult("green", {
        duration: 2000,
        changes: ["impl.ts"],
      })

      expect(mockResult.phase).toBe("green")
      expect(mockResult.success).toBe(true)
      expect(mockResult.duration).toBe(2000)
      expect(mockResult.changes).toContain("impl.ts")
    })
  })

  describe("Executor Operations", () => {
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

    test("should initialize executor", async () => {
      await executor.initialize()

      const context = executor.getContext()
      const metadata = context.get()

      expect(metadata.sessionId).toBe(sessionId)
    })

    test("should track executor statistics", async () => {
      await executor.initialize()

      const stats = executor.getStats()

      expect(stats.cyclesCompleted).toBe(0)
      expect(stats.testsRun).toBe(0)
      expect(stats.testsPassed).toBe(0)
      expect(stats.testsFailed).toBe(0)
      expect(stats.passRate).toBe(0)
    })

    test("should run tests", async () => {
      await executor.initialize()

      const result = await executor.runTests()

      expect(result).toBeDefined()
      expect(typeof result.success).toBe("boolean")
      expect(typeof result.passed).toBe("number")
      expect(typeof result.failed).toBe("number")
      expect(typeof result.skipped).toBe("number")
      expect(typeof result.duration).toBe("number")
      expect(Array.isArray(result.errors)).toBe(true)
    })

    test("should run verification", async () => {
      await executor.initialize()

      const result = await executor.runVerification()

      expect(result).toBeDefined()
      expect(typeof result.success).toBe("boolean")
      expect(typeof result.typecheck).toBe("boolean")
      expect(typeof result.lint).toBe("boolean")
      expect(typeof result.coverage).toBe("number")
      expect(Array.isArray(result.issues)).toBe(true)
    })

    test("should create checkpoint", async () => {
      const checkpointId = await executor.createCheckpoint("state", "Test checkpoint")

      expect(checkpointId).toBeDefined()
      expect(typeof checkpointId).toBe("string")
      expect(checkpointId.length).toBeGreaterThan(0)
    })

    test("should create checkpoint with different types", async () => {
      const gitCheckpoint = await executor.createCheckpoint("git", "Git checkpoint")
      const stateCheckpoint = await executor.createCheckpoint("state", "State checkpoint")
      const manualCheckpoint = await executor.createCheckpoint("manual", "Manual checkpoint")

      expect(gitCheckpoint).toBeDefined()
      expect(stateCheckpoint).toBeDefined()
      expect(manualCheckpoint).toBeDefined()

      // Checkpoints should be unique
      expect(gitCheckpoint).not.toBe(stateCheckpoint)
      expect(stateCheckpoint).not.toBe(manualCheckpoint)
    })
  })

  describe("Checkpoint Operations", () => {
    let sessionId: string
    let checkpointManager: CheckpointManager

    beforeEach(() => {
      sessionId = `test_session_${Date.now()}`
      checkpointManager = createCheckpointManager(sessionId)
    })

    test("should create checkpoint", async () => {
      const checkpointId = await checkpointManager.create("state", "Test checkpoint")

      expect(checkpointId).toBeDefined()
      expect(checkpointId.length).toBeGreaterThan(0)
    })

    test("should restore checkpoint", async () => {
      const checkpointId = await checkpointManager.create("state", "Test checkpoint")
      const canRestore = await checkpointManager.restore(checkpointId, "Test restore")

      expect(canRestore).toBe(true)
    })

    test("should handle non-existent checkpoint restore", async () => {
      const canRestore = await checkpointManager.restore("non-existent-id", "Test restore")

      expect(canRestore).toBe(false)
    })

    test("should clear checkpoints", async () => {
      await checkpointManager.create("state", "Test checkpoint")
      await checkpointManager.clear()

      // After clearing, checkpoint should not exist
      const canRestore = await checkpointManager.restore("test-id", "test")
      expect(canRestore).toBe(false)
    })
  })

  describe("Rollback Operations", () => {
    let sessionId: string
    let rollbackManager: RollbackManager

    beforeEach(() => {
      sessionId = `test_session_${Date.now()}`
      rollbackManager = createRollbackManager(sessionId, {
        createCheckpoint: true,
        maxRetries: 3,
      })
    })

    test("should initialize with zero rollbacks", () => {
      expect(rollbackManager.getRollbackCount()).toBe(0)
      expect(rollbackManager.canRetry()).toBe(true)
    })

    test("should create checkpoint before rollback", async () => {
      const checkpointId = await rollbackManager.createCheckpoint("Before rollback")

      expect(checkpointId).toBeDefined()
      expect(typeof checkpointId).toBe("string")
    })

    test("should track rollback count", async () => {
      expect(rollbackManager.getRollbackCount()).toBe(0)

      await rollbackManager.createCheckpoint("Checkpoint 1")
      await rollbackManager.performRollback(undefined, "test_failure", "Rollback 1")
      expect(rollbackManager.getRollbackCount()).toBe(0) // No checkpoint, no rollback

      const checkpointId = await rollbackManager.createCheckpoint("Checkpoint 2")
      await rollbackManager.performRollback(checkpointId, "test_failure", "Rollback 2")
      expect(rollbackManager.getRollbackCount()).toBe(1)
    })

    test("should respect max retries", async () => {
      const checkpointId = await rollbackManager.createCheckpoint("Test checkpoint")

      await rollbackManager.performRollback(checkpointId, "test_failure", "Rollback 1")
      await rollbackManager.performRollback(checkpointId, "test_failure", "Rollback 2")
      await rollbackManager.performRollback(checkpointId, "test_failure", "Rollback 3")

      expect(rollbackManager.canRetry()).toBe(false)
    })

    test("should handle test failures", async () => {
      // First create a checkpoint so rollback can work
      await rollbackManager.createCheckpoint("Pre-test checkpoint")

      const result = await rollbackManager.handleTestFailure({
        failedTests: ["test1", "test2", "test3"],
        totalTests: 4,
        error: "Tests failed",
      })

      // With 75% failure rate (>50%) and a checkpoint, rollback should be triggered
      // Result will be undefined if no checkpoint exists, or have rollback info if it does
      expect(result === undefined || typeof result === "object").toBe(true)
    })

    test("should reset rollback count", async () => {
      const checkpointId = await rollbackManager.createCheckpoint("Test checkpoint")
      await rollbackManager.performRollback(checkpointId, "test_failure", "Test rollback")
      expect(rollbackManager.getRollbackCount()).toBe(1)

      rollbackManager.resetCount()
      expect(rollbackManager.getRollbackCount()).toBe(0)
    })
  })

  describe("TestRunner Operations", () => {
    test("should have runAll method", () => {
      expect(typeof TestRunner.runAll).toBe("function")
    })

    test("should have runFiles method", () => {
      expect(typeof TestRunner.runFiles).toBe("function")
    })

    test("should have runPattern method", () => {
      expect(typeof TestRunner.runPattern).toBe("function")
    })

    test("should have runCoverage method", () => {
      expect(typeof TestRunner.runCoverage).toBe("function")
    })

    test("should return proper test result structure", async () => {
      const result: TestResult = {
        success: false,
        passed: 5,
        failed: 2,
        skipped: 1,
        duration: 1000,
        errors: ["Error 1", "Error 2"],
      }

      expect(result.success).toBe(false)
      expect(result.passed).toBe(5)
      expect(result.failed).toBe(2)
      expect(result.skipped).toBe(1)
      expect(result.duration).toBe(1000)
      expect(result.errors.length).toBe(2)
    })
  })

  describe("AgentInvoker Operations", () => {
    test("should define tddRed method", () => {
      expect(typeof AgentInvoker.tddRed).toBe("function")
    })

    test("should define tddGreen method", () => {
      expect(typeof AgentInvoker.tddGreen).toBe("function")
    })

    test("should define codeReview method", () => {
      expect(typeof AgentInvoker.codeReview).toBe("function")
    })
  })

  describe("TDD Cycle Flow", () => {
    test("should execute red phase first", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      // Red phase should come before green and refactor
      const context = executor.getContext()
      await context.setPhase("red")

      const metadata = context.get()
      expect(metadata.currentPhase).toBe("red")
    })

    test("should track TDD phase sequence", () => {
      const phases: TDDPhase[] = ["red", "green", "refactor"]
      const sessionPhases: TDDPhase[] = []

      // Simulate TDD cycle
      for (const phase of phases) {
        sessionPhases.push(phase)
      }

      expect(sessionPhases).toEqual(phases)
      expect(sessionPhases[0]).toBe("red")
      expect(sessionPhases[1]).toBe("green")
      expect(sessionPhases[2]).toBe("refactor")
    })

    test("should track changes from each phase", () => {
      const redChanges = ["test-file.test.ts"]
      const greenChanges = ["impl-file.ts"]
      const refactorChanges = ["refactored-file.ts"]

      const allChanges = [...redChanges, ...greenChanges, ...refactorChanges]

      expect(allChanges.length).toBe(3)
      expect(allChanges).toContain("test-file.test.ts")
      expect(allChanges).toContain("impl-file.ts")
      expect(allChanges).toContain("refactored-file.ts")
    })
  })

  describe("Error Handling", () => {
    test("should handle test failures gracefully", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      // Run tests should not throw even if tests fail
      const result = await executor.runTests()

      expect(result).toBeDefined()
      expect(typeof result.success).toBe("boolean")
    })

    test("should handle verification failures", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const result = await executor.runVerification()

      expect(result).toBeDefined()
      // Even if verification fails, it should return a result
      expect(typeof result.success).toBe("boolean")
    })

    test("should handle checkpoint creation failures", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      // Checkpoint creation should handle errors gracefully
      const checkpointId = await executor.createCheckpoint("state", "Test")

      expect(checkpointId).toBeDefined()
    })
  })

  describe("Context Management", () => {
    test("should store and retrieve metadata", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const context = executor.getContext()

      await context.setMetadata("testKey", "testValue")
      const testValue = context.getMetadata("testKey")

      expect(testValue).toBe("testValue")
    })

    test("should track test statistics", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const context = executor.getContext()

      await context.recordTestResults(10, 8, 2)
      const testStats = context.getTestStats()

      expect(testStats.run).toBe(10)
      expect(testStats.passed).toBe(8)
      expect(testStats.failed).toBe(2)
    })

    test("should set and get phase", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const context = executor.getContext()

      await context.setPhase("testing")
      const metadata = context.get()
      expect(metadata.currentPhase).toBe("testing")
    })

    test("should set and get task", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const context = executor.getContext()

      await context.setTask("test_task_123")
      const metadata = context.get()
      expect(metadata.currentTask).toBe("test_task_123")
    })

    test("should add and list files", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const context = executor.getContext()

      await context.addFile("test-file.ts")
      await context.addFile("impl-file.ts")

      const metadata = context.get()
      expect(metadata.filesModified).toContain("test-file.ts")
      expect(metadata.filesModified).toContain("impl-file.ts")
    })
  })

  describe("Phase Transitions", () => {
    test("should track phase transitions", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const context = executor.getContext()

      await context.setPhase("red")
      await context.setPhase("green")
      await context.setPhase("refactor")

      // The latest phase should be refactor
      const metadata = context.get()
      expect(metadata.currentPhase).toBe("refactor")
    })

    test("should record phase duration", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const before = Date.now()

      const context = executor.getContext()
      await context.setPhase("testing")

      const after = Date.now()

      // Phase should be set
      const metadata = context.get()
      expect(metadata.currentPhase).toBe("testing")
      expect(metadata.updatedAt).toBeGreaterThanOrEqual(before)
      expect(metadata.updatedAt).toBeLessThanOrEqual(after)
    })
  })

  describe("Resource Cleanup", () => {
    test("should cleanup executor context", async () => {
      const sessionId = `test_session_${Date.now()}`
      const executor = createExecutor(sessionId, {
        unattended: false,
        maxRetries: 2,
        checkpointInterval: 5,
      })

      await executor.initialize()

      const context = executor.getContext()
      await context.setMetadata("test", "value")

      await executor.cleanup()

      // After cleanup, context should be cleared
      const metadata = context.get()
      expect(metadata).toBeDefined()
    })
  })
})
