/**
 * PDCA Framework Types
 *
 * Defines types for the unified PDCA (Plan-Do-Check-Act) cycle
 * that applies to all task types in the autonomous system.
 */

import z from "zod"
import type { TaskType } from "../classification/types"
import type { CLOSEScore } from "../decision/criteria"

// Re-export CLOSEScore for convenience
export type { CLOSEScore } from "../decision/criteria"

// ============================================================================
// Task Execution Result (Do Phase Output)
// ============================================================================

/** Base task execution result from Do phase */
export interface TaskExecutionResult<T = unknown> {
  /** Task type that was executed */
  taskType: TaskType
  /** Whether execution completed without errors */
  success: boolean
  /** Task-specific output data */
  output: T
  /** Execution duration in milliseconds */
  durationMs: number
  /** Optional metadata */
  metadata?: Record<string, unknown>
}

/** Implementation-specific execution output */
export interface ImplementationOutput {
  /** Summary of what was implemented */
  summary: string
  /** Whether the implementation solved the problem */
  solved: boolean
  /** Generated code or solution */
  solution?: string
  /** Knowledge ID if sedimented */
  knowledgeId?: string
  /** Learned tool ID if any */
  learnedToolId?: string
  /** Files modified */
  modifiedFiles?: string[]
}

/** Research-specific execution output */
export interface ResearchOutput {
  /** Research topic */
  topic: string
  /** Summary of findings */
  summary: string
  /** Full report content */
  report: string
  /** Sources found and analyzed */
  sources: Array<{
    url: string
    title: string
    snippet: string
    credibility: "high" | "medium" | "low"
    content?: string
    publishedDate?: string
  }>
  /** Key insights extracted */
  insights: string[]
  /** Output file path if saved */
  outputPath?: string
  /** Hand created for recurring research */
  handCreated?: string
}

/** Query-specific execution output */
export interface QueryOutput {
  /** Answer to the query */
  answer: string
  /** Confidence in the answer (0-1) */
  confidence: number
  /** Sources or context used */
  context?: string[]
}

// ============================================================================
// PDCA Issue (Check Phase Findings)
// ============================================================================

/** Issue severity levels */
export type IssueSeverity = "critical" | "high" | "medium" | "low"

/** Base PDCA issue found during Check phase */
export interface PDCAIssue {
  /** Unique issue identifier */
  id: string
  /** Issue category (task-type specific) */
  category: string
  /** Severity level */
  severity: IssueSeverity
  /** Human-readable description */
  description: string
  /** Location or context where issue was found */
  location?: string
  /** Suggested action to fix */
  suggestedAction?: string
  /** Metadata for the issue */
  metadata?: Record<string, unknown>
}

/** Implementation-specific issue categories */
export type ImplementationIssueCategory =
  | "test"
  | "type"
  | "lint"
  | "security"
  | "requirement"
  | "expectation"

/** Research-specific issue categories */
export type ResearchIssueCategory =
  | "source_credibility"
  | "coverage"
  | "freshness"
  | "accuracy"
  | "insight_quality"
  | "bias"

/** Query-specific issue categories */
export type QueryIssueCategory =
  | "relevance"
  | "completeness"
  | "accuracy"
  | "context_missing"

// ============================================================================
// PDCA Check Result (Check Phase Output)
// ============================================================================

/** Check result for a single check item */
export interface CheckItemResult {
  /** Whether the check passed */
  passed: boolean
  /** Score from 0-10 */
  score: number
  /** Weight for this check item */
  weight: number
  /** Optional details about the check */
  details?: string
}

/** Overall recommendation from Check phase */
export type CheckRecommendation = "pass" | "fix" | "rework"

/** Complete Check phase result */
export interface PDCACheckResult {
  /** Task type that was checked */
  taskType: TaskType
  /** Whether all checks passed */
  passed: boolean
  /** CLOSE framework scores */
  closeScore: CLOSEScore
  /** Issues found during checking */
  issues: PDCAIssue[]
  /** Overall recommendation */
  recommendation: CheckRecommendation
  /** Individual check item results */
  checks: Record<string, CheckItemResult>
  /** Total duration of Check phase */
  durationMs: number
  /** Optional report content */
  report?: string
}

