/**
 * Markdown Memory Backend Tests
 */

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import { MarkdownMemory } from "../src/backends/markdown"
import path from "path"
import os from "os"

describe("MarkdownMemory", () => {
  let memory: MarkdownMemory
  let tempDir: string

  beforeEach(() => {
    // Use a temporary directory for tests
    tempDir = path.join(os.tmpdir(), `memory-md-test-${Date.now()}`)
    memory = new MarkdownMemory({
      basePath: tempDir,
      projectId: "test-project",
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
    it("should return 'markdown'", () => {
      expect(memory.name).toBe("markdown")
    })
  })

  describe("store and get", () => {
    it("should store and retrieve from long-term memory", async () => {
      await memory.store("test_key", "test content", "preference")
      const entry = await memory.get("test_key")

      expect(entry).not.toBeNull()
      expect(entry?.key).toBe("test_key")
      expect(entry?.content).toBe("test content")
      expect(entry?.source).toBe("markdown")
    })

    it("should store to daily notes for 'daily' category", async () => {
      await memory.store("daily_key", "daily content", "daily")
      const entry = await memory.get("daily_key")

      expect(entry).not.toBeNull()
      expect(entry?.category).toBe("daily")
    })

    it("should return null for non-existent key", async () => {
      const entry = await memory.get("nonexistent")
      expect(entry).toBeNull()
    })
  })

  describe("recall", () => {
    beforeEach(async () => {
      await memory.store("rust_pref", "I prefer Rust for systems programming", "preference")
      await memory.store("python_pref", "Python is good for scripting", "preference")
    })

    it("should find memories matching query", async () => {
      const results = await memory.recall("programming", 10)
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
      const results = await memory.recall("prefer", 1)
      expect(results.length).toBeLessThanOrEqual(1)
    })

    it("should include scores", async () => {
      const results = await memory.recall("Rust", 10)
      if (results.length > 0) {
        for (const result of results) {
          expect(result.score).toBeDefined()
        }
      }
    })
  })

  describe("list", () => {
    beforeEach(async () => {
      await memory.store("key1", "content1", "preference")
      await memory.store("key2", "content2", "decision")
      await memory.store("key3", "content3", "preference")
    })

    it("should list all memories", async () => {
      const entries = await memory.list()
      expect(entries.length).toBe(3)
    })
  })

  describe("forget", () => {
    it("should remove a memory", async () => {
      await memory.store("to_forget", "content", "preference")
      const beforeCount = await memory.count()

      const removed = await memory.forget("to_forget")
      expect(removed).toBe(true)

      const afterCount = await memory.count()
      expect(afterCount).toBeLessThan(beforeCount)
    })

    it("should return false for non-existent key", async () => {
      const removed = await memory.forget("nonexistent")
      expect(removed).toBe(false)
    })
  })

  describe("count", () => {
    it("should count stored memories", async () => {
      await memory.store("key1", "content1", "preference")
      await memory.store("key2", "content2", "decision")

      const count = await memory.count()
      expect(count).toBeGreaterThanOrEqual(2)
    })
  })

  describe("healthCheck", () => {
    it("should return true for healthy storage", async () => {
      const healthy = await memory.healthCheck()
      expect(healthy).toBe(true)
    })
  })

  describe("file structure", () => {
    it("should create MEMORY.md file", async () => {
      await memory.store("test", "content", "preference")

      const longTermPath = path.join(tempDir, "MEMORY.md")
      const file = Bun.file(longTermPath)
      expect(file.size).toBeGreaterThan(0)
    })

    it("should create daily notes in daily directory", async () => {
      await memory.store("daily_entry", "daily content", "daily")

      const dailyPath = path.join(tempDir, "daily")
      const globber = new Bun.Glob("*.md")
      const files: string[] = []
      for await (const file of globber.scan({ cwd: dailyPath })) {
        files.push(file)
      }

      expect(files.length).toBeGreaterThanOrEqual(1)
    })
  })
})
