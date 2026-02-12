/**
 * Markdown-based Memory Layer Types
 *
 * Dual-layer transparent memory architecture:
 * - Flow layer (daily): Chronological log of daily activities
 * - Sediment layer (long-term): Consolidated knowledge and patterns
 */

/**
 * Daily note entry types for categorization
 */
export type DailyEntryType = "decision" | "action" | "output" | "error"

/**
 * Single entry in daily notes (flow layer)
 */
export interface DailyEntry {
  timestamp: string
  type: DailyEntryType
  content: string
  metadata?: Record<string, unknown>
}

/**
 * Long-term memory categories (sediment layer)
 */
export type MemoryCategory = "用户偏好" | "项目上下文" | "关键决策" | "经验教训"

/**
 * Long-term memory section structure
 */
export interface MemorySection {
  category: MemoryCategory
  content: string
  lastUpdated: string
}

/**
 * Options for loading memory context
 */
export interface LoadOptions {
  includeDays?: number
  categories?: MemoryCategory[]
}

/**
 * Combined memory context result
 */
export interface MemoryContext {
  longTerm: string
  daily: string[]
  combined: string
}

/**
 * Configuration for markdown memory storage
 */
export interface MemoryConfig {
  basePath: string
  dailyPath: string
  longTermPath: string
}

/**
 * Storage configuration for markdown memory layer
 *
 * Supports environment variable and config file overrides.
 * Default behavior: uses {process.cwd()}/memory
 */
export interface MemoryStorageConfig {
  /** Base path for memory storage (overrides env var and default) */
  basePath?: string
  /** Project identifier for multi-project shared storage */
  projectId?: string
  /** Storage provider type (for future expansion) */
  provider?: "local" | "http" | "database"
}
