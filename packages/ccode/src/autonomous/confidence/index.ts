/**
 * Confidence Scoring Module
 *
 * Provides multi-dimensional confidence scoring for agent outputs,
 * inspired by the MiroFish OASIS pattern.
 */

export {
  ConfidenceScorer,
  createConfidenceScorer,
  scoreOutput,
  isOutputConfident,
  Confidence,
  type ConfidenceScore,
  type ConfidenceDetails,
  type ConfidenceScorerConfig,
  type AgentOutput,
  type AgentContext,
} from "./scorer.js"
