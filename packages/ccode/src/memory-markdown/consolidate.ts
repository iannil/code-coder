/**
 * Memory Consolidation
 *
 * Automatically extracts important information from daily notes
 * and consolidates it into long-term memory categories.
 */

import path from "path"
import { Log } from "@/util/log"
import { loadDailyNotes, listDailyNoteDates, getDailyPath } from "./daily"
import { mergeToCategory } from "./long-term"
import { parseDailyNotes } from "./util"
import type { DailyEntry, MemoryCategory } from "./types"

const log = Log.create({ service: "memory-markdown.consolidate" })

/** Consolidation configuration */
export interface ConsolidateOptions {
  /** Number of recent days to process */
  days?: number
  /** Whether to preserve processed entries */
  preserveOriginal?: boolean
  /** Minimum importance score for consolidation (0-1) */
  minImportance?: number
}

/** Extraction result */
export interface ExtractionResult {
  category: MemoryCategory
  entries: ExtractedEntry[]
}

/** Extracted entry with metadata */
export interface ExtractedEntry {
  content: string
  source: string // Date of the daily note
  importance: number
  timestamp: string // ISO 8601 timestamp from DailyEntry
}

/**
 * Consolidate daily notes into long-term memory
 *
 * Analyzes recent daily notes and extracts important information
 * into appropriate categories (用户偏好, 项目上下文, 关键决策, 经验教训)
 */
export async function consolidateMemory(options: ConsolidateOptions = {}): Promise<ExtractionResult[]> {
  const { days = 7, minImportance = 0.5 } = options

  log.info("starting memory consolidation", { days, minImportance })

  try {
    // Get recent daily note dates
    const dates = await listDailyNoteDates()
    const recentDates = dates.slice(-days)

    if (recentDates.length === 0) {
      log.info("no daily notes found for consolidation")
      return []
    }

    // Load all recent daily notes
    const allNotes: Array<{ date: string; entries: DailyEntry[] }> = []
    for (const date of recentDates) {
      try {
        const noteContents = await loadDailyNotes(new Date(date))
        for (const content of noteContents) {
          const parsedEntries = parseDailyNotes(content)
          allNotes.push({ date, entries: parsedEntries })
        }
      } catch (error) {
        log.warn("failed to load daily notes", { date, error })
      }
    }

    // Extract important entries by category
    const extractions = await extractImportantEntries(allNotes, minImportance)

    // Merge extractions into long-term memory
    const results: ExtractionResult[] = []
    for (const extraction of extractions) {
      if (extraction.entries.length === 0) continue

      const consolidated = extraction.entries.map((e) => e.content).join("\n")
      await mergeToCategory(extraction.category, consolidated)

      results.push(extraction)
      log.debug("consolidated category", {
        category: extraction.category,
        count: extraction.entries.length,
      })
    }

    log.info("consolidation complete", {
      totalCategories: results.length,
      totalEntries: results.reduce((sum, r) => sum + r.entries.length, 0),
    })

    return results
  } catch (error) {
    log.error("consolidation failed", { error })
    throw error
  }
}

/**
 * Extract important entries from daily notes
 */
async function extractImportantEntries(
  notes: Array<{ date: string; entries: DailyEntry[] }>,
  minImportance: number,
): Promise<ExtractionResult[]> {
  const extractions: Map<MemoryCategory, ExtractedEntry[]> = new Map()

  // Initialize extraction maps
  const categories: MemoryCategory[] = ["用户偏好", "项目上下文", "关键决策", "经验教训"]
  for (const category of categories) {
    extractions.set(category, [])
  }

  // Process each daily note
  for (const { date, entries } of notes) {
    for (const entry of entries) {
      const importance = calculateImportance(entry)
      if (importance < minImportance) continue

      const category = categorizeEntry(entry)
      if (!category) continue

      const extracted: ExtractedEntry = {
        content: formatExtractedEntry(entry, date),
        source: date,
        importance,
        timestamp: entry.timestamp,
      }

      const existing = extractions.get(category) ?? []
      existing.push(extracted)
      extractions.set(category, existing)
    }
  }

  // Convert map to array and deduplicate
  return Array.from(extractions.entries())
    .filter(([_, entries]) => entries.length > 0)
    .map(([category, entries]) => ({
      category,
      entries: deduplicateEntries(entries),
    }))
}

