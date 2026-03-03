/**
 * Acceptance Strategy Base Interface
 *
 * Defines the interface for task-type-specific acceptance strategies.
 * Each strategy implements its own check and fix logic while sharing
 * the common PDCA cycle structure.
 */

import type { TaskType } from "../../classification/types"
import type {
  TaskExecutionResult,
  PDCACheckResult,
  PDCAActResult,
  PDCAIssue,
  PDCAConfig,
} from "../types"

// ============================================================================
// Strategy Interface
// ============================================================================

/**
 * Acceptance strategy interface.
 *
 * Implementations provide task-type-specific:
 * - Check logic (validation criteria)
 * - Fix logic (remediation strategies)
 * - Check item definitions
 */
export interface AcceptanceStrategy<TOutput = unknown> {
  /** Task type this strategy handles */
  readonly taskType: TaskType

  /** Human-readable strategy name */
  readonly name: string

  /**
   * Execute Check phase for a task execution result.
   *
   * @param result - The execution result from Do phase
   * @param originalRequest - The original user request
   * @param config - PDCA configuration
   * @returns Check result with issues and recommendation
   */
  check(
    result: TaskExecutionResult<TOutput>,
    originalRequest: string,
    config: PDCAConfig,
  ): Promise<PDCACheckResult>

  /**
   * Execute Act phase to fix identified issues.
   *
   * @param issues - Issues found during Check phase
   * @param context - The original execution result for context
   * @param config - PDCA configuration
   * @returns Act result with fix outcomes
   */
  fix(
    issues: PDCAIssue[],
    context: TaskExecutionResult<TOutput>,
    config: PDCAConfig,
  ): Promise<PDCAActResult>

  /**
   * Get the list of check items this strategy evaluates.
   *
   * @returns Array of check item names
   */
  getCheckItems(): string[]

  /**
   * Get default weights for check items.
   *
   * @returns Map of check item name to weight
   */
  getCheckWeights(): Record<string, number>
}

// ============================================================================
// Strategy Context
// ============================================================================

/** Context passed to strategy during Check/Act phases */
export interface StrategyContext {
  /** Session ID for tracking */
  sessionId: string
  /** Working directory for file operations */
  workingDir?: string
  /** Original user request */
  originalRequest: string
  /** Current cycle number */
  cycleNumber: number
  /** Previous check results (for retry awareness) */
  previousChecks?: PDCACheckResult[]
}

// ============================================================================
// Base Strategy Implementation Helper
// ============================================================================

/**
 * Abstract base class providing common functionality for strategies.
 */
export abstract class BaseAcceptanceStrategy<TOutput = unknown>
  implements AcceptanceStrategy<TOutput>
{
  abstract readonly taskType: TaskType
  abstract readonly name: string

  abstract check(
    result: TaskExecutionResult<TOutput>,
    originalRequest: string,
    config: PDCAConfig,
  ): Promise<PDCACheckResult>

  abstract fix(
    issues: PDCAIssue[],
    context: TaskExecutionResult<TOutput>,
    config: PDCAConfig,
  ): Promise<PDCAActResult>

  abstract getCheckItems(): string[]

  abstract getCheckWeights(): Record<string, number>

  /**
   * Calculate CLOSE score from check item results.
   */
  protected calculateCLOSEScore(
    checks: Record<string, { passed: boolean; score: number; weight: number }>,
  ): {
    convergence: number
    leverage: number
    optionality: number
    surplus: number
    evolution: number
    total: number
  } {
    // Calculate weighted average of check scores
    let totalWeight = 0
    let weightedSum = 0

    for (const [, check] of Object.entries(checks)) {
      totalWeight += check.weight
      weightedSum += check.score * check.weight
    }

    const avgScore = totalWeight > 0 ? weightedSum / totalWeight : 5.0

    // Map average check score to CLOSE dimensions
    // This is a simplified mapping - strategies can override for more precision
    return {
      convergence: Math.round(avgScore * 10) / 10,
      leverage: Math.round(avgScore * 10) / 10,
      optionality: Math.round((10 - avgScore * 0.5) * 10) / 10, // Inverse relationship
      surplus: Math.round(avgScore * 10) / 10,
      evolution: Math.round((avgScore * 0.8) * 10) / 10, // Slightly lower
      total: Math.round(avgScore * 10) / 10,
    }
  }

  /**
   * Determine recommendation based on score and thresholds.
   */
  protected getRecommendation(
    totalScore: number,
    passThreshold: number,
    fixThreshold: number,
    criticalIssues: number,
  ): "pass" | "fix" | "rework" {
    // Critical issues always require attention
    if (criticalIssues > 0) {
      return totalScore >= fixThreshold ? "fix" : "rework"
    }

    if (totalScore >= passThreshold) {
      return "pass"
    } else if (totalScore >= fixThreshold) {
      return "fix"
    } else {
      return "rework"
    }
  }

  /**
   * Generate unique issue ID.
   */
  protected generateIssueId(category: string, index: number): string {
    return `${this.taskType}-${category}-${index}-${Date.now()}`
  }
}
