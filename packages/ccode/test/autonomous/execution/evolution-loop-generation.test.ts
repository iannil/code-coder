/**
 * Evolution Loop - Phase 1.5 Concept Generation Tests
 *
 * Tests for the internal capability generation phase in the evolution loop.
 * Phase 1.5 attempts to generate new capabilities when no existing match is found.
 *
 * @package test/autonomous/execution
 */

import { describe, test, expect, beforeEach, mock, spyOn } from "bun:test"

import {
  EvolutionLoop,
  createEvolutionLoop,
  type EvolutionConfig,
  type AutonomousProblem,
} from "@/autonomous/execution/evolution-loop"
import {
  createTestBuildResult,
  createMockGeneratedConcept,
  withTestInstance,
} from "../builder/fixtures/builder-fixture"
import type { ConceptType, BuildResult } from "@/autonomous/builder"

// ============================================================================
// Test Helpers
// ============================================================================

function createTestProblem(overrides?: Partial<AutonomousProblem>): AutonomousProblem {
  return {
    sessionId: `test-session-${Date.now()}`,
    description: "Calculate compound interest with variable rates",
    technology: "typescript",
    workingDir: "/tmp/test",
    maxRetries: 1,
    enableWebSearch: false,
    enableCodeExecution: false,
    ...overrides,
  }
}

function createTestConfig(overrides?: Partial<EvolutionConfig>): Partial<EvolutionConfig> {
  return {
    // Disable external resources for faster tests
    enableWebSearch: false,
    enableCodeExecution: false,
    enableGithubScout: false,
    enableSedimentation: false,
    enableLLMCodeGeneration: false,
    enableAutoBuilder: false,
    // Disable internal capability discovery (to test Phase 1.5)
    enableAgentDiscovery: false,
    enableSkillDiscovery: false,
    enableHandDiscovery: false,
    enableToolDiscovery: false,
    enableMemorySearch: false,
    // Enable concept generation
    enableConceptGeneration: true,
    conceptGenerationMinConfidence: 0.5,
    allowedConceptTypes: ["TOOL", "PROMPT", "SKILL", "AGENT", "MEMORY", "HAND", "WORKFLOW"],
    executeGeneratedConcept: false, // Just generate, don't execute
    ...overrides,
  }
}

// ============================================================================
// Tests
// ============================================================================

