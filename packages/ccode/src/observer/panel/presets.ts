/**
 * Gear Presets for the Dial Panel
 *
 * @module observer/panel/presets
 */

import type { GearPreset, DialValues } from "../dial"

/**
 * Detailed gear preset with metadata.
 */
export interface GearPresetDetail {
  /** Gear identifier */
  gear: GearPreset
  /** Short name */
  name: string
  /** Full description */
  description: string
  /** Dial values */
  dials: DialValues
  /** Icon/emoji for display */
  icon: string
  /** Color theme (CSS color) */
  color: string
  /** Scenarios where this gear is appropriate */
  scenarios: string[]
}

/**
 * Full preset details for each gear.
 */
export const GEAR_PRESET_DETAILS: Record<GearPreset, GearPresetDetail> = {
  P: {
    gear: "P",
    name: "Park",
    description: "System inactive, no resource consumption",
    dials: { observe: 0, decide: 0, act: 0 },
    icon: "🅿️",
    color: "#6b7280", // gray
    scenarios: [
      "System maintenance",
      "Explicit user pause",
      "Resource conservation",
    ],
  },
  N: {
    gear: "N",
    name: "Neutral",
    description: "Observe and record only, no intervention",
    dials: { observe: 50, decide: 0, act: 0 },
    icon: "🔵",
    color: "#3b82f6", // blue
    scenarios: [
      "Monitoring without action",
      "Learning phase",
      "Data collection",
      "Post-incident review",
    ],
  },
  D: {
    gear: "D",
    name: "Drive",
    description: "Balanced autonomy for daily operation",
    dials: { observe: 70, decide: 60, act: 40 },
    icon: "🟢",
    color: "#22c55e", // green
    scenarios: [
      "Normal development workflow",
      "Routine tasks",
      "Standard operation",
      "Daily coding sessions",
    ],
  },
  S: {
    gear: "S",
    name: "Sport",
    description: "High autonomy, aggressive mode",
    dials: { observe: 90, decide: 80, act: 70 },
    icon: "🔴",
    color: "#ef4444", // red
    scenarios: [
      "Time-critical tasks",
      "Automated pipelines",
      "Trusted environments",
      "Batch processing",
    ],
  },
  M: {
    gear: "M",
    name: "Manual",
    description: "Full manual control over each dial",
    dials: { observe: 50, decide: 50, act: 50 },
    icon: "⚙️",
    color: "#8b5cf6", // purple
    scenarios: [
      "Custom configurations",
      "Experimental settings",
      "Fine-tuning behavior",
      "Special requirements",
    ],
  },
}

/**
 * Get preset details for a gear.
 */
export function getGearPresetDetail(gear: GearPreset): GearPresetDetail {
  return GEAR_PRESET_DETAILS[gear]
}

/**
 * Get all gear presets in order.
 */
export function getAllGearPresets(): GearPresetDetail[] {
  return ["P", "N", "D", "S", "M"].map((g) => GEAR_PRESET_DETAILS[g as GearPreset])
}

/**
 * Suggest a gear based on context.
 */
export function suggestGear(context: {
  isTimeCritical?: boolean
  isTrustedEnvironment?: boolean
  requiresHumanReview?: boolean
  isLearningMode?: boolean
  isMaintenanceMode?: boolean
}): GearPreset {
  if (context.isMaintenanceMode) return "P"
  if (context.isLearningMode) return "N"
  if (context.requiresHumanReview) return "D"
  if (context.isTimeCritical && context.isTrustedEnvironment) return "S"
  return "D" // Default to Drive
}

/**
 * Get risk level for a gear.
 */
export function getGearRiskLevel(gear: GearPreset): "none" | "low" | "medium" | "high" {
  switch (gear) {
    case "P":
      return "none"
    case "N":
      return "low"
    case "D":
      return "medium"
    case "S":
      return "high"
    case "M":
      return "medium" // Depends on manual settings
  }
}

/**
 * Validate gear transition (some transitions may require confirmation).
 */
export function validateGearTransition(from: GearPreset, to: GearPreset): {
  allowed: boolean
  requiresConfirmation: boolean
  reason?: string
} {
  // All transitions are allowed
  // But upgrading to Sport requires confirmation
  if (to === "S" && from !== "S") {
    return {
      allowed: true,
      requiresConfirmation: true,
      reason: "Sport mode enables high autonomy. Confirm this is intentional.",
    }
  }

  return { allowed: true, requiresConfirmation: false }
}
