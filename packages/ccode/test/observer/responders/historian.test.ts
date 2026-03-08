/**
 * Historian Responder Tests
 *
 * Tests for the Historian component of the Observer Network.
 *
 * @module test/observer/responders/historian.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, mock } from "bun:test"
import {
  Historian,
  createHistorian,
  type HistorianConfig,
  type HistoryEntry,
  type HistoryEventType,
  type HistoryQuery,
} from "@/observer/responders/historian"

// Mock memory-client to avoid actual file operations
mock.module("@/observer/integration/memory-client", () => ({
  getMemoryClient: () => ({
    recordBatch: async () => {},
    getPath: () => "/mock/memory/path",
  }),
}))

describe("Historian", () => {
  let historian: Historian

  beforeEach(() => {
    historian = createHistorian({
      autoRecord: false, // Disable auto recording for manual testing
      maxInMemory: 100,
      flushIntervalMs: 60000, // 1 minute
      sessionId: "test-session",
    })
  })

  afterEach(async () => {
    await historian.stop()
    historian.clear()
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", async () => {
      await historian.start()
      // Historian should be running
      await historian.stop()
      // Should be able to stop without error
    })

    it("should not start twice", async () => {
      await historian.start()
      await historian.start() // Should not throw
      await historian.stop()
    })

    it("should be safe to stop when not started", async () => {
      await historian.stop() // Should not throw
    })

    it("should flush on stop", async () => {
      await historian.start()

      await historian.record("observation", { test: "data" }, ["test"])

      // Stop should trigger flush
      await historian.stop()

      // No error should occur
    })
  })

  describe("record", () => {
    it("should record event with correct fields", async () => {
      await historian.start()

      const entry = await historian.record("observation", { file: "test.ts" }, ["code"])

      expect(entry).toBeDefined()
      expect(entry.id).toMatch(/^hist_/)
      expect(entry.type).toBe("observation")
      expect(entry.data).toEqual({ file: "test.ts" })
      expect(entry.tags).toContain("code")
      expect(entry.timestamp).toBeInstanceOf(Date)
      expect(entry.sessionId).toBe("test-session")
    })

    it("should record different event types", async () => {
      await historian.start()

      const types: HistoryEventType[] = [
        "observation",
        "pattern",
        "anomaly",
        "opportunity",
        "world_model",
        "mode_decision",
        "escalation",
        "execution",
      ]

      for (const type of types) {
        const entry = await historian.record(type, { type }, [type])
        expect(entry.type).toBe(type)
      }
    })

    it("should record with multiple tags", async () => {
      await historian.start()

      const entry = await historian.record("anomaly", { error: "test" }, [
        "error",
        "critical",
        "production",
      ])

      expect(entry.tags).toContain("error")
      expect(entry.tags).toContain("critical")
      expect(entry.tags).toContain("production")
    })

    it("should record without tags", async () => {
      await historian.start()

      const entry = await historian.record("observation", { data: "test" })

      expect(entry.tags).toEqual([])
    })
  })

  describe("query", () => {
    it("should query all entries when no filter provided", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 }, ["a"])
      await historian.record("anomaly", { n: 2 }, ["b"])
      await historian.record("pattern", { n: 3 }, ["c"])

      const results = historian.query()

      expect(results.length).toBe(3)
    })

    it("should filter by single type", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })
      await historian.record("anomaly", { n: 2 })
      await historian.record("observation", { n: 3 })

      const results = historian.query({ type: "observation" })

      expect(results.length).toBe(2)
      for (const r of results) {
        expect(r.type).toBe("observation")
      }
    })

    it("should filter by multiple types", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })
      await historian.record("anomaly", { n: 2 })
      await historian.record("pattern", { n: 3 })

      const results = historian.query({ type: ["observation", "anomaly"] })

      expect(results.length).toBe(2)
      for (const r of results) {
        expect(["observation", "anomaly"]).toContain(r.type)
      }
    })

    it("should filter by time range", async () => {
      await historian.start()

      const now = new Date()
      const past = new Date(now.getTime() - 10000)
      const future = new Date(now.getTime() + 10000)

      await historian.record("observation", { n: 1 })

      // Query with start time in the past
      const results1 = historian.query({ startTime: past })
      expect(results1.length).toBe(1)

      // Query with start time in the future
      const results2 = historian.query({ startTime: future })
      expect(results2.length).toBe(0)
    })

    it("should filter by tags", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 }, ["tag1"])
      await historian.record("observation", { n: 2 }, ["tag2"])
      await historian.record("observation", { n: 3 }, ["tag1", "tag3"])

      const results = historian.query({ tags: ["tag1"] })

      expect(results.length).toBe(2)
      for (const r of results) {
        expect(r.tags).toContain("tag1")
      }
    })

    it("should filter by session id", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })

      // Query with matching session
      const results1 = historian.query({ sessionId: "test-session" })
      expect(results1.length).toBe(1)

      // Query with non-matching session
      const results2 = historian.query({ sessionId: "other-session" })
      expect(results2.length).toBe(0)
    })

    it("should apply limit", async () => {
      await historian.start()

      for (let i = 0; i < 10; i++) {
        await historian.record("observation", { n: i })
      }

      const results = historian.query({ limit: 5 })

      expect(results.length).toBe(5)
    })

    it("should apply offset", async () => {
      await historian.start()

      for (let i = 0; i < 10; i++) {
        await historian.record("observation", { n: i })
      }

      const allResults = historian.query()
      const offsetResults = historian.query({ offset: 3, limit: 100 })

      expect(offsetResults.length).toBe(7)
      expect(offsetResults[0].id).toBe(allResults[3].id)
    })

    it("should return results sorted by timestamp descending", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })
      await new Promise((r) => setTimeout(r, 10))
      await historian.record("observation", { n: 2 })
      await new Promise((r) => setTimeout(r, 10))
      await historian.record("observation", { n: 3 })

      const results = historian.query()

      expect(results[0].timestamp.getTime()).toBeGreaterThanOrEqual(results[1].timestamp.getTime())
      expect(results[1].timestamp.getTime()).toBeGreaterThanOrEqual(results[2].timestamp.getTime())
    })
  })

  describe("get", () => {
    it("should retrieve entry by id", async () => {
      await historian.start()

      const entry = await historian.record("observation", { test: "data" })

      const retrieved = historian.get(entry.id)

      expect(retrieved).toBeDefined()
      expect(retrieved?.id).toBe(entry.id)
      expect(retrieved?.data).toEqual({ test: "data" })
    })

    it("should return null for non-existent id", () => {
      const retrieved = historian.get("non_existent_id")
      expect(retrieved).toBeNull()
    })
  })

  describe("getStats", () => {
    it("should return correct statistics", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })
      await historian.record("observation", { n: 2 })
      await historian.record("anomaly", { n: 3 })
      await historian.record("pattern", { n: 4 })

      const stats = historian.getStats()

      expect(stats.totalEntries).toBe(4)
      expect(stats.byType.observation).toBe(2)
      expect(stats.byType.anomaly).toBe(1)
      expect(stats.byType.pattern).toBe(1)
      expect(stats.sessionCount).toBe(1)
      expect(stats.oldestEntry).toBeInstanceOf(Date)
      expect(stats.newestEntry).toBeInstanceOf(Date)
    })

    it("should return zero stats when empty", () => {
      const stats = historian.getStats()

      expect(stats.totalEntries).toBe(0)
      expect(stats.byType.observation).toBe(0)
      expect(stats.sessionCount).toBe(0)
      expect(stats.oldestEntry).toBeUndefined()
      expect(stats.newestEntry).toBeUndefined()
    })
  })

  describe("getRecent", () => {
    it("should return recent entries of a type", async () => {
      await historian.start()

      for (let i = 0; i < 5; i++) {
        await historian.record("observation", { n: i })
      }
      await historian.record("anomaly", { n: 99 })

      const recent = historian.getRecent("observation", 3)

      expect(recent.length).toBe(3)
      for (const r of recent) {
        expect(r.type).toBe("observation")
      }
    })
  })

  describe("search", () => {
    it("should search entries by predicate", async () => {
      await historian.start()

      await historian.record("observation", { value: 10 })
      await historian.record("observation", { value: 20 })
      await historian.record("observation", { value: 30 })

      const results = historian.search((entry) => {
        const data = entry.data as { value: number }
        return data.value > 15
      })

      expect(results.length).toBe(2)
    })

    it("should limit search results", async () => {
      await historian.start()

      for (let i = 0; i < 10; i++) {
        await historian.record("observation", { value: i })
      }

      const results = historian.search(() => true, 5)

      expect(results.length).toBe(5)
    })
  })

  describe("clear", () => {
    it("should clear all history", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })
      await historian.record("anomaly", { n: 2 })

      historian.clear()

      expect(historian.query().length).toBe(0)
      expect(historian.getStats().totalEntries).toBe(0)
    })
  })

  describe("export", () => {
    it("should export history as JSON", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 }, ["tag1"])
      await historian.record("anomaly", { n: 2 }, ["tag2"])

      const exported = historian.export()
      const parsed = JSON.parse(exported)

      expect(parsed).toBeInstanceOf(Array)
      expect(parsed.length).toBe(2)
    })

    it("should export with query filter", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })
      await historian.record("anomaly", { n: 2 })

      const exported = historian.export({ type: "observation" })
      const parsed = JSON.parse(exported)

      expect(parsed.length).toBe(1)
      expect(parsed[0].type).toBe("observation")
    })
  })

  describe("import", () => {
    it("should import history from JSON", async () => {
      await historian.start()

      const entries: HistoryEntry[] = [
        {
          id: "hist_import_1",
          type: "observation",
          timestamp: new Date(),
          data: { imported: true },
          tags: ["imported"],
        },
        {
          id: "hist_import_2",
          type: "anomaly",
          timestamp: new Date(),
          data: { imported: true },
          tags: ["imported"],
        },
      ]

      const imported = historian.import(JSON.stringify(entries))

      expect(imported).toBe(2)
      expect(historian.getStats().totalEntries).toBe(2)
    })

    it("should not import duplicates", async () => {
      await historian.start()

      const entry = await historian.record("observation", { n: 1 })

      const entries: HistoryEntry[] = [
        entry, // Duplicate
        {
          id: "hist_new",
          type: "anomaly",
          timestamp: new Date(),
          data: { new: true },
          tags: [],
        },
      ]

      const imported = historian.import(JSON.stringify(entries))

      expect(imported).toBe(1) // Only the new one
    })
  })

  describe("max in-memory limit", () => {
    it("should evict oldest entries when limit exceeded", async () => {
      const limitedHistorian = createHistorian({
        autoRecord: false,
        maxInMemory: 5,
        flushIntervalMs: 60000,
      })
      await limitedHistorian.start()

      // Record more than the limit
      for (let i = 0; i < 10; i++) {
        await limitedHistorian.record("observation", { n: i })
        // Small delay to ensure ordering
        await new Promise((r) => setTimeout(r, 5))
      }

      const stats = limitedHistorian.getStats()

      // Should have evicted to stay at limit
      expect(stats.totalEntries).toBeLessThanOrEqual(5)

      await limitedHistorian.stop()
    })
  })

  describe("flush", () => {
    it("should flush pending entries", async () => {
      await historian.start()

      await historian.record("observation", { n: 1 })
      await historian.record("observation", { n: 2 })

      // Manual flush
      await historian.flush()

      // No error should occur
    })

    it("should not error when nothing to flush", async () => {
      await historian.start()
      await historian.flush() // No entries to flush
    })
  })

  describe("auto-record mode", () => {
    it("should enable auto-recording when configured", async () => {
      const autoHistorian = createHistorian({
        autoRecord: true,
        maxInMemory: 100,
        flushIntervalMs: 60000,
      })

      await autoHistorian.start()
      // Auto-recording is enabled - event subscriptions are set up

      await autoHistorian.stop()
    })
  })
})
