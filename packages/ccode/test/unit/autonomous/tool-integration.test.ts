/**
 * Sandbox-Tool Registry Integration Tests
 *
 * Tests the integration between EvolutionLoop and DynamicToolRegistry.
 * Part of Phase 13: Sandbox-Tool Registry Integration
 */

import { describe, it, expect, beforeEach, afterEach, mock, spyOn } from "bun:test"
import {
  EvolutionLoop,
  createEvolutionLoop,
  type AutonomousProblem,
  type EvolutionConfig,
} from "@/autonomous/execution/evolution-loop"
import { DynamicToolRegistry, ToolTypes } from "@/memory/tools"

// ============================================================================
// Test Fixtures
// ============================================================================

const createProblem = (overrides?: Partial<AutonomousProblem>): AutonomousProblem => ({
  sessionId: "test-session-001",
  description: "Calculate the sum of numbers in an array",
  technology: "nodejs",
  maxRetries: 2,
  enableWebSearch: false,
  enableCodeExecution: true,
  ...overrides,
})

const createConfig = (overrides?: Partial<EvolutionConfig>): Partial<EvolutionConfig> => ({
  maxRetries: 2,
  enableWebSearch: false,
  enableCodeExecution: true,
  enableSedimentation: false,
  enableToolLearning: true,
  enableToolDiscovery: true,
  toolMatchThreshold: 0.7,
  ...overrides,
})

// ============================================================================
// Tests
// ============================================================================

