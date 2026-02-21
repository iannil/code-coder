import { describe, test, expect } from "bun:test"
import { ConfidenceSystem } from "@/bootstrap/confidence"

describe("ConfidenceSystem", () => {
  describe("THRESHOLDS", () => {
    test("defines expected threshold values", () => {
      expect(ConfidenceSystem.THRESHOLDS.DISCARD).toBe(0.2)
      expect(ConfidenceSystem.THRESHOLDS.CANDIDATE_MIN).toBe(0.0)
      expect(ConfidenceSystem.THRESHOLDS.EXPERIMENTAL).toBe(0.3)
      expect(ConfidenceSystem.THRESHOLDS.STABLE).toBe(0.6)
      expect(ConfidenceSystem.THRESHOLDS.MATURE).toBe(0.8)
      expect(ConfidenceSystem.THRESHOLDS.PROMOTION_MIN).toBe(0.6)
    })
  })

  describe("calculate", () => {
    test("returns 0 for all-zero factors", () => {
      const confidence = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 0,
        successRate: 0,
        scenarioCoverage: 0,
      })
      expect(confidence).toBe(0)
    })

    test("returns high confidence for all positive factors", () => {
      const confidence = ConfidenceSystem.calculate({
        verificationPassed: true,
        usageCount: 100,
        successRate: 1.0,
        scenarioCoverage: 1.0,
        codeQuality: 1.0,
        userFeedback: 1.0,
      })
      expect(confidence).toBeGreaterThan(0.8)
    })

    test("verification passed contributes significantly", () => {
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
      expect(withVerification - withoutVerification).toBeCloseTo(0.3, 1)
    })

    test("usage count has diminishing returns", () => {
      const low = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 1,
        successRate: 0,
        scenarioCoverage: 0,
      })
      const medium = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 10,
        successRate: 0,
        scenarioCoverage: 0,
      })
      const high = ConfidenceSystem.calculate({
        verificationPassed: false,
        usageCount: 100,
        successRate: 0,
        scenarioCoverage: 0,
      })

      // Each 10x increase should add less
      const firstIncrease = medium - low
      const secondIncrease = high - medium
      expect(secondIncrease).toBeLessThan(firstIncrease)
    })

    test("clamps result to [0, 1]", () => {
      const confidence = ConfidenceSystem.calculate({
        verificationPassed: true,
        usageCount: 10000,
        successRate: 1.0,
        scenarioCoverage: 1.0,
        codeQuality: 1.0,
        userFeedback: 1.0,
      })
      expect(confidence).toBeLessThanOrEqual(1)
      expect(confidence).toBeGreaterThanOrEqual(0)
    })

    test("negative user feedback reduces confidence", () => {
      const positive = ConfidenceSystem.calculate({
        verificationPassed: true,
        usageCount: 10,
        successRate: 0.8,
        scenarioCoverage: 0.5,
        userFeedback: 1.0,
      })
      const negative = ConfidenceSystem.calculate({
        verificationPassed: true,
        usageCount: 10,
        successRate: 0.8,
        scenarioCoverage: 0.5,
        userFeedback: -1.0,
      })
      expect(positive).toBeGreaterThan(negative)
    })
  })

  describe("getLevel", () => {
    test("returns experimental for low confidence", () => {
      expect(ConfidenceSystem.getLevel(0.1)).toBe("experimental")
      expect(ConfidenceSystem.getLevel(0.29)).toBe("experimental")
    })

    test("returns stable for medium confidence", () => {
      expect(ConfidenceSystem.getLevel(0.6)).toBe("stable")
      expect(ConfidenceSystem.getLevel(0.79)).toBe("stable")
    })

    test("returns mature for high confidence", () => {
      expect(ConfidenceSystem.getLevel(0.8)).toBe("mature")
      expect(ConfidenceSystem.getLevel(1.0)).toBe("mature")
    })
  })

  describe("evolve", () => {
    test("success increases confidence", () => {
      const initial = 0.5
      const evolved = ConfidenceSystem.evolve(initial, {
        success: true,
        context: "test",
      })
      expect(evolved).toBeGreaterThan(initial)
    })

    test("failure decreases confidence", () => {
      const initial = 0.5
      const evolved = ConfidenceSystem.evolve(initial, {
        success: false,
        context: "test",
      })
      expect(evolved).toBeLessThan(initial)
    })

    test("failure has stronger impact than success", () => {
      const initial = 0.5
      const afterSuccess = ConfidenceSystem.evolve(initial, {
        success: true,
        context: "test",
      })
      const afterFailure = ConfidenceSystem.evolve(initial, {
        success: false,
        context: "test",
      })

      const successDelta = afterSuccess - initial
      const failureDelta = initial - afterFailure

      expect(failureDelta).toBeGreaterThan(successDelta)
    })

    test("clamps result to [0, 1]", () => {
      // Evolve from near-zero with failure
      const fromLow = ConfidenceSystem.evolve(0.01, {
        success: false,
        context: "test",
      })
      expect(fromLow).toBeGreaterThanOrEqual(0)

      // Evolve from near-one with success
      const fromHigh = ConfidenceSystem.evolve(0.99, {
        success: true,
        context: "test",
      })
      expect(fromHigh).toBeLessThanOrEqual(1)
    })

    test("learning rate decreases as confidence stabilizes", () => {
      // Low confidence should change more
      const lowStart = 0.2
      const lowEvolved = ConfidenceSystem.evolve(lowStart, {
        success: true,
        context: "test",
      })
      const lowDelta = lowEvolved - lowStart

      // High confidence should change less
      const highStart = 0.8
      const highEvolved = ConfidenceSystem.evolve(highStart, {
        success: true,
        context: "test",
      })
      const highDelta = highEvolved - highStart

      expect(lowDelta).toBeGreaterThan(highDelta)
    })
  })

  describe("shouldDiscard", () => {
    test("discards low confidence after multiple attempts", () => {
      expect(ConfidenceSystem.shouldDiscard(0.1, 3)).toBe(true)
      expect(ConfidenceSystem.shouldDiscard(0.1, 2)).toBe(false)
    })

    test("does not discard decent confidence", () => {
      expect(ConfidenceSystem.shouldDiscard(0.5, 5)).toBe(false)
    })

    test("discards experimental after many attempts", () => {
      expect(ConfidenceSystem.shouldDiscard(0.25, 5)).toBe(true)
      expect(ConfidenceSystem.shouldDiscard(0.25, 4)).toBe(false)
    })
  })

  describe("isReadyForPromotion", () => {
    test("requires all conditions met", () => {
      // All conditions met
      expect(ConfidenceSystem.isReadyForPromotion(0.7, 3, true)).toBe(true)

      // Low confidence
      expect(ConfidenceSystem.isReadyForPromotion(0.5, 3, true)).toBe(false)

      // Low usage
      expect(ConfidenceSystem.isReadyForPromotion(0.7, 1, true)).toBe(false)

      // Not verified
      expect(ConfidenceSystem.isReadyForPromotion(0.7, 3, false)).toBe(false)
    })

    test("threshold is exactly 0.6", () => {
      expect(ConfidenceSystem.isReadyForPromotion(0.6, 2, true)).toBe(true)
      expect(ConfidenceSystem.isReadyForPromotion(0.59, 2, true)).toBe(false)
    })
  })

  describe("calculateInitial", () => {
    test("returns base confidence for minimal input", () => {
      const confidence = ConfidenceSystem.calculateInitial({
        toolCallCount: 1,
        problemLength: 10,
        solutionLength: 10,
      })
      expect(confidence).toBeCloseTo(0.3, 1)
    })

    test("increases for more tool calls", () => {
      const low = ConfidenceSystem.calculateInitial({
        toolCallCount: 2,
        problemLength: 10,
        solutionLength: 10,
      })
      const high = ConfidenceSystem.calculateInitial({
        toolCallCount: 5,
        problemLength: 10,
        solutionLength: 10,
      })
      expect(high).toBeGreaterThan(low)
    })

    test("increases for longer problem description", () => {
      const short = ConfidenceSystem.calculateInitial({
        toolCallCount: 2,
        problemLength: 50,
        solutionLength: 10,
      })
      const long = ConfidenceSystem.calculateInitial({
        toolCallCount: 2,
        problemLength: 200,
        solutionLength: 10,
      })
      expect(long).toBeGreaterThan(short)
    })

    test("caps at 0.5", () => {
      const confidence = ConfidenceSystem.calculateInitial({
        toolCallCount: 100,
        problemLength: 10000,
        solutionLength: 10000,
      })
      expect(confidence).toBeLessThanOrEqual(0.5)
    })
  })

  describe("batchEvolve", () => {
    test("evolves multiple candidates", () => {
      const results = ConfidenceSystem.batchEvolve([
        {
          id: "1",
          currentConfidence: 0.5,
          results: [{ success: true, context: "test" }],
        },
        {
          id: "2",
          currentConfidence: 0.5,
          results: [{ success: false, context: "test" }],
        },
      ])

      expect(results).toHaveLength(2)
      expect(results[0].id).toBe("1")
      expect(results[0].newConfidence).toBeGreaterThan(0.5)
      expect(results[1].id).toBe("2")
      expect(results[1].newConfidence).toBeLessThan(0.5)
    })

    test("applies multiple results sequentially", () => {
      const results = ConfidenceSystem.batchEvolve([
        {
          id: "1",
          currentConfidence: 0.5,
          results: [
            { success: true, context: "test1" },
            { success: true, context: "test2" },
            { success: true, context: "test3" },
          ],
        },
      ])

      // Multiple successes should increase more than single
      const singleSuccess = ConfidenceSystem.evolve(0.5, {
        success: true,
        context: "test",
      })
      expect(results[0].newConfidence).toBeGreaterThan(singleSuccess)
    })
  })
})
