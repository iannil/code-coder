/**
 * Tool Registry Stub
 * @deprecated Tools are now implemented in Rust.
 */

import type { z } from "zod"
import type { Tool } from "./tool"

export interface ToolDefinition {
  id: string
  description: string
  parameters?: z.ZodType
  execute: (
    args: unknown,
    ctx: Tool.Context,
  ) => Promise<{
    title: string
    metadata: Record<string, unknown>
    output: string
  }>
}

export type ToolRegistryType = Record<string, Tool>

// Namespace with static methods for compatibility
export const ToolRegistry = {
  /** Get all tool IDs */
  ids(): string[] {
    return []
  },

  /** Get tools for a model and agent */
  tools(
    _model: { providerID: string; modelID: string } | undefined,
    _agent: unknown
  ): Promise<ToolDefinition[]> {
    return Promise.resolve([])
  },

  /** Get a specific tool by ID */
  get(_id: string): Tool | undefined {
    return undefined
  },
}

// Type alias for backwards compatibility
export type ToolRegistry = ToolRegistryType
