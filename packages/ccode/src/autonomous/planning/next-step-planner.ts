import { Log } from "@/util/log"
import type { Requirement } from "./requirement-tracker"
import type { TaskPriority } from "../orchestration/task-queue"
import type { ResourceBudget } from "../safety/constraints"

const log = Log.create({ service: "autonomous.next-step-planner" })

/**
 * Completion criteria for determining if execution should continue
 */
export interface CompletionCriteria {
  requirementsCompleted: boolean
  testsPassing: boolean
  verificationPassed: boolean
  noBlockingIssues: boolean
  resourceExhausted: boolean
}

/**
 * Next step plan for continuing execution
 */
export interface NextStepPlan {
  shouldContinue: boolean
  reason: string
  nextTasks: Array<{
    subject: string
    description: string
    priority: TaskPriority
  }>
  estimatedCycles: number
  confidence: number // 0-1
}

/**
 * Execution context for next step planning
 */
export interface NextStepExecutionContext {
  sessionId: string
  currentIteration: number
  totalCyclesRun: number
  lastPhaseCompleted: string
  recentErrors: string[]
  recentFailures: number
  resourceUsage: {
    tokensUsed: number
    tokensRemaining: number
    costUSD: number
    costRemaining: number
  }
}

/**
 * Next step planner configuration
 */
export interface NextStepPlannerConfig {
  autonomyLevel: "lunatic" | "insane" | "crazy" | "wild" | "bold" | "timid"
  resourceBudget: ResourceBudget
  maxFailuresBeforePause: number
  enableAutoContinue: boolean
}

/**
 * Autonomy level configurations for continuous execution
 */
type AutonomyLevelConfig = {
  autoContinue: boolean
  pauseOnImportantDecisions: boolean
  maxCycles: number
}

const AUTONOMY_CONFIGS: Record<
  NextStepPlannerConfig["autonomyLevel"],
  AutonomyLevelConfig
> = {
  lunatic: { autoContinue: true, pauseOnImportantDecisions: false, maxCycles: Infinity },
  insane: { autoContinue: true, pauseOnImportantDecisions: false, maxCycles: Infinity },
  crazy: { autoContinue: true, pauseOnImportantDecisions: true, maxCycles: Infinity },
  wild: { autoContinue: true, pauseOnImportantDecisions: true, maxCycles: 50 },
  bold: { autoContinue: false, pauseOnImportantDecisions: true, maxCycles: 20 },
  timid: { autoContinue: false, pauseOnImportantDecisions: true, maxCycles: 10 },
}

/**
 * Next step planner for determining continuous execution strategy
 *
 * Analyzes completion criteria and generates next step plans based on autonomy level
 */
export class NextStepPlanner {
  private config: NextStepPlannerConfig
  private autonomyConfig: AutonomyLevelConfig

  constructor(config: NextStepPlannerConfig) {
    this.config = config
    this.autonomyConfig = AUTONOMY_CONFIGS[config.autonomyLevel]
  }

  /**
   * Analyze completion status and determine if execution should continue
   */
  analyzeCompletion(criteria: CompletionCriteria): {
    allComplete: boolean
    canContinue: boolean
    shouldPause: boolean
    reasons: string[]
  } {
    const reasons: string[] = []
    let canContinue = true
    let shouldPause = false

    // Check resource exhaustion first
    if (criteria.resourceExhausted) {
      reasons.push("Resource budget exhausted")
      canContinue = false
      return { allComplete: false, canContinue: false, shouldPause: true, reasons }
    }

    // Check if all criteria met
    if (
      criteria.requirementsCompleted &&
      criteria.testsPassing &&
      criteria.verificationPassed &&
      criteria.noBlockingIssues
    ) {
      reasons.push("All completion criteria satisfied")
      return { allComplete: true, canContinue: false, shouldPause: false, reasons }
    }

    // Check for blocking issues
    if (!criteria.noBlockingIssues) {
      reasons.push("Blocking issues detected")
      shouldPause = !this.autonomyConfig.autoContinue
    }

    // Check test failures
    if (!criteria.testsPassing) {
      reasons.push("Tests failing")
      canContinue = true
    }

    // Check verification failures
    if (!criteria.verificationPassed) {
      reasons.push("Verification failed")
      canContinue = true
    }

    // Check incomplete requirements
    if (!criteria.requirementsCompleted) {
      reasons.push("Requirements not fully completed")
      canContinue = true
    }

    return {
      allComplete: false,
      canContinue,
      shouldPause,
      reasons,
    }
  }

