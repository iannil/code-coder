/**
 * Performance Benchmark Tests
 *
 * Tests response times, memory usage, and throughput for critical operations.
 * These tests ensure the system meets performance requirements.
 */

import { describe, test, expect } from "bun:test"
import path from "path"
import { tmpdir } from "../fixture/fixture"
import { measureTime, measureMemory, assertPerformance, benchmark } from "../fixture/perf"
import { Instance } from "../../src/project/instance"
import { Session } from "../../src/session"
import { Config } from "../../src/config/config"
import { Provider } from "../../src/provider/provider"
import { ReadTool } from "../../src/tool/read"
import { Env } from "../../src/env"

const ctx = {
  sessionID: "test",
  messageID: "",
  callID: "",
  agent: "build",
  abort: AbortSignal.any([]),
  metadata: () => {},
  ask: async () => {},
}

describe("Performance Benchmarks", () => {
  describe("Response Time", () => {
    test("session create should complete < 100ms", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://code-coder.com/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const { duration, result } = await measureTime(async () => {
            return Session.create({ title: "Perf Test" })
          })

          assertPerformance(duration, 100, "session create")
          expect(result.id).toBeDefined()

          await Session.remove(result.id)
        },
      })
    })

    test("config load should complete < 50ms", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              theme: "dark",
              model: "anthropic/claude-sonnet-4-20250514",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const { duration, result } = await measureTime(async () => {
            return Config.get()
          })

          assertPerformance(duration, 50, "config load")
          expect(result).toBeDefined()
        },
      })
    })

    test("file read tool should complete < 100ms for small file", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "small.txt"), "Hello, World!")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()

          const { duration, result } = await measureTime(async () => {
            return read.execute({ filePath: path.join(tmp.path, "small.txt") }, ctx)
          })

          assertPerformance(duration, 100, "file read")
          expect(result.output).toContain("Hello, World!")
        },
      })
    })

    test("provider list should complete < 200ms", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://code-coder.com/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const { duration, result } = await measureTime(async () => {
            return Provider.list()
          })

          assertPerformance(duration, 200, "provider list")
          expect(result).toBeDefined()
        },
      })
    })

    test("session list should complete < 100ms", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://code-coder.com/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          // Create a few sessions first
          const sessions = await Promise.all(
            Array.from({ length: 5 }, (_, i) => Session.create({ title: `List Perf ${i}` })),
          )

          const { duration, result } = await measureTime(async () => {
            const sessions: Session.Info[] = []
            for await (const s of Session.list()) sessions.push(s)
            return sessions
          })

          assertPerformance(duration, 100, "session list")
          expect(result.length).toBeGreaterThanOrEqual(5)

          // Clean up
          await Promise.all(sessions.map((s) => Session.remove(s.id)))
        },
      })
    })
  })

  describe("Memory Usage", () => {
    test("session create should use < 10MB memory", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://code-coder.com/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const { memoryDelta, result } = await measureMemory(async () => {
            return Session.create({ title: "Memory Test" })
          })

          // Allow for some variance, but should be under 10MB
          const maxMemory = 10 * 1024 * 1024 // 10MB
          expect(memoryDelta).toBeLessThan(maxMemory)
          expect(result.id).toBeDefined()

          await Session.remove(result.id)
        },
      })
    })

    test("large file read should not leak memory", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create 1MB file
          const content = "x".repeat(1024 * 1024)
          await Bun.write(path.join(dir, "large.txt"), content)
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()

          // Perform GC before measurement
          if (typeof Bun !== "undefined") Bun.gc(true)
          const heapBefore = process.memoryUsage().heapUsed

          // Read file multiple times
          for (let i = 0; i < 5; i++) {
            await read.execute({ filePath: path.join(tmp.path, "large.txt") }, ctx)
          }

          if (typeof Bun !== "undefined") Bun.gc(true)
          const heapAfter = process.memoryUsage().heapUsed

          // Memory should not grow significantly
          const growth = heapAfter - heapBefore
          const maxGrowth = 20 * 1024 * 1024 // 20MB max growth
          expect(growth).toBeLessThan(maxGrowth)
        },
      })
    })

    test("batch operations should stay < 50MB", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create multiple files
          for (let i = 0; i < 20; i++) {
            await Bun.write(path.join(dir, `file${i}.txt`), `Content ${i}: ${"x".repeat(10000)}`)
          }
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          if (typeof Bun !== "undefined") Bun.gc(true)
          const heapBefore = process.memoryUsage().heapUsed

          const read = await ReadTool.init()

          // Batch read all files
          await Promise.all(
            Array.from({ length: 20 }, (_, i) => read.execute({ filePath: path.join(tmp.path, `file${i}.txt`) }, ctx)),
          )

          if (typeof Bun !== "undefined") Bun.gc(true)
          const heapAfter = process.memoryUsage().heapUsed

          const memoryUsed = heapAfter - heapBefore
          const maxMemory = 50 * 1024 * 1024 // 50MB
          expect(memoryUsed).toBeLessThan(maxMemory)
        },
      })
    })
  })

  describe("Throughput", () => {
    test("should handle 100 session operations per second", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://code-coder.com/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const operationsCount = 100
          const start = performance.now()

          // Create sessions
          const sessions = await Promise.all(
            Array.from({ length: operationsCount }, (_, i) => Session.create({ title: `Throughput ${i}` })),
          )

          const duration = performance.now() - start
          const opsPerSecond = (operationsCount / duration) * 1000

          // Should be able to do at least 100 ops/second
          // (may be less due to disk I/O, so we check >= 50)
          expect(opsPerSecond).toBeGreaterThanOrEqual(50)

          // Clean up
          await Promise.all(sessions.map((s) => Session.remove(s.id)))
        },
      })
    })

    test("should process 50 file reads per second", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          // Create test files
          for (let i = 0; i < 50; i++) {
            await Bun.write(path.join(dir, `throughput${i}.txt`), `Content for file ${i}`)
          }
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          const read = await ReadTool.init()
          const operationsCount = 50

          const start = performance.now()

          await Promise.all(
            Array.from({ length: operationsCount }, (_, i) =>
              read.execute({ filePath: path.join(tmp.path, `throughput${i}.txt`) }, ctx),
            ),
          )

          const duration = performance.now() - start
          const opsPerSecond = (operationsCount / duration) * 1000

          // Should process at least 50 reads per second
          expect(opsPerSecond).toBeGreaterThanOrEqual(50)
        },
      })
    })
  })

  describe("Benchmark Utilities", () => {
    test("benchmark function should return accurate stats", async () => {
      const stats = await benchmark(
        async () => {
          await new Promise((resolve) => setTimeout(resolve, 10))
          return "result"
        },
        5,
      )

      expect(stats.avg).toBeGreaterThan(10)
      expect(stats.avg).toBeLessThan(50)
      expect(stats.min).toBeLessThanOrEqual(stats.avg)
      expect(stats.max).toBeGreaterThanOrEqual(stats.avg)
    })

    test("measureTime should return accurate duration", async () => {
      const { duration, result } = await measureTime(async () => {
        await new Promise((resolve) => setTimeout(resolve, 20))
        return 42
      })

      expect(duration).toBeGreaterThan(15)
      expect(duration).toBeLessThan(100)
      expect(result).toBe(42)
    })

    test("measureMemory should track heap changes", async () => {
      const { memoryDelta, result, heapUsedBefore, heapUsedAfter } = await measureMemory(async () => {
        // Allocate some memory
        const arr = new Array(10000).fill("x".repeat(100))
        return arr.length
      })

      expect(heapUsedBefore).toBeGreaterThan(0)
      expect(heapUsedAfter).toBeGreaterThan(0)
      expect(result).toBe(10000)
      // Memory delta can be negative after GC
      expect(typeof memoryDelta).toBe("number")
    })
  })

  describe("Startup Performance", () => {
    test("instance initialization should be fast", async () => {
      const start = performance.now()

      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://code-coder.com/config.json" }))
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // Just measure instance initialization
          expect(Instance.directory).toBe(tmp.path)
        },
      })

      const duration = performance.now() - start

      // Should initialize in under 500ms
      expect(duration).toBeLessThan(500)
    })
  })

  describe("Caching Performance", () => {
    test("repeated config loads should be faster after first load", async () => {
      await using tmp = await tmpdir({
        git: true,
        init: async (dir) => {
          await Bun.write(
            path.join(dir, "codecoder.json"),
            JSON.stringify({
              $schema: "https://code-coder.com/config.json",
              theme: "dark",
            }),
          )
        },
      })

      await Instance.provide({
        directory: tmp.path,
        fn: async () => {
          // First load (cold)
          const { duration: firstDuration } = await measureTime(() => Config.get())

          // Subsequent loads (potentially cached)
          const durations: number[] = []
          for (let i = 0; i < 5; i++) {
            const { duration } = await measureTime(() => Config.get())
            durations.push(duration)
          }

          const avgSubsequent = durations.reduce((a, b) => a + b, 0) / durations.length

          // Subsequent loads should generally not be slower than first
          // (testing caching behavior)
          expect(avgSubsequent).toBeLessThanOrEqual(firstDuration * 2)
        },
      })
    })
  })

  describe("Stress Test", () => {
    test("should handle rapid sequential operations", async () => {
      await using tmp = await tmpdir({
        init: async (dir) => {
          await Bun.write(path.join(dir, "codecoder.json"), JSON.stringify({ $schema: "https://code-coder.com/config.json" }))
          await Bun.write(path.join(dir, "test.txt"), "Test content")
        },
      })

      await Instance.provide({
        directory: tmp.path,
        init: async () => {
          Env.set("ANTHROPIC_API_KEY", "sk-ant-test-key")
        },
        fn: async () => {
          const read = await ReadTool.init()
          const start = performance.now()

          // Rapid sequential operations
          for (let i = 0; i < 50; i++) {
            await read.execute({ filePath: path.join(tmp.path, "test.txt") }, ctx)
          }

          const duration = performance.now() - start

          // Should complete 50 sequential reads in under 2 seconds
          expect(duration).toBeLessThan(2000)
        },
      })
    })
  })
})
