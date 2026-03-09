/**
 * Observer Network Events
 *
 * Event definitions for the Observer Network using the BusEvent pattern.
 * These events enable loose coupling between watchers, consensus engine,
 * mode controller, and responders.
 *
 * @module observer/events
 */

import { BusEvent } from "@/bus/bus-event"
import z from "zod"
import {
  CodeObservation,
  WorldObservation,
  SelfObservation,
  MetaObservation,
  WorldModel,
  EmergentPattern,
  Anomaly,
  Opportunity,
  OperatingMode,
  ModeDecision,
  WatcherStatus,
} from "./types"

/** GearPreset zod schema */
const GearPresetSchema = z.enum(["P", "N", "D", "S", "M"])

export namespace ObserverEvent {
  // ─────────────────────────────────────────────────────────────────────────────
  // Observation Events (from watchers)
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Code observation received from CodeWatch.
   */
  export const CodeObserved = BusEvent.define(
    "observer.code.observed",
    CodeObservation,
  )

  /**
   * World observation received from WorldWatch.
   */
  export const WorldObserved = BusEvent.define(
    "observer.world.observed",
    WorldObservation,
  )

  /**
   * Self observation received from SelfWatch.
   */
  export const SelfObserved = BusEvent.define(
    "observer.self.observed",
    SelfObservation,
  )

  /**
   * Meta observation received from MetaWatch.
   */
  export const MetaObserved = BusEvent.define(
    "observer.meta.observed",
    MetaObservation,
  )

  // ─────────────────────────────────────────────────────────────────────────────
  // Watcher Lifecycle Events
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Watcher started.
   */
  export const WatcherStarted = BusEvent.define(
    "observer.watcher.started",
    z.object({
      watcherId: z.string(),
      watcherType: z.enum(["code", "world", "self", "meta"]),
      config: z.record(z.string(), z.any()).optional(),
    }),
  )

  /**
   * Watcher stopped.
   */
  export const WatcherStopped = BusEvent.define(
    "observer.watcher.stopped",
    z.object({
      watcherId: z.string(),
      watcherType: z.enum(["code", "world", "self", "meta"]),
      reason: z.string().optional(),
    }),
  )

  /**
   * Watcher error occurred.
   */
  export const WatcherError = BusEvent.define(
    "observer.watcher.error",
    z.object({
      watcherId: z.string(),
      watcherType: z.enum(["code", "world", "self", "meta"]),
      error: z.string(),
      recoverable: z.boolean(),
    }),
  )

  /**
   * Watcher status changed.
   */
  export const WatcherStatusChanged = BusEvent.define(
    "observer.watcher.status_changed",
    WatcherStatus,
  )

  // ─────────────────────────────────────────────────────────────────────────────
  // Consensus Events
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * World model snapshot created.
   */
  export const WorldModelUpdated = BusEvent.define(
    "observer.consensus.world_model_updated",
    WorldModel,
  )

  /**
   * Emergent pattern detected.
   */
  export const PatternDetected = BusEvent.define(
    "observer.consensus.pattern_detected",
    EmergentPattern,
  )

  /**
   * Pattern no longer active.
   */
  export const PatternExpired = BusEvent.define(
    "observer.consensus.pattern_expired",
    z.object({
      patternId: z.string(),
      reason: z.string(),
    }),
  )

  /**
   * Anomaly detected.
   */
  export const AnomalyDetected = BusEvent.define(
    "observer.consensus.anomaly_detected",
    Anomaly,
  )

  /**
   * Anomaly status changed (confirmed/dismissed).
   */
  export const AnomalyStatusChanged = BusEvent.define(
    "observer.consensus.anomaly_status_changed",
    z.object({
      anomalyId: z.string(),
      previousStatus: z.enum(["suspected", "confirmed", "dismissed"]),
      newStatus: z.enum(["suspected", "confirmed", "dismissed"]),
      reason: z.string(),
    }),
  )

