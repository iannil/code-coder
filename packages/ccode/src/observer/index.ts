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
 * ```
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                    Observer Layer (Watchers)                        │
 * │   CodeWatch │ WorldWatch │ SelfWatch │ MetaWatch                   │
 * └──────────────────────────────┬──────────────────────────────────────┘
 *                                │
 *                                ▼
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                    Event Stream                                     │
 * │   Buffering │ Routing │ Aggregation                                │
 * └──────────────────────────────┬──────────────────────────────────────┘
 *                                │
 *                                ▼
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                    Consensus Layer                                  │
 * │   Attention │ Patterns │ Anomalies │ World Model                   │
 * └──────────────────────────────┬──────────────────────────────────────┘
 *                                │
 *                                ▼
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                    Mode Controller                                  │
 * │   AUTO │ MANUAL │ HYBRID │ CLOSE Evaluation                        │
 * └──────────────────────────────┬──────────────────────────────────────┘
 *                                │
 *                                ▼
 * ┌─────────────────────────────────────────────────────────────────────┐
 * │                    Response Layer                                   │
 * │   Notifier │ Analyzer │ Executor │ Historian                       │
 * └─────────────────────────────────────────────────────────────────────┘
 * ```
 *
 * ## Usage
 *
 * ```typescript
 * import { ObserverNetwork } from "@/observer"
 *
 * // Start the network
 * const network = await ObserverNetwork.start({
 *   mode: "HYBRID",
 *   watchers: {
 *     code: { enabled: true },
 *     world: { enabled: true },
 *     self: { enabled: true },
 *     meta: { enabled: true },
 *   },
 * })
 *
 * // Subscribe to observations
 * network.onObservation((obs) => {
 *   console.log(`[${obs.watcherType}] ${obs.type}`)
 * })
 *
 * // Get current world model
 * const model = await network.getWorldModel()
 *
 * // Stop the network
 * await network.stop()
 * ```
 *
 * @module observer
 */

// ─────────────────────────────────────────────────────────────────────────────
// Type Exports
// ─────────────────────────────────────────────────────────────────────────────

export * from "./types"

// ─────────────────────────────────────────────────────────────────────────────
// Dial & Gear Exports (New Architecture)
// ─────────────────────────────────────────────────────────────────────────────

export {
  Dial,
  ThreeDials,
  GEAR_PRESETS,
  GEAR_INFO,
  createDialsFromGear,
  createCustomDials,
  parseGear,
  gearAllowsAutonomousDecisions,
  gearAllowsAutonomousActions,
  type DialMode,
  type GearPreset,
  type DialName,
  type DialConfig,
  type ThreeDialsConfig,
  type DialValues,
} from "./dial"

// ─────────────────────────────────────────────────────────────────────────────
// Tower Exports (New Architecture)
// ─────────────────────────────────────────────────────────────────────────────

export {
  ObserverTower,
  ObserverLevel,
  RawObservationLevel,
  PatternRecognitionLevel,
  MetaObservationLevel,
  createObserverTower,
  createRawObservationLevel,
  createPatternRecognitionLevel,
  createMetaObservationLevel,
  type ObserverTowerConfig,
  type TowerStatus,
  type LevelOutput,
  type LevelStatus,
  type LevelConfig,
  type LevelData,
  type LevelInsight,
  type Level0Config,
  type Level1Config,
  type Level2Config,
} from "./tower"

// ─────────────────────────────────────────────────────────────────────────────
// Panel Exports (New Architecture)
// ─────────────────────────────────────────────────────────────────────────────

export {
  DialPanel,
  createDialPanel,
  getDialPanel,
  resetDialPanel,
  GEAR_PRESET_DETAILS,
  getGearPresetDetail,
  getAllGearPresets,
  suggestGear,
  getGearRiskLevel,
  validateGearTransition,
  type DialPanelConfig,
  type PanelState,
  type DialChangeEvent,
  type GearPresetDetail,
} from "./panel"

// ─────────────────────────────────────────────────────────────────────────────
// Unified API (New Architecture)
// ─────────────────────────────────────────────────────────────────────────────

export {
  ObserverNetwork as ObserverNetworkV2,
  type NetworkConfig,
  type NetworkState,
} from "./api"

// ─────────────────────────────────────────────────────────────────────────────
// Event Exports
// ─────────────────────────────────────────────────────────────────────────────

export { ObserverEvent, ObserverEventHelper } from "./events"

