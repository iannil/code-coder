/**
 * Types Module Tests
 *
 * Tests for types.ts helper functions and type guards.
 *
 * @package test/autonomous/builder
 */

import { describe, test, expect } from "bun:test"

import {
  ConceptTypeSchema,
  CONCEPT_METADATA,
  AUTONOMY_CONCEPT_GATES,
  isConceptAllowed,
  getMinimumAutonomyLevel,
  createSelfBuildingCriteria,
} from "@/autonomous/builder"

import { createTestGap, createTestBuildContext } from "./fixtures/builder-fixture"

describe("Types Module", () => {
  // ==========================================================================
  // ConceptTypeSchema
  // ==========================================================================

  describe("ConceptTypeSchema", () => {
    test("should validate all 7 concept types", () => {
      const validTypes = ["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY", "WORKFLOW"]

      for (const type of validTypes) {
        const result = ConceptTypeSchema.safeParse(type)
        expect(result.success).toBe(true)
      }
    })

    test("should reject invalid concept types", () => {
      const invalidTypes = ["agent", "PLUGIN", "SERVICE", "", null, 123]

      for (const type of invalidTypes) {
        const result = ConceptTypeSchema.safeParse(type)
        expect(result.success).toBe(false)
      }
    })
  })

  // ==========================================================================
  // CONCEPT_METADATA
  // ==========================================================================

  describe("CONCEPT_METADATA", () => {
    test("should have metadata for all concept types", () => {
      const types = ["AGENT", "PROMPT", "SKILL", "TOOL", "HAND", "MEMORY", "WORKFLOW"] as const

      for (const type of types) {
        expect(CONCEPT_METADATA[type]).toBeDefined()
        expect(CONCEPT_METADATA[type].riskLevel).toBeDefined()
        expect(CONCEPT_METADATA[type].requiresApproval).toBeDefined()
        expect(CONCEPT_METADATA[type].autoApprovable).toBeDefined()
        expect(CONCEPT_METADATA[type].description).toBeDefined()
      }
    })

    test("should have correct risk levels", () => {
      // Low risk concepts
      expect(CONCEPT_METADATA.TOOL.riskLevel).toBe("low")
      expect(CONCEPT_METADATA.PROMPT.riskLevel).toBe("low")
      expect(CONCEPT_METADATA.SKILL.riskLevel).toBe("low")

      // Medium risk concepts
      expect(CONCEPT_METADATA.AGENT.riskLevel).toBe("medium")
      expect(CONCEPT_METADATA.MEMORY.riskLevel).toBe("medium")

      // High risk concepts
      expect(CONCEPT_METADATA.HAND.riskLevel).toBe("high")
      expect(CONCEPT_METADATA.WORKFLOW.riskLevel).toBe("high")
    })

    test("should mark high-risk concepts as requiring approval", () => {
      expect(CONCEPT_METADATA.AGENT.requiresApproval).toBe(true)
      expect(CONCEPT_METADATA.HAND.requiresApproval).toBe(true)
      expect(CONCEPT_METADATA.WORKFLOW.requiresApproval).toBe(true)
    })

    test("should mark low-risk concepts as auto-approvable", () => {
      expect(CONCEPT_METADATA.TOOL.autoApprovable).toBe(true)
      expect(CONCEPT_METADATA.PROMPT.autoApprovable).toBe(true)
      expect(CONCEPT_METADATA.SKILL.autoApprovable).toBe(true)
    })
  })

  // ==========================================================================
  // AUTONOMY_CONCEPT_GATES
  // ==========================================================================

  describe("AUTONOMY_CONCEPT_GATES", () => {
    test("should have gates for all autonomy levels", () => {
      const levels = ["timid", "bold", "wild", "crazy", "insane", "lunatic"] as const

      for (const level of levels) {
        expect(AUTONOMY_CONCEPT_GATES[level]).toBeDefined()
        expect(Array.isArray(AUTONOMY_CONCEPT_GATES[level])).toBe(true)
      }
    })

    test("should allow more concepts at higher autonomy levels", () => {
      expect(AUTONOMY_CONCEPT_GATES.timid.length).toBeLessThan(
        AUTONOMY_CONCEPT_GATES.bold.length,
      )
      expect(AUTONOMY_CONCEPT_GATES.bold.length).toBeLessThanOrEqual(
        AUTONOMY_CONCEPT_GATES.wild.length,
      )
      expect(AUTONOMY_CONCEPT_GATES.wild.length).toBeLessThanOrEqual(
        AUTONOMY_CONCEPT_GATES.crazy.length,
      )
      expect(AUTONOMY_CONCEPT_GATES.crazy.length).toBeLessThanOrEqual(
        AUTONOMY_CONCEPT_GATES.insane.length,
      )
      expect(AUTONOMY_CONCEPT_GATES.insane.length).toBeLessThanOrEqual(
        AUTONOMY_CONCEPT_GATES.lunatic.length,
      )
    })

    test("timid should only allow TOOL", () => {
      expect(AUTONOMY_CONCEPT_GATES.timid).toEqual(["TOOL"])
    })

    test("lunatic should allow all concept types", () => {
      expect(AUTONOMY_CONCEPT_GATES.lunatic).toContain("AGENT")
      expect(AUTONOMY_CONCEPT_GATES.lunatic).toContain("PROMPT")
      expect(AUTONOMY_CONCEPT_GATES.lunatic).toContain("SKILL")
      expect(AUTONOMY_CONCEPT_GATES.lunatic).toContain("TOOL")
      expect(AUTONOMY_CONCEPT_GATES.lunatic).toContain("HAND")
      expect(AUTONOMY_CONCEPT_GATES.lunatic).toContain("MEMORY")
      expect(AUTONOMY_CONCEPT_GATES.lunatic).toContain("WORKFLOW")
    })
  })

  // ==========================================================================
  // isConceptAllowed
  // ==========================================================================

  describe("isConceptAllowed", () => {
    test("should allow TOOL at all autonomy levels", () => {
      const levels = ["timid", "bold", "wild", "crazy", "insane", "lunatic"] as const
      for (const level of levels) {
        expect(isConceptAllowed("TOOL", level)).toBe(true)
      }
    })

    test("should not allow AGENT at timid level", () => {
      expect(isConceptAllowed("AGENT", "timid")).toBe(false)
    })

    test("should not allow HAND at bold level", () => {
      expect(isConceptAllowed("HAND", "bold")).toBe(false)
    })

    test("should allow WORKFLOW only at lunatic level", () => {
      expect(isConceptAllowed("WORKFLOW", "timid")).toBe(false)
      expect(isConceptAllowed("WORKFLOW", "bold")).toBe(false)
      expect(isConceptAllowed("WORKFLOW", "wild")).toBe(false)
      expect(isConceptAllowed("WORKFLOW", "crazy")).toBe(false)
      expect(isConceptAllowed("WORKFLOW", "insane")).toBe(false)
      expect(isConceptAllowed("WORKFLOW", "lunatic")).toBe(true)
    })

    test("should allow AGENT only at insane level or above", () => {
      expect(isConceptAllowed("AGENT", "timid")).toBe(false)
      expect(isConceptAllowed("AGENT", "bold")).toBe(false)
      expect(isConceptAllowed("AGENT", "wild")).toBe(false)
      expect(isConceptAllowed("AGENT", "crazy")).toBe(false)
      expect(isConceptAllowed("AGENT", "insane")).toBe(true)
      expect(isConceptAllowed("AGENT", "lunatic")).toBe(true)
    })
  })

  // ==========================================================================
  // getMinimumAutonomyLevel
  // ==========================================================================

  describe("getMinimumAutonomyLevel", () => {
    test("should return timid for TOOL", () => {
      expect(getMinimumAutonomyLevel("TOOL")).toBe("timid")
    })

    test("should return bold for PROMPT", () => {
      expect(getMinimumAutonomyLevel("PROMPT")).toBe("bold")
    })

    test("should return wild for SKILL", () => {
      expect(getMinimumAutonomyLevel("SKILL")).toBe("wild")
    })

    test("should return crazy for HAND", () => {
      expect(getMinimumAutonomyLevel("HAND")).toBe("crazy")
    })

    test("should return insane for AGENT", () => {
      expect(getMinimumAutonomyLevel("AGENT")).toBe("insane")
    })
  })

  // ==========================================================================
  // createSelfBuildingCriteria
  // ==========================================================================

  describe("createSelfBuildingCriteria", () => {
    test("should create valid criteria from gap and context", () => {
      const gap = createTestGap({ type: "TOOL" })
      const context = createTestBuildContext()

      const criteria = createSelfBuildingCriteria(gap, context)

      expect(criteria.type).toBe("feature")
      expect(criteria.description).toContain("Build new TOOL")
      expect(criteria.riskLevel).toBe("low")
    })

    test("should adjust criteria based on risk level", () => {
      const lowRiskGap = createTestGap({ type: "TOOL" })
      const highRiskGap = createTestGap({ type: "HAND" })
      const context = createTestBuildContext()

      const lowRiskCriteria = createSelfBuildingCriteria(lowRiskGap, context)
      const highRiskCriteria = createSelfBuildingCriteria(highRiskGap, context)

      expect(lowRiskCriteria.riskLevel).toBe("low")
      expect(highRiskCriteria.riskLevel).toBe("high")

      // Higher risk should have lower convergence (less reversible)
      expect(lowRiskCriteria.convergence).toBeGreaterThan(highRiskCriteria.convergence!)
    })

    test("should include gap metadata in criteria", () => {
      const gap = createTestGap({
        type: "SKILL",
        confidence: 0.9,
      })
      const context = createTestBuildContext()

      const criteria = createSelfBuildingCriteria(gap, context)

      expect(criteria.metadata).toBeDefined()
      expect(criteria.metadata!.gapId).toBe(gap.id)
      expect(criteria.metadata!.conceptType).toBe("SKILL")
      expect(criteria.metadata!.confidence).toBe(0.9)
    })

    test("should set leverage to 8 for all concept types", () => {
      const types = ["TOOL", "SKILL", "AGENT", "HAND"] as const

      for (const type of types) {
        const gap = createTestGap({ type })
        const context = createTestBuildContext()
        const criteria = createSelfBuildingCriteria(gap, context)

        expect(criteria.leverage).toBe(8)
      }
    })

    test("should set evolution to 7 for learning value", () => {
      const gap = createTestGap()
      const context = createTestBuildContext()

      const criteria = createSelfBuildingCriteria(gap, context)

      expect(criteria.evolution).toBe(7)
    })

    test("should scale surplus based on confidence", () => {
      const lowConfidenceGap = createTestGap({ confidence: 0.5 })
      const highConfidenceGap = createTestGap({ confidence: 1.0 })
      const context = createTestBuildContext()

      const lowConfidenceCriteria = createSelfBuildingCriteria(lowConfidenceGap, context)
      const highConfidenceCriteria = createSelfBuildingCriteria(highConfidenceGap, context)

      expect(highConfidenceCriteria.surplus).toBeGreaterThan(lowConfidenceCriteria.surplus!)
    })
  })
})
