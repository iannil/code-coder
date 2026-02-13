import { describe, test, expect, beforeEach } from "bun:test"
import {
  ExpansionState,
  VALID_EXPANSION_TRANSITIONS,
  isValidExpansionTransition,
  getExpansionPhase,
  createContext,
} from "@/autonomous/expansion/states"
import { ExpansionOrchestrator } from "@/autonomous/expansion/orchestrator"
import { KnowledgeSchema } from "@/document/knowledge/schema"

describe("Autonomous Expansion Module", () => {
  describe("Expansion States", () => {
    test("should have 15 unique states", () => {
      const allStates = Object.values(ExpansionState)
      expect(allStates.length).toBe(15)
    })

    test("should have all required states defined", () => {
      expect(ExpansionState.EXPANSION_IDLE).toBeDefined()
      expect(ExpansionState.EXPANSION_ANALYZING).toBeDefined()
      expect(ExpansionState.EXPANSION_ANALYSIS_COMPLETE).toBeDefined()
      expect(ExpansionState.EXPANSION_BUILDING).toBeDefined()
      expect(ExpansionState.EXPANSION_FRAMEWORK_COMPLETE).toBeDefined()
      expect(ExpansionState.EXPANSION_OUTLINING).toBeDefined()
      expect(ExpansionState.EXPANSION_OUTLINE_COMPLETE).toBeDefined()
      expect(ExpansionState.EXPANSION_WRITING).toBeDefined()
      expect(ExpansionState.EXPANSION_CHAPTER_COMPLETE).toBeDefined()
      expect(ExpansionState.EXPANSION_WRITING_COMPLETE).toBeDefined()
      expect(ExpansionState.EXPANSION_VALIDATING).toBeDefined()
      expect(ExpansionState.EXPANSION_VALIDATION_COMPLETE).toBeDefined()
      expect(ExpansionState.EXPANSION_COMPLETE).toBeDefined()
      expect(ExpansionState.EXPANSION_FAILED).toBeDefined()
      expect(ExpansionState.EXPANSION_PAUSED).toBeDefined()
    })

    test("should detect correct phase for analyzing state", () => {
      const phase = getExpansionPhase(ExpansionState.EXPANSION_ANALYZING)
      expect(phase).toBe("idea_analysis")
    })

    test("should detect correct phase for building state", () => {
      const phase = getExpansionPhase(ExpansionState.EXPANSION_BUILDING)
      expect(phase).toBe("framework_building")
    })

    test("should detect correct phase for outlining state", () => {
      const phase = getExpansionPhase(ExpansionState.EXPANSION_OUTLINING)
      expect(phase).toBe("outline_generation")
    })

    test("should detect correct phase for writing state", () => {
      const phase = getExpansionPhase(ExpansionState.EXPANSION_WRITING)
      expect(phase).toBe("iterative_writing")
    })

    test("should detect correct phase for validating state", () => {
      const phase = getExpansionPhase(ExpansionState.EXPANSION_VALIDATING)
      expect(phase).toBe("consistency_validation")
    })

    test("should detect terminal phase for complete state", () => {
      const phase = getExpansionPhase(ExpansionState.EXPANSION_COMPLETE)
      expect(phase).toBe("terminal")
    })
  })

  describe("Expansion State Transitions", () => {
    test("should allow transition from IDLE to ANALYZING", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_IDLE,
        ExpansionState.EXPANSION_ANALYZING,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from ANALYZING to ANALYSIS_COMPLETE", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_ANALYZING,
        ExpansionState.EXPANSION_ANALYSIS_COMPLETE,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from ANALYSIS_COMPLETE to BUILDING", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_ANALYSIS_COMPLETE,
        ExpansionState.EXPANSION_BUILDING,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from BUILDING to FRAMEWORK_COMPLETE", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_BUILDING,
        ExpansionState.EXPANSION_FRAMEWORK_COMPLETE,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from FRAMEWORK_COMPLETE to OUTLINING", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_FRAMEWORK_COMPLETE,
        ExpansionState.EXPANSION_OUTLINING,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from OUTLINING to OUTLINE_COMPLETE", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_OUTLINING,
        ExpansionState.EXPANSION_OUTLINE_COMPLETE,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from OUTLINE_COMPLETE to WRITING", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_OUTLINE_COMPLETE,
        ExpansionState.EXPANSION_WRITING,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from WRITING to CHAPTER_COMPLETE", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_WRITING,
        ExpansionState.EXPANSION_CHAPTER_COMPLETE,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from WRITING_COMPLETE to VALIDATING", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_WRITING_COMPLETE,
        ExpansionState.EXPANSION_VALIDATING,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from VALIDATING to VALIDATION_COMPLETE", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_VALIDATING,
        ExpansionState.EXPANSION_VALIDATION_COMPLETE,
      )
      expect(isValid).toBe(true)
    })

    test("should allow transition from VALIDATION_COMPLETE to COMPLETE", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_VALIDATION_COMPLETE,
        ExpansionState.EXPANSION_COMPLETE,
      )
      expect(isValid).toBe(true)
    })

    test("should reject invalid transition from IDLE to WRITING", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_IDLE,
        ExpansionState.EXPANSION_WRITING,
      )
      expect(isValid).toBe(false)
    })

    test("should reject invalid transition from ANALYZING to COMPLETE", () => {
      const isValid = isValidExpansionTransition(
        ExpansionState.EXPANSION_ANALYZING,
        ExpansionState.EXPANSION_COMPLETE,
      )
      expect(isValid).toBe(false)
    })
  })

  describe("Expansion Context", () => {
    test("should create context with required fields", () => {
      const context = createContext({
        documentID: "doc_123",
        coreIdea: "Test idea for expansion",
        targetWords: 50000,
        contentType: "auto",
        autonomy: "stage-confirm",
      })

      expect(context.documentID).toBe("doc_123")
      expect(context.coreIdea).toBe("Test idea for expansion")
      expect(context.targetWords).toBe(50000)
      expect(context.contentType).toBe("auto")
      expect(context.autonomy).toBe("stage-confirm")
      expect(context.currentState).toBe(ExpansionState.EXPANSION_IDLE)
      expect(context.startedAt).toBeGreaterThanOrEqual(0)
      expect(context.updatedAt).toBeGreaterThanOrEqual(0)
    })

    test("should create context with fiction content type", () => {
      const context = createContext({
        documentID: "doc_123",
        coreIdea: "A story about a hero",
        targetWords: 80000,
        contentType: "fiction",
        autonomy: "autonomous",
      })

      expect(context.contentType).toBe("fiction")
      expect(context.autonomy).toBe("autonomous")
    })

    test("should create context with nonfiction content type", () => {
      const context = createContext({
        documentID: "doc_123",
        coreIdea: "An analysis of modern economics",
        targetWords: 60000,
        contentType: "nonfiction",
        autonomy: "interactive",
      })

      expect(context.contentType).toBe("nonfiction")
      expect(context.autonomy).toBe("interactive")
    })
  })

  describe("Orchestrator", () => {
    test("should have pause and resume functions", () => {
      expect(ExpansionOrchestrator.pause).toBeFunction()
      expect(ExpansionOrchestrator.resume).toBeFunction()
      expect(ExpansionOrchestrator.cancel).toBeFunction()
      expect(ExpansionOrchestrator.getProgress).toBeFunction()
    })

    test("should have run function with correct signature", () => {
      expect(ExpansionOrchestrator.run).toBeFunction()
      // Async function - tested in integration tests
    })
  })
})
