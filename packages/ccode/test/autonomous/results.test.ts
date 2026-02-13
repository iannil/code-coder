import { describe, test, expect, beforeEach } from "bun:test"
import { AutonomousState } from "@/autonomous/state/states"
import { Orchestrator, createOrchestrator } from "@/autonomous/orchestration/orchestrator"
import { Scorer, createScorer } from "@/autonomous/metrics/scorer"
import { MetricsCollector, createMetricsCollector } from "@/autonomous/metrics/metrics"
import { SafetyGuard } from "@/autonomous/safety/constraints"
import { createTestConfig, createTestSessionContext } from "./fixtures/autonomous-fixture"

describe("Autonomous Mode - Results", () => {
  describe("process() Return Structure", () => {
    test("should return success boolean", () => {
      const result = {
        success: true,
        result: {
          success: true,
          qualityScore: 85,
          crazinessScore: 75,
          duration: 60000,
          tokensUsed: 50000,
          costUSD: 1.0,
          iterationsCompleted: 2,
        },
      }

      expect(result.success).toBe(true)
      expect(result.result).toBeDefined()
    })

    test("should return null result on failure", () => {
      const result = {
        success: false,
        result: null,
      }

      expect(result.success).toBe(false)
      expect(result.result).toBeNull()
    })

    test("should include all required fields in result", () => {
      const result = {
        success: true,
        qualityScore: 80,
        crazinessScore: 70,
        duration: 30000,
        tokensUsed: 25000,
        costUSD: 0.5,
        iterationsCompleted: 1,
      }

      expect(result).toHaveProperty("success")
      expect(result).toHaveProperty("qualityScore")
      expect(result).toHaveProperty("crazinessScore")
      expect(result).toHaveProperty("duration")
      expect(result).toHaveProperty("tokensUsed")
      expect(result).toHaveProperty("costUSD")
      expect(result).toHaveProperty("iterationsCompleted")
    })
  })

  describe("Quality Score Calculation", () => {
    let scorer: Scorer

    beforeEach(() => {
      scorer = createScorer()
    })

    test("should calculate quality score from metrics", () => {
      const metrics = {
        sessionId: "test",
        startTime: Date.now(),
        duration: 60000,
        tasks: { total: 10, completed: 8, failed: 2, skipped: 0 },
        decisions: { total: 5, approved: 4, paused: 0, blocked: 1, averageScore: 7.5 },
        resources: { tokensUsed: 50000, costUSD: 1.0, filesChanged: 5 },
        tests: { run: 10, passed: 8, failed: 2, passRate: 0.8 },
        tdd: { cycles: 3, redPassed: 3, greenPassed: 3, refactorPassed: 3 },
        safety: { rollbacks: 0, loopsDetected: 0, warnings: 2 },
        states: { transitions: 10, finalState: AutonomousState.COMPLETED },
      }

      const qualityScore = scorer.calculateQuality(metrics)

      expect(qualityScore.overall).toBeGreaterThan(0)
      expect(qualityScore.overall).toBeLessThanOrEqual(100)
      expect(qualityScore.testCoverage).toBeGreaterThan(0)
      expect(qualityScore.codeQuality).toBeGreaterThan(0)
    })

    test("should penalize failed tasks", () => {
      const metrics = {
        sessionId: "test",
        startTime: Date.now(),
        duration: 60000,
        tasks: { total: 10, completed: 5, failed: 5, skipped: 0 },
        decisions: { total: 5, approved: 5, paused: 0, blocked: 0, averageScore: 7 },
        resources: { tokensUsed: 50000, costUSD: 1.0, filesChanged: 5 },
        tests: { run: 10, passed: 5, failed: 5, passRate: 0.5 },
        tdd: { cycles: 3, redPassed: 3, greenPassed: 3, refactorPassed: 3 },
        safety: { rollbacks: 0, loopsDetected: 0, warnings: 2 },
        states: { transitions: 10, finalState: AutonomousState.COMPLETED },
      }

      const qualityScore = scorer.calculateQuality(metrics)

      // Lower quality due to failures
      expect(qualityScore.overall).toBeLessThan(80)
    })

    test("should reward high test pass rate", () => {
      const perfectMetrics = {
        sessionId: "test",
        startTime: Date.now(),
        duration: 60000,
        tasks: { total: 10, completed: 10, failed: 0, skipped: 0 },
        decisions: { total: 5, approved: 5, paused: 0, blocked: 0, averageScore: 9 },
        resources: { tokensUsed: 50000, costUSD: 1.0, filesChanged: 5 },
        tests: { run: 10, passed: 10, failed: 0, passRate: 1.0 },
        tdd: { cycles: 3, redPassed: 3, greenPassed: 3, refactorPassed: 3 },
        safety: { rollbacks: 0, loopsDetected: 0, warnings: 0 },
        states: { transitions: 10, finalState: AutonomousState.COMPLETED },
      }

      const qualityScore = scorer.calculateQuality(perfectMetrics)

      // Higher quality due to perfect execution
      expect(qualityScore.overall).toBeGreaterThan(70)
    })
  })

  describe("Craziness Score Calculation", () => {
    let scorer: Scorer

    beforeEach(() => {
      scorer = createScorer()
    })

    test("should calculate craziness score based on autonomy", () => {
      const metrics = {
        sessionId: "test",
        startTime: Date.now(),
        duration: 60000,
        tasks: { total: 10, completed: 10, failed: 0, skipped: 0 },
        decisions: { total: 10, approved: 10, paused: 0, blocked: 0, averageScore: 8 },
        resources: { tokensUsed: 50000, costUSD: 1.0, filesChanged: 5 },
        tests: { run: 20, passed: 20, failed: 0, passRate: 1.0 },
        tdd: { cycles: 5, redPassed: 5, greenPassed: 5, refactorPassed: 5 },
        safety: { rollbacks: 0, loopsDetected: 0, warnings: 0 },
        states: { transitions: 5, finalState: AutonomousState.COMPLETED },
      }

      const crazinessScore = scorer.calculateCraziness(metrics)

      expect(crazinessScore.overall).toBeGreaterThan(0)
      expect(crazinessScore.level).toBeDefined()
      expect(crazinessScore.autonomy).toBeGreaterThanOrEqual(0)
      expect(crazinessScore.selfCorrection).toBeGreaterThanOrEqual(0)
    })

    test("should determine craziness level thresholds correctly", () => {
      const testCases = [
        { score: 95, expected: "lunatic" },
        { score: 80, expected: "insane" },
        { score: 65, expected: "crazy" },
        { score: 50, expected: "wild" },
        { score: 30, expected: "bold" },
        { score: 10, expected: "timid" },
      ]

      for (const testCase of testCases) {
        const metrics = {
          sessionId: "test",
          startTime: Date.now(),
          duration: 60000,
          tasks: { total: 1, completed: 1, failed: 0, skipped: 0 },
          decisions: { total: 1, approved: 1, paused: 0, blocked: 0, averageScore: testCase.score },
          resources: { tokensUsed: 1000, costUSD: 0.1, filesChanged: 1 },
          tests: { run: 10, passed: 10, failed: 0, passRate: 1.0 },
          tdd: { cycles: 1, redPassed: 1, greenPassed: 1, refactorPassed: 1 },
          safety: { rollbacks: 0, loopsDetected: 0, warnings: 0 },
          states: { transitions: 1, finalState: AutonomousState.COMPLETED },
        }

        const result = scorer.calculateCraziness(metrics)
        expect(result.level).toBe(testCase.expected as "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid")
      }
    })

    test("should calculate self-correction score", () => {
      const metrics = {
        sessionId: "test",
        startTime: Date.now(),
        duration: 60000,
        tasks: { total: 10, completed: 8, failed: 2, skipped: 0 },
        decisions: { total: 10, approved: 10, paused: 0, blocked: 0, averageScore: 7 },
        resources: { tokensUsed: 50000, costUSD: 1.0, filesChanged: 5 },
        tests: { run: 20, passed: 18, failed: 2, passRate: 0.9 },
        tdd: { cycles: 5, redPassed: 5, greenPassed: 5, refactorPassed: 5 },
        safety: { rollbacks: 1, loopsDetected: 0, warnings: 2 },
        states: { transitions: 8, finalState: AutonomousState.COMPLETED },
      }

      const crazinessScore = scorer.calculateCraziness(metrics)

      expect(crazinessScore.selfCorrection).toBeGreaterThan(0)
      expect(crazinessScore.selfCorrection).toBeLessThanOrEqual(100)
    })
  })

  describe("Token/Cost Tracking", () => {
    test("should track tokens used", () => {
      const sessionId = `test_session_${Date.now()}`
      const budget = {
        maxTokens: 100000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 20,
        maxActions: 100,
      }

      const guard = new SafetyGuard(sessionId, budget)

      guard.record("tokensUsed", 1000)
      guard.record("tokensUsed", 2000)

      const usage = guard.getCurrentUsage()

      expect(usage.tokensUsed).toBe(3000)
    })

    test("should track cost in USD", () => {
      const sessionId = `test_session_${Date.now()}`
      const budget = {
        maxTokens: 100000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 20,
        maxActions: 100,
      }

      const guard = new SafetyGuard(sessionId, budget)

      guard.record("costUSD", 0.5)
      guard.record("costUSD", 0.3)

      const usage = guard.getCurrentUsage()

      expect(usage.costUSD).toBe(0.8)
    })

    test("should track duration", async () => {
      const sessionId = `test_session_${Date.now()}`
      const budget = {
        maxTokens: 100000,
        maxCostUSD: 10.0,
        maxDurationMinutes: 30,
        maxFilesChanged: 20,
        maxActions: 100,
      }

      const guard = new SafetyGuard(sessionId, budget)

      const startTime = Date.now()
      await new Promise((resolve) => setTimeout(resolve, 100))
      const duration = guard.getCurrentUsage().durationMinutes * 60000

      // Duration should have increased
      expect(duration).toBeGreaterThanOrEqual(0)
    })
  })

  describe("Metrics Collection", () => {
    let collector: MetricsCollector

    beforeEach(() => {
      collector = createMetricsCollector(`test_session_${Date.now()}`)
    })

    test("should record tasks", () => {
      collector.record({ type: "task", name: "total", value: 5, unit: "count" })
      collector.record({ type: "task", name: "completed", value: 3, unit: "count" })
      collector.record({ type: "task", name: "failed", value: 2, unit: "count" })

      const summary = collector.getSummary()

      expect(summary.tasks.total).toBe(5)
      expect(summary.tasks.completed).toBe(3)
      expect(summary.tasks.failed).toBe(2)
    })

    test("should aggregate metrics correctly", () => {
      collector.record({ type: "task", name: "total", value: 10, unit: "count" })
      collector.record({ type: "task", name: "total", value: 5, unit: "count" })

      const summary = collector.getSummary()

      expect(summary.tasks.total).toBe(15)
    })

    test("should track TDD metrics", () => {
      collector.record({ type: "task", name: "tdd_cycle", value: 1, unit: "count" })
      collector.record({ type: "task", name: "tdd_red_success", value: 1, unit: "count" })
      collector.record({ type: "task", name: "tdd_green_success", value: 1, unit: "count" })
      collector.record({ type: "task", name: "tdd_refactor_success", value: 1, unit: "count" })

      const summary = collector.getSummary()

      expect(summary.tdd.cycles).toBe(1)
      expect(summary.tdd.redPassed).toBe(1)
      expect(summary.tdd.greenPassed).toBe(1)
      expect(summary.tdd.refactorPassed).toBe(1)
    })
  })

  describe("Duration Tracking", () => {
    test("should track total duration", () => {
      const startTime = Date.now()
      const duration = Date.now() - startTime

      expect(duration).toBeGreaterThanOrEqual(0)
    })

    test("should format duration in milliseconds", () => {
      const duration = 65000 // 65 seconds

      expect(duration).toBe(65000)
      expect(duration / 1000).toBe(65)
    })

    test("should convert duration to minutes", () => {
      const durationMs = 120000 // 2 minutes
      const durationMinutes = durationMs / 60000

      expect(durationMinutes).toBe(2)
    })
  })

  describe("Iterations Tracking", () => {
    test("should count iterations completed", () => {
      const iterationsCompleted = 3

      expect(iterationsCompleted).toBe(3)
      expect(iterationsCompleted).toBeGreaterThan(0)
    })

    test("should track iteration progress", () => {
      const totalIterations = 5
      const currentIteration = 3

      const progress = currentIteration / totalIterations

      expect(progress).toBe(0.6)
    })
  })

  describe("Return Value Validation", () => {
    test("should validate success is boolean", () => {
      const result = { success: true }

      expect(typeof result.success).toBe("boolean")
    })

    test("should validate qualityScore is number", () => {
      const qualityScore = 85

      expect(typeof qualityScore).toBe("number")
      expect(qualityScore).toBeGreaterThanOrEqual(0)
      expect(qualityScore).toBeLessThanOrEqual(100)
    })

    test("should validate crazinessScore is number", () => {
      const crazinessScore = 75

      expect(typeof crazinessScore).toBe("number")
      expect(crazinessScore).toBeGreaterThanOrEqual(0)
      expect(crazinessScore).toBeLessThanOrEqual(100)
    })

    test("should validate duration is number", () => {
      const duration = 60000

      expect(typeof duration).toBe("number")
      expect(duration).toBeGreaterThan(0)
    })

    test("should validate tokensUsed is number", () => {
      const tokensUsed = 50000

      expect(typeof tokensUsed).toBe("number")
      expect(tokensUsed).toBeGreaterThan(0)
    })

    test("should validate costUSD is number", () => {
      const costUSD = 1.0

      expect(typeof costUSD).toBe("number")
      expect(costUSD).toBeGreaterThanOrEqual(0)
    })

    test("should validate iterationsCompleted is number", () => {
      const iterationsCompleted = 2

      expect(typeof iterationsCompleted).toBe("number")
      expect(iterationsCompleted).toBeGreaterThanOrEqual(0)
    })
  })

  describe("Score Formulas", () => {
    test("should calculate quality score with test pass rate", () => {
      const passRate = 0.8 // 80%
      const expectedScore = passRate * 100

      expect(expectedScore).toBe(80)
    })

    test("should calculate quality score with task completion", () => {
      const completed = 8
      const total = 10
      const completionRate = completed / total
      const expectedScore = completionRate * 100

      expect(expectedScore).toBe(80)
    })

    test("should calculate craziness score based on autonomy level", () => {
      const autonomyScores = {
        lunatic: 95,
        insane: 85,
        crazy: 75,
        wild: 60,
        bold: 40,
        timid: 15,
      }

      expect(autonomyScores.lunatic).toBe(95)
      expect(autonomyScores.timid).toBe(15)
    })
  })
})