// ─────────────────────────────────────────────────────────────────────────────
// Event Stream Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  EventStream,
  getEventStream,
  resetEventStream,
  createEventStream,
  type EventStreamConfig,
  type ObservationWindow,
  type ObservationHandler,
} from "./event-stream"

// ─────────────────────────────────────────────────────────────────────────────
// Watcher Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  BaseWatcher,
  CodeWatch,
  WorldWatch,
  SelfWatch,
  MetaWatch,
  createCodeWatch,
  createWorldWatch,
  createSelfWatch,
  createMetaWatch,
  type WatcherOptions,
  type WatcherType,
  type CodeWatchOptions,
  type WorldWatchOptions,
  type SelfWatchOptions,
  type MetaWatchOptions,
} from "./watchers"

// ─────────────────────────────────────────────────────────────────────────────
// Consensus Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  AttentionCalculator,
  createAttentionCalculator,
  PatternDetector,
  createPatternDetector,
  AnomalyDetector,
  createAnomalyDetector,
  OpportunityIdentifier,
  createOpportunityIdentifier,
  WorldModelBuilder,
  createWorldModelBuilder,
  ConsensusEngine,
  createConsensusEngine,
  getConsensusEngine,
  resetConsensusEngine,
  type AttentionConfig,
  type WeightedObservation,
  type PatternConfig,
  type AnomalyConfig,
  type OpportunityConfig,
  type WorldModelConfig,
  type ConsensusEngineConfig,
  type ConsensusSnapshot,
} from "./consensus"

// ─────────────────────────────────────────────────────────────────────────────
// Controller Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  ModeThresholdsSchema,
  ThresholdManager,
  createThresholdManager,
  CLOSEEvaluator,
  createCLOSEEvaluator,
  EscalationManager,
  createEscalationManager,
  ModeController,
  createModeController,
  getModeController,
  resetModeController,
  THRESHOLD_PRESETS,
  type ModeThresholds,
  type RiskTolerance,
  type CLOSEDimension,
  type CLOSEEvaluation,
  type CLOSEWeights,
  type CLOSEEvaluatorConfig,
  type EscalationPriority,
  type EscalationStatus,
  type EscalationContext,
  type Escalation,
  type HumanDecision,
  type EscalationConfig,
  type ModeDecision,
  type ModeControllerConfig,
  type ModeControllerStats,
} from "./controller"

// ─────────────────────────────────────────────────────────────────────────────
// Responder Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  Notifier,
  createNotifier,
  Analyzer,
  createAnalyzer,
  Executor,
  createExecutor,
  Historian,
  createHistorian,
  type NotificationPriority,
  type NotificationChannel,
  type Notification,
  type NotificationRule,
  type NotifierConfig,
  type AnalysisType,
  type AnalysisStatus,
  type AnalysisRequest,
  type AnalysisResult,
  type AnalyzerConfig,
  type ExecutionType,
  type ExecutionStatus,
  type ExecutionRequest,
  type ExecutionAction,
  type ExecutionResult,
  type ExecutorConfig,
  type HistoryEventType,
  type HistoryEntry,
  type HistoryQuery,
  type HistoryStats,
  type HistorianConfig,
} from "./responders"

// ─────────────────────────────────────────────────────────────────────────────
// Integration Exports
// ─────────────────────────────────────────────────────────────────────────────

export {
  ChannelsClient,
  getChannelsClient,
  resetChannelsClient,
  createChannelsClient,
  MemoryClient,
  getMemoryClient,
  resetMemoryClient,
  createMemoryClient,
  AgentClient,
  getAgentClient,
  resetAgentClient,
  createAgentClient,
  type ChannelType,
  type SendMessageRequest,
  type MessageContent,
  type SendMessageResponse,
  type InlineButton,
  type SendWithButtonsRequest,
  type ChannelsClientConfig,
  type ObserverMemoryConfig,
  type ObserverHistoryEntry,
  type AgentInvocation,
  type AgentResult,
  type AgentClientConfig,
} from "./integration"

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
// Observer Network (Main Entry Point)
// ─────────────────────────────────────────────────────────────────────────────

