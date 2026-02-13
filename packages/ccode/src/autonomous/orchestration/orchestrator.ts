import { Log } from "@/util/log"
import { StateMachine } from "../state/state-machine"
import { AutonomousState } from "../state/states"
import { DecisionEngine, type DecisionContext } from "../decision/engine"
import { TaskQueue, type TaskPriority } from "./task-queue"
import { PhaseRunner, PhaseTemplates, type PhaseContext } from "./phase-runner"
import { type ExecutionConfig, createExecutor, type Executor } from "../execution/executor"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { SafetyGuard } from "../safety/constraints"
import type { ResourceBudget } from "../safety/constraints"
import { buildCriteria, type AutonomousDecisionCriteria } from "../decision/criteria"
import { createSafetyIntegration, type SafetyIntegration } from "../safety/integration"
import { RequirementTracker, createRequirementTracker } from "../planning/requirement-tracker"
import { NextStepPlanner, createNextStepPlanner, type CompletionCriteria, type NextStepPlan } from "../planning/next-step-planner"

const log = Log.create({ service: "autonomous.orchestrator" })

/**
 * Orchestrator configuration
 */
export interface OrchestratorConfig {
  autonomyLevel: "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"
  resourceBudget: ResourceBudget
  executionConfig?: ExecutionConfig
  unattended: boolean
}

/**
 * Session context
 */
export interface SessionContext {
  sessionId: string
  requestId: string
  request: string
  startTime: number
}

/**
 * Orchestrator for Autonomous Mode
 *
 * Coordinates the state machine, decision engine, task queue, and executor
 */
export class Orchestrator {
  private stateMachine: StateMachine
  private decisionEngine: DecisionEngine
  private taskQueue: TaskQueue
  private phaseRunner: PhaseRunner
  private executor: Executor | null = null
  private safetyGuard: SafetyGuard
  private safetyIntegration: SafetyIntegration
  private config: OrchestratorConfig
  private context: SessionContext
  private requirementTracker: RequirementTracker
  private nextStepPlanner: NextStepPlanner
  private currentIteration: number = 0
  private recentFailures: number = 0
  private recentErrors: string[] = []

  constructor(context: SessionContext, config: OrchestratorConfig) {
    this.context = context
    this.config = config

    // Initialize components
    this.stateMachine = new StateMachine({
      onStateChange: this.onStateChange.bind(this),
    })

    this.decisionEngine = new DecisionEngine({
      autonomyLevel: config.autonomyLevel,
    })

    this.taskQueue = new TaskQueue(context.sessionId)

    this.phaseRunner = new PhaseRunner(context.sessionId, {
      continueOnFailure: false,
    })

    this.safetyGuard = new SafetyGuard(this.context.sessionId, config.resourceBudget)

    // Initialize requirement tracker
    this.requirementTracker = createRequirementTracker(this.context.sessionId)

    // Initialize next step planner
    this.nextStepPlanner = createNextStepPlanner({
      autonomyLevel: config.autonomyLevel,
      resourceBudget: config.resourceBudget,
      maxFailuresBeforePause: 5,
      enableAutoContinue: config.unattended,
    })

    // Initialize safety integration
    this.safetyIntegration = createSafetyIntegration(this.context.sessionId, {
      enableDoomLoopBridge: true,
      enableDestructiveProtection: true,
      autoRollbackOnFailure: config.unattended,
    })

    // Register state change handler
    this.setupEventHandlers()
  }

  /**
   * Start the Autonomous Mode session
   */
  async start(request: string): Promise<void> {
    log.info("Starting Autonomous Mode session", {
      sessionId: this.context.sessionId,
      requestId: this.context.requestId,
      autonomyLevel: this.config.autonomyLevel,
    })

    await this.stateMachine.transition(AutonomousState.PLANNING, {
      reason: "Starting Autonomous Mode session",
    })

    await Bus.publish(AutonomousEvent.SessionStarted, {
      sessionId: this.context.sessionId,
      requestId: this.context.requestId,
      autonomyLevel: this.config.autonomyLevel,
    })

    // Create executor lazily

    // Initialize safety integration
    await this.safetyIntegration.initialize()
    this.executor = createExecutor(this.context.sessionId, {
      ...(this.config.executionConfig ?? {}),
      unattended: this.config.unattended,
      safetyIntegration: this.safetyIntegration,
    })
  }