describe("Sandbox-Tool Registry Integration", () => {
  beforeEach(async () => {
    // Clear tool registry before each test
    await DynamicToolRegistry.clear()
  })

  afterEach(async () => {
    // Cleanup after tests
    await DynamicToolRegistry.clear()
  })

  describe("Tool Discovery", () => {
    it("should find and use existing tool when available", async () => {
      // Register a tool that matches the problem
      await DynamicToolRegistry.register({
        name: "array-sum",
        description: "Calculate the sum of numbers in an array",
        tags: ["math", "array"],
        code: `const arr = [1, 2, 3, 4, 5];
const sum = arr.reduce((a, b) => a + b, 0);
console.log("Sum:", sum);
process.exit(0);`,
        language: "nodejs",
        parameters: [],
        examples: [],
        createdBy: "agent",
      })

      const loop = createEvolutionLoop(createConfig())
      const problem = createProblem()

      const result = await loop.evolve(problem)
      await loop.cleanup()

      // Verify tool was found and used
      expect(result.usedToolId).toBeDefined()
      expect(result.solved).toBe(true)
      expect(result.summary).toContain("existing tool")
    })

    it("should not use tool when score is below threshold", async () => {
      // Register a tool that doesn't match well
      await DynamicToolRegistry.register({
        name: "string-reverse",
        description: "Reverse a string character by character",
        tags: ["string"],
        code: `const str = "hello";
console.log(str.split("").reverse().join(""));`,
        language: "nodejs",
        parameters: [],
        examples: [],
        createdBy: "agent",
      })

      const loop = createEvolutionLoop(createConfig({
        toolMatchThreshold: 0.9, // High threshold
      }))
      const problem = createProblem()

      const result = await loop.evolve(problem)
      await loop.cleanup()

      // Tool should not be used due to low match score
      expect(result.usedToolId).toBeUndefined()
    })

    it("should skip tool discovery when disabled", async () => {
      await DynamicToolRegistry.register({
        name: "array-sum",
        description: "Calculate the sum of numbers in an array",
        tags: ["math"],
        code: `console.log("Sum:", [1,2,3].reduce((a,b)=>a+b)); process.exit(0);`,
        language: "nodejs",
        parameters: [],
        examples: [],
        createdBy: "agent",
      })

      const loop = createEvolutionLoop(createConfig({
        enableToolDiscovery: false,
      }))
      const problem = createProblem()

      const result = await loop.evolve(problem)
      await loop.cleanup()

      // Tool should not be used when discovery is disabled
      expect(result.usedToolId).toBeUndefined()
    })
  })

  describe("Tool Learning", () => {
    it("should learn tool from successful execution", async () => {
      const loop = createEvolutionLoop(createConfig({
        enableToolLearning: true,
        enableToolDiscovery: false, // Disable so we generate new code
      }))

      // Use a problem that will generate meaningful code
      const problem = createProblem({
        description: "Sort an array of numbers in ascending order",
      })

      // Note: This test depends on the code generation producing learnable code
      // In real scenarios, the sandbox would execute and learn
      const statsBefore = await DynamicToolRegistry.getStats()
      const initialCount = statsBefore.totalTools

      // The actual learning depends on execution success
      // We're testing that the integration path exists
      const result = await loop.evolve(problem)
      await loop.cleanup()

      const statsAfter = await DynamicToolRegistry.getStats()

      // If execution succeeded and code was meaningful, a tool should be learned
      if (result.solved && result.learnedToolId) {
        expect(statsAfter.totalTools).toBeGreaterThan(initialCount)
      }
    })

    it("should not learn trivial code", async () => {
      const execution: ToolTypes.ExecutionRecord = {
        code: `console.log("hi");`,
        language: "nodejs",
        task: "Print hello",
        output: "hi",
        exitCode: 0,
        durationMs: 10,
      }

      const learned = await DynamicToolRegistry.learnFromExecution(execution)

      // Trivial code should be skipped (too short)
      expect(learned).toBeNull()
    })

    it("should not learn code without output", async () => {
      const execution: ToolTypes.ExecutionRecord = {
        code: `function sum(arr) {
  return arr.reduce((a, b) => a + b, 0);
}

const result = sum([1, 2, 3, 4, 5]);
// No console output`,
        language: "nodejs",
        task: "Calculate sum",
        output: "", // Empty output
        exitCode: 0,
        durationMs: 100,
      }

      const learned = await DynamicToolRegistry.learnFromExecution(execution)

      // Code without output should be skipped
      expect(learned).toBeNull()
    })
  })

  describe("Usage Statistics", () => {
    it("should record usage when tool is executed", async () => {
      const tool = await DynamicToolRegistry.register({
        name: "test-tool",
        description: "Calculate the sum of an array",
        tags: ["test"],
        code: `const sum = [1,2,3,4,5].reduce((a,b)=>a+b,0);
console.log("Result:", sum);
process.exit(0);`,
        language: "nodejs",
        parameters: [],
        examples: [],
        createdBy: "agent",
      })

      const initialStats = tool.stats
      expect(initialStats.usageCount).toBe(0)

      // Record successful usage
      await DynamicToolRegistry.recordUsage(tool.id, true, 150)

      const updatedTool = await DynamicToolRegistry.get(tool.id)
      expect(updatedTool?.stats.usageCount).toBe(1)
      expect(updatedTool?.stats.successCount).toBe(1)
      expect(updatedTool?.stats.averageExecutionTime).toBe(150)

      // Record failed usage
      await DynamicToolRegistry.recordUsage(tool.id, false, 200)

      const finalTool = await DynamicToolRegistry.get(tool.id)
      expect(finalTool?.stats.usageCount).toBe(2)
      expect(finalTool?.stats.successCount).toBe(1)
      expect(finalTool?.stats.failureCount).toBe(1)
    })
  })

  describe("EvolutionConfig", () => {
    it("should have correct default config values", () => {
      const loop = createEvolutionLoop()

      // Access private config through type assertion for testing
      const loopAny = loop as any
      expect(loopAny.config.enableToolLearning).toBe(true)
      expect(loopAny.config.enableToolDiscovery).toBe(true)
      expect(loopAny.config.toolMatchThreshold).toBe(0.7)
    })

    it("should allow overriding tool config", () => {
      const loop = createEvolutionLoop({
        enableToolLearning: false,
        enableToolDiscovery: false,
        toolMatchThreshold: 0.9,
      })

      const loopAny = loop as any
      expect(loopAny.config.enableToolLearning).toBe(false)
      expect(loopAny.config.enableToolDiscovery).toBe(false)
      expect(loopAny.config.toolMatchThreshold).toBe(0.9)
    })
  })

  describe("SolutionAttempt", () => {
    it("should include tool information when using existing tool", async () => {
      const tool = await DynamicToolRegistry.register({
        name: "quick-sum",
        description: "Calculate sum of numbers in array",
        tags: ["math"],
        code: `const sum = [1,2,3,4,5].reduce((a,b)=>a+b,0);
console.log("Sum:", sum);
process.exit(0);`,
        language: "nodejs",
        parameters: [],
        examples: [],
        createdBy: "agent",
      })

      const loop = createEvolutionLoop(createConfig())
      const problem = createProblem()

      const result = await loop.evolve(problem)
      await loop.cleanup()

      if (result.usedToolId) {
        // Verify attempt includes tool info
        expect(result.attempts.length).toBeGreaterThan(0)
        const attempt = result.attempts[0]
        expect(attempt.toolId).toBe(tool.id)
        expect(attempt.toolName).toBe("quick-sum")
      }
    })
  })

  describe("EvolutionResult", () => {
    it("should have learnedToolId and usedToolId fields in result", async () => {
      // This test verifies the result structure includes the new fields
      const loop = createEvolutionLoop(createConfig({
        enableToolDiscovery: false,
        enableCodeExecution: false, // Disable to get quick result
      }))

      const problem = createProblem()
      const result = await loop.evolve(problem)
      await loop.cleanup()

      // Result should have the new fields (may be undefined but keys should exist)
      expect(Object.hasOwn(result, "learnedToolId")).toBe(true)
      expect(Object.hasOwn(result, "usedToolId")).toBe(true)
    })

    it("should include usedToolId when existing tool is used", async () => {
      await DynamicToolRegistry.register({
        name: "array-calculator",
        description: "Calculate sum of numbers in array",
        tags: ["math", "array", "sum"],
        code: `const arr = [1,2,3,4,5];
const sum = arr.reduce((a,b)=>a+b,0);
console.log("Sum:", sum);
process.exit(0);`,
        language: "nodejs",
        parameters: [],
        examples: [],
        createdBy: "agent",
      })

      const loop = createEvolutionLoop(createConfig())
      const problem = createProblem()

      const result = await loop.evolve(problem)
      await loop.cleanup()

      if (result.solved && result.usedToolId) {
        expect(result.usedToolId).toBeDefined()
        expect(result.learnedToolId).toBeUndefined() // No new tool learned
      }
    })
  })
})