import { Log } from "@/util/log"
import type {
  Observation,
  WorldModel,
  ObserverNetworkConfig,
  WatcherStatus,
  OperatingMode,
} from "./types"
import { getEventStream, resetEventStream, type EventStreamConfig } from "./event-stream"
import { ObserverEvent } from "./events"
import {
  CodeWatch,
  WorldWatch,
  SelfWatch,
  MetaWatch,
  type CodeWatchOptions,
  type WorldWatchOptions,
  type SelfWatchOptions,
  type MetaWatchOptions,
} from "./watchers"
import {
  ConsensusEngine,
  getConsensusEngine,
  resetConsensusEngine,
  type ConsensusEngineConfig,
} from "./consensus"
import {
  ModeController,
  getModeController,
  resetModeController,
  type ModeControllerConfig,
  type RiskTolerance,
} from "./controller"

const log = Log.create({ service: "observer.network" })

/**
 * Configuration for starting the Observer Network.
 */
export interface NetworkStartConfig {
  /** Operating mode */
  mode?: OperatingMode
  /** Risk tolerance level for mode controller */
  riskTolerance?: RiskTolerance
  /** Enable automatic mode switching based on CLOSE evaluation */
  autoModeSwitch?: boolean
  /** Event stream configuration */
  stream?: Partial<EventStreamConfig>
  /** Watcher configurations */
  watchers?: {
    code?: CodeWatchOptions | boolean
    world?: WorldWatchOptions | boolean
    self?: SelfWatchOptions | boolean
    meta?: MetaWatchOptions | boolean
  }
  /** Mode controller configuration */
  controller?: Partial<ModeControllerConfig>
}

/**
 * Observer Network instance.
 */
export interface ObserverNetworkInstance {
  /** Get current operating mode */
  getMode(): OperatingMode
  /** Get all watcher statuses */
  getWatcherStatuses(): WatcherStatus[]
  /** Get event stream statistics */
  getStats(): { observations: number; patterns: number; anomalies: number }
  /** Subscribe to observations */
  onObservation(handler: (observation: Observation) => void): () => void
  /** Get current world model */
  getWorldModel(): Promise<WorldModel | null>
  /** Get consensus snapshot */
  getSnapshot(): import("./consensus").ConsensusSnapshot | null
  /** Get active opportunities */
  getOpportunities(): import("./types").Opportunity[]
  /** Switch operating mode */
  switchMode(mode: OperatingMode, reason?: string): Promise<void>
  /** Get pending escalations */
  getPendingEscalations(): import("./controller").Escalation[]
  /** Handle human decision for escalation */
  handleHumanDecision(escalationId: string, decision: import("./controller").HumanDecision): Promise<void>
  /** Get mode controller statistics */
  getModeControllerStats(): import("./controller").ModeControllerStats | null
  /** Stop the network */
  stop(): Promise<void>
  /** Check if running */
  isRunning(): boolean
}

/**
 * Observer Network namespace.
 */
export namespace ObserverNetwork {
  let instance: ObserverNetworkInstance | null = null
  let watchers: {
    code?: CodeWatch
    world?: WorldWatch
    self?: SelfWatch
    meta?: MetaWatch
  } = {}
  let consensusEngine: ConsensusEngine | null = null
  let modeController: ModeController | null = null
  let currentMode: OperatingMode = "HYBRID"
  let running = false
  let startTime: Date | null = null
  let modeSwitchCount = 0

  /**
   * Start the Observer Network.
   */
  export async function start(
    config: NetworkStartConfig = {},
  ): Promise<ObserverNetworkInstance> {
    if (running) {
      log.warn("Observer Network already running")
      return instance!
    }

    currentMode = config.mode ?? "HYBRID"
    startTime = new Date()

    // Initialize event stream
    const stream = getEventStream(config.stream)
    stream.start()

    // Initialize watchers
    const watcherConfig = config.watchers ?? {}

    if (watcherConfig.code !== false) {
      const opts = typeof watcherConfig.code === "object" ? watcherConfig.code : {}
      watchers.code = new CodeWatch(opts)
      await watchers.code.start()
    }

    if (watcherConfig.world !== false) {
      const opts = typeof watcherConfig.world === "object" ? watcherConfig.world : {}
      watchers.world = new WorldWatch(opts)
      await watchers.world.start()
    }

    if (watcherConfig.self !== false) {
      const opts = typeof watcherConfig.self === "object" ? watcherConfig.self : {}
      watchers.self = new SelfWatch(opts)
      await watchers.self.start()
    }

    if (watcherConfig.meta !== false) {
      const opts = typeof watcherConfig.meta === "object" ? watcherConfig.meta : {}
      watchers.meta = new MetaWatch(opts)
      await watchers.meta.start()
    }

    // Initialize and start consensus engine
    consensusEngine = getConsensusEngine({
      windowMs: 60000,
      updateIntervalMs: 5000,
    })
    consensusEngine.start()

    // Initialize mode controller if auto mode switching is enabled
    if (config.autoModeSwitch !== false) {
      modeController = getModeController({
        initialMode: currentMode,
        riskTolerance: config.riskTolerance ?? "balanced",
        autoApply: true,
        ...config.controller,
      })

      // Start mode controller with snapshot provider
      modeController.start(async () => consensusEngine?.getSnapshot() ?? null)
    }

    running = true

    // Publish network started event
    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.NetworkStarted, {
      config: config as Record<string, unknown>,
      watcherCount: Object.keys(watchers).length,
      mode: currentMode,
    })

