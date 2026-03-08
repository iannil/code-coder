/**
 * Observer Network Core Types
 *
 * Type definitions for the Observer Network Architecture that transforms
 * CodeCoder from an execution-centric system to an observation-centric system.
 *
 * Based on "祝融说" philosophy:
 * - 可能性基底 (Possibility Substrate): Raw observation events
 * - 观察即收敛 (Observation as Convergence): Consensus formation
 * - 可用余量 (Available Margin): Mode switching freedom
 *
 * @module observer/types
 */

import z from "zod"

// ─────────────────────────────────────────────────────────────────────────────
// Observation Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Base observation event schema shared by all watcher types.
 */
export const BaseObservation = z.object({
  /** Unique observation ID */
  id: z.string(),
  /** Observation timestamp */
  timestamp: z.date(),
  /** Watcher that produced this observation */
  watcherId: z.string(),
  /** Watcher type for routing */
  watcherType: z.enum(["code", "world", "self", "meta"]),
  /** Confidence score 0-1 */
  confidence: z.number().min(0).max(1),
  /** Optional tags for categorization */
  tags: z.array(z.string()).default([]),
  /** Optional metadata */
  metadata: z.record(z.string(), z.any()).optional(),
})
export type BaseObservation = z.infer<typeof BaseObservation>

/**
 * Code observation types (CodeWatch).
 */
export const CodeObservationType = z.enum([
  "git_change",
  "build_status",
  "test_coverage",
  "tech_debt",
  "file_change",
  "dependency_update",
  "lint_issue",
  "type_error",
])
export type CodeObservationType = z.infer<typeof CodeObservationType>

/**
 * Code observation event.
 */
export const CodeObservation = BaseObservation.extend({
  watcherType: z.literal("code"),
  type: CodeObservationType,
  source: z.string(), // File path, repo, or module
  change: z.object({
    action: z.enum(["add", "modify", "delete", "move"]),
    before: z.any().optional(),
    after: z.any().optional(),
    diff: z.string().optional(),
  }),
  impact: z.object({
    scope: z.enum(["file", "module", "package", "project"]),
    severity: z.enum(["low", "medium", "high", "critical"]),
    affectedFiles: z.array(z.string()).default([]),
  }),
})
export type CodeObservation = z.infer<typeof CodeObservation>

/**
 * World observation types (WorldWatch).
 */
export const WorldObservationType = z.enum([
  "market_data",
  "news",
  "api_change",
  "competitor",
  "dependency_release",
  "security_advisory",
  "regulatory",
  "trend",
])
export type WorldObservationType = z.infer<typeof WorldObservationType>

/**
 * World observation event.
 */
export const WorldObservation = BaseObservation.extend({
  watcherType: z.literal("world"),
  type: WorldObservationType,
  source: z.string(), // URL, API name, or data source
  data: z.object({
    title: z.string().optional(),
    summary: z.string().optional(),
    content: z.any(),
    sourceUrl: z.string().optional(),
    publishedAt: z.date().optional(),
  }),
  relevance: z.number().min(0).max(1), // How relevant to current context
  sentiment: z.enum(["positive", "negative", "neutral"]).optional(),
})
export type WorldObservation = z.infer<typeof WorldObservation>

/**
 * Self observation types (SelfWatch).
 */
export const SelfObservationType = z.enum([
  "agent_behavior",
  "decision_log",
  "resource_usage",
  "error_pattern",
  "tool_invocation",
  "quality_metric",
  "latency",
  "cost",
])
export type SelfObservationType = z.infer<typeof SelfObservationType>

/**
 * Self observation event.
 */
export const SelfObservation = BaseObservation.extend({
  watcherType: z.literal("self"),
  type: SelfObservationType,
  agentId: z.string(),
  observation: z.object({
    action: z.string(),
    input: z.any().optional(),
    output: z.any().optional(),
    duration: z.number().optional(), // ms
    success: z.boolean(),
    error: z.string().optional(),
  }),
  quality: z.object({
    closeScore: z.number().min(0).max(10).optional(),
    accuracy: z.number().min(0).max(1).optional(),
    efficiency: z.number().min(0).max(1).optional(),
  }),
})
export type SelfObservation = z.infer<typeof SelfObservation>

