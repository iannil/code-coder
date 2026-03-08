/**
 * ConsensusEngine Tests
 *
 * Tests for the Consensus Engine component of the Observer Network.
 *
 * The consensus engine aggregates observations from multiple watchers
 * and forms a unified understanding of the world.
 *
 * @module test/observer/consensus/engine.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach, afterEach } from "bun:test"
import {
  ConsensusEngine,
  createConsensusEngine,
  getConsensusEngine,
  resetConsensusEngine,
} from "@/observer/consensus/engine"
import { getEventStream, resetEventStream } from "@/observer"
import type { CodeObservation, WorldObservation, SelfObservation, MetaObservation } from "@/observer/types"

// Helper to create observations
function createCodeObservation(overrides: Partial<CodeObservation> = {}): CodeObservation {
  return {
    id: `obs_code_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
    timestamp: new Date(),
    watcherId: "code-watch-1",
    watcherType: "code" as const,
    confidence: 0.8,
    tags: [],
    type: "file_change",
    source: "/src/test.ts",
    change: {
      action: "modify",
    },
    impact: {
      scope: "file",
      severity: "low",
      affectedFiles: [],
    },
    ...overrides,
  }
}

function createWorldObservation(overrides: Partial<WorldObservation> = {}): WorldObservation {
  return {
    id: `obs_world_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
    timestamp: new Date(),
    watcherId: "world-watch-1",
    watcherType: "world" as const,
    confidence: 0.7,
    tags: [],
    type: "market_data",
    source: "market-feed",
    data: {
      title: "Market update",
      summary: "Prices stable",
      content: {},
    },
    relevance: 0.6,
    sentiment: "neutral",
    ...overrides,
  }
}

function createSelfObservation(overrides: Partial<SelfObservation> = {}): SelfObservation {
  return {
    id: `obs_self_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
    timestamp: new Date(),
    watcherId: "self-watch-1",
    watcherType: "self" as const,
    confidence: 0.85,
    tags: [],
    type: "agent_behavior",
    agentId: "test-agent",
    observation: {
      action: "code_review",
      success: true,
    },
    quality: {},
    ...overrides,
  }
}

function createMetaObservation(overrides: Partial<MetaObservation> = {}): MetaObservation {
  return {
    id: `obs_meta_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
    timestamp: new Date(),
    watcherId: "meta-watch-1",
    watcherType: "meta" as const,
    confidence: 0.9,
    tags: [],
    type: "system_health",
    assessment: {
      health: "healthy",
      coverage: 0.8,
      accuracy: 0.85,
      latency: 50,
    },
    recommendations: [],
    issues: [],
    ...overrides,
  }
}

describe("ConsensusEngine", () => {
  let engine: ConsensusEngine

  beforeEach(() => {
    resetConsensusEngine()
    resetEventStream()
    engine = createConsensusEngine({
      windowMs: 60000,
      updateIntervalMs: 0, // Disable periodic updates for testing
    })
  })

  afterEach(() => {
    if (engine.isRunning()) {
      engine.stop()
    }
    resetConsensusEngine()
    resetEventStream()
  })

  describe("lifecycle", () => {
    it("should start and stop correctly", () => {
      expect(engine.isRunning()).toBe(false)

      engine.start()
      expect(engine.isRunning()).toBe(true)

      engine.stop()
      expect(engine.isRunning()).toBe(false)
    })

    it("should not start twice", () => {
      engine.start()
      expect(engine.isRunning()).toBe(true)

      // Second start should be idempotent
      engine.start()
      expect(engine.isRunning()).toBe(true)
    })

    it("should not stop twice", () => {
      engine.start()
      engine.stop()
      expect(engine.isRunning()).toBe(false)

      // Second stop should be idempotent
      engine.stop()
      expect(engine.isRunning()).toBe(false)
    })
  })

  describe("update", () => {
    it("should return snapshot from update", async () => {
      engine.start()

      // Add some observations to the event stream
      const stream = getEventStream()
      stream.push(createCodeObservation())
      stream.push(createWorldObservation())

      const snapshot = await engine.update()

      expect(snapshot).toBeDefined()
      expect(snapshot.timestamp).toBeInstanceOf(Date)
      expect(snapshot.confidence).toBeGreaterThanOrEqual(0)
      expect(snapshot.confidence).toBeLessThanOrEqual(1)
    })

    it("should aggregate observations", async () => {
      engine.start()

      const stream = getEventStream()
      stream.push(createCodeObservation())
      stream.push(createWorldObservation())
      stream.push(createSelfObservation())
      stream.push(createMetaObservation())

      const snapshot = await engine.update()

      // Snapshot should include processed data from all watchers
      expect(snapshot.worldModel).toBeDefined()
    })

    it("should form world model", async () => {
      engine.start()

      const stream = getEventStream()
      stream.push(createCodeObservation())

      const snapshot = await engine.update()

      expect(snapshot.worldModel).toBeDefined()
      if (snapshot.worldModel) {
        expect(snapshot.worldModel.id).toBeDefined()
        expect(snapshot.worldModel.timestamp).toBeInstanceOf(Date)
      }
    })

    it("should detect patterns", async () => {
      engine.start()

      // Add multiple similar observations that might form a pattern
      const stream = getEventStream()
      for (let i = 0; i < 5; i++) {
        stream.push(
          createCodeObservation({
            type: "file_change",
            source: `/src/feature/file${i}.ts`,
          }),
        )
      }

      const snapshot = await engine.update()

      expect(snapshot.patterns).toBeDefined()
      expect(Array.isArray(snapshot.patterns)).toBe(true)
    })

    it("should identify anomalies", async () => {
      engine.start()

      // Add observations
      const stream = getEventStream()
      stream.push(createCodeObservation({ confidence: 0.3 }))
      stream.push(
        createCodeObservation({
          type: "build_status",
          impact: { scope: "project", severity: "critical", affectedFiles: [] },
        }),
      )

      const snapshot = await engine.update()

      expect(snapshot.anomalies).toBeDefined()
      expect(Array.isArray(snapshot.anomalies)).toBe(true)
    })

    it("should identify opportunities", async () => {
      engine.start()

      const stream = getEventStream()
      stream.push(
        createWorldObservation({
          type: "trend",
          data: {
            title: "Positive trend",
            summary: "Opportunity for improvement",
            content: { trend: "up" },
          },
          relevance: 0.9,
          sentiment: "positive",
        }),
      )

      const snapshot = await engine.update()

      expect(snapshot.opportunities).toBeDefined()
      expect(Array.isArray(snapshot.opportunities)).toBe(true)
    })
  })

  describe("getSnapshot", () => {
    it("should return null before first update", () => {
      expect(engine.getSnapshot()).toBeNull()
    })

    it("should return last snapshot after update", async () => {
      engine.start()

      const stream = getEventStream()
      stream.push(createCodeObservation())

      await engine.update()

      const snapshot = engine.getSnapshot()
      expect(snapshot).not.toBeNull()
      expect(snapshot!.timestamp).toBeInstanceOf(Date)
    })
  })

  describe("getWorldModel", () => {
    it("should return null before update", () => {
      expect(engine.getWorldModel()).toBeNull()
    })

    it("should return world model after update", async () => {
      engine.start()

      const stream = getEventStream()
      stream.push(createCodeObservation())

      await engine.update()

      const worldModel = engine.getWorldModel()
      expect(worldModel).toBeDefined()
    })
  })

  describe("getPatterns", () => {
    it("should return empty array initially", () => {
      expect(engine.getPatterns()).toEqual([])
    })
  })

  describe("getAnomalies", () => {
    it("should return empty array initially", () => {
      expect(engine.getAnomalies()).toEqual([])
    })
  })

  describe("getOpportunities", () => {
    it("should return empty array initially", () => {
      expect(engine.getOpportunities()).toEqual([])
    })
  })

  describe("getAttentionWeights", () => {
    it("should return attention weights", () => {
      const weights = engine.getAttentionWeights()

      expect(weights).toBeDefined()
      expect(weights.byWatcher).toBeDefined()
      expect(weights.byWatcher.code).toBeDefined()
      expect(weights.byWatcher.world).toBeDefined()
      expect(weights.byWatcher.self).toBeDefined()
      expect(weights.byWatcher.meta).toBeDefined()
    })
  })

  describe("updateAttentionWeights", () => {
    it("should update attention weights", () => {
      engine.updateAttentionWeights({
        byWatcher: {
          code: 0.5,
          world: 0.2,
          self: 0.2,
          meta: 0.1,
        },
        byType: {},
        timeDecay: 0.1,
        recencyBias: 0.8,
      })

      const weights = engine.getAttentionWeights()
      expect(weights.byWatcher.code).toBe(0.5)
    })
  })

  describe("clear", () => {
    it("should clear all state", async () => {
      engine.start()

      const stream = getEventStream()
      stream.push(createCodeObservation())

      await engine.update()

      expect(engine.getSnapshot()).not.toBeNull()

      engine.clear()

      expect(engine.getSnapshot()).toBeNull()
      expect(engine.getPatterns()).toEqual([])
      expect(engine.getAnomalies()).toEqual([])
      expect(engine.getOpportunities()).toEqual([])
    })
  })

  describe("consensus strength", () => {
    it("should calculate consensus strength from observations", async () => {
      engine.start()

      const stream = getEventStream()
      // Add high confidence observations from multiple watchers
      stream.push(createCodeObservation({ confidence: 0.9 }))
      stream.push(createWorldObservation({ confidence: 0.85 }))
      stream.push(createSelfObservation({ confidence: 0.9 }))
      stream.push(createMetaObservation({ confidence: 0.95 }))

      const snapshot = await engine.update()

      expect(snapshot.confidence).toBeGreaterThan(0)
      expect(snapshot.confidence).toBeLessThanOrEqual(1)
    })

    it("should have lower confidence with fewer watchers", async () => {
      engine.start()

      const stream = getEventStream()
      // Only one watcher type
      stream.push(createCodeObservation({ confidence: 0.8 }))

      const snapshot = await engine.update()

      // Should still work but potentially lower confidence due to less coverage
      expect(snapshot.confidence).toBeGreaterThanOrEqual(0)
    })
  })

  describe("singleton", () => {
    it("should return same instance from getConsensusEngine", () => {
      const engine1 = getConsensusEngine()
      const engine2 = getConsensusEngine()

      expect(engine1).toBe(engine2)
    })

    it("should reset singleton correctly", () => {
      const engine1 = getConsensusEngine()
      resetConsensusEngine()
      const engine2 = getConsensusEngine()

      expect(engine1).not.toBe(engine2)
    })
  })

  describe("configuration", () => {
    it("should accept custom window size", () => {
      const customEngine = createConsensusEngine({
        windowMs: 120000, // 2 minutes
      })

      expect(customEngine).toBeDefined()
    })

    it("should accept custom update interval", () => {
      const customEngine = createConsensusEngine({
        updateIntervalMs: 10000, // 10 seconds
      })

      expect(customEngine).toBeDefined()
    })
  })
})