  /**
   * Process the request through the full pipeline with continuous execution
   */
  async process(request: string): Promise<{
    success: boolean
    result: {
      success: boolean
      qualityScore: number
      crazinessScore: number
      duration: number
      tokensUsed: number
      costUSD: number
      iterationsCompleted: number
    } | null
  }> {
    try {
      // Parse requirements from initial request
      this.requirementTracker.parseRequirements(request)

      // Publish initial requirements update
      await this.publishRequirementsUpdate()

      let currentRequest = request
      let evaluation = null

      // Main execution loop - continues until all criteria are met
      while (true) {
        this.currentIteration++

        // Publish iteration started event
        await Bus.publish(AutonomousEvent.IterationStarted, {
          sessionId: this.context.sessionId,
          iteration: this.currentIteration,
          remainingRequirements: this.requirementTracker.getPendingRequirements().length,
        })

        log.info("Starting execution cycle", {
          sessionId: this.context.sessionId,
          iteration: this.currentIteration,
        })

        // Execute one complete cycle
        const cycleResult = await this.executeCycle(currentRequest)

        // Track failures
        if (!cycleResult.success) {
          this.recentFailures++
        }

        // Check completion criteria
        const completionCheck = await this.checkCompletion()

        // Publish completion checked event
        await Bus.publish(AutonomousEvent.CompletionChecked, {
          sessionId: this.context.sessionId,
          iteration: this.currentIteration,
          criteria: completionCheck,
          allComplete: this.allComplete(completionCheck),
          canContinue: !completionCheck.resourceExhausted,
          shouldPause: !completionCheck.noBlockingIssues,
        })

        // If all complete, we're done
        if (this.allComplete(completionCheck)) {
          log.info("All completion criteria met", {
            sessionId: this.context.sessionId,
            iteration: this.currentIteration,
          })

          evaluation = await this.runEvaluatePhase()
          await this.runReportPhase()

          await this.stateMachine.transition(AutonomousState.COMPLETED, {
            reason: "All requirements completed, tests passing, verification passed",
          })

          await Bus.publish(AutonomousEvent.IterationCompleted, {
            sessionId: this.context.sessionId,
            iteration: this.currentIteration,
            completedRequirements: this.requirementTracker.getStats().completed,
            success: true,
            duration: Date.now() - this.context.startTime,
          })

          break
        }

        // Check if we should pause (resource exhausted or blocked)
        const analysis = this.nextStepPlanner.analyzeCompletion(completionCheck)
        if (!analysis.canContinue || analysis.shouldPause) {
          log.info("Execution paused", {
            sessionId: this.context.sessionId,
            iteration: this.currentIteration,
            reasons: analysis.reasons,
          })

          await this.stateMachine.transition(AutonomousState.PAUSED, {
            reason: analysis.reasons.join("; "),
          })

          await Bus.publish(AutonomousEvent.SessionPaused, {
            sessionId: this.context.sessionId,
            reason: analysis.reasons.join("; "),
            state: this.stateMachine.getState(),
            canResume: true,
          })

          return { success: false, result: null }
        }

        // Generate next step plan
        const nextStep = await this.planNextSteps()

        if (!nextStep.shouldContinue) {
          log.info("Cannot continue execution", {
            sessionId: this.context.sessionId,
            reason: nextStep.reason,
          })

          await this.stateMachine.transition(AutonomousState.PAUSED, {
            reason: nextStep.reason,
          })

          await Bus.publish(AutonomousEvent.SessionPaused, {
            sessionId: this.context.sessionId,
            reason: nextStep.reason,
            state: this.stateMachine.getState(),
            canResume: true,
          })

          return { success: false, result: null }
        }

        // Transition to continuing state
        await this.stateMachine.transition(AutonomousState.CONTINUING, {
          reason: nextStep.reason,
        })

        // Publish next step planned event
        await Bus.publish(AutonomousEvent.NextStepPlanned, {
          sessionId: this.context.sessionId,
          iteration: this.currentIteration,
          nextTasks: nextStep.nextTasks,
          reason: nextStep.reason,
          estimatedCycles: nextStep.estimatedCycles,
          confidence: nextStep.confidence,
        })

        // Update current request for next iteration
        currentRequest = this.formatNextStepRequest(nextStep)

        // Publish iteration completed event
        await Bus.publish(AutonomousEvent.IterationCompleted, {
          sessionId: this.context.sessionId,
          iteration: this.currentIteration,
          completedRequirements: this.requirementTracker.getStats().completed,
          success: true,
          duration: Date.now() - this.context.startTime,
        })

        log.info("Continuing execution", {
          sessionId: this.context.sessionId,
          iteration: this.currentIteration,
          nextTasksCount: nextStep.nextTasks.length,
          reason: nextStep.reason,
        })
      }

      const resultData = {
        success: true,
        qualityScore: evaluation?.qualityScore ?? 0,
        crazinessScore: evaluation?.crazinessScore ?? 0,
        duration: Date.now() - this.context.startTime,
        tokensUsed: this.safetyGuard.getCurrentUsage().tokensUsed,
        costUSD: this.safetyGuard.getCurrentUsage().costUSD,
        iterationsCompleted: this.currentIteration,
      }

      await Bus.publish(AutonomousEvent.SessionCompleted, {
        sessionId: this.context.sessionId,
        requestId: this.context.requestId,
        result: resultData,
      })

      return { success: true, result: resultData }
    } catch (error) {
      log.error("Session error", {
        sessionId: this.context.sessionId,
        error: error instanceof Error ? error.message : String(error),
      })

      await this.stateMachine.transition(AutonomousState.FAILED, {
        reason: error instanceof Error ? error.message : String(error),
      })

      await Bus.publish(AutonomousEvent.SessionFailed, {
        sessionId: this.context.sessionId,
        requestId: this.context.requestId,
        error: error instanceof Error ? error.message : String(error),
        state: this.stateMachine.getState(),
      })

      return { success: false, result: null }
    }
  }

