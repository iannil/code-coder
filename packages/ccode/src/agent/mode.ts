/**
 * Agent Mode System
 *
 * Provides a simplified interface for users to interact with agents by grouping
 * them into logical "modes". Each mode represents a primary use case and exposes
 * relevant subagents as "capabilities".
 *
 * ## Modes
 *
 * - `@build` (default) - Software development mode
 * - `@writer` - Long-form content creation mode
 * - `@decision` - Decision-making and advisory mode
 *
 * ## Usage
 *
 * ```bash
 * # Enter a mode
 * bun dev              # Uses default mode (build)
 * bun dev -m writer    # Uses writer mode
 * bun dev -m decision  # Uses decision mode
 *
 * # Access capabilities within a mode
 * bun dev @build:security-review
 * bun dev @decision:macro
 * ```
 */

import z from "zod"

/**
 * Mode definition schema
 */
export const ModeSchema = z.object({
  /** Mode identifier (e.g., "build", "writer", "decision") */
  id: z.string(),
  /** Human-readable name */
  name: z.string(),
  /** Description of the mode's purpose */
  description: z.string(),
  /** Primary agent for this mode */
  primaryAgent: z.string(),
  /** Alternative primary agents (e.g., plan mode under build) */
  alternativePrimaries: z.array(z.string()).default([]),
  /** Subagents available as capabilities in this mode */
  capabilities: z.array(z.string()),
  /** Color for TUI display */
  color: z.string().optional(),
  /** Icon or emoji for the mode */
  icon: z.string().optional(),
})

export type Mode = z.infer<typeof ModeSchema>

/**
 * Mode definitions
 */
export const MODES: Record<string, Mode> = {
  build: {
    id: "build",
    name: "Development",
    description: "Software development, code review, architecture, and testing",
    primaryAgent: "build",
    alternativePrimaries: ["plan", "autonomous"],
    capabilities: [
      // Code quality
      "code-reviewer",
      "security-reviewer",
      "tdd-guide",
      "architect",
      // Exploration
      "explore",
      "general",
      // Reverse engineering
      "code-reverse",
      "jar-code-reverse",
      // Verification
      "verifier",
      // Product
      "prd-generator",
      "feasibility-assess",
    ],
    color: "green",
    icon: "🔨",
  },

  writer: {
    id: "writer",
    name: "Writing",
    description: "Long-form content creation, editing, and proofreading",
    primaryAgent: "writer",
    alternativePrimaries: [],
    capabilities: [
      // Content expansion (unified: supports fiction/nonfiction auto-detection)
      "expander",
      // Quality assurance
      "proofreader",
      "verifier",
    ],
    color: "blue",
    icon: "✍️",
  },

  decision: {
    id: "decision",
    name: "Decision",
    description: "Decision-making, analysis, and strategic advisory based on 祝融说 philosophy",
    primaryAgent: "decision",
    alternativePrimaries: ["observer"],
    capabilities: [
      // Philosophy
      "observer",
      // Economics & Trading
      "macro",
      "trader",
      "value-analyst",
      // Business
      "picker",
      "miniproduct",
      // Technology
      "ai-engineer",
      "synton-assistant",
    ],
    color: "magenta",
    icon: "🎯",
  },
}

/**
 * Default mode
 */
export const DEFAULT_MODE = "build"

/**
 * Get a mode by ID
 */
export function getMode(modeId: string): Mode | undefined {
  return MODES[modeId]
}

/**
 * Get the default mode
 */
export function getDefaultMode(): Mode {
  return MODES[DEFAULT_MODE]
}

/**
 * List all available modes
 */
export function listModes(): Mode[] {
  return Object.values(MODES)
}

/**
 * Check if an agent belongs to a mode (as primary or capability)
 */
export function agentBelongsToMode(agentName: string, modeId: string): boolean {
  const mode = MODES[modeId]
  if (!mode) return false

  return (
    mode.primaryAgent === agentName ||
    mode.alternativePrimaries.includes(agentName) ||
    mode.capabilities.includes(agentName)
  )
}

/**
 * Find which mode(s) an agent belongs to
 */
export function findModesForAgent(agentName: string): string[] {
  return Object.keys(MODES).filter((modeId) => agentBelongsToMode(agentName, modeId))
}

/**
 * Get all agents in a mode (primary + capabilities)
 */
export function getAgentsInMode(modeId: string): string[] {
  const mode = MODES[modeId]
  if (!mode) return []

  return [mode.primaryAgent, ...mode.alternativePrimaries, ...mode.capabilities]
}

/**
 * Parse mode:capability notation (e.g., "@build:security-review")
 */
export function parseModeCapability(input: string): { mode: string; capability: string } | null {
  // Handle @mode:capability format
  const match = input.match(/^@?(\w+):(\w+(?:-\w+)*)$/)
  if (!match) return null

  const [, mode, capability] = match
  return { mode, capability }
}

/**
 * Validate that a capability is available in a mode
 */
export function validateCapability(modeId: string, capabilityName: string): boolean {
  const mode = MODES[modeId]
  if (!mode) return false

  return mode.capabilities.includes(capabilityName)
}
