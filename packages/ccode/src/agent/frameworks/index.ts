/**
 * Agent Frameworks
 *
 * Shared base frameworks for agent groups that share structural patterns.
 * These frameworks reduce code duplication while keeping agents independent.
 *
 * Framework Overview:
 *
 * 1. BaseReviewer - For code analysis agents (code-reviewer, security-reviewer)
 *    - Consistent finding structures and severity levels
 *    - Standard report formats and verdict calculation
 *    - Shared markdown formatting utilities
 *
 * 2. BaseMarketAnalyst - For market/economic analysis agents (macro, trader, value-analyst)
 *    - Unified market data and indicator structures
 *    - Cycle analysis and causal chain types
 *    - Standard risk and recommendation formats
 *
 * 3. BasePhilosophyAdvisor - For decision/observer agents (observer, decision)
 *    - CLOSE framework scoring (祝融说 philosophy)
 *    - Decision analysis with scenario planning
 *    - Philosophical concept types and utilities
 *
 * Usage:
 * ```typescript
 * import { BaseReviewer, BaseAnalyst, BaseAdvisor } from "@/agent/frameworks"
 *
 * // Use BaseReviewer types
 * const finding: BaseReviewer.CodeFinding = { ... }
 *
 * // Use BaseAnalyst utilities
 * const mermaid = BaseAnalyst.formatCausalChainMermaid(chain)
 *
 * // Use BaseAdvisor CLOSE scoring
 * const score = BaseAdvisor.quickCLOSEScore({ convergence: 7, ... })
 * ```
 *
 * @module agent/frameworks
 */

// ─────────────────────────────────────────────────────────────────────────────
// BaseReviewer Framework
// ─────────────────────────────────────────────────────────────────────────────

export * as BaseReviewer from "./base-reviewer"
export {
  // Types
  type Severity,
  type CodeReviewCategory,
  type SecurityCategory,
  type Location,
  type BaseFinding,
  type CodeFinding,
  type SecurityFinding,
  type SeverityMetrics,
  type Verdict,
  type BaseReviewReport,
  type CodeReviewReport,
  type SecurityReviewReport,
  type BaseReviewerConfig,
  // Utilities
  calculateMetrics,
  calculateScore,
  calculateVerdict,
  formatFindingsMarkdown,
  formatReportMarkdown,
  generateFindingId,
  generateReportId,
  // Constants
  SEVERITY_WEIGHTS,
} from "./base-reviewer"

// ─────────────────────────────────────────────────────────────────────────────
// BaseMarketAnalyst Framework
// ─────────────────────────────────────────────────────────────────────────────

export * as BaseAnalyst from "./base-analyst"
export {
  // Types
  type TimeHorizon,
  type DataFrequency,
  type CyclePhase,
  type CyclePosition,
  type TrendDirection,
  type CurrentState,
  type Indicator,
  type MarketData,
  type CausalLink,
  type CausalChain,
  type RiskAssessment,
  type Recommendation,
  type BaseAnalysisReport,
  type MacroAnalysisReport,
  type TradingAnalysisReport,
  type ValueAnalysisReport,
  type BaseAnalystConfig,
  // Utilities
  calculateCycleState,
  formatCausalChainMermaid,
  formatAnalysisMarkdown,
  generateAnalysisReportId,
  // Constants
  DEFAULT_DISCLAIMERS,
} from "./base-analyst"

// ─────────────────────────────────────────────────────────────────────────────
// BasePhilosophyAdvisor Framework
// ─────────────────────────────────────────────────────────────────────────────

export * as BaseAdvisor from "./base-advisor"
export {
  // CLOSE Types
  type CLOSEDimension,
  type CLOSEScore,
  type CLOSEWeights,
  DEFAULT_CLOSE_WEIGHTS,
  // Decision Types
  type ScenarioAnalysis,
  type DecisionOption,
  type SurplusProtection,
  type DecisionAnalysis,
  // Philosophy Types
  type PossibilitySpace,
  type ObservationRecord,
  type MarginAssessment,
  // Config Types
  type AdvisorFocusMode,
  type AdvisorOutputStyle,
  type BaseAdvisorConfig,
  // Utilities
  calculateCLOSETotal,
  calculateCLOSERisk,
  calculateCLOSEConfidence,
  buildCLOSEScore,
  quickCLOSEScore,
  formatCLOSEMarkdown,
  formatDecisionMarkdown,
  generateAnalysisId,
  // Constants
  PHILOSOPHY_PRINCIPLES,
} from "./base-advisor"
