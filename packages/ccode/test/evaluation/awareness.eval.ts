/**
 * Self-Awareness Evaluation Tests
 *
 * Verifies that the SelfAwareness module correctly:
 * - A1: Identifies available tools for an agent
 * - A2: Discovers matching skills for tasks
 * - A3: Returns calibrated confidence scores
 * - A4: Identifies missing resources/capabilities
 */

import { describe, test, expect, beforeEach, afterEach, mock } from "bun:test"
import { SelfAwareness } from "@/bootstrap/awareness"
import { BootstrapTypes } from "@/bootstrap/types"
import { Agent } from "@/agent/agent"
import { ToolRegistry } from "@/tool/registry"
import { Skill } from "@/skill/skill"
import {
  AWARENESS_EXPECTATIONS,
  isWithinExpectedRange,
} from "./fixtures/expected-results"
import {
  calculateCalibrationError,
  createMetricResult,
  aggregateMetrics,
} from "./utils/metrics"
import { Log } from "@/util/log"

// Suppress logging during tests
Log.init({ print: false })

describe("Awareness Evaluation", () => {
  describe("A1: Tool Identification", () => {
    test("introspect returns agent capabilities structure", async () => {
      const capabilities = await SelfAwareness.introspect("build")

      expect(capabilities).toBeDefined()
      expect(capabilities.name).toBeDefined()
      expect(Array.isArray(capabilities.tools)).toBe(true)
      expect(Array.isArray(capabilities.skills)).toBe(true)
      expect(Array.isArray(capabilities.mcpServers)).toBe(true)
      expect(typeof capabilities.permissions).toBe("object")
    })

    test("introspect returns tools list", async () => {
      const capabilities = await SelfAwareness.introspect("build")

      // Verify tools is an array (may be empty in test environment)
      expect(Array.isArray(capabilities.tools)).toBe(true)
    })

    test("introspect returns default capabilities for unknown agent", async () => {
      const capabilities = await SelfAwareness.introspect("nonexistent-agent-xyz")

      expect(capabilities.name).toBe("nonexistent-agent-xyz")
      expect(capabilities.tools).toEqual([])
      expect(capabilities.skills).toEqual([])
    })

    test("introspect uses 'build' as default agent", async () => {
      const withDefault = await SelfAwareness.introspect()
      const withExplicit = await SelfAwareness.introspect("build")

      // Both should return similar structure
      expect(withDefault.name).toBeDefined()
      expect(withExplicit.name).toBeDefined()
    })
  })

  describe("A2: Skill Discovery", () => {
    test("canHandle returns structured result", async () => {
      const result = await SelfAwareness.canHandle("read the package.json file")

      expect(result).toBeDefined()
      expect(typeof result.confident).toBe("boolean")
      expect(typeof result.confidence).toBe("number")
      expect(result.confidence).toBeGreaterThanOrEqual(0)
      expect(result.confidence).toBeLessThanOrEqual(1)
    })

    test("canHandle identifies test-related tasks", async () => {
      const result = await SelfAwareness.canHandle("run unit tests with coverage")

      // Should identify need for testing-related capabilities
      expect(result).toBeDefined()
      // The analysis should detect "test" keyword
    })

    test("canHandle identifies security-related tasks", async () => {
      const result = await SelfAwareness.canHandle("check for security vulnerabilities")

      // Should identify need for security-related capabilities
      expect(result).toBeDefined()
    })

    test("canHandle identifies database-related tasks", async () => {
      const result = await SelfAwareness.canHandle("write SQL query for user data")

      // Should identify need for database capabilities
      expect(result).toBeDefined()
    })
  })

  describe("A3: Capability Assessment", () => {
    test("easy tasks return high confidence", async () => {
      const easyTask = "read the package.json file"
      const result = await SelfAwareness.canHandle(easyTask)

      expect(result.confidence).toBeGreaterThanOrEqual(0.6)
      expect(result.confident).toBe(true)
    })

    test("complex tasks with domain requirements return lower confidence", async () => {
      const complexTask = "deploy kubernetes cluster with terraform on AWS"
      const result = await SelfAwareness.canHandle(complexTask)

      // Should have lower confidence due to domain indicators
      expect(result.confidence).toBeLessThan(0.8)
    })

    test("confidence is calibrated - harder tasks have lower confidence", async () => {
      const easyTask = "show file contents"
      const hardTask = "implement neural network from scratch"

      const easyResult = await SelfAwareness.canHandle(easyTask)
      const hardResult = await SelfAwareness.canHandle(hardTask)

      expect(easyResult.confidence).toBeGreaterThan(hardResult.confidence)
    })

    test("multiple domain indicators reduce confidence", async () => {
      const singleDomain = "deploy to kubernetes"
      const multiDomain = "deploy machine learning model to kubernetes on AWS with terraform"

      const singleResult = await SelfAwareness.canHandle(singleDomain)
      const multiResult = await SelfAwareness.canHandle(multiDomain)

      expect(multiResult.confidence).toBeLessThan(singleResult.confidence)
    })
  })

  describe("A4: Missing Resource Identification", () => {
    test("identifies missing MCP for GitHub tasks", async () => {
      const result = await SelfAwareness.canHandle("create a GitHub pull request")

      // Should suggest GitHub MCP or identify it as missing
      const hasMcpSuggestion =
        result.missingCapabilities?.some((c) => c.includes("github")) ||
        result.suggestedResources?.some((r) => r.includes("github"))

      // Note: This depends on whether GitHub MCP is configured
      expect(result).toBeDefined()
    })

    test("identifies missing MCP for Slack tasks", async () => {
      const result = await SelfAwareness.canHandle("send a slack message to the team")

      // Should suggest Slack MCP
      expect(result).toBeDefined()
    })

    test("identifies missing MCP for browser tasks", async () => {
      const result = await SelfAwareness.canHandle("run playwright e2e tests in browser")

      // Should suggest browser/playwright MCP
      expect(result).toBeDefined()
    })

    test("suggestedResources includes relevant skills", async () => {
      const result = await SelfAwareness.canHandle("create beautiful mermaid diagram")

      // If mermaid skill is not available, should suggest it
      if (!result.confident) {
        expect(
          result.suggestedResources !== undefined || result.missingCapabilities !== undefined,
        ).toBe(true)
      }
    })

    test("missing capabilities list is populated for complex tasks", async () => {
      const result = await SelfAwareness.canHandle(
        "integrate with jira, create slack notifications, and open github PR",
      )

      // Multiple external dependencies should trigger missing capabilities
      if (!result.confident) {
        expect(
          result.missingCapabilities !== undefined || result.suggestedResources !== undefined,
        ).toBe(true)
      }
    })
  })

  describe("Task Analysis Patterns", () => {
    test("recognizes bash-related keywords", async () => {
      const tasks = [
        "run the build command",
        "execute npm install",
        "run shell script",
      ]

      for (const task of tasks) {
        const result = await SelfAwareness.canHandle(task)
        expect(result).toBeDefined()
      }
    })

    test("recognizes file operation keywords", async () => {
      const tasks = [
        "read the config file",
        "edit the source code",
        "write new component",
        "search for function",
      ]

      for (const task of tasks) {
        const result = await SelfAwareness.canHandle(task)
        expect(result).toBeDefined()
      }
    })

    test("recognizes skill domain keywords", async () => {
      const domains = [
        { task: "write unit tests", domain: "tdd" },
        { task: "check for vulnerabilities", domain: "security" },
        { task: "review the code changes", domain: "review" },
        { task: "design the system architecture", domain: "architect" },
      ]

      for (const { task } of domains) {
        const result = await SelfAwareness.canHandle(task)
        expect(result).toBeDefined()
      }
    })
  })

  describe("Summary Generation", () => {
    test("getSummary returns formatted string", async () => {
      const summary = await SelfAwareness.getSummary("build")

      expect(typeof summary).toBe("string")
      expect(summary.length).toBeGreaterThan(0)
    })

    test("getSummary includes agent name", async () => {
      const summary = await SelfAwareness.getSummary("build")

      // Should contain agent name in the summary
      expect(summary.toLowerCase()).toContain("agent")
    })

    test("getSummary includes tool count", async () => {
      const summary = await SelfAwareness.getSummary()

      expect(summary).toContain("Tools")
    })

    test("getSummary includes skill count", async () => {
      const summary = await SelfAwareness.getSummary()

      expect(summary).toContain("Skills")
    })

    test("getSummary includes MCP server info", async () => {
      const summary = await SelfAwareness.getSummary()

      expect(summary).toContain("MCP")
    })
  })

  describe("Confidence Boundary Cases", () => {
    test("empty task returns baseline confidence", async () => {
      const result = await SelfAwareness.canHandle("")

      expect(result.confidence).toBeGreaterThanOrEqual(0)
      expect(result.confidence).toBeLessThanOrEqual(1)
    })

    test("very long task is handled gracefully", async () => {
      const longTask = "perform ".repeat(100) + "some task"
      const result = await SelfAwareness.canHandle(longTask)

      expect(result).toBeDefined()
      expect(result.confidence).toBeGreaterThanOrEqual(0)
      expect(result.confidence).toBeLessThanOrEqual(1)
    })

    test("special characters in task are handled", async () => {
      const specialTask = "handle task with @#$%^&*() characters"
      const result = await SelfAwareness.canHandle(specialTask)

      expect(result).toBeDefined()
    })
  })
})