  /**
   * Opportunity identified.
   */
  export const OpportunityIdentified = BusEvent.define(
    "observer.consensus.opportunity_identified",
    Opportunity,
  )

  /**
   * Consensus strength changed significantly.
   */
  export const ConsensusStrengthChanged = BusEvent.define(
    "observer.consensus.strength_changed",
    z.object({
      previousStrength: z.number(),
      newStrength: z.number(),
      change: z.enum(["increased", "decreased"]),
      trigger: z.string().optional(),
    }),
  )

  // ─────────────────────────────────────────────────────────────────────────────
  // Mode Control Events
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Mode switch requested.
   */
  export const ModeSwitchRequested = BusEvent.define(
    "observer.mode.switch_requested",
    z.object({
      currentMode: OperatingMode,
      requestedMode: OperatingMode,
      reason: z.string(),
      trigger: z.enum(["automatic", "user", "escalation"]),
      decision: ModeDecision,
    }),
  )

  /**
   * Mode switched successfully.
   */
  export const ModeSwitched = BusEvent.define(
    "observer.mode.switched",
    z.object({
      previousMode: OperatingMode,
      newMode: OperatingMode,
      previousGear: GearPresetSchema.optional(),
      newGear: GearPresetSchema.optional(),
      reason: z.string(),
      timestamp: z.date(),
    }),
  )

  /**
   * Escalation to human required.
   */
  export const EscalationRequired = BusEvent.define(
    "observer.mode.escalation_required",
    z.object({
      reason: z.string(),
      severity: z.enum(["low", "medium", "high", "critical"]),
      context: z.record(z.string(), z.any()).optional(),
      deadline: z.date().optional(),
      suggestedActions: z.array(z.string()).default([]),
    }),
  )

  /**
   * Human decision received.
   */
  export const HumanDecisionReceived = BusEvent.define(
    "observer.mode.human_decision_received",
    z.object({
      escalationId: z.string(),
      decision: z.string(),
      rationale: z.string().optional(),
      timestamp: z.date(),
    }),
  )

  /**
   * Escalation created.
   */
  export const EscalationCreated = BusEvent.define(
    "observer.mode.escalation_created",
    z.object({
      escalationId: z.string(),
      priority: z.enum(["critical", "high", "medium", "low"]),
      title: z.string(),
      currentMode: OperatingMode,
      recommendedMode: OperatingMode,
      expiresAt: z.date(),
    }),
  )

  /**
   * Escalation resolved by human.
   */
  export const EscalationResolved = BusEvent.define(
    "observer.mode.escalation_resolved",
    z.object({
      escalationId: z.string(),
      decision: z.object({
        action: z.enum(["approve", "reject", "modify", "defer"]),
        chosenMode: OperatingMode.optional(),
        reason: z.string().optional(),
        timestamp: z.date(),
      }),
      resolvedAt: z.date(),
    }),
  )

  /**
   * Escalation expired without resolution.
   */
  export const EscalationExpired = BusEvent.define(
    "observer.mode.escalation_expired",
    z.object({
      escalationId: z.string(),
      expiredAt: z.date(),
    }),
  )

  /**
   * Mode evaluation completed.
   */
  export const ModeEvaluated = BusEvent.define(
    "observer.mode.evaluated",
    z.object({
      currentMode: OperatingMode,
      recommendedMode: OperatingMode,
      shouldSwitch: z.boolean(),
      reason: z.string(),
      closeScore: z.number(),
      risk: z.number(),
      confidence: z.number(),
    }),
  )

  // ─────────────────────────────────────────────────────────────────────────────
  // Response Events
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Notification sent.
   */
  export const NotificationSent = BusEvent.define(
    "observer.response.notification_sent",
    z.object({
      channel: z.string(),
      message: z.string(),
      priority: z.enum(["low", "medium", "high", "urgent"]),
      triggeredBy: z.string(), // Observation or pattern ID
    }),
  )

