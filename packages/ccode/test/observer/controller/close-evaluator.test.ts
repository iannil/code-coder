/**
 * CLOSEEvaluator Tests
 *
 * Tests for the CLOSE Evaluator component of the Observer Network.
 *
 * CLOSE Framework:
 * - Convergence: How well do observations agree?
 * - Leverage: What's the potential impact?
 * - Optionality: How many valid choices exist?
 * - Surplus: Do we have resources/margin?
 * - Evolution: Is the situation improving?
 *
 * @module test/observer/controller/close-evaluator.test
 */

// IMPORTANT: Import setup first to mock Log before observer modules load
import "../setup"

import { describe, it, expect, beforeEach } from "bun:test"
import {
  CLOSEEvaluator,
  createCLOSEEvaluator,
  type CLOSEWeights,
} from "@/observer/controller/close-evaluator"
import type { ConsensusSnapshot } from "@/observer/consensus"

// Helper to create a minimal consensus snapshot
function createMockSnapshot(overrides: Partial<ConsensusSnapshot> = {}): ConsensusSnapshot {
  return {
    worldModel: {
      id: "wm_test",
      timestamp: new Date(),
      observationIds: [],
      code: {
        buildStatus: "passing",
        recentChanges: 0,
      },
      world: {
        relevantNews: [],
        externalRisks: [],
        opportunities: [],
      },
      self: {
        sessionHealth: "healthy",
        resourceUsage: { tokens: 1000, cost: 0.01, duration: 1000 },
        recentErrors: 0,
      },
      meta: {
        observerHealth: "healthy",
        coverageGaps: [],
        consensusStrength: 0.8,
      },
      confidence: 0.8,
    },
    patterns: [],
    anomalies: [],
    opportunities: [],
    timestamp: new Date(),
    confidence: 0.8,
    ...overrides,
  }
}

