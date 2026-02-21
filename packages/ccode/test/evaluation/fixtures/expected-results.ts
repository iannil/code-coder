/**
 * Expected results for evaluation tests
 *
 * These define the expected outcomes for various test scenarios,
 * enabling automated verification of evaluation metrics.
 */

import type { BootstrapTypes } from "@/bootstrap/types"
import { MOCK_CANDIDATES } from "./mock-candidates"

/**
 * Expected confidence evolution results
 */
export const CONFIDENCE_EXPECTATIONS = {
  /**
   * Expected behavior when confidence evolves with success
   */
  successEvolution: {
    initialConfidence: 0.5,
    afterOneSuccess: { min: 0.51, max: 0.6 }, // Should increase modestly
    afterThreeSuccesses: { min: 0.55, max: 0.7 }, // Should continue increasing
    afterTenSuccesses: { min: 0.65, max: 0.85 }, // Should approach but not exceed mature
  },

  /**
   * Expected behavior when confidence evolves with failure
   */
  failureEvolution: {
    initialConfidence: 0.5,
    afterOneFailure: { min: 0.4, max: 0.48 }, // Should decrease more than success increases
    afterThreeFailures: { min: 0.25, max: 0.38 }, // Should decrease significantly
    afterTenFailures: { min: 0.1, max: 0.25 }, // Should approach discard threshold
  },

  /**
   * Asymmetric learning rate verification
   * Failure impact should be ~1.5x success impact
   */
  asymmetricLearning: {
    failureToSuccessRatio: { min: 1.3, max: 1.7 },
  },

  /**
   * High confidence learning rate reduction
   * Learning rate should decrease at high confidence
   */
  learningRateReduction: {
    atLowConfidence: { min: 0.08, max: 0.12 }, // ~10% base rate
    atHighConfidence: { min: 0.04, max: 0.07 }, // ~5-7% reduced rate
  },

  /**
   * Discard threshold expectations
   */
  discardThreshold: {
    confidenceThreshold: 0.2,
    minimumAttempts: 3,
    extendedAttempts: 5, // For below experimental
    experimentalThreshold: 0.3,
  },

  /**
   * Promotion threshold expectations
   */
  promotionThreshold: {
    confidenceThreshold: 0.6,
    minimumUsageCount: 2,
    requiresVerificationPassed: true,
  },
} as const

/**
 * Expected awareness assessment results
 */
export const AWARENESS_EXPECTATIONS = {
  /**
   * Easy tasks should return high confidence
   */
  easyTasks: {
    examples: [
      "Read the package.json file",
      "Show the contents of README.md",
      "List files in the current directory",
    ],
    expectedConfidence: { min: 0.7, max: 1.0 },
    shouldBeConfident: true,
  },

  /**
   * Medium tasks should return moderate confidence
   */
  mediumTasks: {
    examples: [
      "Write a function to parse JSON safely",
      "Fix the type error in user.ts",
      "Add a new test for the login component",
    ],
    expectedConfidence: { min: 0.5, max: 0.8 },
    shouldBeConfident: true,
  },

  /**
   * Hard tasks should return lower confidence
   */
  hardTasks: {
    examples: [
      "Deploy a Kubernetes cluster with Terraform on AWS",
      "Implement real-time video streaming with WebRTC",
      "Set up a blockchain node with smart contract deployment",
    ],
    expectedConfidence: { min: 0.2, max: 0.6 },
    shouldBeConfident: false,
  },

  /**
   * Tasks requiring external resources should identify missing capabilities
   */
  externalResourceTasks: {
    examples: [
      "Create a GitHub pull request", // Needs MCP: github
      "Send a Slack message", // Needs MCP: slack
      "Run Playwright E2E tests", // Needs MCP: browser
    ],
    shouldIdentifyMissingResources: true,
  },

  /**
   * Tool identification expectations
   */
  toolIdentification: {
    requiredBuiltinTools: ["read", "edit", "write", "bash", "glob", "grep"],
    optionalTools: ["task", "websearch", "webfetch"],
  },
} as const

/**
 * Expected verification results
 */
