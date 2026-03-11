/**
 * Observer Network Module
 *
 * The Observer Network transforms CodeCoder from an execution-centric system
 * to an observation-centric system, embodying the "祝融说" philosophy:
 *
 * - 可能性基底 (Possibility Substrate): Raw observation events
 * - 观察即收敛 (Observation as Convergence): Consensus formation
 * - 可用余量 (Available Margin): Mode switching freedom
 * - 评估权 (Evaluation Authority): Human intervention points
 *
 * ## Architecture
 *
 * The Observer Network core logic runs in Rust (services/zero-cli/src/observer/).
 * This TypeScript module provides:
 *
 * 1. **API Client** - `ObserverApiClient` to call the Rust HTTP API
 * 2. **Types** - Re-exported from @/sdk/types for convenience
 * 3. **Dial Controls** - Gear and dial value utilities
 *
 * ### Usage
 *
 * ```typescript
 * import { getObserverClient, isObserverRunning } from "@/observer"
 *
 * // Get the API client
 * const client = getObserverClient({ baseUrl: "http://127.0.0.1:4402" })
 *
 * // Start via API
 * await client.start()
 *
 * // Subscribe to SSE events
 * const eventSource = client.subscribeToEvents((event) => {
 *   console.log(`[${event.type}] ${JSON.stringify(event.data)}`)
 * })
 *
 * // Get world model
 * const response = await client.getWorldModel()
 * if (response.success && response.data) {
 *   console.log("World model:", response.data)
 * }
 *
 * // Stop
 * await client.stop()
 * eventSource.close()
 * ```
 *
 * @module observer
 */

// ─────────────────────────────────────────────────────────────────────────────
// Type Exports (from SDK)
// ─────────────────────────────────────────────────────────────────────────────

// Re-export Observer types from SDK for convenience
export type {
  WatcherType,
  OperatingMode,
  GearPreset,
  DialValues,
  WatcherStatus,
  Observation,
  CLOSEDimension,
  CLOSEEvaluation,
  EscalationPriority,
  EscalationStatus,
  EscalationContext,
  Escalation,
  HumanDecision,
  ConsensusSnapshot,
  ModeDecision,
  ModeControllerStats,
} from "@/sdk/types"

export {
  GEAR_INFO,
  GEAR_PRESETS,
  operatingModeToGear,
  gearToOperatingMode,
} from "@/sdk/types"

// ─────────────────────────────────────────────────────────────────────────────
// Local Type Exports (complex types still in types.ts)
// ─────────────────────────────────────────────────────────────────────────────

export * from "./types"

// ─────────────────────────────────────────────────────────────────────────────
// Dial & Gear Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  Dial,
  ThreeDials,
  GEAR_PRESETS as DIAL_GEAR_PRESETS,
  GEAR_INFO as DIAL_GEAR_INFO,
  createDialsFromGear,
  createCustomDials,
  parseGear,
  gearAllowsAutonomousDecisions,
  gearAllowsAutonomousActions,
  type DialMode,
  type DialName,
  type DialConfig,
  type ThreeDialsConfig,
} from "./dial"

// ─────────────────────────────────────────────────────────────────────────────
// Event Exports
// ─────────────────────────────────────────────────────────────────────────────

export { ObserverEvent, ObserverEventHelper } from "./events"

// ─────────────────────────────────────────────────────────────────────────────
// Agent Registry Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  getObserverAgents,
  getAgentsForWatcher,
  getConsensusAgents,
  getMetaReportingAgents,
  hasObserverCapability,
  getAgentObserverCapability,
  logObserverAgentSummary,
  type ObserverAgentInfo,
} from "./agent-registry"

// ─────────────────────────────────────────────────────────────────────────────
// API Client Exports (calls Rust Observer API)
// ─────────────────────────────────────────────────────────────────────────────

export {
  ObserverApiClient,
  getObserverClient,
  resetObserverClient,
  isObserverRunning,
  getCurrentGear,
  getCurrentDials,
  type ObserverClientConfig,
  type ApiResponse,
  type ObserverStatus,
  type GearStatus,
  type ApiGearPresetDetail,
  type ApiObserverEventType,
  type ApiObserverEvent,
} from "./client"