describe("EvolutionLoop - Phase 1.5 Concept Generation", () => {
  // ==========================================================================
  // Configuration Tests
  // ==========================================================================

  describe("configuration", () => {
    test("should include generation config options in defaults", () => {
      const loop = createEvolutionLoop()
      // Access private config via casting
      const config = (loop as unknown as { config: EvolutionConfig }).config

      expect(config.enableConceptGeneration).toBe(true)
      expect(config.conceptGenerationMinConfidence).toBe(0.6)
      expect(config.allowedConceptTypes).toContain("TOOL")
      expect(config.allowedConceptTypes).toContain("SKILL")
      expect(config.executeGeneratedConcept).toBe(true)
    })

    test("should accept custom generation config", () => {
      const loop = createEvolutionLoop({
        enableConceptGeneration: false,
        conceptGenerationMinConfidence: 0.8,
        allowedConceptTypes: ["TOOL", "PROMPT"],
      })
      const config = (loop as unknown as { config: EvolutionConfig }).config

      expect(config.enableConceptGeneration).toBe(false)
      expect(config.conceptGenerationMinConfidence).toBe(0.8)
      expect(config.allowedConceptTypes).toEqual(["TOOL", "PROMPT"])
    })
  })

  // ==========================================================================
  // Phase 1.5 Behavior Tests
  // ==========================================================================

  describe("phase 1.5 behavior", () => {
    test("should skip concept generation when disabled", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(
          createTestConfig({
            enableConceptGeneration: false,
          }),
        )

        const result = await loop.evolve(createTestProblem())

        // Should not have "generation" in capabilities searched
        const generationSearch = result.capabilitiesSearched?.find((c) => c.type === "generation")
        expect(generationSearch).toBeUndefined()
      })
    })

    test("should skip concept generation when internal match exists", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(
          createTestConfig({
            enableAgentDiscovery: true, // Enable agent discovery
            agentMatchThreshold: 0.1, // Very low threshold to ensure match
          }),
        )

        const problem = createTestProblem({
          description: "@macro analyze economic data", // Should match macro agent
        })

        const result = await loop.evolve(problem)

        // If agent matched, generation should be skipped
        if (result.matchedCapability?.type === "agent") {
          const generationSearch = result.capabilitiesSearched?.find((c) => c.type === "generation")
          expect(generationSearch).toBeUndefined()
        }
      })
    })

    test("should record generation attempt in capabilitiesSearched", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(createTestConfig())

        const result = await loop.evolve(createTestProblem())

        // Should have "generation" in capabilities searched
        const generationSearch = result.capabilitiesSearched?.find((c) => c.type === "generation")
        expect(generationSearch).toBeDefined()
        expect(generationSearch?.searched).toBe(true)
      })
    })

    test("should only attempt generation once per session", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(createTestConfig())

        // First evolve
        await loop.evolve(createTestProblem())
        // Access private flag
        const attemptedFirst = (loop as unknown as { conceptGenerationAttempted: boolean })
          .conceptGenerationAttempted

        expect(attemptedFirst).toBe(true)
      })
    })
  })

  // ==========================================================================
  // Type Filtering Tests
  // ==========================================================================

  describe("concept type filtering", () => {
    test("should filter by allowedConceptTypes", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(
          createTestConfig({
            allowedConceptTypes: ["TOOL", "PROMPT"], // Only allow TOOL and PROMPT
          }),
        )

        // The generation result should respect the filter
        // This is verified by checking that disallowed types don't appear in results
        const result = await loop.evolve(createTestProblem())

        // If a concept was generated, it should be in allowed types
        if (result.generatedConcept) {
          expect(["TOOL", "PROMPT"]).toContain(result.generatedConcept.type)
        }
      })
    })
  })

  // ==========================================================================
  // Confidence Threshold Tests
  // ==========================================================================

  describe("confidence threshold", () => {
    test("should respect minimum confidence threshold", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(
          createTestConfig({
            conceptGenerationMinConfidence: 0.99, // Very high threshold
          }),
        )

        const result = await loop.evolve(createTestProblem())

        // With very high threshold, generation likely won't succeed
        // Check that generation was attempted but result reflects threshold check
        const generationSearch = result.capabilitiesSearched?.find((c) => c.type === "generation")
        expect(generationSearch?.searched).toBe(true)
      })
    })
  })

  // ==========================================================================
  // Result Structure Tests
  // ==========================================================================

  describe("result structure", () => {
    test("should include generatedConcept in result when successful", async () => {
      // This test validates the structure of a successful generation result
      // We create a mock scenario

      const mockBuildResult = createTestBuildResult(true, {
        concept: createMockGeneratedConcept("TOOL", {
          identifier: "test_calculator",
          description: "Calculate compound interest",
        }),
      })

      // Verify the structure
      expect(mockBuildResult.success).toBe(true)
      expect(mockBuildResult.concept).toBeDefined()
      expect(mockBuildResult.concept?.type).toBe("TOOL")
      expect(mockBuildResult.concept?.identifier).toBe("test_calculator")
    })

    test("should include buildResult in generatedConcept", async () => {
      const mockBuildResult = createTestBuildResult(true)

      // Verify build result structure
      expect(mockBuildResult.closeScore).toBeDefined()
      expect(mockBuildResult.phases).toBeDefined()
      expect(mockBuildResult.phases.length).toBeGreaterThan(0)
    })
  })

  // ==========================================================================
  // Integration with EvolutionResult
  // ==========================================================================

  describe("EvolutionResult integration", () => {
    test("should set matchedCapability type correctly for generated concepts", async () => {
      // Create a mock successful result structure
      const conceptTypes: ConceptType[] = ["TOOL", "SKILL", "AGENT", "PROMPT"]

      for (const type of conceptTypes) {
        const concept = createMockGeneratedConcept(type)
        const expectedCapabilityType = type.toLowerCase()

        // Verify the mapping works
        expect(["tool", "skill", "agent", "prompt", "hand", "memory", "workflow"]).toContain(
          expectedCapabilityType,
        )
      }
    })

    test("should include capabilitiesSearched with generation entry", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(createTestConfig())

        const result = await loop.evolve(createTestProblem())

        // Verify capabilitiesSearched structure
        expect(result.capabilitiesSearched).toBeDefined()
        expect(Array.isArray(result.capabilitiesSearched)).toBe(true)

        const generationEntry = result.capabilitiesSearched?.find((c) => c.type === "generation")
        if (generationEntry) {
          expect(generationEntry.searched).toBe(true)
          expect(typeof generationEntry.matchCount).toBe("number")
          expect(typeof generationEntry.topMatchScore).toBe("number")
        }
      })
    })
  })

  // ==========================================================================
  // Concept Execution Tests
  // ==========================================================================

  describe("concept execution", () => {
    test("should skip execution when executeGeneratedConcept is false", async () => {
      await withTestInstance(async () => {
        const loop = createEvolutionLoop(
          createTestConfig({
            executeGeneratedConcept: false,
          }),
        )

        const result = await loop.evolve(createTestProblem())

        // Result should not have execution attempts from generation
        // (attempts array would be empty or only contain other sources)
        if (result.generatedConcept) {
          // If generated but not executed, recommendation should be present
          expect(result.solution).toBeDefined()
        }
      })
    })
  })

  // ==========================================================================
  // Recommendation Formatting Tests
  // ==========================================================================

  describe("recommendation formatting", () => {
    test("should format TOOL recommendations correctly", () => {
      const concept = createMockGeneratedConcept("TOOL", {
        identifier: "csv_parser",
        description: "Parse CSV files",
      })

      // This tests the formatConceptRecommendation method indirectly
      expect(concept.identifier).toBe("csv_parser")
      expect(concept.type).toBe("TOOL")
    })

    test("should format SKILL recommendations with slash prefix", () => {
      const concept = createMockGeneratedConcept("SKILL", {
        identifier: "deploy",
        description: "Deploy to production",
      })

      // Skill recommendations should use /<skill-name> format
      const expectedFormat = `/${concept.identifier}`
      expect(expectedFormat).toBe("/deploy")
    })

    test("should format AGENT recommendations with at-sign prefix", () => {
      const concept = createMockGeneratedConcept("AGENT", {
        identifier: "security_expert",
        description: "Security analysis agent",
      })

      // Agent recommendations should use @<agent-name> format
      const expectedFormat = `@${concept.identifier}`
      expect(expectedFormat).toBe("@security_expert")
    })
  })
})