/**
 * Meta observation types (MetaWatch).
 */
export const MetaObservationType = z.enum([
  "observation_quality",
  "system_health",
  "blind_spot",
  "consensus_drift",
  "watcher_status",
  "coverage_gap",
  "calibration",
])
export type MetaObservationType = z.infer<typeof MetaObservationType>

/**
 * Meta observation event (observing the observers).
 */
export const MetaObservation = BaseObservation.extend({
  watcherType: z.literal("meta"),
  type: MetaObservationType,
  targetWatcherId: z.string().optional(),
  assessment: z.object({
    health: z.enum(["healthy", "degraded", "failing"]),
    coverage: z.number().min(0).max(1), // How much is being observed
    accuracy: z.number().min(0).max(1), // How accurate are observations
    latency: z.number(), // Average observation latency in ms
  }),
  recommendations: z.array(z.string()).default([]),
  issues: z.array(z.object({
    type: z.string(),
    severity: z.enum(["low", "medium", "high", "critical"]),
    description: z.string(),
  })).default([]),
})
export type MetaObservation = z.infer<typeof MetaObservation>

/**
 * Union of all observation types.
 */
export const Observation = z.discriminatedUnion("watcherType", [
  CodeObservation,
  WorldObservation,
  SelfObservation,
  MetaObservation,
])
export type Observation = z.infer<typeof Observation>

// ─────────────────────────────────────────────────────────────────────────────
// World Model Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * World model snapshot - convergence of observations.
 */
export const WorldModel = z.object({
  /** Snapshot ID */
  id: z.string(),
  /** When this snapshot was created */
  timestamp: z.date(),
  /** Observations that contributed to this snapshot */
  observationIds: z.array(z.string()),
  /** Aggregated code state */
  code: z.object({
    lastCommit: z.string().optional(),
    buildStatus: z.enum(["passing", "failing", "unknown"]),
    testCoverage: z.number().optional(),
    techDebtLevel: z.enum(["low", "medium", "high"]).optional(),
    recentChanges: z.number(), // Count of recent changes
  }),
  /** Aggregated world state */
  world: z.object({
    marketSentiment: z.enum(["bullish", "bearish", "neutral"]).optional(),
    relevantNews: z.array(z.string()).default([]),
    externalRisks: z.array(z.string()).default([]),
    opportunities: z.array(z.string()).default([]),
  }),
  /** Aggregated self state */
  self: z.object({
    currentAgent: z.string().optional(),
    sessionHealth: z.enum(["healthy", "degraded", "critical"]),
    resourceUsage: z.object({
      tokens: z.number(),
      cost: z.number(),
      duration: z.number(),
    }),
    recentErrors: z.number(),
    decisionQuality: z.number().min(0).max(10).optional(),
  }),
  /** Meta state */
  meta: z.object({
    observerHealth: z.enum(["healthy", "degraded", "failing"]),
    coverageGaps: z.array(z.string()).default([]),
    consensusStrength: z.number().min(0).max(1),
  }),
  /** Overall confidence in this world model */
  confidence: z.number().min(0).max(1),
})
export type WorldModel = z.infer<typeof WorldModel>

// ─────────────────────────────────────────────────────────────────────────────
// Attention & Pattern Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Attention weights for prioritizing observations.
 */
export const AttentionWeights = z.object({
  /** Per-watcher type weights */
  byWatcher: z.object({
    code: z.number().min(0).max(1),
    world: z.number().min(0).max(1),
    self: z.number().min(0).max(1),
    meta: z.number().min(0).max(1),
  }),
  /** Per-observation type weights */
  byType: z.record(z.string(), z.number()),
  /** Time decay factor (how fast old observations lose weight) */
  timeDecay: z.number().min(0).max(1),
  /** Recency bias (prioritize recent observations) */
  recencyBias: z.number().min(0).max(1),
})
export type AttentionWeights = z.infer<typeof AttentionWeights>

/**
 * Emergent pattern detected from observations.
 */
