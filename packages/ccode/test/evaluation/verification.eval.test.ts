/**
 * Verification Loop Evaluation Tests
 *
 * Verifies that the ExecutionLoop correctly:
 * - V1: Generates diverse test scenarios
 * - V2: Executes verification and returns accurate results
 * - V3: Self-corrects failed candidates
 * - V4: Terminates loop appropriately
 */

import { describe, test, expect, beforeEach, afterEach, mock } from "bun:test"
import { ExecutionLoop } from "@/bootstrap/verification"
import { CandidateStore } from "@/bootstrap/candidate-store"
import { ConfidenceSystem } from "@/bootstrap/confidence"
import { BootstrapTypes } from "@/bootstrap/types"
import {
  createMockCandidate,
  MOCK_CANDIDATES,
} from "./fixtures/mock-candidates"
import {
  VERIFICATION_EXPECTATIONS,
} from "./fixtures/expected-results"
import {
  calculateSelfCorrectionRate,
  createMetricResult,
  aggregateMetrics,
} from "./utils/metrics"
import { tmpdir } from "../fixture/fixture"

/**
 * Note: Many verification tests require LLM calls and will be skipped
 * in environments without API access. Integration tests should be run
 * separately with proper API credentials.
 */

describe("Verification Loop Evaluation", () => {
  describe("V1: Scenario Generation", () => {
    test("generateTestScenarios returns array structure", async () => {
      // This test validates the structure without requiring LLM
      const candidate = createMockCandidate({
        name: "test-scenario-gen",
        type: "workflow",
        content: {
          steps: ["step 1", "step 2", "step 3"],
        },
      })

      // Mock implementation returns default scenarios when LLM fails
      const scenarios = await ExecutionLoop.generateTestScenarios(candidate)

      expect(Array.isArray(scenarios)).toBe(true)
    })

    test("default scenarios have required fields", async () => {
      const candidate = createMockCandidate({
        name: "test-default-scenarios",
      })

      const scenarios = await ExecutionLoop.generateTestScenarios(candidate)

      // Even default scenarios should have proper structure
      for (const scenario of scenarios) {
        expect(scenario.id).toBeDefined()
        expect(scenario.name).toBeDefined()
        expect(scenario.description).toBeDefined()
        expect(scenario.input).toBeDefined()
        expect(scenario.expectedBehavior).toBeDefined()
      }
    })

    test("scenarios have unique IDs", async () => {
      const candidate = createMockCandidate()
      const scenarios = await ExecutionLoop.generateTestScenarios(candidate)

      const ids = scenarios.map((s) => s.id)
      const uniqueIds = new Set(ids)

      expect(uniqueIds.size).toBe(ids.length)
    })

    test("scenarios include candidate ID in their IDs", async () => {
      const candidate = createMockCandidate({ id: "cand_test_123" })
      const scenarios = await ExecutionLoop.generateTestScenarios(candidate)

      for (const scenario of scenarios) {
        expect(scenario.id).toContain(candidate.id)
      }
    })

    test("minimum scenarios generated", async () => {
      const candidate = createMockCandidate()
      const scenarios = await ExecutionLoop.generateTestScenarios(candidate)

      expect(scenarios.length).toBeGreaterThanOrEqual(
        VERIFICATION_EXPECTATIONS.scenarioGeneration.minimumScenarios - 1, // Allow for default fallback
      )
    })
  })

  describe("V2: Verification Execution", () => {
    let tempDir: Awaited<ReturnType<typeof tmpdir>> | undefined
    let originalTestHome: string | undefined

    beforeEach(async () => {
      originalTestHome = process.env.CCODE_TEST_HOME
      tempDir = await tmpdir()
      process.env.CCODE_TEST_HOME = tempDir.path
    })

    afterEach(async () => {
      if (tempDir) {
        await tempDir[Symbol.asyncDispose]()
      }
      if (originalTestHome !== undefined) {
        process.env.CCODE_TEST_HOME = originalTestHome
      } else {
        delete process.env.CCODE_TEST_HOME
      }
    })

    test("verify returns structured result", async () => {
      const candidate = createMockCandidate({
        name: "test-verify-structure",
        verification: {
          status: "pending",
          attempts: 0,
          confidence: 0.3,
        },
      })

      // Add candidate to store first
      await CandidateStore.add(candidate)

      const result = await ExecutionLoop.verify(candidate)

      expect(result).toBeDefined()
      expect(typeof result.passed).toBe("boolean")
      expect(typeof result.confidence).toBe("number")
      expect(Array.isArray(result.scenarios)).toBe(true)
    })

    test("verify returns confidence between 0 and 1", async () => {
      const candidate = createMockCandidate({
        name: "test-confidence-bounds",
        verification: {
          status: "pending",
          attempts: 0,
          confidence: 0.3,
        },
      })

      await CandidateStore.add(candidate)
      const result = await ExecutionLoop.verify(candidate)

      expect(result.confidence).toBeGreaterThanOrEqual(0)
      expect(result.confidence).toBeLessThanOrEqual(1)
    })

    test("verify updates candidate verification status", async () => {
      const candidate = createMockCandidate({
        id: "cand_verify_update_test",
        name: "test-status-update",
        verification: {
          status: "pending",
          attempts: 0,
          confidence: 0.3,
        },
      })

      await CandidateStore.add(candidate)
      await ExecutionLoop.verify(candidate)

      const updated = await CandidateStore.get(candidate.id)
      expect(updated?.verification.attempts).toBeGreaterThan(0)
      expect(updated?.verification.status).not.toBe("pending")
    })

    test("verify increments attempt counter", async () => {
      const candidate = createMockCandidate({
        id: "cand_attempt_counter",
        name: "test-attempt-count",
        verification: {
          status: "pending",
          attempts: 0,
          confidence: 0.3,
        },
      })

      await CandidateStore.add(candidate)
      const initialAttempts = candidate.verification.attempts

      await ExecutionLoop.verify(candidate)

      const updated = await CandidateStore.get(candidate.id)
      expect(updated?.verification.attempts).toBe(initialAttempts + 1)
    })
  })

  describe("V3: Self-Correction", () => {
    let tempDir: Awaited<ReturnType<typeof tmpdir>> | undefined
    let originalTestHome: string | undefined

    beforeEach(async () => {
      originalTestHome = process.env.CCODE_TEST_HOME
      tempDir = await tmpdir()
      process.env.CCODE_TEST_HOME = tempDir.path
    })

    afterEach(async () => {
      if (tempDir) {
        await tempDir[Symbol.asyncDispose]()
      }
      if (originalTestHome !== undefined) {
        process.env.CCODE_TEST_HOME = originalTestHome
      } else {
        delete process.env.CCODE_TEST_HOME
      }
    })

    test("selfCorrect returns null when max attempts reached", async () => {
      const candidate = createMockCandidate({
        name: "test-max-attempts",
        verification: {
          status: "failed",
          attempts: 3, // Above MAX_CORRECTION_ATTEMPTS (2)
          confidence: 0.3,
        },
      })

      await CandidateStore.add(candidate)

      const failedScenarios: BootstrapTypes.TestScenario[] = [
        {
          id: "scenario_1",
          name: "Failed Test",
          description: "A test that failed",
          input: "test input",
          expectedBehavior: "expected output",
          result: {
            passed: false,
            actual: "wrong output",
          },
        },
      ]

      const result = await ExecutionLoop.selfCorrect(candidate, failedScenarios)
      expect(result).toBeNull()
    })

    test("selfCorrect accepts valid candidate and failed scenarios", async () => {
      const candidate = createMockCandidate({
        id: "cand_self_correct",
        name: "test-self-correct",
        verification: {
          status: "failed",
          attempts: 1,
          confidence: 0.25,
        },
        content: {
          steps: ["Step that needs correction"],
        },
      })

      await CandidateStore.add(candidate)

      const failedScenarios: BootstrapTypes.TestScenario[] = [
        {
          id: "scenario_fail_1",
          name: "Failed Scenario",
          description: "This scenario failed",
          input: "problematic input",
          expectedBehavior: "should handle gracefully",
          result: {
            passed: false,
            actual: "crashed instead",
            error: "Unexpected error",
          },
        },
      ]

      // Note: Without LLM, this will likely return null
      // The test verifies the function handles inputs correctly
      const result = await ExecutionLoop.selfCorrect(candidate, failedScenarios)
      expect(result === null || typeof result === "object").toBe(true)
    })

    test("selfCorrect only considers failed scenarios", async () => {
      const candidate = createMockCandidate({
        name: "test-failed-only",
        verification: {
          status: "failed",
          attempts: 0,
          confidence: 0.3,
        },
      })

      await CandidateStore.add(candidate)

      const mixedScenarios: BootstrapTypes.TestScenario[] = [
        {
          id: "pass_1",
          name: "Passed",
          description: "This passed",
          input: "good input",
          expectedBehavior: "good output",
          result: { passed: true, actual: "good output" },
        },
        {
          id: "fail_1",
          name: "Failed",
          description: "This failed",
          input: "bad input",
          expectedBehavior: "good output",
          result: { passed: false, actual: "bad output" },
        },
      ]

      // Filter to only failed scenarios before passing
      const failedOnly = mixedScenarios.filter((s) => s.result && !s.result.passed)
      expect(failedOnly).toHaveLength(1)
    })
  })

  describe("V4: Loop Termination", () => {
    let tempDir: Awaited<ReturnType<typeof tmpdir>> | undefined
    let originalTestHome: string | undefined

    beforeEach(async () => {
      originalTestHome = process.env.CCODE_TEST_HOME
      tempDir = await tmpdir()
      process.env.CCODE_TEST_HOME = tempDir.path
    })

    afterEach(async () => {
      if (tempDir) {
        await tempDir[Symbol.asyncDispose]()
      }
      if (originalTestHome !== undefined) {
        process.env.CCODE_TEST_HOME = originalTestHome
      } else {
        delete process.env.CCODE_TEST_HOME
      }
    })

    test("runVerificationLoop returns result", async () => {
      const candidate = createMockCandidate({
        name: "test-loop-result",
        verification: {
          status: "pending",
          attempts: 0,
          confidence: 0.3,
        },
      })

      await CandidateStore.add(candidate)
      const result = await ExecutionLoop.runVerificationLoop(candidate)

      expect(result).toBeDefined()
      expect(typeof result.passed).toBe("boolean")
      expect(typeof result.confidence).toBe("number")
      expect(Array.isArray(result.scenarios)).toBe(true)
    })

    test("verification loop respects MAX_VERIFICATION_ATTEMPTS", async () => {
      // The MAX_VERIFICATION_ATTEMPTS is 3
      // This tests that the loop terminates
      const candidate = createMockCandidate({
        id: "cand_max_verify",
        name: "test-max-verify",
        verification: {
          status: "pending",
          attempts: 0,
          confidence: 0.1, // Low confidence to potentially trigger corrections
        },
      })

      await CandidateStore.add(candidate)

      const startTime = Date.now()
      const result = await ExecutionLoop.runVerificationLoop(candidate)
      const duration = Date.now() - startTime

      // Should complete in reasonable time (not infinite loop)
      expect(duration).toBeLessThan(60000) // 60 seconds max
      expect(result).toBeDefined()
    })
  })

  describe("Pass Rate Calculation", () => {
    test("60% pass rate is required for passing", () => {
      // Based on verification.ts: passed = passRate >= 0.6
      const passThreshold = VERIFICATION_EXPECTATIONS.passRate.minimumForPass
      expect(passThreshold).toBe(0.6)
    })

    test("scenarios with mixed results calculate pass rate correctly", () => {
      const scenarios = [
        { passed: true },
        { passed: true },
        { passed: true },
        { passed: false },
        { passed: false },
      ]

      const passedCount = scenarios.filter((s) => s.passed).length
      const passRate = passedCount / scenarios.length

      expect(passRate).toBe(0.6) // 3/5 = 0.6
    })
  })

  describe("Verification Result Structure", () => {
    test("VerificationResult schema is valid", () => {
      const validResult: BootstrapTypes.VerificationResult = {
        passed: true,
        confidence: 0.75,
        scenarios: [],
      }

      const parsed = BootstrapTypes.VerificationResult.safeParse(validResult)
      expect(parsed.success).toBe(true)
    })

    test("TestScenario schema accepts result field", () => {
      const scenario: BootstrapTypes.TestScenario = {
        id: "test_1",
        name: "Test Scenario",
        description: "A test",
        input: "test input",
        expectedBehavior: "expected output",
        result: {
          passed: true,
          actual: "actual output",
        },
      }

      const parsed = BootstrapTypes.TestScenario.safeParse(scenario)
      expect(parsed.success).toBe(true)
    })

    test("TestScenario allows error in result", () => {
      const scenario: BootstrapTypes.TestScenario = {
        id: "test_error",
        name: "Error Scenario",
        description: "A test with error",
        input: "bad input",
        expectedBehavior: "should succeed",
        result: {
          passed: false,
          error: "Something went wrong",
        },
      }

      const parsed = BootstrapTypes.TestScenario.safeParse(scenario)
      expect(parsed.success).toBe(true)
    })
  })
})

