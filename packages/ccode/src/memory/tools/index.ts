/**
 * Dynamic Tool Registry Module
 *
 * Provides a unified API for the dynamic tool library that stores
 * and retrieves tools learned from successful code executions.
 *
 * Core capabilities:
 * - Tool registration and management
 * - Semantic search and discovery
 * - Learning from successful executions
 * - Usage tracking and statistics
 *
 * Part of Phase 12: Dynamic Tool Library
 */

import { Log } from "@/util/log"
import { ToolTypes } from "./types"
import { ToolRegistry } from "./registry"
import { ToolSearch } from "./search"
import { ToolLearner } from "./learner"
import { LLMAbstractor } from "./llm-abstractor"

const log = Log.create({ service: "memory.tools" })

// ============================================================================
// Re-exports
// ============================================================================

export { ToolTypes } from "./types"
export { ToolRegistry } from "./registry"
export { ToolSearch } from "./search"
export { ToolLearner } from "./learner"
export { LLMAbstractor } from "./llm-abstractor"

// ============================================================================
// Unified API
// ============================================================================

export namespace DynamicToolRegistry {
  // ============================================================================
  // Tool Registration
  // ============================================================================

  /**
   * Register a new dynamic tool
   */
  export async function register(input: ToolTypes.CreateToolInput): Promise<ToolTypes.DynamicTool> {
    const tool = await ToolRegistry.register(input)
    await ToolSearch.updateEmbedding(tool.id)
    return tool
  }

  /**
   * Get a tool by ID
   */
  export async function get(id: string): Promise<ToolTypes.DynamicTool | undefined> {
    return ToolRegistry.get(id)
  }

  /**
   * Update a tool
   */
  export async function update(
    id: string,
    updates: ToolTypes.UpdateToolInput,
  ): Promise<ToolTypes.DynamicTool> {
    const tool = await ToolRegistry.update(id, updates)
    await ToolSearch.updateEmbedding(tool.id)
    return tool
  }

  /**
   * Remove a tool
   */
  export async function remove(id: string): Promise<boolean> {
    return ToolRegistry.remove(id)
  }

  /**
   * List all tools with optional filtering
   */
  export async function list(options?: {
    tags?: string[]
    language?: ToolTypes.DynamicTool["language"]
    limit?: number
    sortBy?: "name" | "usage" | "created" | "updated"
  }): Promise<ToolTypes.DynamicTool[]> {
    return ToolRegistry.list(options)
  }

  // ============================================================================
  // Tool Discovery
  // ============================================================================

  /**
   * Search for tools matching a query
   *
   * Uses hybrid retrieval combining:
   * - Semantic similarity (vector search)
   * - Keyword matching
   * - Usage statistics
   */
  export async function search(
    query: string,
    options?: Partial<ToolTypes.SearchOptions>,
  ): Promise<ToolTypes.ScoredTool[]> {
    return ToolSearch.search(query, options)
  }

  /**
   * Find similar tools to a given tool
   */
  export async function findSimilar(
    toolId: string,
    options?: { limit?: number; minScore?: number },
  ): Promise<ToolTypes.ScoredTool[]> {
    return ToolSearch.findSimilar(toolId, options)
  }

  /**
   * Get recommended tools based on usage patterns
   */
  export async function getRecommendations(options?: {
    limit?: number
  }): Promise<ToolTypes.DynamicTool[]> {
    return ToolSearch.getRecommendations(options)
  }

  // ============================================================================
  // Learning from Execution
  // ============================================================================

  /**
   * Learn a new tool from a successful execution
   *
   * This is the primary way tools get added to the registry.
   * When the sandbox executes code successfully, call this method
   * to potentially create a reusable tool from it.
   *
   * @returns The learned tool, or null if learning was skipped
   */
  export async function learnFromExecution(
    execution: ToolTypes.ExecutionRecord,
    options?: Partial<ToolLearner.LearnerConfig>,
  ): Promise<ToolTypes.DynamicTool | null> {
    return ToolLearner.learnFromExecution(execution, options)
  }

  /**
   * Batch learn from multiple executions
   */
  export async function learnFromExecutions(
    executions: ToolTypes.ExecutionRecord[],
    options?: Partial<ToolLearner.LearnerConfig>,
  ): Promise<{ learned: number; skipped: number; duplicates: number }> {
    return ToolLearner.learnFromExecutions(executions, options)
  }

  // ============================================================================
  // Tool Execution
  // ============================================================================

  /**
   * Get tool code with parameters substituted
   *
   * Use this to prepare a tool for execution.
   */
  export async function getToolCode(
    id: string,
    params: Record<string, unknown>,
  ): Promise<string> {
    return ToolRegistry.getToolCode(id, params)
  }

  /**
   * Record tool usage (called after execution)
   */
  export async function recordUsage(
    id: string,
    success: boolean,
    durationMs: number,
  ): Promise<void> {
    return ToolRegistry.recordUsage(id, success, durationMs)
  }

  // ============================================================================
  // Statistics
  // ============================================================================

  /**
   * Get registry statistics
   */
  export async function getStats(): Promise<ToolTypes.RegistryStats> {
    return ToolRegistry.getStats()
  }

  // ============================================================================
  // Maintenance
  // ============================================================================

  /**
   * Clean up unused tools
   */
  export async function cleanup(maxAgeDays = 90): Promise<number> {
    return ToolRegistry.cleanup(maxAgeDays)
  }

  /**
   * Rebuild search embeddings
   */
  export async function rebuildEmbeddings(): Promise<number> {
    return ToolSearch.rebuildEmbeddings()
  }

  /**
   * Clear all tools and embeddings
   */
  export async function clear(): Promise<void> {
    await ToolRegistry.clear()
    await ToolSearch.clearEmbeddings()
    log.info("Dynamic tool registry cleared")
  }

  /**
   * Invalidate all cached data
   */
  export async function invalidate(): Promise<void> {
    await ToolRegistry.invalidate()
    await ToolSearch.invalidate()
  }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/**
 * Quick search for tools
 */
export async function searchTools(query: string, limit = 5): Promise<ToolTypes.DynamicTool[]> {
  const results = await DynamicToolRegistry.search(query, { limit })
  return results.map((r) => r.tool)
}

/**
 * Learn from a successful sandbox execution
 */
export async function learnTool(
  code: string,
  language: "python" | "nodejs" | "bash",
  task: string,
  output: string,
): Promise<ToolTypes.DynamicTool | null> {
  return DynamicToolRegistry.learnFromExecution({
    code,
    language,
    task,
    output,
    exitCode: 0,
  })
}

/**
 * Execute a tool by ID with given parameters
 */
export async function getToolForExecution(
  toolId: string,
  params: Record<string, unknown> = {},
): Promise<{ tool: ToolTypes.DynamicTool; code: string } | null> {
  const tool = await DynamicToolRegistry.get(toolId)
  if (!tool) return null

  const code = await DynamicToolRegistry.getToolCode(toolId, params)
  return { tool, code }
}
