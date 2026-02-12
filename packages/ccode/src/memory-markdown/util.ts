/**
 * Utility functions for markdown memory layer
 */

import type { DailyEntry, MemoryCategory } from "./types"

/**
 * Format date as YYYY-MM-DD
 */
export function formatDate(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, "0")
  const day = String(date.getDate()).padStart(2, "0")
  return `${year}-${month}-${day}`
}

/**
 * Format timestamp as ISO 8601
 */
export function formatTimestamp(date: Date): string {
  return date.toISOString()
}

/**
 * Parse date from YYYY-MM-DD string
 */
export function parseDate(dateStr: string): Date {
  const [year, month, day] = dateStr.split("-").map(Number)
  return new Date(year, month - 1, day)
}

/**
 * Format daily entry as markdown
 */
export function formatDailyEntry(entry: DailyEntry): string {
  const time = new Date(entry.timestamp).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  })

  const icon = entryTypeIcon(entry.type)
  const metadata = entry.metadata ? ` ${JSON.stringify(entry.metadata)}` : ""

  return `### [${time}] ${icon} ${entry.type.toUpperCase()}${metadata}\n\n${entry.content}\n`
}

/**
 * Get emoji icon for entry type
 */
function entryTypeIcon(type: DailyEntry["type"]): string {
  const icons = {
    decision: "💭",
    action: "⚡",
    output: "📤",
    error: "❌",
  }
  return icons[type] || "📝"
}

/**
 * Parse daily notes markdown content into DailyEntry array
 */
export function parseDailyNotes(content: string): DailyEntry[] {
  const entries: DailyEntry[] = []
  const lines = content.split("\n")

  let currentEntry: Partial<DailyEntry> | null = null
  let contentLines: string[] = []

  for (const line of lines) {
    // Match entry header: ### [HH:MM:SS] emoji TYPE {metadata}
    const headerMatch = line.match(/^### \[(\d{2}:\d{2}:\d{2})\]\s+(.+?)\s+(\w+)(\s+\{.*\})?$/)

    if (headerMatch) {
      // Save previous entry if exists
      if (currentEntry && currentEntry.type && currentEntry.timestamp) {
        entries.push({
          type: currentEntry.type,
          timestamp: currentEntry.timestamp,
          content: contentLines.join("\n").trim(),
          metadata: currentEntry.metadata,
        } as DailyEntry)
      }

      // Start new entry
      const [, time, , type, metadataStr] = headerMatch
      const today = new Date()
      const [hours, minutes, seconds] = time.split(":").map(Number)
      today.setHours(hours, minutes, seconds, 0)

      currentEntry = {
        type: type.toLowerCase() as DailyEntry["type"],
        timestamp: today.toISOString(),
        metadata: metadataStr ? JSON.parse(metadataStr.trim()) : undefined,
      }
      contentLines = []
    } else if (currentEntry) {
      // Skip empty lines at the start of content
      if (contentLines.length > 0 || line.trim() !== "") {
        contentLines.push(line)
      }
    }
  }

  // Don't forget the last entry
  if (currentEntry && currentEntry.type && currentEntry.timestamp) {
    entries.push({
      type: currentEntry.type,
      timestamp: currentEntry.timestamp,
      content: contentLines.join("\n").trim(),
      metadata: currentEntry.metadata,
    } as DailyEntry)
  }

  return entries
}

/**
 * Format memory section header
 */
export function formatSectionHeader(category: MemoryCategory): string {
  const separator = "─".repeat(40)
  return `## ${category}\n${separator}\n`
}

/**
 * Extract category content from MEMORY.md
 */
export function extractCategory(content: string, category: MemoryCategory): string | null {
  const lines = content.split("\n")
  let inCategory = false
  const result: string[] = []

  for (const line of lines) {
    if (line.startsWith(`## ${category}`)) {
      inCategory = true
      result.push(line)
      continue
    }

    if (inCategory) {
      if (line.startsWith("## ") && !line.startsWith(`## ${category}`)) {
        break
      }
      result.push(line)
    }
  }

  return result.length > 0 ? result.join("\n").trim() : null
}

/**
 * Ensure directory exists
 */
export async function ensureDir(path: string): Promise<void> {
  try {
    await Bun.write(path + "/.keep", "")
  } catch {
    // Directory may already exist or failed to create
  }
}

/**
 * Sanitize filename
 */
export function sanitizeFilename(name: string): string {
  return name.replace(/[^a-zA-Z0-9-_]/g, "_")
}

/**
 * Get dates for last N days
 */
export function getLastNDays(n: number): Date[] {
  const dates: Date[] = []
  const today = new Date()

  for (let i = 0; i < n; i++) {
    const date = new Date(today)
    date.setDate(date.getDate() - i)
    dates.push(date)
  }

  return dates
}

/**
 * Add project context metadata to an entry
 *
 * Ensures project ID and path are included in entry metadata.
 */
export function withProjectContext<T extends { metadata?: Record<string, unknown> }>(
  entry: T,
  projectId: string,
  projectPath: string,
): T {
  return {
    ...entry,
    metadata: {
      ...entry.metadata,
      projectId,
      projectPath,
    },
  }
}

/**
 * Format project metadata as a markdown comment
 */
export function formatProjectMetadata(projectId: string, projectPath?: string): string {
  const parts = ["<!--", `  Project ID: ${projectId}`]
  if (projectPath) {
    parts.push(`  Project Path: ${projectPath}`)
  }
  parts.push("-->")
  return parts.join("\n")
}

/**
 * Extract project ID from entry metadata
 */
export function getProjectIdFromEntry(entry: { metadata?: Record<string, unknown> } | undefined): string | undefined {
  return entry?.metadata?.projectId as string | undefined
}

/**
 * Check if an entry is from a specific project
 */
export function isEntryFromProject(
  entry: { metadata?: Record<string, unknown> } | undefined,
  projectId: string,
): boolean {
  return getProjectIdFromEntry(entry) === projectId
}

/**
 * Filter entries by project ID
 */
export function filterEntriesByProject<T extends { metadata?: Record<string, unknown> }>(
  entries: T[],
  projectId: string,
): T[] {
  return entries.filter((entry) => isEntryFromProject(entry, projectId))
}

/**
 * Get unique project IDs from a list of entries
 */
export function getUniqueProjectIds(entries: { metadata?: Record<string, unknown> }[]): string[] {
  const projectIds = new Set<string>()
  for (const entry of entries) {
    const projectId = getProjectIdFromEntry(entry)
    if (projectId) {
      projectIds.add(projectId)
    }
  }
  return Array.from(projectIds)
}
