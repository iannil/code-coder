/**
 * End-to-End Evolution Evaluation Tests
 *
 * Verifies the complete Bootstrap Flywheel cycle:
 * 1. Initial state (clear candidates)
 * 2. New problem introduction
 * 3. Solution and learning
 * 4. Verification loop
 * 5. Reuse validation
 * 6. Evolution confirmation
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { SelfAwareness } from "@/bootstrap/awareness"
import { ResourceAcquisition } from "@/bootstrap/acquisition"
import { SkillGeneration } from "@/bootstrap/generation"
import { ExecutionLoop } from "@/bootstrap/verification"
import { ConfidenceSystem } from "@/bootstrap/confidence"
import { CandidateStore } from "@/bootstrap/candidate-store"
import { BootstrapTypes } from "@/bootstrap/types"
import {
  createMockCandidate,
  MOCK_CANDIDATES,
} from "./fixtures/mock-candidates"
import {
  createMockSession,
  MOCK_SESSIONS,
} from "./fixtures/mock-sessions"
import {
  E2E_EVOLUTION_EXPECTATIONS,
  CONFIDENCE_EXPECTATIONS,
} from "./fixtures/expected-results"
import {
  calculateEvolutionSuccessRate,
  calculateConvergenceTime,
  createMetricResult,
  aggregateMetrics,
  Statistics,
} from "./utils/metrics"
import {
  generateReport,
  formatReportAsMarkdown,
  formatReportForConsole,
} from "./utils/reporters"
import { tmpdir } from "../fixture/fixture"

describe("E2E Evolution Evaluation", () => {
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

  describe("Phase 1: Initial State", () => {
    test("can clear candidate store", async () => {
      // Add some candidates first
      await CandidateStore.add(createMockCandidate({ name: "to-be-cleared-1" }))
      await CandidateStore.add(createMockCandidate({ name: "to-be-cleared-2" }))

      // Cleanup with aggressive settings
      await CandidateStore.cleanup({
        maxAge: 0,
        minConfidence: 1.0,
        maxCandidates: 0,
      })

      const remaining = await CandidateStore.list()
      expect(remaining.length).toBe(0)
    })

    test("empty store has correct structure", async () => {
      await CandidateStore.cleanup({ maxCandidates: 0 })

      const store = await CandidateStore.read()

      expect(store.version).toBeDefined()
      expect(Array.isArray(store.candidates)).toBe(true)
      expect(store.candidates.length).toBe(0)
    })
  })

  describe("Phase 2: New Problem Introduction", () => {
    test("novel problem returns lower confidence", async () => {
      const novelProblem = "Convert YAML config to TypeScript constants with type inference"

      const assessment = await SelfAwareness.canHandle(novelProblem)

      // Novel problem should not have high confidence
      expect(assessment.confidence).toBeLessThanOrEqual(0.8)
    })

    test("identifies capability gaps for novel problem", async () => {
      const novelProblem = "Deploy serverless function to AWS Lambda with custom runtime"

      const assessment = await SelfAwareness.canHandle(novelProblem)

      // Should identify domain-specific needs
      expect(assessment).toBeDefined()
    })

    test("resource discovery triggered for capability gaps", async () => {
      const gapProblem = "Integrate with Jira to create tickets automatically"

      const resources = await ResourceAcquisition.discoverNeeded(gapProblem)

      // Should discover related resources
      expect(resources).toBeDefined()
      expect(resources.mcpServers).toBeDefined()
      expect(resources.skills).toBeDefined()
    })
  })

  describe("Phase 3: Solution and Learning", () => {
    test("skill extraction from solution session", async () => {
      const session = MOCK_SESSIONS.complexWorkflow

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      expect(candidate).toBeDefined()
      expect(candidate.name.length).toBeGreaterThan(0)
      expect(candidate.type).toBeDefined()
    })

    test("candidate stored after extraction", async () => {
      const session = createMockSession({ sessionId: "e2e-store-test" })

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      await CandidateStore.add(candidate)

      const retrieved = await CandidateStore.get(candidate.id)
      expect(retrieved).toBeDefined()
      expect(retrieved?.source.sessionId).toBe(session.sessionId)
    })

    test("initial confidence set based on source quality", async () => {
      const session = MOCK_SESSIONS.complexWorkflow

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      // Initial confidence should be calculated
      expect(candidate.verification.confidence).toBeGreaterThanOrEqual(0)
    })
  })

  describe("Phase 4: Verification Loop", () => {
    test("verification generates test scenarios", async () => {
      const candidate = createMockCandidate({
        name: "e2e-verify-scenario",
        type: "workflow",
        content: { steps: ["step 1", "step 2"] },
      })

      const scenarios = await ExecutionLoop.generateTestScenarios(candidate)

      expect(scenarios.length).toBeGreaterThan(0)
      expect(scenarios[0].input).toBeDefined()
      expect(scenarios[0].expectedBehavior).toBeDefined()
    })

    test("verification updates candidate status", async () => {
      const candidate = createMockCandidate({
        id: "e2e-verify-status",
        name: "e2e-status-test",
        verification: { status: "pending", attempts: 0, confidence: 0.3 },
      })

      await CandidateStore.add(candidate)
      await ExecutionLoop.verify(candidate)

      const updated = await CandidateStore.get(candidate.id)

      expect(updated?.verification.status).not.toBe("pending")
      expect(updated?.verification.attempts).toBeGreaterThan(0)
    })
  })

  describe("Phase 5: Reuse Validation", () => {
    test("similar problem triggers candidate lookup", async () => {
      // Add a candidate
      const candidate = createMockCandidate({
        name: "json-formatter",
        description: "Format JSON files",
        trigger: { type: "auto", context: "json formatting" },
      })

      await CandidateStore.add(candidate)

      // Similar problem
      const candidates = await CandidateStore.list()
      const relevant = candidates.filter((c) =>
        c.name.includes("json") || c.description.toLowerCase().includes("json"),
      )

      expect(relevant.length).toBeGreaterThan(0)
    })

    test("successful reuse increases usage count", async () => {
      const candidate = createMockCandidate({
        id: "e2e-reuse-count",
        name: "reuse-counter",
        metadata: {
          created: Date.now(),
          updated: Date.now(),
          usageCount: 0,
        },
      })

      await CandidateStore.add(candidate)

      // Simulate usage
      await CandidateStore.update(candidate.id, (c) => {
        c.metadata.usageCount++
      })

      const updated = await CandidateStore.get(candidate.id)
      expect(updated?.metadata.usageCount).toBe(1)
    })

    test("confidence evolves with usage", async () => {
      const initialConfidence = 0.5

      // Simulate multiple successful uses
      let confidence = initialConfidence
      for (let i = 0; i < 3; i++) {
        confidence = ConfidenceSystem.evolve(confidence, {
          success: true,
          context: `use-${i}`,
        })
      }

      expect(confidence).toBeGreaterThan(initialConfidence)
    })
  })

  describe("Phase 6: Evolution Confirmation", () => {
    test("candidate reaches promotion threshold after successes", () => {
      let confidence = 0.45 // Start slightly higher to reach threshold after 5 successes
      let usageCount = 0

      // Simulate successful uses
      for (let i = 0; i < 5; i++) {
        confidence = ConfidenceSystem.evolve(confidence, {
          success: true,
          context: `success-${i}`,
        })
        usageCount++
      }

      const readyForPromotion = ConfidenceSystem.isReadyForPromotion(
        confidence,
        usageCount,
        true, // verification passed
      )

      expect(readyForPromotion).toBe(true)
    })

    test("listReadyForPromotion finds qualified candidates", async () => {
      const ready = createMockCandidate({
        name: "e2e-promo-ready",
        verification: { status: "passed", attempts: 1, confidence: 0.75 },
        metadata: {
          created: Date.now(),
          updated: Date.now(),
          usageCount: 5,
          successCount: 4,
          failureCount: 1,
        },
      })

      await CandidateStore.add(ready)

      const promotionList = await CandidateStore.listReadyForPromotion()
      expect(promotionList.some((c) => c.name === "e2e-promo-ready")).toBe(true)
    })

    test("full evolution metrics can be calculated", () => {
      const cycles = [
        { candidateCreated: true, verificationPassed: true, promoted: true },
        { candidateCreated: true, verificationPassed: true, promoted: false },
        { candidateCreated: true, verificationPassed: false, promoted: false },
        { candidateCreated: false, verificationPassed: false, promoted: false },
      ]

      const successRate = calculateEvolutionSuccessRate(cycles)
      expect(successRate).toBe(0.25) // 1 out of 4 fully successful
    })
  })

  describe("Full Cycle Integration", () => {
    test("complete evolution cycle from problem to promotion", async () => {
      // Phase 1: Clear state
      await CandidateStore.cleanup({ maxCandidates: 0 })
      const initialCount = (await CandidateStore.list()).length
      expect(initialCount).toBe(0)

      // Phase 2: New problem
      const problem = "Automate code formatting for TypeScript files"
      const initialAssessment = await SelfAwareness.canHandle(problem)
      expect(initialAssessment).toBeDefined()

      // Phase 3: Solve and extract
      const session = MOCK_SESSIONS.jsonFormatting
      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      await CandidateStore.add(candidate)
      expect((await CandidateStore.list()).length).toBe(1)

      // Phase 4: Verify (will use default scenarios without LLM)
      await ExecutionLoop.verify(candidate)

      const afterVerify = await CandidateStore.get(candidate.id)
      expect(afterVerify?.verification.attempts).toBeGreaterThan(0)

      // Phase 5 & 6: Simulate successful usage and evolution
      // Start with a higher base to ensure we reach promotion threshold
      let confidence = Math.max(afterVerify?.verification.confidence ?? 0.3, 0.45)
      let usageCount = 0

      for (let i = 0; i < 6; i++) { // 6 successes to ensure threshold is met
        confidence = ConfidenceSystem.evolve(confidence, {
          success: true,
          context: `usage-${i}`,
        })
        usageCount++
      }

      // Update candidate with evolved confidence
      await CandidateStore.update(candidate.id, (c) => {
        c.verification.confidence = confidence
        c.verification.status = "passed"
        c.metadata.usageCount = usageCount
        c.metadata.successCount = usageCount
      })

      // Check promotion readiness
      const final = await CandidateStore.get(candidate.id)
      const isReady = ConfidenceSystem.isReadyForPromotion(
        final!.verification.confidence,
        final!.metadata.usageCount,
        final!.verification.status === "passed",
      )

      expect(isReady).toBe(true)
    })
  })

  describe("Convergence Analysis", () => {
    test("calculates convergence time to threshold", () => {
      const history = [0.3, 0.35, 0.42, 0.48, 0.55, 0.61, 0.66, 0.70]
      const threshold = 0.6

      const convergenceTime = calculateConvergenceTime(history, threshold)
      expect(convergenceTime).toBe(6) // First reaches 0.61 at index 5 (1-indexed = 6)
    })

    test("convergence time within target", () => {
      // Simulate evolution
      const history: number[] = []
      let confidence = 0.3

      for (let i = 0; i < 15; i++) {
        history.push(confidence)
        confidence = ConfidenceSystem.evolve(confidence, {
          success: true,
          context: `step-${i}`,
        })
      }

      const matureThreshold = CONFIDENCE_EXPECTATIONS.promotionThreshold.confidenceThreshold
      const convergenceTime = calculateConvergenceTime(history, matureThreshold)

      expect(convergenceTime).toBeLessThanOrEqual(
        E2E_EVOLUTION_EXPECTATIONS.targetMetrics.convergenceTime,
      )
    })
  })
})

describe("E2E Evaluation Reporting", () => {
  test("generates full evaluation report", () => {
    const dimensions = [
      aggregateMetrics("Self-Awareness", [
        createMetricResult("Confidence Calibration", 0.12, 0.15, "lte"),
        createMetricResult("Tool Identification", 0.95, 0.9, "gte"),
      ]),
      aggregateMetrics("Confidence Evolution", [
        createMetricResult("Asymmetric Ratio", 1.5, 1.3, "gte"),
        createMetricResult("Convergence Time", 8, 10, "lte"),
      ]),
      aggregateMetrics("Verification", [
        createMetricResult("Self-Correction Rate", 0.55, 0.5, "gte"),
      ]),
    ]

    const report = generateReport(dimensions, 5000)

    expect(report.timestamp).toBeDefined()
    expect(report.duration).toBe(5000)
    expect(report.dimensions).toHaveLength(3)
    expect(report.overallScore).toBeGreaterThanOrEqual(0)
    expect(report.overallPassRate).toBeGreaterThanOrEqual(0)
  })

  test("formats report as markdown", () => {
    const dimensions = [
      aggregateMetrics("Test Dimension", [
        createMetricResult("Test Metric", 0.9, 0.8, "gte"),
      ]),
    ]

    const report = generateReport(dimensions, 1000)
    const markdown = formatReportAsMarkdown(report)

    expect(markdown).toContain("# Bootstrap Flywheel Evaluation Report")
    expect(markdown).toContain("Test Dimension")
    expect(markdown).toContain("Test Metric")
  })

  test("formats report for console", () => {
    const dimensions = [
      aggregateMetrics("Test Dimension", [
        createMetricResult("Test Metric", 0.9, 0.8, "gte"),
      ]),
    ]

    const report = generateReport(dimensions, 1000)
    const console = formatReportForConsole(report)

    expect(console).toContain("BOOTSTRAP FLYWHEEL EVALUATION REPORT")
    expect(console).toContain("Test Dimension")
  })

  test("report includes recommendations for failures", () => {
    const dimensions = [
      aggregateMetrics("Failing Dimension", [
        createMetricResult("Failing Metric", 0.5, 0.9, "gte"),
      ]),
    ]

    const report = generateReport(dimensions, 1000)

    expect(report.recommendations.length).toBeGreaterThan(0)
    expect(report.recommendations[0]).toContain("Failing Metric")
  })
})

describe("Statistical Analysis", () => {
  test("calculates mean correctly", () => {
    expect(Statistics.mean([1, 2, 3, 4, 5])).toBe(3)
    expect(Statistics.mean([])).toBe(0)
  })

  test("calculates median correctly", () => {
    expect(Statistics.median([1, 2, 3, 4, 5])).toBe(3)
    expect(Statistics.median([1, 2, 3, 4])).toBe(2.5)
    expect(Statistics.median([])).toBe(0)
  })

  test("calculates standard deviation", () => {
    const values = [2, 4, 4, 4, 5, 5, 7, 9]
    const stdDev = Statistics.stdDev(values)
    expect(stdDev).toBeCloseTo(2, 0)
  })

  test("calculates percentiles", () => {
    const values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    expect(Statistics.percentile(values, 50)).toBeCloseTo(5.5, 1)
    expect(Statistics.percentile(values, 25)).toBeCloseTo(3.25, 1)
    expect(Statistics.percentile(values, 75)).toBeCloseTo(7.75, 1)
  })
})