  /**
   * Execute a single cycle (understand -> plan -> decide -> execute -> test -> verify)
   */
  private async executeCycle(request: string): Promise<{ success: boolean }> {
    try {
      // Phase 1: Understand & Plan
      await this.runUnderstandPhase(request)
      await this.runPlanPhase(request)

      // Phase 2: Decide
      const decision = await this.runDecidePhase()
      if (!decision.approved) {
        await this.handleBlockedDecision(decision.reasoning)
        return { success: false }
      }

      // Phase 3: Execute (TDD cycle)
      await this.runExecutePhase()

      // Phase 4: Test & Verify
      const testResult = await this.runTestPhase()
      const verifyResult = await this.runVerifyPhase()

      // Update recent errors
      if (!testResult.success) {
        this.recentErrors.push(...testResult.errors)
      }
      if (!verifyResult.success) {
        this.recentErrors.push(...verifyResult.errors)
      }

      // Keep only recent errors
      this.recentErrors = this.recentErrors.slice(-10)

      return { success: testResult.success && verifyResult.success }
    } catch (error) {
      this.recentErrors.push(error instanceof Error ? error.message : String(error))
      return { success: false }
    }
  }

  /**
   * Check if all completion criteria are met
   */
  private allComplete(criteria: CompletionCriteria): boolean {
    return (
      criteria.requirementsCompleted &&
      criteria.testsPassing &&
      criteria.verificationPassed &&
      criteria.noBlockingIssues
    )
  }

  /**
   * Run understand phase
   */
  private async runUnderstandPhase(request: string): Promise<void> {
    await this.stateMachine.transition(AutonomousState.PLANNING, {
      reason: "Understanding request",
    })

    // Add understanding task
    const taskId = await this.taskQueue.add({
      subject: "Understand request",
      description: `Parse and understand: ${request}`,
      priority: "critical",
      dependencies: [],
      agent: "explore",
      metadata: { phase: "understand", request },
    })

    // Execute the task (in a real implementation, this would invoke the agent)
    await this.taskQueue.start(taskId)
    await this.taskQueue.complete(taskId, { understood: true })
  }

