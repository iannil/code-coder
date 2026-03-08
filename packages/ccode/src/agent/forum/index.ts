/**
 * Forum-based multi-agent collaboration module.
 *
 * Provides tools for structured multi-agent discussions where agents
 * take turns contributing to a topic across multiple rounds.
 *
 * ## Key Components
 *
 * - **ForumModerator**: Guides discussions, provides context, summarizes outcomes
 * - **Consensus utilities**: Track positions, analyze consensus evolution
 *
 * ## Design Principle
 *
 * The TypeScript components handle **non-deterministic** aspects:
 * - Generating guidance prompts for speakers
 * - Detecting consensus and dissent
 * - Summarizing discussions
 *
 * The Rust components (in zero-hub) handle **deterministic** aspects:
 * - Session state management
 * - Turn order scheduling
 * - Timeout handling
 *
 * ## Example
 *
 * ```typescript
 * const moderator = createModerator(
 *   'Should we adopt microservices?',
 *   ['architect', 'developer', 'ops']
 * )
 *
 * // Get guidance for a speaker
 * const guidance = await moderator.guideTurn('architect', 1)
 *
 * // Record the response
 * moderator.recordTurn({
 *   round: 1,
 *   speaker: 'architect',
 *   content: '...',
 *   timestamp: new Date()
 * })
 *
 * // Later: summarize
 * const summary = await moderator.summarize()
 * ```
 */

export {
  ForumModerator,
  createModerator,
  type ForumTurn,
  type GuidancePrompt,
  type ForumConsensusResult,
  type ConsensusPoint,
  type ForumSummary,
} from './moderator'

export {
  trackPositions,
  analyzeConsensusEvolution,
  identifyTurningPoints,
  calculateWeightedConsensus,
  type PositionTracker,
  type ConsensusEvolution,
} from './consensus'