export const EmergentPattern = z.object({
  /** Pattern ID */
  id: z.string(),
  /** Pattern name */
  name: z.string(),
  /** Description of what this pattern represents */
  description: z.string(),
  /** Pattern type */
  type: z.enum([
    "trend",
    "anomaly",
    "correlation",
    "cycle",
    "threshold",
    "sequence",
  ]),
  /** Observations that form this pattern */
  observationIds: z.array(z.string()),
  /** Confidence in this pattern */
  confidence: z.number().min(0).max(1),
  /** When pattern was first detected */
  detectedAt: z.date(),
  /** When pattern was last seen */
  lastSeenAt: z.date(),
  /** Pattern strength (how pronounced it is) */
  strength: z.number().min(0).max(1),
  /** Suggested actions based on this pattern */
  suggestedActions: z.array(z.string()).default([]),
})
export type EmergentPattern = z.infer<typeof EmergentPattern>

/**
 * Anomaly detected in observations.
 */
export const Anomaly = z.object({
  /** Anomaly ID */
  id: z.string(),
  /** Type of anomaly */
  type: z.enum([
    "outlier",
    "sudden_change",
    "missing_expected",
    "unexpected_presence",
    "timing",
    "frequency",
  ]),
  /** Description */
  description: z.string(),
  /** Severity */
  severity: z.enum(["low", "medium", "high", "critical"]),
  /** Related observations */
  observationIds: z.array(z.string()),
  /** When detected */
  detectedAt: z.date(),
  /** Is this anomaly confirmed or suspected */
  status: z.enum(["suspected", "confirmed", "dismissed"]),
  /** Confidence */
  confidence: z.number().min(0).max(1),
})
export type Anomaly = z.infer<typeof Anomaly>

/**
 * Opportunity identified from observations.
 */
export const Opportunity = z.object({
  /** Opportunity ID */
  id: z.string(),
  /** Type */
  type: z.enum([
    "optimization",
    "automation",
    "learning",
    "improvement",
    "market",
    "timing",
  ]),
  /** Description */
  description: z.string(),
  /** Potential impact */
  impact: z.enum(["low", "medium", "high"]),
  /** Time sensitivity */
  urgency: z.enum(["low", "medium", "high"]),
  /** Related observations */
  observationIds: z.array(z.string()),
  /** When detected */
  detectedAt: z.date(),
  /** Confidence */
  confidence: z.number().min(0).max(1),
  /** Suggested actions */
  suggestedActions: z.array(z.string()).default([]),
})
export type Opportunity = z.infer<typeof Opportunity>

// ─────────────────────────────────────────────────────────────────────────────
// Mode Control Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Operating mode for the system.
 */
export const OperatingMode = z.enum(["AUTO", "MANUAL", "HYBRID"])
export type OperatingMode = z.infer<typeof OperatingMode>

/**
 * CLOSE score thresholds for mode switching.
 */
export const CLOSEThresholds = z.object({
  /** Above this score, remain in AUTO mode */
  autoThreshold: z.number().min(0).max(10),
  /** Below this score, switch to MANUAL mode */
  manualThreshold: z.number().min(0).max(10),
  /** Risk threshold for escalation */
  riskThreshold: z.number().min(0).max(10),
  /** Optionality minimum before requiring human input */
  optionalityMinimum: z.number().min(0).max(10),
})
export type CLOSEThresholds = z.infer<typeof CLOSEThresholds>

/**
 * Mode decision result.
 */
export const ModeDecision = z.object({
  /** Current mode */
  currentMode: OperatingMode,
  /** Recommended mode */
  recommendedMode: OperatingMode,
  /** Should switch modes */
  shouldSwitch: z.boolean(),
  /** Reason for the decision */
  reason: z.string(),
  /** CLOSE scores that led to this decision */
  closeScores: z.object({
    convergence: z.number(),
    leverage: z.number(),
    optionality: z.number(),
    surplus: z.number(),
    evolution: z.number(),
    total: z.number(),
  }),
  /** Confidence in this decision */
  confidence: z.number().min(0).max(1),
  /** If escalation is needed */
  requiresEscalation: z.boolean(),
})
export type ModeDecision = z.infer<typeof ModeDecision>

// ─────────────────────────────────────────────────────────────────────────────
// Watcher Configuration Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Configuration for a watcher.
 */
