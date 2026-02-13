import { KnowledgeSchema } from "../../document/knowledge/schema"

/**
 * Expansion-specific states for the autonomous book-writer workflow.
 */
export enum ExpansionState {
  // Initial state
  EXPANSION_IDLE = "expansion_idle",

  // Phase 1: Idea Analysis
  EXPANSION_ANALYZING = "expansion_analyzing",
  EXPANSION_ANALYSIS_COMPLETE = "expansion_analysis_complete",

  // Phase 2: Framework Building
  EXPANSION_BUILDING = "expansion_building",
  EXPANSION_FRAMEWORK_COMPLETE = "expansion_framework_complete",

  // Phase 3: Outline Generation
  EXPANSION_OUTLINING = "expansion_outlining",
  EXPANSION_OUTLINE_COMPLETE = "expansion_outline_complete",

  // Phase 4: Iterative Writing
  EXPANSION_WRITING = "expansion_writing",
  EXPANSION_CHAPTER_COMPLETE = "expansion_chapter_complete",
  EXPANSION_WRITING_COMPLETE = "expansion_writing_complete",

  // Phase 5: Consistency Validation
  EXPANSION_VALIDATING = "expansion_validating",
  EXPANSION_VALIDATION_COMPLETE = "expansion_validation_complete",

  // Terminal states
  EXPANSION_COMPLETE = "expansion_complete",
  EXPANSION_FAILED = "expansion_failed",
  EXPANSION_PAUSED = "expansion_paused",
}

/**
 * Valid transitions between expansion states.
 */
export const VALID_EXPANSION_TRANSITIONS: Record<ExpansionState, ExpansionState[]> = {
  [ExpansionState.EXPANSION_IDLE]: [
    ExpansionState.EXPANSION_ANALYZING,
    ExpansionState.EXPANSION_FAILED,
  ],

  [ExpansionState.EXPANSION_ANALYZING]: [
    ExpansionState.EXPANSION_ANALYSIS_COMPLETE,
    ExpansionState.EXPANSION_FAILED,
  ],

  [ExpansionState.EXPANSION_ANALYSIS_COMPLETE]: [
    ExpansionState.EXPANSION_BUILDING,
    ExpansionState.EXPANSION_PAUSED,
    ExpansionState.EXPANSION_FAILED,
  ],

  [ExpansionState.EXPANSION_BUILDING]: [
    ExpansionState.EXPANSION_FRAMEWORK_COMPLETE,
    ExpansionState.EXPANSION_PAUSED,
    ExpansionState.EXPANSION_FAILED,
  ],

  [ExpansionState.EXPANSION_FRAMEWORK_COMPLETE]: [
    ExpansionState.EXPANSION_OUTLINING,
    ExpansionState.EXPANSION_PAUSED,
  ],

  [ExpansionState.EXPANSION_OUTLINING]: [
    ExpansionState.EXPANSION_OUTLINE_COMPLETE,
    ExpansionState.EXPANSION_FAILED,
  ],

  [ExpansionState.EXPANSION_OUTLINE_COMPLETE]: [
    ExpansionState.EXPANSION_WRITING,
    ExpansionState.EXPANSION_PAUSED,
  ],

  [ExpansionState.EXPANSION_WRITING]: [
    ExpansionState.EXPANSION_CHAPTER_COMPLETE,
    ExpansionState.EXPANSION_WRITING_COMPLETE,
    ExpansionState.EXPANSION_PAUSED,
    ExpansionState.EXPANSION_FAILED,
  ],

  [ExpansionState.EXPANSION_CHAPTER_COMPLETE]: [
    ExpansionState.EXPANSION_WRITING,
    ExpansionState.EXPANSION_VALIDATING,
    ExpansionState.EXPANSION_PAUSED,
  ],

  [ExpansionState.EXPANSION_WRITING_COMPLETE]: [
    ExpansionState.EXPANSION_VALIDATING,
    ExpansionState.EXPANSION_COMPLETE,
  ],

  [ExpansionState.EXPANSION_VALIDATING]: [
    ExpansionState.EXPANSION_VALIDATION_COMPLETE,
    ExpansionState.EXPANSION_WRITING, // Go back to fix issues
    ExpansionState.EXPANSION_FAILED,
  ],

  [ExpansionState.EXPANSION_VALIDATION_COMPLETE]: [
    ExpansionState.EXPANSION_COMPLETE,
    ExpansionState.EXPANSION_WRITING, // More chapters to write
  ],

  [ExpansionState.EXPANSION_COMPLETE]: [
    ExpansionState.EXPANSION_IDLE,
  ],

  [ExpansionState.EXPANSION_FAILED]: [
    ExpansionState.EXPANSION_IDLE,
  ],

  [ExpansionState.EXPANSION_PAUSED]: [
    ExpansionState.EXPANSION_ANALYZING,
    ExpansionState.EXPANSION_BUILDING,
    ExpansionState.EXPANSION_OUTLINING,
    ExpansionState.EXPANSION_WRITING,
    ExpansionState.EXPANSION_VALIDATING,
    ExpansionState.EXPANSION_COMPLETE,
    ExpansionState.EXPANSION_FAILED,
  ],
}

