import z from "zod"

export enum AutonomousState {
  IDLE = "idle",
  PLANNING = "planning",
  PLAN_APPROVED = "plan_approved",
  EXECUTING = "executing",
  TESTING = "testing",
  VERIFYING = "verifying",
  DECIDING = "deciding",
  DECISION_MADE = "decision_made",
  FIXING = "fixing",
  RETRYING = "retrying",
  EVALUATING = "evaluating",
  SCORING = "scoring",
  CHECKPOINTING = "checkpointing",
  ROLLING_BACK = "rolling_back",
  CONTINUING = "continuing",
  COMPLETED = "completed",
  FAILED = "failed",
  PAUSED = "paused",
  BLOCKED = "blocked",
  TERMINATED = "terminated",
  // ============================================================================
  // Expansion States (BookExpander)
  // ============================================================================
  EXPANSION_IDLE = "expansion_idle",
  EXPANSION_ANALYZING = "expansion_analyzing",
  EXPANSION_ANALYSIS_COMPLETE = "expansion_analysis_complete",
  EXPANSION_BUILDING = "expansion_building",
  EXPANSION_FRAMEWORK_COMPLETE = "expansion_framework_complete",
  EXPANSION_OUTLINING = "expansion_outlining",
  EXPANSION_OUTLINE_COMPLETE = "expansion_outline_complete",
  EXPANSION_WRITING = "expansion_writing",
  EXPANSION_CHAPTER_COMPLETE = "expansion_chapter_complete",
  EXPANSION_WRITING_COMPLETE = "expansion_writing_complete",
  EXPANSION_VALIDATING = "expansion_validating",
  EXPANSION_VALIDATION_COMPLETE = "expansion_validation_complete",
  EXPANSION_COMPLETE = "expansion_complete",
  EXPANSION_FAILED = "expansion_failed",
  EXPANSION_PAUSED = "expansion_paused",
}

export const StateMetadata = z.object({
  state: z.nativeEnum(AutonomousState),
  enteredAt: z.number(),
  previousState: z.nativeEnum(AutonomousState).optional(),
  reason: z.string().optional(),
})
export type StateMetadata = z.infer<typeof StateMetadata>

