/**
 * Memory Client Tests
 *
 * Tests for the Observer Network memory integration client.
 *
 * @module test/observer/integration/memory-client.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import { rm } from "fs/promises"
import path from "path"
import { MemoryClient, createMemoryClient, resetMemoryClient } from "@/observer/integration/memory-client"
import type { ObserverHistoryEntry } from "@/observer/integration/memory-client"

describe("MemoryClient", () => {
  const testBasePath = path.join(import.meta.dir, ".test-observer-memory")
  let client: MemoryClient

  beforeEach(async () => {
    resetMemoryClient()
    client = createMemoryClient({
      basePath: testBasePath,
      enableDailyNotes: false, // Disable to avoid affecting real daily notes
      enableFileStorage: true,
    })
  })

  afterEach(async () => {
    // Clean up test directory
    try {
      await rm(testBasePath, { recursive: true, force: true })
    } catch {
      // Directory may not exist
    }
  })

  describe("initialize", () => {
    it("should initialize successfully", async () => {
      await client.initialize()
      // Verify directory was created
      const file = Bun.file(path.join(testBasePath, "events", ".keep"))
      expect(file.size).toBe(0)
    })
  })

  describe("recordObservation", () => {
    it("should record a single observation", async () => {
      const entry: ObserverHistoryEntry = {
        id: "obs_test_1",
        type: "observation",
        timestamp: new Date(),
        data: { description: "Test observation" },
        tags: ["test"],
      }

      await client.recordObservation(entry)

      // Query should find it
      const results = await client.query({ limit: 10 })
      expect(results.length).toBe(1)
      expect(results[0].id).toBe("obs_test_1")
      expect(results[0].type).toBe("observation")
    })

    it("should record observation with sessionId", async () => {
      const entry: ObserverHistoryEntry = {
        id: "obs_test_2",
        type: "pattern",
        timestamp: new Date(),
        data: { description: "Pattern detected" },
        tags: ["pattern"],
        sessionId: "session_123",
      }

      await client.recordObservation(entry)

      const results = await client.query({ type: "pattern" })
      expect(results.length).toBe(1)
      expect(results[0].sessionId).toBe("session_123")
    })
  })

  describe("recordBatch", () => {
    it("should record multiple observations in batch", async () => {
      const entries: ObserverHistoryEntry[] = [
        {
          id: "batch_1",
          type: "observation",
          timestamp: new Date(),
          data: { index: 1 },
          tags: [],
        },
        {
          id: "batch_2",
          type: "anomaly",
          timestamp: new Date(),
          data: { index: 2 },
          tags: [],
        },
        {
          id: "batch_3",
          type: "pattern",
          timestamp: new Date(),
          data: { index: 3 },
          tags: [],
        },
      ]

      await client.recordBatch(entries)

      const results = await client.query()
      expect(results.length).toBe(3)
    })
  })

  describe("query", () => {
    beforeEach(async () => {
      // Populate with test data
      const entries: ObserverHistoryEntry[] = [
        {
          id: "q_1",
          type: "observation",
          timestamp: new Date("2026-03-08T10:00:00Z"),
          data: {},
          tags: ["code"],
        },
        {
          id: "q_2",
          type: "pattern",
          timestamp: new Date("2026-03-08T11:00:00Z"),
          data: {},
          tags: ["world"],
        },
        {
          id: "q_3",
          type: "anomaly",
          timestamp: new Date("2026-03-08T12:00:00Z"),
          data: {},
          tags: ["self"],
        },
      ]

      await client.recordBatch(entries)
    })

    it("should query by type", async () => {
      const results = await client.query({ type: "pattern" })
      expect(results.length).toBe(1)
      expect(results[0].id).toBe("q_2")
    })

    it("should query by multiple types", async () => {
      const results = await client.query({ type: ["observation", "anomaly"] })
      expect(results.length).toBe(2)
    })

    it("should limit results", async () => {
      const results = await client.query({ limit: 2 })
      expect(results.length).toBe(2)
    })

    it("should sort results by timestamp descending", async () => {
      const results = await client.query()
      expect(results[0].id).toBe("q_3") // Most recent first
      expect(results[2].id).toBe("q_1") // Oldest last
    })

    it("should filter by start time", async () => {
      const results = await client.query({
        startTime: new Date("2026-03-08T11:30:00Z"),
      })
      expect(results.length).toBe(1)
      expect(results[0].id).toBe("q_3")
    })

    it("should filter by end time", async () => {
      const results = await client.query({
        endTime: new Date("2026-03-08T10:30:00Z"),
      })
      expect(results.length).toBe(1)
      expect(results[0].id).toBe("q_1")
    })
  })

  describe("getPath", () => {
    it("should return the observer storage path", () => {
      const storagePath = client.getPath()
      expect(storagePath).toBe(testBasePath)
    })
  })

  describe("file storage disabled", () => {
    it("should not write to storage when disabled", async () => {
      const noStorageClient = createMemoryClient({
        basePath: testBasePath,
        enableDailyNotes: false,
        enableFileStorage: false,
      })

      await noStorageClient.recordObservation({
        id: "no_storage",
        type: "observation",
        timestamp: new Date(),
        data: {},
        tags: [],
      })

      const results = await noStorageClient.query()
      expect(results.length).toBe(0)
    })
  })
})
