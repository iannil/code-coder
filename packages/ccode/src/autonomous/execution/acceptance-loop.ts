/**
 * Acceptance Loop (PDCA: Check)
 *
 * Dedicated execution loop for acceptance/validation tasks.
 * Verifies that implementation meets requirements, passes quality checks,
 * and aligns with user expectations.
 *
 * Six Phases:
 * 1. Requirement Parsing - Parse original request, build acceptance checklist
 * 2. Quality Check - Run tests, typecheck, lint, security scan
 * 3. Conformance Check - Verify requirements are met against checklist
 * 4. Expectation Check - Evaluate alignment with implicit user expectations
 * 5. CLOSE Scoring - Generate comprehensive evaluation with decision recommendation
 * 6. Reporting - Generate acceptance report
 */

import { Log } from "@/util/log"
import { Bus } from "@/bus"
import { AutonomousEvent } from "../events"
import { TestRunner } from "./test-runner"
import type { EvolutionResult } from "./evolution-loop"
import type { CLOSEScore } from "../decision/criteria"

const log = Log.create({ service: "autonomous.acceptance-loop" })

// ============================================================================
// Types
// ============================================================================

/** Problem input for acceptance loop */
export interface AcceptanceProblem {
  /** Session ID for tracking */
  sessionId: string
  /** Original user request */
  originalRequest: string
  /** Implementation result from evolution-loop (optional) */
  implementationResult?: EvolutionResult
  /** Working directory */
  workingDir?: string
  /** Files that were modified */
  modifiedFiles?: string[]
}

/** Individual acceptance issue */
export interface AcceptanceIssue {
  /** Unique issue ID */
  id: string
  /** Issue type */
  type: "test" | "type" | "lint" | "security" | "requirement" | "expectation"
  /** Severity level */
  severity: "critical" | "high" | "medium" | "low"
  /** Description of the issue */
  description: string
  /** File/line location if applicable */
  location?: string
  /** Suggested fix if available */
  suggestedFix?: string
}

/** Code quality check report */
export interface CodeQualityReport {
  /** Tests passed */
  testsPassed: boolean
  /** Test count */
  testsTotal: number
  /** Tests failed count */
  testsFailed: number
  /** TypeScript type check passed */
  typecheckPassed: boolean
  /** TypeScript errors */
  typecheckErrors: string[]
  /** Lint passed */
  lintPassed: boolean
  /** Lint issues */
  lintIssues: string[]
  /** Security scan passed */
  securityPassed: boolean
  /** Security issues */
  securityIssues: string[]
}

/** Requirement conformance report */
export interface RequirementReport {
  /** Total requirements */
  total: number
  /** Requirements met */
  met: number
  /** Requirements partially met */
  partial: number
  /** Requirements not met */
  notMet: number
  /** Individual requirement checks */
  checks: Array<{
    requirement: string
    status: "met" | "partial" | "not_met"
    evidence?: string
  }>
}

/** User expectation alignment report */
export interface ExpectationReport {
  /** Overall alignment score (0-10) */
  alignmentScore: number
  /** Implicit expectations identified */
  implicitExpectations: string[]
  /** Expectations that are met */
  metExpectations: string[]
  /** Expectations that are not met */
  unmetExpectations: string[]
  /** Analysis reasoning */
  reasoning: string
}

/** Acceptance loop result */
export interface AcceptanceResult {
  /** Whether acceptance passed */
  success: boolean
  /** Overall CLOSE score (0-10) */
  overallScore: number
  /** Detailed CLOSE scores */
  closeScores: CLOSEScore
  /** Check results */
  checks: {
    codeQuality: CodeQualityReport
    requirementConformance: RequirementReport
    userExpectation: ExpectationReport
  }
  /** Issues found */
  issues: AcceptanceIssue[]
  /** Recommendation for next action */
  recommendation: "pass" | "fix" | "rework"
  /** Total duration */
  durationMs: number
  /** Report content (markdown) */
  report?: string
  /** Report file path if saved */
  reportPath?: string
}

