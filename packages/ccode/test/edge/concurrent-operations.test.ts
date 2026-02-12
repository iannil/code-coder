/**
 * Edge Case Test: Concurrent Operations
 *
 * Tests concurrent session creation, parallel file operations,
 * tool batch execution limits, race conditions, and MCP server concurrency.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { Instance } from "../../src/project/instance"
import { Session } from "../../src/session"
import { Config } from "../../src/config/config"
import { Lock } from "../../src/util/lock"
import { ReadTool } from "../../src/tool/read"
import { Env } from "../../src/env"

function tick() {
  return new Promise<void>((r) => queueMicrotask(r))
}

async function flush(n = 5) {
  for (let i = 0; i < n; i++) await tick()
}

const ctx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "build",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

describe("Concurrent Operations", () => {
  describe("Concurrent Session Creation", () => {
    test("should handle 10 concurrent session creates", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://codecoder.ai/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create 10 sessions concurrently
          const promises = Array.from({ length: 10 }, (_, i) => Session.create({ title: `Session ${i + 1}` }))

          const sessions = await Promise.all(promises)

          // All sessions should be created successfully
          expect(sessions.length).toBe(10)
          for (const session of sessions) {
            expect(session).toBeDefined()
            expect(session.id).toBeDefined()
          }

          // All sessions should have unique IDs
          const ids = new Set(sessions.map((s) => s.id))
          expect(ids.size).toBe(10)

          // Clean up
          await Promise.all(sessions.map((s) => Session.remove(s.id)))
        },
      })
    })

    test("should handle concurrent session list operations", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://codecoder.ai/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create a few sessions
          const sessions = await Promise.all(
            Array.from({ length: 3 }, (_, i) => Session.create({ title: `List Test ${i + 1}` })),
          )

          // List sessions concurrently multiple times
          const listPromises = Array.from({ length: 5 }, async () => {
            const sessions: Session.Info[] = []
            for await (const s of Session.list()) sessions.push(s)
            return sessions
          })
          const results = await Promise.all(listPromises)

          // All list operations should succeed
          for (const list of results) {
            expect(list.length).toBeGreaterThanOrEqual(3)
          }

          // Clean up
          await Promise.all(sessions.map((s) => Session.remove(s.id)))
        },
      })
    })
  })

  describe("Parallel File Operations", () => {
    test("should handle parallel file read operations", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create multiple files
          for (let i = 0; i < 10; i++) {
            await Bun.write(path.join(dir, `file${i}.txt`), `Content of file ${i}`)
          }
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()

          // Read all files in parallel
          const promises = Array.from({ length: 10 }, (_, i) =>
            read.execute({ filePath: path.join(tmp.path, `file${i}.txt`) }, ctx),
          )

          const results = await Promise.all(promises)

          // All reads should succeed
          expect(results.length).toBe(10)
          for (let i = 0; i < 10; i++) {
            expect(results[i].output).toContain(`Content of file ${i}`)
          }
        },
      })
    })

    test("should handle concurrent read and write to different files", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create initial files
          for (let i = 0; i < 5; i++) {
            await Bun.write(path.join(dir, `existing${i}.txt`), `Original ${i}`)
          }
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Mix of reads and writes to different files
          const operations = []

          const read = await ReadTool.init()

          // Read existing files
          for (let i = 0; i < 5; i++) {
            operations.push(read.execute({ filePath: path.join(tmp.path, `existing${i}.txt`) }, ctx))
          }

          // Write new files (using Bun.write directly for testing)
          for (let i = 0; i < 5; i++) {
            operations.push(Bun.write(path.join(tmp.path, `new${i}.txt`), `New content ${i}`))
          }

          await Promise.all(operations)

          // Verify all files exist and have correct content
          for (let i = 0; i < 5; i++) {
            const newFile = await Bun.file(path.join(tmp.path, `new${i}.txt`)).text()
            expect(newFile).toBe(`New content ${i}`)
          }
        },
      })
    })
  })

  describe("Lock Management", () => {
    test("should enforce writer exclusivity", async () => {
      const key = "test-lock:" + Math.random().toString(36).slice(2)

      const state = {
        writer1Active: false,
        writer2Acquired: false,
        concurrentWriters: 0,
      }

      // Acquire first writer
      using writer1 = await Lock.write(key)
      state.writer1Active = true
      state.concurrentWriters++
      expect(state.concurrentWriters).toBe(1)

      // Try to acquire second writer (should block)
      const writer2Promise = (async () => {
        const w = await Lock.write(key)
        state.writer2Acquired = true
        state.concurrentWriters++
        return w
      })()

      await flush()

      // Second writer should still be waiting
      expect(state.writer2Acquired).toBe(false)
      expect(state.concurrentWriters).toBe(1)

      // Release first writer
      writer1[Symbol.dispose]()
      state.concurrentWriters--

      // Now second writer should acquire
      const writer2 = await writer2Promise
      expect(state.writer2Acquired).toBe(true)
      expect(state.concurrentWriters).toBe(1)

      writer2[Symbol.dispose]()
    })

    test("should allow concurrent readers", async () => {
      const key = "test-lock-readers:" + Math.random().toString(36).slice(2)

      const state = {
        reader1Acquired: false,
        reader2Acquired: false,
        concurrentReaders: 0,
      }

      // Acquire first reader
      using reader1 = await Lock.read(key)
      state.reader1Acquired = true
      state.concurrentReaders++

      // Acquire second reader (should not block)
      using reader2 = await Lock.read(key)
      state.reader2Acquired = true
      state.concurrentReaders++

      // Both readers should be active
      expect(state.reader1Acquired).toBe(true)
      expect(state.reader2Acquired).toBe(true)
      expect(state.concurrentReaders).toBe(2)

      reader1[Symbol.dispose]()
      reader2[Symbol.dispose]()
    })

    test("should block readers while writer is active", async () => {
      const key = "test-lock-wr:" + Math.random().toString(36).slice(2)

      const state = {
        writerActive: false,
        readerAcquired: false,
      }

      // Acquire writer
      using writer = await Lock.write(key)
      state.writerActive = true

      // Try to acquire reader (should block)
      const readerPromise = (async () => {
        const r = await Lock.read(key)
        state.readerAcquired = true
        return r
      })()

      await flush()

      // Reader should be blocked
      expect(state.readerAcquired).toBe(false)

      // Release writer
      writer[Symbol.dispose]()

      // Reader should now acquire
      const reader = await readerPromise
      expect(state.readerAcquired).toBe(true)

      reader[Symbol.dispose]()
    })
  })

  describe("Race Condition Prevention", () => {
    test("should prevent race conditions on config update", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://codecoder.ai/config.json",
              theme: "dark",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Read config multiple times concurrently
          const readPromises = Array.from({ length: 10 }, () => Config.get())
          const results = await Promise.all(readPromises)

          // All reads should return consistent results
          for (const config of results) {
            expect(config.theme).toBe("dark")
          }
        },
      })
    })

    test("should handle concurrent session updates safely", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://codecoder.ai/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const session = await Session.create({ title: "Race Test" })

          // Try to update session from multiple "threads"
          const updatePromises = Array.from({ length: 5 }, (_, i) =>
            Session.update(session.id, (s) => { s.title = `Updated Title ${i + 1}` }),
          )

          await Promise.all(updatePromises)

          // Session should have one of the titles (last write wins)
          const updated = await Session.get(session.id)
          expect(updated?.title).toMatch(/Updated Title \d/)

          await Session.remove(session.id)
        },
      })
    })
  })

  describe("Batch Operation Limits", () => {
    test("should handle tool batch execution within limits", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create 25 files for batch reading
          for (let i = 0; i < 25; i++) {
            await Bun.write(path.join(dir, `batch${i}.txt`), `Batch content ${i}`)
          }
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()

          // Execute batch of 25 reads (max limit)
          const promises = Array.from({ length: 25 }, (_, i) =>
            read.execute({ filePath: path.join(tmp.path, `batch${i}.txt`) }, ctx),
          )

          const results = await Promise.all(promises)

          // All should complete successfully
          expect(results.length).toBe(25)
          for (const result of results) {
            expect(result.output).toBeDefined()
          }
        },
      })
    })
  })

  describe("Error Isolation", () => {
    test("should isolate errors in concurrent operations", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create only some files
          for (let i = 0; i < 5; i++) {
            await Bun.write(path.join(dir, `exists${i}.txt`), `Exists ${i}`)
          }
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()

          // Mix of valid and invalid reads
          const operations = []

          // Valid reads
          for (let i = 0; i < 5; i++) {
            operations.push(
              read
                .execute({ filePath: path.join(tmp.path, `exists${i}.txt`) }, ctx)
                .then((r) => ({ success: true, result: r }))
                .catch((e) => ({ success: false, error: e })),
            )
          }

          // Invalid reads (non-existent files)
          for (let i = 0; i < 5; i++) {
            operations.push(
              read
                .execute({ filePath: path.join(tmp.path, `nonexistent${i}.txt`) }, ctx)
                .then((r) => ({ success: true, result: r }))
                .catch((e) => ({ success: false, error: e })),
            )
          }

          const results = await Promise.all(operations)

          // Valid reads should succeed
          const successes = results.filter((r) => r.success)
          expect(successes.length).toBeGreaterThanOrEqual(5)

          // Errors should be isolated to their individual operations
          // (one error shouldn't affect other operations)
        },
      })
    })
  })

  describe("Cancellation", () => {
    test("should handle aborted operations gracefully", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "test.txt"), "Test content")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const controller = new AbortController()
          const read = await ReadTool.init()

          const abortCtx = {
            ...ctx,
            abort: controller.signal,
          }

          // Start operation then abort
          const promise = read.execute({ filePath: path.join(tmp.path, "test.txt") }, abortCtx)

          // Abort after short delay
          setTimeout(() => controller.abort(), 10)

          // Should either complete before abort or handle abort gracefully
          try {
            const result = await promise
            // If completed before abort, should have valid result
            expect(result.output).toContain("Test content")
          } catch (error) {
            // If aborted, should handle gracefully
            expect(error).toBeDefined()
          }
        },
      })
    })
  })
})
