/**
 * Multi-agent aggregation module.
 *
 * Provides OASIS (Optimal Agent Synthesis and Integration System) for
 * aggregating outputs from multiple parallel agents.
 *
 * ## Key Components
 *
 * - **OASISAggregator**: Main aggregator with multiple aggregation modes
 * - **Consensus utilities**: Functions for analyzing agreement and dissent
 *
 * ## Aggregation Modes
 *
 * - `concatenate`: Simple concatenation (default)
 * - `vote`: Majority voting for discrete choices
 * - `weighted`: Confidence-weighted combination
 * - `oasis`: Full synthesis with conflict resolution
 *
 * ## Design Principle
 *
 * Aggregation is an inherently **uncertain** task that requires understanding
 * diverse outputs and synthesizing them appropriately. This module belongs
 * in TypeScript (ccode) as it benefits from LLM reasoning for complex cases.
 */

export {
  OASISAggregator,
  createAggregator,
  type AggregationMode,
  type AgentResult,
  type AggregatedResult,
  type ConsensusInfo,
  type ConflictInfo,
} from './oasis'

export {
  analyzeAgreement,
  extractCommonThemes,
  identifyDissentingViews,
  buildConsensus,
  resolveConflicts,
  type AgreementScore,
  type DissentInfo,
} from './consensus'