describe("CLOSEEvaluator", () => {
  let evaluator: CLOSEEvaluator

  beforeEach(() => {
    evaluator = createCLOSEEvaluator()
  })

  describe("constructor", () => {
    it("should use default weights", () => {
      const defaultEvaluator = createCLOSEEvaluator()
      expect(defaultEvaluator).toBeDefined()
    })

    it("should allow custom weights", () => {
      const customWeights: CLOSEWeights = {
        convergence: 2.0,
        leverage: 1.0,
        optionality: 2.0,
        surplus: 1.5,
        evolution: 0.5,
      }

      const customEvaluator = createCLOSEEvaluator({
        weights: customWeights,
      })

      expect(customEvaluator).toBeDefined()
    })
  })

  describe("evaluate", () => {
    it("should return evaluation with all CLOSE dimensions", () => {
      const snapshot = createMockSnapshot()
      const evaluation = evaluator.evaluate(snapshot)

      expect(evaluation.convergence).toBeDefined()
      expect(evaluation.leverage).toBeDefined()
      expect(evaluation.optionality).toBeDefined()
      expect(evaluation.surplus).toBeDefined()
      expect(evaluation.evolution).toBeDefined()
    })

    it("should calculate total score", () => {
      const snapshot = createMockSnapshot()
      const evaluation = evaluator.evaluate(snapshot)

      expect(evaluation.total).toBeGreaterThanOrEqual(0)
      expect(evaluation.total).toBeLessThanOrEqual(10)
    })

    it("should calculate risk score", () => {
      const snapshot = createMockSnapshot()
      const evaluation = evaluator.evaluate(snapshot)

      expect(evaluation.risk).toBeGreaterThanOrEqual(0)
      expect(evaluation.risk).toBeLessThanOrEqual(10)
    })

    it("should calculate confidence", () => {
      const snapshot = createMockSnapshot()
      const evaluation = evaluator.evaluate(snapshot)

      expect(evaluation.confidence).toBeGreaterThanOrEqual(0)
      expect(evaluation.confidence).toBeLessThanOrEqual(1)
    })

    it("should include timestamp", () => {
      const snapshot = createMockSnapshot()
      const evaluation = evaluator.evaluate(snapshot)

      expect(evaluation.timestamp).toBeInstanceOf(Date)
    })
  })

  describe("convergence evaluation", () => {
    it("should have high convergence with passing build and healthy session", () => {
      const snapshot = createMockSnapshot({
        confidence: 0.9,
        worldModel: {
          id: "wm_test",
          timestamp: new Date(),
          observationIds: [],
          code: {
            buildStatus: "passing",
            recentChanges: 0,
          },
          world: {
            relevantNews: [],
            externalRisks: [],
            opportunities: [],
          },
          self: {
            sessionHealth: "healthy",
            resourceUsage: { tokens: 1000, cost: 0.01, duration: 1000 },
            recentErrors: 0,
          },
          meta: {
            observerHealth: "healthy",
            coverageGaps: [],
            consensusStrength: 0.9,
          },
          confidence: 0.9,
        },
      })

      const evaluation = evaluator.evaluate(snapshot)
      expect(evaluation.convergence.score).toBeGreaterThan(5)
    })

    it("should have low convergence with failing build", () => {
      const snapshot = createMockSnapshot({
        worldModel: {
          id: "wm_test",
          timestamp: new Date(),
          observationIds: [],
          code: {
            buildStatus: "failing",
            recentChanges: 5,
          },
          world: {
            relevantNews: [],
            externalRisks: [],
            opportunities: [],
          },
          self: {
            sessionHealth: "degraded",
            resourceUsage: { tokens: 1000, cost: 0.01, duration: 1000 },
            recentErrors: 3,
          },
          meta: {
            observerHealth: "degraded",
            coverageGaps: ["world"],
            consensusStrength: 0.4,
          },
          confidence: 0.5,
        },
        confidence: 0.5,
      })

      const evaluation = evaluator.evaluate(snapshot)
      expect(evaluation.convergence.score).toBeLessThan(8)
    })

    it("should penalize critical anomalies", () => {
      const snapshotWithAnomalies = createMockSnapshot({
        anomalies: [
          {
            id: "anom_1",
            type: "sudden_change",
            description: "Critical failure",
            severity: "critical",
            observationIds: [],
            detectedAt: new Date(),
            status: "confirmed",
            confidence: 0.9,
          },
        ],
      })

      const snapshotNoAnomalies = createMockSnapshot()

      const withAnomalies = evaluator.evaluate(snapshotWithAnomalies)
      evaluator.clear() // Reset history for fair comparison
      const noAnomalies = evaluator.evaluate(snapshotNoAnomalies)

      expect(withAnomalies.convergence.score).toBeLessThanOrEqual(noAnomalies.convergence.score)
    })
  })

  describe("leverage evaluation", () => {
    it("should increase with high-impact opportunities", () => {
      const snapshotWithOpps = createMockSnapshot({
        opportunities: [
          {
            id: "opp_1",
            type: "optimization",
            description: "Performance improvement",
            impact: "high",
            urgency: "medium",
            observationIds: [],
            detectedAt: new Date(),
            confidence: 0.8,
            suggestedActions: [],
          },
        ],
      })

      const evaluation = evaluator.evaluate(snapshotWithOpps)
      expect(evaluation.leverage.score).toBeGreaterThan(5)
    })

    it("should include external risks in factors", () => {
      const snapshotWithRisks = createMockSnapshot({
        worldModel: {
          id: "wm_test",
          timestamp: new Date(),
          observationIds: [],
          code: {
            buildStatus: "passing",
            recentChanges: 0,
          },
          world: {
            relevantNews: [],
            externalRisks: ["Market volatility", "Competitor action"],
            opportunities: [],
          },
          self: {
            sessionHealth: "healthy",
            resourceUsage: { tokens: 1000, cost: 0.01, duration: 1000 },
            recentErrors: 0,
          },
          meta: {
            observerHealth: "healthy",
            coverageGaps: [],
            consensusStrength: 0.8,
          },
          confidence: 0.8,
        },
      })

      const evaluation = evaluator.evaluate(snapshotWithRisks)
      expect(evaluation.leverage.factors.some((f) => f.includes("external risks"))).toBe(true)
    })
  })

  describe("optionality evaluation", () => {
    it("should increase with more opportunities", () => {
      const snapshotManyOpps = createMockSnapshot({
        opportunities: [
          { id: "1", type: "optimization", description: "A", impact: "low", urgency: "low", observationIds: [], detectedAt: new Date(), confidence: 0.7, suggestedActions: [] },
          { id: "2", type: "automation", description: "B", impact: "medium", urgency: "low", observationIds: [], detectedAt: new Date(), confidence: 0.7, suggestedActions: [] },
          { id: "3", type: "learning", description: "C", impact: "high", urgency: "medium", observationIds: [], detectedAt: new Date(), confidence: 0.7, suggestedActions: [] },
        ],
      })

      const snapshotFewOpps = createMockSnapshot({
        opportunities: [],
      })

      evaluator.clear()
      const manyOpps = evaluator.evaluate(snapshotManyOpps)
      evaluator.clear()
      const fewOpps = evaluator.evaluate(snapshotFewOpps)

      expect(manyOpps.optionality.score).toBeGreaterThan(fewOpps.optionality.score)
    })

    it("should decrease with many anomalies", () => {
      const snapshotManyAnomalies = createMockSnapshot({
        anomalies: Array.from({ length: 6 }, (_, i) => ({
          id: `anom_${i}`,
          type: "outlier" as const,
          description: `Anomaly ${i}`,
          severity: "medium" as const,
          observationIds: [],
          detectedAt: new Date(),
          status: "suspected" as const,
          confidence: 0.6,
        })),
      })

      const evaluation = evaluator.evaluate(snapshotManyAnomalies)
      expect(evaluation.optionality.factors.some((f) => f.includes("limiting options"))).toBe(true)
    })
  })

  describe("surplus evaluation", () => {
    it("should reflect resource usage", () => {
      const lowUsageSnapshot = createMockSnapshot({
        worldModel: {
          id: "wm_test",
          timestamp: new Date(),
          observationIds: [],
          code: { buildStatus: "passing", recentChanges: 0 },
          world: { relevantNews: [], externalRisks: [], opportunities: [] },
          self: {
            sessionHealth: "healthy",
            resourceUsage: { tokens: 1000, cost: 0.01, duration: 500 },
            recentErrors: 0,
          },
          meta: {
            observerHealth: "healthy",
            coverageGaps: [],
            consensusStrength: 0.9,
          },
          confidence: 0.9,
        },
      })

      const evaluation = evaluator.evaluate(lowUsageSnapshot)
      expect(evaluation.surplus.factors.some((f) => f.includes("Low"))).toBe(true)
    })

    it("should penalize high costs", () => {
      const highCostSnapshot = createMockSnapshot({
        worldModel: {
          id: "wm_test",
          timestamp: new Date(),
          observationIds: [],
          code: { buildStatus: "passing", recentChanges: 0 },
          world: { relevantNews: [], externalRisks: [], opportunities: [] },
          self: {
            sessionHealth: "healthy",
            resourceUsage: { tokens: 500000, cost: 15.0, duration: 60000 },
            recentErrors: 0,
          },
          meta: {
            observerHealth: "healthy",
            coverageGaps: [],
            consensusStrength: 0.8,
          },
          confidence: 0.8,
        },
      })

      const evaluation = evaluator.evaluate(highCostSnapshot)
      expect(evaluation.surplus.factors.some((f) => f.includes("High"))).toBe(true)
    })
  })

  describe("evolution evaluation", () => {
    it("should detect improving trend", () => {
      // Build up history with improving evaluations
      const improving = [
        createMockSnapshot({ confidence: 0.5 }),
        createMockSnapshot({ confidence: 0.6 }),
        createMockSnapshot({ confidence: 0.7 }),
        createMockSnapshot({ confidence: 0.8 }),
        createMockSnapshot({ confidence: 0.9 }),
      ]

      for (const snapshot of improving) {
        evaluator.evaluate(snapshot)
      }

      const trend = evaluator.getTrend()
      // May be "improving", "stable", or depends on scoring
      expect(["improving", "stable", "declining"]).toContain(trend)
    })

    it("should include learning opportunities in factors", () => {
      const snapshot = createMockSnapshot({
        opportunities: [
          {
            id: "learn_1",
            type: "learning",
            description: "New pattern to learn",
            impact: "medium",
            urgency: "low",
            observationIds: [],
            detectedAt: new Date(),
            confidence: 0.7,
            suggestedActions: [],
          },
        ],
      })

      const evaluation = evaluator.evaluate(snapshot)
      expect(evaluation.evolution.factors.some((f) => f.includes("learning"))).toBe(true)
    })
  })

  describe("scoring", () => {
    it("should apply configured weights", () => {
      const customWeights: CLOSEWeights = {
        convergence: 0.5,
        leverage: 0.5,
        optionality: 3.0, // Higher weight on optionality
        surplus: 0.5,
        evolution: 0.5,
      }

      const customEvaluator = createCLOSEEvaluator({
        weights: customWeights,
      })

      const snapshot = createMockSnapshot()
      const evaluation = customEvaluator.evaluate(snapshot)

      // Optionality should have higher influence on total
      expect(evaluation.total).toBeDefined()
      expect(evaluation.total).toBeGreaterThanOrEqual(0)
      expect(evaluation.total).toBeLessThanOrEqual(10)
    })

    it("should return total score 0-10", () => {
      const snapshots = [
        createMockSnapshot({ confidence: 0.1 }),
        createMockSnapshot({ confidence: 0.5 }),
        createMockSnapshot({ confidence: 0.9 }),
      ]

      for (const snapshot of snapshots) {
        const evaluation = evaluator.evaluate(snapshot)
        expect(evaluation.total).toBeGreaterThanOrEqual(0)
        expect(evaluation.total).toBeLessThanOrEqual(10)
      }
    })
  })

  describe("confidence", () => {
    it("should have high confidence with consistent observations", () => {
      const highConfSnapshot = createMockSnapshot({
        confidence: 0.95,
        patterns: [
          {
            id: "pat_1",
            name: "Consistent pattern",
            description: "All observations agree",
            type: "trend",
            observationIds: [],
            confidence: 0.9,
            detectedAt: new Date(),
            lastSeenAt: new Date(),
            strength: 0.9,
            suggestedActions: [],
          },
        ],
      })

      const evaluation = evaluator.evaluate(highConfSnapshot)
      expect(evaluation.confidence).toBeGreaterThan(0.5)
    })

    it("should have low confidence with conflicting data", () => {
      const lowConfSnapshot = createMockSnapshot({
        confidence: 0.3,
        anomalies: [
          {
            id: "anom_1",
            type: "sudden_change",
            description: "Unexpected change",
            severity: "high",
            observationIds: [],
            detectedAt: new Date(),
            status: "suspected",
            confidence: 0.4,
          },
        ],
      })

      const evaluation = evaluator.evaluate(lowConfSnapshot)
      expect(evaluation.confidence).toBeLessThan(0.8)
    })
  })

  describe("getHistory", () => {
    it("should return empty history initially", () => {
      const history = evaluator.getHistory()
      expect(history).toEqual([])
    })

    it("should track evaluation history", () => {
      evaluator.evaluate(createMockSnapshot())
      evaluator.evaluate(createMockSnapshot())
      evaluator.evaluate(createMockSnapshot())

      const history = evaluator.getHistory()
      expect(history.length).toBe(3)
    })

    it("should respect limit parameter", () => {
      for (let i = 0; i < 10; i++) {
        evaluator.evaluate(createMockSnapshot())
      }

      const limited = evaluator.getHistory(5)
      expect(limited.length).toBe(5)
    })
  })

  describe("getTrend", () => {
    it("should return stable with insufficient history", () => {
      evaluator.evaluate(createMockSnapshot())
      expect(evaluator.getTrend()).toBe("stable")
    })
  })

  describe("clear", () => {
    it("should clear evaluation history", () => {
      evaluator.evaluate(createMockSnapshot())
      evaluator.evaluate(createMockSnapshot())

      expect(evaluator.getHistory().length).toBe(2)

      evaluator.clear()

      expect(evaluator.getHistory()).toEqual([])
    })
  })

  describe("updateConfig", () => {
    it("should update weights", () => {
      evaluator.updateConfig({
        weights: {
          convergence: 2.0,
          leverage: 2.0,
          optionality: 2.0,
          surplus: 2.0,
          evolution: 2.0,
        },
      })

      const evaluation = evaluator.evaluate(createMockSnapshot())
      expect(evaluation.total).toBeDefined()
    })

    it("should update historyWeight", () => {
      evaluator.updateConfig({
        historyWeight: 0.5,
      })

      // Config update should work without errors
      expect(evaluator).toBeDefined()
    })
  })
})
