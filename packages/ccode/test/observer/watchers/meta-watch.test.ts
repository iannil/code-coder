/**
 * MetaWatch Tests
 *
 * Tests for the Meta Watcher component of the Observer Network.
 *
 * @module test/observer/watchers/meta-watch.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import { MetaWatch, createMetaWatch } from "@/observer/watchers/meta-watch"
import { resetEventStream } from "@/observer"

describe("MetaWatch", () => {
  let watcher: MetaWatch

  beforeEach(() => {
    resetEventStream()
    watcher = createMetaWatch({
      intervalMs: 0,
      qualityThreshold: 0.7,
      coverageThreshold: 0.6,
      maxConsensusDrift: 0.3,
      latencyThreshold: 1000,
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
      await watcher.start()
      expect(watcher.isRunning()).toBe(true)
    })
  })

  describe("getStatus", () => {
    it("should return correct status when stopped", () => {
      const status = watcher.getStatus()
      expect(status.type).toBe("meta")
      expect(status.running).toBe(false)
      expect(status.health).toBe("stopped")
    })

    it("should return correct status when running", async () => {
      await watcher.start()
      const status = watcher.getStatus()

      expect(status.type).toBe("meta")
      expect(status.running).toBe(true)
      expect(status.health).toBe("healthy")
    })
  })

  describe("getWatcherMetrics", () => {
    it("should return empty metrics initially", () => {
      const metrics = watcher.getWatcherMetrics()
      expect(metrics).toEqual([])
    })
  })

  describe("getSystemHealth", () => {
    it("should return healthy status initially", () => {
      const health = watcher.getSystemHealth()
      expect(health.overall).toBe("healthy")
      expect(health.quality).toBe(1) // No observations = perfect quality
    })

    it("should include coverage calculation", () => {
      const health = watcher.getSystemHealth()
      expect(typeof health.coverage).toBe("number")
      expect(health.coverage).toBeGreaterThanOrEqual(0)
      expect(health.coverage).toBeLessThanOrEqual(1)
    })
  })

  describe("getLatencyStatus", () => {
    it("should return latency status with threshold", () => {
      const latencyStatus = watcher.getLatencyStatus()

      expect(latencyStatus.threshold).toBe(1000)
      expect(latencyStatus.avgLatency).toBe(0)
      expect(latencyStatus.exceededCount).toBe(0)
      expect(latencyStatus.watcherLatencies).toEqual({})
    })

    it("should respect custom latency threshold", () => {
      const customWatcher = createMetaWatch({
        latencyThreshold: 500,
      })

      const latencyStatus = customWatcher.getLatencyStatus()
      expect(latencyStatus.threshold).toBe(500)
    })
  })

  describe("calibrate", () => {
    it("should perform calibration and return observation", async () => {
      await watcher.start()

      const observation = await watcher.calibrate()

      expect(observation.type).toBe("calibration")
      expect(observation.watcherType).toBe("meta")
      expect(observation.assessment).toBeDefined()
      expect(observation.recommendations).toBeDefined()
      expect(observation.issues).toBeDefined()
    })

    it("should include coverage and quality in assessment", async () => {
      await watcher.start()

      const observation = await watcher.calibrate()

      expect(typeof observation.assessment.coverage).toBe("number")
      expect(typeof observation.assessment.accuracy).toBe("number")
      expect(typeof observation.assessment.latency).toBe("number")
    })
  })

  describe("triggerObservation", () => {
    it("should return null when not running", async () => {
      const result = await watcher.triggerObservation()
      expect(result).toBeNull()
    })

    it("should check health when triggered", async () => {
      await watcher.start()
      const result = await watcher.triggerObservation()
      // Result may be null if all checks pass
      expect(result === null || typeof result === "object").toBe(true)
    })
  })

  describe("latency threshold configuration", () => {
    it("should use default latency threshold of 1000ms", () => {
      const defaultWatcher = createMetaWatch({})
      const status = defaultWatcher.getLatencyStatus()
      expect(status.threshold).toBe(1000)
    })

    it("should accept custom latency threshold", () => {
      const customWatcher = createMetaWatch({
        latencyThreshold: 2000,
      })
      const status = customWatcher.getLatencyStatus()
      expect(status.threshold).toBe(2000)
    })

    it("should accept low latency threshold", () => {
      const lowThresholdWatcher = createMetaWatch({
        latencyThreshold: 100,
      })
      const status = lowThresholdWatcher.getLatencyStatus()
      expect(status.threshold).toBe(100)
    })
  })
})
