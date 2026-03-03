import { describe, test, expect } from "bun:test"
import { createAcceptanceLoop } from "../../../src/autonomous/execution/acceptance-loop"

describe("AcceptanceLoop", () => {
  test("creates acceptance loop with default config", () => {
    const loop = createAcceptanceLoop()
    expect(loop).toBeDefined()
    expect(loop.accept).toBeInstanceOf(Function)
  })

  test("creates acceptance loop with custom config", () => {
    const loop = createAcceptanceLoop({
      enableTests: false,
      enableTypecheck: false,
      passThreshold: 7.0,
    })
    expect(loop).toBeDefined()
  })

  test("accept returns result structure", async () => {
    const loop = createAcceptanceLoop({
      enableTests: false,
      enableTypecheck: false,
      enableLint: false,
      enableSecurityScan: false,
      enableExpectationCheck: false,
    })

    const result = await loop.accept({
      sessionId: "test-session",
      originalRequest: "Add a function that adds two numbers",
      implementationResult: {
        solved: true,
        solution: "function add(a, b) { return a + b }",
        attempts: [],
        durationMs: 100,
        summary: "Added function",
      },
    })

    expect(result).toHaveProperty("success")
    expect(result).toHaveProperty("overallScore")
    expect(result).toHaveProperty("closeScores")
    expect(result).toHaveProperty("checks")
    expect(result).toHaveProperty("issues")
    expect(result).toHaveProperty("recommendation")
    expect(result).toHaveProperty("durationMs")

    // Check CLOSE scores structure
    expect(result.closeScores).toHaveProperty("convergence")
    expect(result.closeScores).toHaveProperty("leverage")
    expect(result.closeScores).toHaveProperty("optionality")
    expect(result.closeScores).toHaveProperty("surplus")
    expect(result.closeScores).toHaveProperty("evolution")
    expect(result.closeScores).toHaveProperty("total")

    // Check recommendation is valid
    expect(["pass", "fix", "rework"]).toContain(result.recommendation)
  })

  test("accept with no implementation returns rework recommendation", async () => {
    const loop = createAcceptanceLoop({
      enableTests: false,
      enableTypecheck: false,
      enableLint: false,
      enableSecurityScan: false,
      enableExpectationCheck: false,
    })

    const result = await loop.accept({
      sessionId: "test-session",
      originalRequest: "Build a web server",
      // No implementation result
    })

    expect(result.success).toBe(false)
    expect(result.recommendation).toBe("rework")
    expect(result.issues.length).toBeGreaterThan(0)
  })

  test("parses requirements from numbered list", async () => {
    const loop = createAcceptanceLoop({
      enableTests: false,
      enableTypecheck: false,
      enableLint: false,
      enableSecurityScan: false,
      enableExpectationCheck: false,
    })

    const result = await loop.accept({
      sessionId: "test-session",
      originalRequest: `Create a calculator:
1. Add function
2. Subtract function
3. Multiply function`,
      implementationResult: {
        solved: true,
        solution: "calculator code",
        attempts: [],
        durationMs: 100,
        summary: "Created calculator",
      },
    })

    expect(result.checks.requirementConformance.total).toBeGreaterThanOrEqual(1)
  })

  test("generates report content", async () => {
    const loop = createAcceptanceLoop({
      enableTests: false,
      enableTypecheck: false,
      enableLint: false,
      enableSecurityScan: false,
      enableExpectationCheck: false,
    })

    const result = await loop.accept({
      sessionId: "test-session",
      originalRequest: "Test request",
      implementationResult: {
        solved: true,
        solution: "test code",
        attempts: [],
        durationMs: 100,
        summary: "Done",
      },
    })

    expect(result.report).toBeDefined()
    expect(result.report).toContain("# Acceptance Report")
    expect(result.report).toContain("## CLOSE Scores")
  })
})
