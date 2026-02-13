import { describe, test, expect, mock, beforeEach, afterEach } from "bun:test"
import { RequirementTracker, createRequirementTracker } from "@/autonomous/planning/requirement-tracker"
import { NextStepPlanner, createNextStepPlanner } from "@/autonomous/planning/next-step-planner"
import { AutonomousState, isValidTransition, getStateCategory } from "@/autonomous/state/states"
import type { CompletionCriteria, NextStepExecutionContext } from "@/autonomous/planning/next-step-planner"
import type { ResourceBudget } from "@/autonomous/safety/constraints"

describe("RequirementTracker", () => {
  let tracker: RequirementTracker

  beforeEach(() => {
    tracker = createRequirementTracker("test-session")
  })

  test("should parse simple requirement from request", () => {
    const result = tracker.parseRequirements("Implement user authentication")

    expect(result.requirements).toHaveLength(1)
    expect(result.requirements[0].description).toBe("Implement user authentication")
    expect(result.requirements[0].status).toBe("pending")
    expect(result.requirements[0].priority).toBe("high")
    expect(result.requirements[0].source).toBe("original")
  })

  test("should parse multiple requirements from request", () => {
    const request = `
      Must implement user login
      Should add password reset functionality
      Could add social login
    `
    const result = tracker.parseRequirements(request)

    expect(result.requirements.length).toBeGreaterThanOrEqual(1)
    const hasLogin = result.requirements.some((r) =>
      r.description.toLowerCase().includes("login"),
    )
    expect(hasLogin).toBe(true)
  })

  test("should track requirement status updates", () => {
    tracker.parseRequirements("Implement feature X")
    const req = tracker.getAllRequirements()[0]

    tracker.updateStatus(req.id, "in_progress")
    expect(tracker.getRequirement(req.id)?.status).toBe("in_progress")

    tracker.updateStatus(req.id, "completed")
    expect(tracker.getRequirement(req.id)?.status).toBe("completed")
  })

  test("should detect all requirements completed", () => {
    tracker.parseRequirements("Implement feature X")

    expect(tracker.allRequirementsCompleted()).toBe(false)

    const req = tracker.getAllRequirements()[0]
    tracker.updateStatus(req.id, "completed")

    expect(tracker.allRequirementsCompleted()).toBe(true)
  })

  test("should calculate completion percentage", () => {
    tracker.parseRequirements("Implement feature A")
    tracker.parseRequirements("Implement feature B")

    expect(tracker.getCompletionPercentage()).toBe(0)

    const reqs = tracker.getAllRequirements()
    tracker.updateStatus(reqs[0].id, "completed")

    expect(tracker.getCompletionPercentage()).toBe(50)

    tracker.updateStatus(reqs[1].id, "completed")

    expect(tracker.getCompletionPercentage()).toBe(100)
  })

  test("should add derived requirements", () => {
    tracker.parseRequirements("Implement feature X")

    const derivedId = tracker.addDerivedRequirement("Add tests for feature X", "high")

    const derivedReq = tracker.getRequirement(derivedId)
    expect(derivedReq).toBeDefined()
    expect(derivedReq?.description).toBe("Add tests for feature X")
    expect(derivedReq?.priority).toBe("high")
    expect(derivedReq?.source).toBe("derived")
  })

  test("should update acceptance criteria status", () => {
    tracker.parseRequirements("Implement feature X")
    const req = tracker.getAllRequirements()[0]
    const criterion = req.acceptanceCriteria[0]

    tracker.updateCriterionStatus(req.id, criterion.id, "passed")

    expect(tracker.getRequirement(req.id)?.acceptanceCriteria[0].status).toBe("passed")
  })

  test("should provide statistics", () => {
    tracker.parseRequirements("Feature A")
    tracker.parseRequirements("Feature B")
    tracker.parseRequirements("Feature C")

    const reqs = tracker.getAllRequirements()
    tracker.updateStatus(reqs[0].id, "completed")
    tracker.updateStatus(reqs[1].id, "in_progress")

    const stats = tracker.getStats()
    expect(stats.total).toBe(3)
    expect(stats.completed).toBe(1)
    expect(stats.inProgress).toBe(1)
    expect(stats.pending).toBe(1)
    expect(stats.completionPercentage).toBe(33)
  })
})

