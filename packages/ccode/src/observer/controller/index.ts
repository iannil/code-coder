/**
 * Mode Controller Module
 *
 * Exports all mode controller components for the Observer Network.
 *
 * The controller implements the decision layer from the architecture:
 * - AUTO mode: Full autonomous operation
 * - MANUAL mode: Human decision required
 * - HYBRID mode: Auto-execute with human confirmation
 *
 * @module observer/controller
 */

export {
  ModeThresholdsSchema,
  type ModeThresholds,
  type RiskTolerance,
  THRESHOLD_PRESETS,
  ThresholdManager,
  createThresholdManager,
} from "./thresholds"

export {
  CLOSEEvaluator,
  createCLOSEEvaluator,
  type CLOSEDimension,
  type CLOSEEvaluation,
  type CLOSEWeights,
  type CLOSEEvaluatorConfig,
} from "./close-evaluator"

export {
  EscalationManager,
  createEscalationManager,
  type EscalationPriority,
  type EscalationStatus,
  type EscalationContext,
  type Escalation,
  type HumanDecision,
  type EscalationConfig,
} from "./escalation"

export {
  ModeController,
  createModeController,
  getModeController,
  resetModeController,
  type ModeDecision,
  type ModeControllerConfig,
  type ModeControllerStats,
} from "./mode"
