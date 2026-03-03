/**
 * Implementation Strategy
 *
 * Acceptance strategy for implementation/code tasks.
 * Wraps the existing AcceptanceLoop and FixLoop for compatibility.
 */

import { Log } from "@/util/log"
import { BaseAcceptanceStrategy } from "./base"
import type { TaskType } from "../../classification/types"
import type {
  TaskExecutionResult,
  PDCACheckResult,
  PDCAActResult,
  PDCAIssue,
  PDCAConfig,
  ImplementationOutput,
  CheckItemResult,
} from "../types"

const log = Log.create({ service: "pdca.strategy.implementation" })

// ============================================================================
// Check Items for Implementation Tasks
// ============================================================================

const IMPLEMENTATION_CHECK_ITEMS = {
  tests: {
    name: "tests",
    description: "All tests pass",
    weight: 1.2,
  },
  typecheck: {
    name: "typecheck",
    description: "No TypeScript type errors",
    weight: 1.1,
  },
  lint: {
    name: "lint",
    description: "Code follows lint rules",
    weight: 0.8,
  },
  security: {
    name: "security",
    description: "No security vulnerabilities",
    weight: 1.5,
  },
  requirement: {
    name: "requirement",
    description: "Requirements are met",
    weight: 1.3,
  },
  expectation: {
    name: "expectation",
    description: "User expectations are satisfied",
    weight: 1.0,
  },
} as const

// ============================================================================
// Implementation Strategy
// ============================================================================

export class ImplementationStrategy extends BaseAcceptanceStrategy<ImplementationOutput> {
  readonly taskType: TaskType = "implementation"
  readonly name = "Implementation Acceptance Strategy"

  getCheckItems(): string[] {
    return Object.keys(IMPLEMENTATION_CHECK_ITEMS)
  }

  getCheckWeights(): Record<string, number> {
    return Object.fromEntries(
      Object.entries(IMPLEMENTATION_CHECK_ITEMS).map(([k, v]) => [k, v.weight]),
    )
  }

  async check(
    result: TaskExecutionResult<ImplementationOutput>,
    originalRequest: string,
    config: PDCAConfig,
  ): Promise<PDCACheckResult> {
    const startTime = Date.now()
    const issues: PDCAIssue[] = []
    const checks: Record<string, CheckItemResult> = {}

    log.debug("Running implementation acceptance check", {
      sessionId: config.sessionId,
      solved: result.output?.solved,
    })

    // Use existing AcceptanceLoop for comprehensive checking
    const { createAcceptanceLoop } = await import("../../execution/acceptance-loop")

    const acceptanceLoop = createAcceptanceLoop({
      enableTests: true,
      enableTypecheck: true,
      enableLint: true,
      enableSecurityScan: true,
      enableExpectationCheck: true,
      passThreshold: config.passThreshold,
      fixThreshold: config.fixThreshold,
    })

    // Build acceptance problem from execution result
    const acceptanceProblem = {
      sessionId: config.sessionId,
      originalRequest,
      implementationResult: result.output?.solved
        ? {
            solved: true,
            summary: result.output.summary,
            solution: result.output.solution,
            knowledgeId: result.output.knowledgeId,
            attempts: [],
            durationMs: result.durationMs,
          }
        : undefined,
      workingDir: result.metadata?.workingDir as string | undefined,
      modifiedFiles: result.output?.modifiedFiles,
    }

    try {
      const acceptanceResult = await acceptanceLoop.accept(acceptanceProblem)

      // Map AcceptanceLoop result to PDCACheckResult format
      checks.tests = {
        passed: acceptanceResult.checks.codeQuality.testsPassed,
        score: acceptanceResult.checks.codeQuality.testsPassed ? 10 : 0,
        weight: IMPLEMENTATION_CHECK_ITEMS.tests.weight,
        details: `${acceptanceResult.checks.codeQuality.testsTotal - acceptanceResult.checks.codeQuality.testsFailed}/${acceptanceResult.checks.codeQuality.testsTotal} tests passed`,
      }

      checks.typecheck = {
        passed: acceptanceResult.checks.codeQuality.typecheckPassed,
        score: acceptanceResult.checks.codeQuality.typecheckPassed ? 10 : 0,
        weight: IMPLEMENTATION_CHECK_ITEMS.typecheck.weight,
        details: acceptanceResult.checks.codeQuality.typecheckErrors.length > 0
          ? `${acceptanceResult.checks.codeQuality.typecheckErrors.length} type errors`
          : "No type errors",
      }

      checks.lint = {
        passed: acceptanceResult.checks.codeQuality.lintPassed,
        score: acceptanceResult.checks.codeQuality.lintPassed ? 10 : 5,
        weight: IMPLEMENTATION_CHECK_ITEMS.lint.weight,
        details: acceptanceResult.checks.codeQuality.lintIssues.length > 0
          ? `${acceptanceResult.checks.codeQuality.lintIssues.length} lint issues`
          : "No lint issues",
      }

      checks.security = {
        passed: acceptanceResult.checks.codeQuality.securityPassed,
        score: acceptanceResult.checks.codeQuality.securityPassed ? 10 : 0,
        weight: IMPLEMENTATION_CHECK_ITEMS.security.weight,
        details: acceptanceResult.checks.codeQuality.securityIssues.length > 0
          ? `${acceptanceResult.checks.codeQuality.securityIssues.length} security issues`
          : "No security issues",
      }

      const reqReport = acceptanceResult.checks.requirementConformance
      checks.requirement = {
        passed: reqReport.notMet === 0,
        score: reqReport.total > 0
          ? ((reqReport.met + reqReport.partial * 0.5) / reqReport.total) * 10
          : 10,
        weight: IMPLEMENTATION_CHECK_ITEMS.requirement.weight,
        details: `${reqReport.met}/${reqReport.total} requirements met`,
      }

      checks.expectation = {
        passed: acceptanceResult.checks.userExpectation.alignmentScore >= 7,
        score: acceptanceResult.checks.userExpectation.alignmentScore,
        weight: IMPLEMENTATION_CHECK_ITEMS.expectation.weight,
        details: acceptanceResult.checks.userExpectation.reasoning,
      }

      // Convert acceptance issues to PDCA issues
      for (const issue of acceptanceResult.issues) {
        issues.push({
          id: issue.id,
          category: issue.type,
          severity: issue.severity,
          description: issue.description,
          location: issue.location,
          suggestedAction: issue.suggestedFix,
        })
      }

      const durationMs = Date.now() - startTime

      return {
        taskType: this.taskType,
        passed: acceptanceResult.success,
        closeScore: acceptanceResult.closeScores,
        issues,
        recommendation: acceptanceResult.recommendation,
        checks,
        durationMs,
        report: acceptanceResult.report,
      }
    } catch (error) {
      log.error("Implementation check failed", { error })

      // Return failure result
      const durationMs = Date.now() - startTime
      return {
        taskType: this.taskType,
        passed: false,
        closeScore: {
          convergence: 0,
          leverage: 0,
          optionality: 10,
          surplus: 0,
          evolution: 0,
          total: 0,
        },
        issues: [
          {
            id: this.generateIssueId("error", 0),
            category: "error",
            severity: "critical",
            description: error instanceof Error ? error.message : String(error),
          },
        ],
        recommendation: "rework",
        checks,
        durationMs,
      }
    }
  }