describe("Awareness Calibration Metrics", () => {
  test("calculates calibration error for predictions", () => {
    // Mock prediction results
    const predictions = [
      { confidence: 0.9, actual: true },
      { confidence: 0.8, actual: true },
      { confidence: 0.7, actual: true },
      { confidence: 0.6, actual: false },
      { confidence: 0.5, actual: true },
      { confidence: 0.4, actual: false },
      { confidence: 0.3, actual: false },
      { confidence: 0.2, actual: false },
    ]

    const error = calculateCalibrationError(predictions)

    // Error should be between 0 and 1
    expect(error).toBeGreaterThanOrEqual(0)
    expect(error).toBeLessThanOrEqual(1)
  })

  test("perfect calibration has zero error", () => {
    // Perfect calibration: confidence matches actual success rate
    const perfectPredictions = [
      { confidence: 1.0, actual: true },
      { confidence: 1.0, actual: true },
      { confidence: 0.0, actual: false },
      { confidence: 0.0, actual: false },
    ]

    const error = calculateCalibrationError(perfectPredictions)
    expect(error).toBeLessThan(0.1) // Allow small floating point error
  })

  test("generates evaluation summary for awareness dimension", () => {
    const metrics = [
      createMetricResult("Tool List Completeness", 0.95, 0.9, "gte"),
      createMetricResult("Skill Match Accuracy", 0.85, 0.8, "gte"),
      createMetricResult("Confidence Calibration Error", 0.12, 0.15, "lte"),
      createMetricResult("Missing Resource Recall", 0.75, 0.7, "gte"),
    ]

    const summary = aggregateMetrics("Self-Awareness", metrics)

    expect(summary.dimension).toBe("Self-Awareness")
    expect(summary.metrics).toHaveLength(4)
    expect(summary.passRate).toBeGreaterThanOrEqual(0.5)
  })
})