export const VERIFICATION_EXPECTATIONS = {
  /**
   * Test scenario generation
   */
  scenarioGeneration: {
    minimumScenarios: 3,
    maximumScenarios: 5,
    scenariosMustBeDiverse: true,
  },

  /**
   * Verification pass rate requirements
   */
  passRate: {
    minimumForPass: 0.6, // 60% of scenarios must pass
    idealPassRate: 0.8, // 80% for high confidence
  },

  /**
   * Self-correction expectations
   */
  selfCorrection: {
    maximumAttempts: 2,
    expectedImprovementRate: 0.5, // 50% of corrections should improve pass rate
  },

  /**
   * Verification loop termination
   */
  loopTermination: {
    maximumVerificationAttempts: 3,
    shouldTerminateOnPass: true,
    shouldTerminateOnMaxAttempts: true,
  },
} as const

/**
 * Expected crystallization results
 */
export const CRYSTALLIZATION_EXPECTATIONS = {
  /**
   * Minimum tool calls to consider for crystallization
   */
  minimumToolCalls: 2,

  /**
   * Skill type inference rules
   */
  skillTypeInference: {
    workflow: {
      minToolCalls: 3,
      orBashDominant: true, // > 50% bash commands
    },
    agent: {
      requiresTaskDelegation: true,
    },
    pattern: {
      isDefault: true, // When no other type matches
    },
  },

  /**
   * Initial confidence based on source quality
   */
  initialConfidence: {
    baseValue: 0.3,
    toolCallBonus: { threshold: 3, bonus: 0.1 },
    problemLengthBonus: { threshold: 100, bonus: 0.05 },
    solutionLengthBonus: { threshold: 200, bonus: 0.05 },
    maximumInitial: 0.5,
  },
} as const

/**
 * Expected acquisition results
 */
export const ACQUISITION_EXPECTATIONS = {
  /**
   * Known MCP servers that should be suggested
   */
  knownMcpServers: ["github", "filesystem", "slack", "browser", "memory"],

  /**
   * Resource discovery confidence thresholds
   */
  discoveryConfidence: {
    knownResource: { min: 0.6, max: 1.0 },
    suggestedResource: { min: 0.3, max: 0.6 },
    unknownResource: { min: 0.1, max: 0.3 },
  },
} as const

/**
 * E2E evolution expectations
 */
export const E2E_EVOLUTION_EXPECTATIONS = {
  /**
   * Full cycle success criteria
   */
  fullCycleSuccess: {
    mustCreateCandidate: true,
    mustPassVerification: true,
    mustIncreaseConfidenceOnReuse: true,
    promotionAfterSuccesses: 3,
  },

  /**
   * Target metrics for evaluation
   */
  targetMetrics: {
    confidenceCalibrationError: 0.15, // < 15% error
    resourceSuggestionRelevance: 0.8, // > 80% relevant
    patternExtractionSuccess: 0.7, // > 70% success rate
    convergenceTime: 10, // < 10 uses to mature
    selfCorrectionSuccess: 0.5, // > 50% success rate
    experienceRetrievalRecall: 0.7, // > 70% recall
    fullEvolutionCycleSuccess: 0.6, // > 60% complete successfully
  },
} as const

/**
 * Get expected result for a specific candidate
 */
export function getExpectedResultForCandidate(
  candidateKey: keyof typeof MOCK_CANDIDATES,
): {
  shouldDiscard: boolean
  shouldPromote: boolean
  confidenceLevel: BootstrapTypes.ConfidenceLevel
} {
  const candidate = MOCK_CANDIDATES[candidateKey]

  const shouldDiscard =
    candidate.verification.confidence < CONFIDENCE_EXPECTATIONS.discardThreshold.confidenceThreshold &&
    candidate.verification.attempts >= CONFIDENCE_EXPECTATIONS.discardThreshold.minimumAttempts

  const shouldPromote =
    candidate.verification.confidence >= CONFIDENCE_EXPECTATIONS.promotionThreshold.confidenceThreshold &&
    candidate.metadata.usageCount >= CONFIDENCE_EXPECTATIONS.promotionThreshold.minimumUsageCount &&
    candidate.verification.status === "passed"

  const confidenceLevel: BootstrapTypes.ConfidenceLevel =
    candidate.verification.confidence >= 0.6
      ? "mature"
      : candidate.verification.confidence >= 0.3
        ? "stable"
        : "experimental"

  return { shouldDiscard, shouldPromote, confidenceLevel }
}

/**
 * Validate that a confidence value falls within expected range
 */
export function isWithinExpectedRange(
  value: number,
  range: { min: number; max: number },
): boolean {
  return value >= range.min && value <= range.max
}
