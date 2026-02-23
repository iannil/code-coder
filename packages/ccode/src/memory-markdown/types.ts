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
export type DailyEntryType = "decision" | "action" | "output" | "error" | "solution"

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
export type MemoryCategory = "用户偏好" | "项目上下文" | "关键决策" | "经验教训" | "成功方案"

/**
 * Long-term memory section structure
 */
export interface MemorySection {
  category: MemoryCategory
  content: string
  lastUpdated: string
}

/**
 * Autonomous mode solution entry for knowledge extraction
 */
export interface AutonomousSolution {
  /** Unique solution identifier */
  id: string
  /** Session ID where solution was discovered */
  sessionId: string
  /** Problem description */
  problem: string
  /** Solution description */
  solution: string
  /** Confidence score (0-1) */
  confidence: number
  /** Source of the solution */
  source: "autonomous" | "web_search" | "user"
  /** Tags for categorization */
  tags: string[]
  /** Technology/domain */
  technology?: string
  /** Code snippets if any */
  codeSnippets?: Array<{
    language: string
    code: string
  }>
  /** Timestamp when discovered */
  discoveredAt: string
  /** Number of times solution was reused */
  reuseCount: number
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
