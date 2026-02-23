/**
 * Dynamic Tool Registry
 *
 * Provides persistent storage and management for dynamically learned tools.
 * Tools are stored per-project with version control and statistics tracking.
 *
 * Part of Phase 12: Dynamic Tool Library
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { ToolTypes } from "./types"

const log = Log.create({ service: "memory.tools.registry" })

export namespace ToolRegistry {
  // ============================================================================
  // Constants
  // ============================================================================

  const STORAGE_PREFIX = ["memory", "tools", "registry"]

  // ============================================================================
  // Core CRUD Operations
  // ============================================================================

  /**
   * Register a new dynamic tool
   */
  export async function register(input: ToolTypes.CreateToolInput): Promise<ToolTypes.DynamicTool> {
    const validated = ToolTypes.CreateToolInput.parse(input)
    const projectID = Instance.project.id
    const now = Date.now()

    const id = generateToolId(validated.name, now)

    const tool: ToolTypes.DynamicTool = {
      id,
      name: validated.name,
      description: validated.description,
      tags: validated.tags,
      code: validated.code,
      language: validated.language,
      parameters: validated.parameters,
      examples: validated.examples,
      metadata: {
        createdAt: now,
        updatedAt: now,
        createdBy: validated.createdBy,
        sourceTask: validated.sourceTask,
        version: 1,
      },
      stats: {
        usageCount: 0,
        successCount: 0,
        failureCount: 0,
        lastUsedAt: null,
        averageExecutionTime: 0,
      },
    }

    await Storage.write([...STORAGE_PREFIX, projectID, id], tool)
    log.info("Tool registered", { id, name: tool.name, language: tool.language })

    return tool
  }

  /**
   * Get a tool by ID
   */
  export async function get(id: string): Promise<ToolTypes.DynamicTool | undefined> {
    const projectID = Instance.project.id

    try {
      return await Storage.read<ToolTypes.DynamicTool>([...STORAGE_PREFIX, projectID, id])
    } catch {
      return undefined
    }
  }

  /**
   * Update an existing tool
   */
  export async function update(id: string, updates: ToolTypes.UpdateToolInput): Promise<ToolTypes.DynamicTool> {
    const validated = ToolTypes.UpdateToolInput.parse(updates)
    const projectID = Instance.project.id

    const existing = await get(id)
    if (!existing) {
      throw new Error(`Tool not found: ${id}`)
    }

    const updated: ToolTypes.DynamicTool = {
      ...existing,
      ...validated,
      metadata: {
        ...existing.metadata,
        updatedAt: Date.now(),
        version: existing.metadata.version + 1,
      },
    }

    await Storage.write([...STORAGE_PREFIX, projectID, id], updated)
    log.info("Tool updated", { id, version: updated.metadata.version })

    return updated
  }

  /**
   * Delete a tool
   */
  export async function remove(id: string): Promise<boolean> {
    const projectID = Instance.project.id

    try {
      await Storage.remove([...STORAGE_PREFIX, projectID, id])
      log.info("Tool removed", { id })
      return true
    } catch {
      return false
    }
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
    const projectID = Instance.project.id
    const tools: ToolTypes.DynamicTool[] = []

    try {
      const keys = await Storage.list([...STORAGE_PREFIX, projectID])

      for (const key of keys) {
        try {
          const tool = await Storage.read<ToolTypes.DynamicTool>(key)
          if (!ToolTypes.DynamicTool.safeParse(tool).success) continue

          // Apply filters
          if (options?.language && tool.language !== options.language) continue
          if (options?.tags?.length) {
            const hasAllTags = options.tags.every((tag) => tool.tags.includes(tag))
            if (!hasAllTags) continue
          }

          tools.push(tool)
        } catch {
          // Skip invalid entries
        }
      }

      // Sort results
      const sortBy = options?.sortBy ?? "created"
      tools.sort((a, b) => {
        switch (sortBy) {
          case "name":
            return a.name.localeCompare(b.name)
          case "usage":
            return b.stats.usageCount - a.stats.usageCount
          case "updated":
            return b.metadata.updatedAt - a.metadata.updatedAt
          case "created":
          default:
            return b.metadata.createdAt - a.metadata.createdAt
        }
      })

      // Apply limit
      if (options?.limit && options.limit > 0) {
        return tools.slice(0, options.limit)
      }

      return tools
    } catch {
      return []
    }
  }

  // ============================================================================
  // Statistics Operations
  // ============================================================================

  /**
   * Record a tool usage and update statistics
   */
  export async function recordUsage(
    id: string,
    success: boolean,
    durationMs: number,
  ): Promise<void> {
    const projectID = Instance.project.id

    try {
      const tool = await get(id)
      if (!tool) return

      const stats = tool.stats
      const newUsageCount = stats.usageCount + 1
      const newSuccessCount = success ? stats.successCount + 1 : stats.successCount
      const newFailureCount = success ? stats.failureCount : stats.failureCount + 1

      // Calculate new average execution time
      const prevTotal = stats.averageExecutionTime * stats.usageCount
      const newAverage = (prevTotal + durationMs) / newUsageCount

      const updatedStats: ToolTypes.ToolStats = {
        usageCount: newUsageCount,
        successCount: newSuccessCount,
        failureCount: newFailureCount,
        lastUsedAt: Date.now(),
        averageExecutionTime: Math.round(newAverage),
      }

      await Storage.write([...STORAGE_PREFIX, projectID, id], {
        ...tool,
        stats: updatedStats,
      })

      log.debug("Tool usage recorded", { id, success, durationMs })
    } catch (error) {
      log.warn("Failed to record tool usage", { id, error })
    }
  }

  /**
   * Get registry statistics
   */
  export async function getStats(): Promise<ToolTypes.RegistryStats> {
    const tools = await list()

    const byLanguage: Record<string, number> = {}
    const byTag: Record<string, number> = {}

    for (const tool of tools) {
      // Count by language
      byLanguage[tool.language] = (byLanguage[tool.language] ?? 0) + 1

      // Count by tag
      for (const tag of tool.tags) {
        byTag[tag] = (byTag[tag] ?? 0) + 1
      }
    }

    // Get most used tools
    const sortedByUsage = [...tools]
      .sort((a, b) => b.stats.usageCount - a.stats.usageCount)
      .slice(0, 10)
      .map((t) => ({
        id: t.id,
        name: t.name,
        usageCount: t.stats.usageCount,
      }))

    // Get recently added tools
    const sortedByCreated = [...tools]
      .sort((a, b) => b.metadata.createdAt - a.metadata.createdAt)
      .slice(0, 10)
      .map((t) => ({
        id: t.id,
        name: t.name,
        createdAt: t.metadata.createdAt,
      }))

    return {
      totalTools: tools.length,
      byLanguage,
      byTag,
      mostUsed: sortedByUsage,
      recentlyAdded: sortedByCreated,
      lastUpdated: Date.now(),
    }
  }

  // ============================================================================
  // Deduplication
  // ============================================================================

  /**
   * Check if a similar tool already exists
   */
  export async function findDuplicate(
    name: string,
    code: string,
  ): Promise<ToolTypes.DynamicTool | undefined> {
    const tools = await list()

    // Check for exact name match
    const exactNameMatch = tools.find(
      (t) => t.name.toLowerCase() === name.toLowerCase(),
    )
    if (exactNameMatch) return exactNameMatch

    // Check for code similarity (simple hash comparison)
    const codeHash = simpleHash(normalizeCode(code))
    for (const tool of tools) {
      const existingHash = simpleHash(normalizeCode(tool.code))
      if (codeHash === existingHash) {
        return tool
      }
    }

    return undefined
  }

  /**
   * Get or create a tool (upsert semantics)
   */
  export async function getOrCreate(
    input: ToolTypes.CreateToolInput,
  ): Promise<{ tool: ToolTypes.DynamicTool; created: boolean }> {
    const duplicate = await findDuplicate(input.name, input.code)

    if (duplicate) {
      return { tool: duplicate, created: false }
    }

    const tool = await register(input)
    return { tool, created: true }
  }

  // ============================================================================
  // Tool Code Generation
  // ============================================================================

  /**
   * Get tool code with parameters substituted
   */
  export async function getToolCode(
    id: string,
    params: Record<string, unknown>,
  ): Promise<string> {
    const tool = await get(id)
    if (!tool) {
      throw new Error(`Tool not found: ${id}`)
    }

    let code = tool.code

    // Simple parameter substitution
    // Supports {{paramName}} syntax
    for (const [key, value] of Object.entries(params)) {
      const placeholder = `{{${key}}}`
      const stringValue = typeof value === "string" ? value : JSON.stringify(value)
      code = code.replace(new RegExp(escapeRegExp(placeholder), "g"), stringValue)
    }

    // Validate required parameters
    for (const param of tool.parameters) {
      if (param.required && params[param.name] === undefined && param.default === undefined) {
        throw new Error(`Missing required parameter: ${param.name}`)
      }
    }

    return code
  }

  // ============================================================================
  // Cleanup Operations
  // ============================================================================

  /**
   * Clear all tools
   */
  export async function clear(): Promise<number> {
    const projectID = Instance.project.id
    let removed = 0

    try {
      const keys = await Storage.list([...STORAGE_PREFIX, projectID])

      for (const key of keys) {
        await Storage.remove(key)
        removed++
      }

      log.info("Tool registry cleared", { removed })
    } catch {
      // Ignore errors
    }

    return removed
  }

  /**
   * Remove unused tools (no usage in the specified period)
   */
  export async function cleanup(maxAgeDays = 90): Promise<number> {
    const cutoff = Date.now() - maxAgeDays * 24 * 60 * 60 * 1000
    let removed = 0

    const tools = await list()

    for (const tool of tools) {
      // Keep tools that have been used recently
      if (tool.stats.lastUsedAt && tool.stats.lastUsedAt > cutoff) continue

      // Keep tools created recently
      if (tool.metadata.createdAt > cutoff) continue

      // Keep tools with significant usage
      if (tool.stats.usageCount >= 5) continue

      await remove(tool.id)
      removed++
    }

    if (removed > 0) {
      log.info("Tool cleanup completed", { removed })
    }

    return removed
  }

  /**
   * Invalidate all registry data (for testing/reset)
   */
  export async function invalidate(): Promise<void> {
    await clear()
  }

  // ============================================================================
  // Helper Functions
  // ============================================================================

  function generateToolId(name: string, timestamp: number): string {
    const sanitized = name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .slice(0, 30)
    const random = Math.random().toString(36).slice(2, 8)
    return `tool_${sanitized}_${timestamp}_${random}`
  }

  function normalizeCode(code: string): string {
    return code
      .replace(/\s+/g, " ") // Normalize whitespace
      .replace(/['"`]/g, "'") // Normalize quotes
      .trim()
  }

  function simpleHash(str: string): number {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash = hash & hash // Convert to 32-bit integer
    }
    return Math.abs(hash)
  }

  function escapeRegExp(string: string): string {
    return string.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
  }
}