  /**
   * Plan next steps based on remaining requirements and context
   */
  planNextSteps(
    pendingRequirements: Requirement[],
    context: NextStepExecutionContext,
  ): NextStepPlan {
    log.info("Planning next steps", {
      sessionId: context.sessionId,
      iteration: context.currentIteration,
      pendingCount: pendingRequirements.length,
    })

    // If no pending requirements, we're done
    if (pendingRequirements.length === 0) {
      return {
        shouldContinue: false,
        reason: "All requirements completed",
        nextTasks: [],
        estimatedCycles: 0,
        confidence: 1.0,
      }
    }

    // Sort requirements by priority and dependencies
    const sorted = this.sortRequirementsByPriority(pendingRequirements)

    // Take the next batch of requirements to work on
    const batchSize = this.calculateBatchSize(context)
    const nextBatch = sorted.slice(0, batchSize)

    // Convert requirements to tasks
    const nextTasks = nextBatch.map((req) => ({
      subject: req.description.slice(0, 60),
      description: req.description,
      priority: this.mapPriority(req.priority),
    }))

    // Estimate cycles needed
    const estimatedCycles = this.estimateCycles(nextBatch)

    // Determine if we should continue
    const shouldContinue = this.shouldContinueExecution(context)

    const confidence = this.calculateConfidence(context, nextBatch)

    return {
      shouldContinue,
      reason: this.generateReason(nextBatch, context),
      nextTasks,
      estimatedCycles,
      confidence,
    }
  }

  /**
   * Plan next steps after a test failure
   */
  planAfterTestFailure(
    failureContext: {
      failedTests: string[]
      failureCount: number
      lastError?: string
    },
    context: NextStepExecutionContext,
  ): NextStepPlan {
    const nextTasks = [
      {
        subject: "Fix failing tests",
        description: `Fix ${failureContext.failedTests.length} failing test(s)`,
        priority: "critical" as TaskPriority,
      },
    ]

    // After multiple failures, pause for review
    if (failureContext.failureCount >= this.config.maxFailuresBeforePause) {
      return {
        shouldContinue: false,
        reason: `Too many test failures (${failureContext.failureCount}), pausing for review`,
        nextTasks,
        estimatedCycles: 1,
        confidence: 0.5,
      }
    }

    return {
      shouldContinue: this.autonomyConfig.autoContinue,
      reason: "Tests failed, entering fix cycle",
      nextTasks,
      estimatedCycles: 1,
      confidence: 0.8,
    }
  }

  /**
   * Plan next steps after verification failure
   */
  planAfterVerificationFailure(
    verificationContext: {
      failedChecks: string[]
      issues: string[]
    },
    context: NextStepExecutionContext,
  ): NextStepPlan {
    const nextTasks = [
      {
        subject: "Fix verification issues",
        description: `Address ${verificationContext.issues.length} verification issue(s)`,
        priority: "high" as TaskPriority,
      },
    ]

    return {
      shouldContinue: this.autonomyConfig.autoContinue,
      reason: "Verification failed, fixing issues",
      nextTasks,
      estimatedCycles: 1,
      confidence: 0.7,
    }
  }

  /**
   * Check if execution should continue based on autonomy level
   */
  shouldContinueExecution(context: NextStepExecutionContext): boolean {
    // Check iteration limit
    if (context.currentIteration >= this.autonomyConfig.maxCycles) {
      log.info("Max iterations reached", {
        sessionId: context.sessionId,
        iteration: context.currentIteration,
        max: this.autonomyConfig.maxCycles,
      })
      return false
    }

    // Check resource budget
    const { tokensRemaining, costRemaining } = context.resourceUsage
    if (tokensRemaining <= 0 || costRemaining <= 0) {
      log.warn("Resource budget exhausted", {
        sessionId: context.sessionId,
        tokensRemaining,
        costRemaining,
      })
      return false
    }

    // Check failure count
    if (context.recentFailures >= this.config.maxFailuresBeforePause) {
      log.warn("Too many failures, pausing", {
        sessionId: context.sessionId,
        failures: context.recentFailures,
      })
      return false
    }

    return this.autonomyConfig.autoContinue
  }