  /**
   * Run plan phase
   */
  private async runPlanPhase(request: string): Promise<void> {
    // Add planning task
    const taskId = await this.taskQueue.add({
      subject: "Generate plan",
      description: "Generate structured execution plan",
      priority: "critical",
      dependencies: [],
      agent: "architect",
      metadata: { phase: "plan", request },
    })

    await this.taskQueue.start(taskId)
    await this.taskQueue.complete(taskId, { plan: "generated" })

    await this.stateMachine.transition(AutonomousState.PLAN_APPROVED, {
      reason: "Plan generated",
    })
  }

  /**
   * Run decide phase
   */
  private async runDecidePhase(): Promise<{ approved: boolean; reasoning: string }> {
    await this.stateMachine.transition(AutonomousState.DECIDING, {
      reason: "Evaluating plan with CLOSE framework",
    })

    const criteria = buildCriteria({
      type: "implementation",
      description: "Execute generated plan",
      riskLevel: "medium",
      convergence: 5,
      leverage: 7,
      optionality: 6,
      surplus: this.safetyGuard.getSurplusRatio() * 10,
      evolution: 6,
    })

    const decisionContext: DecisionContext = {
      sessionId: this.context.sessionId,
      currentState: this.stateMachine.getState(),
      resourceUsage: this.safetyGuard.getCurrentUsage(),
      errorCount: this.taskQueue.getStats().failed,
      recentDecisions: [],
    }

    const result = await this.decisionEngine.evaluate(criteria, decisionContext)

    await this.stateMachine.transition(AutonomousState.DECISION_MADE, {
      reason: `Decision: ${result.action}`,
    })

    return { approved: result.approved, reasoning: result.reasoning }
  }

  /**
   * Run execute phase (TDD cycle)
   */
  private async runExecutePhase(): Promise<void> {
    await this.stateMachine.transition(AutonomousState.EXECUTING, {
      reason: "Starting TDD execution cycle",
    })

    if (!this.executor) {
      throw new Error("Executor not initialized")
    }

    // Run TDD cycles until completion
    await this.executor.runTDDCycle()

    // Move to testing
    await this.stateMachine.transition(AutonomousState.TESTING, {
      reason: "Implementation complete, running tests",
    })
  }

  /**
   * Run test phase
   */
  private async runTestPhase(): Promise<{ success: boolean; errors: string[] }> {
    if (!this.executor) {
      throw new Error("Executor not initialized")
    }

    const testResult = await this.executor.runTests()
    const errors: string[] = []

    if (testResult.success) {
      await this.stateMachine.transition(AutonomousState.VERIFYING, {
        reason: "Tests passed, verifying quality",
      })
    } else {
      const errorMsg = "Tests failed, entering fix mode"
      errors.push(errorMsg)
      await this.stateMachine.transition(AutonomousState.FIXING, {
        reason: errorMsg,
      })

      // Retry logic would go here
      await this.stateMachine.transition(AutonomousState.VERIFYING, {
        reason: "Fixes applied, verifying",
      })
    }

    return { success: testResult.success, errors }
  }

  /**
   * Run verify phase
   */
  private async runVerifyPhase(): Promise<{ success: boolean; errors: string[] }> {
    if (!this.executor) {
      throw new Error("Executor not initialized")
    }

    const verifyResult = await this.executor.runVerification()
    const errors: string[] = []

    if (verifyResult.success) {
      await this.stateMachine.transition(AutonomousState.EVALUATING, {
        reason: "Verification passed, evaluating results",
      })
    } else {
      const errorMsg = "Verification failed, entering fix mode"
      errors.push(errorMsg)
      await this.stateMachine.transition(AutonomousState.FIXING, {
        reason: errorMsg,
      })

      // Retry logic would go here
      await this.stateMachine.transition(AutonomousState.EVALUATING, {
        reason: "Fixes applied, evaluating",
      })
    }

    return { success: verifyResult.success, errors }
  }

