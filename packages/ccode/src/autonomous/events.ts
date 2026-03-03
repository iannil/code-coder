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

  export const WebSearchCompleted = BusEvent.define(
    "autonomous.web_search.completed",
    z.object({
      sessionId: z.string(),
      queriesRun: z.number(),
      resultsFound: z.number(),
      contentFetched: z.number(),
      solutionConfidence: z.number(),
      duration: z.number(),
    }),
  )

  export const KnowledgeConsolidated = BusEvent.define(
    "autonomous.knowledge.consolidated",
    z.object({
      sessionId: z.string(),
      solutionId: z.string(),
      category: z.string(),
      tags: z.array(z.string()),
      confidence: z.number(),
    }),
  )

  export const EvolutionCompleted = BusEvent.define(
    "autonomous.evolution.completed",
    z.object({
      sessionId: z.string(),
      solved: z.boolean(),
      attempts: z.number(),
      summary: z.string(),
      knowledgeId: z.string().optional(),
      learnedToolId: z.string().optional(),
      durationMs: z.number().optional(),
    }),
  )

  // ============================================================================
  // GitHub Scout Events
  // ============================================================================

  export const GithubScoutTriggered = BusEvent.define(
    "autonomous.github_scout.triggered",
    z.object({
      sessionId: z.string(),
      confidence: z.number(),
      category: z.enum(["high", "medium", "low"]),
      queries: z.array(z.string()),
    }),
  )

  export const RepoEvaluated = BusEvent.define(
    "autonomous.github_scout.repo_evaluated",
    z.object({
      sessionId: z.string(),
      reposEvaluated: z.number(),
      topRepo: z.string().optional(),
      topScore: z.number().optional(),
      recommendation: z.enum(["adopt", "trial", "assess", "avoid"]).optional(),
    }),
  )

  export const IntegrationExecuted = BusEvent.define(
    "autonomous.github_scout.integration_executed",
    z.object({
      sessionId: z.string(),
      repo: z.string(),
      mode: z.enum(["autonomous", "recommend", "ask"]),
      action: z.enum(["installed", "recommended", "user_declined", "skipped", "failed"]),
      success: z.boolean(),
    }),
  )

  // ============================================================================
  // Project Creation Events
  // ============================================================================

  export const ProjectCreated = BusEvent.define(
    "autonomous.project.created",
    z.object({
      projectId: z.string().optional(),
      name: z.string(),
      slug: z.string(),
      technology: z.array(z.string()),
      action: z.enum(["clone_template", "create_from_scratch", "ask_user"]),
      template: z.string().optional(),
      success: z.boolean(),
    }),
  )

  export const ProjectSwitched = BusEvent.define(
    "autonomous.project.switched",
    z.object({
      projectId: z.string(),
      slug: z.string(),
      path: z.string(),
    }),
  )

  export const ProjectPushed = BusEvent.define(
    "autonomous.project.pushed",
    z.object({
      projectId: z.string(),
      slug: z.string(),
      remote: z.string(),
      success: z.boolean(),
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

  // ============================================================================
  // Research Loop Events
  // ============================================================================

  export const ResearchStarted = BusEvent.define(
    "autonomous.research.started",
    z.object({
      sessionId: z.string(),
      topic: z.string(),
      dimensions: z.array(z.string()).optional(),
      sourceTypes: z.array(z.enum(["web", "financial", "news"])).optional(),
    }),
  )

  export const ResearchPhaseChanged = BusEvent.define(
    "autonomous.research.phase_changed",
    z.object({
      sessionId: z.string(),
      phase: z.enum([
        "understanding",
        "searching",
        "synthesizing",
        "analyzing",
        "reporting",
        "learning",
      ]),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const ResearchSourceFound = BusEvent.define(
    "autonomous.research.source_found",
    z.object({
      sessionId: z.string(),
      sourceCount: z.number(),
      credibilityBreakdown: z.object({
        high: z.number(),
        medium: z.number(),
        low: z.number(),
      }),
    }),
  )

  export const ResearchCompleted = BusEvent.define(
    "autonomous.research.completed",
    z.object({
      sessionId: z.string(),
      topic: z.string(),
      success: z.boolean(),
      reportMode: z.enum(["inline", "file"]),
      reportPath: z.string().optional(),
      insightCount: z.number(),
      sourceCount: z.number(),
      durationMs: z.number(),
      handCreated: z.string().optional(),
    }),
  )

  export const ResearchFailed = BusEvent.define(
    "autonomous.research.failed",
    z.object({
      sessionId: z.string(),
      topic: z.string(),
      phase: z.string(),
      error: z.string(),
      retryable: z.boolean(),
    }),
  )

  export const ResearchPatternLearned = BusEvent.define(
    "autonomous.research.pattern_learned",
    z.object({
      sessionId: z.string(),
      patternId: z.string(),
      topic: z.string(),
      keywords: z.array(z.string()),
      frequency: z.enum(["daily", "weekly", "monthly"]).optional(),
      confidence: z.number(),
    }),
  )

  // ============================================================================
  // Acceptance Loop Events (PDCA: Check)
  // ============================================================================

  export const AcceptanceStarted = BusEvent.define(
    "autonomous.acceptance.started",
    z.object({
      sessionId: z.string(),
      originalRequest: z.string(),
      checkTypes: z.array(z.enum(["quality", "requirement", "expectation"])),
    }),
  )

  export const AcceptancePhaseChanged = BusEvent.define(
    "autonomous.acceptance.phase_changed",
    z.object({
      sessionId: z.string(),
      phase: z.enum([
        "requirement_parsing",
        "quality_check",
        "conformance_check",
        "expectation_check",
        "scoring",
        "reporting",
      ]),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const AcceptanceIssueFound = BusEvent.define(
    "autonomous.acceptance.issue_found",
    z.object({
      sessionId: z.string(),
      issueId: z.string(),
      type: z.enum(["test", "type", "lint", "security", "requirement", "expectation"]),
      severity: z.enum(["critical", "high", "medium", "low"]),
      description: z.string(),
      location: z.string().optional(),
    }),
  )

  export const AcceptanceCompleted = BusEvent.define(
    "autonomous.acceptance.completed",
    z.object({
      sessionId: z.string(),
      success: z.boolean(),
      overallScore: z.number(),
      issueCount: z.number(),
      recommendation: z.enum(["pass", "fix", "rework"]),
      durationMs: z.number(),
    }),
  )

  export const AcceptanceFailed = BusEvent.define(
    "autonomous.acceptance.failed",
    z.object({
      sessionId: z.string(),
      phase: z.string(),
      error: z.string(),
      retryable: z.boolean(),
    }),
  )

  // ============================================================================
  // Fix Loop Events (PDCA: Adjust)
  // ============================================================================

  export const FixStarted = BusEvent.define(
    "autonomous.fix.started",
    z.object({
      sessionId: z.string(),
      issueCount: z.number(),
      triggerSource: z.enum(["acceptance", "manual", "test_failure"]),
    }),
  )

  export const FixPhaseChanged = BusEvent.define(
    "autonomous.fix.phase_changed",
    z.object({
      sessionId: z.string(),
      phase: z.enum([
        "issue_analysis",
        "strategy_selection",
        "fix_execution",
        "fix_verification",
        "learning",
      ]),
      metadata: z.record(z.string(), z.any()).optional(),
    }),
  )

  export const FixAttemptMade = BusEvent.define(
    "autonomous.fix.attempt_made",
    z.object({
      sessionId: z.string(),
      issueId: z.string(),
      strategy: z.enum(["auto_fix", "agent_fix", "llm_generate", "evolution"]),
      success: z.boolean(),
      durationMs: z.number(),
      error: z.string().optional(),
    }),
  )

  export const FixCompleted = BusEvent.define(
    "autonomous.fix.completed",
    z.object({
      sessionId: z.string(),
      success: z.boolean(),
      fixedCount: z.number(),
      remainingCount: z.number(),
      shouldRecheck: z.boolean(),
      durationMs: z.number(),
    }),
  )

  export const FixFailed = BusEvent.define(
    "autonomous.fix.failed",
    z.object({
      sessionId: z.string(),
      error: z.string(),
      attemptsMade: z.number(),
      retryable: z.boolean(),
    }),
  )

  export const FixPatternLearned = BusEvent.define(
    "autonomous.fix.pattern_learned",
    z.object({
      sessionId: z.string(),
      patternId: z.string(),
      issueType: z.string(),
      strategy: z.string(),
      confidence: z.number(),
    }),
  )

  // ============================================================================
  // Unified PDCA Cycle Events
  // ============================================================================

  export const PDCACycleStarted = BusEvent.define(
    "autonomous.pdca.cycle_started",
    z.object({
      sessionId: z.string(),
      taskType: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),
      cycle: z.number(),
      maxCycles: z.number(),
      strategy: z.string(),
    }),
  )

  export const PDCAPhaseChanged = BusEvent.define(
    "autonomous.pdca.phase_changed",
    z.object({
      sessionId: z.string(),
      phase: z.enum(["do", "check", "act"]),
      cycle: z.number(),
      taskType: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),
    }),
  )

  export const PDCACheckCompleted = BusEvent.define(
    "autonomous.pdca.check_completed",
    z.object({
      sessionId: z.string(),
      taskType: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),
      cycle: z.number(),
      passed: z.boolean(),
      closeScore: z.object({
        convergence: z.number(),
        leverage: z.number(),
        optionality: z.number(),
        surplus: z.number(),
        evolution: z.number(),
        total: z.number(),
      }),
      recommendation: z.enum(["pass", "fix", "rework"]),
      issueCount: z.number(),
      durationMs: z.number(),
    }),
  )

  export const PDCAActCompleted = BusEvent.define(
    "autonomous.pdca.act_completed",
    z.object({
      sessionId: z.string(),
      taskType: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),
      cycle: z.number(),
      fixed: z.boolean(),
      fixedCount: z.number(),
      remainingCount: z.number(),
      shouldRecheck: z.boolean(),
      durationMs: z.number(),
    }),
  )

  export const PDCACycleCompleted = BusEvent.define(
    "autonomous.pdca.cycle_completed",
    z.object({
      sessionId: z.string(),
      taskType: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),
      cycle: z.number(),
      success: z.boolean(),
      closeScore: z.number(),
      totalDurationMs: z.number(),
      reason: z.string().optional(),
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
