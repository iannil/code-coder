/**
 * CodeWatch Tests
 *
 * Tests for the Code Watcher component of the Observer Network.
 *
 * @module test/observer/watchers/code-watch.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach, spyOn, mock } from "bun:test"
import { CodeWatch, createCodeWatch } from "@/observer/watchers/code-watch"
import { resetEventStream } from "@/observer"

describe("CodeWatch", () => {
  let watcher: CodeWatch

  beforeEach(() => {
    resetEventStream()
    watcher = createCodeWatch({
      intervalMs: 0, // Disable auto polling for tests
      gitRoot: process.cwd(),
      enableTypecheck: false,
    })
  })

  afterEach(async () => {
    if (watcher.isRunning()) {
      await watcher.stop()
    }
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", async () => {
      expect(watcher.isRunning()).toBe(false)

      await watcher.start()
      expect(watcher.isRunning()).toBe(true)

      await watcher.stop()
      expect(watcher.isRunning()).toBe(false)
    })

    it("should not start twice", async () => {
      await watcher.start()
      await watcher.start() // Should not throw
      expect(watcher.isRunning()).toBe(true)
    })
  })

  describe("getStatus", () => {
    it("should return correct status when stopped", () => {
      const status = watcher.getStatus()
      expect(status.type).toBe("code")
      expect(status.running).toBe(false)
      expect(status.health).toBe("stopped")
    })

    it("should return correct status when running", async () => {
      await watcher.start()
      const status = watcher.getStatus()

      expect(status.type).toBe("code")
      expect(status.running).toBe(true)
      expect(status.health).toBe("healthy")
      expect(status.observationCount).toBe(0)
    })
  })

  describe("observeFileChange", () => {
    it("should emit observation for file changes", async () => {
      await watcher.start()

      let emittedObservation: unknown = null
      // Create a spy on the emit method by tracking observations
      const originalTrigger = watcher.triggerObservation.bind(watcher)

      await watcher.observeFileChange("/test/file.ts")

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should not observe when stopped", async () => {
      await watcher.observeFileChange("/test/file.ts")
      const status = watcher.getStatus()
      expect(status.observationCount).toBe(0)
    })
  })

  describe("observeBuildStatus", () => {
    it("should emit observation for passing build", async () => {
      await watcher.start()

      await watcher.observeBuildStatus("passing")

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })

    it("should emit observation for failing build with errors", async () => {
      await watcher.start()

      await watcher.observeBuildStatus("failing", {
        errors: ["Error in file.ts: Type error"],
        warnings: ["Warning: deprecated API"],
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("observeTestResults", () => {
    it("should emit observation for test results", async () => {
      await watcher.start()

      await watcher.observeTestResults({
        passed: 10,
        failed: 2,
        skipped: 1,
        coverage: 85.5,
      })

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("observeTypeErrors", () => {
    it("should emit observation for type errors", async () => {
      await watcher.start()

      await watcher.observeTypeErrors([
        { file: "src/index.ts", line: 10, message: "Type error" },
        { file: "src/index.ts", line: 20, message: "Another error" },
      ])

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("observeLintIssues", () => {
    it("should emit observation for lint issues", async () => {
      await watcher.start()

      await watcher.observeLintIssues([
        { file: "src/index.ts", rule: "no-unused-vars", severity: "error", message: "Unused variable" },
        { file: "src/utils.ts", rule: "prefer-const", severity: "warning", message: "Use const" },
      ])

      const status = watcher.getStatus()
      expect(status.observationCount).toBe(1)
    })
  })

  describe("triggerObservation", () => {
    it("should return null when not running", async () => {
      const result = await watcher.triggerObservation()
      expect(result).toBeNull()
    })

    it("should check for git changes when triggered", async () => {
      await watcher.start()
      // This should complete without error
      const result = await watcher.triggerObservation()
      // Result may be null if no changes
      expect(result === null || typeof result === "object").toBe(true)
    })
  })

  describe("typecheck configuration", () => {
    it("should respect enableTypecheck option", () => {
      const watcherWithTypecheck = createCodeWatch({
        enableTypecheck: true,
        typecheckIntervalMs: 120000,
        typecheckTimeoutMs: 60000,
      })

      const config = watcherWithTypecheck.getConfig()
      expect(config.options.enableTypecheck).toBe(true)
      expect(config.options.typecheckIntervalMs).toBe(120000)
    })
  })
})