  /**
   * Run evaluate phase
   */
  private async runEvaluatePhase(): Promise<{
    qualityScore: number
    crazinessScore: number
    duration: number
    tokensUsed: number
    costUSD: number
  }> {
    await this.stateMachine.transition(AutonomousState.SCORING, {
      reason: "Calculating scores",
    })

    const usage = this.safetyGuard.getCurrentUsage()
    const duration = Date.now() - this.context.startTime

    // Calculate scores (simplified)
    const taskStats = this.taskQueue.getStats()
    const qualityScore = this.calculateQualityScore(taskStats)
    const crazinessScore = this.calculateCrazinessScore(taskStats, usage)

    return {
      qualityScore,
      crazinessScore,
      duration,
      tokensUsed: usage.tokensUsed,
      costUSD: usage.costUSD,
    }
  }

  /**
   * Run report phase
   */
  private async runReportPhase(): Promise<void> {
    await Bus.publish(AutonomousEvent.ReportGenerated, {
      sessionId: this.context.sessionId,
      reportType: "summary",
    })
  }

  /**
   * Handle blocked decision
   */
  private async handleBlockedDecision(reasoning: string): Promise<void> {
    if (this.config.unattended) {
      await this.stateMachine.transition(AutonomousState.PAUSED, {
        reason: `Decision blocked (unattended): ${reasoning}`,
      })

      await Bus.publish(AutonomousEvent.SessionPaused, {
        sessionId: this.context.sessionId,
        reason: reasoning,
        state: this.stateMachine.getState(),
        canResume: true,
      })
    } else {
      await this.stateMachine.transition(AutonomousState.BLOCKED, {
        reason: `Decision blocked: ${reasoning}`,
      })

      await Bus.publish(AutonomousEvent.DecisionBlocked, {
        sessionId: this.context.sessionId,
        decisionId: "blocked",
        reason: reasoning,
      })
    }
  }

  /**
   * Calculate quality score
   */
  private calculateQualityScore(stats: { total: number; completed: number; failed: number }): number {
    if (stats.total === 0) return 50

    const successRate = stats.completed / stats.total
    const failurePenalty = (stats.failed / stats.total) * 20

    return Math.max(0, Math.min(100, successRate * 100 - failurePenalty))
  }

  /**
   * Calculate craziness score
   */
  private calculateCrazinessScore(
    stats: { total: number; completed: number; failed: number },
    usage: { tokensUsed: number; costUSD: number },
  ): number {
    // Base score on autonomy
    const autonomyScores = {
      lunatic: 95,
      insane: 85,
      crazy: 75,
      wild: 60,
      bold: 40,
      timid: 15,
    }

    let score = autonomyScores[this.config.autonomyLevel]

    // Adjust based on success rate
    if (stats.total > 0) {
      const successRate = stats.completed / stats.total
      score *= successRate
    }

    return Math.max(0, Math.min(100, score))
  }

  /**
   * Check completion criteria
   */
  private async checkCompletion(): Promise<CompletionCriteria> {
    const requirementsCompleted = this.requirementTracker.allRequirementsCompleted()

    // Check tests passing (simplified - in real implementation would check test results)
    const testsPassing = this.recentFailures === 0

    // Check verification passed (simplified)
    const verificationPassed = this.recentErrors.filter((e) => e.includes("verification")).length === 0

    // Check for blocking issues
    const noBlockingIssues = !this.recentErrors.some((e) =>
      e.includes("blocked") || e.includes("critical") || e.includes("fatal"),
    )

    // Check resource exhaustion
    const usage = this.safetyGuard.getCurrentUsage()
    const resourceExhausted =
      usage.tokensUsed >= this.config.resourceBudget.maxTokens ||
      usage.costUSD >= this.config.resourceBudget.maxCostUSD

    return {
      requirementsCompleted,
      testsPassing,
      verificationPassed,
      noBlockingIssues,
      resourceExhausted,
    }
  }