  /**
   * Analysis triggered.
   */
  export const AnalysisTriggered = BusEvent.define(
    "observer.response.analysis_triggered",
    z.object({
      analysisType: z.string(),
      agentUsed: z.string(),
      triggeredBy: z.string(),
      status: z.enum(["started", "completed", "failed"]),
      result: z.any().optional(),
    }),
  )

  /**
   * Execution triggered.
   */
  export const ExecutionTriggered = BusEvent.define(
    "observer.response.execution_triggered",
    z.object({
      executionType: z.string(),
      triggeredBy: z.string(),
      status: z.enum(["started", "completed", "failed"]),
      result: z.any().optional(),
    }),
  )

  /**
   * History recorded.
   */
  export const HistoryRecorded = BusEvent.define(
    "observer.response.history_recorded",
    z.object({
      eventType: z.string(),
      eventId: z.string(),
      storagePath: z.string().optional(),
    }),
  )

  // ─────────────────────────────────────────────────────────────────────────────
  // Network Lifecycle Events
  // ─────────────────────────────────────────────────────────────────────────────

  /**
   * Observer network started.
   */
  export const NetworkStarted = BusEvent.define(
    "observer.network.started",
    z.object({
      config: z.record(z.string(), z.any()),
      watcherCount: z.number(),
      mode: OperatingMode,
    }),
  )

  /**
   * Observer network stopped.
   */
  export const NetworkStopped = BusEvent.define(
    "observer.network.stopped",
    z.object({
      reason: z.string(),
      stats: z.object({
        uptime: z.number(),
        totalObservations: z.number(),
        patternsDetected: z.number(),
        anomaliesDetected: z.number(),
        modeSwitches: z.number(),
      }),
    }),
  )

  /**
   * Network health changed.
   */
  export const NetworkHealthChanged = BusEvent.define(
    "observer.network.health_changed",
    z.object({
      previousHealth: z.enum(["healthy", "degraded", "failing"]),
      newHealth: z.enum(["healthy", "degraded", "failing"]),
      reason: z.string(),
      watcherStatuses: z.array(WatcherStatus),
    }),
  )
}

// ─────────────────────────────────────────────────────────────────────────────
// Event Helpers
// ─────────────────────────────────────────────────────────────────────────────

const BusPromise = import("@/bus").then((m) => m.Bus)

export namespace ObserverEventHelper {
  /**
   * Subscribe to all observer events.
   */
  export async function subscribeAll(
    callback: (event: { type: string; properties: unknown }) => void,
  ) {
    const Bus = await BusPromise
    return Bus.subscribeAll((event: { type: string; properties: unknown }) => {
      if (event.type.startsWith("observer.")) {
        callback(event)
      }
    })
  }

  /**
   * Subscribe to observation events only.
   */
  export async function subscribeObservations(
    callback: (event: { type: string; properties: unknown }) => void,
  ) {
    const Bus = await BusPromise
    const unsubscribers = await Promise.all([
      Bus.subscribe(ObserverEvent.CodeObserved, callback),
      Bus.subscribe(ObserverEvent.WorldObserved, callback),
      Bus.subscribe(ObserverEvent.SelfObserved, callback),
      Bus.subscribe(ObserverEvent.MetaObserved, callback),
    ])
    return () => unsubscribers.forEach((u) => u())
  }

  /**
   * Subscribe to consensus events only.
   */
  export async function subscribeConsensus(
    callback: (event: { type: string; properties: unknown }) => void,
  ) {
    const Bus = await BusPromise
    return Bus.subscribeAll((event: { type: string; properties: unknown }) => {
      if (event.type.startsWith("observer.consensus.")) {
        callback(event)
      }
    })
  }

  /**
   * Subscribe to mode events only.
   */
  export async function subscribeMode(
    callback: (event: { type: string; properties: unknown }) => void,
  ) {
    const Bus = await BusPromise
    return Bus.subscribeAll((event: { type: string; properties: unknown }) => {
      if (event.type.startsWith("observer.mode.")) {
        callback(event)
      }
    })
  }
}