    log.info("Observer Network started", {
      mode: currentMode,
      watchers: Object.keys(watchers),
    })

    instance = createInstance()
    return instance
  }

  /**
   * Stop the Observer Network.
   */
  export async function stop(reason = "Manual stop"): Promise<void> {
    if (!running) return

    // Stop all watchers
    for (const [name, watcher] of Object.entries(watchers)) {
      if (watcher) {
        await watcher.stop(reason)
      }
    }

    // Stop event stream
    const stream = getEventStream()
    stream.stop()

    // Get stats from consensus engine before stopping
    const streamStats = stream.getStats()
    const patterns = consensusEngine?.getPatterns() ?? []
    const anomalies = consensusEngine?.getAnomalies() ?? []

    // Stop consensus engine
    if (consensusEngine) {
      consensusEngine.stop()
    }

    // Stop mode controller
    if (modeController) {
      modeController.stop()
    }

    running = false

    // Publish network stopped event
    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.NetworkStopped, {
      reason,
      stats: {
        uptime: startTime ? Date.now() - startTime.getTime() : 0,
        totalObservations: streamStats.received,
        patternsDetected: patterns.length,
        anomaliesDetected: anomalies.length,
        modeSwitches: modeSwitchCount,
      },
    })

    // Clear state
    watchers = {}
    instance = null
    startTime = null
    modeSwitchCount = 0
    resetEventStream()
    resetConsensusEngine()
    resetModeController()
    consensusEngine = null
    modeController = null

    log.info("Observer Network stopped", { reason })
  }

  /**
   * Get the current instance.
   */
  export function getInstance(): ObserverNetworkInstance | null {
    return instance
  }

  /**
   * Check if the network is running.
   */
  export function isRunning(): boolean {
    return running
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private
  // ─────────────────────────────────────────────────────────────────────────────

  function createInstance(): ObserverNetworkInstance {
    return {
      getMode: () => modeController?.getMode() ?? currentMode,

      getWatcherStatuses: () => {
        return Object.values(watchers)
          .filter(Boolean)
          .map((w) => w!.getStatus())
      },

      getStats: () => {
        const stream = getEventStream()
        const streamStats = stream.getStats()
        const patterns = consensusEngine?.getPatterns() ?? []
        const anomalies = consensusEngine?.getAnomalies() ?? []
        return {
          observations: streamStats.received,
          patterns: patterns.length,
          anomalies: anomalies.length,
        }
      },

      onObservation: (handler) => {
        const stream = getEventStream()
        return stream.subscribe(handler)
      },

      getWorldModel: async () => {
        return consensusEngine?.getWorldModel() ?? null
      },

      getSnapshot: () => {
        return consensusEngine?.getSnapshot() ?? null
      },

      getOpportunities: () => {
        return consensusEngine?.getOpportunities() ?? []
      },

      switchMode: async (mode, reason) => {
        if (modeController) {
          await modeController.switchMode(mode, reason)
        } else {
          const previousMode = currentMode
          currentMode = mode
          modeSwitchCount++

          const BusModule = await import("@/bus")
          await BusModule.Bus.publish(ObserverEvent.ModeSwitched, {
            previousMode,
            newMode: mode,
            reason: reason ?? "Manual switch",
            timestamp: new Date(),
          })

          log.info("Mode switched", { from: previousMode, to: mode, reason })
        }
      },

      getPendingEscalations: () => {
        return modeController?.getPendingEscalations() ?? []
      },

      handleHumanDecision: async (escalationId, decision) => {
        if (modeController) {
          await modeController.handleHumanDecision(escalationId, decision)
        }
      },

      getModeControllerStats: () => {
        return modeController?.getStats() ?? null
      },

      stop: () => stop(),

      isRunning: () => running,
    }
  }
}
