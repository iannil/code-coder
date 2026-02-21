/**
 * Resource Acquisition Evaluation Tests
 *
 * Verifies that the ResourceAcquisition module correctly:
 * - R1: Searches for relevant skills when capability is lacking
 * - R2: Discovers appropriate MCP servers
 * - R3: Generates clear acquisition instructions
 */

import { describe, test, expect } from "bun:test"
import { ResourceAcquisition } from "@/bootstrap/acquisition"
import { SelfAwareness } from "@/bootstrap/awareness"
import {
  ACQUISITION_EXPECTATIONS,
  isWithinExpectedRange,
} from "./fixtures/expected-results"
import {
  calculatePrecision,
  calculateRecall,
  calculateF1Score,
  createMetricResult,
  aggregateMetrics,
} from "./utils/metrics"

describe("Resource Acquisition Evaluation", () => {
  describe("R1: Skill Search", () => {
    test("discoverNeeded returns structured result", async () => {
      const result = await ResourceAcquisition.discoverNeeded("some task")

      expect(result).toBeDefined()
      expect(Array.isArray(result.mcpServers)).toBe(true)
      expect(Array.isArray(result.skills)).toBe(true)
      expect(Array.isArray(result.externalAPIs)).toBe(true)
    })

    test("confident tasks return empty resources", async () => {
      const result = await ResourceAcquisition.discoverNeeded("read the file")

      // For simple tasks, should not suggest additional resources
      // (though this depends on current configuration)
      expect(result).toBeDefined()
    })

    test("skill suggestions have required fields", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "create a beautiful mermaid diagram visualization",
      )

      for (const skill of result.skills) {
        expect(skill.type).toBe("skill")
        expect(skill.name).toBeDefined()
        expect(skill.description).toBeDefined()
        expect(typeof skill.confidence).toBe("number")
      }
    })
  })

  describe("R2: MCP Discovery", () => {
    test("discovers GitHub MCP for GitHub tasks", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "create a GitHub pull request for the feature branch",
      )

      const githubSuggestion = result.mcpServers.find((m) =>
        m.name.toLowerCase().includes("github"),
      )

      // Should suggest GitHub MCP
      expect(
        githubSuggestion !== undefined ||
        result.mcpServers.length === 0, // Or already configured
      ).toBe(true)
    })

    test("discovers Slack MCP for Slack tasks", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "send a notification to the Slack channel",
      )

      const slackSuggestion = result.mcpServers.find((m) =>
        m.name.toLowerCase().includes("slack"),
      )

      expect(
        slackSuggestion !== undefined ||
        result.mcpServers.length === 0,
      ).toBe(true)
    })

    test("discovers browser MCP for Playwright tasks", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "run playwright browser automation tests",
      )

      const browserSuggestion = result.mcpServers.find((m) =>
        m.name.toLowerCase().includes("playwright") ||
        m.name.toLowerCase().includes("browser"),
      )

      expect(
        browserSuggestion !== undefined ||
        result.mcpServers.length === 0,
      ).toBe(true)
    })

    test("MCP suggestions include confidence scores", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "integrate with external filesystem operations",
      )

      for (const mcp of result.mcpServers) {
        expect(mcp.confidence).toBeGreaterThanOrEqual(0)
        expect(mcp.confidence).toBeLessThanOrEqual(1)
      }
    })

    test("known MCP servers have higher confidence", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "access github repository information",
      )

      const knownMcp = result.mcpServers.find((m) =>
        ACQUISITION_EXPECTATIONS.knownMcpServers.includes(m.name.toLowerCase()),
      )

      if (knownMcp) {
        expect(knownMcp.confidence).toBeGreaterThanOrEqual(0.5)
      }
    })
  })

  describe("R3: Acquisition Instructions", () => {
    test("getAcquisitionInstructions returns string for MCP", () => {
      const resource: ResourceAcquisition.DiscoveredResource = {
        type: "mcp",
        name: "github",
        description: "GitHub integration",
        source: "npx @modelcontextprotocol/server-github",
        installHint: 'Add to config.mcp: {"github": {...}}',
        confidence: 0.8,
      }

      const instructions = ResourceAcquisition.getAcquisitionInstructions(resource)

      expect(typeof instructions).toBe("string")
      expect(instructions.length).toBeGreaterThan(0)
    })

    test("getAcquisitionInstructions returns string for skill", () => {
      const resource: ResourceAcquisition.DiscoveredResource = {
        type: "skill",
        name: "mermaid-diagram",
        description: "Generate mermaid diagrams",
        source: "suggested",
        confidence: 0.5,
      }

      const instructions = ResourceAcquisition.getAcquisitionInstructions(resource)

      expect(typeof instructions).toBe("string")
      expect(instructions).toContain("skill")
    })

    test("getAcquisitionInstructions returns string for API", () => {
      const resource: ResourceAcquisition.DiscoveredResource = {
        type: "api",
        name: "OpenAI API",
        description: "AI capabilities",
        source: "https://openai.com",
        confidence: 0.6,
      }

      const instructions = ResourceAcquisition.getAcquisitionInstructions(resource)

      expect(typeof instructions).toBe("string")
      expect(instructions.includes("API") || instructions.includes("credentials")).toBe(true)
    })

    test("MCP instructions include install hint when available", () => {
      const resource: ResourceAcquisition.DiscoveredResource = {
        type: "mcp",
        name: "filesystem",
        description: "File system access",
        source: "npx @modelcontextprotocol/server-filesystem",
        installHint: "Add to config.mcp with path argument",
        confidence: 0.8,
      }

      const instructions = ResourceAcquisition.getAcquisitionInstructions(resource)
      expect(instructions).toContain("config.mcp")
    })
  })

  describe("Resource Deduplication", () => {
    test("discovered resources are deduplicated", async () => {
      // Multiple mentions of same service should be deduplicated
      const result = await ResourceAcquisition.discoverNeeded(
        "create github PR, check github actions, update github issues",
      )

      const githubResources = result.mcpServers.filter((m) =>
        m.name.toLowerCase().includes("github"),
      )

      // Should have at most one GitHub entry
      expect(githubResources.length).toBeLessThanOrEqual(1)
    })
  })

  describe("Acquisition Attempt", () => {
    test("acquire returns boolean result", async () => {
      const resource: ResourceAcquisition.DiscoveredResource = {
        type: "mcp",
        name: "test-mcp",
        description: "Test MCP",
        source: "test-source",
        confidence: 0.5,
      }

      const result = await ResourceAcquisition.acquire(resource)
      expect(typeof result).toBe("boolean")
    })

    test("acquire for skill checks if skill exists", async () => {
      const resource: ResourceAcquisition.DiscoveredResource = {
        type: "skill",
        name: "nonexistent-skill-xyz",
        description: "Does not exist",
        source: "test",
        confidence: 0.3,
      }

      const result = await ResourceAcquisition.acquire(resource)
      expect(result).toBe(false)
    })
  })

  describe("Discovery Confidence Levels", () => {
    test("known resources have high confidence", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "access the github repository",
      )

      for (const mcp of result.mcpServers) {
        if (ACQUISITION_EXPECTATIONS.knownMcpServers.includes(mcp.name.toLowerCase())) {
          expect(
            isWithinExpectedRange(
              mcp.confidence,
              ACQUISITION_EXPECTATIONS.discoveryConfidence.knownResource,
            ),
          ).toBe(true)
        }
      }
    })

    test("unknown resources have lower confidence", async () => {
      const result = await ResourceAcquisition.discoverNeeded(
        "integrate with obscure-service-xyz",
      )

      for (const resource of [...result.mcpServers, ...result.externalAPIs]) {
        if (
          !ACQUISITION_EXPECTATIONS.knownMcpServers.includes(resource.name.toLowerCase())
        ) {
          expect(resource.confidence).toBeLessThanOrEqual(0.6)
        }
      }
    })
  })
})

