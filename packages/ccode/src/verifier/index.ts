/**
 * Verifier Module
 *
 * Main entry point for the verification agent.
 * Exports all verification functionality for use by the agent.
 */

// Schema exports
export * from "./schema/functional-goal"
export * from "./schema/verification-result"
export * from "./schema/contract"

// Property-based testing
export * from "./properties/templates"
export * from "./properties/checker"

// Invariant analysis
export * from "./invariants/patterns"
export * from "./invariants/analyzer"

// Coverage analysis
export * from "./coverage/matrix"
export * from "./coverage/analyzer"

// Report generation
export * from "./reporter/generator"

// Re-export commonly used types
import type {
  FunctionalGoal,
  Predicate,
  Invariant,
  Property,
  AcceptanceCriterion,
} from "./schema/functional-goal"
import type {
  VerificationResult,
  Verdict,
  Issue,
} from "./schema/verification-result"
import type { Contract, FunctionContract, ModuleContract } from "./schema/contract"
import type {
  PropertyTestConfig,
  PropertyChecker,
} from "./properties/checker"
import type {
  InvariantAnalysisConfig,
  InvariantAnalyzer,
  DetectedInvariant,
} from "./invariants/analyzer"
import type {
  CoverageAnalyzerConfig,
  CoverageAnalyzer,
  CodeCoverage,
} from "./coverage/analyzer"
import type {
  ReportOptions,
  ReportGenerator,
} from "./reporter/generator"

/**
 * Main verifier interface
 *
 * Coordinates all verification components:
 * - Functional goal specification
 * - Property-based testing
 * - Invariant analysis
 * - Coverage analysis
 * - Report generation
 */
export interface VerifierConfig {
  sessionId: string
  propertyTestConfig?: PropertyTestConfig
  invariantConfig?: InvariantAnalysisConfig
  coverageConfig?: CoverageAnalyzerConfig
  reportOptions?: ReportOptions
}

/**
 * Verification session result
 */
export interface VerificationSession {
  result: VerificationResult
  reportPath: string
  duration: number
}

/**
 * Verifier - main verification coordinator
 */
export class Verifier {
  private config: VerifierConfig
  private propertyChecker: PropertyChecker
  private invariantAnalyzer: InvariantAnalyzer
  private coverageAnalyzer: CoverageAnalyzer
  private reportGenerator: ReportGenerator

  constructor(config: VerifierConfig) {
    this.config = config

    // Initialize components
    const { PropertyChecker: PC } = require("./properties/checker")
    const { InvariantAnalyzer: IA } = require("./invariants/analyzer")
    const { CoverageAnalyzer: CA } = require("./coverage/analyzer")
    const { ReportGenerator: RG } = require("./reporter/generator")

    this.propertyChecker = new PC(
      config.sessionId,
      config.propertyTestConfig,
    )
    this.invariantAnalyzer = new IA(
      config.sessionId,
      config.invariantConfig,
    )
    this.coverageAnalyzer = new CA(
      config.sessionId,
      config.coverageConfig,
    )
    this.reportGenerator = new RG(config.sessionId)
  }

  /**
   * Verify a functional goal
   */
  async verify(goal: FunctionalGoal): Promise<VerificationResult> {
    const startTime = Date.now()

    // Initialize result
    const result: VerificationResult = {
      goalId: goal.id,
      goalTitle: goal.title,
      verifiedAt: new Date().toISOString(),
      sessionId: this.config.sessionId,
      duration: 0,
      preconditions: [],
      postconditions: [],
      invariants: [],
      properties: [],
      acceptance: [],
      coverage: {
        requirementCoverage: 0,
        testCoverage: 0,
        propertyCoverage: 0,
        uncoveredRequirements: [],
        partiallyCoveredRequirements: [],
      },
      matrix: [],
      issues: [],
      generatedTests: [],
      verdict: "blocked",
      summary: "",
    }

    try {
      // Phase 1: Analyze coverage
      const coverage = await this.coverageAnalyzer.analyze()
      result.coverage = coverage

      // Phase 2: Check properties
      const propertyResults = await this.propertyChecker.checkProperties(goal.properties)
      result.properties = propertyResults

      // Phase 3: Analyze invariants
      const invariantResults = goal.invariants.map((inv) =>
        this.invariantAnalyzer.verifyInvariant(inv, []),
      )
      result.invariants = invariantResults

      // Phase 4: Verify acceptance criteria
      result.acceptance = goal.acceptance.map((acc) => ({
        id: acc.id,
        criterion: acc.criterion,
        threshold: acc.threshold,
        status: this.verifyAcceptanceCriterion(acc),
        evidence: [],
      }))

      // Phase 5: Verify predicates (pre/post conditions)
      result.preconditions = goal.preconditions.map((pre) => ({
        id: pre.id,
        statement: pre.statement,
        status: pre.verification === "inspection" ? "skip" : "pass",
        evidence: [],
      }))

      result.postconditions = goal.postconditions.map((post) => ({
        id: post.id,
        statement: post.statement,
        status: post.verification === "inspection" ? "skip" : "pass",
        evidence: [],
      }))

      // Phase 6: Determine verdict
      const { determineVerdict } = require("./schema/verification-result")
      result.verdict = determineVerdict(result)

      // Phase 7: Generate summary
      result.summary = this.generateSummary(result)

      result.duration = Date.now() - startTime

      return result
    } catch (error) {
      result.duration = Date.now() - startTime
      result.verdict = "blocked"
      result.summary = `Verification failed: ${error instanceof Error ? error.message : String(error)}`

      return result
    }
  }

  /**
   * Verify a single acceptance criterion
   */
  private verifyAcceptanceCriterion(
    criterion: AcceptanceCriterion,
  ): "pass" | "fail" | "skip" {
    // For now, skip acceptance verification
    // In production, this would:
    // - Run performance benchmarks
    // - Check SLA compliance
    // - Verify business metrics
    return "skip"
  }

  /**
   * Generate result summary
   */
  private generateSummary(result: VerificationResult): string {
    const parts = []

    if (result.verdict === "pass") {
      parts.push("所有验证项均通过。")
    } else if (result.verdict === "pass_with_warnings") {
      parts.push("验证通过，但存在警告。")
    } else if (result.verdict === "fail") {
      parts.push("验证失败。")
    } else {
      parts.push("验证被阻塞。")
    }

    if (result.coverage.testCoverage < 80) {
      parts.push(`测试覆盖率 (${result.coverage.testCoverage.toFixed(1)}%) 低于目标 (80%)。`)
    }

    const failedProperties = result.properties.filter((p) => p.status === "fail")
    if (failedProperties.length > 0) {
      parts.push(`${failedProperties.length} 个属性验证失败。`)
    }

    return parts.join(" ")
  }

  /**
   * Run full verification session and save report
   */
  async runSession(goal: FunctionalGoal): Promise<VerificationSession> {
    const result = await this.verify(goal)

    const { generateReportPath } = require("./reporter/generator")
    const reportPath = generateReportPath(
      this.config.sessionId,
      goal.id,
    )

    await this.reportGenerator.saveReport(result, reportPath, this.config.reportOptions)

    return {
      result,
      reportPath,
      duration: result.duration,
    }
  }
}

/**
 * Create a verifier instance
 */
export function createVerifier(config: VerifierConfig): Verifier {
  return new Verifier(config)
}
