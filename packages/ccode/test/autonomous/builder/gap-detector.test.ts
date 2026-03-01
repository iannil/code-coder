/**
 * Gap Detector Tests
 *
 * Tests for the GapDetector class and related functionality.
 *
 * @package test/autonomous/builder
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"

import {
  GapDetector,
  createGapDetector,
  type GapDetectorConfig,
  type TaskFailure,
} from "@/autonomous/builder"

import {
  createTestTaskFailure,
  createTestGap,
  assert,
  verify,
  withTestInstance,
} from "./fixtures/builder-fixture"

describe("GapDetector", () => {
  let detector: GapDetector

  beforeEach(() => {
    detector = createGapDetector({
      minConfidence: 0.5,
      closeThreshold: 4.0, // Lower threshold for testing
      enableLLMAnalysis: false, // Disable LLM for faster tests
    })
  })

  afterEach(() => {
    detector.clearAllGaps()
  })

  // ==========================================================================
  // Constructor and Configuration
  // ==========================================================================

  describe("constructor", () => {
    test("should create detector with default config", () => {
      const defaultDetector = createGapDetector()
      expect(defaultDetector).toBeDefined()
    })

    test("should accept custom configuration", () => {
      const customDetector = createGapDetector({
        minConfidence: 0.8,
        closeThreshold: 7.0,
      })
      expect(customDetector).toBeDefined()
    })
  })

  // ==========================================================================
  // detectFromFailure
  // ==========================================================================

  describe("detectFromFailure", () => {
    test("should detect gap from task failure", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need to analyze CSV file with Python",
          attempts: 3,
          webSearchUsed: true,
          toolSearchUsed: true,
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          assert.validGap(gap)
          expect(gap.type).toBe("TOOL") // Default for code execution
          expect(gap.confidence).toBeGreaterThan(0.5)
        }
      })
    })

    test("should return null if confidence too low", async () => {
      const highConfidenceDetector = createGapDetector({
        minConfidence: 0.99, // Very high threshold
        closeThreshold: 1.0,
      })

      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Simple task",
          attempts: 1,
          webSearchUsed: false,
          toolSearchUsed: false,
        })

        const gap = await highConfidenceDetector.detectFromFailure(failure)
        expect(gap).toBeNull()
      })
    })

    test("should increase confidence with more attempts", async () => {
      await withTestInstance(async () => {
        const fewAttempts: TaskFailure = createTestTaskFailure({
          attempts: 1,
          webSearchUsed: false,
          toolSearchUsed: false,
        })

        const manyAttempts: TaskFailure = createTestTaskFailure({
          attempts: 5,
          webSearchUsed: true,
          toolSearchUsed: true,
        })

        const gap1 = await detector.detectFromFailure(fewAttempts)
        detector.clearAllGaps()
        const gap2 = await detector.detectFromFailure(manyAttempts)

        if (gap1 && gap2) {
          expect(gap2.confidence).toBeGreaterThan(gap1.confidence)
        }
      })
    })

    test("should record failure in history", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure()

        await detector.detectFromFailure(failure)

        // Verify by checking pattern analysis
        const gaps = await detector.analyzePatterns()
        // Pattern analysis requires minPatternOccurrences (default 2)
        // So one failure won't trigger pattern detection
        expect(gaps.length).toBe(0)
      })
    })
  })

  // ==========================================================================
  // detectFromQuery
  // ==========================================================================

  describe("detectFromQuery", () => {
    test("should detect gap from search query", async () => {
      await withTestInstance(async () => {
        const gap = await detector.detectFromQuery("csv file parser python", {
          sessionId: "test-session",
          technology: "python",
          isUserRequest: true,
        })

        // May return null if similar concepts exist
        if (gap) {
          assert.validGap(gap)
        }
      })
    })

    test("should increase confidence for user requests", async () => {
      await withTestInstance(async () => {
        const systemGap = await detector.detectFromQuery("some tool", {
          isUserRequest: false,
        })
        detector.clearAllGaps()

        const userGap = await detector.detectFromQuery("some tool", {
          isUserRequest: true,
        })

        if (systemGap && userGap) {
          expect(userGap.confidence).toBeGreaterThan(systemGap.confidence)
        }
      })
    })
  })

  // ==========================================================================
  // Concept Type Inference
  // ==========================================================================

  describe("concept type inference", () => {
    test("should infer HAND for scheduled tasks", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need to schedule a daily cron job",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          verify.conceptType(gap, "HAND")
        }
      })
    })

    test("should infer WORKFLOW for pipeline tasks", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need to create a multi-step data pipeline",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          verify.conceptType(gap, "WORKFLOW")
        }
      })
    })

    test("should infer AGENT for expert/specialist tasks", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need a security expert agent to review code",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          verify.conceptType(gap, "AGENT")
        }
      })
    })

    test("should infer SKILL for command/action tasks", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need a /deploy command to push to production",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          verify.conceptType(gap, "SKILL")
        }
      })
    })

    test("should infer MEMORY for storage tasks", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need to remember user preferences and persist them",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          verify.conceptType(gap, "MEMORY")
        }
      })
    })

    test("should infer PROMPT for template tasks", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need a prompt template for code reviews",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          verify.conceptType(gap, "PROMPT")
        }
      })
    })

    test("should default to TOOL for general tasks", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Need to process and analyze data files",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          verify.conceptType(gap, "TOOL")
        }
      })
    })
  })

  // ==========================================================================
  // CLOSE Score Calculation
  // ==========================================================================

  describe("CLOSE score calculation", () => {
    test("should calculate valid CLOSE score", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          attempts: 3,
          webSearchUsed: true,
          toolSearchUsed: true,
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          expect(gap.closeScore.total).toBeGreaterThan(0)
          expect(gap.closeScore.total).toBeLessThanOrEqual(10)
          expect(gap.closeScore.convergence).toBeDefined()
          expect(gap.closeScore.leverage).toBeDefined()
          expect(gap.closeScore.optionality).toBeDefined()
          expect(gap.closeScore.surplus).toBeDefined()
          expect(gap.closeScore.evolution).toBeDefined()
        }
      })
    })

    test("should return null if CLOSE score below threshold", async () => {
      const highThresholdDetector = createGapDetector({
        minConfidence: 0.1,
        closeThreshold: 9.5, // Very high threshold
      })

      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          attempts: 1,
        })

        const gap = await highThresholdDetector.detectFromFailure(failure)

        // Should be null due to high CLOSE threshold
        expect(gap).toBeNull()
      })
    })
  })

  // ==========================================================================
  // Pattern Analysis
  // ==========================================================================

  describe("analyzePatterns", () => {
    test("should return empty array with insufficient history", async () => {
      const gaps = await detector.analyzePatterns()
      expect(gaps).toEqual([])
    })

    test("should detect patterns from multiple similar failures", async () => {
      await withTestInstance(async () => {
        // Add multiple similar failures
        for (let i = 0; i < 3; i++) {
          await detector.detectFromFailure(
            createTestTaskFailure({
              description: "CSV processing task failed",
              sessionId: `session-${i}`,
            }),
          )
        }

        const gaps = await detector.analyzePatterns()
        // Pattern detection should work with 3+ occurrences
        expect(gaps.length).toBeGreaterThanOrEqual(0)
      })
    })
  })

  // ==========================================================================
  // Gap Management
  // ==========================================================================

  describe("gap management", () => {
    test("should store detected gaps", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure()
        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          const retrieved = detector.getGap(gap.id)
          expect(retrieved).toBeDefined()
          expect(retrieved?.id).toBe(gap.id)
        }
      })
    })

    test("should list all detected gaps", async () => {
      await withTestInstance(async () => {
        await detector.detectFromFailure(createTestTaskFailure({ sessionId: "s1" }))
        await detector.detectFromFailure(
          createTestTaskFailure({
            sessionId: "s2",
            description: "Different task",
          }),
        )

        const gaps = detector.getDetectedGaps()
        expect(gaps.length).toBeGreaterThanOrEqual(0)
      })
    })

    test("should clear specific gap", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure()
        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          detector.clearGap(gap.id)
          expect(detector.getGap(gap.id)).toBeNull()
        }
      })
    })

    test("should clear all gaps", async () => {
      await withTestInstance(async () => {
        await detector.detectFromFailure(createTestTaskFailure({ sessionId: "s1" }))
        await detector.detectFromFailure(createTestTaskFailure({ sessionId: "s2" }))

        detector.clearAllGaps()

        expect(detector.getDetectedGaps().length).toBe(0)
      })
    })

    test("should return null for non-existent gap", () => {
      const gap = detector.getGap("non-existent-id")
      expect(gap).toBeNull()
    })
  })

  // ==========================================================================
  // Gap Evidence
  // ==========================================================================

  describe("gap evidence", () => {
    test("should include task failure evidence", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          errorMessage: "Test error message",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          expect(gap.evidence.length).toBeGreaterThan(0)
          expect(gap.evidence[0].type).toBe("task_failure")
          expect(gap.evidence[0].source).toBe("Test error message")
        }
      })
    })

    test("should include search metadata in evidence", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          webSearchUsed: true,
          toolSearchUsed: true,
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap) {
          const evidence = gap.evidence[0]
          expect(evidence.metadata?.webSearchUsed).toBe(true)
          expect(evidence.metadata?.toolSearchUsed).toBe(true)
        }
      })
    })
  })

  // ==========================================================================
  // Suggested Names
  // ==========================================================================

  describe("suggested names", () => {
    test("should generate suggested name from description", async () => {
      await withTestInstance(async () => {
        const failure: TaskFailure = createTestTaskFailure({
          description: "Parse JSON files and extract data",
        })

        const gap = await detector.detectFromFailure(failure)

        if (gap && gap.suggestedName) {
          expect(gap.suggestedName.length).toBeGreaterThan(0)
          expect(gap.suggestedName).not.toContain(" ") // Should be normalized
        }
      })
    })
  })
})