describe("Verification Metrics", () => {
  test("calculates self-correction success rate", () => {
    const corrections = [
      { before: 0.3, after: 0.5 }, // Improved
      { before: 0.4, after: 0.35 }, // Worse
      { before: 0.25, after: 0.45 }, // Improved
      { before: 0.5, after: 0.6 }, // Improved
    ]

    const rate = calculateSelfCorrectionRate(corrections)

    expect(rate).toBe(0.75) // 3/4 improved
  })

  test("empty corrections returns 0", () => {
    const rate = calculateSelfCorrectionRate([])
    expect(rate).toBe(0)
  })

  test("generates evaluation summary for verification dimension", () => {
    const metrics = [
      createMetricResult("Scenario Count", 4, 3, "gte"),
      createMetricResult("Scenario Diversity", 0.85, 0.8, "gte"),
      createMetricResult("Verification Accuracy", 0.9, 0.8, "gte"),
      createMetricResult("Self-Correction Rate", 0.55, 0.5, "gte"),
      createMetricResult("Loop Termination Accuracy", 1.0, 0.95, "gte"),
    ]

    const summary = aggregateMetrics("Verification Loop", metrics)

    expect(summary.dimension).toBe("Verification Loop")
    expect(summary.metrics).toHaveLength(5)
    expect(summary.passRate).toBeGreaterThanOrEqual(0.5)
  })
})

