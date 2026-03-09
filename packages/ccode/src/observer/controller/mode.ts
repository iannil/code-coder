/**
 * Mode Controller
 *
 * The central controller for AUTO/MANUAL/HYBRID mode switching.
 *
 * Implements the decision logic from 祝融说 philosophy:
 *
 * ```
 * IF (CLOSE.risk > threshold.critical) THEN
 *   SWITCH TO MANUAL + ESCALATE
 * ELSE IF (CLOSE.optionality < threshold.low) THEN
 *   SWITCH TO MANUAL (需要人类扩展选项)
 * ELSE IF (confidence > threshold.high AND risk < threshold.safe) THEN
 *   REMAIN AUTO
 * ELSE
 *   HYBRID (自动执行 + 事后确认)
 * ```
 *
 * @module observer/controller/mode
 */

import { Log } from "@/util/log"
import type { OperatingMode, Anomaly, Opportunity, GearPreset } from "../types"
import { operatingModeToGear, gearToOperatingMode } from "../types"
import type { ConsensusSnapshot } from "../consensus"
import { ObserverEvent } from "../events"
import {
  ThresholdManager,
  type ModeThresholds,
  type RiskTolerance,
} from "./thresholds"
import {
  CLOSEEvaluator,
  type CLOSEEvaluation,
  type CLOSEEvaluatorConfig,
} from "./close-evaluator"
import {
  EscalationManager,
  type Escalation,
  type HumanDecision,
  type EscalationConfig,
} from "./escalation"

