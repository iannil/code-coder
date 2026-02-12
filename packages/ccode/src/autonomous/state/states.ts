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
  COMPLETED = "completed",
  FAILED = "failed",
  PAUSED = "paused",
  BLOCKED = "blocked",
  TERMINATED = "terminated",
}

export const StateMetadata = z.object({
  state: z.nativeEnum(AutonomousState),
  enteredAt: z.number(),
  previousState: z.nativeEnum(AutonomousState).optional(),
  reason: z.string().optional(),
})
export type StateMetadata = z.infer<typeof StateMetadata>

export const VALID_TRANSITIONS: Record<AutonomousState, AutonomousState[]> = {
  [AutonomousState.IDLE]: [AutonomousState.PLANNING, AutonomousState.TERMINATED],

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
  [AutonomousState.SCORING]: [AutonomousState.COMPLETED, AutonomousState.FAILED, AutonomousState.PAUSED],

  [AutonomousState.CHECKPOINTING]: [AutonomousState.EXECUTING, AutonomousState.TESTING, AutonomousState.FAILED],
  [AutonomousState.ROLLING_BACK]: [
    AutonomousState.EXECUTING,
    AutonomousState.PLANNING,
    AutonomousState.FAILED,
    AutonomousState.PAUSED,
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
}

export function isValidTransition(from: AutonomousState, to: AutonomousState): boolean {
  return VALID_TRANSITIONS[from]?.includes(to) ?? false
}

export function getStateCategory(state: AutonomousState): "initial" | "active" | "terminal" | "recovery" {
  if (state === AutonomousState.IDLE) return "initial"
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
    state === AutonomousState.SCORING
  ) {
    return "active"
  }
  if (
    state === AutonomousState.COMPLETED ||
    state === AutonomousState.FAILED ||
    state === AutonomousState.PAUSED ||
    state === AutonomousState.BLOCKED ||
    state === AutonomousState.TERMINATED
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
]

export function isTerminal(state: AutonomousState): boolean {
  return TERMINAL_STATES.includes(state)
}

export function isRecoverable(state: AutonomousState): boolean {
  return state === AutonomousState.PAUSED || state === AutonomousState.BLOCKED
}