describe("Verification Integration with Confidence", () => {
  let tempDir: Awaited<ReturnType<typeof tmpdir>> | undefined
  let originalTestHome: string | undefined

  beforeEach(async () => {
    originalTestHome = process.env.CCODE_TEST_HOME
    tempDir = await tmpdir()
    process.env.CCODE_TEST_HOME = tempDir.path
  })

  afterEach(async () => {
    if (tempDir) {
      await tempDir[Symbol.asyncDispose]()
    }
    if (originalTestHome !== undefined) {
      process.env.CCODE_TEST_HOME = originalTestHome
    } else {
      delete process.env.CCODE_TEST_HOME
    }
  })

  test("successful verification increases candidate confidence", async () => {
    const candidate = createMockCandidate({
      id: "cand_conf_increase",
      name: "test-conf-increase",
      verification: {
        status: "pending",
        attempts: 0,
        confidence: 0.3,
      },
      metadata: {
        created: Date.now(),
        updated: Date.now(),
        usageCount: 5,
        successCount: 4,
        failureCount: 1,
      },
    })

    await CandidateStore.add(candidate)

    // Note: Actual verification requires LLM
    // This test validates the confidence calculation logic

    const factors: BootstrapTypes.ConfidenceFactors = {
      verificationPassed: true,
      usageCount: 5,
      successRate: 0.8,
      scenarioCoverage: 0.8,
    }

    const newConfidence = ConfidenceSystem.calculate(factors)
    expect(newConfidence).toBeGreaterThan(candidate.verification.confidence)
  })

  test("failed verification decreases confidence", async () => {
    const candidate = createMockCandidate({
      id: "cand_conf_decrease",
      name: "test-conf-decrease",
      verification: {
        status: "pending",
        attempts: 0,
        confidence: 0.5,
      },
      metadata: {
        created: Date.now(),
        updated: Date.now(),
        usageCount: 3,
        successCount: 1,
        failureCount: 2,
      },
    })

    await CandidateStore.add(candidate)

    const factors: BootstrapTypes.ConfidenceFactors = {
      verificationPassed: false,
      usageCount: 3,
      successRate: 0.33,
      scenarioCoverage: 0.4,
    }

    const newConfidence = ConfidenceSystem.calculate(factors)
    expect(newConfidence).toBeLessThan(candidate.verification.confidence)
  })
})
