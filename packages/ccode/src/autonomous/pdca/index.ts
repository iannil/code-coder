/**
 * PDCA Framework
 *
 * Unified Plan-Do-Check-Act cycle implementation for all autonomous task types.
 *
 * Features:
 * - Task-type-specific acceptance strategies
 * - Configurable cycle limits and thresholds
 * - Automatic fix attempts with learning
 * - Full event integration for observability
 *
 * Usage:
 * ```typescript
 * import { createPDCAController } from "@/autonomous/pdca"
 *
 * const pdca = createPDCAController({
 *   taskType: "research",
 *   sessionId: "session-123",
 *   maxCycles: 3,
 * })
 *
 * const result = await pdca.execute(
 *   async () => executeResearchTask(),
 *   "用户原始请求"
 * )
 *
 * if (result.success) {
 *   console.log("PDCA passed in", result.cycles, "cycles")
 * } else {
 *   console.log("PDCA failed:", result.reason)
 * }
 * ```
 *
 * @packageDocumentation
 */

// Core types
export type {
  TaskExecutionResult,
  ImplementationOutput,
  ResearchOutput,
  QueryOutput,
  PDCAIssue,
  IssueSeverity,
  ImplementationIssueCategory,
  ResearchIssueCategory,
  QueryIssueCategory,
  CheckItemResult,
  CheckRecommendation,
  PDCACheckResult,
  PDCAActResult,
  PDCACycleResult,
  PDCAConfig,
} from "./types"

export { DEFAULT_PDCA_CONFIG } from "./types"

// Controller
export { UnifiedPDCAController, createPDCAController } from "./controller"
export type { PDCAControllerOptions } from "./controller"

// Strategies
export {
  BaseAcceptanceStrategy,
  StrategyFactory,
  createStrategy,
  ImplementationStrategy,
  createImplementationStrategy,
  ResearchStrategy,
  createResearchStrategy,
  QueryStrategy,
  createQueryStrategy,
  GenericStrategy,
  createGenericStrategy,
} from "./strategies"

export type { AcceptanceStrategy, StrategyContext } from "./strategies"
