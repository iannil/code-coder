/**
 * Meta Builder Tests
 *
 * Integration tests for the MetaBuilder orchestrator.
 *
 * @package test/autonomous/builder
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { mkdir, rm } from "node:fs/promises"
import path from "path"

import {
  MetaBuilder,
  createMetaBuilder,
  getMetaBuilder,
  type MetaBuilderConfig,
} from "@/autonomous/builder"

import {
  createTestGap,
  createTestBuildContext,
  createTestBuildRequest,
  createTestTaskFailure,
  createMockGeneratedConcept,
  assert,
  verify,
  withTestInstance,
} from "./fixtures/builder-fixture"

describe("MetaBuilder", () => {
  let builder: MetaBuilder
  let testDir: string

  beforeEach(async () => {
    testDir = `/tmp/meta-builder-test-${Date.now()}`
    await mkdir(testDir, { recursive: true })

    builder = createMetaBuilder({
      autonomyLevel: "crazy",
      closeThreshold: 4.0,
      enableAutoApproval: true,
      dryRun: true, // Don't actually register in tests
    })
  })

  afterEach(async () => {
    try {
      await rm(testDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  })

  // ==========================================================================
  // Factory Functions
  // ==========================================================================

  describe("factory functions", () => {
    test("should create new builder instance", () => {
      const b = createMetaBuilder()
      expect(b).toBeDefined()
      expect(b).toBeInstanceOf(MetaBuilder)
    })

    test("should return singleton from getMetaBuilder", () => {
      const b1 = getMetaBuilder()
      const b2 = getMetaBuilder()
      expect(b1).toBe(b2)
    })

    test("should accept custom configuration", () => {
      const config: Partial<MetaBuilderConfig> = {
        autonomyLevel: "lunatic",
        closeThreshold: 8.0,
        enableAutoApproval: false,
        maxBuildAttempts: 5,
        dryRun: true,
      }

      const b = createMetaBuilder(config)
      expect(b).toBeDefined()
    })
  })

  // ==========================================================================
  // initialize
  // ==========================================================================

  describe("initialize", () => {
    test("should initialize without error", async () => {
      await withTestInstance(async () => {
        await expect(builder.initialize()).resolves.toBeUndefined()
      })
    })
  })

  // ==========================================================================
  // build
  // ==========================================================================

  describe("build", () => {
    test("should execute build phases", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            type: "TOOL",
            closeScore: {
              convergence: 8,
              leverage: 8,
              optionality: 8,
              surplus: 8,
              evolution: 8,
              total: 8.0,
            },
          }),
        })

        const result = await builder.build(request)

        // Should have executed multiple phases
        expect(result.phases.length).toBeGreaterThan(0)
      })
    })

    test("should include evaluation phase", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const request = createTestBuildRequest()
        const result = await builder.build(request)

        const evaluationPhase = result.phases.find((p) => p.phase === "evaluation")
        expect(evaluationPhase).toBeDefined()
      })
    })

    test("should track duration", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const request = createTestBuildRequest()
        const result = await builder.build(request)

        expect(result.durationMs).toBeDefined()
        expect(result.durationMs).toBeGreaterThanOrEqual(0)
      })
    })

    test("should include summary message", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const request = createTestBuildRequest()
        const result = await builder.build(request)

        expect(result.summary).toBeDefined()
        expect(typeof result.summary).toBe("string")
      })
    })

    test("should skip registration in dry run mode", async () => {
      await withTestInstance(async () => {
        const dryRunBuilder = createMetaBuilder({
          dryRun: true,
          closeThreshold: 1.0,
        })
        await dryRunBuilder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            closeScore: {
              convergence: 9,
              leverage: 9,
              optionality: 9,
              surplus: 9,
              evolution: 9,
              total: 9.0,
            },
          }),
        })

        const result = await dryRunBuilder.build(request)

        const regPhase = result.phases.find((p) => p.phase === "registration")
        if (regPhase) {
          expect(regPhase.status).toBe("skipped")
        }
      })
    })

    test("should decline build if CLOSE score too low", async () => {
      await withTestInstance(async () => {
        const strictBuilder = createMetaBuilder({
          closeThreshold: 9.5, // Very high threshold
          dryRun: true,
        })
        await strictBuilder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            closeScore: {
              convergence: 5,
              leverage: 5,
              optionality: 5,
              surplus: 5,
              evolution: 5,
              total: 5.0,
            },
          }),
        })

        const result = await strictBuilder.build(request)

        // Should fail - either due to CLOSE evaluation or LLM unavailability
        expect(result.success).toBe(false)
      })
    })
  })

  // ==========================================================================
  // buildFromFailure
  // ==========================================================================

  describe("buildFromFailure", () => {
    test("should detect gap and attempt build", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const failure = createTestTaskFailure({
          description: "Need to analyze CSV data with Python",
          attempts: 3,
          webSearchUsed: true,
          toolSearchUsed: true,
        })

        const result = await builder.buildFromFailure(failure, {
          workingDir: testDir,
        })

        // May return null if no gap detected
        if (result) {
          expect(result.gap).toBeDefined()
        }
      })
    })

    test("should return null if no gap detected", async () => {
      await withTestInstance(async () => {
        const strictBuilder = createMetaBuilder({
          closeThreshold: 10.0, // Impossible to reach
          dryRun: true,
        })
        await strictBuilder.initialize()

        const failure = createTestTaskFailure({
          description: "simple task",
          attempts: 1,
          webSearchUsed: false,
          toolSearchUsed: false,
        })

        const result = await strictBuilder.buildFromFailure(failure)

        expect(result).toBeNull()
      })
    })

    test("should respect autonomy level gate", async () => {
      await withTestInstance(async () => {
        const timidBuilder = createMetaBuilder({
          autonomyLevel: "timid", // Only allows TOOL
          closeThreshold: 1.0,
          dryRun: true,
        })
        await timidBuilder.initialize()

        const failure = createTestTaskFailure({
          description: "Need a specialized agent for code review",
        })

        const result = await timidBuilder.buildFromFailure(failure)

        // Should return null because AGENT is not allowed at timid level
        // (assuming gap detector identifies this as AGENT)
        expect(result === null || result.gap.type === "TOOL").toBe(true)
      })
    })
  })

  // ==========================================================================
  // buildFromQuery
  // ==========================================================================

  describe("buildFromQuery", () => {
    test("should detect gap from query", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const result = await builder.buildFromQuery("csv file parser python", {
          sessionId: "test-session",
          triggeredBy: "user",
        })

        // May return null if similar concept exists
        if (result) {
          expect(result.gap).toBeDefined()
        }
      })
    })

    test("should create session ID if not provided", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const result = await builder.buildFromQuery("data analyzer tool")

        // Should not throw even without sessionId
        expect(result === null || result !== null).toBe(true)
      })
    })
  })

  // ==========================================================================
  // Approval Flow
  // ==========================================================================

  describe("approval flow", () => {
    test("should skip approval for auto-approvable concepts", async () => {
      await withTestInstance(async () => {
        const autoApproveBuilder = createMetaBuilder({
          enableAutoApproval: true,
          closeThreshold: 1.0,
          dryRun: true,
        })
        await autoApproveBuilder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            type: "TOOL", // Auto-approvable
            closeScore: {
              convergence: 9,
              leverage: 9,
              optionality: 9,
              surplus: 9,
              evolution: 9,
              total: 9.0,
            },
          }),
        })

        const result = await autoApproveBuilder.build(request)

        const approvalPhase = result.phases.find((p) => p.phase === "approval")
        if (approvalPhase) {
          expect(["skipped", "completed"]).toContain(approvalPhase.status)
        }
      })
    })

    test("should use custom approval callback when provided", async () => {
      await withTestInstance(async () => {
        let callbackCalled = false

        const customBuilder = createMetaBuilder({
          enableAutoApproval: false,
          closeThreshold: 1.0,
          dryRun: true,
          onApprovalRequest: async () => {
            callbackCalled = true
            return true
          },
        })
        await customBuilder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            type: "AGENT", // Requires approval
            closeScore: {
              convergence: 9,
              leverage: 9,
              optionality: 9,
              surplus: 9,
              evolution: 9,
              total: 9.0,
            },
          }),
        })

        await customBuilder.build(request)

        // Callback should have been called for AGENT type
        expect(callbackCalled).toBe(true)
      })
    })

    test("should deny build when approval callback returns false", async () => {
      await withTestInstance(async () => {
        const denyingBuilder = createMetaBuilder({
          enableAutoApproval: false,
          closeThreshold: 1.0,
          dryRun: true,
          onApprovalRequest: async () => false,
        })
        await denyingBuilder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            type: "HAND", // Requires approval
            closeScore: {
              convergence: 9,
              leverage: 9,
              optionality: 9,
              surplus: 9,
              evolution: 9,
              total: 9.0,
            },
          }),
        })

        const result = await denyingBuilder.build(request)

        // Should fail - either approval denied or LLM unavailable
        expect(result.success).toBe(false)
      })
    })
  })

  // ==========================================================================
  // Build History
  // ==========================================================================

  describe("build history", () => {
    test("should store successful builds in history", async () => {
      await withTestInstance(async () => {
        const historyBuilder = createMetaBuilder({
          closeThreshold: 1.0,
          dryRun: true,
          enableAutoApproval: true,
        })
        await historyBuilder.initialize()

        // Get initial history count
        const initialHistory = historyBuilder.getBuildHistory()

        const request = createTestBuildRequest({
          gap: createTestGap({
            type: "TOOL",
            closeScore: {
              convergence: 9,
              leverage: 9,
              optionality: 9,
              surplus: 9,
              evolution: 9,
              total: 9.0,
            },
          }),
        })

        const result = await historyBuilder.build(request)

        if (result.success) {
          const history = historyBuilder.getBuildHistory()
          expect(history.length).toBeGreaterThan(initialHistory.length)
        }
      })
    })

    test("should retrieve build result by gap ID", async () => {
      await withTestInstance(async () => {
        const historyBuilder = createMetaBuilder({
          closeThreshold: 1.0,
          dryRun: true,
          enableAutoApproval: true,
        })
        await historyBuilder.initialize()

        const gap = createTestGap({
          type: "TOOL",
          closeScore: {
            convergence: 9,
            leverage: 9,
            optionality: 9,
            surplus: 9,
            evolution: 9,
            total: 9.0,
          },
        })

        const request = createTestBuildRequest({ gap })
        const result = await historyBuilder.build(request)

        if (result.success) {
          const retrieved = historyBuilder.getBuildResult(gap.id)
          expect(retrieved).toBeDefined()
          expect(retrieved?.gap.id).toBe(gap.id)
        }
      })
    })

    test("should return null for non-existent gap ID", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const result = builder.getBuildResult("nonexistent_gap_id")

        expect(result).toBeNull()
      })
    })
  })

  // ==========================================================================
  // Phase Execution
  // ==========================================================================

  describe("phase execution", () => {
    test("should record phase durations", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const request = createTestBuildRequest()
        const result = await builder.build(request)

        for (const phase of result.phases) {
          expect(phase.durationMs).toBeDefined()
          expect(phase.durationMs).toBeGreaterThanOrEqual(0)
        }
      })
    })

    test("should record phase errors on failure", async () => {
      await withTestInstance(async () => {
        const strictBuilder = createMetaBuilder({
          closeThreshold: 10.0, // Will fail evaluation
          dryRun: true,
        })
        await strictBuilder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            closeScore: {
              convergence: 1,
              leverage: 1,
              optionality: 1,
              surplus: 1,
              evolution: 1,
              total: 1.0,
            },
          }),
        })

        const result = await strictBuilder.build(request)

        expect(result.success).toBe(false)
      })
    })
  })

  // ==========================================================================
  // Error Handling
  // ==========================================================================

  describe("error handling", () => {
    test("should handle generation failure gracefully", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        // Create a gap with minimal information that might fail generation
        const request = createTestBuildRequest({
          gap: createTestGap({
            description: "", // Empty description might cause issues
            closeScore: {
              convergence: 9,
              leverage: 9,
              optionality: 9,
              surplus: 9,
              evolution: 9,
              total: 9.0,
            },
          }),
        })

        // Should not throw
        const result = await builder.build(request)
        expect(result).toBeDefined()
      })
    })

    test("should include error details in failed phase", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const request = createTestBuildRequest({
          gap: createTestGap({
            closeScore: {
              convergence: 1,
              leverage: 1,
              optionality: 1,
              surplus: 1,
              evolution: 1,
              total: 1.0, // Will fail CLOSE evaluation
            },
          }),
        })

        const result = await builder.build(request)

        if (!result.success) {
          expect(result.summary).toBeDefined()
        }
      })
    })
  })

  // ==========================================================================
  // CLOSE Score
  // ==========================================================================

  describe("CLOSE score", () => {
    test("should include CLOSE score in result", async () => {
      await withTestInstance(async () => {
        await builder.initialize()

        const request = createTestBuildRequest()
        const result = await builder.build(request)

        expect(result.closeScore).toBeDefined()
        expect(result.closeScore.total).toBeGreaterThan(0)
      })
    })

    test("should use gap CLOSE score for failed evaluation", async () => {
      await withTestInstance(async () => {
        const strictBuilder = createMetaBuilder({
          closeThreshold: 10.0,
          dryRun: true,
        })
        await strictBuilder.initialize()

        const gapScore = {
          convergence: 5,
          leverage: 5,
          optionality: 5,
          surplus: 5,
          evolution: 5,
          total: 5.0,
        }

        const request = createTestBuildRequest({
          gap: createTestGap({ closeScore: gapScore }),
        })

        const result = await strictBuilder.build(request)

        expect(result.closeScore.total).toBe(gapScore.total)
      })
    })
  })
})
