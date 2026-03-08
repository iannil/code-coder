/**
 * Composite Memory Backend
 *
 * Combines SQLite and Markdown backends for unified memory access:
 * - Dual-write to both backends for consistency
 * - Configurable read priority (SQLite for speed, Markdown for transparency)
 * - Conflict resolution strategies
 *
 * @module memory/backends/composite
 */

import type {
  CompositeConfig,
  ConflictStrategy,
  MarkdownConfig,
  MemoryCategory,
  MemoryEntry,
  SqliteConfig,
  UnifiedMemory,
} from "../types"
import { DEFAULT_CONFIG } from "../types"
import { SqliteMemory } from "./sqlite"
import { MarkdownMemory } from "./markdown"

/**
 * Composite Memory Backend
 *
 * Provides unified access to both SQLite and Markdown backends.
 */
export class CompositeMemory implements UnifiedMemory {
  readonly name = "composite"
  private readonly sqlite: SqliteMemory
  private readonly markdown: MarkdownMemory
  private readonly primary: "sqlite" | "markdown"
  private readonly writeToAll: boolean
  private readonly conflictStrategy: ConflictStrategy

  constructor(
    sqliteConfig?: SqliteConfig,
    markdownConfig?: MarkdownConfig,
    compositeConfig?: CompositeConfig,
  ) {
    const defaults = DEFAULT_CONFIG.composite
    this.primary = compositeConfig?.primary ?? defaults.primary
    this.writeToAll = compositeConfig?.writeToAll ?? defaults.writeToAll
    this.conflictStrategy = compositeConfig?.conflictStrategy ?? defaults.conflictStrategy

    this.sqlite = new SqliteMemory(sqliteConfig)
    this.markdown = new MarkdownMemory(markdownConfig)
  }

  /**
   * Get the primary backend instance
   */
  private getPrimary(): UnifiedMemory {
    return this.primary === "sqlite" ? this.sqlite : this.markdown
  }

  /**
   * Get the secondary backend instance
   */
  private getSecondary(): UnifiedMemory {
    return this.primary === "sqlite" ? this.markdown : this.sqlite
  }

  async store(key: string, content: string, category: MemoryCategory): Promise<void> {
    const primary = this.getPrimary()

    // Always write to primary
    await primary.store(key, content, category)

    // Optionally write to secondary
    if (this.writeToAll) {
      const secondary = this.getSecondary()
      try {
        await secondary.store(key, content, category)
      } catch {
        // Secondary backend write failed - continue with primary only
      }
    }
  }

  async recall(query: string, limit: number = 10): Promise<MemoryEntry[]> {
    const primary = this.getPrimary()
    const secondary = this.getSecondary()

    // Get results from both backends
    const [primaryResults, secondaryResults] = await Promise.all([
      primary.recall(query, limit),
      secondary.recall(query, limit).catch(() => [] as MemoryEntry[]),
    ])

    // Merge results based on conflict strategy
    return this.mergeResults(primaryResults, secondaryResults, limit)
  }

  /**
   * Merge results from both backends
   */
  private mergeResults(
    primaryResults: MemoryEntry[],
    secondaryResults: MemoryEntry[],
    limit: number,
  ): MemoryEntry[] {
    switch (this.conflictStrategy) {
      case "primary-wins":
        return this.mergePrimaryWins(primaryResults, secondaryResults, limit)

      case "newest-wins":
        return this.mergeNewestWins(primaryResults, secondaryResults, limit)

      case "merge":
        return this.mergeAll(primaryResults, secondaryResults, limit)

      default:
        return this.mergePrimaryWins(primaryResults, secondaryResults, limit)
    }
  }

  /**
   * Primary results take precedence, secondary fills gaps
   */
  private mergePrimaryWins(
    primaryResults: MemoryEntry[],
    secondaryResults: MemoryEntry[],
    limit: number,
  ): MemoryEntry[] {
    const seen = new Set(primaryResults.map((r) => r.key))
    const merged = [...primaryResults]

    for (const entry of secondaryResults) {
      if (!seen.has(entry.key) && merged.length < limit) {
        merged.push(entry)
        seen.add(entry.key)
      }
    }

    return merged.slice(0, limit)
  }

  /**
   * Keep newest version of each entry
   */
  private mergeNewestWins(
    primaryResults: MemoryEntry[],
    secondaryResults: MemoryEntry[],
    limit: number,
  ): MemoryEntry[] {
    const byKey = new Map<string, MemoryEntry>()

    // Add all entries, keeping newest
    for (const entry of [...primaryResults, ...secondaryResults]) {
      const existing = byKey.get(entry.key)
      if (!existing || this.isNewer(entry, existing)) {
        byKey.set(entry.key, entry)
      }
    }

    // Sort by score descending
    const merged = Array.from(byKey.values())
    merged.sort((a, b) => (b.score ?? 0) - (a.score ?? 0))

    return merged.slice(0, limit)
  }

