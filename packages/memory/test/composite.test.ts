/**
 * Composite Memory Backend Tests
 */

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import { CompositeMemory } from "../src/backends/composite"
import path from "path"
import os from "os"

describe("CompositeMemory", () => {
  let memory: CompositeMemory
  let tempDir: string

  beforeEach(() => {
    tempDir = path.join(os.tmpdir(), `memory-composite-test-${Date.now()}`)
    memory = new CompositeMemory(
      { dbPath: path.join(tempDir, "brain.db"), readOnly: false },
      { basePath: path.join(tempDir, "markdown"), projectId: "test" },
      { primary: "sqlite", writeToAll: true, conflictStrategy: "primary-wins" },
    )
  })

  afterEach(async () => {
    await memory.close()
    try {
      Bun.spawnSync(["rm", "-rf", tempDir])
    } catch {
      // Ignore cleanup errors
    }
  })

  describe("name", () => {
    it("should return 'composite'", () => {
      expect(memory.name).toBe("composite")
    })
  })

  describe("store and get", () => {
    it("should store in both backends", async () => {
      await memory.store("test_key", "test content", "core")

      const entry = await memory.get("test_key")
      expect(entry).not.toBeNull()
      expect(entry?.key).toBe("test_key")
      expect(entry?.content).toBe("test content")
    })

    it("should retrieve from primary backend first", async () => {
      await memory.store("primary_test", "content", "core")

      const entry = await memory.get("primary_test")
      expect(entry).not.toBeNull()
      // Should come from SQLite (primary)
      expect(entry?.source).toBe("sqlite")
    })
  })

  describe("recall", () => {
    beforeEach(async () => {
      await memory.store("rust", "Rust is a systems language", "core")
      await memory.store("python", "Python is interpreted", "core")
    })

    it("should merge results from both backends", async () => {
      const results = await memory.recall("language", 10)
      expect(results.length).toBeGreaterThan(0)
    })

    it("should deduplicate results by key", async () => {
      const results = await memory.recall("Rust", 10)
      const keys = results.map((r) => r.key)
      const uniqueKeys = [...new Set(keys)]
      expect(keys.length).toBe(uniqueKeys.length)
    })
  })

  describe("forget", () => {
    it("should remove from both backends", async () => {
      await memory.store("to_forget", "content", "core")
      const removed = await memory.forget("to_forget")
      expect(removed).toBe(true)

      const entry = await memory.get("to_forget")
      expect(entry).toBeNull()
    })
  })

  describe("healthCheck", () => {
    it("should return true if primary is healthy", async () => {
      const healthy = await memory.healthCheck()
      expect(healthy).toBe(true)
    })
  })

  describe("getStats", () => {
    it("should return statistics for both backends", async () => {
      await memory.store("key1", "content1", "core")

      const stats = await memory.getStats()
      expect(stats.sqliteHealthy).toBe(true)
      expect(stats.markdownHealthy).toBe(true)
      expect(stats.sqliteCount).toBeGreaterThanOrEqual(1)
      expect(stats.markdownCount).toBeGreaterThanOrEqual(1)
    })
  })

  describe("syncToSecondary", () => {
    it("should sync data from primary to secondary", async () => {
      await memory.store("sync_test", "content", "core")

      const synced = await memory.syncToSecondary()
      expect(synced).toBeGreaterThanOrEqual(1)
    })
  })

  describe("conflict strategies", () => {
    it("should use primary-wins strategy by default", async () => {
      const primaryWins = new CompositeMemory(
        { dbPath: path.join(tempDir, "pw-brain.db"), readOnly: false },
        { basePath: path.join(tempDir, "pw-markdown"), projectId: "test" },
        { primary: "sqlite", conflictStrategy: "primary-wins" },
      )

      await primaryWins.store("conflict", "primary content", "core")
      const entry = await primaryWins.get("conflict")
      expect(entry?.source).toBe("sqlite")

      await primaryWins.close()
    })
  })
})