const log = Log.create({ service: "observer.controller.mode" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ModeDecision {
  currentMode: OperatingMode
  recommendedMode: OperatingMode
  shouldSwitch: boolean
  reason: string
  evaluation: CLOSEEvaluation
  escalation?: Escalation
  timestamp: Date
}

export interface ModeControllerConfig {
  /** Initial operating mode */
  initialMode: OperatingMode
  /** Risk tolerance level */
  riskTolerance: RiskTolerance
  /** Custom threshold overrides */
  thresholds?: Partial<ModeThresholds>
  /** CLOSE evaluator config */
  closeConfig?: Partial<CLOSEEvaluatorConfig>
  /** Escalation config */
  escalationConfig?: Partial<EscalationConfig>
  /** Auto-apply mode decisions */
  autoApply: boolean
  /** Evaluation interval (ms) */
  evaluationIntervalMs: number
}

export interface ModeControllerStats {
  currentMode: OperatingMode
  currentGear: GearPreset
  modeSwitches: number
  escalations: number
  pendingEscalations: number
  lastEvaluation: CLOSEEvaluation | null
  lastDecision: ModeDecision | null
  uptime: number
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: ModeControllerConfig = {
  initialMode: "HYBRID",
  riskTolerance: "balanced",
  autoApply: true,
  evaluationIntervalMs: 10000, // 10 seconds
}

// ─────────────────────────────────────────────────────────────────────────────
// Mode Controller
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Controls operating mode based on observation consensus.
 */
export class ModeController {
  private config: ModeControllerConfig
  private thresholds: ThresholdManager
  private evaluator: CLOSEEvaluator
  private escalation: EscalationManager

  private currentMode: OperatingMode
  private currentGear: GearPreset
  private previousMode: OperatingMode | null = null
  private previousGear: GearPreset | null = null
  private running = false
  private evaluationTimer: ReturnType<typeof setInterval> | null = null
  private startTime: Date | null = null

  private modeSwitchCount = 0
  private escalationCount = 0
  private lastEvaluation: CLOSEEvaluation | null = null
  private lastDecision: ModeDecision | null = null
  private decisionHistory: ModeDecision[] = []
  private maxHistory = 100

  /** Callback for getting consensus snapshots */
  private snapshotProvider: (() => Promise<ConsensusSnapshot | null>) | null = null

  constructor(config: Partial<ModeControllerConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    this.currentMode = this.config.initialMode
    this.currentGear = operatingModeToGear(this.currentMode)
    this.thresholds = new ThresholdManager(this.config.riskTolerance)

    if (this.config.thresholds) {
      this.thresholds.update(this.config.thresholds)
    }

    this.evaluator = new CLOSEEvaluator(this.config.closeConfig)
    this.escalation = new EscalationManager(this.config.escalationConfig)
  }

  /**
   * Start the mode controller.
   */
  start(snapshotProvider: () => Promise<ConsensusSnapshot | null>): void {
    if (this.running) return

    this.snapshotProvider = snapshotProvider
    this.startTime = new Date()
    this.running = true

    this.escalation.start()

    // Start periodic evaluation
    if (this.config.evaluationIntervalMs > 0) {
      this.evaluationTimer = setInterval(async () => {
        await this.evaluate()
      }, this.config.evaluationIntervalMs)
    }

    log.info("Mode controller started", {
      mode: this.currentMode,
      riskTolerance: this.config.riskTolerance,
      autoApply: this.config.autoApply,
    })
  }

  /**
   * Stop the mode controller.
   */
  stop(): void {
    if (!this.running) return

    this.running = false

    if (this.evaluationTimer) {
      clearInterval(this.evaluationTimer)
      this.evaluationTimer = null
    }

    this.escalation.stop()

    log.info("Mode controller stopped", {
      modeSwitches: this.modeSwitchCount,
      escalations: this.escalationCount,
    })
  }

  /**
   * Evaluate current state and make mode decision.
   */
  async evaluate(): Promise<ModeDecision | null> {
    if (!this.snapshotProvider) {
      log.warn("No snapshot provider configured")
      return null
    }

    const snapshot = await this.snapshotProvider()
    if (!snapshot) {
      log.debug("No consensus snapshot available")
      return null
    }

    // Evaluate using CLOSE framework
    const evaluation = this.evaluator.evaluate(snapshot)
    this.lastEvaluation = evaluation

    // Determine recommended mode
    const decision = await this.decideMode(evaluation, snapshot)
    this.lastDecision = decision

    // Store in history
    this.decisionHistory.push(decision)
    if (this.decisionHistory.length > this.maxHistory) {
      this.decisionHistory.shift()
    }

    // Publish evaluation event
    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.ModeEvaluated, {
      currentMode: decision.currentMode,
      recommendedMode: decision.recommendedMode,
      shouldSwitch: decision.shouldSwitch,
      reason: decision.reason,
      closeScore: evaluation.total,
      risk: evaluation.risk,
      confidence: evaluation.confidence,
    })

    // Auto-apply if configured
    if (this.config.autoApply && decision.shouldSwitch && !decision.escalation) {
      await this.switchMode(decision.recommendedMode, decision.reason)
    }

    return decision
  }

  /**
   * Manually switch mode.
   */
  async switchMode(newMode: OperatingMode, reason?: string): Promise<void> {
    if (newMode === this.currentMode) return

    this.previousMode = this.currentMode
    this.previousGear = this.currentGear
    this.currentMode = newMode
    this.currentGear = operatingModeToGear(newMode)
    this.modeSwitchCount++

    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.ModeSwitched, {
      previousMode: this.previousMode,
      newMode: this.currentMode,
      previousGear: this.previousGear,
      newGear: this.currentGear,
      reason: reason ?? "Manual switch",
      timestamp: new Date(),
    })

    log.info("Mode switched", {
      from: this.previousMode,
      to: this.currentMode,
      gear: this.currentGear,
      reason,
    })
  }

  /**
   * Switch gear directly (preferred over switchMode).
   */
  async switchGear(newGear: GearPreset, reason?: string): Promise<void> {
    const newMode = gearToOperatingMode(newGear)
    if (newGear === this.currentGear) return

    this.previousMode = this.currentMode
    this.previousGear = this.currentGear
    this.currentGear = newGear
    this.currentMode = newMode
    this.modeSwitchCount++

    const BusModule = await import("@/bus")
    await BusModule.Bus.publish(ObserverEvent.ModeSwitched, {
      previousMode: this.previousMode,
      newMode: this.currentMode,
      previousGear: this.previousGear,
      newGear: this.currentGear,
      reason: reason ?? "Gear switch",
      timestamp: new Date(),
    })

    log.info("Gear switched", {
      from: this.previousGear,
      to: this.currentGear,
      mode: this.currentMode,
      reason,
    })
  }

  /**
   * Get current gear.
   */
  getGear(): GearPreset {
    return this.currentGear
  }

  /**
   * Handle human decision for an escalation.
   */
  async handleHumanDecision(escalationId: string, decision: HumanDecision): Promise<void> {
    const resolved = await this.escalation.resolve(escalationId, decision)
    if (!resolved) return

    // Apply mode change if approved
    if (decision.action === "approve" && decision.chosenMode) {
      await this.switchMode(decision.chosenMode, `Human decision: ${decision.reason ?? "approved"}`)
    } else if (decision.action === "modify" && decision.chosenMode) {
      await this.switchMode(decision.chosenMode, `Human override: ${decision.reason ?? "modified"}`)
    }
  }

  /**
   * Get current mode.
   */
  getMode(): OperatingMode {
    return this.currentMode
  }

  /**
   * Get controller statistics.
   */
  getStats(): ModeControllerStats {
    return {
      currentMode: this.currentMode,
      currentGear: this.currentGear,
      modeSwitches: this.modeSwitchCount,
      escalations: this.escalationCount,
      pendingEscalations: this.escalation.getPending().length,
      lastEvaluation: this.lastEvaluation,
      lastDecision: this.lastDecision,
      uptime: this.startTime ? Date.now() - this.startTime.getTime() : 0,
    }
  }

  /**
   * Get decision history.
   */
  getHistory(limit?: number): ModeDecision[] {
    return this.decisionHistory.slice(-(limit ?? 20))
  }

  /**
   * Get pending escalations.
   */
  getPendingEscalations(): Escalation[] {
    return this.escalation.getPending()
  }

  /**
   * Update risk tolerance.
   */
  setRiskTolerance(level: RiskTolerance): void {
    this.thresholds.setRiskTolerance(level)
    log.info("Risk tolerance updated", { level })
  }

  /**
   * Check if running.
   */
  isRunning(): boolean {
    return this.running
  }

  /**
   * Clear all state.
   */
  clear(): void {
    this.decisionHistory = []
    this.lastEvaluation = null
    this.lastDecision = null
    this.modeSwitchCount = 0
    this.escalationCount = 0
    this.previousMode = null
    this.previousGear = null
    this.escalation.clear()
    this.evaluator.clear()
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async decideMode(
    evaluation: CLOSEEvaluation,
    snapshot: ConsensusSnapshot,
  ): Promise<ModeDecision> {
    const thresholds = this.thresholds.get()
    let recommendedMode: OperatingMode = this.currentMode
    let reason = "No change needed"
    let escalation: Escalation | undefined

    // Decision logic from 祝融说
    // IF (CLOSE.risk > threshold.critical) THEN
    //   SWITCH TO MANUAL + ESCALATE
    if (this.thresholds.isCriticalRisk(evaluation.risk)) {
      recommendedMode = "MANUAL"
      reason = `Critical risk detected (${evaluation.risk.toFixed(2)} >= ${thresholds.criticalRisk})`

      // Create escalation
      escalation = await this.escalation.escalate(
        "critical",
        "Critical Risk Requires Human Decision",
        reason,
        {
          currentMode: this.currentMode,
          recommendedMode: "MANUAL",
          closeEvaluation: evaluation,
          anomalies: snapshot.anomalies,
          opportunities: snapshot.opportunities,
          trigger: "critical_risk",
        },
      )
      this.escalationCount++
    }
    // ELSE IF (CLOSE.optionality < threshold.low) THEN
    //   SWITCH TO MANUAL (需要人类扩展选项)
    else if (this.thresholds.isLowOptionality(evaluation.optionality.score)) {
      recommendedMode = "MANUAL"
      reason = `Low optionality (${evaluation.optionality.score.toFixed(2)} <= ${thresholds.lowOptionality}) - human needed to expand options`

      // High priority escalation for low optionality
      escalation = await this.escalation.escalate(
        "high",
        "Low Optionality - Human Input Needed",
        reason,
        {
          currentMode: this.currentMode,
          recommendedMode: "MANUAL",
          closeEvaluation: evaluation,
          anomalies: snapshot.anomalies,
          opportunities: snapshot.opportunities,
          trigger: "low_optionality",
        },
      )
      this.escalationCount++
    }
    // ELSE IF (confidence > threshold.high AND risk < threshold.safe) THEN
    //   REMAIN AUTO
    else if (
      this.thresholds.isHighConfidence(evaluation.confidence) &&
      this.thresholds.isSafeRisk(evaluation.risk) &&
      this.thresholds.qualifiesForAuto(evaluation.total)
    ) {
      recommendedMode = "AUTO"
      reason = `High confidence (${(evaluation.confidence * 100).toFixed(0)}%), safe risk (${evaluation.risk.toFixed(2)}), strong CLOSE (${evaluation.total.toFixed(2)})`
    }
    // ELSE IF (high risk but not critical)
    else if (this.thresholds.isHighRisk(evaluation.risk)) {
      recommendedMode = "HYBRID"
      reason = `High risk (${evaluation.risk.toFixed(2)} >= ${thresholds.highRisk}) - recommend human oversight`

      // Medium priority escalation
      if (this.currentMode === "AUTO") {
        escalation = await this.escalation.escalate(
          "medium",
          "High Risk Detected - Switching to Hybrid",
          reason,
          {
            currentMode: this.currentMode,
            recommendedMode: "HYBRID",
            closeEvaluation: evaluation,
            anomalies: snapshot.anomalies,
            opportunities: snapshot.opportunities,
            trigger: "high_risk",
          },
        )
        this.escalationCount++
      }
    }
    // ELSE IF (low confidence)
    else if (this.thresholds.isLowConfidence(evaluation.confidence)) {
      recommendedMode = "HYBRID"
      reason = `Low confidence (${(evaluation.confidence * 100).toFixed(0)}%) - recommend human oversight`
    }
    // ELSE HYBRID (自动执行 + 事后确认)
    else if (this.thresholds.qualifiesForHybrid(evaluation.total)) {
      recommendedMode = "HYBRID"
      reason = `Moderate conditions (CLOSE: ${evaluation.total.toFixed(2)}, Risk: ${evaluation.risk.toFixed(2)}) - auto-execute with confirmation`
    }
    // Default to MANUAL for uncertain conditions
    else {
      recommendedMode = "MANUAL"
      reason = `Below hybrid threshold (CLOSE: ${evaluation.total.toFixed(2)} < ${thresholds.hybridApprovalScore}) - human decision needed`
    }

    // Check hysteresis to prevent oscillation
    const shouldSwitch = recommendedMode !== this.currentMode && this.shouldSwitchWithHysteresis(evaluation)

    if (!shouldSwitch && recommendedMode !== this.currentMode) {
      reason += " (hysteresis prevented switch)"
    }

    return {
      currentMode: this.currentMode,
      recommendedMode,
      shouldSwitch,
      reason,
      evaluation,
      escalation,
      timestamp: new Date(),
    }
  }

  private shouldSwitchWithHysteresis(evaluation: CLOSEEvaluation): boolean {
    // If no previous evaluation, allow switch
    if (!this.lastEvaluation) return true

    // Check if the change is significant enough
    const totalChange = Math.abs(evaluation.total - this.lastEvaluation.total)
    const riskChange = Math.abs(evaluation.risk - this.lastEvaluation.risk)
    const confidenceChange = Math.abs(evaluation.confidence - this.lastEvaluation.confidence)

    // Allow switch if any metric changed significantly
    const thresholds = this.thresholds.get()
    return (
      totalChange > thresholds.hysteresis * 10 ||
      riskChange > thresholds.hysteresis * 10 ||
      confidenceChange > thresholds.hysteresis
    )
  }
}

/**
 * Create a mode controller.
 */
export function createModeController(config?: Partial<ModeControllerConfig>): ModeController {
  return new ModeController(config)
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let controllerInstance: ModeController | null = null

/**
 * Get the global mode controller instance.
 */
export function getModeController(config?: Partial<ModeControllerConfig>): ModeController {
  if (!controllerInstance) {
    controllerInstance = new ModeController(config)
  }
  return controllerInstance
}

/**
 * Reset the mode controller (for testing).
 */
export function resetModeController(): void {
  if (controllerInstance) {
    controllerInstance.stop()
    controllerInstance.clear()
    controllerInstance = null
  }
}
