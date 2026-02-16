/**
 * Memory Router
 *
 * Unified entry point for memory writes that routes data to the correct storage layer
 * based on data type. This eliminates ambiguity about which memory layer to use.
 *
 * Routing rules:
 * - preference, decision, lesson → Markdown long-term (MEMORY.md) + SQLite (via unified)
 * - daily → Markdown daily notes (daily/*.md) + SQLite (via unified)
 * - pattern, context → Technical layer (preferences/patterns)
 *
 * @module agent/memory-router
 */

import { Log } from "@/util/log"
import {
  appendDailyNote,
  mergeToCategory,
  createEntry,
  type MemoryCategory,
  type DailyEntryType,
} from "@/memory-markdown"
import { Preferences } from "@/memory/preferences"
import { invalidateMemoryCache, storeUnifiedMemory } from "./memory-bridge"

const log = Log.create({ service: "agent.memory-router" })

/**
 * Types of memory that can be written
 */
export type MemoryWriteType =
  | "preference" // User preferences → long-term markdown
  | "decision" // Key decisions → long-term markdown
  | "lesson" // Lessons learned → long-term markdown
  | "daily" // Daily log entries → daily notes
  | "pattern" // Code patterns → technical layer
  | "context" // Project context → long-term markdown

/**
 * Request to write memory
 */
export interface MemoryWriteRequest {
  /** Type determines routing destination */
  type: MemoryWriteType
  /** Unique key for the memory entry */
  key: string
  /** Content to store */
  content: string
  /** Optional metadata */
  metadata?: Record<string, unknown>
}

/**
 * Result of a memory write operation
 */
export interface MemoryWriteResult {
  success: boolean
  destination: string
  error?: string
}

/**
 * Maps memory write types to markdown categories
 */
const TYPE_TO_CATEGORY: Record<Exclude<MemoryWriteType, "daily" | "pattern">, MemoryCategory> = {
  preference: "用户偏好",
  decision: "关键决策",
  lesson: "经验教训",
  context: "项目上下文",
}

/**
 * Maps memory write types to daily entry types
 */
const TYPE_TO_DAILY_ENTRY: Record<string, DailyEntryType> = {
  action: "action",
  output: "output",
  decision: "decision",
  error: "error",
}

/**
 * Route a memory write to the correct storage layer
 *
 * This is the unified entry point for all memory writes. It automatically
 * routes to the appropriate storage based on the data type.
 * Also syncs to the unified memory system (SQLite) for cross-system sharing.
 *
 * @param request - The memory write request
 * @returns Result indicating success and destination
 */
export async function routeMemoryWrite(request: MemoryWriteRequest): Promise<MemoryWriteResult> {
  const { type, key, content, metadata } = request

  log.debug("routing memory write", { type, key })

  try {
    switch (type) {
      case "preference":
      case "decision":
      case "lesson":
      case "context": {
        const category = TYPE_TO_CATEGORY[type]
        const formattedContent = formatForLongTerm(key, content, metadata)
        await mergeToCategory(category, formattedContent)

        // Sync to unified memory (SQLite) for cross-system access
        await storeUnifiedMemory(key, content, type).catch((error) => {
          log.warn("failed to sync to unified memory", { key, error })
        })

        // Invalidate cache after successful write
        invalidateMemoryCache()

        log.info("wrote to long-term memory", { type, category, key })
        return { success: true, destination: `MEMORY.md/${category}` }
      }

      case "daily": {
        const entryType = (metadata?.entryType as DailyEntryType) ?? "action"
        const entry = createEntry(entryType, content, {
          ...metadata,
          key,
        })
        await appendDailyNote(entry)

        // Sync to unified memory (SQLite) for cross-system access
        await storeUnifiedMemory(key, content, "daily").catch((error) => {
          log.warn("failed to sync to unified memory", { key, error })
        })

        // Invalidate cache after successful write
        invalidateMemoryCache()

        log.info("wrote to daily notes", { key, entryType })
        return { success: true, destination: "daily/*.md" }
      }

      case "pattern": {
        await Preferences.learnPattern(content)

        // Note: pattern writes don't affect the memory context cache
        log.info("learned pattern", { key, pattern: content })
        return { success: true, destination: "preferences/patterns" }
      }

      default: {
        const exhaustiveCheck: never = type
        throw new Error(`Unknown memory type: ${exhaustiveCheck}`)
      }
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)
    log.error("memory write failed", { type, key, error: errorMessage })

    return {
      success: false,
      destination: getDestinationForType(type),
      error: errorMessage,
    }
  }
}

/**
 * Batch write multiple memory entries
 *
 * @param requests - Array of write requests
 * @returns Array of results matching input order
 */
export async function batchMemoryWrite(requests: MemoryWriteRequest[]): Promise<MemoryWriteResult[]> {
  return Promise.all(requests.map(routeMemoryWrite))
}

/**
 * Format content for long-term memory storage
 */
function formatForLongTerm(key: string, content: string, metadata?: Record<string, unknown>): string {
  const timestamp = new Date().toISOString()
  const lines: string[] = []

  lines.push(`- **${key}**: ${content}`)

  if (metadata && Object.keys(metadata).length > 0) {
    const metaStr = Object.entries(metadata)
      .filter(([k]) => k !== "entryType")
      .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
      .join(", ")
    if (metaStr) {
      lines.push(`  _[${timestamp}] ${metaStr}_`)
    }
  }

  return lines.join("\n")
}

/**
 * Get the destination string for a memory type
 */
function getDestinationForType(type: MemoryWriteType): string {
  switch (type) {
    case "preference":
      return "MEMORY.md/用户偏好"
    case "decision":
      return "MEMORY.md/关键决策"
    case "lesson":
      return "MEMORY.md/经验教训"
    case "context":
      return "MEMORY.md/项目上下文"
    case "daily":
      return "daily/*.md"
    case "pattern":
      return "preferences/patterns"
    default:
      return "unknown"
  }
}

/**
 * Helper to write a user preference
 */
export async function writePreference(key: string, content: string): Promise<MemoryWriteResult> {
  return routeMemoryWrite({ type: "preference", key, content })
}

/**
 * Helper to write a decision
 */
export async function writeDecision(key: string, content: string): Promise<MemoryWriteResult> {
  return routeMemoryWrite({ type: "decision", key, content })
}

/**
 * Helper to write a lesson learned
 */
export async function writeLesson(key: string, content: string): Promise<MemoryWriteResult> {
  return routeMemoryWrite({ type: "lesson", key, content })
}

/**
 * Helper to write a daily note
 */
export async function writeDailyNote(
  key: string,
  content: string,
  entryType: DailyEntryType = "action",
): Promise<MemoryWriteResult> {
  return routeMemoryWrite({ type: "daily", key, content, metadata: { entryType } })
}

/**
 * Helper to learn a code pattern
 */
export async function learnPattern(pattern: string): Promise<MemoryWriteResult> {
  return routeMemoryWrite({ type: "pattern", key: pattern, content: pattern })
}