export const VALID_TRANSITIONS: Record<AutonomousState, AutonomousState[]> = {
  [AutonomousState.IDLE]: [AutonomousState.PLANNING, AutonomousState.TERMINATED, AutonomousState.EXPANSION_ANALYZING],

  [AutonomousState.PLANNING]: [
    AutonomousState.PLAN_APPROVED,
    AutonomousState.DECIDING,
    AutonomousState.FAILED,
    AutonomousState.PAUSED,
  ],
  [AutonomousState.PLAN_APPROVED]: [AutonomousState.EXECUTING, AutonomousState.DECIDING, AutonomousState.PAUSED],

  [AutonomousState.EXECUTING]: [
    AutonomousState.TESTING,
    AutonomousState.DECIDING,
    AutonomousState.CHECKPOINTING,
    AutonomousState.FIXING,
    AutonomousState.FAILED,
    AutonomousState.PAUSED,
  ],
  [AutonomousState.TESTING]: [
    AutonomousState.VERIFYING,
    AutonomousState.FIXING,
    AutonomousState.DECIDING,
    AutonomousState.RETRYING,
    AutonomousState.FAILED,
  ],
  [AutonomousState.VERIFYING]: [
    AutonomousState.EVALUATING,
    AutonomousState.FIXING,
    AutonomousState.DECIDING,
    AutonomousState.RETRYING,
    AutonomousState.FAILED,
  ],

  [AutonomousState.DECIDING]: [AutonomousState.DECISION_MADE, AutonomousState.PAUSED, AutonomousState.BLOCKED],
  [AutonomousState.DECISION_MADE]: [
    AutonomousState.EXECUTING,
    AutonomousState.PLANNING,
    AutonomousState.FAILED,
    AutonomousState.PAUSED,
  ],

  [AutonomousState.FIXING]: [
    AutonomousState.TESTING,
    AutonomousState.EXECUTING,
    AutonomousState.DECIDING,
    AutonomousState.FAILED,
  ],
  [AutonomousState.RETRYING]: [AutonomousState.PLANNING, AutonomousState.EXECUTING, AutonomousState.FAILED],

  [AutonomousState.EVALUATING]: [AutonomousState.SCORING, AutonomousState.DECIDING, AutonomousState.FAILED],
  [AutonomousState.SCORING]: [AutonomousState.COMPLETED, AutonomousState.CONTINUING, AutonomousState.FAILED, AutonomousState.PAUSED],

  [AutonomousState.CHECKPOINTING]: [AutonomousState.EXECUTING, AutonomousState.TESTING, AutonomousState.FAILED],
  [AutonomousState.ROLLING_BACK]: [
    AutonomousState.EXECUTING,
    AutonomousState.PLANNING,
    AutonomousState.FAILED,
    AutonomousState.PAUSED,
  ],

  [AutonomousState.CONTINUING]: [
    AutonomousState.PLANNING,
    AutonomousState.EXECUTING,
    AutonomousState.PAUSED,
    AutonomousState.COMPLETED,
  ],

  [AutonomousState.COMPLETED]: [AutonomousState.IDLE, AutonomousState.TERMINATED],
  [AutonomousState.FAILED]: [AutonomousState.IDLE, AutonomousState.PLANNING, AutonomousState.TERMINATED],
  [AutonomousState.PAUSED]: [
    AutonomousState.EXECUTING,
    AutonomousState.PLANNING,
    AutonomousState.DECIDING,
    AutonomousState.TERMINATED,
  ],
  [AutonomousState.BLOCKED]: [
    AutonomousState.DECIDING,
    AutonomousState.PAUSED,
    AutonomousState.FAILED,
    AutonomousState.TERMINATED,
  ],
  [AutonomousState.TERMINATED]: [],

  // ============================================================================
  // Expansion State Transitions (BookExpander)
  // ============================================================================
  [AutonomousState.EXPANSION_IDLE]: [
    AutonomousState.EXPANSION_ANALYZING,
    AutonomousState.EXPANSION_FAILED,
  ],
  [AutonomousState.EXPANSION_ANALYZING]: [
    AutonomousState.EXPANSION_ANALYSIS_COMPLETE,
    AutonomousState.EXPANSION_FAILED,
  ],
  [AutonomousState.EXPANSION_ANALYSIS_COMPLETE]: [
    AutonomousState.EXPANSION_BUILDING,
    AutonomousState.EXPANSION_PAUSED,
    AutonomousState.EXPANSION_FAILED,
  ],
  [AutonomousState.EXPANSION_BUILDING]: [
    AutonomousState.EXPANSION_FRAMEWORK_COMPLETE,
    AutonomousState.EXPANSION_PAUSED,
    AutonomousState.EXPANSION_FAILED,
  ],
  [AutonomousState.EXPANSION_FRAMEWORK_COMPLETE]: [
    AutonomousState.EXPANSION_OUTLINING,
    AutonomousState.EXPANSION_PAUSED,
  ],
  [AutonomousState.EXPANSION_OUTLINING]: [
    AutonomousState.EXPANSION_OUTLINE_COMPLETE,
    AutonomousState.EXPANSION_FAILED,
  ],
  [AutonomousState.EXPANSION_OUTLINE_COMPLETE]: [
    AutonomousState.EXPANSION_WRITING,
    AutonomousState.EXPANSION_PAUSED,
  ],
  [AutonomousState.EXPANSION_WRITING]: [
    AutonomousState.EXPANSION_CHAPTER_COMPLETE,
    AutonomousState.EXPANSION_WRITING_COMPLETE,
    AutonomousState.EXPANSION_PAUSED,
    AutonomousState.EXPANSION_FAILED,
  ],
  [AutonomousState.EXPANSION_CHAPTER_COMPLETE]: [
    AutonomousState.EXPANSION_WRITING,
    AutonomousState.EXPANSION_VALIDATING,
    AutonomousState.EXPANSION_PAUSED,
  ],
  [AutonomousState.EXPANSION_WRITING_COMPLETE]: [
    AutonomousState.EXPANSION_VALIDATING,
    AutonomousState.EXPANSION_COMPLETE,
  ],
  [AutonomousState.EXPANSION_VALIDATING]: [
    AutonomousState.EXPANSION_VALIDATION_COMPLETE,
    AutonomousState.EXPANSION_WRITING,
    AutonomousState.EXPANSION_FAILED,
  ],
  [AutonomousState.EXPANSION_VALIDATION_COMPLETE]: [
    AutonomousState.EXPANSION_COMPLETE,
    AutonomousState.EXPANSION_WRITING,
  ],
  [AutonomousState.EXPANSION_COMPLETE]: [AutonomousState.EXPANSION_IDLE, AutonomousState.TERMINATED],
  [AutonomousState.EXPANSION_FAILED]: [AutonomousState.EXPANSION_IDLE, AutonomousState.TERMINATED],
  [AutonomousState.EXPANSION_PAUSED]: [
    AutonomousState.EXPANSION_ANALYZING,
    AutonomousState.EXPANSION_BUILDING,
    AutonomousState.EXPANSION_OUTLINING,
    AutonomousState.EXPANSION_WRITING,
    AutonomousState.EXPANSION_VALIDATING,
    AutonomousState.EXPANSION_COMPLETE,
    AutonomousState.EXPANSION_FAILED,
    AutonomousState.TERMINATED,
  ],
}