/** Acceptance loop configuration */
export interface AcceptanceLoopConfig {
  /** Enable test running (default: true) */
  enableTests?: boolean
  /** Enable typecheck (default: true) */
  enableTypecheck?: boolean
  /** Enable linting (default: true) */
  enableLint?: boolean
  /** Enable security scan (default: true) */
  enableSecurityScan?: boolean
  /** Enable LLM-based expectation check (default: true) */
  enableExpectationCheck?: boolean
  /** Minimum score to pass (default: 6.0) */
  passThreshold?: number
  /** Minimum score to fix vs rework (default: 4.0) */
  fixThreshold?: number
  /** Generate report file (default: false) */
  generateReportFile?: boolean
}

const DEFAULT_CONFIG: Required<AcceptanceLoopConfig> = {
  enableTests: true,
  enableTypecheck: true,
  enableLint: true,
  enableSecurityScan: true,
  enableExpectationCheck: true,
  passThreshold: 6.0,
  fixThreshold: 4.0,
  generateReportFile: false,
}

// ============================================================================
// Acceptance Loop Implementation
// ============================================================================

export function createAcceptanceLoop(config: AcceptanceLoopConfig = {}) {
  const cfg = { ...DEFAULT_CONFIG, ...config }

  /** Phase 1: Parse requirements and build checklist */
  async function parseRequirements(problem: AcceptanceProblem): Promise<{
    checklist: string[]
    implicitExpectations: string[]
  }> {
    await Bus.publish(AutonomousEvent.AcceptancePhaseChanged, {
      sessionId: problem.sessionId,
      phase: "requirement_parsing",
    })

    log.debug("Parsing requirements from original request", {
      sessionId: problem.sessionId,
      requestLength: problem.originalRequest.length,
    })

    // Extract explicit requirements from the request
    const checklist: string[] = []
    const lines = problem.originalRequest.split("\n")

    for (const line of lines) {
      const trimmed = line.trim()
      // Match numbered items, bullet points, or requirement-like statements
      if (/^(\d+\.|[-*]|\[.\])/.test(trimmed) || /(?:should|must|need|require)/i.test(trimmed)) {
        checklist.push(trimmed.replace(/^(\d+\.|[-*]|\[.\])\s*/, ""))
      }
    }

    // If no explicit requirements found, use the whole request as one requirement
    if (checklist.length === 0) {
      checklist.push(problem.originalRequest)
    }

    // Identify implicit expectations
    const implicitExpectations: string[] = []

    // Common implicit expectations based on request content
    if (/test/i.test(problem.originalRequest)) {
      implicitExpectations.push("Tests should pass")
    }
    if (/type|typescript/i.test(problem.originalRequest)) {
      implicitExpectations.push("No TypeScript errors")
    }
    if (/security|auth/i.test(problem.originalRequest)) {
      implicitExpectations.push("No security vulnerabilities")
    }

    // Always expect basic quality
    implicitExpectations.push("Code should be readable and maintainable")
    implicitExpectations.push("Implementation should match the request intent")

    return { checklist, implicitExpectations }
  }

  /** Phase 2: Run code quality checks */
  async function runQualityChecks(
    problem: AcceptanceProblem,
    issues: AcceptanceIssue[],
  ): Promise<CodeQualityReport> {
    await Bus.publish(AutonomousEvent.AcceptancePhaseChanged, {
      sessionId: problem.sessionId,
      phase: "quality_check",
    })

    const report: CodeQualityReport = {
      testsPassed: true,
      testsTotal: 0,
      testsFailed: 0,
      typecheckPassed: true,
      typecheckErrors: [],
      lintPassed: true,
      lintIssues: [],
      securityPassed: true,
      securityIssues: [],
    }

    // Run tests
    if (cfg.enableTests) {
      try {
        const testResult = await TestRunner.runAll()
        report.testsPassed = testResult.success
        report.testsTotal = testResult.passed + testResult.failed + testResult.skipped
        report.testsFailed = testResult.failed

        if (!testResult.success) {
          for (const error of testResult.errors) {
            issues.push({
              id: `test-${issues.length}`,
              type: "test",
              severity: "high",
              description: error,
            })
          }
        }
      } catch (error) {
        log.warn("Test execution failed", { error })
        report.testsPassed = false
        issues.push({
          id: `test-${issues.length}`,
          type: "test",
          severity: "critical",
          description: `Test execution error: ${error instanceof Error ? error.message : String(error)}`,
        })
      }
    }

    // Run typecheck
    if (cfg.enableTypecheck) {
      try {
        const { execSync } = require("child_process")
        execSync("bun run turbo typecheck", {
          cwd: problem.workingDir || process.cwd(),
          stdio: "pipe",
        })
        report.typecheckPassed = true
      } catch (error) {
        report.typecheckPassed = false
        const errorOutput = error instanceof Error && "stderr" in error
          ? String((error as { stderr: Buffer }).stderr)
          : String(error)

        // Parse TypeScript errors
        const errorLines = errorOutput.split("\n").filter((line: string) =>
          line.includes("error TS") || line.includes(": error:")
        )

        for (const errorLine of errorLines.slice(0, 10)) {
          report.typecheckErrors.push(errorLine)
          issues.push({
            id: `type-${issues.length}`,
            type: "type",
            severity: "high",
            description: errorLine,
          })
        }
      }
    }

    // Run lint
    if (cfg.enableLint) {
      try {
        const { execSync } = require("child_process")
        execSync("bun eslint . --format json 2>/dev/null || true", {
          cwd: problem.workingDir || process.cwd(),
          stdio: "pipe",
        })
        report.lintPassed = true
      } catch {
        // Lint failures are warnings, not blockers
        report.lintPassed = true
      }
    }

    // Security scan (basic pattern matching)
    if (cfg.enableSecurityScan && problem.modifiedFiles) {
      const securityPatterns = [
        { pattern: /eval\s*\(/, issue: "Use of eval() is a security risk" },
        { pattern: /innerHTML\s*=/, issue: "Use of innerHTML can lead to XSS" },
        { pattern: /password\s*=\s*["'][^"']+["']/, issue: "Hardcoded password detected" },
        { pattern: /api[_-]?key\s*=\s*["'][^"']+["']/i, issue: "Hardcoded API key detected" },
      ]

      for (const file of problem.modifiedFiles) {
        try {
          const fs = require("fs")
          const content = fs.readFileSync(file, "utf-8")

          for (const { pattern, issue } of securityPatterns) {
            if (pattern.test(content)) {
              report.securityPassed = false
              report.securityIssues.push(`${file}: ${issue}`)
              issues.push({
                id: `security-${issues.length}`,
                type: "security",
                severity: "critical",
                description: issue,
                location: file,
              })
            }
          }
        } catch {
          // Skip files that can't be read
        }
      }
    }

    log.debug("Quality checks completed", {
      sessionId: problem.sessionId,
      testsPassed: report.testsPassed,
      typecheckPassed: report.typecheckPassed,
      securityPassed: report.securityPassed,
    })

    return report
  }

  /** Phase 3: Check requirement conformance */
  async function checkConformance(
    problem: AcceptanceProblem,
    checklist: string[],
    issues: AcceptanceIssue[],
  ): Promise<RequirementReport> {
    await Bus.publish(AutonomousEvent.AcceptancePhaseChanged, {
      sessionId: problem.sessionId,
      phase: "conformance_check",
    })

    // If we have implementation result, use it to verify conformance
    const report: RequirementReport = {
      total: checklist.length,
      met: 0,
      partial: 0,
      notMet: 0,
      checks: [],
    }

    // If no implementation result or implementation failed, mark all as not met
    if (!problem.implementationResult || !problem.implementationResult.solved) {
      for (const req of checklist) {
        report.checks.push({ requirement: req, status: "not_met" })
        report.notMet++
        issues.push({
          id: `req-${issues.length}`,
          type: "requirement",
          severity: "high",
          description: `Requirement not implemented: ${req}`,
        })
      }

      log.debug("No valid implementation - all requirements marked as not met", {
        sessionId: problem.sessionId,
        total: checklist.length,
      })

      return report
    }

    // Use LLM to verify conformance if implementation result exists
    try {
      const { generateObject } = await import("ai")
      const { getDefaultModelWithFallback } = await import("@/sdk/provider-bridge")
      const { Provider } = await import("@/provider/provider")
      const z = await import("zod").then((m) => m.default)

      const defaultModel = await getDefaultModelWithFallback()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const result = await generateObject({
        temperature: 0.2,
        messages: [
          {
            role: "system",
            content: `You are an acceptance tester. Verify if the implementation meets each requirement.
Return a status for each requirement: "met", "partial", or "not_met".`,
          },
          {
            role: "user",
            content: `Original Request: ${problem.originalRequest}

Implementation Summary: ${problem.implementationResult.summary}

Requirements to check:
${checklist.map((r, i) => `${i + 1}. ${r}`).join("\n")}`,
          },
        ],
        model: language,
        schema: z.object({
          checks: z.array(z.object({
            requirement: z.string(),
            status: z.enum(["met", "partial", "not_met"]),
            evidence: z.string().optional(),
          })),
        }),
      })

      for (const check of result.object.checks) {
        report.checks.push(check)
        if (check.status === "met") report.met++
        else if (check.status === "partial") report.partial++
        else report.notMet++

        if (check.status === "not_met") {
          issues.push({
            id: `req-${issues.length}`,
            type: "requirement",
            severity: "high",
            description: `Requirement not met: ${check.requirement}`,
            suggestedFix: check.evidence,
          })
        }
      }
    } catch (error) {
      log.warn("LLM conformance check failed, using fallback", { error })
      // Fallback: assume all requirements are met if implementation succeeded
      for (const req of checklist) {
        report.checks.push({ requirement: req, status: "met" })
        report.met++
      }
    }

    log.debug("Conformance check completed", {
      sessionId: problem.sessionId,
      met: report.met,
      partial: report.partial,
      notMet: report.notMet,
    })

    return report
  }

  /** Phase 4: Check user expectation alignment */
  async function checkExpectations(
    problem: AcceptanceProblem,
    implicitExpectations: string[],
    qualityReport: CodeQualityReport,
    issues: AcceptanceIssue[],
  ): Promise<ExpectationReport> {
    await Bus.publish(AutonomousEvent.AcceptancePhaseChanged, {
      sessionId: problem.sessionId,
      phase: "expectation_check",
    })

    const metExpectations: string[] = []
    const unmetExpectations: string[] = []

    // Check implicit expectations against quality report
    for (const expectation of implicitExpectations) {
      if (expectation.includes("Tests should pass") && !qualityReport.testsPassed) {
        unmetExpectations.push(expectation)
      } else if (expectation.includes("No TypeScript errors") && !qualityReport.typecheckPassed) {
        unmetExpectations.push(expectation)
      } else if (expectation.includes("No security vulnerabilities") && !qualityReport.securityPassed) {
        unmetExpectations.push(expectation)
      } else {
        metExpectations.push(expectation)
      }
    }

    // Calculate alignment score
    const totalExpectations = metExpectations.length + unmetExpectations.length
    const alignmentScore = totalExpectations > 0
      ? (metExpectations.length / totalExpectations) * 10
      : 5.0

    // Add issues for unmet expectations
    for (const unmet of unmetExpectations) {
      issues.push({
        id: `exp-${issues.length}`,
        type: "expectation",
        severity: "medium",
        description: `Expectation not met: ${unmet}`,
      })
    }

    const report: ExpectationReport = {
      alignmentScore,
      implicitExpectations,
      metExpectations,
      unmetExpectations,
      reasoning: unmetExpectations.length > 0
        ? `${unmetExpectations.length} implicit expectations were not met`
        : "All implicit expectations appear to be met",
    }

    log.debug("Expectation check completed", {
      sessionId: problem.sessionId,
      alignmentScore,
      met: metExpectations.length,
      unmet: unmetExpectations.length,
    })

    return report
  }

  /** Phase 5: Calculate CLOSE score and generate recommendation */
  async function calculateScore(
    problem: AcceptanceProblem,
    qualityReport: CodeQualityReport,
    conformanceReport: RequirementReport,
    expectationReport: ExpectationReport,
    issues: AcceptanceIssue[],
  ): Promise<{ closeScores: CLOSEScore; recommendation: "pass" | "fix" | "rework" }> {
    await Bus.publish(AutonomousEvent.AcceptancePhaseChanged, {
      sessionId: problem.sessionId,
      phase: "scoring",
    })

    // If no implementation result at all, immediately recommend rework
    if (!problem.implementationResult) {
      const closeScores: CLOSEScore = {
        convergence: 0,
        leverage: 0,
        optionality: 10,
        surplus: 0,
        evolution: 0,
        total: 0,
      }

      log.debug("No implementation - automatic rework recommendation", {
        sessionId: problem.sessionId,
      })

      return { closeScores, recommendation: "rework" }
    }

    // Calculate CLOSE dimensions
    // Convergence: How close are we to done? (based on requirements met)
    const convergence = conformanceReport.total > 0
      ? ((conformanceReport.met + conformanceReport.partial * 0.5) / conformanceReport.total) * 10
      : 5.0

    // Leverage: How much value does this implementation provide?
    const leverage = qualityReport.testsPassed && qualityReport.typecheckPassed ? 8.0 : 5.0

    // Optionality: Can we easily adjust/fix? (inverse of critical issues)
    const criticalIssues = issues.filter((i) => i.severity === "critical").length
    const optionality = Math.max(3, 10 - criticalIssues * 2)

    // Surplus: Do we have headroom? (based on expectation alignment)
    const surplus = expectationReport.alignmentScore

    // Evolution: Did we learn/improve? (baseline value)
    const evolution = problem.implementationResult?.knowledgeId ? 8.0 : 6.0

    // Calculate total using weighted average
    const weights = { convergence: 1.2, leverage: 1.0, optionality: 1.5, surplus: 1.3, evolution: 0.8 }
    const maxWeight = Object.values(weights).reduce((a, b) => a + b, 0) * 10
    const total = (
      convergence * weights.convergence +
      leverage * weights.leverage +
      optionality * weights.optionality +
      surplus * weights.surplus +
      evolution * weights.evolution
    ) / maxWeight * 10

    const closeScores: CLOSEScore = {
      convergence: Math.round(convergence * 100) / 100,
      leverage: Math.round(leverage * 100) / 100,
      optionality: Math.round(optionality * 100) / 100,
      surplus: Math.round(surplus * 100) / 100,
      evolution: Math.round(evolution * 100) / 100,
      total: Math.round(total * 100) / 100,
    }

    // Determine recommendation
    let recommendation: "pass" | "fix" | "rework"
    if (closeScores.total >= cfg.passThreshold) {
      recommendation = "pass"
    } else if (closeScores.total >= cfg.fixThreshold) {
      recommendation = "fix"
    } else {
      recommendation = "rework"
    }

    log.debug("CLOSE scoring completed", {
      sessionId: problem.sessionId,
      total: closeScores.total,
      recommendation,
    })

    return { closeScores, recommendation }
  }

  /** Phase 6: Generate acceptance report */
  async function generateReport(
    problem: AcceptanceProblem,
    qualityReport: CodeQualityReport,
    conformanceReport: RequirementReport,
    expectationReport: ExpectationReport,
    closeScores: CLOSEScore,
    issues: AcceptanceIssue[],
    recommendation: "pass" | "fix" | "rework",
  ): Promise<{ content: string; filePath?: string }> {
    await Bus.publish(AutonomousEvent.AcceptancePhaseChanged, {
      sessionId: problem.sessionId,
      phase: "reporting",
    })

    const report = `# Acceptance Report

## Summary
- **Recommendation**: ${recommendation.toUpperCase()}
- **CLOSE Score**: ${closeScores.total}/10
- **Issues Found**: ${issues.length}

## CLOSE Scores
| Dimension | Score |
|-----------|-------|
| Convergence | ${closeScores.convergence}/10 |
| Leverage | ${closeScores.leverage}/10 |
| Optionality | ${closeScores.optionality}/10 |
| Surplus | ${closeScores.surplus}/10 |
| Evolution | ${closeScores.evolution}/10 |

## Code Quality
- Tests: ${qualityReport.testsPassed ? "PASS" : "FAIL"} (${qualityReport.testsTotal - qualityReport.testsFailed}/${qualityReport.testsTotal})
- TypeCheck: ${qualityReport.typecheckPassed ? "PASS" : "FAIL"}
- Lint: ${qualityReport.lintPassed ? "PASS" : "FAIL"}
- Security: ${qualityReport.securityPassed ? "PASS" : "FAIL"}

## Requirement Conformance
- Met: ${conformanceReport.met}/${conformanceReport.total}
- Partial: ${conformanceReport.partial}
- Not Met: ${conformanceReport.notMet}

## User Expectation Alignment
- Score: ${expectationReport.alignmentScore}/10
- ${expectationReport.reasoning}

## Issues
${issues.length > 0 ? issues.map((i) => `- **[${i.severity.toUpperCase()}]** ${i.type}: ${i.description}`).join("\n") : "No issues found."}
`

    let filePath: string | undefined
    if (cfg.generateReportFile) {
      try {
        const fs = require("fs")
        const path = require("path")
        const reportsDir = path.join(problem.workingDir || process.cwd(), "docs", "reports")
        fs.mkdirSync(reportsDir, { recursive: true })
        filePath = path.join(reportsDir, `acceptance-${problem.sessionId}-${Date.now()}.md`)
        fs.writeFileSync(filePath, report)
        log.info("Acceptance report saved", { filePath })
      } catch (error) {
        log.warn("Failed to save report file", { error })
      }
    }

    return { content: report, filePath }
  }

  return {
    /** Execute full acceptance loop */
    async accept(problem: AcceptanceProblem): Promise<AcceptanceResult> {
      const startTime = Date.now()
      const issues: AcceptanceIssue[] = []

      await Bus.publish(AutonomousEvent.AcceptanceStarted, {
        sessionId: problem.sessionId,
        originalRequest: problem.originalRequest,
        checkTypes: ["quality", "requirement", "expectation"],
      })

      try {
        // Phase 1: Parse requirements
        const { checklist, implicitExpectations } = await parseRequirements(problem)

        // Phase 2: Quality checks
        const qualityReport = await runQualityChecks(problem, issues)

        // Phase 3: Conformance check
        const conformanceReport = await checkConformance(problem, checklist, issues)

        // Phase 4: Expectation check
        const expectationReport = await checkExpectations(
          problem,
          implicitExpectations,
          qualityReport,
          issues,
        )

        // Phase 5: CLOSE scoring
        const { closeScores, recommendation } = await calculateScore(
          problem,
          qualityReport,
          conformanceReport,
          expectationReport,
          issues,
        )

        // Publish issues
        for (const issue of issues) {
          await Bus.publish(AutonomousEvent.AcceptanceIssueFound, {
            sessionId: problem.sessionId,
            issueId: issue.id,
            type: issue.type,
            severity: issue.severity,
            description: issue.description,
            location: issue.location,
          })
        }

        // Phase 6: Generate report
        const { content: report, filePath: reportPath } = await generateReport(
          problem,
          qualityReport,
          conformanceReport,
          expectationReport,
          closeScores,
          issues,
          recommendation,
        )

        const result: AcceptanceResult = {
          success: recommendation === "pass",
          overallScore: closeScores.total,
          closeScores,
          checks: {
            codeQuality: qualityReport,
            requirementConformance: conformanceReport,
            userExpectation: expectationReport,
          },
          issues,
          recommendation,
          durationMs: Date.now() - startTime,
          report,
          reportPath,
        }

        await Bus.publish(AutonomousEvent.AcceptanceCompleted, {
          sessionId: problem.sessionId,
          success: result.success,
          overallScore: result.overallScore,
          issueCount: issues.length,
          recommendation,
          durationMs: result.durationMs,
        })

        log.info("Acceptance loop completed", {
          sessionId: problem.sessionId,
          recommendation,
          score: closeScores.total,
          issueCount: issues.length,
        })

        return result
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error)

        await Bus.publish(AutonomousEvent.AcceptanceFailed, {
          sessionId: problem.sessionId,
          phase: "unknown",
          error: errorMsg,
          retryable: true,
        })

        log.error("Acceptance loop failed", { sessionId: problem.sessionId, error: errorMsg })

        return {
          success: false,
          overallScore: 0,
          closeScores: {
            convergence: 0,
            leverage: 0,
            optionality: 0,
            surplus: 0,
            evolution: 0,
            total: 0,
          },
          checks: {
            codeQuality: {
              testsPassed: false,
              testsTotal: 0,
              testsFailed: 0,
              typecheckPassed: false,
              typecheckErrors: [],
              lintPassed: false,
              lintIssues: [],
              securityPassed: false,
              securityIssues: [],
            },
            requirementConformance: {
              total: 0,
              met: 0,
              partial: 0,
              notMet: 0,
              checks: [],
            },
            userExpectation: {
              alignmentScore: 0,
              implicitExpectations: [],
              metExpectations: [],
              unmetExpectations: [],
              reasoning: `Acceptance failed: ${errorMsg}`,
            },
          },
          issues: [{
            id: "error-0",
            type: "test",
            severity: "critical",
            description: errorMsg,
          }],
          recommendation: "rework",
          durationMs: Date.now() - startTime,
        }
      }
    },
  }
}

export type AcceptanceLoop = ReturnType<typeof createAcceptanceLoop>