// ============================================================================
// PDCA Act Result (Act/Fix Phase Output)
// ============================================================================

/** Complete Act phase result */
export interface PDCAActResult {
  /** Whether issues were successfully fixed */
  fixed: boolean
  /** Issue IDs that were fixed */
  fixedIssues: string[]
  /** Issues that remain unfixed */
  remainingIssues: PDCAIssue[]
  /** Whether to re-run Check phase */
  shouldRecheck: boolean
  /** Number of fix attempts made */
  attempts: number
  /** Total duration of Act phase */
  durationMs: number
  /** Patterns learned from fixes */
  learnedPatterns?: string[]
}

// ============================================================================
// PDCA Cycle Result
// ============================================================================

/** Final result from complete PDCA cycle */
export interface PDCACycleResult<T = unknown> {
  /** Whether the PDCA cycle succeeded */
  success: boolean
  /** Execution result from Do phase */
  result?: TaskExecutionResult<T>
  /** Check result from final Check phase */
  checkResult?: PDCACheckResult
  /** Act result if fixes were attempted */
  actResult?: PDCAActResult
  /** Number of PDCA cycles completed */
  cycles: number
  /** Reason for failure if unsuccessful */
  reason?: string
  /** Total duration across all cycles */
  totalDurationMs: number
}

// ============================================================================
// PDCA Configuration
// ============================================================================

/** PDCA cycle configuration */
export interface PDCAConfig {
  /** Maximum number of PDCA cycles (default: 3) */
  maxCycles: number
  /** CLOSE score threshold to pass (default: 6.0) */
  passThreshold: number
  /** CLOSE score threshold for fix vs rework (default: 4.0) */
  fixThreshold: number
  /** Task type being processed */
  taskType: TaskType
  /** Enable automatic fixing (default: true) */
  enableFix: boolean
  /** Enable learning from fixes (default: true) */
  enableLearning: boolean
  /** Session ID for event tracking */
  sessionId: string
}

/** Default PDCA configuration */
export const DEFAULT_PDCA_CONFIG: Omit<PDCAConfig, "taskType" | "sessionId"> = {
  maxCycles: 3,
  passThreshold: 6.0,
  fixThreshold: 4.0,
  enableFix: true,
  enableLearning: true,
}

// ============================================================================
// Zod Schemas for Validation
// ============================================================================

export const PDCAIssueSchema = z.object({
  id: z.string(),
  category: z.string(),
  severity: z.enum(["critical", "high", "medium", "low"]),
  description: z.string(),
  location: z.string().optional(),
  suggestedAction: z.string().optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
})

export const CheckItemResultSchema = z.object({
  passed: z.boolean(),
  score: z.number().min(0).max(10),
  weight: z.number().min(0),
  details: z.string().optional(),
})

export const PDCACheckResultSchema = z.object({
  taskType: z.enum(["implementation", "research", "query", "acceptance", "fix", "other"]),
  passed: z.boolean(),
  closeScore: z.object({
    convergence: z.number(),
    leverage: z.number(),
    optionality: z.number(),
    surplus: z.number(),
    evolution: z.number(),
    total: z.number(),
  }),
  issues: z.array(PDCAIssueSchema),
  recommendation: z.enum(["pass", "fix", "rework"]),
  checks: z.record(z.string(), CheckItemResultSchema),
  durationMs: z.number(),
  report: z.string().optional(),
})

export const PDCAActResultSchema = z.object({
  fixed: z.boolean(),
  fixedIssues: z.array(z.string()),
  remainingIssues: z.array(PDCAIssueSchema),
  shouldRecheck: z.boolean(),
  attempts: z.number(),
  durationMs: z.number(),
  learnedPatterns: z.array(z.string()).optional(),
})

export const PDCACycleResultSchema = z.object({
  success: z.boolean(),
  cycles: z.number(),
  reason: z.string().optional(),
  totalDurationMs: z.number(),
})