/**
 * Consolidation-specific entry types (extends base types)
 */
type ConsolidationEntryType = "decision" | "preference" | "lesson" | "task" | "note" | "error"

/**
 * Calculate importance score for an entry (0-1)
 */
function calculateImportance(entry: DailyEntry): number {
  let score = 0

  // Entry type base scores
  const typeScores: Record<ConsolidationEntryType, number> = {
    decision: 0.8,
    preference: 0.7,
    lesson: 0.9,
    task: 0.3,
    note: 0.2,
    error: 0.6,
  }

  score += typeScores[entry.type as ConsolidationEntryType] ?? 0.5

  // Content-based boosts
  const content = entry.content.toLowerCase()

  // Keywords indicating importance
  const importantKeywords = [
    "critical",
    "important",
    "key",
    "must",
    "always",
    "never",
    "best practice",
    "architecture",
    "design",
    "约定",
    "重要",
    "必须",
    "关键",
  ]

  for (const keyword of importantKeywords) {
    if (content.includes(keyword)) {
      score += 0.1
    }
  }

  // Length matters (very short entries are less important)
  if (entry.content.length > 100) {
    score += 0.1
  }
  if (entry.content.length > 300) {
    score += 0.1
  }

  return Math.min(score, 1)
}

/**
 * Categorize an entry into a memory category
 */
function categorizeEntry(entry: DailyEntry): MemoryCategory | null {
  const content = entry.content.toLowerCase()
  const type = entry.type as ConsolidationEntryType

  // Explicit category mapping from type
  if (type === "preference") return "用户偏好"
  if (type === "decision") return "关键决策"
  if (type === "lesson") return "经验教训"

  // Content-based classification
  if (content.includes("prefer") || content.includes("偏好") || content.includes("喜欢")) {
    return "用户偏好"
  }

  if (
    content.includes("decided") ||
    content.includes("decision") ||
    content.includes("决定") ||
    content.includes("选择")
  ) {
    return "关键决策"
  }

  if (
    content.includes("learned") ||
    content.includes("lesson") ||
    content.includes("经验") ||
    content.includes("教训") ||
    content.includes("mistake")
  ) {
    return "经验教训"
  }

  if (
    content.includes("project") ||
    content.includes("项目") ||
    content.includes("architecture") ||
    content.includes("架构")
  ) {
    return "项目上下文"
  }

  // Default to project context for general notes
  if (type === "note" || type === "task") {
    return "项目上下文"
  }

  return null
}

/**
 * Format an extracted entry for long-term storage
 */
function formatExtractedEntry(entry: DailyEntry, date: string): string {
  const timestamp = new Date(entry.timestamp ?? Date.now()).toISOString().split("T")[0]
  const prefix = entry.type ? `[${entry.type}] ` : ""

  return `- ${prefix}${entry.content.trim()} (${timestamp})`
}

/**
 * Remove duplicate entries based on content similarity
 */
function deduplicateEntries(entries: ExtractedEntry[]): ExtractedEntry[] {
  const seen = new Set<string>()
  const unique: ExtractedEntry[] = []

  for (const entry of entries) {
    // Normalize content for comparison
    const normalized = entry.content
      .toLowerCase()
      .replace(/\[\w+\]\s*/, "") // Remove type tags
      .replace(/\s*\(\d{4}-\d{2}-\d{2}\)\s*$/, "") // Remove dates
      .trim()

    if (!seen.has(normalized)) {
      seen.add(normalized)
      unique.push(entry)
    }
  }

  return unique.sort((a, b) => b.importance - a.importance)
}

/**
 * Get consolidation statistics
 */
export async function getConsolidationStats(): Promise<{
  totalDailyNotes: number
  lastConsolidated: string | null
  pendingEntries: number
}> {
  try {
    const dates = await listDailyNoteDates()

    return {
      totalDailyNotes: dates.length,
      lastConsolidated: dates[dates.length - 1] ?? null,
      // Note: Tracking processed entries would require modifying storage format
      // Current implementation uses deduplication based on content similarity
      pendingEntries: 0,
    }
  } catch (error) {
    log.warn("failed to get consolidation stats", { error })
    return {
      totalDailyNotes: 0,
      lastConsolidated: null,
      pendingEntries: 0,
    }
  }
}