export const WatcherConfig = z.object({
  /** Watcher ID */
  id: z.string(),
  /** Watcher type */
  type: z.enum(["code", "world", "self", "meta"]),
  /** Is watcher enabled */
  enabled: z.boolean().default(true),
  /** Observation interval in ms (0 = event-driven) */
  intervalMs: z.number().default(0),
  /** Filter patterns for this watcher */
  filters: z.array(z.string()).default([]),
  /** Priority (higher = more attention) */
  priority: z.number().min(0).max(10).default(5),
  /** Configuration specific to watcher type */
  options: z.record(z.string(), z.any()).default({}),
})
export type WatcherConfig = z.infer<typeof WatcherConfig>

/**
 * Watcher status.
 */
export const WatcherStatus = z.object({
  /** Watcher ID */
  id: z.string(),
  /** Watcher type */
  type: z.enum(["code", "world", "self", "meta"]),
  /** Is running */
  running: z.boolean(),
  /** Health status */
  health: z.enum(["healthy", "degraded", "failing", "stopped"]),
  /** Last observation time */
  lastObservation: z.date().optional(),
  /** Observation count */
  observationCount: z.number(),
  /** Error count */
  errorCount: z.number(),
  /** Average latency in ms */
  avgLatency: z.number(),
})
export type WatcherStatus = z.infer<typeof WatcherStatus>

// ─────────────────────────────────────────────────────────────────────────────
// Observer Network Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Observer Network configuration.
 */
export const ObserverNetworkConfig = z.object({
  /** Is the observer network enabled */
  enabled: z.boolean().default(false),
  /** Default operating mode */
  defaultMode: OperatingMode.default("HYBRID"),
  /** CLOSE thresholds */
  thresholds: CLOSEThresholds.default({
    autoThreshold: 7.0,
    manualThreshold: 4.0,
    riskThreshold: 8.0,
    optionalityMinimum: 3.0,
  }),
  /** Watcher configurations */
  watchers: z.array(WatcherConfig).default([]),
  /** Attention weights */
  attention: AttentionWeights.default({
    byWatcher: { code: 0.3, world: 0.2, self: 0.3, meta: 0.2 },
    byType: {},
    timeDecay: 0.1,
    recencyBias: 0.7,
  }),
  /** Maximum observations to keep in memory */
  maxObservations: z.number().default(1000),
  /** Consensus window in ms */
  consensusWindowMs: z.number().default(5000),
  /** Hysteresis factor to prevent mode flapping */
  hysteresisFactor: z.number().min(0).max(1).default(0.2),
})
export type ObserverNetworkConfig = z.infer<typeof ObserverNetworkConfig>

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Generate a unique observation ID.
 */
export function generateObservationId(watcherType: string): string {
  const timestamp = Date.now()
  const random = Math.random().toString(36).slice(2, 9)
  return `obs_${watcherType}_${timestamp}_${random}`
}

/**
 * Generate a unique pattern ID.
 */
export function generatePatternId(): string {
  const timestamp = Date.now()
  const random = Math.random().toString(36).slice(2, 9)
  return `pat_${timestamp}_${random}`
}

/**
 * Generate a unique anomaly ID.
 */
export function generateAnomalyId(): string {
  const timestamp = Date.now()
  const random = Math.random().toString(36).slice(2, 9)
  return `anom_${timestamp}_${random}`
}

/**
 * Generate a unique world model ID.
 */
export function generateWorldModelId(): string {
  const timestamp = Date.now()
  const random = Math.random().toString(36).slice(2, 9)
  return `wm_${timestamp}_${random}`
}

/**
 * Default CLOSE thresholds.
 */
export const DEFAULT_CLOSE_THRESHOLDS: CLOSEThresholds = {
  autoThreshold: 7.0,
  manualThreshold: 4.0,
  riskThreshold: 8.0,
  optionalityMinimum: 3.0,
}

/**
 * Default attention weights.
 */
export const DEFAULT_ATTENTION_WEIGHTS: AttentionWeights = {
  byWatcher: {
    code: 0.3,
    world: 0.2,
    self: 0.3,
    meta: 0.2,
  },
  byType: {},
  timeDecay: 0.1,
  recencyBias: 0.7,
}
