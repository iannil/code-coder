/**
 * Mode System Stub
 * @deprecated This module has been moved to Rust. Use the Rust API instead.
 */

export interface Mode {
  id: string
  name: string
  description: string
  primaryAgent: string
  alternativePrimaries: string[]
  capabilities: string[]
  color?: string
  icon?: string
}

export const MODES: Record<string, Mode> = {}
export const DEFAULT_MODE = "build"

export function getMode(_modeId: string): Mode | undefined {
  return undefined
}

export function getDefaultMode(): Mode {
  return { id: "build", name: "Development", description: "", primaryAgent: "build", alternativePrimaries: [], capabilities: [] }
}

export function listModes(): Mode[] {
  return []
}

export function parseModeCapability(_input: string): { mode: string; capability: string } | null {
  return null
}

export function validateCapability(_modeId: string, _capabilityName: string): boolean {
  return false
}
