import { describe, test, expect, beforeEach } from "bun:test"
import { AutonomousState, VALID_TRANSITIONS, isValidTransition, isTerminal, isRecoverable } from "@/autonomous/state/states"
import { StateMachine } from "@/autonomous/state/state-machine"
import {
  createTrackedStateMachine,
  StateTransitionTracker,
} from "./fixtures/autonomous-fixture"

describe("Autonomous Mode - State Machine", () => {
  describe("VALID_TRANSITIONS Coverage", () => {
    test("should have all 35 states defined", () => {
      const allStates = Object.values(AutonomousState)
      // 20 core states + 15 expansion states
      expect(allStates.length).toBe(35)
    })

    test("should define transitions for all states", () => {
      const allStates = Object.values(AutonomousState)

      for (const state of allStates) {
        expect(VALID_TRANSITIONS[state]).toBeDefined()
        expect(Array.isArray(VALID_TRANSITIONS[state])).toBe(true)
      }
    })

    test("should have valid transitions from IDLE", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.IDLE]
      expect(transitions).toContain(AutonomousState.PLANNING)
      expect(transitions).toContain(AutonomousState.TERMINATED)
      expect(transitions).toContain(AutonomousState.EXPANSION_ANALYZING)
      expect(transitions.length).toBe(3)
    })

    test("should have valid transitions from PLANNING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.PLANNING]
      expect(transitions).toContain(AutonomousState.PLAN_APPROVED)
      expect(transitions).toContain(AutonomousState.DECIDING)
      expect(transitions).toContain(AutonomousState.FAILED)
      expect(transitions).toContain(AutonomousState.PAUSED)
    })

    test("should have valid transitions from EXECUTING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.EXECUTING]
      expect(transitions).toContain(AutonomousState.TESTING)
      expect(transitions).toContain(AutonomousState.DECIDING)
      expect(transitions).toContain(AutonomousState.CHECKPOINTING)
      expect(transitions).toContain(AutonomousState.FIXING)
      expect(transitions).toContain(AutonomousState.FAILED)
      expect(transitions).toContain(AutonomousState.PAUSED)
    })

    test("should have valid transitions from TESTING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.TESTING]
      expect(transitions).toContain(AutonomousState.VERIFYING)
      expect(transitions).toContain(AutonomousState.FIXING)
      expect(transitions).toContain(AutonomousState.DECIDING)
      expect(transitions).toContain(AutonomousState.RETRYING)
      expect(transitions).toContain(AutonomousState.FAILED)
    })

    test("should have valid transitions from VERIFYING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.VERIFYING]
      expect(transitions).toContain(AutonomousState.EVALUATING)
      expect(transitions).toContain(AutonomousState.FIXING)
      expect(transitions).toContain(AutonomousState.DECIDING)
      expect(transitions).toContain(AutonomousState.RETRYING)
      expect(transitions).toContain(AutonomousState.FAILED)
    })

    test("should have valid transitions from DECIDING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.DECIDING]
      expect(transitions).toContain(AutonomousState.DECISION_MADE)
      expect(transitions).toContain(AutonomousState.PAUSED)
      expect(transitions).toContain(AutonomousState.BLOCKED)
    })

    test("should have valid transitions from DECISION_MADE", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.DECISION_MADE]
      expect(transitions).toContain(AutonomousState.EXECUTING)
      expect(transitions).toContain(AutonomousState.PLANNING)
      expect(transitions).toContain(AutonomousState.FAILED)
      expect(transitions).toContain(AutonomousState.PAUSED)
    })

    test("should have valid transitions from FIXING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.FIXING]
      expect(transitions).toContain(AutonomousState.TESTING)
      expect(transitions).toContain(AutonomousState.EXECUTING)
      expect(transitions).toContain(AutonomousState.DECIDING)
      expect(transitions).toContain(AutonomousState.FAILED)
    })

    test("should have valid transitions from RETRYING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.RETRYING]
      expect(transitions).toContain(AutonomousState.PLANNING)
      expect(transitions).toContain(AutonomousState.EXECUTING)
      expect(transitions).toContain(AutonomousState.FAILED)
    })

    test("should have valid transitions from EVALUATING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.EVALUATING]
      expect(transitions).toContain(AutonomousState.SCORING)
      expect(transitions).toContain(AutonomousState.DECIDING)
      expect(transitions).toContain(AutonomousState.FAILED)
    })

    test("should have valid transitions from SCORING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.SCORING]
      expect(transitions).toContain(AutonomousState.COMPLETED)
      expect(transitions).toContain(AutonomousState.CONTINUING)
      expect(transitions).toContain(AutonomousState.FAILED)
      expect(transitions).toContain(AutonomousState.PAUSED)
    })

    test("should have valid transitions from CONTINUING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.CONTINUING]
      expect(transitions).toContain(AutonomousState.PLANNING)
      expect(transitions).toContain(AutonomousState.EXECUTING)
      expect(transitions).toContain(AutonomousState.PAUSED)
      expect(transitions).toContain(AutonomousState.COMPLETED)
    })

    test("should have valid transitions from CHECKPOINTING", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.CHECKPOINTING]
      expect(transitions).toContain(AutonomousState.EXECUTING)
      expect(transitions).toContain(AutonomousState.TESTING)
      expect(transitions).toContain(AutonomousState.FAILED)
    })

    test("should have valid transitions from ROLLING_BACK", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.ROLLING_BACK]
      expect(transitions).toContain(AutonomousState.EXECUTING)
      expect(transitions).toContain(AutonomousState.PLANNING)
      expect(transitions).toContain(AutonomousState.FAILED)
      expect(transitions).toContain(AutonomousState.PAUSED)
    })

    test("should have valid transitions from terminal states", () => {
      const completedTransitions = VALID_TRANSITIONS[AutonomousState.COMPLETED]
      expect(completedTransitions).toContain(AutonomousState.IDLE)
      expect(completedTransitions).toContain(AutonomousState.TERMINATED)

      const failedTransitions = VALID_TRANSITIONS[AutonomousState.FAILED]
      expect(failedTransitions).toContain(AutonomousState.IDLE)
      expect(failedTransitions).toContain(AutonomousState.PLANNING)
      expect(failedTransitions).toContain(AutonomousState.TERMINATED)

      const pausedTransitions = VALID_TRANSITIONS[AutonomousState.PAUSED]
      expect(pausedTransitions).toContain(AutonomousState.EXECUTING)
      expect(pausedTransitions).toContain(AutonomousState.PLANNING)
      expect(pausedTransitions).toContain(AutonomousState.DECIDING)
      expect(pausedTransitions).toContain(AutonomousState.TERMINATED)

      const blockedTransitions = VALID_TRANSITIONS[AutonomousState.BLOCKED]
      expect(blockedTransitions).toContain(AutonomousState.DECIDING)
      expect(blockedTransitions).toContain(AutonomousState.PAUSED)
      expect(blockedTransitions).toContain(AutonomousState.FAILED)
      expect(blockedTransitions).toContain(AutonomousState.TERMINATED)
    })

    test("should have no transitions from TERMINATED", () => {
      const transitions = VALID_TRANSITIONS[AutonomousState.TERMINATED]
      expect(transitions).toEqual([])
    })
  })

  describe("isValidTransition Function", () => {
    test("should return true for valid transitions", () => {
      expect(isValidTransition(AutonomousState.IDLE, AutonomousState.PLANNING)).toBe(true)
      expect(isValidTransition(AutonomousState.PLANNING, AutonomousState.PLAN_APPROVED)).toBe(true)
      expect(isValidTransition(AutonomousState.EXECUTING, AutonomousState.TESTING)).toBe(true)
      expect(isValidTransition(AutonomousState.TESTING, AutonomousState.VERIFYING)).toBe(true)
    })

    test("should return false for invalid transitions", () => {
      expect(isValidTransition(AutonomousState.IDLE, AutonomousState.COMPLETED)).toBe(false)
      expect(isValidTransition(AutonomousState.TERMINATED, AutonomousState.PLANNING)).toBe(false)
      expect(isValidTransition(AutonomousState.TESTING, AutonomousState.IDLE)).toBe(false)
    })

    test("should return false for unknown states", () => {
      expect(isValidTransition("unknown" as AutonomousState, AutonomousState.PLANNING)).toBe(false)
      expect(isValidTransition(AutonomousState.IDLE, "unknown" as AutonomousState)).toBe(false)
    })
  })

  describe("Terminal States", () => {
    test("should identify COMPLETED as terminal", () => {
      expect(isTerminal(AutonomousState.COMPLETED)).toBe(true)
    })

    test("should identify FAILED as terminal", () => {
      expect(isTerminal(AutonomousState.FAILED)).toBe(true)
    })

    test("should identify PAUSED as terminal", () => {
      expect(isTerminal(AutonomousState.PAUSED)).toBe(true)
    })

    test("should identify BLOCKED as terminal", () => {
      expect(isTerminal(AutonomousState.BLOCKED)).toBe(true)
    })

    test("should not identify active states as terminal", () => {
      expect(isTerminal(AutonomousState.IDLE)).toBe(false)
      expect(isTerminal(AutonomousState.PLANNING)).toBe(false)
      expect(isTerminal(AutonomousState.EXECUTING)).toBe(false)
      expect(isTerminal(AutonomousState.TESTING)).toBe(false)
      expect(isTerminal(AutonomousState.VERIFYING)).toBe(false)
    })

    test("should not identify recovery states as terminal", () => {
      expect(isTerminal(AutonomousState.CHECKPOINTING)).toBe(false)
      expect(isTerminal(AutonomousState.ROLLING_BACK)).toBe(false)
    })
  })

  describe("Recoverable States", () => {
    test("should identify PAUSED as recoverable", () => {
      expect(isRecoverable(AutonomousState.PAUSED)).toBe(true)
    })

    test("should identify BLOCKED as recoverable", () => {
      expect(isRecoverable(AutonomousState.BLOCKED)).toBe(true)
    })

    test("should not identify other terminal states as recoverable", () => {
      expect(isRecoverable(AutonomousState.COMPLETED)).toBe(false)
      expect(isRecoverable(AutonomousState.FAILED)).toBe(false)
    })

    test("should not identify active states as recoverable", () => {
      expect(isRecoverable(AutonomousState.PLANNING)).toBe(false)
      expect(isRecoverable(AutonomousState.EXECUTING)).toBe(false)
    })
  })

  describe("StateMachine Class", () => {
    let tracker: StateTransitionTracker
    let stateMachine: StateMachine

    beforeEach(() => {
      tracker = new StateTransitionTracker()
      stateMachine = createTrackedStateMachine(tracker)
    })

    test("should initialize in IDLE state", () => {
      expect(stateMachine.getState()).toBe(AutonomousState.IDLE)
    })

    test("should successfully transition to valid next state", async () => {
      const success = await stateMachine.transition(AutonomousState.PLANNING, {
        reason: "Starting to plan",
      })

      expect(success).toBe(true)
      expect(stateMachine.getState()).toBe(AutonomousState.PLANNING)
      expect(tracker.hasTransition(AutonomousState.IDLE, AutonomousState.PLANNING)).toBe(true)
    })

    test("should fail to transition to invalid state", async () => {
      const success = await stateMachine.transition(AutonomousState.COMPLETED, {
        reason: "Invalid transition",
      })

      expect(success).toBe(false)
      expect(stateMachine.getState()).toBe(AutonomousState.IDLE)
      expect(tracker.hasTransition(AutonomousState.IDLE, AutonomousState.COMPLETED)).toBe(false)
    })

    test("should track state history", async () => {
      await stateMachine.transition(AutonomousState.PLANNING, { reason: "First" })
      await stateMachine.transition(AutonomousState.PLAN_APPROVED, { reason: "Second" })
      await stateMachine.transition(AutonomousState.EXECUTING, { reason: "Third" })

      const history = stateMachine.getHistory()
      expect(history.length).toBeGreaterThanOrEqual(3)

      // Check that the transitions are in order
      const states = history.map((h) => h.state)
      expect(states).toContain(AutonomousState.PLANNING)
      expect(states).toContain(AutonomousState.PLAN_APPROVED)
      expect(states).toContain(AutonomousState.EXECUTING)
    })

    test("should store metadata with transitions", async () => {
      await stateMachine.transition(AutonomousState.PLANNING, { reason: "Test reason" })

      const history = stateMachine.getHistory()
      const planningEntry = history.find((h) => h.state === AutonomousState.PLANNING)

      expect(planningEntry).toBeDefined()
      expect(planningEntry?.reason).toBe("Test reason")
      expect(planningEntry?.enteredAt).toBeDefined()
      expect(typeof planningEntry?.enteredAt).toBe("number")
    })

    test("should track previous state in metadata", async () => {
      await stateMachine.transition(AutonomousState.PLANNING, { reason: "First" })
      await stateMachine.transition(AutonomousState.PLAN_APPROVED, { reason: "Second" })

      const history = stateMachine.getHistory()
      const planApprovedEntry = history.find((h) => h.state === AutonomousState.PLAN_APPROVED)

      expect(planApprovedEntry?.previousState).toBe(AutonomousState.PLANNING)
    })

    test("should allow sequential valid transitions", async () => {
      const transitions = [
        [AutonomousState.IDLE, AutonomousState.PLANNING],
        [AutonomousState.PLANNING, AutonomousState.PLAN_APPROVED],
        [AutonomousState.PLAN_APPROVED, AutonomousState.EXECUTING],
        [AutonomousState.EXECUTING, AutonomousState.TESTING],
        [AutonomousState.TESTING, AutonomousState.VERIFYING],
        [AutonomousState.VERIFYING, AutonomousState.EVALUATING],
        [AutonomousState.EVALUATING, AutonomousState.SCORING],
        [AutonomousState.SCORING, AutonomousState.COMPLETED],
      ] as const

      for (const [from, to] of transitions) {
        const success = await stateMachine.transition(to, { reason: `Transition to ${to}` })
        expect(success).toBe(true)
        expect(stateMachine.getState()).toBe(to)
      }
    })

    test("should call onStateChange callback", async () => {
      let callbackCalled = false
      let capturedFrom: AutonomousState | undefined
      let capturedTo: AutonomousState | undefined

      const callbackStateMachine = new StateMachine({
        onStateChange: async (from, to) => {
          callbackCalled = true
          capturedFrom = from
          capturedTo = to
        },
      })

      await callbackStateMachine.transition(AutonomousState.PLANNING, { reason: "Test" })

      expect(callbackCalled).toBe(true)
      expect(capturedFrom).toBe(AutonomousState.IDLE)
      expect(capturedTo).toBe(AutonomousState.PLANNING)
    })

    test("should handle rapid state changes", async () => {
      const rapidTransitions = [
        AutonomousState.PLANNING,
        AutonomousState.DECIDING,
        AutonomousState.DECISION_MADE,
        AutonomousState.EXECUTING,
        AutonomousState.TESTING,
      ]

      for (const state of rapidTransitions) {
        await stateMachine.transition(state, { reason: "Rapid" })
      }

      expect(stateMachine.getState()).toBe(AutonomousState.TESTING)
      expect(tracker.getTransitions().length).toBe(rapidTransitions.length)
    })
  })

  describe("State Transition Tracker", () => {
    let tracker: StateTransitionTracker

    beforeEach(() => {
      tracker = new StateTransitionTracker()
    })

    test("should record state transitions", () => {
      tracker.record(AutonomousState.IDLE, AutonomousState.PLANNING)

      const transitions = tracker.getTransitions()
      expect(transitions.length).toBe(1)
      expect(transitions[0].from).toBe(AutonomousState.IDLE)
      expect(transitions[0].to).toBe(AutonomousState.PLANNING)
    })

    test("should record multiple transitions", () => {
      tracker.record(AutonomousState.IDLE, AutonomousState.PLANNING)
      tracker.record(AutonomousState.PLANNING, AutonomousState.PLAN_APPROVED)
      tracker.record(AutonomousState.PLAN_APPROVED, AutonomousState.EXECUTING)

      expect(tracker.getTransitions().length).toBe(3)
    })

    test("should track timestamps", () => {
      const before = Date.now()
      tracker.record(AutonomousState.IDLE, AutonomousState.PLANNING)
      const after = Date.now()

      const transitions = tracker.getTransitions()
      expect(transitions[0].timestamp).toBeGreaterThanOrEqual(before)
      expect(transitions[0].timestamp).toBeLessThanOrEqual(after)
    })

    test("should check if transition occurred", () => {
      tracker.record(AutonomousState.IDLE, AutonomousState.PLANNING)
      tracker.record(AutonomousState.PLANNING, AutonomousState.EXECUTING)

      expect(tracker.hasTransition(AutonomousState.IDLE, AutonomousState.PLANNING)).toBe(true)
      expect(tracker.hasTransition(AutonomousState.PLANNING, AutonomousState.EXECUTING)).toBe(true)
      expect(tracker.hasTransition(AutonomousState.IDLE, AutonomousState.EXECUTING)).toBe(false)
    })

    test("should count transitions to a state", () => {
      tracker.record(AutonomousState.IDLE, AutonomousState.PLANNING)
      tracker.record(AutonomousState.PLANNING, AutonomousState.DECIDING)
      tracker.record(AutonomousState.DECIDING, AutonomousState.PLANNING)
      tracker.record(AutonomousState.PLANNING, AutonomousState.EXECUTING)

      expect(tracker.countTransitions(AutonomousState.PLANNING)).toBe(2)
      expect(tracker.countTransitions(AutonomousState.DECIDING)).toBe(1)
      expect(tracker.countTransitions(AutonomousState.EXECUTING)).toBe(1)
    })

    test("should clear all transitions", () => {
      tracker.record(AutonomousState.IDLE, AutonomousState.PLANNING)
      tracker.record(AutonomousState.PLANNING, AutonomousState.EXECUTING)

      expect(tracker.getTransitions().length).toBe(2)

      tracker.clear()

      expect(tracker.getTransitions().length).toBe(0)
    })
  })

  describe("Complex State Scenarios", () => {
    test("should handle complete workflow", async () => {
      const tracker = new StateTransitionTracker()
      const stateMachine = createTrackedStateMachine(tracker)

      // Complete workflow: IDLE -> PLANNING -> PLAN_APPROVED -> EXECUTING -> TESTING -> VERIFYING -> EVALUATING -> SCORING -> COMPLETED
      const workflow = [
        AutonomousState.PLANNING,
        AutonomousState.PLAN_APPROVED,
        AutonomousState.EXECUTING,
        AutonomousState.TESTING,
        AutonomousState.VERIFYING,
        AutonomousState.EVALUATING,
        AutonomousState.SCORING,
        AutonomousState.COMPLETED,
      ]

      for (const state of workflow) {
        const success = await stateMachine.transition(state, { reason: `Transition to ${state}` })
        expect(success).toBe(true)
      }

      expect(stateMachine.getState()).toBe(AutonomousState.COMPLETED)
      expect(tracker.getTransitions().length).toBe(workflow.length)
    })

    test("should handle fix workflow", async () => {
      const tracker = new StateTransitionTracker()
      const stateMachine = createTrackedStateMachine(tracker)

      // Fix workflow: IDLE -> PLANNING -> PLAN_APPROVED -> EXECUTING -> TESTING -> FIXING -> TESTING -> VERIFYING
      const workflow = [
        AutonomousState.PLANNING,
        AutonomousState.PLAN_APPROVED,
        AutonomousState.EXECUTING,
        AutonomousState.TESTING,
        AutonomousState.FIXING,
        AutonomousState.TESTING,
        AutonomousState.VERIFYING,
      ]

      for (const state of workflow) {
        const success = await stateMachine.transition(state, { reason: `Transition to ${state}` })
        expect(success).toBe(true)
      }

      expect(stateMachine.getState()).toBe(AutonomousState.VERIFYING)
    })

    test("should handle continue workflow", async () => {
      const tracker = new StateTransitionTracker()
      const stateMachine = createTrackedStateMachine(tracker)

      // Continue workflow: IDLE -> PLANNING -> PLAN_APPROVED -> EXECUTING -> TESTING -> VERIFYING -> EVALUATING -> SCORING -> CONTINUING -> PLANNING
      const workflow = [
        AutonomousState.PLANNING,
        AutonomousState.PLAN_APPROVED,
        AutonomousState.EXECUTING,
        AutonomousState.TESTING,
        AutonomousState.VERIFYING,
        AutonomousState.EVALUATING,
        AutonomousState.SCORING,
        AutonomousState.CONTINUING,
        AutonomousState.PLANNING,
      ]

      for (const state of workflow) {
        const success = await stateMachine.transition(state, { reason: `Transition to ${state}` })
        expect(success).toBe(true)
      }

      expect(stateMachine.getState()).toBe(AutonomousState.PLANNING)
      expect(tracker.countTransitions(AutonomousState.PLANNING)).toBe(2)
    })

    test("should handle pause and resume", async () => {
      const tracker = new StateTransitionTracker()
      const stateMachine = createTrackedStateMachine(tracker)

      // Pause and resume: IDLE -> PLANNING -> PAUSED -> PLANNING
      await stateMachine.transition(AutonomousState.PLANNING, { reason: "Start" })
      await stateMachine.transition(AutonomousState.PAUSED, { reason: "Paused" })
      await stateMachine.transition(AutonomousState.PLANNING, { reason: "Resume" })

      expect(stateMachine.getState()).toBe(AutonomousState.PLANNING)
      expect(tracker.hasTransition(AutonomousState.PLANNING, AutonomousState.PAUSED)).toBe(true)
      expect(tracker.hasTransition(AutonomousState.PAUSED, AutonomousState.PLANNING)).toBe(true)
    })

    test("should handle blocked decision", async () => {
      const tracker = new StateTransitionTracker()
      const stateMachine = createTrackedStateMachine(tracker)

      // Blocked decision: IDLE -> PLANNING -> DECIDING -> BLOCKED -> DECIDING
      await stateMachine.transition(AutonomousState.PLANNING, { reason: "Start" })
      await stateMachine.transition(AutonomousState.DECIDING, { reason: "Need decision" })
      await stateMachine.transition(AutonomousState.BLOCKED, { reason: "Blocked" })
      await stateMachine.transition(AutonomousState.DECIDING, { reason: "Re-evaluating" })

      expect(stateMachine.getState()).toBe(AutonomousState.DECIDING)
      expect(tracker.hasTransition(AutonomousState.DECIDING, AutonomousState.BLOCKED)).toBe(true)
    })
  })
})
