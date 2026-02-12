/**
 * Daily notes management (flow layer)
 *
 * Immutable chronological log stored in ./memory/daily/{YYYY-MM-DD}.md
 */

import path from "path"
import { Log } from "@/util/log"
import type { DailyEntry } from "./types"
import { formatDate, formatTimestamp, formatDailyEntry } from "./util"
import { getStorage } from "./storage"

const log = Log.create({ service: "memory-markdown.daily" })

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
 */
export async function appendDailyNote(entry: DailyEntry): Promise<void> {
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
 */
export async function getTodayNotes(): Promise<string> {
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
 */
export async function listDailyNoteDates(): Promise<string[]> {
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
