/**
 * Generic Strategy
 *
 * Fallback acceptance strategy for task types without specialized strategies.
 * Provides basic quality and intent matching checks.
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
  CheckItemResult,
  CLOSEScore,
} from "../types"

const log = Log.create({ service: "pdca.strategy.generic" })

// ============================================================================
// Check Items for Generic Tasks
// ============================================================================

const GENERIC_CHECK_ITEMS = {
  basic_quality: {
    name: "basic_quality",
    description: "Output meets basic quality standards",
    weight: 1.0,
  },
  intent_match: {
    name: "intent_match",
    description: "Output matches the user's intent",
    weight: 1.2,
  },
  completeness: {
    name: "completeness",
    description: "Task was completed fully",
    weight: 1.0,
  },
} as const

// ============================================================================
// Generic Strategy
// ============================================================================

export class GenericStrategy extends BaseAcceptanceStrategy<unknown> {
  readonly taskType: TaskType
  readonly name: string

  constructor(taskType: TaskType = "other") {
    super()
    this.taskType = taskType
    this.name = `Generic Acceptance Strategy (${taskType})`
  }

  getCheckItems(): string[] {
    return Object.keys(GENERIC_CHECK_ITEMS)
  }

  getCheckWeights(): Record<string, number> {
    return Object.fromEntries(
      Object.entries(GENERIC_CHECK_ITEMS).map(([k, v]) => [k, v.weight]),
    )
  }

  async check(
    result: TaskExecutionResult<unknown>,
    originalRequest: string,
    config: PDCAConfig,
  ): Promise<PDCACheckResult> {
    const startTime = Date.now()
    const issues: PDCAIssue[] = []
    const checks: Record<string, CheckItemResult> = {}

    log.debug("Running generic acceptance check", {
      sessionId: config.sessionId,
      taskType: this.taskType,
      success: result.success,
    })

    // Check 1: Basic Quality
    const qualityCheck = this.checkBasicQuality(result, issues)
    checks.basic_quality = qualityCheck

    // Check 2: Intent Match (requires LLM for complex matching)
    const intentCheck = await this.checkIntentMatch(result, originalRequest, issues, config)
    checks.intent_match = intentCheck

    // Check 3: Completeness
    const completenessCheck = this.checkCompleteness(result, issues)
    checks.completeness = completenessCheck

    // Calculate CLOSE scores
    const closeScore = this.calculateGenericCLOSEScore(checks, result)

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
    context: TaskExecutionResult<unknown>,
    config: PDCAConfig,
  ): Promise<PDCAActResult> {
    const startTime = Date.now()

    log.debug("Running generic fix loop", {
      sessionId: config.sessionId,
      issueCount: issues.length,
    })

    // Generic strategy has limited fix capabilities
    // Just report issues as remaining
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

  // ==========================================================================
  // Check Implementations
  // ==========================================================================

  private checkBasicQuality(
    result: TaskExecutionResult<unknown>,
    issues: PDCAIssue[],
  ): CheckItemResult {
    // Basic quality: did execution succeed?
    const passed = result.success

    if (!passed) {
      issues.push({
        id: this.generateIssueId("basic_quality", 0),
        category: "basic_quality",
        severity: "high",
        description: "Task execution failed",
        suggestedAction: "Retry the task or investigate the failure",
      })
    }

    // Also check if we have output
    const hasOutput = result.output !== undefined && result.output !== null
    if (!hasOutput && passed) {
      issues.push({
        id: this.generateIssueId("basic_quality", issues.length),
        category: "basic_quality",
        severity: "medium",
        description: "Task succeeded but produced no output",
      })
    }

    const score = passed ? (hasOutput ? 10 : 6) : 0

    return {
      passed,
      score,
      weight: GENERIC_CHECK_ITEMS.basic_quality.weight,
      details: passed ? (hasOutput ? "Execution successful with output" : "Execution successful, no output") : "Execution failed",
    }
  }

  private async checkIntentMatch(
    result: TaskExecutionResult<unknown>,
    originalRequest: string,
    issues: PDCAIssue[],
    config: PDCAConfig,
  ): Promise<CheckItemResult> {
    if (!result.success || !result.output) {
      return {
        passed: false,
        score: 0,
        weight: GENERIC_CHECK_ITEMS.intent_match.weight,
        details: "Cannot check intent without successful output",
      }
    }

    // Try to use LLM for intent matching
    try {
      const { generateObject } = await import("ai")
      const { Provider } = await import("@/provider/provider")
      const z = await import("zod").then((m) => m.default)

      const defaultModel = await Provider.defaultModel()
      const model = await Provider.getModel(defaultModel.providerID, defaultModel.modelID)
      const language = await Provider.getLanguage(model)

      const outputStr = typeof result.output === "string"
        ? result.output
        : JSON.stringify(result.output).slice(0, 1000)

      const llmResult = await generateObject({
        temperature: 0.2,
        messages: [
          {
            role: "system",
            content: `You evaluate if a task output matches the user's intent. Score 0-10.`,
          },
          {
            role: "user",
            content: `User Request: ${originalRequest}

Task Output: ${outputStr}

Rate how well the output matches the intent 0-10.`,
          },
        ],
        model: language,
        schema: z.object({
          score: z.number().min(0).max(10),
          reasoning: z.string(),
        }),
      })

      const passed = llmResult.object.score >= 7

      if (!passed) {
        issues.push({
          id: this.generateIssueId("intent_match", issues.length),
          category: "intent_match",
          severity: llmResult.object.score >= 4 ? "medium" : "high",
          description: llmResult.object.reasoning,
          suggestedAction: "Re-execute the task with clearer intent",
        })
      }

      return {
        passed,
        score: llmResult.object.score,
        weight: GENERIC_CHECK_ITEMS.intent_match.weight,
        details: llmResult.object.reasoning,
      }
    } catch (error) {
      log.warn("Intent match LLM check failed", { error })
      // Fallback: assume intent is matched if execution succeeded
      return {
        passed: result.success,
        score: result.success ? 7 : 0,
        weight: GENERIC_CHECK_ITEMS.intent_match.weight,
        details: "Intent check skipped (LLM unavailable)",
      }
    }
  }

  private checkCompleteness(
    result: TaskExecutionResult<unknown>,
    issues: PDCAIssue[],
  ): CheckItemResult {
    // Check if execution completed without being cut short
    const passed = result.success

    // Check duration - very short tasks might be incomplete
    const minDurationMs = 100
    const hasReasonableDuration = result.durationMs >= minDurationMs

    const score = passed ? (hasReasonableDuration ? 10 : 6) : 0

    if (passed && !hasReasonableDuration) {
      issues.push({
        id: this.generateIssueId("completeness", issues.length),
        category: "completeness",
        severity: "low",
        description: "Task completed very quickly - verify completeness",
      })
    }

    return {
      passed,
      score,
      weight: GENERIC_CHECK_ITEMS.completeness.weight,
      details: `Duration: ${result.durationMs}ms`,
    }
  }

  // ==========================================================================
  // CLOSE Score Calculation
  // ==========================================================================

  private calculateGenericCLOSEScore(
    checks: Record<string, CheckItemResult>,
    result: TaskExecutionResult<unknown>,
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
      leverage: Math.round(avgScore * 0.8 * 10) / 10,
      optionality: 8, // Generic tasks usually reversible
      surplus: Math.round(avgScore * 10) / 10,
      evolution: 5, // Moderate learning
      total: Math.round(avgScore * 10) / 10,
    }
  }
}

/** Factory function */
export function createGenericStrategy(taskType: TaskType = "other"): GenericStrategy {
  return new GenericStrategy(taskType)
}
