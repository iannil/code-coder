/**
 * Confidence Evolution Evaluation Tests
 *
 * Verifies that the confidence system correctly:
 * - E1: Increases confidence on success
 * - E2: Decreases confidence on failure (with stronger impact)
 * - E3: Reduces learning rate at high confidence
 * - E4: Discards candidates below threshold
 * - E5: Promotes candidates meeting criteria
 */

import { describe, test, expect, beforeEach } from "bun:test"
import { ConfidenceSystem } from "@/bootstrap/confidence"
import { CandidateStore } from "@/bootstrap/candidate-store"
import { BootstrapTypes } from "@/bootstrap/types"
import {
  createMockCandidate,
  createCandidateWithConfidence,
  MOCK_CANDIDATES,
} from "./fixtures/mock-candidates"
import {
  CONFIDENCE_EXPECTATIONS,
  isWithinExpectedRange,
} from "./fixtures/expected-results"
import {
  calculateAsymmetricLearningRatio,
  createMetricResult,
  aggregateMetrics,
} from "./utils/metrics"
import { formatMetricResult } from "./utils/reporters"
import { Log } from "@/util/log"
import { tmpdir } from "../fixture/fixture"
import path from "path"
import os from "os"

// Suppress logging during tests
Log.init({ print: false })

describe("Confidence Evolution Evaluation", () => {
  describe("E1: Success Enhancement", () => {
    test("confidence increases after success", () => {
      const initial = 0.5
      const result = ConfidenceSystem.evolve(initial, { success: true, context: "test" })

      expect(result).toBeGreaterThan(initial)
      expect(result).toBeLessThanOrEqual(1.0)
    })

    test("successive successes continue to increase confidence", () => {
      let confidence = 0.3
      const history: number[] = [confidence]

      for (let i = 0; i < 5; i++) {
        confidence = ConfidenceSystem.evolve(confidence, { success: true, context: `test-${i}` })
        history.push(confidence)
      }

      // Each step should increase
      for (let i = 1; i < history.length; i++) {
        expect(history[i]).toBeGreaterThan(history[i - 1])
      }
    })

    test("confidence after one success is within expected range", () => {
      const initial = CONFIDENCE_EXPECTATIONS.successEvolution.initialConfidence
      const result = ConfidenceSystem.evolve(initial, { success: true, context: "test" })

      const expected = CONFIDENCE_EXPECTATIONS.successEvolution.afterOneSuccess
      expect(isWithinExpectedRange(result, expected)).toBe(true)
    })

    test("confidence approaches but does not exceed 1.0", () => {
      let confidence = 0.9

      for (let i = 0; i < 20; i++) {
        confidence = ConfidenceSystem.evolve(confidence, { success: true, context: `test-${i}` })
      }

      expect(confidence).toBeLessThanOrEqual(1.0)
      expect(confidence).toBeGreaterThan(0.95)
    })
  })

  describe("E2: Failure Penalty", () => {
    test("confidence decreases after failure", () => {
      const initial = 0.5
      const result = ConfidenceSystem.evolve(initial, { success: false, context: "test" })

      expect(result).toBeLessThan(initial)
      expect(result).toBeGreaterThanOrEqual(0)
    })

    test("successive failures continue to decrease confidence", () => {
      let confidence = 0.7
      const history: number[] = [confidence]

      for (let i = 0; i < 5; i++) {
        confidence = ConfidenceSystem.evolve(confidence, { success: false, context: `test-${i}` })
        history.push(confidence)
      }

      // Each step should decrease
      for (let i = 1; i < history.length; i++) {
        expect(history[i]).toBeLessThan(history[i - 1])
      }
    })

    test("failure has stronger impact than success (asymmetric learning)", () => {
      const initial = 0.5
      const afterSuccess = ConfidenceSystem.evolve(initial, { success: true, context: "test" })
      const afterFailure = ConfidenceSystem.evolve(initial, { success: false, context: "test" })

      const successDelta = afterSuccess - initial
      const failureDelta = initial - afterFailure

      // Failure impact should be ~1.5x success impact
      const ratio = failureDelta / successDelta
      expect(ratio).toBeGreaterThan(1.2)
      expect(ratio).toBeLessThan(2.0)

      const expectedRatio = CONFIDENCE_EXPECTATIONS.asymmetricLearning.failureToSuccessRatio
      expect(isWithinExpectedRange(ratio, expectedRatio)).toBe(true)
    })

    test("confidence does not go below 0", () => {
      let confidence = 0.1

      for (let i = 0; i < 20; i++) {
        confidence = ConfidenceSystem.evolve(confidence, { success: false, context: `test-${i}` })
      }

      expect(confidence).toBeGreaterThanOrEqual(0)
    })
  })

  describe("E3: Stable Convergence", () => {
    test("learning rate is higher at low confidence", () => {
      const lowConfidence = 0.2
      const highConfidence = 0.8

      const lowAfterSuccess = ConfidenceSystem.evolve(lowConfidence, { success: true, context: "test" })
      const highAfterSuccess = ConfidenceSystem.evolve(highConfidence, { success: true, context: "test" })

      const lowDelta = lowAfterSuccess - lowConfidence
      const highDelta = highAfterSuccess - highConfidence

      // Low confidence should have larger delta
      expect(lowDelta).toBeGreaterThan(highDelta)
    })

    test("learning rate decreases as confidence approaches 1.0", () => {
      const deltas: number[] = []
      const confidenceLevels = [0.2, 0.4, 0.6, 0.8]

      for (const conf of confidenceLevels) {
        const after = ConfidenceSystem.evolve(conf, { success: true, context: "test" })
        deltas.push(after - conf)
      }

      // Each delta should be smaller than the previous (monotonically decreasing)
      for (let i = 1; i < deltas.length; i++) {
        expect(deltas[i]).toBeLessThanOrEqual(deltas[i - 1])
      }
    })

    test("high confidence delta is less than half of low confidence delta", () => {
      const lowDelta = ConfidenceSystem.evolve(0.3, { success: true, context: "test" }) - 0.3
      const highDelta = ConfidenceSystem.evolve(0.8, { success: true, context: "test" }) - 0.8

      expect(highDelta).toBeLessThan(lowDelta * 0.5)
    })
  })

  describe("E4: Discard Mechanism", () => {
    test("shouldDiscard returns false for candidates above threshold", () => {
      const result = ConfidenceSystem.shouldDiscard(0.5, 3)
      expect(result).toBe(false)
    })

    test("shouldDiscard returns true for low confidence after 3 attempts", () => {
      const result = ConfidenceSystem.shouldDiscard(0.15, 3)
      expect(result).toBe(true)
    })

    test("shouldDiscard returns false for low confidence with few attempts", () => {
      const result = ConfidenceSystem.shouldDiscard(0.15, 1)
      expect(result).toBe(false)
    })

    test("shouldDiscard returns true for experimental confidence after 5 attempts", () => {
      const result = ConfidenceSystem.shouldDiscard(0.25, 5)
      expect(result).toBe(true)
    })

    test("mock candidate 'shouldDiscard' meets discard criteria", () => {
      const candidate = MOCK_CANDIDATES.shouldDiscard
      const result = ConfidenceSystem.shouldDiscard(
        candidate.verification.confidence,
        candidate.verification.attempts,
      )
      expect(result).toBe(true)
    })

    test("verified candidate does not meet discard criteria", () => {
      const candidate = MOCK_CANDIDATES.verified
      const result = ConfidenceSystem.shouldDiscard(
        candidate.verification.confidence,
        candidate.verification.attempts,
      )
      expect(result).toBe(false)
    })
  })

  describe("E5: Promotion Conditions", () => {
    test("isReadyForPromotion returns true when all criteria met", () => {
      const result = ConfidenceSystem.isReadyForPromotion(0.7, 3, true)
      expect(result).toBe(true)
    })

    test("isReadyForPromotion returns false when confidence too low", () => {
      const result = ConfidenceSystem.isReadyForPromotion(0.5, 3, true)
      expect(result).toBe(false)
    })

    test("isReadyForPromotion returns false when usage count too low", () => {
      const result = ConfidenceSystem.isReadyForPromotion(0.7, 1, true)
      expect(result).toBe(false)
    })

    test("isReadyForPromotion returns false when verification not passed", () => {
      const result = ConfidenceSystem.isReadyForPromotion(0.7, 3, false)
      expect(result).toBe(false)
    })

    test("readyForPromotion candidate meets promotion criteria", () => {
      const candidate = MOCK_CANDIDATES.readyForPromotion
      const result = ConfidenceSystem.isReadyForPromotion(
        candidate.verification.confidence,
        candidate.metadata.usageCount,
        candidate.verification.status === "passed",
      )
      expect(result).toBe(true)
    })

    test("experimental candidate does not meet promotion criteria", () => {
      const candidate = MOCK_CANDIDATES.experimental
      const result = ConfidenceSystem.isReadyForPromotion(
        candidate.verification.confidence,
        candidate.metadata.usageCount,
        candidate.verification.status === "passed",
      )
      expect(result).toBe(false)
    })
  })

  describe("Confidence Level Classification", () => {
    test("getLevel returns 'experimental' for low confidence", () => {
      expect(ConfidenceSystem.getLevel(0.1)).toBe("experimental")
      expect(ConfidenceSystem.getLevel(0.25)).toBe("experimental")
    })

    test("getLevel returns 'stable' for medium confidence", () => {
      expect(ConfidenceSystem.getLevel(0.3)).toBe("stable")
      expect(ConfidenceSystem.getLevel(0.5)).toBe("stable")
      expect(ConfidenceSystem.getLevel(0.7)).toBe("stable")
    })

    test("getLevel returns 'mature' for high confidence", () => {
      expect(ConfidenceSystem.getLevel(0.8)).toBe("mature")
      expect(ConfidenceSystem.getLevel(0.9)).toBe("mature")
      expect(ConfidenceSystem.getLevel(1.0)).toBe("mature")
    })
  })

  describe("Confidence Calculation", () => {
    test("calculate produces score within [0, 1]", () => {
      const factors: BootstrapTypes.ConfidenceFactors = {
        verificationPassed: true,
        usageCount: 10,
        successRate: 0.8,
        scenarioCoverage: 0.7,
        codeQuality: 0.9,
        userFeedback: 0.5,
      }

      const score = ConfidenceSystem.calculate(factors)
      expect(score).toBeGreaterThanOrEqual(0)
      expect(score).toBeLessThanOrEqual(1)
    })

    test("verification passed has significant impact", () => {
      const withVerification = ConfidenceSystem.calculate({
        verificationPassed: true,
        usageCount: 0,
        successRate: 0,
        scenarioCoverage: 0,
      })

      const withoutVerification = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 0,
        successRate: 0,
        scenarioCoverage: 0,
      })

      expect(withVerification).toBeGreaterThan(withoutVerification)
      expect(withVerification - withoutVerification).toBeGreaterThanOrEqual(0.2)
    })

    test("usage count has diminishing returns", () => {
      const score1 = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 1,
        successRate: 0,
        scenarioCoverage: 0,
      })

      const score10 = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 10,
        successRate: 0,
        scenarioCoverage: 0,
      })

      const score100 = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 100,
        successRate: 0,
        scenarioCoverage: 0,
      })

      // Diminishing returns: 1->10 delta should be greater than 10->100 delta
      const delta1to10 = score10 - score1
      const delta10to100 = score100 - score10

      expect(delta1to10).toBeGreaterThan(delta10to100)
    })

    test("success rate directly impacts score", () => {
      const lowSuccess = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 5,
        successRate: 0.2,
        scenarioCoverage: 0.5,
      })

      const highSuccess = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 5,
        successRate: 0.9,
        scenarioCoverage: 0.5,
      })

      expect(highSuccess).toBeGreaterThan(lowSuccess)
    })
  })

  describe("Initial Confidence Calculation", () => {
    test("base confidence is 0.3", () => {
      const result = ConfidenceSystem.calculateInitial({
        toolCallCount: 1,
        problemLength: 10,
        solutionLength: 10,
      })
      expect(result).toBeGreaterThanOrEqual(0.3)
    })

    test("more tool calls increase initial confidence", () => {
      const fewTools = ConfidenceSystem.calculateInitial({
        toolCallCount: 1,
        problemLength: 50,
        solutionLength: 50,
      })

      const manyTools = ConfidenceSystem.calculateInitial({
        toolCallCount: 5,
        problemLength: 50,
        solutionLength: 50,
      })

      expect(manyTools).toBeGreaterThan(fewTools)
    })

    test("detailed problem and solution increase initial confidence", () => {
      const brief = ConfidenceSystem.calculateInitial({
        toolCallCount: 3,
        problemLength: 50,
        solutionLength: 50,
      })

      const detailed = ConfidenceSystem.calculateInitial({
        toolCallCount: 3,
        problemLength: 200,
        solutionLength: 300,
      })

      expect(detailed).toBeGreaterThan(brief)
    })

    test("initial confidence capped at 0.5", () => {
      const result = ConfidenceSystem.calculateInitial({
        toolCallCount: 100,
        problemLength: 10000,
        solutionLength: 10000,
      })

      expect(result).toBeLessThanOrEqual(0.5)
    })
  })

  describe("Batch Evolution", () => {
    test("batchEvolve updates multiple candidates", () => {
      const candidates = [
        { id: "1", currentConfidence: 0.5, results: [{ success: true, context: "test" }] },
        { id: "2", currentConfidence: 0.5, results: [{ success: false, context: "test" }] },
        { id: "3", currentConfidence: 0.5, results: [{ success: true, context: "test" }, { success: true, context: "test" }] },
      ]

      const results = ConfidenceSystem.batchEvolve(candidates)

      expect(results).toHaveLength(3)
      expect(results[0].newConfidence).toBeGreaterThan(0.5) // Success
      expect(results[1].newConfidence).toBeLessThan(0.5) // Failure
      expect(results[2].newConfidence).toBeGreaterThan(results[0].newConfidence) // Two successes
    })

    test("batchEvolve applies multiple results sequentially", () => {
      const candidates = [
        {
          id: "1",
          currentConfidence: 0.5,
          results: [
            { success: true, context: "1" },
            { success: true, context: "2" },
            { success: false, context: "3" },
          ],
        },
      ]

      const results = ConfidenceSystem.batchEvolve(candidates)

      // Two successes then one failure - should be slightly above initial
      // but less than pure success
      expect(results[0].newConfidence).toBeGreaterThan(0.5)
    })
  })

  describe("Threshold Constants", () => {
    test("thresholds are properly ordered", () => {
      expect(ConfidenceSystem.THRESHOLDS.DISCARD).toBeLessThan(ConfidenceSystem.THRESHOLDS.EXPERIMENTAL)
      expect(ConfidenceSystem.THRESHOLDS.EXPERIMENTAL).toBeLessThan(ConfidenceSystem.THRESHOLDS.STABLE)
      expect(ConfidenceSystem.THRESHOLDS.STABLE).toBeLessThan(ConfidenceSystem.THRESHOLDS.MATURE)
    })

    test("weights sum to approximately 1", () => {
      const totalWeight =
        ConfidenceSystem.WEIGHTS.verificationPassed +
        ConfidenceSystem.WEIGHTS.usageCount +
        ConfidenceSystem.WEIGHTS.successRate +
        ConfidenceSystem.WEIGHTS.scenarioCoverage +
        ConfidenceSystem.WEIGHTS.codeQuality +
        ConfidenceSystem.WEIGHTS.userFeedback

      expect(totalWeight).toBeCloseTo(1.0, 1)
    })
  })
})

describe("Confidence Metrics Aggregation", () => {
  test("generates evaluation summary for confidence dimension", () => {
    const metrics = [
      createMetricResult("Asymmetric Learning Ratio", 1.5, 1.5, "gte"),
      createMetricResult("Learning Rate at Low Conf", 0.1, 0.08, "gte"),
      createMetricResult("Learning Rate at High Conf", 0.05, 0.07, "lte"),
      createMetricResult("Discard Threshold Accuracy", 1.0, 0.9, "gte"),
      createMetricResult("Promotion Accuracy", 1.0, 0.9, "gte"),
    ]

    const summary = aggregateMetrics("Confidence Evolution", metrics)

    expect(summary.dimension).toBe("Confidence Evolution")
    expect(summary.metrics).toHaveLength(5)
    expect(summary.passRate).toBeGreaterThan(0.5)
  })
})
