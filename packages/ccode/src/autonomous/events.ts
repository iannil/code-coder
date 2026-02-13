import { BusEvent } from "@/bus/bus-event"
import { AutonomousState } from "./state/states"
import z from "zod"

export namespace AutonomousEvent {
  export const StateChanged = BusEvent.define(
    "autonomous.state.changed",
    z.object({
      from: z.nativeEnum(AutonomousState),
      to: z.nativeEnum(AutonomousState),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const InvalidTransition = BusEvent.define(
    "autonomous.state.invalid_transition",
    z.object({
      from: z.nativeEnum(AutonomousState),
      to: z.nativeEnum(AutonomousState),
      reason: z.string(),
    }),
  )

  export const SessionStarted = BusEvent.define(
    "autonomous.session.started",
    z.object({
      sessionId: z.string(),
      requestId: z.string(),
      autonomyLevel: z.enum(["lunatic", "insane", "crazy", "wild", "bold", "timid"]),
      config: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const SessionCompleted = BusEvent.define(
    "autonomous.session.completed",
    z.object({
      sessionId: z.string(),
      requestId: z.string(),
      result: z.object({
        success: z.boolean(),
        qualityScore: z.number(),
        crazinessScore: z.number(),
        duration: z.number(),
        tokensUsed: z.number(),
        costUSD: z.number(),
      }),
    }),
  )

  export const SessionFailed = BusEvent.define(
    "autonomous.session.failed",
    z.object({
      sessionId: z.string(),
      requestId: z.string(),
      error: z.string(),
      state: z.nativeEnum(AutonomousState),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const SessionPaused = BusEvent.define(
    "autonomous.session.paused",
    z.object({
      sessionId: z.string(),
      reason: z.string(),
      state: z.nativeEnum(AutonomousState),
      canResume: z.boolean(),
    }),
  )

  export const DecisionMade = BusEvent.define(
    "autonomous.decision.made",
    z.object({
      sessionId: z.string(),
      decisionId: z.string(),
      type: z.string(),
      description: z.string(),
      score: z.number(),
      approved: z.boolean(),
      closeScores: z.object({
        convergence: z.number(),
        leverage: z.number(),
        optionality: z.number(),
        surplus: z.number(),
        evolution: z.number(),
        total: z.number(),
      }),
    }),
  )

  export const DecisionBlocked = BusEvent.define(
    "autonomous.decision.blocked",
    z.object({
      sessionId: z.string(),
      decisionId: z.string(),
      reason: z.string(),
      context: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const TaskCreated = BusEvent.define(
    "autonomous.task.created",
    z.object({
      sessionId: z.string(),
      taskId: z.string(),
      subject: z.string(),
      description: z.string(),
      priority: z.number(),
      dependencies: z.array(z.string()),
    }),
  )

  export const TaskStarted = BusEvent.define(
    "autonomous.task.started",
    z.object({
      sessionId: z.string(),
      taskId: z.string(),
    }),
  )

  export const TaskCompleted = BusEvent.define(
    "autonomous.task.completed",
    z.object({
      sessionId: z.string(),
      taskId: z.string(),
      success: z.boolean(),
      duration: z.number(),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const TaskFailed = BusEvent.define(
    "autonomous.task.failed",
    z.object({
      sessionId: z.string(),
      taskId: z.string(),
      error: z.string(),
      retryable: z.boolean(),
      retryCount: z.number(),
    }),
  )

  export const PhaseStarted = BusEvent.define(
    "autonomous.phase.started",
    z.object({
      sessionId: z.string(),
      phase: z.string(),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const PhaseCompleted = BusEvent.define(
    "autonomous.phase.completed",
    z.object({
      sessionId: z.string(),
      phase: z.string(),
      duration: z.number(),
      success: z.boolean(),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const TDDCycleStarted = BusEvent.define(
    "autonomous.tdd.cycle_started",
    z.object({
      sessionId: z.string(),
      cycleId: z.string(),
      phase: z.enum(["red", "green", "refactor"]),
    }),
  )

  export const TDDCycleCompleted = BusEvent.define(
    "autonomous.tdd.cycle_completed",
    z.object({
      sessionId: z.string(),
      cycleId: z.string(),
      phase: z.enum(["red", "green", "refactor"]),
      success: z.boolean(),
      duration: z.number(),
    }),
  )

  export const CheckpointCreated = BusEvent.define(
    "autonomous.checkpoint.created",
    z.object({
      sessionId: z.string(),
      checkpointId: z.string(),
      type: z.enum(["git", "state", "manual"]),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const RollbackPerformed = BusEvent.define(
    "autonomous.rollback.performed",
    z.object({
      sessionId: z.string(),
      checkpointId: z.string(),
      reason: z.string(),
      success: z.boolean(),
    }),
  )

  export const ResourceWarning = BusEvent.define(
    "autonomous.resource.warning",
    z.object({
      sessionId: z.string(),
      resource: z.enum(["tokens", "cost", "time", "files", "actions", "destructive_operation"]),
      current: z.number(),
      limit: z.number(),
      percentage: z.number(),
    }),
  )

  export const ResourceExceeded = BusEvent.define(
    "autonomous.resource.exceeded",
    z.object({
      sessionId: z.string(),
      resource: z.enum(["tokens", "cost", "time", "files", "actions"]),
      current: z.number(),
      limit: z.number(),
    }),
  )

  export const LoopDetected = BusEvent.define(
    "autonomous.loop.detected",
    z.object({
      sessionId: z.string(),
      loopType: z.enum(["state", "tool", "decision"]),
      pattern: z.array(z.any()),
      count: z.number(),
      broken: z.boolean(),
    }),
  )

  export const MetricsUpdated = BusEvent.define(
    "autonomous.metrics.updated",
    z.object({
      sessionId: z.string(),
      metrics: z.object({
        qualityScore: z.number(),
        crazinessScore: z.number(),
        autonomyLevel: z.string(),
        tasksCompleted: z.number(),
        tasksTotal: z.number(),
      }),
    }),
  )

  export const ReportGenerated = BusEvent.define(
    "autonomous.report.generated",
    z.object({
      sessionId: z.string(),
      reportType: z.enum(["summary", "detailed", "metrics", "decisions"]),
      filePath: z.string().optional(),
    }),
  )

  export const SafetyTriggered = BusEvent.define(
    "autonomous.safety.triggered",
    z.object({
      sessionId: z.string(),
      rule: z.string(),
      severity: z.enum(["low", "medium", "high", "critical"]),
      action: z.string(),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const AgentInvoked = BusEvent.define(
    "autonomous.agent.invoked",
    z.object({
      sessionId: z.string(),
      agentName: z.string(),
      task: z.string(),
      success: z.boolean(),
      duration: z.number(),
      error: z.string().optional(),
    }),
  )

  export const IterationStarted = BusEvent.define(
    "autonomous.iteration.started",
    z.object({
      sessionId: z.string(),
      iteration: z.number(),
      remainingRequirements: z.number(),
      context: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const IterationCompleted = BusEvent.define(
    "autonomous.iteration.completed",
    z.object({
      sessionId: z.string(),
      iteration: z.number(),
      completedRequirements: z.number(),
      success: z.boolean(),
      duration: z.number(),
    }),
  )

  export const NextStepPlanned = BusEvent.define(
    "autonomous.next_step.planned",
    z.object({
      sessionId: z.string(),
      iteration: z.number(),
      nextTasks: z.array(
        z.object({
          subject: z.string(),
          priority: z.string(),
        }),
      ),
      reason: z.string(),
      estimatedCycles: z.number(),
      confidence: z.number(),
    }),
  )

  export const RequirementsUpdated = BusEvent.define(
    "autonomous.requirements.updated",
    z.object({
      sessionId: z.string(),
      stats: z.object({
        total: z.number(),
        completed: z.number(),
        inProgress: z.number(),
        pending: z.number(),
        blocked: z.number(),
        completionPercentage: z.number(),
      }),
    }),
  )

  export const CompletionChecked = BusEvent.define(
    "autonomous.completion.checked",
    z.object({
      sessionId: z.string(),
      iteration: z.number(),
      criteria: z.object({
        requirementsCompleted: z.boolean(),
        testsPassing: z.boolean(),
        verificationPassed: z.boolean(),
        noBlockingIssues: z.boolean(),
        resourceExhausted: z.boolean(),
      }),
      allComplete: z.boolean(),
      canContinue: z.boolean(),
      shouldPause: z.boolean(),
    }),
  )

  // ============================================================================
  // Expansion Events (BookExpander)
  // ============================================================================

  export const ExpansionStarted = BusEvent.define(
    "autonomous.expansion.started",
    z.object({
      documentID: z.string(),
      coreIdea: z.string(),
      targetWords: z.number(),
      contentType: z.enum(["fiction", "nonfiction", "auto"]),
      autonomy: z.enum(["autonomous", "stage-confirm", "interactive"]),
    }),
  )

  export const ExpansionPhaseChanged = BusEvent.define(
    "autonomous.expansion.phase_changed",
    z.object({
      documentID: z.string(),
      from: z.string(),
      to: z.string(),
      phase: z.enum([
        "idea_analysis",
        "framework_building",
        "outline_generation",
        "iterative_writing",
        "consistency_validation",
      ]),
    }),
  )

  export const ExpansionChapterCompleted = BusEvent.define(
    "autonomous.expansion.chapter_completed",
    z.object({
      documentID: z.string(),
      chapterID: z.string(),
      chapterIndex: z.number(),
      wordCount: z.number(),
      duration: z.number(),
    }),
  )

  export const ExpansionCompleted = BusEvent.define(
    "autonomous.expansion.completed",
    z.object({
      documentID: z.string(),
      targetWords: z.number(),
      actualWords: z.number(),
      consistencyScore: z.number(),
      duration: z.number(),
    }),
  )

  export const ExpansionFailed = BusEvent.define(
    "autonomous.expansion.failed",
    z.object({
      documentID: z.string(),
      phase: z.string(),
      error: z.string(),
      retryable: z.boolean(),
    }),
  )

  export const ExpansionPaused = BusEvent.define(
    "autonomous.expansion.paused",
    z.object({
      documentID: z.string(),
      phase: z.string(),
      reason: z.string(),
    }),
  )

  export const FrameworkValidated = BusEvent.define(
    "autonomous.expansion.framework_validated",
    z.object({
      documentID: z.string(),
      isValid: z.boolean(),
      issues: z.array(z.object({
        type: z.string(),
        description: z.string(),
        severity: z.enum(["low", "medium", "high"]),
      })),
    }),
  )
}

const BusPromise = import("@/bus").then((m) => m.Bus)

export namespace AutonomousEventHelper {
  export async function subscribeAll(callback: (event: { type: string; properties: unknown }) => void) {
    const Bus = await BusPromise
    return Bus.subscribeAll((event: { type: string; properties: unknown }) => {
      if (event.type.startsWith("autonomous.")) {
        callback(event)
      }
    })
  }

  export async function waitFor<T extends BusEvent.Definition>(
    eventType: T,
    timeout = 30000,
  ): Promise<{ type: string; properties: unknown } | undefined> {
    const Bus = await BusPromise
    let result: { type: string; properties: unknown } | undefined

    const unsubscribe = Bus.subscribe(eventType, (event: { type: string; properties: unknown }) => {
      result = event
    })

    const timeoutPromise = new Promise<undefined>((resolve) => {
      setTimeout(() => resolve(undefined), timeout).unref()
    })

    await Promise.race([
      (async () => {
        while (!result) {
          await new Promise((resolve) => setTimeout(resolve, 100).unref())
        }
        unsubscribe()
      })(),
      timeoutPromise,
    ])

    return result
  }
}