describe("NextStepPlanner", () => {
  const baseConfig = {
    autonomyLevel: "crazy" as const,
    resourceBudget: {
      maxTokens: 100000,
      maxCostUSD: 10,
      maxDurationMinutes: 300,
      maxFilesChanged: 100,
      maxActions: 1000,
    } as ResourceBudget,
    maxFailuresBeforePause: 5,
    enableAutoContinue: true,
  }

  test("should analyze completion with all criteria met", () => {
    const planner = createNextStepPlanner(baseConfig)
    const criteria: CompletionCriteria = {
      requirementsCompleted: true,
      testsPassing: true,
      verificationPassed: true,
      noBlockingIssues: true,
      resourceExhausted: false,
    }

    const analysis = planner.analyzeCompletion(criteria)

    expect(analysis.allComplete).toBe(true)
    expect(analysis.canContinue).toBe(false)
    expect(analysis.reasons).toContain("All completion criteria satisfied")
  })

  test("should analyze completion with incomplete requirements", () => {
    const planner = createNextStepPlanner(baseConfig)
    const criteria: CompletionCriteria = {
      requirementsCompleted: false,
      testsPassing: true,
      verificationPassed: true,
      noBlockingIssues: true,
      resourceExhausted: false,
    }

    const analysis = planner.analyzeCompletion(criteria)

    expect(analysis.allComplete).toBe(false)
    expect(analysis.canContinue).toBe(true)
    expect(analysis.reasons).toContain("Requirements not fully completed")
  })

  test("should stop when resource exhausted", () => {
    const planner = createNextStepPlanner(baseConfig)
    const criteria: CompletionCriteria = {
      requirementsCompleted: false,
      testsPassing: true,
      verificationPassed: true,
      noBlockingIssues: true,
      resourceExhausted: true,
    }

    const analysis = planner.analyzeCompletion(criteria)

    expect(analysis.allComplete).toBe(false)
    expect(analysis.canContinue).toBe(false)
    expect(analysis.shouldPause).toBe(true)
    expect(analysis.reasons).toContain("Resource budget exhausted")
  })

  test("should plan next steps for pending requirements", () => {
    const planner = createNextStepPlanner(baseConfig)

    const pendingRequirements = [
      {
        id: "req-1",
        description: "Implement feature A",
        status: "pending" as const,
        priority: "high" as const,
        acceptanceCriteria: [],
        createdAt: Date.now(),
        updatedAt: Date.now(),
        dependencies: [],
        source: "original" as const,
      },
      {
        id: "req-2",
        description: "Implement feature B",
        status: "pending" as const,
        priority: "medium" as const,
        acceptanceCriteria: [],
        createdAt: Date.now(),
        updatedAt: Date.now(),
        dependencies: [],
        source: "original" as const,
      },
    ]

    const context: NextStepExecutionContext = {
      sessionId: "test-session",
      currentIteration: 1,
      totalCyclesRun: 1,
      lastPhaseCompleted: "verify",
      recentErrors: [],
      recentFailures: 0,
      resourceUsage: {
        tokensUsed: 10000,
        tokensRemaining: 90000,
        costUSD: 1,
        costRemaining: 9,
      },
    }

    const plan = planner.planNextSteps(pendingRequirements, context)

    expect(plan.shouldContinue).toBe(true)
    expect(plan.nextTasks.length).toBeGreaterThan(0)
    expect(plan.estimatedCycles).toBeGreaterThan(0)
    expect(plan.reason).toContain("iteration")
  })

  test("should not continue when no pending requirements", () => {
    const planner = createNextStepPlanner(baseConfig)

    const plan = planner.planNextSteps([], {
      sessionId: "test-session",
      currentIteration: 1,
      totalCyclesRun: 1,
      lastPhaseCompleted: "verify",
      recentErrors: [],
      recentFailures: 0,
      resourceUsage: {
        tokensUsed: 10000,
        tokensRemaining: 90000,
        costUSD: 1,
        costRemaining: 9,
      },
    })

    expect(plan.shouldContinue).toBe(false)
    expect(plan.nextTasks).toHaveLength(0)
    expect(plan.estimatedCycles).toBe(0)
    expect(plan.reason).toContain("All requirements completed")
  })

  test("should respect autonomy level for continuation", () => {
    const timidConfig = { ...baseConfig, autonomyLevel: "timid" as const }
    const timidPlanner = createNextStepPlanner(timidConfig)

    const context: NextStepExecutionContext = {
      sessionId: "test",
      currentIteration: 5,
      totalCyclesRun: 5,
      lastPhaseCompleted: "verify",
      recentErrors: [],
      recentFailures: 0,
      resourceUsage: {
        tokensUsed: 10000,
        tokensRemaining: 90000,
        costUSD: 1,
        costRemaining: 9,
      },
    }
    const timidContinue = timidPlanner.shouldContinueExecution(context)

    // Timid should pause after a few iterations
    expect(timidContinue).toBe(false)
  })

  test("should pause after too many failures", () => {
    const planner = createNextStepPlanner(baseConfig)

    const context: NextStepExecutionContext = {
      sessionId: "test",
      currentIteration: 1,
      totalCyclesRun: 1,
      lastPhaseCompleted: "verify",
      recentErrors: [],
      recentFailures: 10, // Exceeds maxFailuresBeforePause
      resourceUsage: {
        tokensUsed: 10000,
        tokensRemaining: 90000,
        costUSD: 1,
        costRemaining: 9,
      },
    }
    const shouldContinue = planner.shouldContinueExecution(context)

    expect(shouldContinue).toBe(false)
  })

  test("should plan after test failure", () => {
    const planner = createNextStepPlanner(baseConfig)

    const context: NextStepExecutionContext = {
      sessionId: "test",
      currentIteration: 1,
      totalCyclesRun: 1,
      lastPhaseCompleted: "test",
      recentErrors: [],
      recentFailures: 1,
      resourceUsage: {
        tokensUsed: 10000,
        tokensRemaining: 90000,
        costUSD: 1,
        costRemaining: 9,
      },
    }
    const plan = planner.planAfterTestFailure(
      {
        failedTests: ["test-1", "test-2"],
        failureCount: 1,
        lastError: "Assertion failed",
      },
      context,
    )

    expect(plan.nextTasks).toHaveLength(1)
    expect(plan.nextTasks[0].subject).toContain("Fix")
    expect(plan.nextTasks[0].priority).toBe("critical")
  })

  test("should pause after too many test failures", () => {
    const planner = createNextStepPlanner(baseConfig)

    const context: NextStepExecutionContext = {
      sessionId: "test",
      currentIteration: 1,
      totalCyclesRun: 1,
      lastPhaseCompleted: "test",
      recentErrors: [],
      recentFailures: 10,
      resourceUsage: {
        tokensUsed: 10000,
        tokensRemaining: 90000,
        costUSD: 1,
        costRemaining: 9,
      },
    }
    const plan = planner.planAfterTestFailure(
      {
        failedTests: ["test-1"],
        failureCount: 10, // Exceeds maxFailuresBeforePause
      },
      context,
    )

    expect(plan.shouldContinue).toBe(false)
    expect(plan.reason).toContain("pausing for review")
  })
})