  /**
   * Check if entry A is newer than entry B
   */
  private isNewer(a: MemoryEntry, b: MemoryEntry): boolean {
    try {
      const dateA = new Date(a.timestamp)
      const dateB = new Date(b.timestamp)
      return dateA > dateB
    } catch {
      return false
    }
  }

  /**
   * Merge all results with deduplication
   */
  private mergeAll(
    primaryResults: MemoryEntry[],
    secondaryResults: MemoryEntry[],
    limit: number,
  ): MemoryEntry[] {
    const byKey = new Map<string, MemoryEntry[]>()

    // Group by key
    for (const entry of [...primaryResults, ...secondaryResults]) {
      const existing = byKey.get(entry.key) ?? []
      existing.push(entry)
      byKey.set(entry.key, existing)
    }

    // For each key, pick the one with highest score
    const merged: MemoryEntry[] = []
    for (const entries of byKey.values()) {
      entries.sort((a, b) => (b.score ?? 0) - (a.score ?? 0))
      if (entries[0]) {
        merged.push(entries[0])
      }
    }

    // Sort by score
    merged.sort((a, b) => (b.score ?? 0) - (a.score ?? 0))

    return merged.slice(0, limit)
  }

  async get(key: string): Promise<MemoryEntry | null> {
    const primary = this.getPrimary()
    const secondary = this.getSecondary()

    // Try primary first
    const primaryResult = await primary.get(key)

    switch (this.conflictStrategy) {
      case "primary-wins":
        return primaryResult ?? (await secondary.get(key).catch(() => null))

      case "newest-wins": {
        if (!primaryResult) {
          return await secondary.get(key).catch(() => null)
        }
        const secondaryResult = await secondary.get(key).catch(() => null)
        if (!secondaryResult) return primaryResult
        return this.isNewer(secondaryResult, primaryResult) ? secondaryResult : primaryResult
      }

      case "merge":
        return primaryResult ?? (await secondary.get(key).catch(() => null))

      default:
        return primaryResult ?? (await secondary.get(key).catch(() => null))
    }
  }

  async list(category?: MemoryCategory): Promise<MemoryEntry[]> {
    const primary = this.getPrimary()
    const secondary = this.getSecondary()

    const [primaryResults, secondaryResults] = await Promise.all([
      primary.list(category),
      secondary.list(category).catch(() => [] as MemoryEntry[]),
    ])

    return this.mergeResults(primaryResults, secondaryResults, 10000)
  }

  async forget(key: string): Promise<boolean> {
    const primary = this.getPrimary()
    let success = await primary.forget(key)

    if (this.writeToAll) {
      const secondary = this.getSecondary()
      try {
        const secondarySuccess = await secondary.forget(key)
        success = success || secondarySuccess
      } catch {
        // Log but don't fail if secondary forget fails
      }
    }

    return success
  }

  async count(): Promise<number> {
    // Return count from primary backend
    const primary = this.getPrimary()
    return primary.count()
  }

  async healthCheck(): Promise<boolean> {
    const primary = this.getPrimary()
    const secondary = this.getSecondary()

    const [primaryHealth, secondaryHealth] = await Promise.all([
      primary.healthCheck(),
      secondary.healthCheck().catch(() => false),
    ])

    // Healthy if at least primary is healthy
    return primaryHealth
  }

  async close(): Promise<void> {
    await Promise.all([this.sqlite.close(), this.markdown.close()])
  }

  /**
   * Get individual backend instances for direct access
   */
  getBackends(): { sqlite: SqliteMemory; markdown: MarkdownMemory } {
    return { sqlite: this.sqlite, markdown: this.markdown }
  }

  /**
   * Sync data from primary to secondary backend
   *
   * Useful for migrating data or ensuring consistency.
   */
  async syncToSecondary(): Promise<number> {
    const primary = this.getPrimary()
    const secondary = this.getSecondary()

    const entries = await primary.list()
    let synced = 0

    for (const entry of entries) {
      try {
        await secondary.store(entry.key, entry.content, entry.category)
        synced++
      } catch {
        // Skip failed entries
      }
    }

    return synced
  }

  /**
   * Get statistics about both backends
   */
  async getStats(): Promise<{
    sqliteCount: number
    markdownCount: number
    sqliteHealthy: boolean
    markdownHealthy: boolean
  }> {
    const [sqliteCount, markdownCount, sqliteHealthy, markdownHealthy] = await Promise.all([
      this.sqlite.count(),
      this.markdown.count(),
      this.sqlite.healthCheck(),
      this.markdown.healthCheck(),
    ])

    return {
      sqliteCount,
      markdownCount,
      sqliteHealthy,
      markdownHealthy,
    }
  }
}

/**
 * Create a Composite memory backend with default configuration
 */
export function createCompositeMemory(
  sqliteConfig?: SqliteConfig,
  markdownConfig?: MarkdownConfig,
  compositeConfig?: CompositeConfig,
): CompositeMemory {
  return new CompositeMemory(sqliteConfig, markdownConfig, compositeConfig)
}
