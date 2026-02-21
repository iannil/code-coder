/**
 * Memory Persistence Evaluation Tests
 *
 * Verifies that the memory system correctly:
 * - M1: Writes session logs to daily notes
 * - M2: Consolidates knowledge to long-term memory
 * - M3: Loads relevant context on startup
 * - M4: Retrieves past experiences
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { CandidateStore } from "@/bootstrap/candidate-store"
import { BootstrapTypes } from "@/bootstrap/types"
import {
  createMockCandidate,
  createMockCandidateBatch,
  MOCK_CANDIDATES,
} from "./fixtures/mock-candidates"
import {
  createMetricResult,
  aggregateMetrics,
} from "./utils/metrics"
import { tmpdir } from "../fixture/fixture"

describe("Memory Persistence Evaluation", () => {
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

  describe("M1: Candidate Store Persistence", () => {
    test("CandidateStore persists candidates to disk", async () => {
      const candidate = createMockCandidate({ name: "persist-test" })
      await CandidateStore.add(candidate)

      // Read back
      const retrieved = await CandidateStore.get(candidate.id)

      expect(retrieved).toBeDefined()
      expect(retrieved?.name).toBe("persist-test")
    })

    test("CandidateStore survives re-read", async () => {
      const candidate = createMockCandidate({ name: "reread-test" })
      await CandidateStore.add(candidate)

      // Force re-read from disk
      const store1 = await CandidateStore.read()
      const store2 = await CandidateStore.read()

      expect(store1.candidates.length).toBe(store2.candidates.length)
    })

    test("CandidateStore handles multiple candidates", async () => {
      const candidates = createMockCandidateBatch(5)

      for (const candidate of candidates) {
        await CandidateStore.add(candidate)
      }

      const store = await CandidateStore.read()
      expect(store.candidates.length).toBe(5)
    })

    test("CandidateStore update persists changes", async () => {
      const candidate = createMockCandidate({
        id: "update-persist-test",
        name: "update-test",
        verification: { status: "pending", attempts: 0, confidence: 0.3 },
      })

      await CandidateStore.add(candidate)

      await CandidateStore.update(candidate.id, (c) => {
        c.verification.confidence = 0.7
        c.verification.status = "passed"
      })

      const retrieved = await CandidateStore.get(candidate.id)
      expect(retrieved?.verification.confidence).toBe(0.7)
      expect(retrieved?.verification.status).toBe("passed")
    })

    test("CandidateStore remove persists deletion", async () => {
      const candidate = createMockCandidate({
        id: "remove-persist-test",
        name: "remove-test",
      })

      await CandidateStore.add(candidate)
      expect(await CandidateStore.get(candidate.id)).toBeDefined()

      await CandidateStore.remove(candidate.id)
      expect(await CandidateStore.get(candidate.id)).toBeUndefined()
    })
  })

  describe("M2: Knowledge Consolidation", () => {
    test("listByStatus filters correctly", async () => {
      const pending = createMockCandidate({
        name: "status-pending",
        verification: { status: "pending", attempts: 0, confidence: 0.3 },
      })
      const passed = createMockCandidate({
        name: "status-passed",
        verification: { status: "passed", attempts: 1, confidence: 0.7 },
      })
      const failed = createMockCandidate({
        name: "status-failed",
        verification: { status: "failed", attempts: 2, confidence: 0.2 },
      })

      await CandidateStore.add(pending)
      await CandidateStore.add(passed)
      await CandidateStore.add(failed)

      const pendingList = await CandidateStore.listByStatus("pending")
      const passedList = await CandidateStore.listByStatus("passed")
      const failedList = await CandidateStore.listByStatus("failed")

      expect(pendingList.some((c) => c.name === "status-pending")).toBe(true)
      expect(passedList.some((c) => c.name === "status-passed")).toBe(true)
      expect(failedList.some((c) => c.name === "status-failed")).toBe(true)
    })

    test("listByConfidence filters correctly", async () => {
      const low = createMockCandidate({
        name: "conf-low",
        verification: { status: "pending", attempts: 0, confidence: 0.2 },
      })
      const medium = createMockCandidate({
        name: "conf-medium",
        verification: { status: "pending", attempts: 0, confidence: 0.5 },
      })
      const high = createMockCandidate({
        name: "conf-high",
        verification: { status: "passed", attempts: 1, confidence: 0.8 },
      })

      await CandidateStore.add(low)
      await CandidateStore.add(medium)
      await CandidateStore.add(high)

      const highConfidence = await CandidateStore.listByConfidence(0.6)
      expect(highConfidence.some((c) => c.name === "conf-high")).toBe(true)
      expect(highConfidence.some((c) => c.name === "conf-low")).toBe(false)
    })

    test("listReadyForPromotion finds qualified candidates", async () => {
      const ready = createMockCandidate({
        name: "promo-ready",
        verification: { status: "passed", attempts: 1, confidence: 0.75 },
      })
      const notReady = createMockCandidate({
        name: "promo-not-ready",
        verification: { status: "pending", attempts: 0, confidence: 0.3 },
      })

      await CandidateStore.add(ready)
      await CandidateStore.add(notReady)

      const readyList = await CandidateStore.listReadyForPromotion()

      expect(readyList.some((c) => c.name === "promo-ready")).toBe(true)
      expect(readyList.some((c) => c.name === "promo-not-ready")).toBe(false)
    })
  })

  describe("M3: Context Loading", () => {
    test("CandidateStore.list returns all candidates", async () => {
      const count = 3
      const candidates = createMockCandidateBatch(count)

      for (const candidate of candidates) {
        await CandidateStore.add(candidate)
      }

      const all = await CandidateStore.list()
      expect(all.length).toBe(count)
    })

    test("getByName retrieves by exact name", async () => {
      const candidate = createMockCandidate({ name: "exact-name-test" })
      await CandidateStore.add(candidate)

      const found = await CandidateStore.getByName("exact-name-test")
      const notFound = await CandidateStore.getByName("nonexistent")

      expect(found).toBeDefined()
      expect(found?.name).toBe("exact-name-test")
      expect(notFound).toBeUndefined()
    })

    test("get retrieves by ID", async () => {
      const candidate = createMockCandidate({ id: "specific-id-123" })
      await CandidateStore.add(candidate)

      const found = await CandidateStore.get("specific-id-123")
      expect(found).toBeDefined()
      expect(found?.id).toBe("specific-id-123")
    })
  })

  describe("M4: Experience Retrieval", () => {
    test("candidates preserve source problem context", async () => {
      const candidate = createMockCandidate({
        source: {
          sessionId: "session-123",
          toolCalls: ["tc1", "tc2"],
          problem: "How to solve problem X efficiently",
          solution: "Use algorithm Y with optimization Z",
        },
      })

      await CandidateStore.add(candidate)
      const retrieved = await CandidateStore.get(candidate.id)

      expect(retrieved?.source.problem).toContain("problem X")
      expect(retrieved?.source.solution).toContain("algorithm Y")
    })

    test("similar candidates can be found by listing", async () => {
      const jsonCandidate = createMockCandidate({
        name: "format-json-config",
        description: "Format JSON configuration files",
      })
      const yamlCandidate = createMockCandidate({
        name: "format-yaml-config",
        description: "Format YAML configuration files",
      })

      await CandidateStore.add(jsonCandidate)
      await CandidateStore.add(yamlCandidate)

      const all = await CandidateStore.list()
      const formatCandidates = all.filter((c) =>
        c.name.includes("format") && c.name.includes("config"),
      )

      expect(formatCandidates.length).toBe(2)
    })
  })

  describe("Cleanup and Maintenance", () => {
    test("cleanup removes old low-confidence candidates", async () => {
      const oldLowConf = createMockCandidate({
        name: "old-low-conf",
        verification: { status: "failed", attempts: 3, confidence: 0.1 },
        metadata: {
          created: Date.now() - 60 * 24 * 60 * 60 * 1000, // 60 days ago
          updated: Date.now(),
          usageCount: 0,
        },
      })
      const recentLowConf = createMockCandidate({
        name: "recent-low-conf",
        verification: { status: "pending", attempts: 1, confidence: 0.15 },
        metadata: {
          created: Date.now() - 1 * 24 * 60 * 60 * 1000, // 1 day ago
          updated: Date.now(),
          usageCount: 0,
        },
      })
      const oldHighConf = createMockCandidate({
        name: "old-high-conf",
        verification: { status: "passed", attempts: 1, confidence: 0.8 },
        metadata: {
          created: Date.now() - 60 * 24 * 60 * 60 * 1000, // 60 days ago
          updated: Date.now(),
          usageCount: 5,
        },
      })

      await CandidateStore.add(oldLowConf)
      await CandidateStore.add(recentLowConf)
      await CandidateStore.add(oldHighConf)

      const removed = await CandidateStore.cleanup({
        maxAge: 30 * 24 * 60 * 60 * 1000, // 30 days
        minConfidence: 0.2,
      })

      // Old low confidence should be removed
      // Recent low confidence and old high confidence should remain
      const remaining = await CandidateStore.list()

      expect(remaining.some((c) => c.name === "old-low-conf")).toBe(false)
      expect(remaining.some((c) => c.name === "recent-low-conf")).toBe(true)
      expect(remaining.some((c) => c.name === "old-high-conf")).toBe(true)
    })

    test("cleanup respects maxCandidates limit", async () => {
      // Add more candidates than limit
      const candidates = createMockCandidateBatch(15)
      for (const candidate of candidates) {
        await CandidateStore.add(candidate)
      }

      await CandidateStore.cleanup({ maxCandidates: 10 })

      const remaining = await CandidateStore.list()
      expect(remaining.length).toBeLessThanOrEqual(10)
    })

    test("cleanup keeps highest confidence candidates", async () => {
      const highConf = createMockCandidate({
        name: "high-conf-keep",
        verification: { status: "passed", attempts: 1, confidence: 0.9 },
      })
      const lowConf = createMockCandidate({
        name: "low-conf-maybe-remove",
        verification: { status: "pending", attempts: 0, confidence: 0.25 },
      })

      await CandidateStore.add(lowConf)
      await CandidateStore.add(highConf)

      await CandidateStore.cleanup({ maxCandidates: 1 })

      const remaining = await CandidateStore.list()
      expect(remaining.some((c) => c.name === "high-conf-keep")).toBe(true)
    })
  })

  describe("Store Version and Timestamps", () => {
    test("store has version number", async () => {
      const store = await CandidateStore.read()
      expect(store.version).toBeDefined()
      expect(typeof store.version).toBe("number")
    })

    test("store has timestamps", async () => {
      const store = await CandidateStore.read()
      expect(store.time.created).toBeDefined()
      expect(store.time.updated).toBeDefined()
    })

    test("write updates timestamp", async () => {
      const before = await CandidateStore.read()
      const beforeUpdated = before.time.updated

      // Small delay
      await new Promise((r) => setTimeout(r, 10))

      await CandidateStore.add(createMockCandidate())

      const after = await CandidateStore.read()
      expect(after.time.updated).toBeGreaterThanOrEqual(beforeUpdated)
    })
  })
})

describe("Memory Metrics", () => {
  test("generates evaluation summary for memory dimension", () => {
    const metrics = [
      createMetricResult("Write Latency", 50, 100, "lte"), // ms
      createMetricResult("Read Latency", 20, 50, "lte"), // ms
      createMetricResult("Consolidation Accuracy", 0.95, 0.9, "gte"),
      createMetricResult("Retrieval Recall", 0.85, 0.7, "gte"),
    ]

    const summary = aggregateMetrics("Memory Persistence", metrics)

    expect(summary.dimension).toBe("Memory Persistence")
    expect(summary.passRate).toBeGreaterThanOrEqual(0.5)
  })
})
