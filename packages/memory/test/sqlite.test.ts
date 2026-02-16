/**
 * SQLite Memory Backend Tests
 */

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import { SqliteMemory } from "../src/backends/sqlite"
import path from "path"
import os from "os"

describe("SqliteMemory", () => {
  let memory: SqliteMemory
  let tempDir: string

  beforeEach(() => {
    // Use a temporary directory for tests
    tempDir = path.join(os.tmpdir(), `memory-test-${Date.now()}`)
    memory = new SqliteMemory({
      dbPath: path.join(tempDir, "test-brain.db"),
      readOnly: false,
    })
  })

  afterEach(async () => {
    await memory.close()
    // Clean up temp directory
    try {
      Bun.spawnSync(["rm", "-rf", tempDir])
    } catch {
      // Ignore cleanup errors
    }
  })

  describe("name", () => {
    it("should return 'sqlite'", () => {
      expect(memory.name).toBe("sqlite")
    })
  })

  describe("store and get", () => {
    it("should store and retrieve a memory", async () => {
      await memory.store("test_key", "test content", "core")
      const entry = await memory.get("test_key")

      expect(entry).not.toBeNull()
      expect(entry?.key).toBe("test_key")
      expect(entry?.content).toBe("test content")
      expect(entry?.category).toBe("core")
      expect(entry?.source).toBe("sqlite")
    })

    it("should upsert on duplicate key", async () => {
      await memory.store("upsert_key", "original content", "core")
      await memory.store("upsert_key", "updated content", "preference")

      const entry = await memory.get("upsert_key")
      expect(entry?.content).toBe("updated content")
      expect(entry?.category).toBe("preference")

      const count = await memory.count()
      expect(count).toBe(1)
    })

    it("should return null for non-existent key", async () => {
      const entry = await memory.get("nonexistent")
      expect(entry).toBeNull()
    })
  })

  describe("recall", () => {
    beforeEach(async () => {
      await memory.store("rust_lang", "Rust is a systems programming language", "core")
      await memory.store("python_lang", "Python is great for scripting", "core")
      await memory.store("typescript_lang", "TypeScript adds types to JavaScript", "core")
    })

    it("should find memories matching query", async () => {
      const results = await memory.recall("programming language", 10)
      expect(results.length).toBeGreaterThan(0)
    })

    it("should return empty for no matches", async () => {
      const results = await memory.recall("nonexistent_xyz", 10)
      expect(results.length).toBe(0)
    })

    it("should return empty for empty query", async () => {
      const results = await memory.recall("", 10)
      expect(results.length).toBe(0)
    })

    it("should respect limit", async () => {
      const results = await memory.recall("language", 1)
      expect(results.length).toBeLessThanOrEqual(1)
    })

    it("should include scores", async () => {
      const results = await memory.recall("Rust", 10)
      expect(results.length).toBeGreaterThan(0)
      for (const result of results) {
        expect(result.score).toBeDefined()
      }
    })
  })

  describe("list", () => {
    beforeEach(async () => {
      await memory.store("key1", "content1", "core")
      await memory.store("key2", "content2", "preference")
      await memory.store("key3", "content3", "core")
    })

    it("should list all memories", async () => {
      const entries = await memory.list()
      expect(entries.length).toBe(3)
    })

    it("should filter by category", async () => {
      const coreEntries = await memory.list("core")
      expect(coreEntries.length).toBe(2)
      expect(coreEntries.every((e) => e.category === "core")).toBe(true)
    })
  })

  describe("forget", () => {
    it("should remove a memory", async () => {
      await memory.store("to_forget", "content", "core")
      expect(await memory.count()).toBe(1)

      const removed = await memory.forget("to_forget")
      expect(removed).toBe(true)
      expect(await memory.count()).toBe(0)
    })

    it("should return false for non-existent key", async () => {
      const removed = await memory.forget("nonexistent")
      expect(removed).toBe(false)
    })
  })

  describe("count", () => {
    it("should return 0 for empty database", async () => {
      const count = await memory.count()
      expect(count).toBe(0)
    })

    it("should count stored memories", async () => {
      await memory.store("key1", "content1", "core")
      await memory.store("key2", "content2", "core")

      const count = await memory.count()
      expect(count).toBe(2)
    })
  })

  describe("healthCheck", () => {
    it("should return true for healthy database", async () => {
      const healthy = await memory.healthCheck()
      expect(healthy).toBe(true)
    })
  })

  describe("special characters", () => {
    it("should handle unicode content", async () => {
      await memory.store("unicode", "日本語テスト 🚀 Ñoño", "core")
      const entry = await memory.get("unicode")
      expect(entry?.content).toBe("日本語テスト 🚀 Ñoño")
    })

    it("should handle special characters in query", async () => {
      await memory.store("special", "content with quotes 'single' and \"double\"", "core")
      const results = await memory.recall("quotes", 10)
      expect(results.length).toBe(1)
    })
  })
})
