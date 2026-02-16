/**
 * Memory Bridge
 *
 * Bridges existing memory system with the new Markdown layer.
 * Each system serves different purposes:
 *
 * - Existing memory (@/memory): Vector search, code indexing, pattern learning
 * - Markdown layer (@/memory-markdown): User preferences, decisions, lessons learned
 * - Unified memory (@codecoder-ai/memory): Combined SQLite + Markdown with shared ZeroBot access
 *
 * Features a TTL-based cache to reduce repeated loading overhead.
 */

import { Log } from "@/util/log"
import { getAgentContext } from "./context"
import { loadMarkdownMemoryContext, loadRecentContext } from "@/memory-markdown"
import {
  createMemory,
  type UnifiedMemory,
  type MemoryEntry,
  type MemoryCategory,
} from "@codecoder-ai/memory"

const log = Log.create({ service: "agent.memory-bridge" })

/** Cache TTL in milliseconds (30 seconds) */
const CACHE_TTL_MS = 30_000

/**
 * Cached memory context
 */
interface CachedMemoryContext {
  context: BridgedMemoryContext
  timestamp: number
  /** Hash of the options used to build this context */
  optionsHash: string
}

/** Memory context cache */
let memoryCache: CachedMemoryContext | null = null

/**
 * Combined memory context for agents
 */
export interface BridgedMemoryContext {
  technical: Awaited<ReturnType<typeof getAgentContext>>
  markdown: Awaited<ReturnType<typeof loadMarkdownMemoryContext>>
  formatted: string
}

/**
 * Options for building memory context
 */
export interface BuildMemoryContextOptions {
  task?: string
  filePaths?: string[]
  includeMarkdownDays?: number
  /** Skip cache and force fresh load */
  skipCache?: boolean
}

/**
 * Compute a simple hash of options for cache invalidation
 */
function hashOptions(options: BuildMemoryContextOptions): string {
  return JSON.stringify({
    task: options.task ?? "",
    filePaths: options.filePaths?.sort() ?? [],
    includeMarkdownDays: options.includeMarkdownDays ?? 3,
  })
}

/**
 * Build complete memory context combining both systems
 *
 * Uses TTL-based caching to reduce repeated loading overhead.
 * Cache is invalidated after 30 seconds or when options change.
 */
export async function buildMemoryContext(options?: BuildMemoryContextOptions): Promise<BridgedMemoryContext> {
  const opts = options ?? {}
  const { task, filePaths, includeMarkdownDays = 3, skipCache = false } = opts

  const now = Date.now()
  const optionsHash = hashOptions(opts)

  // Check cache validity
  if (
    !skipCache &&
    memoryCache &&
    now - memoryCache.timestamp < CACHE_TTL_MS &&
    memoryCache.optionsHash === optionsHash
  ) {
    log.debug("using cached memory context", { age: now - memoryCache.timestamp })
    return memoryCache.context
  }

  try {
    const [technical, markdown] = await Promise.all([
      getAgentContext(task ?? "", filePaths),
      loadMarkdownMemoryContext({ includeDays: includeMarkdownDays }),
    ])

    const formatted = formatBridgedContext(technical, markdown)

    const context: BridgedMemoryContext = { technical, markdown, formatted }

    // Update cache
    memoryCache = {
      context,
      timestamp: now,
      optionsHash,
    }

    log.debug("built bridged memory context", {
      hasTechnical: !!technical,
      markdownDailyCount: markdown.daily.length,
      combinedLength: formatted.length,
      cached: true,
    })

    return context
  } catch (error) {
    log.warn("failed to build complete context, using partial", { error })

    const technical = await getAgentContext(task ?? "", filePaths).catch(() => null)

    return {
      technical: technical ?? getDefaultTechnicalContext(),
      markdown: {
        longTerm: "",
        daily: [],
        combined: "",
      },
      formatted: formatTechnicalOnly(technical),
    }
  }
}

/**
 * Invalidate the memory context cache
 *
 * Call this after writing to memory to ensure fresh data on next read.
 */
export function invalidateMemoryCache(): void {
  if (memoryCache) {
    log.debug("memory cache invalidated")
    memoryCache = null
  }
}

/**
 * Get recent markdown memory for prompt injection
 */
