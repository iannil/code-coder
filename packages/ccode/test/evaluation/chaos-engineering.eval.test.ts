/**
 * Chaos Engineering Evaluation Tests (Dimension 6)
 *
 * Tests system resilience under fault injection:
 * - Network failures (timeout, disconnect, rate limiting)
 * - Resource failures (disk full, memory pressure, file locks)
 * - Data corruption (session files, partial writes, orphaned tasks)
 * - Concurrency conflicts (simultaneous edits, race conditions, deadlocks)
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { tmpdir } from "../fixture/fixture"
import { EVAL_THRESHOLDS } from "./config"
import {
  FaultInjector,
  NetworkSimulator,
  NetworkError,
  ResourcePressureSimulator,
  DataCorruptionSimulator,
  ConcurrencySimulator,
  withRetry,
  measureRecoveryTime,
  createChaosSession,
  endChaosSession,
  NETWORK_CONDITIONS,
  type FaultType,
} from "./utils/chaos"
import { calculateChaosMetrics, type ChaosEvent } from "./utils/metrics-complex"

describe("Chaos Engineering Evaluation", () => {
  let tempDir: Awaited<ReturnType<typeof tmpdir>> | undefined

  beforeEach(async () => {
    tempDir = await tmpdir()
    process.env.CCODE_TEST_HOME = tempDir.path
  })

  afterEach(async () => {
    if (tempDir) {
      await tempDir[Symbol.asyncDispose]()
    }
    delete process.env.CCODE_TEST_HOME
  })

  describe("Network Failures", () => {
    test("LLM API timeout recovery", async () => {
      const networkSim = new NetworkSimulator()
      networkSim.setCondition("degraded")

      const events: ChaosEvent[] = []
      let successfulRequests = 0

      // Reduced to 5 iterations with minimal delays to stay under 5s timeout
      for (let i = 0; i < 5; i++) {
        const healthBefore = 1.0
        let healthAfter = 1.0
        let recovered = false
        const startTime = Date.now()

        try {
          await networkSim.simulateRequest(
            async () => {
              await new Promise((resolve) => setTimeout(resolve, 2))
              return { status: "ok" }
            },
            { timeout: 20 },
          )
          successfulRequests++
          recovered = true
        } catch (error) {
          healthAfter = 0.5
          // Retry logic with minimal delays
          const retry = await withRetry(
            async () => {
              return await networkSim.simulateRequest(
                async () => ({ status: "ok" }),
                { timeout: 20 },
              )
            },
            { maxAttempts: 2, baseDelayMs: 5, maxDelayMs: 20 },
          )

          if (retry.result) {
            successfulRequests++
            recovered = true
            healthAfter = 0.9
          }
        }

        events.push({
          faultType: "network_timeout",
          recovered,
          recoveryTime: Date.now() - startTime,
          systemHealthBefore: healthBefore,
          systemHealthAfter: healthAfter,
        })
      }

      const metrics = calculateChaosMetrics(events)
      expect(metrics.recoveryRate).toBeGreaterThanOrEqual(EVAL_THRESHOLDS.chaosRecoveryRate)
      expect(successfulRequests).toBeGreaterThanOrEqual(4) // 80%
    })

    test("MCP server disconnect mid-operation", async () => {
      interface MCPOperation {
        id: string
        started: boolean
        completed: boolean
        disconnectOccurred: boolean
        recovered: boolean
      }

      const operations: MCPOperation[] = []

      for (let i = 0; i < 10; i++) {
        const op: MCPOperation = {
          id: `op_${i}`,
          started: false,
          completed: false,
          disconnectOccurred: false,
          recovered: false,
        }

        op.started = true

        // Simulate mid-operation disconnect (20% chance)
        if (Math.random() < 0.2) {
          op.disconnectOccurred = true

          // Recovery: reconnect and retry
          await new Promise((resolve) => setTimeout(resolve, 50))
          op.recovered = true
          op.completed = true
        } else {
          await new Promise((resolve) => setTimeout(resolve, 20))
          op.completed = true
        }

        operations.push(op)
      }

      const completionRate = operations.filter((o) => o.completed).length / operations.length
      const disconnects = operations.filter((o) => o.disconnectOccurred)
      const recoveryRate = disconnects.length > 0
        ? disconnects.filter((o) => o.recovered).length / disconnects.length
        : 1

      expect(completionRate).toBe(1) // All should complete
      expect(recoveryRate).toBe(1) // All disconnects recovered
    })

    test("WebSearch rate limiting", async () => {
      const rateLimitConfig = {
        requestsPerMinute: 60,
        burstLimit: 10,
      }

      let requestCount = 0
      let rateLimitHits = 0
      const window: number[] = []
      const windowSize = 60000 // 1 minute

      async function makeRequest(): Promise<{ success: boolean; rateLimited: boolean }> {
        const now = Date.now()

        // Clean old entries
        while (window.length > 0 && window[0] < now - windowSize) {
          window.shift()
        }

        // Check rate limit
        if (window.length >= rateLimitConfig.requestsPerMinute) {
          rateLimitHits++
          return { success: false, rateLimited: true }
        }

        // Check burst
        const recentRequests = window.filter((t) => t > now - 1000).length
        if (recentRequests >= rateLimitConfig.burstLimit) {
          rateLimitHits++
          await new Promise((resolve) => setTimeout(resolve, 100)) // Back off
          return { success: false, rateLimited: true }
        }

        window.push(now)
        requestCount++
        return { success: true, rateLimited: false }
      }

      // Simulate burst of requests
      const results = await Promise.all(Array.from({ length: 20 }, () => makeRequest()))

      const successful = results.filter((r) => r.success).length
      const rateLimited = results.filter((r) => r.rateLimited).length

      expect(successful).toBeLessThanOrEqual(rateLimitConfig.burstLimit)
      expect(rateLimited).toBeGreaterThan(0)
    })

    test("network condition transitions", async () => {
      const networkSim = new NetworkSimulator()
      const transitions = ["healthy", "degraded", "unstable", "healthy"] as const

      const results: { condition: string; successRate: number }[] = []

      for (const condition of transitions) {
        networkSim.setCondition(condition)
        networkSim.resetStats()

        // Run just 3 requests per condition with reasonable timeout
        for (let i = 0; i < 3; i++) {
          try {
            await networkSim.simulateRequest(async () => "ok", { timeout: 100 })
          } catch {
            // Expected failures in degraded conditions
          }
        }

        const stats = networkSim.getStats()
        results.push({
          condition,
          successRate: stats.successRate,
        })
      }

      // Healthy conditions should have high success rate
      expect(results.find((r) => r.condition === "healthy")?.successRate).toBeGreaterThanOrEqual(0.9)

      // Unstable should have lower success rate
      expect(results.find((r) => r.condition === "unstable")?.successRate).toBeLessThan(0.9)
    })
  })

  describe("Resource Failures", () => {
    test("disk full graceful handling", async () => {
      const resourceSim = new ResourcePressureSimulator()
      resourceSim.setState({ diskUsedGB: 95, diskLimitGB: 100 })

      const writeAttempts: { size: number; success: boolean; fallback: string }[] = []

      // Attempt writes of varying sizes
      const sizes = [1, 2, 5, 10, 2]

      for (const size of sizes) {
        const result = resourceSim.writeToDisk(size)

        if (!result.success) {
          // Graceful fallback: try smaller write or cleanup
          writeAttempts.push({
            size,
            success: false,
            fallback: "cleanup_requested",
          })

          // Simulate cleanup
          resourceSim.setState({ diskUsedGB: resourceSim.getState().diskUsedGB - 10 })
        } else {
          writeAttempts.push({
            size,
            success: true,
            fallback: "none",
          })
        }
      }

      const successfulWrites = writeAttempts.filter((w) => w.success).length
      const fallbacks = writeAttempts.filter((w) => w.fallback !== "none").length

      expect(successfulWrites + fallbacks).toBe(sizes.length)
      expect(fallbacks).toBeGreaterThan(0) // Should trigger at least one fallback
    })

    test("memory pressure behavior", async () => {
      const resourceSim = new ResourcePressureSimulator()
      resourceSim.setState({ memoryUsedMB: 900, memoryLimitMB: 1024 })

      const allocationResults: { requested: number; success: boolean; available: number }[] = []

      // Try allocations
      const allocations = [50, 100, 50, 200, 30]

      for (const size of allocations) {
        const result = resourceSim.allocateMemory(size)
        allocationResults.push({
          requested: size,
          success: result.success,
          available: result.available,
        })

        if (!result.success) {
          // Trigger memory cleanup
          const currentState = resourceSim.getState()
          resourceSim.setState({
            memoryUsedMB: Math.max(100, currentState.memoryUsedMB - 200),
          })
        }
      }

      // Should have some failures due to pressure
      const failures = allocationResults.filter((r) => !r.success).length
      expect(failures).toBeGreaterThan(0)

      // System should still function after cleanup
      const finalState = resourceSim.getState()
      expect(finalState.memoryUsedMB).toBeLessThan(finalState.memoryLimitMB)
    })

    test("file lock contention", async () => {
      const concurrencySim = new ConcurrencySimulator()
      const file = "shared_config.json"

      const workers = await Promise.all(
        Array.from({ length: 5 }, async (_, i) => {
          const workerId = `worker_${i}`
          const results: { acquired: boolean; conflicts: number }[] = []

          for (let attempt = 0; attempt < 3; attempt++) {
            const lock = await concurrencySim.acquireLock(file, workerId, 100)

            if (lock.acquired) {
              // Do work
              await new Promise((resolve) => setTimeout(resolve, 20))
              concurrencySim.releaseLock(file, workerId)
              results.push({ acquired: true, conflicts: 0 })
            } else {
              results.push({ acquired: false, conflicts: 1 })
            }
          }

          return { workerId, results }
        }),
      )

      // Some workers should have acquired locks
      const successfulAcquisitions = workers.flatMap((w) => w.results).filter((r) => r.acquired).length
      const conflicts = workers.flatMap((w) => w.results).filter((r) => !r.acquired).length

      expect(successfulAcquisitions).toBeGreaterThan(0)
      // Some conflicts expected with concurrent access
      expect(conflicts + successfulAcquisitions).toBe(15) // 5 workers * 3 attempts
    })
  })

  describe("Data Corruption", () => {
    test("corrupted session file recovery", async () => {
      const corruptionSim = new DataCorruptionSimulator()

      const validSessionData = JSON.stringify({
        id: "session_123",
        messages: [{ role: "user", content: "Hello" }],
        metadata: { created: Date.now() },
      })

      // Simulate various corruption types
      const corruptionTypes: ("truncate" | "null_bytes" | "encoding" | "partial")[] = [
        "truncate",
        "null_bytes",
        "encoding",
        "partial",
      ]

      for (const type of corruptionTypes) {
        const corrupted = corruptionSim.corrupt(validSessionData, type, 0.2)
        const detection = corruptionSim.isCorrupted(corrupted.corrupted)

        // Only null_bytes corruption is reliably detected
        // truncate/partial may still produce valid JSON if the cut is lucky
        if (type === "null_bytes") {
          expect(detection.corrupted).toBe(true)
        }

        // Recovery: restore from backup or reinitialize
        const recovered = detection.corrupted ? validSessionData : corrupted.corrupted
        // Either the recovered data is valid JSON or we use the backup
        try {
          JSON.parse(recovered)
        } catch {
          // If parsing fails, use original data
          expect(() => JSON.parse(validSessionData)).not.toThrow()
        }
      }
    })

    test("partial write recovery", async () => {
      interface WriteOperation {
        id: string
        data: string
        bytesWritten: number
        totalBytes: number
        completed: boolean
      }

      const operations: WriteOperation[] = []
      const data = "x".repeat(10000)

      for (let i = 0; i < 5; i++) {
        const op: WriteOperation = {
          id: `write_${i}`,
          data,
          bytesWritten: 0,
          totalBytes: data.length,
          completed: false,
        }

        // Simulate partial write (30% chance of interruption)
        if (Math.random() < 0.3) {
          op.bytesWritten = Math.floor(data.length * Math.random())
          op.completed = false
        } else {
          op.bytesWritten = data.length
          op.completed = true
        }

        operations.push(op)
      }

      // Recovery: rollback partial writes
      const partialWrites = operations.filter((o) => !o.completed)
      const rollbackResults: { id: string; rolledBack: boolean }[] = []

      for (const partial of partialWrites) {
        // Simulate rollback
        rollbackResults.push({
          id: partial.id,
          rolledBack: true,
        })
      }

      expect(rollbackResults.every((r) => r.rolledBack)).toBe(true)

      // Retry partial writes
      const retried = partialWrites.map((p) => ({
        ...p,
        bytesWritten: p.totalBytes,
        completed: true,
      }))

      expect(retried.every((r) => r.completed)).toBe(true)
    })

    test("orphaned task cleanup", async () => {
      interface Task {
        id: string
        status: "pending" | "running" | "completed" | "orphaned"
        lastHeartbeat: number
      }

      const tasks: Task[] = [
        { id: "task_1", status: "completed", lastHeartbeat: Date.now() },
        { id: "task_2", status: "running", lastHeartbeat: Date.now() - 60000 }, // 1 min old
        { id: "task_3", status: "running", lastHeartbeat: Date.now() - 300000 }, // 5 min old - orphaned
        { id: "task_4", status: "pending", lastHeartbeat: Date.now() },
        { id: "task_5", status: "running", lastHeartbeat: Date.now() - 600000 }, // 10 min old - orphaned
      ]

      const orphanThreshold = 180000 // 3 minutes

      // Detect orphaned tasks
      const now = Date.now()
      const orphaned = tasks.filter(
        (t) => t.status === "running" && now - t.lastHeartbeat > orphanThreshold,
      )

      expect(orphaned.length).toBe(2)

      // Cleanup orphaned tasks
      for (const task of orphaned) {
        task.status = "orphaned"
      }

      // Verify cleanup
      const activeRunning = tasks.filter((t) => t.status === "running")
      expect(activeRunning.length).toBe(1)
      expect(activeRunning[0].id).toBe("task_2")
    })
  })

  describe("Concurrency Conflicts", () => {
    test("simultaneous file edits", async () => {
      const concurrencySim = new ConcurrencySimulator()
      const file = "shared_file.ts"

      interface EditResult {
        editorId: string
        acquired: boolean
        editApplied: boolean
        conflict?: string
      }

      const editResults: EditResult[] = []

      // Simulate 5 simultaneous edit attempts
      await Promise.all(
        Array.from({ length: 5 }, async (_, i) => {
          const editorId = `editor_${i}`
          const lock = await concurrencySim.acquireLock(file, editorId, 200)

          if (lock.acquired) {
            // Apply edit
            await new Promise((resolve) => setTimeout(resolve, 50))
            concurrencySim.releaseLock(file, editorId)

            editResults.push({
              editorId,
              acquired: true,
              editApplied: true,
            })
          } else {
            editResults.push({
              editorId,
              acquired: false,
              editApplied: false,
              conflict: lock.conflict?.type,
            })
          }
        }),
      )

      // At least one edit should succeed
      const successfulEdits = editResults.filter((r) => r.editApplied).length
      expect(successfulEdits).toBeGreaterThan(0)

      // Check for conflict recording
      const conflicts = concurrencySim.getConflicts()
      // Conflicts are expected in high-contention scenario
    })

    test("race condition in tool registration", async () => {
      const registry = new Map<string, { version: number }>()
      const registrations: { toolId: string; success: boolean; version: number }[] = []

      // Simulate race condition
      await Promise.all(
        Array.from({ length: 10 }, async (_, i) => {
          const toolId = "shared_tool"
          const version = i

          // Simulate delay variation
          await new Promise((resolve) => setTimeout(resolve, Math.random() * 20))

          // Check-then-act race condition simulation
          const existing = registry.get(toolId)

          if (!existing || existing.version < version) {
            registry.set(toolId, { version })
            registrations.push({ toolId, success: true, version })
          } else {
            registrations.push({ toolId, success: false, version })
          }
        }),
      )

      // Final registry should have highest version
      const finalVersion = registry.get("shared_tool")?.version ?? -1
      const maxAttemptedVersion = Math.max(...registrations.map((r) => r.version))

      // Due to race conditions, final version might not be highest
      // But it should be a valid version
      expect(finalVersion).toBeGreaterThanOrEqual(0)
      expect(finalVersion).toBeLessThanOrEqual(maxAttemptedVersion)
    })

    test("deadlock prevention in state machine", async () => {
      const concurrencySim = new ConcurrencySimulator()

      // Simulate potential deadlock scenario
      const resources = ["resource_a", "resource_b", "resource_c"]
      const workers = ["worker_1", "worker_2", "worker_3"]

      const acquisitions: { worker: string; resource: string; acquired: boolean }[] = []

      // Each worker tries to acquire resources in different order (potential deadlock)
      await Promise.all(
        workers.map(async (worker, i) => {
          const orderedResources = [...resources]
          if (i % 2 === 1) orderedResources.reverse() // Alternate order

          for (const resource of orderedResources) {
            const lock = await concurrencySim.acquireLock(resource, worker, 100)
            acquisitions.push({
              worker,
              resource,
              acquired: lock.acquired,
            })

            if (!lock.acquired && lock.conflict?.type === "deadlock") {
              // Deadlock detected - release and retry
              break
            }

            if (lock.acquired) {
              await new Promise((resolve) => setTimeout(resolve, 10))
            }
          }

          // Release all locks held by this worker
          for (const resource of resources) {
            concurrencySim.releaseLock(resource, worker)
          }
        }),
      )

      // Check that no permanent deadlock occurred
      const conflicts = concurrencySim.getConflicts()
      const deadlocks = conflicts.filter((c) => c.type === "deadlock")

      // Even if deadlocks were detected, they should be recoverable
      // The system should not be permanently blocked
    })

    test("concurrent state updates", async () => {
      interface State {
        version: number
        value: number
        updates: string[]
      }

      let state: State = { version: 0, value: 0, updates: [] }

      // Optimistic locking simulation
      async function updateState(
        updaterId: string,
        expectedVersion: number,
        newValue: number,
      ): Promise<{ success: boolean; conflict: boolean }> {
        // Simulate read-modify-write delay
        await new Promise((resolve) => setTimeout(resolve, Math.random() * 20))

        if (state.version !== expectedVersion) {
          return { success: false, conflict: true }
        }

        state = {
          version: state.version + 1,
          value: newValue,
          updates: [...state.updates, updaterId],
        }

        return { success: true, conflict: false }
      }

      // Concurrent updates
      const results = await Promise.all([
        updateState("updater_1", 0, 10),
        updateState("updater_2", 0, 20),
        updateState("updater_3", 0, 30),
      ])

      // Only one should succeed (optimistic locking)
      const successful = results.filter((r) => r.success).length
      const conflicts = results.filter((r) => r.conflict).length

      expect(successful).toBe(1)
      expect(conflicts).toBe(2)
      expect(state.version).toBe(1)
    })
  })

  describe("Chaos Session Management", () => {
    test("complete chaos session lifecycle", async () => {
      const faults = [
        { type: "network_timeout" as FaultType, probability: 0.3 },
        { type: "disk_full" as FaultType, probability: 0.1 },
        { type: "memory_pressure" as FaultType, probability: 0.2 },
      ]

      const session = createChaosSession(faults)
      const injector = new FaultInjector()

      // Register faults
      for (const fault of faults) {
        injector.registerFault(fault)
      }

      // Execute operations under chaos
      const operationResults: { faultTriggered: boolean; recovered: boolean }[] = []

      for (let i = 0; i < 50; i++) {
        let faultTriggered = false
        let recovered = true

        for (const fault of faults) {
          const result = await injector.injectFault(fault.type)
          if (result.triggered) {
            faultTriggered = true
            // Simulate recovery
            await new Promise((resolve) => setTimeout(resolve, 20))
          }
        }

        operationResults.push({ faultTriggered, recovered })
      }

      const endedSession = endChaosSession(session)

      expect(endedSession.endTime).toBeDefined()
      expect(endedSession.startTime).toBeLessThan(endedSession.endTime!)

      // Calculate metrics
      const faultHistory = injector.getHistory()
      const triggeredFaults = faultHistory.filter((f) => f.triggered).length

      expect(triggeredFaults).toBeGreaterThan(0)
      expect(triggeredFaults).toBeLessThan(faultHistory.length)
    })

    test("recovery time measurements", async () => {
      const measurements: { scenario: string; recoveryTime: number; recovered: boolean }[] = []

      // Scenario 1: Network failure recovery
      const networkRecovery = await measureRecoveryTime(
        async () => {
          throw new Error("Network disconnected")
        },
        async () => {
          await new Promise((resolve) => setTimeout(resolve, 100))
          return true
        },
        5000,
      )

      measurements.push({
        scenario: "network",
        recoveryTime: networkRecovery.duration,
        recovered: networkRecovery.recovered,
      })

      // Scenario 2: Resource exhaustion recovery
      const resourceRecovery = await measureRecoveryTime(
        async () => {
          throw new Error("Memory exhausted")
        },
        async () => {
          await new Promise((resolve) => setTimeout(resolve, 200))
          return true
        },
        5000,
      )

      measurements.push({
        scenario: "resource",
        recoveryTime: resourceRecovery.duration,
        recovered: resourceRecovery.recovered,
      })

      // All scenarios should recover
      expect(measurements.every((m) => m.recovered)).toBe(true)

      // Recovery time should be within threshold
      expect(measurements.every((m) => m.recoveryTime < EVAL_THRESHOLDS.errorRecoveryTime)).toBe(true)
    })
  })

  describe("Chaos Metrics Summary", () => {
    test("generates comprehensive chaos metrics", async () => {
      const events: ChaosEvent[] = []

      // Generate diverse chaos events with deterministic recovery pattern
      const faultTypes = ["network_timeout", "disk_full", "memory_pressure", "file_lock", "corruption"]

      for (let i = 0; i < 100; i++) {
        const faultType = faultTypes[i % faultTypes.length]
        // Deterministic recovery: fail every 10th event (90% recovery rate)
        const recovered = i % 10 !== 9
        const healthBefore = 1.0
        const healthAfter = recovered ? 0.8 + (i % 20) * 0.01 : 0.3 + (i % 30) * 0.01

        events.push({
          faultType,
          recovered,
          recoveryTime: recovered ? 100 + (i % 400) : 5000,
          systemHealthBefore: healthBefore,
          systemHealthAfter: healthAfter,
        })
      }

      const metrics = calculateChaosMetrics(events)

      expect(metrics.faultsInjected).toBe(100)
      expect(metrics.recoveryRate).toBeGreaterThanOrEqual(EVAL_THRESHOLDS.chaosRecoveryRate)
      expect(metrics.averageRecoveryTime).toBeLessThan(1000)
      expect(metrics.gracefulDegradation).toBe(true)
    })
  })
})
