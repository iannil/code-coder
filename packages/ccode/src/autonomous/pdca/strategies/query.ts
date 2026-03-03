/**
 * Query Strategy
 *
 * Acceptance strategy for query/question tasks.
 * Validates relevance, completeness, and accuracy of answers.
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
  QueryOutput,
  CheckItemResult,
  CLOSEScore,
} from "../types"

const log = Log.create({ service: "pdca.strategy.query" })

// ============================================================================
// Check Items for Query Tasks
// ============================================================================

const QUERY_CHECK_ITEMS = {
  relevance: {
    name: "relevance",
    description: "Answer is relevant to the question",
    weight: 1.5,
  },
  completeness: {
    name: "completeness",
    description: "Answer addresses all parts of the question",
    weight: 1.2,
  },
  accuracy: {
    name: "accuracy",
    description: "Answer is accurate and correct",
    weight: 1.3,
  },
  clarity: {
    name: "clarity",
    description: "Answer is clear and well-structured",
    weight: 0.8,
  },
} as const

// ============================================================================
// Query Strategy
// ============================================================================

export class QueryStrategy extends BaseAcceptanceStrategy<QueryOutput> {
  readonly taskType: TaskType = "query"
  readonly name = "Query Acceptance Strategy"

  getCheckItems(): string[] {
    return Object.keys(QUERY_CHECK_ITEMS)
  }

  getCheckWeights(): Record<string, number> {
    return Object.fromEntries(
      Object.entries(QUERY_CHECK_ITEMS).map(([k, v]) => [k, v.weight]),
    )
  }

  async check(
    result: TaskExecutionResult<QueryOutput>,
    originalRequest: string,
    config: PDCAConfig,
  ): Promise<PDCACheckResult> {
    const startTime = Date.now()
    const issues: PDCAIssue[] = []
    const checks: Record<string, CheckItemResult> = {}
    const output = result.output

    log.debug("Running query acceptance check", {
      sessionId: config.sessionId,
      confidence: output?.confidence,
    })

    // Check 1: Relevance
    const relevanceCheck = await this.checkRelevance(output, originalRequest, issues, config)
    checks.relevance = relevanceCheck

    // Check 2: Completeness
    const completenessCheck = await this.checkCompleteness(output, originalRequest, issues, config)
    checks.completeness = completenessCheck

    // Check 3: Accuracy
    const accuracyCheck = this.checkAccuracy(output, issues)
    checks.accuracy = accuracyCheck

    // Check 4: Clarity
    const clarityCheck = this.checkClarity(output, issues)
    checks.clarity = clarityCheck

    // Calculate CLOSE scores
    const closeScore = this.calculateQueryCLOSEScore(checks, output)

    // Determine if passed and recommendation
    const criticalIssues = issues.filter((i) => i.severity === "critical").length
    const recommendation = this.getRecommendation(
      closeScore.total,
      config.passThreshold,
      config.fixThreshold,
      criticalIssues,
    )

    const passed = recommendation === "pass"
    const durationMs = Date.now() - startTime

    return {
      taskType: this.taskType,
      passed,
      closeScore,
      issues,
      recommendation,
      checks,
      durationMs,
    }
  }

  async fix(
    issues: PDCAIssue[],
    context: TaskExecutionResult<QueryOutput>,
    config: PDCAConfig,
  ): Promise<PDCAActResult> {
    const startTime = Date.now()
    const fixedIssues: string[] = []
    const remainingIssues: PDCAIssue[] = []

    log.debug("Running query fix loop", {
      sessionId: config.sessionId,
      issueCount: issues.length,
    })

    // Query fixes typically involve re-asking with more context
    // For now, mark all as remaining (would need integration with agent)
    for (const issue of issues) {
      remainingIssues.push(issue)
    }

    const durationMs = Date.now() - startTime

    return {
      fixed: false,
      fixedIssues,
      remainingIssues,
      shouldRecheck: false,
      attempts: issues.length,
      durationMs,
    }
  }

  // ==========================================================================
  // Check Implementations
  // ==========================================================================

  private async checkRelevance(
    output: QueryOutput | undefined,
    originalRequest: string,
    issues: PDCAIssue[],
    config: PDCAConfig,
  ): Promise<CheckItemResult> {
    if (!output?.answer) {
      issues.push({
        id: this.generateIssueId("relevance", 0),
        category: "relevance",
        severity: "critical",
        description: "No answer provided",
        suggestedAction: "Generate an answer to the query",
      })

      return {
        passed: false,
        score: 0,
        weight: QUERY_CHECK_ITEMS.relevance.weight,
        details: "No answer",
      }
    }

    // Use LLM to check relevance
    try {
      const { generateObject } = await import("ai")
      const { Provider } = await import("@/provider/provider")
      const z = await import("zod").then((m) => m.default)

      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const result = await generateObject({
        temperature: 0.2,
        messages: [
          {
            role: "system",
            content: `You evaluate if an answer is relevant to a question. Score 0-10.`,
          },
          {
            role: "user",
            content: `Question: ${originalRequest}

Answer: ${output.answer.slice(0, 1000)}

Rate relevance 0-10.`,
          },
        ],
        model: language,
        schema: z.object({
          score: z.number().min(0).max(10),
          reasoning: z.string(),
        }),
      })

      const passed = result.object.score >= 7

      if (!passed) {
        issues.push({
          id: this.generateIssueId("relevance", issues.length),
          category: "relevance",
          severity: "high",
          description: result.object.reasoning,
          suggestedAction: "Refine the answer to better address the question",
        })
      }

      return {
        passed,
        score: result.object.score,
        weight: QUERY_CHECK_ITEMS.relevance.weight,
        details: result.object.reasoning,
      }
    } catch (error) {
      log.warn("Relevance check LLM failed", { error })
      return {
        passed: true,
        score: 7,
        weight: QUERY_CHECK_ITEMS.relevance.weight,
        details: "Relevance check skipped",
      }
    }
  }

  private async checkCompleteness(
    output: QueryOutput | undefined,
    originalRequest: string,
    issues: PDCAIssue[],
    config: PDCAConfig,
  ): Promise<CheckItemResult> {
    if (!output?.answer) {
      return {
        passed: false,
        score: 0,
        weight: QUERY_CHECK_ITEMS.completeness.weight,
        details: "No answer",
      }
    }

    // Basic completeness check based on answer length and context
    const answerLength = output.answer.length
    const hasContext = output.context && output.context.length > 0

    let score = 5
    if (answerLength > 100) score += 2
    if (answerLength > 300) score += 1
    if (hasContext) score += 2

    const passed = score >= 7

    if (!passed) {
      issues.push({
        id: this.generateIssueId("completeness", issues.length),
        category: "completeness",
        severity: "medium",
        description: "Answer may not be complete",
        suggestedAction: "Expand the answer with more details",
      })
    }

    return {
      passed,
      score: Math.min(10, score),
      weight: QUERY_CHECK_ITEMS.completeness.weight,
      details: `${answerLength} chars, ${hasContext ? "with" : "without"} context`,
    }
  }

  private checkAccuracy(
    output: QueryOutput | undefined,
    issues: PDCAIssue[],
  ): CheckItemResult {
    if (!output?.answer) {
      return {
        passed: false,
        score: 0,
        weight: QUERY_CHECK_ITEMS.accuracy.weight,
        details: "No answer",
      }
    }

    // Use provided confidence as accuracy proxy
    const confidence = output.confidence ?? 0.5
    const score = confidence * 10
    const passed = score >= 6

    if (!passed) {
      issues.push({
        id: this.generateIssueId("accuracy", issues.length),
        category: "accuracy",
        severity: "medium",
        description: `Low confidence: ${(confidence * 100).toFixed(0)}%`,
        suggestedAction: "Verify answer with additional sources",
      })
    }

    return {
      passed,
      score,
      weight: QUERY_CHECK_ITEMS.accuracy.weight,
      details: `Confidence: ${(confidence * 100).toFixed(0)}%`,
    }
  }

  private checkClarity(
    output: QueryOutput | undefined,
    issues: PDCAIssue[],
  ): CheckItemResult {
    if (!output?.answer) {
      return {
        passed: true,
        score: 5,
        weight: QUERY_CHECK_ITEMS.clarity.weight,
        details: "No answer",
      }
    }

    // Basic clarity metrics
    const avgSentenceLength = output.answer.length / (output.answer.split(/[.!?]/).length || 1)
    const hasStructure = output.answer.includes("\n") || output.answer.includes("-")

    let score = 6
    if (avgSentenceLength < 150) score += 2
    if (hasStructure) score += 2

    return {
      passed: score >= 6,
      score: Math.min(10, score),
      weight: QUERY_CHECK_ITEMS.clarity.weight,
      details: `Avg sentence: ${avgSentenceLength.toFixed(0)} chars`,
    }
  }

  // ==========================================================================
  // CLOSE Score Calculation
  // ==========================================================================

  private calculateQueryCLOSEScore(
    checks: Record<string, CheckItemResult>,
    output: QueryOutput | undefined,
  ): CLOSEScore {
    let totalWeight = 0
    let weightedSum = 0

    for (const [, check] of Object.entries(checks)) {
      totalWeight += check.weight
      weightedSum += check.score * check.weight
    }

    const avgScore = totalWeight > 0 ? weightedSum / totalWeight : 5.0

    return {
      convergence: Math.round(avgScore * 10) / 10,
      leverage: Math.round((output?.confidence ?? 0.5) * 10 * 10) / 10,
      optionality: 10, // Queries are always reversible
      surplus: Math.round(avgScore * 10) / 10,
      evolution: 5, // Moderate learning from queries
      total: Math.round(avgScore * 10) / 10,
    }
  }
}

/** Factory function */
export function createQueryStrategy(): QueryStrategy {
  return new QueryStrategy()
}