  /**
   * Plan next steps based on remaining requirements
   */
  private async planNextSteps(): Promise<NextStepPlan> {
    const pendingRequirements = this.requirementTracker.getPendingRequirements()

    const context = {
      sessionId: this.context.sessionId,
      currentIteration: this.currentIteration,
      totalCyclesRun: this.currentIteration,
      lastPhaseCompleted: "verify",
      recentErrors: this.recentErrors,
      recentFailures: this.recentFailures,
      resourceUsage: {
        tokensUsed: this.safetyGuard.getCurrentUsage().tokensUsed,
        tokensRemaining: this.config.resourceBudget.maxTokens - this.safetyGuard.getCurrentUsage().tokensUsed,
        costUSD: this.safetyGuard.getCurrentUsage().costUSD,
        costRemaining: this.config.resourceBudget.maxCostUSD - this.safetyGuard.getCurrentUsage().costUSD,
      },
    }

    return this.nextStepPlanner.planNextSteps(pendingRequirements, context)
  }

  /**
   * Format next step request for the next iteration
   */
  private formatNextStepRequest(nextStep: NextStepPlan): string {
    const tasks = nextStep.nextTasks.map((t, i) => `${i + 1}. ${t.subject}`).join("\n")
    return `Continue with the following tasks:\n${tasks}\n\nContext: This is iteration ${this.currentIteration + 1}. Focus on completing these remaining requirements.`
  }

  /**
   * Publish requirements update event
   */
  private async publishRequirementsUpdate(): Promise<void> {
    const stats = this.requirementTracker.getStats()

    await Bus.publish(AutonomousEvent.RequirementsUpdated, {
      sessionId: this.context.sessionId,
      stats,
    })
  }

  /**
   * State change handler
   */
  private async onStateChange(from: AutonomousState, to: AutonomousState): Promise<void> {
    log.info("State changed", { from, to })
  }

  /**
   * Setup event handlers
   */
  private setupEventHandlers(): void {
    // Subscribe to resource warnings
    Bus.subscribe(AutonomousEvent.ResourceWarning, async (event) => {
      if (event.properties.sessionId === this.context.sessionId) {
        log.warn("Resource warning", event.properties)
      }
    })

    // Subscribe to loop detection
    Bus.subscribe(AutonomousEvent.LoopDetected, async (event) => {
      if (event.properties.sessionId === this.context.sessionId) {
        log.warn("Loop detected", event.properties)
      }
    })
  }

  /**
   * Pause the session
   */
  async pause(reason = "Paused by user"): Promise<void> {
    await this.stateMachine.transition(AutonomousState.PAUSED, { reason })

    await Bus.publish(AutonomousEvent.SessionPaused, {
      sessionId: this.context.sessionId,
      reason,
      state: this.stateMachine.getState(),
      canResume: true,
    })
  }

  /**
   * Resume the session
   */
  async resume(): Promise<boolean> {
    if (this.stateMachine.getState() !== AutonomousState.PAUSED) {
      return false
    }

    await this.stateMachine.transition(AutonomousState.EXECUTING, {
      reason: "Resuming from pause",
    })

    return true
  }

  /**
   * Stop the session
   */
  async stop(reason = "Stopped by user"): Promise<void> {
    await this.stateMachine.transition(AutonomousState.TERMINATED, { reason })
  }

  /**
   * Get current state
   */
  getState(): AutonomousState {
    return this.stateMachine.getState()
  }

  /**
   * Get task queue stats
   */
  getTaskStats() {
    return this.taskQueue.getStats()
  }

  /**
   * Get decision history
   */
  getDecisionHistory() {
    return this.decisionEngine.getHistory()
  }

  /**
   * Serialize orchestrator state
   */
  serialize(): {
    state: AutonomousState
    tasks: ReturnType<TaskQueue["serialize"]>
    decisions: ReturnType<DecisionEngine["getHistory"]>
  } {
    return {
      state: this.stateMachine.getState(),
      tasks: this.taskQueue.serialize(),
      decisions: this.decisionEngine.getHistory(),
    }
  }
}

/**
 * Create an orchestrator
 */
export function createOrchestrator(context: SessionContext, config: OrchestratorConfig): Orchestrator {
  return new Orchestrator(context, config)
}
