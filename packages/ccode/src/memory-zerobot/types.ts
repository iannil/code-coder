/**
 * ZeroBot Memory Types
 *
 * TypeScript types matching ZeroBot's Rust memory schema.
 * Database: ~/.codecoder/workspace/memory/brain.db
 *
 * Source of truth: services/zero-bot/src/memory/traits.rs
 *
 * Note: In the database, categories are stored as simple strings.
 * The Rust enum MemoryCategory { Core, Daily, Conversation, Custom(String) }
 * serializes to snake_case strings: "core", "daily", "conversation", or custom values.
 *
 * TODO (P3 Debt): Set up ts-rs for automatic type generation:
 * 1. Add to services/zero-bot/Cargo.toml:
 *    ```toml
 *    [dependencies]
 *    ts-rs = "10"
 *    ```
 * 2. Annotate Rust types with #[derive(TS)] and #[ts(export)]
 * 3. Run: cargo test export_bindings
 * 4. Output will be in: services/zero-bot/bindings/*.ts
 */

/**
 * Memory category matching ZeroBot's MemoryCategory enum
 *
 * Rust enum (serialized as snake_case strings):
 * - Core: "core" - Long-term facts, preferences, decisions
 * - Daily: "daily" - Daily session logs
 * - Conversation: "conversation" - Conversation context
 * - Custom(String): Any other string - User-defined custom category
 */
export type MemoryCategory = "core" | "daily" | "conversation" | string

/**
 * Well-known category constants
 */
export const MEMORY_CATEGORIES = {
  CORE: "core" as const,
  DAILY: "daily" as const,
  CONVERSATION: "conversation" as const,
} as const

/**
 * Memory entry matching ZeroBot's memories table schema
 *
 * Rust struct: services/zero-bot/src/memory/traits.rs:MemoryEntry
 */
export interface MemoryEntry {
  id: string
  key: string
  content: string
  category: MemoryCategory
  timestamp: string
  /** Session ID if memory was created within a session */
  session_id?: string
  /** Relevance score from search (0.0 - 1.0) */
  score?: number
}

/**
 * Configuration for ZeroBot memory provider
 */
export interface ZeroBotMemoryConfig {
  /** Path to ZeroBot workspace (default: ~/.codecoder/workspace) */
  workspacePath?: string
  /** Database filename (default: memory/brain.db) */
  dbFilename?: string
  /** Enable read-only mode (default: true to prevent accidental writes) */
  readOnly?: boolean
}

