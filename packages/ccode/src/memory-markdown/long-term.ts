/**
 * Long-term memory management (sediment layer)
 *
 * Consolidated knowledge stored in ./memory/MEMORY.md
 * Categories: 用户偏好, 项目上下文, 关键决策, 经验教训
 */

import { Log } from "@/util/log"
import type { MemoryCategory, MemorySection } from "./types"
import { extractCategory, formatSectionHeader } from "./util"
import { getStorage } from "./storage"

const log = Log.create({ service: "memory-markdown.long-term" })

/**
 * Load entire long-term memory file
 */
export async function loadLongTermMemory(): Promise<string> {
  const storage = getStorage()

  try {
    await ensureMemoryFile()

    const content = await storage.readLongTermMemory()
    return content
  } catch (error) {
    log.warn("failed to load long-term memory, returning empty", { error })
    return getDefaultMemoryContent()
  }
}

/**
 * Load specific category from long-term memory
 */
export async function loadCategory(category: MemoryCategory): Promise<string> {
  const content = await loadLongTermMemory()
  const categoryContent = extractCategory(content, category)

  return categoryContent || formatSectionHeader(category) + "\n_No entries yet._\n"
}

/**
 * Update or create a category in long-term memory
 */
export async function updateCategory(category: MemoryCategory, content: string): Promise<void> {
  const storage = getStorage()

  try {
    await ensureMemoryFile()

    const existing = await storage.readLongTermMemory()
    const existingContent = extractCategory(existing, category)

    const updatedContent = existingContent
      ? replaceCategory(existing, category, content)
      : appendCategory(existing, category, content)

    await storage.writeLongTermMemory(updatedContent)

    log.debug("updated category", { category })
  } catch (error) {
    log.error("failed to update category", { error, category })
    throw error
  }
}

/**
 * Merge new content into existing category
 */
export async function mergeToCategory(category: MemoryCategory, update: string): Promise<void> {
  const existing = await loadCategory(category)
  const merged = smartMerge(category, existing, update)

  await updateCategory(category, merged)
}

/**
 * Get all memory sections as typed objects
 */
export async function getMemorySections(): Promise<MemorySection[]> {
  const content = await loadLongTermMemory()
  const categories: MemoryCategory[] = ["用户偏好", "项目上下文", "关键决策", "经验教训"]

  return categories.map((cat) => ({
    category: cat,
    content: extractCategory(content, cat) || "",
    lastUpdated: new Date().toISOString(),
  }))
}

/**
 * Add item to a category list
 */
export async function addListItem(category: MemoryCategory, item: string, subtext?: string): Promise<void> {
  const existing = await loadCategory(category)

  const header = formatSectionHeader(category)
  const itemEntry = subtext ? `- **${item}**: ${subtext}` : `- ${item}`

  let updated: string

  if (existing.includes(header)) {
    const existingItems = existing.replace(header, "").trim()
    updated = header + "\n" + existingItems + "\n" + itemEntry + "\n"
  } else {
    updated = header + "\n" + itemEntry + "\n"
  }

  await updateCategory(category, updated)
}

/**
 * Remove item from a category
 */
export async function removeListItem(category: MemoryCategory, itemPattern: string): Promise<void> {
  const existing = await loadCategory(category)
  const lines = existing.split("\n")

  const filtered = lines.filter(
    (line) => !line.includes(itemPattern) || line.startsWith("## ") || line.trim() === "─".repeat(40),
  )

  await updateCategory(category, filtered.join("\n"))
}

/**
 * Ensure memory file exists with default structure
 */
async function ensureMemoryFile(): Promise<void> {
  const storage = getStorage()

  try {
    await storage.ensureDir(storage.basePath)

    const exists = await storage.fileExists(storage.longTermPath)
    if (!exists) {
      await storage.writeLongTermMemory(getDefaultMemoryContent())
      log.info("created default MEMORY.md")
    }
  } catch {
    // Directory or file may already exist
  }
}

/**
 * Replace existing category with new content
 */
function replaceCategory(content: string, category: MemoryCategory, newContent: string): string {
  const lines = content.split("\n")
  const result: string[] = []
  let inCategory = false
  let skipped = false

  for (const line of lines) {
    if (line.startsWith(`## ${category}`)) {
      inCategory = true
      result.push(line)
      result.push(newContent.split("\n").slice(1).join("\n"))
      skipped = true
      continue
    }

    if (inCategory) {
      if (line.startsWith("## ")) {
        inCategory = false
        result.push(line)
      }
      continue
    }

    result.push(line)
  }

  if (!skipped) {
    return appendCategory(content, category, newContent)
  }

  return result.join("\n")
}

/**
 * Append new category to end of file
 */
function appendCategory(content: string, category: MemoryCategory, newContent: string): string {
  const trimmedContent = content.trimEnd()
  return trimmedContent + "\n\n" + newContent.trim() + "\n"
}

/**
 * Smart merge of category content
 */
function smartMerge(category: MemoryCategory, existing: string, update: string): string {
  const header = formatSectionHeader(category)
  const existingLines = existing
    .replace(header, "")
    .split("\n")
    .filter((l) => l.trim() && !l.startsWith("_No entries"))
  const updateLines = update.split("\n").filter((l) => l.trim())

  const merged = new Set([...existingLines, ...updateLines])

  return header + "\n" + Array.from(merged).join("\n") + "\n"
}

/**
 * Get default memory file content
 */
function getDefaultMemoryContent(): string {
  const storage = getStorage()
  return `# Long-term Memory

Transparent markdown-based memory storage. Last updated: ${new Date().toISOString()}
Project ID: ${storage.projectId}

## 用户偏好
────────────────────────────────────────────────

_No preferences recorded yet._

## 项目上下文
────────────────────────────────────────────────

_No project context recorded yet._

## 关键决策
────────────────────────────────────────────────

_No decisions recorded yet._

## 经验教训
────────────────────────────────────────────────

_No lessons learned yet._
`
}