export async function getRecentMarkdownMemory(days = 1): Promise<string> {
  try {
    return await loadRecentContext(days)
  } catch (error) {
    log.warn("failed to load recent markdown memory", { error })
    return ""
  }
}

/**
 * Format combined context for agent consumption
 */
function formatBridgedContext(
  technical: Awaited<ReturnType<typeof getAgentContext>>,
  markdown: Awaited<ReturnType<typeof loadMarkdownMemoryContext>>,
): string {
  const parts: string[] = []

  parts.push("# Complete Memory Context")
  parts.push("")

  // Technical context from existing memory system
  if (technical) {
    parts.push("## Technical Context")
    parts.push("")
    parts.push(formatTechnicalContext(technical))
    parts.push("")
  }

  // Markdown memory layer
  if (markdown.combined) {
    parts.push("## Markdown Memory Layer")
    parts.push("")
    parts.push(markdown.combined)
  }

  return parts.join("\n")
}

/**
 * Format technical context section
 */
function formatTechnicalContext(context: Awaited<ReturnType<typeof getAgentContext>>): string {
  const lines: string[] = []

  if (context.projectFingerprint) {
    lines.push(`**Project:** ${context.projectFingerprint}`)
  }

  if (context.codeStyle) {
    lines.push(`**Code Style:** ${context.codeStyle}`)
  }

  if (context.learnedPatterns.length > 0) {
    lines.push(`**Patterns:** ${context.learnedPatterns.join(", ")}`)
  }

  return lines.join("\n")
}

/**
 * Get default technical context when loading fails
 */
function getDefaultTechnicalContext(): Awaited<ReturnType<typeof getAgentContext>> {
  return {
    projectFingerprint: "unknown",
    codeStyle: "",
    learnedPatterns: [],
    projectKnowledge: { apiEndpoints: 0, components: 0, dataModels: 0 },
    relevantFiles: [],
    recentEdits: [],
    decisions: [],
  }
}

/**
 * Format technical-only context
 */
function formatTechnicalOnly(technical: Awaited<ReturnType<typeof getAgentContext>> | null): string {
  if (!technical) return "# Technical Context\n\n_No technical context available._\n"

  return `# Technical Context\n\n${formatTechnicalContext(technical)}\n`
}

// ─────────────────────────────────────────────────────────────────────────────
// Unified Memory System
// ─────────────────────────────────────────────────────────────────────────────

/** Global unified memory instance */
let unifiedMemory: UnifiedMemory | null = null

/**
 * Get the unified memory instance
 *
 * Creates a composite backend that writes to both SQLite (shared with ZeroBot)
 * and Markdown (human-readable) backends.
 */
export function getUnifiedMemory(): UnifiedMemory {
  if (!unifiedMemory) {
    unifiedMemory = createMemory({
      backend: "composite",
      sqlite: {
        // Share with ZeroBot's database
        dbPath: "~/.codecoder/workspace/memory/brain.db",
        readOnly: false,
      },
      markdown: {
        basePath: "./memory",
      },
      composite: {
        primary: "sqlite",
        writeToAll: true,
        conflictStrategy: "newest-wins",
      },
    })
    log.info("initialized unified memory (composite backend)")
  }
  return unifiedMemory
}

/**
 * Store a memory using the unified system
 */
export async function storeUnifiedMemory(
  key: string,
  content: string,
  category: MemoryCategory,
): Promise<void> {
  const memory = getUnifiedMemory()
  await memory.store(key, content, category)
  invalidateMemoryCache()
}

/**
 * Recall memories from the unified system
 */
export async function recallUnifiedMemory(
  query: string,
  limit = 10,
): Promise<MemoryEntry[]> {
  const memory = getUnifiedMemory()
  return memory.recall(query, limit)
}

/**
 * Get a specific memory from the unified system
 */
export async function getUnifiedMemoryEntry(key: string): Promise<MemoryEntry | null> {
  const memory = getUnifiedMemory()
  return memory.get(key)
}

/**
 * Close the unified memory connection
 */
export async function closeUnifiedMemory(): Promise<void> {
  if (unifiedMemory) {
    await unifiedMemory.close()
    unifiedMemory = null
    log.debug("closed unified memory connection")
  }
}

// Re-export types for convenience
export type { UnifiedMemory, MemoryEntry, MemoryCategory }

