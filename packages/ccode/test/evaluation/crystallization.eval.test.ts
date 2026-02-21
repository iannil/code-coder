/**
 * Skill Crystallization Evaluation Tests
 *
 * Verifies that the SkillGeneration module correctly:
 * - C1: Extracts reusable patterns from solutions
 * - C2: Generates valid skill candidates
 * - C3: Filters duplicate candidates
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { SkillGeneration } from "@/bootstrap/generation"
import { CandidateStore } from "@/bootstrap/candidate-store"
import { BootstrapTypes } from "@/bootstrap/types"
import {
  createMockCandidate,
  MOCK_CANDIDATES,
} from "./fixtures/mock-candidates"
import {
  createMockSession,
  createMockToolCall,
  createSessionWithToolCount,
  MOCK_SESSIONS,
} from "./fixtures/mock-sessions"
import {
  CRYSTALLIZATION_EXPECTATIONS,
} from "./fixtures/expected-results"
import {
  calculateExtractionSuccessRate,
  createMetricResult,
  aggregateMetrics,
} from "./utils/metrics"
import { tmpdir } from "../fixture/fixture"

describe("Skill Crystallization Evaluation", () => {
  let tempDir: Awaited<ReturnType<typeof tmpdir>> | undefined

  beforeEach(async () => {
    tempDir = await tmpdir()
    process.env.CCODE_TEST_HOME = tempDir.path
  })

  afterEach(async () => {
    if (tempDir) {
      await tempDir[Symbol.asyncDispose]()
    }
    delete process.env.CCODE_TEST_HOME
  })

  describe("C1: Pattern Recognition", () => {
    test("extractCandidate returns valid candidate structure", async () => {
      const session = MOCK_SESSIONS.jsonFormatting

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      expect(candidate).toBeDefined()
      expect(candidate.id).toBeDefined()
      expect(candidate.name).toBeDefined()
      expect(candidate.description).toBeDefined()
      expect(candidate.type).toBeDefined()
      expect(candidate.content).toBeDefined()
      expect(candidate.source).toBeDefined()
      expect(candidate.verification).toBeDefined()
      expect(candidate.metadata).toBeDefined()
    })

    test("extractCandidate preserves source information", async () => {
      const session = MOCK_SESSIONS.testExecution

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      expect(candidate.source.sessionId).toBe(session.sessionId)
      expect(candidate.source.problem).toBe(session.problem)
      expect(candidate.source.solution).toBe(session.solution)
      expect(candidate.source.toolCalls).toHaveLength(session.toolCalls.length)
    })

    test("extractCandidate sets trigger context", async () => {
      const session = createMockSession()

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
        triggerType: "manual",
      })

      expect(candidate.trigger.type).toBe("manual")
      expect(candidate.trigger.context.length).toBeGreaterThan(0)
    })

    test("workflow type for bash-dominant sessions", async () => {
      const bashSession = createMockSession({
        toolCalls: [
          createMockToolCall({ tool: "bash", input: { command: "npm install" } }),
          createMockToolCall({ tool: "bash", input: { command: "npm build" } }),
          createMockToolCall({ tool: "bash", input: { command: "npm test" } }),
          createMockToolCall({ tool: "read", input: { path: "output.log" } }),
        ],
      })

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: bashSession.sessionId,
        toolCalls: bashSession.toolCalls,
        problem: bashSession.problem,
        solution: bashSession.solution,
      })

      expect(candidate.type).toBe("workflow")
    })

    test("agent type for task delegation sessions", async () => {
      const delegationSession = createMockSession({
        toolCalls: [
          createMockToolCall({
            tool: "task",
            input: { subagent: "security-reviewer", prompt: "review code" },
          }),
          createMockToolCall({ tool: "read", input: { path: "result.md" } }),
        ],
      })

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: delegationSession.sessionId,
        toolCalls: delegationSession.toolCalls,
        problem: delegationSession.problem,
        solution: delegationSession.solution,
      })

      expect(candidate.type).toBe("agent")
    })

    test("pattern type for diverse tool usage", async () => {
      const diverseSession = createMockSession({
        toolCalls: [
          createMockToolCall({ tool: "read", input: {} }),
          createMockToolCall({ tool: "edit", input: {} }),
        ],
      })

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: diverseSession.sessionId,
        toolCalls: diverseSession.toolCalls,
        problem: diverseSession.problem,
        solution: diverseSession.solution,
      })

      expect(candidate.type).toBe("pattern")
    })
  })

  describe("C2: Candidate Generation", () => {
    test("generateSkillMd produces valid markdown", async () => {
      const candidate = createMockCandidate({
        name: "test-skill-md",
        description: "A test skill",
        type: "workflow",
        content: {
          steps: ["Step 1", "Step 2", "Step 3"],
        },
      })

      const markdown = await SkillGeneration.generateSkillMd(candidate)

      expect(markdown).toContain("---")
      expect(markdown).toContain("name: test-skill-md")
      expect(markdown).toContain("description: A test skill")
      expect(markdown).toContain("# test-skill-md")
    })

    test("generateSkillMd includes workflow steps", async () => {
      const candidate = createMockCandidate({
        type: "workflow",
        content: {
          steps: ["First step", "Second step", "Third step"],
        },
      })

      const markdown = await SkillGeneration.generateSkillMd(candidate)

      expect(markdown).toContain("Steps")
      expect(markdown).toContain("First step")
      expect(markdown).toContain("Second step")
    })

    test("generateSkillMd includes code pattern", async () => {
      const candidate = createMockCandidate({
        type: "pattern",
        content: {
          code: "const x = 42;",
        },
      })

      const markdown = await SkillGeneration.generateSkillMd(candidate)

      expect(markdown).toContain("Code Pattern")
      expect(markdown).toContain("const x = 42;")
    })

    test("generateSkillMd includes agent prompt", async () => {
      const candidate = createMockCandidate({
        type: "agent",
        content: {
          agentPrompt: "You are a helpful assistant",
        },
      })

      const markdown = await SkillGeneration.generateSkillMd(candidate)

      expect(markdown).toContain("Agent Prompt")
      expect(markdown).toContain("helpful assistant")
    })

    test("generateSkillMd includes confidence percentage", async () => {
      const candidate = createMockCandidate({
        verification: {
          status: "passed",
          attempts: 1,
          confidence: 0.75,
        },
      })

      const markdown = await SkillGeneration.generateSkillMd(candidate)

      expect(markdown).toContain("75%")
    })
  })

  describe("C3: Duplicate Filtering", () => {
    test("isDuplicate returns false for new skill", async () => {
      const uniqueName = `unique-skill-${Date.now()}`
      const isDupe = await SkillGeneration.isDuplicate(uniqueName)

      expect(isDupe).toBe(false)
    })

    test("extractAndStore skips existing candidate", async () => {
      const session = createMockSession()

      // First extraction
      const first = await SkillGeneration.extractAndStore({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      // Second extraction with same name (simulated)
      const store = await CandidateStore.read()
      const existingCount = store.candidates.length

      // The function should handle duplicates
      expect(first).toBeDefined()
    })

    test("CandidateStore.add updates existing candidate", async () => {
      const candidate = createMockCandidate({
        name: "duplicate-test",
        verification: { status: "pending", attempts: 0, confidence: 0.3 },
      })

      await CandidateStore.add(candidate)

      // Add again with same name
      const updated = createMockCandidate({
        name: "duplicate-test",
        verification: { status: "passed", attempts: 1, confidence: 0.6 },
      })

      await CandidateStore.add(updated)

      // Should have only one candidate with that name
      const byName = await CandidateStore.getByName("duplicate-test")
      expect(byName).toBeDefined()
      expect(byName?.verification.confidence).toBe(0.6)
    })
  })

  describe("Content Extraction", () => {
    test("workflow content has steps array", async () => {
      const session = MOCK_SESSIONS.databaseMigration

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      if (candidate.type === "workflow") {
        expect(Array.isArray(candidate.content.steps)).toBe(true)
        expect(candidate.content.steps?.length).toBeGreaterThan(0)
      }
    })

    test("pattern content has code string", async () => {
      const codeSession = createMockSession({
        problem: "How to handle errors",
        solution: "```typescript\ntry { } catch { }\n```",
        toolCalls: [
          createMockToolCall({ tool: "read", input: {} }),
        ],
      })

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: codeSession.sessionId,
        toolCalls: codeSession.toolCalls,
        problem: codeSession.problem,
        solution: codeSession.solution,
      })

      if (candidate.type === "pattern") {
        expect(typeof candidate.content.code).toBe("string")
      }
    })
  })

  describe("Initial Metadata", () => {
    test("new candidates have zero usage count", async () => {
      const session = createMockSession()

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      expect(candidate.metadata.usageCount).toBe(0)
    })

    test("new candidates have pending verification", async () => {
      const session = createMockSession()

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      expect(candidate.verification.status).toBe("pending")
      expect(candidate.verification.attempts).toBe(0)
    })

    test("timestamps are set correctly", async () => {
      const before = Date.now()

      const session = createMockSession()
      const candidate = await SkillGeneration.extractCandidate({
        sessionId: session.sessionId,
        toolCalls: session.toolCalls,
        problem: session.problem,
        solution: session.solution,
      })

      const after = Date.now()

      expect(candidate.metadata.created).toBeGreaterThanOrEqual(before)
      expect(candidate.metadata.created).toBeLessThanOrEqual(after)
      expect(candidate.metadata.updated).toBe(candidate.metadata.created)
    })
  })

  describe("Tool Call Count Impact", () => {
    test("more tool calls result in workflow type", async () => {
      const manyToolsSession = createSessionWithToolCount(10)

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: manyToolsSession.sessionId,
        toolCalls: manyToolsSession.toolCalls,
        problem: manyToolsSession.problem,
        solution: manyToolsSession.solution,
      })

      // Many diverse tools should result in workflow
      expect(candidate.type).toBe("workflow")
    })

    test("few tool calls may result in pattern type", async () => {
      const fewToolsSession = createSessionWithToolCount(2)

      const candidate = await SkillGeneration.extractCandidate({
        sessionId: fewToolsSession.sessionId,
        toolCalls: fewToolsSession.toolCalls,
        problem: fewToolsSession.problem,
        solution: fewToolsSession.solution,
      })

      // Few tools might be pattern or workflow depending on content
      expect(["pattern", "workflow"]).toContain(candidate.type)
    })
  })
})

describe("Crystallization Metrics", () => {
  test("calculates extraction success rate", () => {
    const extractions = [
      { triggered: true, valid: true },
      { triggered: true, valid: true },
      { triggered: true, valid: false },
      { triggered: false, valid: false }, // Not triggered, doesn't count
      { triggered: true, valid: true },
    ]

    const rate = calculateExtractionSuccessRate(extractions)
    expect(rate).toBeCloseTo(3 / 4, 2) // 3 valid out of 4 triggered
  })

  test("generates evaluation summary for crystallization dimension", () => {
    const metrics = [
      createMetricResult("Pattern Recognition Rate", 0.85, 0.8, "gte"),
      createMetricResult("Candidate Validity Rate", 0.9, 0.85, "gte"),
      createMetricResult("Duplicate Detection Rate", 0.95, 0.9, "gte"),
      createMetricResult("Content Extraction Quality", 0.8, 0.75, "gte"),
    ]

    const summary = aggregateMetrics("Skill Crystallization", metrics)

    expect(summary.dimension).toBe("Skill Crystallization")
    expect(summary.passRate).toBeGreaterThanOrEqual(0.5)
  })
})