  async fix(
    issues: PDCAIssue[],
    context: TaskExecutionResult<ImplementationOutput>,
    config: PDCAConfig,
  ): Promise<PDCAActResult> {
    const startTime = Date.now()

    log.debug("Running implementation fix loop", {
      sessionId: config.sessionId,
      issueCount: issues.length,
    })

    // Use existing FixLoop for comprehensive fixing
    const { createFixLoop } = await import("../../execution/fix-loop")

    const fixLoop = createFixLoop({
      maxAttemptsPerIssue: 3,
      enableAutoFix: true,
      enableAgentFix: true,
      enableLLMGeneration: true,
      enableEvolutionFallback: true,
      enableLearning: config.enableLearning,
      verifyAfterEachFix: true,
    })

    // Convert PDCA issues to AcceptanceIssue format
    const acceptanceIssues = issues.map((issue) => ({
      id: issue.id,
      type: issue.category as "test" | "type" | "lint" | "security" | "requirement" | "expectation",
      severity: issue.severity,
      description: issue.description,
      location: issue.location,
      suggestedFix: issue.suggestedAction,
    }))

    try {
      const fixResult = await fixLoop.fix({
        sessionId: config.sessionId,
        issues: acceptanceIssues,
        workingDir: context.metadata?.workingDir as string | undefined,
      })

      const durationMs = Date.now() - startTime

      return {
        fixed: fixResult.success,
        fixedIssues: fixResult.fixedIssues,
        remainingIssues: fixResult.remainingIssues.map((i) => ({
          id: i.id,
          category: i.type,
          severity: i.severity,
          description: i.description,
          location: i.location,
          suggestedAction: i.suggestedFix,
        })),
        shouldRecheck: fixResult.shouldRecheck,
        attempts: fixResult.attempts.length,
        durationMs,
        learnedPatterns: fixResult.learnedPatterns,
      }
    } catch (error) {
      log.error("Implementation fix failed", { error })

      const durationMs = Date.now() - startTime
      return {
        fixed: false,
        fixedIssues: [],
        remainingIssues: issues,
        shouldRecheck: false,
        attempts: 0,
        durationMs,
      }
    }
  }
}

/** Factory function */
export function createImplementationStrategy(): ImplementationStrategy {
  return new ImplementationStrategy()
}
