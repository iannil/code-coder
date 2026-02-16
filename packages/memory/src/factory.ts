/**
 * Memory Factory
 *
 * Factory function for creating memory instances based on configuration.
 *
 * @module memory/factory
 */

import type { MemoryConfig, UnifiedMemory } from "./types"
import { DEFAULT_CONFIG, MemoryConfigSchema } from "./types"
import { SqliteMemory } from "./backends/sqlite"
import { MarkdownMemory } from "./backends/markdown"
import { CompositeMemory } from "./backends/composite"

/**
 * Create a memory instance based on configuration
 *
 * @param config - Memory configuration
 * @returns UnifiedMemory instance
 *
 * @example
 * ```typescript
 * // SQLite backend (default, shares with ZeroBot)
 * const memory = createMemory({ backend: "sqlite" })
 *
 * // Markdown backend (human-readable)
 * const memory = createMemory({
 *   backend: "markdown",
 *   markdown: { basePath: "./memory" }
 * })
 *
 * // Composite backend (dual-write)
 * const memory = createMemory({
 *   backend: "composite",
 *   composite: { primary: "sqlite", writeToAll: true }
 * })
 * ```
 */
export function createMemory(config?: Partial<MemoryConfig>): UnifiedMemory {
  const fullConfig: MemoryConfig = {
    backend: config?.backend ?? DEFAULT_CONFIG.backend,
    sqlite: { ...DEFAULT_CONFIG.sqlite, ...config?.sqlite },
    markdown: { ...DEFAULT_CONFIG.markdown, ...config?.markdown },
    composite: { ...DEFAULT_CONFIG.composite, ...config?.composite },
  }

  // Validate configuration
  const result = MemoryConfigSchema.safeParse(fullConfig)
  if (!result.success) {
    throw new Error(`Invalid memory configuration: ${result.error.message}`)
  }

  switch (fullConfig.backend) {
    case "sqlite":
      return new SqliteMemory(fullConfig.sqlite)

    case "markdown":
      return new MarkdownMemory(fullConfig.markdown)

    case "composite":
      return new CompositeMemory(fullConfig.sqlite, fullConfig.markdown, fullConfig.composite)

    default: {
      const exhaustiveCheck: never = fullConfig.backend
      throw new Error(`Unknown backend: ${exhaustiveCheck}`)
    }
  }
}

/**
 * Create a SQLite memory instance with default settings
 *
 * Uses ZeroBot's database path for shared memory.
 */
export function createSqliteMemory(): SqliteMemory {
  return new SqliteMemory()
}

/**
 * Create a Markdown memory instance with default settings
 */
export function createMarkdownMemory(): MarkdownMemory {
  return new MarkdownMemory()
}

/**
 * Create a Composite memory instance with default settings
 */
export function createCompositeMemory(): CompositeMemory {
  return new CompositeMemory()
}

/**
 * Get the default memory configuration
 */
export function getDefaultConfig(): typeof DEFAULT_CONFIG {
  return DEFAULT_CONFIG
}