describe("Continuous Execution Integration", () => {
  test("should handle full cycle of requirement tracking and planning", () => {
    const tracker = createRequirementTracker("test-session")
    const planner = createNextStepPlanner({
      autonomyLevel: "crazy" as const,
      resourceBudget: {
        maxTokens: 100000,
        maxCostUSD: 10,
        maxDurationMinutes: 300,
        maxFilesChanged: 100,
        maxActions: 1000,
      } as ResourceBudget,
      maxFailuresBeforePause: 5,
      enableAutoContinue: true,
    })

    // Parse initial request
    const parseResult = tracker.parseRequirements("Implement user authentication system")

    expect(parseResult.requirements).toHaveLength(1)
    expect(tracker.getCompletionPercentage()).toBe(0)

    // Simulate first cycle - requirement still pending
    let completionCheck: CompletionCriteria = {
      requirementsCompleted: false,
      testsPassing: false,
      verificationPassed: true,
      noBlockingIssues: true,
      resourceExhausted: false,
    }

    let analysis = planner.analyzeCompletion(completionCheck)
    expect(analysis.allComplete).toBe(false)
    expect(analysis.canContinue).toBe(true)

    // Generate next steps
    const pending = tracker.getPendingRequirements()
    const context: NextStepExecutionContext = {
      sessionId: "test-session",
      currentIteration: 1,
      totalCyclesRun: 1,
      lastPhaseCompleted: "verify",
      recentErrors: [],
      recentFailures: 0,
      resourceUsage: {
        tokensUsed: 10000,
        tokensRemaining: 90000,
        costUSD: 1,
        costRemaining: 9,
      },
    }
    const nextStep = planner.planNextSteps(pending, context)

    expect(nextStep.shouldContinue).toBe(true)
    expect(nextStep.nextTasks.length).toBeGreaterThan(0)

    // Simulate completion
    const req = tracker.getAllRequirements()[0]
    tracker.updateStatus(req.id, "completed")

    completionCheck = {
      requirementsCompleted: true,
      testsPassing: true,
      verificationPassed: true,
      noBlockingIssues: true,
      resourceExhausted: false,
    }

    analysis = planner.analyzeCompletion(completionCheck)
    expect(analysis.allComplete).toBe(true)
    expect(analysis.canContinue).toBe(false)
  })
})

describe("AutonomousState with CONTINUING", () => {
  test("should have CONTINUING state", () => {
    expect(AutonomousState.CONTINUING).toBe(AutonomousState.CONTINUING)
  })

  test("should allow transition from SCORING to CONTINUING", () => {
    expect(isValidTransition(AutonomousState.SCORING, AutonomousState.CONTINUING)).toBe(true)
  })

  test("should allow transition from CONTINUING to PLANNING", () => {
    expect(isValidTransition(AutonomousState.CONTINUING, AutonomousState.PLANNING)).toBe(true)
  })

  test("should allow transition from CONTINUING to EXECUTING", () => {
    expect(isValidTransition(AutonomousState.CONTINUING, AutonomousState.EXECUTING)).toBe(true)
  })

  test("should categorize CONTINUING as active state", () => {
    expect(getStateCategory(AutonomousState.CONTINUING)).toBe("active")
  })
})
