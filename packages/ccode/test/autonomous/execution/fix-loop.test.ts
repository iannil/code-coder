import { describe, test, expect } from "bun:test"
import { createFixLoop } from "../../../src/autonomous/execution/fix-loop"
import type { AcceptanceIssue } from "../../../src/autonomous/execution/acceptance-loop"

describe("FixLoop", () => {
  test("creates fix loop with default config", () => {
    const loop = createFixLoop()
    expect(loop).toBeDefined()
    expect(loop.fix).toBeInstanceOf(Function)
  })

  test("creates fix loop with custom config", () => {
    const loop = createFixLoop({
      maxAttemptsPerIssue: 5,
      enableAutoFix: false,
      enableLearning: false,
    })
    expect(loop).toBeDefined()
  })

  test("fix returns result structure", async () => {
    const loop = createFixLoop({
      enableAutoFix: false,
      enableAgentFix: false,
      enableLLMGeneration: false,
      enableEvolutionFallback: false,
      enableLearning: false,
      verifyAfterEachFix: false,
    })

    const issues: AcceptanceIssue[] = [
      {
        id: "test-1",
        type: "lint",
        severity: "low",
        description: "Test lint issue",
      },
    ]

    const result = await loop.fix({
      sessionId: "test-session",
      issues,
    })

    expect(result).toHaveProperty("success")
    expect(result).toHaveProperty("fixedIssues")
    expect(result).toHaveProperty("remainingIssues")
    expect(result).toHaveProperty("attempts")
    expect(result).toHaveProperty("durationMs")
    expect(result).toHaveProperty("shouldRecheck")
  })

  test("fix with empty issues returns success", async () => {
    const loop = createFixLoop()

    const result = await loop.fix({
      sessionId: "test-session",
      issues: [],
    })

    expect(result.success).toBe(true)
    expect(result.fixedIssues).toEqual([])
    expect(result.remainingIssues).toEqual([])
    expect(result.durationMs).toBe(0)
  })

  test("prioritizes critical issues first", async () => {
    const loop = createFixLoop({
      enableAutoFix: false,
      enableAgentFix: false,
      enableLLMGeneration: false,
      enableEvolutionFallback: false,
      enableLearning: false,
      verifyAfterEachFix: false,
    })

    const issues: AcceptanceIssue[] = [
      {
        id: "low-1",
        type: "lint",
        severity: "low",
        description: "Low severity issue",
      },
      {
        id: "critical-1",
        type: "security",
        severity: "critical",
        description: "Critical security issue",
      },
      {
        id: "medium-1",
        type: "type",
        severity: "medium",
        description: "Medium severity issue",
      },
    ]

    const result = await loop.fix({
      sessionId: "test-session",
      issues,
    })

    // Attempts should be made in priority order
    // (critical security > medium type > low lint)
    expect(result.attempts.length).toBeGreaterThan(0)
    if (result.attempts.length >= 2) {
      // First attempt should be for critical issue
      expect(result.attempts[0].issueId).toBe("critical-1")
    }
  })

  test("respects max attempts per issue", async () => {
    const loop = createFixLoop({
      maxAttemptsPerIssue: 1,
      enableAutoFix: false,
      enableAgentFix: false,
      enableLLMGeneration: false,
      enableEvolutionFallback: false,
      enableLearning: false,
      verifyAfterEachFix: false,
    })

    const issues: AcceptanceIssue[] = [
      {
        id: "test-1",
        type: "test",
        severity: "high",
        description: "Test failure",
      },
    ]

    const result = await loop.fix({
      sessionId: "test-session",
      issues,
    })

    // Should only have 1 attempt due to maxAttemptsPerIssue
    const attemptsForIssue = result.attempts.filter((a) => a.issueId === "test-1")
    expect(attemptsForIssue.length).toBeLessThanOrEqual(1)
  })

  test("calculates shouldRecheck based on fixed issues", async () => {
    const loop = createFixLoop({
      enableAutoFix: true,
      enableAgentFix: false,
      enableLLMGeneration: false,
      enableEvolutionFallback: false,
      enableLearning: false,
      verifyAfterEachFix: false,
    })

    const issues: AcceptanceIssue[] = [
      {
        id: "lint-1",
        type: "lint",
        severity: "low",
        description: "Lint issue that can be auto-fixed",
      },
    ]

    const result = await loop.fix({
      sessionId: "test-session",
      issues,
    })

    // shouldRecheck should be true if any issues were fixed
    if (result.fixedIssues.length > 0) {
      expect(result.shouldRecheck).toBe(true)
    }
  })

  test("success is false when critical issues remain", async () => {
    const loop = createFixLoop({
      enableAutoFix: false,
      enableAgentFix: false,
      enableLLMGeneration: false,
      enableEvolutionFallback: false,
      enableLearning: false,
      verifyAfterEachFix: false,
    })

    const issues: AcceptanceIssue[] = [
      {
        id: "critical-1",
        type: "security",
        severity: "critical",
        description: "Critical unfixable issue",
      },
    ]

    const result = await loop.fix({
      sessionId: "test-session",
      issues,
    })

    // Since all strategies are disabled, the critical issue should remain
    expect(result.remainingIssues.length).toBe(1)
    expect(result.success).toBe(false)
  })

  test("maps issue types to correct strategies", async () => {
    const loop = createFixLoop({
      enableAutoFix: true,
      enableAgentFix: true,
      enableLLMGeneration: true,
      enableEvolutionFallback: true,
      enableLearning: false,
      verifyAfterEachFix: false,
    })

    // Test with different issue types to verify strategy selection
    const issues: AcceptanceIssue[] = [
      {
        id: "lint-1",
        type: "lint",
        severity: "low",
        description: "Lint issue",
      },
    ]

    const result = await loop.fix({
      sessionId: "test-session",
      issues,
    })

    // Lint issues should use auto_fix strategy first
    if (result.attempts.length > 0) {
      expect(result.attempts[0].strategy).toBe("auto_fix")
    }
  })
})