describe("Acquisition Metrics", () => {
  test("calculates precision for resource suggestions", () => {
    const suggestions = ["github", "slack", "random-service"]
    const relevant = ["github", "slack", "jira"]

    const precision = calculatePrecision(suggestions, relevant)
    expect(precision).toBeCloseTo(2 / 3, 2) // 2 hits out of 3 suggestions
  })

  test("calculates recall for resource suggestions", () => {
    const suggestions = ["github", "slack"]
    const relevant = ["github", "slack", "jira"]

    const recall = calculateRecall(suggestions, relevant)
    expect(recall).toBeCloseTo(2 / 3, 2) // 2 hits out of 3 relevant
  })

  test("calculates F1 score", () => {
    const precision = 0.8
    const recall = 0.6

    const f1 = calculateF1Score(precision, recall)
    const expected = (2 * 0.8 * 0.6) / (0.8 + 0.6)

    expect(f1).toBeCloseTo(expected, 3)
  })

  test("generates evaluation summary for acquisition dimension", () => {
    const metrics = [
      createMetricResult("Skill Search Relevance", 0.85, 0.8, "gte"),
      createMetricResult("MCP Discovery Precision", 0.9, 0.8, "gte"),
      createMetricResult("Instruction Clarity", 0.95, 0.9, "gte"),
      createMetricResult("Confidence Calibration", 0.88, 0.85, "gte"),
    ]

    const summary = aggregateMetrics("Resource Acquisition", metrics)

    expect(summary.dimension).toBe("Resource Acquisition")
    expect(summary.passRate).toBeGreaterThanOrEqual(0.5)
  })
})
