/**
 * Hands Module
 *
 * TypeScript bindings for the Rust Hands autonomous agent system.
 */

export {
  HandsBridge,
  HandsApiError,
  getBridge,
  createBridge,
  triggerHands,
  listHands,
  getHandExecution,
  isHandsServiceHealthy,
  type HandsBridgeConfig,
  type HandConfig,
  type HandManifest,
  type HandExecution,
  type HandSummary,
  type TriggerRequest,
  type TriggerResponse,
  type AutonomyLevel,
  type RiskThreshold,
  type HandTrigger,
  type GitEvent,
  type AutoApproveConfig,
  type ResourceLimits,
  type AutonomyConfig,
  type DecisionConfig,
  type ExecutionStatus,
  HandConfigSchema,
  HandExecutionSchema,
} from "./bridge"