  /**
   * Calculate batch size based on context
   */
  private calculateBatchSize(context: NextStepExecutionContext): number {
    // Higher autonomy = larger batches
    const autonomyBatchMultiplier: Record<
      NextStepPlannerConfig["autonomyLevel"],
      number
    > = {
      lunatic: 5,
      insane: 4,
      crazy: 3,
      wild: 2,
      bold: 1,
      timid: 1,
    }

    const baseSize = autonomyBatchMultiplier[this.config.autonomyLevel]

    // Adjust based on resource availability
    const resourceRatio = Math.min(
      context.resourceUsage.tokensRemaining / this.config.resourceBudget.maxTokens,
      context.resourceUsage.costRemaining / this.config.resourceBudget.maxCostUSD,
    )

    if (resourceRatio < 0.2) {
      return 1 // Conservative when low on resources
    }

    return baseSize
  }

  /**
   * Sort requirements by priority
   */
  private sortRequirementsByPriority(requirements: Requirement[]): Requirement[] {
    const priorityOrder: Record<Requirement["priority"], number> = {
      critical: 0,
      high: 1,
      medium: 2,
      low: 3,
    }

    return [...requirements].sort((a, b) => {
      // First sort by priority
      const priorityDiff = priorityOrder[a.priority] - priorityOrder[b.priority]
      if (priorityDiff !== 0) return priorityDiff

      // Then by dependencies (items with fewer dependencies first)
      return a.dependencies.length - b.dependencies.length
    })
  }

  /**
   * Map requirement priority to task priority
   */
  private mapPriority(priority: Requirement["priority"]): TaskPriority {
    const mapping: Record<Requirement["priority"], TaskPriority> = {
      critical: "critical",
      high: "high",
      medium: "medium",
      low: "low",
    }
    return mapping[priority]
  }

  /**
   * Estimate cycles needed for a batch of requirements
   */
  private estimateCycles(requirements: Requirement[]): number {
    // Each requirement typically needs at least one cycle
    let cycles = requirements.length

    // Add cycles for complex requirements
    for (const req of requirements) {
      if (req.description.length > 200) {
        cycles += 1
      }
      if (req.priority === "critical") {
        cycles += 1 // Extra verification cycle
      }
    }

    return cycles
  }

  /**
   * Calculate confidence in the next step plan
   */
  private calculateConfidence(context: NextStepExecutionContext, requirements: Requirement[]): number {
    let confidence = 0.8

    // Reduce confidence with many recent errors
    if (context.recentErrors.length > 3) {
      confidence -= 0.2
    }

    // Reduce confidence with many failures
    if (context.recentFailures > 2) {
      confidence -= 0.15 * context.recentFailures
    }

    // Increase confidence for low-priority, simple tasks
    const avgPriority = requirements.reduce((sum, r) => {
      const order = { critical: 4, high: 3, medium: 2, low: 1 }
      return sum + order[r.priority]
    }, 0) / requirements.length

    if (avgPriority <= 2) {
      confidence += 0.1
    }

    return Math.max(0, Math.min(1, confidence))
  }

  /**
   * Generate reason for next step
   */
  private generateReason(requirements: Requirement[], context: NextStepExecutionContext): string {
    const count = requirements.length
    const priorityGroups = requirements.reduce((acc, r) => {
      acc[r.priority] = (acc[r.priority] || 0) + 1
      return acc
    }, {} as Record<string, number>)

    const parts: string[] = []

    if (priorityGroups.critical > 0) {
      parts.push(`${priorityGroups.critical} critical`)
    }
    if (priorityGroups.high > 0) {
      parts.push(`${priorityGroups.high} high priority`)
    }
    if (priorityGroups.medium > 0) {
      parts.push(`${priorityGroups.medium} medium priority`)
    }
    if (priorityGroups.low > 0) {
      parts.push(`${priorityGroups.low} low priority`)
    }

    const priorityDesc = parts.length > 0 ? parts.join(", ") : `${count} requirement(s)`

    return `Continuing with ${priorityDesc} (iteration ${context.currentIteration + 1})`
  }
}

/**
 * Create a next step planner
 */
export function createNextStepPlanner(config: NextStepPlannerConfig): NextStepPlanner {
  return new NextStepPlanner(config)
}
