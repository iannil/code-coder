import type { Tool } from "./tool"

export type ToolRegistry = Record<string, Tool>

// Default empty registry value for runtime use
export const ToolRegistry: ToolRegistry = {}
