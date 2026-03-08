/**
 * Consensus Module
 *
 * Exports all consensus components for the Observer Network.
 *
 * @module observer/consensus
 */

export {
  AttentionCalculator,
  createAttentionCalculator,
  type AttentionConfig,
  type WeightedObservation,
} from "./attention"

export {
  PatternDetector,
  createPatternDetector,
  type PatternConfig,
} from "./patterns"

export {
  AnomalyDetector,
  createAnomalyDetector,
  type AnomalyConfig,
} from "./anomaly"

export {
  OpportunityIdentifier,
  createOpportunityIdentifier,
  type OpportunityConfig,
} from "./opportunity"

export {
  WorldModelBuilder,
  createWorldModelBuilder,
  type WorldModelConfig,
} from "./world-model"

export {
  ConsensusEngine,
  createConsensusEngine,
  getConsensusEngine,
  resetConsensusEngine,
  type ConsensusEngineConfig,
  type ConsensusSnapshot,
} from "./engine"
