import { Log } from "@/util/log"
import { BootstrapTypes } from "./types"

const log = Log.create({ service: "bootstrap.confidence" })

/**
 * ConfidenceSystem manages confidence scoring and evolution for skill candidates.
 *
 * Confidence levels:
 * - 0.0 - 0.2: Discard candidate (too unreliable)
 * - 0.0 - 0.3: Experimental (needs more testing)
 * - 0.3 - 0.6: Stable (can be used with caution)
 * - 0.6 - 0.8: Mature (ready for promotion to skill)
 * - 0.8 - 1.0: Trusted (highly reliable)
 */
export namespace ConfidenceSystem {
  /**
   * Confidence thresholds for different levels
   */
  export const THRESHOLDS = {
    DISCARD: 0.2,
    CANDIDATE_MIN: 0.0,
    EXPERIMENTAL: 0.3,
    STABLE: 0.6,
    MATURE: 0.8,
    PROMOTION_MIN: 0.6,
  } as const

  /**
   * Weights for different confidence factors
   */
  export const WEIGHTS = {
    verificationPassed: 0.3,
    usageCount: 0.15,
    successRate: 0.25,
    scenarioCoverage: 0.15,
    codeQuality: 0.1,
    userFeedback: 0.05,
  } as const

  /**
   * Calculate confidence score from factors
   */
  export function calculate(factors: BootstrapTypes.ConfidenceFactors): number {
    let score = 0

    // Verification is the most important factor
    if (factors.verificationPassed) {
      score += WEIGHTS.verificationPassed
    }

    // Usage count with diminishing returns (saturating formula)
    // Formula: 1 - 1/(1 + log10(usageCount + 1)) gives true diminishing returns
    // Each 10x increase adds less than the previous 10x increase
    const logUsage = Math.log10(factors.usageCount + 1)
    const usageScore = Math.min(1, 1 - 1 / (1 + logUsage))
    score += usageScore * WEIGHTS.usageCount

    // Success rate directly maps to confidence
    score += factors.successRate * WEIGHTS.successRate

    // Scenario coverage
    score += factors.scenarioCoverage * WEIGHTS.scenarioCoverage

    // Optional code quality
    if (factors.codeQuality !== undefined) {
      score += factors.codeQuality * WEIGHTS.codeQuality
    }

    // User feedback can boost or reduce
    if (factors.userFeedback !== undefined) {
      score += (factors.userFeedback + 1) / 2 * WEIGHTS.userFeedback
    }

    // Normalize to [0, 1]
    score = Math.max(0, Math.min(1, score))

    log.info("calculated confidence", {
      score,
      factors,
    })

    return score
  }

  /**
   * Get the confidence level classification
   */
  export function getLevel(confidence: number): BootstrapTypes.ConfidenceLevel {
    if (confidence >= THRESHOLDS.MATURE) {
      return "mature"
    }
    if (confidence >= THRESHOLDS.STABLE) {
      return "stable"
    }
    return "experimental"
  }

  /**
   * Evolve confidence based on usage result
   */
  export function evolve(
    currentConfidence: number,
    result: { success: boolean; context: string },
  ): number {
    // Learning rate decreases as confidence stabilizes
    const learningRate = 0.1 * (1 - currentConfidence * 0.5)

    let delta: number
    if (result.success) {
      // Success increases confidence
      delta = learningRate * (1 - currentConfidence)
    } else {
      // Failure decreases confidence
      delta = -learningRate * currentConfidence * 1.5 // Failures have more impact
    }

    const newConfidence = Math.max(0, Math.min(1, currentConfidence + delta))

    log.info("evolved confidence", {
      previous: currentConfidence,
      new: newConfidence,
      delta,
      success: result.success,
    })

    return newConfidence
  }

  /**
   * Check if a candidate should be discarded
   */
  export function shouldDiscard(confidence: number, attempts: number): boolean {
    // Discard if confidence is too low after multiple attempts
    if (confidence < THRESHOLDS.DISCARD && attempts >= 3) {
      return true
    }

    // Discard if confidence is below minimum after many attempts
    if (confidence < THRESHOLDS.EXPERIMENTAL && attempts >= 5) {
      return true
    }

    return false
  }

  /**
   * Check if a candidate is ready for promotion
   */
  export function isReadyForPromotion(
    confidence: number,
    usageCount: number,
    verificationPassed: boolean,
  ): boolean {
    return (
      confidence >= THRESHOLDS.PROMOTION_MIN &&
      usageCount >= 2 &&
      verificationPassed
    )
  }

  /**
   * Calculate initial confidence based on source quality
   */
  export function calculateInitial(input: {
    toolCallCount: number
    problemLength: number
    solutionLength: number
  }): number {
    let confidence = 0.3 // Base confidence

    // More tool calls suggest more complex, valuable pattern
    if (input.toolCallCount >= 3) {
      confidence += 0.1
    }

    // Well-documented problem and solution
    if (input.problemLength > 100) {
      confidence += 0.05
    }
    if (input.solutionLength > 200) {
      confidence += 0.05
    }

    return Math.min(0.5, confidence) // Cap initial at 0.5
  }

  /**
   * Batch update confidence for multiple candidates
   */
  export function batchEvolve(
    candidates: Array<{
      id: string
      currentConfidence: number
      results: Array<{ success: boolean; context: string }>
    }>,
  ): Array<{ id: string; newConfidence: number }> {
    return candidates.map((candidate) => {
      let confidence = candidate.currentConfidence

      for (const result of candidate.results) {
        confidence = evolve(confidence, result)
      }

      return {
        id: candidate.id,
        newConfidence: confidence,
      }
    })
  }
}