/**
 * Check if a transition between expansion states is valid.
 */
export function isValidExpansionTransition(
  from: ExpansionState,
  to: ExpansionState,
): boolean {
  return VALID_EXPANSION_TRANSITIONS[from]?.includes(to) ?? false
}

/**
 * Get the phase for a given expansion state.
 */
export function getExpansionPhase(state: ExpansionState): "idea_analysis" | "framework_building" | "outline_generation" | "iterative_writing" | "consistency_validation" | "terminal" {
  switch (state) {
    case ExpansionState.EXPANSION_ANALYZING:
    case ExpansionState.EXPANSION_ANALYSIS_COMPLETE:
      return "idea_analysis"

    case ExpansionState.EXPANSION_BUILDING:
    case ExpansionState.EXPANSION_FRAMEWORK_COMPLETE:
      return "framework_building"

    case ExpansionState.EXPANSION_OUTLINING:
    case ExpansionState.EXPANSION_OUTLINE_COMPLETE:
      return "outline_generation"

    case ExpansionState.EXPANSION_WRITING:
    case ExpansionState.EXPANSION_CHAPTER_COMPLETE:
    case ExpansionState.EXPANSION_WRITING_COMPLETE:
      return "iterative_writing"

    case ExpansionState.EXPANSION_VALIDATING:
    case ExpansionState.EXPANSION_VALIDATION_COMPLETE:
      return "consistency_validation"

    case ExpansionState.EXPANSION_COMPLETE:
    case ExpansionState.EXPANSION_FAILED:
    case ExpansionState.EXPANSION_PAUSED:
    case ExpansionState.EXPANSION_IDLE:
      return "terminal"
  }
}

/**
 * Expansion context for tracking progress.
 */
export interface ExpansionContext {
  documentID: string
  currentState: ExpansionState
  previousState: ExpansionState | undefined
  startedAt: number
  updatedAt: number

  // Input parameters
  coreIdea: string
  targetWords: number
  contentType: "fiction" | "nonfiction" | "auto"
  autonomy: "autonomous" | "stage-confirm" | "interactive"

  // Phase 1 output
  ideaAnalysis: KnowledgeSchema.CoreIdeaAnalysis | undefined

  // Phase 2 output
  thematicFramework: KnowledgeSchema.ThematicFramework | undefined
  worldFramework: KnowledgeSchema.WorldFramework | undefined

  // Phase 3 output
  outline: KnowledgeSchema.Outline | undefined

  // Phase 4 progress
  currentChapterIndex: number
  wordsWritten: number

  // Phase 5 output
  consistencyScore: number
  consistencyIssues: number

  // Error tracking
  error: string | undefined
  retryCount: number
}

/**
 * Create a new expansion context.
 */
export function createContext(input: {
  documentID: string
  coreIdea: string
  targetWords: number
  contentType: "fiction" | "nonfiction" | "auto"
  autonomy: "autonomous" | "stage-confirm" | "interactive"
}): ExpansionContext {
  return {
    documentID: input.documentID,
    currentState: ExpansionState.EXPANSION_IDLE,
    previousState: undefined,
    startedAt: Date.now(),
    updatedAt: Date.now(),

    coreIdea: input.coreIdea,
    targetWords: input.targetWords,
    contentType: input.contentType,
    autonomy: input.autonomy,

    ideaAnalysis: undefined,
    thematicFramework: undefined,
    worldFramework: undefined,
    outline: undefined,

    currentChapterIndex: 0,
    wordsWritten: 0,

    consistencyScore: 0,
    consistencyIssues: 0,

    error: undefined,
    retryCount: 0,
  }
}

/**
 * State transition metadata.
 */
export interface StateTransition {
  from: ExpansionState
  to: ExpansionState
  timestamp: number
  reason?: string
}
