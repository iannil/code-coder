/**
 * Daily notes management (flow layer)
 *
 * Immutable chronological log stored in ./memory/daily/{YYYY-MM-DD}.md
 *
 * Uses NAPI (Rust) backend when available for high performance,
 * falls back to local filesystem storage otherwise.
 */

import path from "path"
import { Log } from "@/util/log"
import type { DailyEntry, DailyEntryType } from "./types"
import { formatDate, formatTimestamp, formatDailyEntry } from "./util"
import { getStorage, getNapiMemoryHandle } from "./storage"
import type { NapiDailyEntryType } from "@codecoder-ai/core"

const log = Log.create({ service: "memory-markdown.daily" })

/**
 * Map TypeScript entry type to NAPI format
 * Uses double cast due to const enum with verbatimModuleSyntax
 */
function mapEntryTypeToNapi(entryType: DailyEntryType): NapiDailyEntryType {
  const mapping: Record<DailyEntryType, string> = {
    decision: "Decision",
    action: "Action",
    output: "Output",
    error: "Error",
    solution: "Solution",
  }
  return (mapping[entryType] ?? "Action") as unknown as NapiDailyEntryType
}

/**
 * Get daily note file path for a given date
 */
export function getDailyPath(date: Date): string {
  const storage = getStorage()
  const filename = `${formatDate(date)}.md`
  return path.join(storage.dailyPath, filename)
}

/**
 * Append a new entry to today's daily notes
 *
 * Uses NAPI backend when available for better performance.
 */
export async function appendDailyNote(entry: DailyEntry): Promise<void> {
  // Try NAPI first (high performance Rust backend)
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      const metadataStr = entry.metadata ? JSON.stringify({
        ...entry.metadata,
        projectId: getStorage().projectId,
        projectPath: process.cwd(),
      }) : undefined

      napiHandle.appendDailyNote(
        mapEntryTypeToNapi(entry.type),
        entry.content,
        metadataStr,
      )
      log.debug("appended daily note via NAPI", { type: entry.type })
      return
    } catch (error) {
      log.warn("NAPI appendDailyNote failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
  const storage = getStorage()
  const today = new Date()
  const dailyPath = getDailyPath(today)

  try {
    await storage.ensureDir(storage.dailyPath)

    const timestamp = entry.timestamp || formatTimestamp(today)
    // Auto-inject project context into metadata
    const entryWithProject = {
      ...entry,
      timestamp,
      metadata: {
        ...entry.metadata,
        projectId: storage.projectId,
        projectPath: process.cwd(),
      },
    }

    const markdown = formatDailyEntry(entryWithProject)

    const exists = await storage.fileExists(dailyPath)

    if (exists) {
      const existingContent = await Bun.file(dailyPath).text()
      const updatedContent = existingContent.trimEnd() + "\n\n" + markdown + "\n"
      await Bun.write(dailyPath, updatedContent)
    } else {
      const header = `# Daily Notes - ${formatDate(today)}\n\n`
      await Bun.write(dailyPath, header + markdown + "\n")
    }

    log.debug("appended daily note", { path: dailyPath, type: entry.type })
  } catch (error) {
    log.error("failed to append daily note", { error, path: dailyPath })
    throw error
  }
}

/**
 * Load daily notes for a specific date range
 */
export async function loadDailyNotes(startDate: Date, days = 1): Promise<string[]> {
  const storage = getStorage()
  const notes: string[] = []

  for (let i = 0; i < days; i++) {
    const date = new Date(startDate)
    date.setDate(date.getDate() + i)
    const dailyPath = getDailyPath(date)

    try {
      const exists = await storage.fileExists(dailyPath)
      if (exists) {
        const content = await Bun.file(dailyPath).text()
        notes.push(content)
      }
    } catch (error) {
      log.warn("failed to load daily note", { error, date: formatDate(date) })
    }
  }

  return notes
}

/**
 * Get content from today's daily notes
 *
 * Uses NAPI backend when available.
 */
export async function getTodayNotes(): Promise<string> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      const content = napiHandle.getTodayNotes()
      if (content) return content
    } catch (error) {
      log.warn("NAPI getTodayNotes failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
  const today = new Date()
  const dailyPath = getDailyPath(today)

  try {
    const storage = getStorage()
    const exists = await storage.fileExists(dailyPath)
    if (exists) {
      return await Bun.file(dailyPath).text()
    }
    return `# Daily Notes - ${formatDate(today)}\n\n_No entries yet._\n`
  } catch (error) {
    log.warn("failed to get today's notes", { error })
    return ""
  }
}

/**
 * List all available daily note dates
 *
 * Uses NAPI backend when available.
 */
export async function listDailyNoteDates(): Promise<string[]> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      return napiHandle.listDailyNoteDates()
    } catch (error) {
      log.warn("NAPI listDailyNoteDates failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
  const storage = getStorage()
  return await storage.listDailyNotes()
}

/**
 * Create a daily note entry helper
 */
export function createEntry(
  type: DailyEntry["type"],
  content: string,
  metadata?: Record<string, unknown>,
): DailyEntry {
  return {
    timestamp: new Date().toISOString(),
    type,
    content,
    metadata,
  }
}