export function isValidTransition(from: AutonomousState, to: AutonomousState): boolean {
  return VALID_TRANSITIONS[from]?.includes(to) ?? false
}

export function getStateCategory(state: AutonomousState): "initial" | "active" | "terminal" | "recovery" {
  if (state === AutonomousState.IDLE || state === AutonomousState.EXPANSION_IDLE) return "initial"
  if (
    state === AutonomousState.PLANNING ||
    state === AutonomousState.PLAN_APPROVED ||
    state === AutonomousState.EXECUTING ||
    state === AutonomousState.TESTING ||
    state === AutonomousState.VERIFYING ||
    state === AutonomousState.DECIDING ||
    state === AutonomousState.DECISION_MADE ||
    state === AutonomousState.FIXING ||
    state === AutonomousState.RETRYING ||
    state === AutonomousState.EVALUATING ||
    state === AutonomousState.SCORING ||
    state === AutonomousState.CONTINUING ||
    state === AutonomousState.EXPANSION_ANALYZING ||
    state === AutonomousState.EXPANSION_ANALYSIS_COMPLETE ||
    state === AutonomousState.EXPANSION_BUILDING ||
    state === AutonomousState.EXPANSION_FRAMEWORK_COMPLETE ||
    state === AutonomousState.EXPANSION_OUTLINING ||
    state === AutonomousState.EXPANSION_OUTLINE_COMPLETE ||
    state === AutonomousState.EXPANSION_WRITING ||
    state === AutonomousState.EXPANSION_CHAPTER_COMPLETE ||
    state === AutonomousState.EXPANSION_WRITING_COMPLETE ||
    state === AutonomousState.EXPANSION_VALIDATING ||
    state === AutonomousState.EXPANSION_VALIDATION_COMPLETE
  ) {
    return "active"
  }
  if (
    state === AutonomousState.COMPLETED ||
    state === AutonomousState.FAILED ||
    state === AutonomousState.PAUSED ||
    state === AutonomousState.BLOCKED ||
    state === AutonomousState.TERMINATED ||
    state === AutonomousState.EXPANSION_COMPLETE ||
    state === AutonomousState.EXPANSION_FAILED ||
    state === AutonomousState.EXPANSION_PAUSED
  ) {
    return "terminal"
  }
  return "recovery"
}

export const TERMINAL_STATES = [
  AutonomousState.COMPLETED,
  AutonomousState.FAILED,
  AutonomousState.PAUSED,
  AutonomousState.BLOCKED,
  AutonomousState.EXPANSION_COMPLETE,
  AutonomousState.EXPANSION_FAILED,
  AutonomousState.EXPANSION_PAUSED,
]

export function isTerminal(state: AutonomousState): boolean {
  return TERMINAL_STATES.includes(state)
}

export function isRecoverable(state: AutonomousState): boolean {
  return state === AutonomousState.PAUSED || state === AutonomousState.BLOCKED
}
