/**
 * Evaluation Test Configuration
 *
 * Configuration for complex reliability tests.
 * All tests use real LLM API calls for comprehensive validation.
 */

export interface EvalConfig {
  /** Use mock LLM for tests (false = real API calls) */
  useMockLLM: boolean
  /** Model ID for evaluation tests */
  modelID: string
  /** Timeout for complex operations (ms) */
  timeout: number
  /** Number of retries on failure */
  retryCount: number
  /** Maximum cost per test run (USD) */
  costLimit: number
  /** Maximum tokens per test run */
  tokenLimit: number
}

/**
 * Default evaluation configuration
 */
export const EVAL_CONFIG: EvalConfig = {
  useMockLLM: false,
  modelID: "claude-opus-4-5",
  timeout: 300_000, // 5 minutes
  retryCount: 3,
  costLimit: 50.0,
  tokenLimit: 1_000_000,
}

/**
 * Test thresholds for reliability metrics
 */
export const EVAL_THRESHOLDS = {
  /** Minimum chain completion rate */
  chainCompletionRate: 0.95,
  /** Maximum context loss tolerance */
  contextLossRate: 0.0,
  /** Maximum error recovery time (ms) */
  errorRecoveryTime: 5000,
  /** Minimum parallel speedup factor */
  parallelSpeedup: 2.0,
  /** Minimum memory consistency rate */
  memoryConsistencyRate: 1.0,
  /** Minimum chaos recovery rate */
  chaosRecoveryRate: 0.9,
  /** Maximum query latency (ms) */
  maxQueryLatency: 500,
  /** Minimum test pass rate */
  minTestPassRate: 0.95,
}

/**
 * Autonomy level configurations for testing
 */
export const AUTONOMY_LEVELS = [
  "lunatic",
  "insane",
  "crazy",
  "wild",
  "bold",
  "timid",
] as const

export type AutonomyLevel = (typeof AUTONOMY_LEVELS)[number]

/**
 * Agent chain configurations for multi-agent testing
 */
export const AGENT_CHAINS = {
  /** Standard 4-agent chain: build → review → security → tdd */
  standard: ["build", "code-reviewer", "security-reviewer", "tdd-guide"],
  /** Minimal 2-agent chain */
  minimal: ["build", "code-reviewer"],
  /** Deep 6-agent chain */
  deep: ["explore", "architect", "build", "code-reviewer", "security-reviewer", "tdd-guide"],
  /** Content creation chain */
  content: ["writer", "expander", "proofreader"],
  /** Decision-driven chain */
  decision: ["decision", "architect", "build"],
} as const

/**
 * Test complexity levels
 */
export type ComplexityLevel = "low" | "medium" | "high" | "extreme"

/**
 * Get timeout for a complexity level
 */
export function getTimeoutForComplexity(level: ComplexityLevel): number {
  const timeouts: Record<ComplexityLevel, number> = {
    low: 30_000,
    medium: 60_000,
    high: 180_000,
    extreme: 300_000,
  }
  return timeouts[level]
}

/**
 * Resource budget for tests
 */
export interface ResourceBudget {
  maxTokens: number
  maxCostUSD: number
  maxDurationMinutes: number
}

/**
 * Get resource budget for complexity level
 */
export function getResourceBudget(level: ComplexityLevel): ResourceBudget {
  const budgets: Record<ComplexityLevel, ResourceBudget> = {
    low: { maxTokens: 10_000, maxCostUSD: 1.0, maxDurationMinutes: 1 },
    medium: { maxTokens: 50_000, maxCostUSD: 5.0, maxDurationMinutes: 5 },
    high: { maxTokens: 200_000, maxCostUSD: 20.0, maxDurationMinutes: 15 },
    extreme: { maxTokens: 1_000_000, maxCostUSD: 50.0, maxDurationMinutes: 30 },
  }
  return budgets[level]
}
