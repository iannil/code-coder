/**
 * Unified Memory Module
 *
 * Provides a pluggable memory system supporting multiple backends:
 * - SQLite: High-performance with FTS5 full-text search (shares with ZeroBot)
 * - Markdown: Human-readable, Git-friendly storage
 * - Composite: Dual-write to both backends for consistency
 *
 * @example
 * ```typescript
 * import { createMemory } from "@codecoder-ai/memory"
 *
 * // Create with SQLite backend (default, shares with ZeroBot)
 * const memory = createMemory({ backend: "sqlite" })
 *
 * // Store a memory
 * await memory.store("user_preference", "Prefers TypeScript", "preference")
 *
 * // Recall related memories
 * const results = await memory.recall("programming language", 5)
 *
 * // Get specific memory
 * const pref = await memory.get("user_preference")
 *
 * // List all memories in a category
 * const prefs = await memory.list("preference")
 *
 * // Forget a memory
 * await memory.forget("outdated_key")
 *
 * // Clean up
 * await memory.close()
 * ```
 *
 * @module memory
 */

// Types
export type {
  MemoryCategory,
  MemoryEntry,
  UnifiedMemory,
  SqliteConfig,
  MarkdownConfig,
  CompositeConfig,
  ConflictStrategy,
  MemoryConfig,
  MemoryResult,
  MemoryStats,
} from "./types"

export { DEFAULT_CONFIG, MemoryEntrySchema, MemoryConfigSchema } from "./types"

// Backends
export { SqliteMemory, MarkdownMemory, CompositeMemory } from "./backends"
export { createSqliteMemory as createSqliteBackend } from "./backends/sqlite"
export { createMarkdownMemory as createMarkdownBackend } from "./backends/markdown"
export { createCompositeMemory as createCompositeBackend } from "./backends/composite"

// Factory
export {
  createMemory,
  createSqliteMemory,
  createMarkdownMemory,
  createCompositeMemory,
  getDefaultConfig,
} from "./factory"
