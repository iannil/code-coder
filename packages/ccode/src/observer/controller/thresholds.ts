/**
 * Mode Controller Thresholds
 *
 * Defines thresholds for AUTO/MANUAL/HYBRID mode switching.
 *
 * @module observer/controller/thresholds
 */

import { z } from "zod"

// ─────────────────────────────────────────────────────────────────────────────
// Threshold Schemas
// ─────────────────────────────────────────────────────────────────────────────

export const ModeThresholdsSchema = z.object({
  /**
   * When risk exceeds this, force MANUAL mode and escalate.
   * Range: 0-10, higher = more risk tolerance.
   */
  criticalRisk: z.number().min(0).max(10).default(8.0),

  /**
   * When risk exceeds this, recommend HYBRID mode.
   * Range: 0-10.
   */
  highRisk: z.number().min(0).max(10).default(6.0),

  /**
   * Below this risk level, AUTO mode is safe.
   * Range: 0-10.
   */
  safeRisk: z.number().min(0).max(10).default(3.0),

  /**
   * Below this optionality, MANUAL mode is recommended.
   * Low optionality = few choices = need human to expand options.
   * Range: 0-10.
   */
  lowOptionality: z.number().min(0).max(10).default(3.0),

  /**
   * Above this confidence, AUTO mode is safe.
   * Range: 0-1.
   */
  highConfidence: z.number().min(0).max(1).default(0.8),

  /**
   * Below this confidence, MANUAL mode is recommended.
   * Range: 0-1.
   */
  lowConfidence: z.number().min(0).max(1).default(0.4),

  /**
   * Minimum CLOSE score for AUTO mode.
   * Range: 0-10.
   */
  autoApprovalScore: z.number().min(0).max(10).default(7.0),

  /**
   * Minimum CLOSE score for HYBRID mode.
   * Range: 0-10.
   */
  hybridApprovalScore: z.number().min(0).max(10).default(5.0),

  /**
   * Hysteresis threshold to prevent mode oscillation.
   * Only switch mode if change exceeds this.
   * Range: 0-1.
   */
  hysteresis: z.number().min(0).max(1).default(0.1),
})

export type ModeThresholds = z.infer<typeof ModeThresholdsSchema>

// ─────────────────────────────────────────────────────────────────────────────
// Preset Configurations
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Risk tolerance levels for different operational contexts.
 */
export type RiskTolerance = "conservative" | "balanced" | "aggressive" | "autonomous"

/**
 * Preset threshold configurations for different risk tolerances.
 */
export const THRESHOLD_PRESETS: Record<RiskTolerance, ModeThresholds> = {
  /**
   * Conservative: Prefer MANUAL mode, low risk tolerance.
   * Good for: Production deployments, financial operations.
   */
  conservative: {
    criticalRisk: 6.0,
    highRisk: 4.0,
    safeRisk: 2.0,
    lowOptionality: 4.0,
    highConfidence: 0.9,
    lowConfidence: 0.6,
    autoApprovalScore: 8.0,
    hybridApprovalScore: 6.0,
    hysteresis: 0.15,
  },

  /**
   * Balanced: Default settings, moderate risk tolerance.
   * Good for: Development, testing.
   */
  balanced: {
    criticalRisk: 8.0,
    highRisk: 6.0,
    safeRisk: 3.0,
    lowOptionality: 3.0,
    highConfidence: 0.8,
    lowConfidence: 0.4,
    autoApprovalScore: 7.0,
    hybridApprovalScore: 5.0,
    hysteresis: 0.1,
  },

  /**
   * Aggressive: Prefer AUTO mode, high risk tolerance.
   * Good for: Exploration, non-critical tasks.
   */
  aggressive: {
    criticalRisk: 9.0,
    highRisk: 7.0,
    safeRisk: 4.0,
    lowOptionality: 2.0,
    highConfidence: 0.7,
    lowConfidence: 0.3,
    autoApprovalScore: 6.0,
    hybridApprovalScore: 4.0,
    hysteresis: 0.05,
  },

  /**
   * Autonomous: Maximum AUTO mode, minimal escalation.
   * Good for: Trusted environments, low-stakes operations.
   */
  autonomous: {
    criticalRisk: 9.5,
    highRisk: 8.0,
    safeRisk: 5.0,
    lowOptionality: 1.5,
    highConfidence: 0.6,
    lowConfidence: 0.2,
    autoApprovalScore: 5.0,
    hybridApprovalScore: 3.0,
    hysteresis: 0.02,
  },
}

// ─────────────────────────────────────────────────────────────────────────────
// Threshold Manager
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Manages mode switching thresholds.
 */
export class ThresholdManager {
  private thresholds: ModeThresholds
  private riskTolerance: RiskTolerance

  constructor(riskTolerance: RiskTolerance = "balanced") {
    this.riskTolerance = riskTolerance
    this.thresholds = { ...THRESHOLD_PRESETS[riskTolerance] }
  }

  /**
   * Get current thresholds.
   */
  get(): ModeThresholds {
    return { ...this.thresholds }
  }

  /**
   * Get current risk tolerance level.
   */
  getRiskTolerance(): RiskTolerance {
    return this.riskTolerance
  }

  /**
   * Set risk tolerance level (applies preset).
   */
  setRiskTolerance(level: RiskTolerance): void {
    this.riskTolerance = level
    this.thresholds = { ...THRESHOLD_PRESETS[level] }
  }

  /**
   * Update individual thresholds.
   */
  update(partial: Partial<ModeThresholds>): void {
    const merged = { ...this.thresholds, ...partial }
    this.thresholds = ModeThresholdsSchema.parse(merged)
  }

  /**
   * Check if risk exceeds critical threshold.
   */
  isCriticalRisk(risk: number): boolean {
    return risk >= this.thresholds.criticalRisk
  }

  /**
   * Check if risk exceeds high threshold.
   */
  isHighRisk(risk: number): boolean {
    return risk >= this.thresholds.highRisk
  }

  /**
   * Check if risk is safe for AUTO mode.
   */
  isSafeRisk(risk: number): boolean {
    return risk <= this.thresholds.safeRisk
  }

  /**
   * Check if optionality is low (need human input).
   */
  isLowOptionality(optionality: number): boolean {
    return optionality <= this.thresholds.lowOptionality
  }

  /**
   * Check if confidence is high enough for AUTO mode.
   */
  isHighConfidence(confidence: number): boolean {
    return confidence >= this.thresholds.highConfidence
  }

  /**
   * Check if confidence is too low.
   */
  isLowConfidence(confidence: number): boolean {
    return confidence <= this.thresholds.lowConfidence
  }

  /**
   * Check if CLOSE score qualifies for AUTO mode.
   */
  qualifiesForAuto(closeScore: number): boolean {
    return closeScore >= this.thresholds.autoApprovalScore
  }

  /**
   * Check if CLOSE score qualifies for HYBRID mode.
   */
  qualifiesForHybrid(closeScore: number): boolean {
    return closeScore >= this.thresholds.hybridApprovalScore
  }

  /**
   * Check if change exceeds hysteresis threshold.
   */
  exceedsHysteresis(previousValue: number, newValue: number): boolean {
    return Math.abs(newValue - previousValue) >= this.thresholds.hysteresis
  }
}

/**
 * Create a threshold manager.
 */
export function createThresholdManager(riskTolerance?: RiskTolerance): ThresholdManager {
  return new ThresholdManager(riskTolerance)
}
