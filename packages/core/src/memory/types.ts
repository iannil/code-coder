/**
 * Unified Memory Module Types
 *
 * Defines the core interfaces for the memory system that supports
 * both SQLite and Markdown backends.
 *
 * @module memory/types
 */

import { z } from "zod"

/**
 * Memory category for organizing memories
 *
 * Matches categories used in both ZeroBot (Rust) and CodeCoder (TypeScript)
 */
export type MemoryCategory = "core" | "daily" | "preference" | "decision" | "lesson" | "conversation" | string

/**
 * Memory entry representing a single stored memory
 */
export interface MemoryEntry {
  /** Unique identifier */
  readonly id: string
  /** Human-readable key for lookup */
  readonly key: string
  /** Memory content */
  readonly content: string
  /** Category for organization */
  readonly category: MemoryCategory
  /** ISO timestamp of creation/update */
  readonly timestamp: string
  /** Relevance score from search (optional) */
  readonly score?: number
  /** Source backend that provided this entry */
  readonly source?: "sqlite" | "markdown"
}

/**
 * Zod schema for validating MemoryEntry
 */
export const MemoryEntrySchema = z.object({
  id: z.string(),
  key: z.string(),
  content: z.string(),
  category: z.string(),
  timestamp: z.string(),
  score: z.number().optional(),
  source: z.enum(["sqlite", "markdown"]).optional(),
})

/**
 * Core memory interface that all backends must implement
 */
export interface UnifiedMemory {
  /** Backend name identifier */
  readonly name: string

  /**
   * Store a memory entry (upsert by key)
   *
   * @param key - Unique key for the memory
   * @param content - Content to store
   * @param category - Category for organization
   */
  store(key: string, content: string, category: MemoryCategory): Promise<void>

  /**
   * Recall memories matching a query
   *
   * Uses full-text search (FTS5 for SQLite, keyword matching for Markdown)
   *
   * @param query - Search query
   * @param limit - Maximum number of results (default: 10)
   */
  recall(query: string, limit?: number): Promise<MemoryEntry[]>

  /**
   * Get a specific memory by key
   *
   * @param key - Memory key to retrieve
   */
  get(key: string): Promise<MemoryEntry | null>

  /**
   * List all memories, optionally filtered by category
   *
   * @param category - Optional category filter
   */
  list(category?: MemoryCategory): Promise<MemoryEntry[]>

  /**
   * Remove a memory by key
   *
   * @param key - Memory key to remove
   * @returns true if memory was found and removed
   */
  forget(key: string): Promise<boolean>

  /**
   * Count total memories
   *
   * @returns Total number of stored memories
   */
  count(): Promise<number>

  /**
   * Check if the memory backend is healthy and accessible
   */
  healthCheck(): Promise<boolean>

  /**
   * Close the memory backend and release resources
   */
  close(): Promise<void>
}

/**
 * SQLite backend configuration
 */
export interface SqliteConfig {
  /**
   * Path to the SQLite database file
   * @default "~/.codecoder/workspace/memory/brain.db"
   */
  dbPath?: string

  /**
   * Weight for vector similarity in hybrid search (0-1)
   * @default 0.7
   */
  vectorWeight?: number

  /**
   * Weight for keyword matching in hybrid search (0-1)
   * @default 0.3
   */
  keywordWeight?: number

  /**
   * Maximum size of embedding cache
   * @default 10000
   */
  embeddingCacheSize?: number

  /**
   * Read-only mode (prevents writes)
   * @default false
   */
  readOnly?: boolean
}

/**
 * Markdown backend configuration
 */
export interface MarkdownConfig {
  /**
   * Base path for markdown memory files
   * @default "./memory"
   */
  basePath?: string

  /**
   * Long-term memory file name
   * @default "MEMORY.md"
   */
  longTermFile?: string

  /**
   * Directory for daily notes
   * @default "daily"
   */
  dailyDir?: string

  /**
   * Project identifier for multi-project scenarios
   */
  projectId?: string
}

/**
 * Conflict resolution strategy for composite backend
 */
export type ConflictStrategy = "primary-wins" | "newest-wins" | "merge"

/**
 * Composite backend configuration
 */
export interface CompositeConfig {
  /**
   * Primary backend for read operations
   * @default "sqlite"
   */
  primary?: "sqlite" | "markdown"

  /**
   * Write to all backends on store operations
   * @default true
   */
  writeToAll?: boolean

  /**
   * Strategy for resolving conflicts between backends
   * @default "primary-wins"
   */
  conflictStrategy?: ConflictStrategy
}

/**
 * Full memory configuration
 */
export interface MemoryConfig {
  /**
   * Backend type to use
   */
  backend: "sqlite" | "markdown" | "composite"

  /**
   * SQLite backend configuration (required if backend is "sqlite" or "composite")
   */
  sqlite?: SqliteConfig

  /**
   * Markdown backend configuration (required if backend is "markdown" or "composite")
   */
  markdown?: MarkdownConfig

  /**
   * Composite backend configuration (required if backend is "composite")
   */
  composite?: CompositeConfig
}

/**
 * Zod schema for validating MemoryConfig
 */
export const MemoryConfigSchema = z.object({
  backend: z.enum(["sqlite", "markdown", "composite"]),
  sqlite: z
    .object({
      dbPath: z.string().optional(),
      vectorWeight: z.number().min(0).max(1).optional(),
      keywordWeight: z.number().min(0).max(1).optional(),
      embeddingCacheSize: z.number().positive().optional(),
      readOnly: z.boolean().optional(),
    })
    .optional(),
  markdown: z
    .object({
      basePath: z.string().optional(),
      longTermFile: z.string().optional(),
      dailyDir: z.string().optional(),
      projectId: z.string().optional(),
    })
    .optional(),
  composite: z
    .object({
      primary: z.enum(["sqlite", "markdown"]).optional(),
      writeToAll: z.boolean().optional(),
      conflictStrategy: z.enum(["primary-wins", "newest-wins", "merge"]).optional(),
    })
    .optional(),
})

/**
 * Default configuration values
 */
export const DEFAULT_CONFIG: Required<
  Pick<MemoryConfig, "backend"> & {
    sqlite: Required<SqliteConfig>
    markdown: Required<Omit<MarkdownConfig, "projectId">>
    composite: Required<CompositeConfig>
  }
> = {
  backend: "sqlite",
  sqlite: {
    dbPath: "~/.codecoder/workspace/memory/brain.db",
    vectorWeight: 0.7,
    keywordWeight: 0.3,
    embeddingCacheSize: 10000,
    readOnly: false,
  },
  markdown: {
    basePath: "./memory",
    longTermFile: "MEMORY.md",
    dailyDir: "daily",
  },
  composite: {
    primary: "sqlite",
    writeToAll: true,
    conflictStrategy: "primary-wins",
  },
}

/**
 * Result of a memory operation
 */
export interface MemoryResult<T> {
  success: boolean
  data?: T
  error?: string
}

/**
 * Memory statistics
 */
export interface MemoryStats {
  totalCount: number
  byCategory: Record<MemoryCategory, number>
  lastUpdated?: string
  backendName: string
}
