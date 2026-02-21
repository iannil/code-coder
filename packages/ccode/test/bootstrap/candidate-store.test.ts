import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { CandidateStore } from "@/bootstrap/candidate-store"
import { BootstrapTypes } from "@/bootstrap/types"
import fs from "fs/promises"
import path from "path"
import os from "os"

describe("CandidateStore", () => {
  // Use temp directory for tests
  const originalHome = process.env.CCODE_TEST_HOME
  let testDir: string

  beforeEach(async () => {
    testDir = await fs.mkdtemp(path.join(os.tmpdir(), "ccode-test-"))
    process.env.CCODE_TEST_HOME = testDir
  })

  afterEach(async () => {
    process.env.CCODE_TEST_HOME = originalHome
    await fs.rm(testDir, { recursive: true, force: true }).catch(() => {})
  })

  describe("generateId", () => {
    test("generates unique IDs", () => {
      const id1 = CandidateStore.generateId()
      const id2 = CandidateStore.generateId()
      expect(id1).not.toBe(id2)
    })

    test("IDs have correct prefix", () => {
      const id = CandidateStore.generateId()
      expect(id.startsWith("cand_")).toBe(true)
    })
  })

  describe("create", () => {
    test("creates candidate with defaults", () => {
      const candidate = CandidateStore.create({
        type: "pattern",
        name: "test-skill",
        description: "A test skill",
        trigger: { type: "auto", context: "test" },
        content: { code: "const x = 1" },
        source: {
          sessionId: "session_123",
          toolCalls: ["tc_1"],
          problem: "problem",
          solution: "solution",
        },
      })

      expect(candidate.id).toMatch(/^cand_/)
      expect(candidate.type).toBe("pattern")
      expect(candidate.name).toBe("test-skill")
      expect(candidate.verification.status).toBe("pending")
      expect(candidate.verification.attempts).toBe(0)
      expect(candidate.verification.confidence).toBe(0)
      expect(candidate.metadata.usageCount).toBe(0)
    })

    test("sets timestamps", () => {
      const before = Date.now()
      const candidate = CandidateStore.create({
        type: "workflow",
        name: "test",
        description: "test",
        trigger: { type: "manual", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      const after = Date.now()

      expect(candidate.metadata.created).toBeGreaterThanOrEqual(before)
      expect(candidate.metadata.created).toBeLessThanOrEqual(after)
      expect(candidate.metadata.updated).toBe(candidate.metadata.created)
    })
  })

  describe("CRUD operations", () => {
    // Counter to ensure unique names even within the same millisecond
    let counter = 0
    const createTestCandidate = () =>
      CandidateStore.create({
        type: "pattern",
        name: `test-${Date.now()}-${counter++}`,
        description: "test",
        trigger: { type: "auto", context: "test" },
        content: { code: "x" },
        source: { sessionId: "s", toolCalls: [], problem: "p", solution: "s" },
      })

    test("add and get candidate", async () => {
      const candidate = createTestCandidate()
      await CandidateStore.add(candidate)

      const retrieved = await CandidateStore.get(candidate.id)
      expect(retrieved).toBeDefined()
      expect(retrieved?.id).toBe(candidate.id)
      expect(retrieved?.name).toBe(candidate.name)
    })

    test("getByName returns candidate", async () => {
      const candidate = createTestCandidate()
      await CandidateStore.add(candidate)

      const retrieved = await CandidateStore.getByName(candidate.name)
      expect(retrieved?.id).toBe(candidate.id)
    })

    test("getByName returns undefined for missing", async () => {
      const retrieved = await CandidateStore.getByName("nonexistent")
      expect(retrieved).toBeUndefined()
    })

    test("update modifies candidate", async () => {
      const candidate = createTestCandidate()
      await CandidateStore.add(candidate)

      await CandidateStore.update(candidate.id, (c) => {
        c.verification.status = "passed"
        c.verification.confidence = 0.8
      })

      const retrieved = await CandidateStore.get(candidate.id)
      expect(retrieved?.verification.status).toBe("passed")
      expect(retrieved?.verification.confidence).toBe(0.8)
    })

    test("update sets updated timestamp", async () => {
      const candidate = createTestCandidate()
      await CandidateStore.add(candidate)

      const originalUpdated = candidate.metadata.updated
      await new Promise((r) => setTimeout(r, 10)) // Small delay

      await CandidateStore.update(candidate.id, (c) => {
        c.metadata.usageCount++
      })

      const retrieved = await CandidateStore.get(candidate.id)
      expect(retrieved?.metadata.updated).toBeGreaterThan(originalUpdated)
    })

    test("remove deletes candidate", async () => {
      const candidate = createTestCandidate()
      await CandidateStore.add(candidate)

      const removed = await CandidateStore.remove(candidate.id)
      expect(removed).toBe(true)

      const retrieved = await CandidateStore.get(candidate.id)
      expect(retrieved).toBeUndefined()
    })

    test("remove returns false for nonexistent", async () => {
      const removed = await CandidateStore.remove("nonexistent")
      expect(removed).toBe(false)
    })

    test("list returns all candidates", async () => {
      const c1 = createTestCandidate()
      const c2 = createTestCandidate()
      await CandidateStore.add(c1)
      await CandidateStore.add(c2)

      const all = await CandidateStore.list()
      expect(all.length).toBeGreaterThanOrEqual(2)
      expect(all.some((c) => c.id === c1.id)).toBe(true)
      expect(all.some((c) => c.id === c2.id)).toBe(true)
    })
  })

  describe("listByStatus", () => {
    test("filters by verification status", async () => {
      const pending = CandidateStore.create({
        type: "pattern",
        name: "pending-skill",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      await CandidateStore.add(pending)

      const passed = CandidateStore.create({
        type: "pattern",
        name: "passed-skill",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      passed.verification.status = "passed"
      await CandidateStore.add(passed)

      const pendingList = await CandidateStore.listByStatus("pending")
      const passedList = await CandidateStore.listByStatus("passed")

      expect(pendingList.some((c) => c.id === pending.id)).toBe(true)
      expect(passedList.some((c) => c.id === passed.id)).toBe(true)
      expect(passedList.some((c) => c.id === pending.id)).toBe(false)
    })
  })

  describe("listByConfidence", () => {
    test("filters by minimum confidence", async () => {
      const low = CandidateStore.create({
        type: "pattern",
        name: "low-conf",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      low.verification.confidence = 0.3
      await CandidateStore.add(low)

      const high = CandidateStore.create({
        type: "pattern",
        name: "high-conf",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      high.verification.confidence = 0.8
      await CandidateStore.add(high)

      const highConfList = await CandidateStore.listByConfidence(0.6)
      expect(highConfList.some((c) => c.id === high.id)).toBe(true)
      expect(highConfList.some((c) => c.id === low.id)).toBe(false)
    })
  })

  describe("listReadyForPromotion", () => {
    test("returns passed candidates with high confidence", async () => {
      const ready = CandidateStore.create({
        type: "pattern",
        name: "ready-skill",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      ready.verification.status = "passed"
      ready.verification.confidence = 0.7
      await CandidateStore.add(ready)

      const notReady = CandidateStore.create({
        type: "pattern",
        name: "not-ready",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      notReady.verification.status = "pending"
      notReady.verification.confidence = 0.7
      await CandidateStore.add(notReady)

      const readyList = await CandidateStore.listReadyForPromotion()
      expect(readyList.some((c) => c.id === ready.id)).toBe(true)
      expect(readyList.some((c) => c.id === notReady.id)).toBe(false)
    })
  })

  describe("cleanup", () => {
    test("removes low confidence candidates after threshold", async () => {
      const lowConf = CandidateStore.create({
        type: "pattern",
        name: "low-conf-old",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      lowConf.verification.confidence = 0.1
      // Set created time to 40 days ago
      lowConf.metadata.created = Date.now() - 40 * 24 * 60 * 60 * 1000
      await CandidateStore.add(lowConf)

      const removed = await CandidateStore.cleanup({
        maxAge: 30 * 24 * 60 * 60 * 1000,
        minConfidence: 0.2,
      })

      expect(removed).toBeGreaterThanOrEqual(1)
      const retrieved = await CandidateStore.get(lowConf.id)
      expect(retrieved).toBeUndefined()
    })

    test("keeps recent candidates even with low confidence", async () => {
      const recent = CandidateStore.create({
        type: "pattern",
        name: "recent-low-conf",
        description: "test",
        trigger: { type: "auto", context: "" },
        content: {},
        source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
      })
      recent.verification.confidence = 0.1
      await CandidateStore.add(recent)

      await CandidateStore.cleanup({
        maxAge: 30 * 24 * 60 * 60 * 1000,
        minConfidence: 0.2,
      })

      const retrieved = await CandidateStore.get(recent.id)
      expect(retrieved).toBeDefined()
    })

    test("enforces max candidates limit", async () => {
      // Add more candidates than limit
      for (let i = 0; i < 5; i++) {
        const c = CandidateStore.create({
          type: "pattern",
          name: `test-${i}`,
          description: "test",
          trigger: { type: "auto", context: "" },
          content: {},
          source: { sessionId: "s", toolCalls: [], problem: "", solution: "" },
        })
        c.verification.confidence = i * 0.1
        await CandidateStore.add(c)
      }

      await CandidateStore.cleanup({ maxCandidates: 3 })

      const remaining = await CandidateStore.list()
      expect(remaining.length).toBeLessThanOrEqual(3)
    })
  })

  describe("duplicate handling", () => {
    test("updates existing candidate with same name", async () => {
      const original = CandidateStore.create({
        type: "pattern",
        name: "duplicate-test",
        description: "original",
        trigger: { type: "auto", context: "" },
        content: { code: "original" },
        source: { sessionId: "s1", toolCalls: [], problem: "", solution: "" },
      })
      await CandidateStore.add(original)

      const duplicate = CandidateStore.create({
        type: "workflow",
        name: "duplicate-test",
        description: "updated",
        trigger: { type: "manual", context: "" },
        content: { steps: ["step1"] },
        source: { sessionId: "s2", toolCalls: [], problem: "", solution: "" },
      })
      await CandidateStore.add(duplicate)

      const all = await CandidateStore.list()
      const matches = all.filter((c) => c.name === "duplicate-test")
      expect(matches.length).toBe(1)
      expect(matches[0].description).toBe("updated")
      expect(matches[0].type).toBe("workflow")
      // Original created time should be preserved
      expect(matches[0].metadata.created).toBe(original.metadata.created)
    })
  })
})
