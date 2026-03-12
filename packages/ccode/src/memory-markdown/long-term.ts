/**
 * Long-term memory management (sediment layer)
 *
 * Consolidated knowledge stored in ./memory/MEMORY.md
 * Categories: 用户偏好, 项目上下文, 关键决策, 经验教训, 成功方案
 *
 * Uses NAPI (Rust) backend when available for high performance,
 * falls back to local filesystem storage otherwise.
 */

import { Log } from "@/util/log"
import type { MemoryCategory, MemorySection } from "./types"
import { extractCategory, formatSectionHeader } from "./util"
import { getStorage, getNapiMemoryHandle } from "./storage"
import type { NapiMemoryCategory } from "@codecoder-ai/core"

const log = Log.create({ service: "memory-markdown.long-term" })

/**
 * Map TypeScript category to NAPI format
 * Uses double cast due to const enum with verbatimModuleSyntax
 */
function mapCategoryToNapi(category: MemoryCategory): NapiMemoryCategory {
  const mapping: Record<MemoryCategory, string> = {
    用户偏好: "UserPreferences",
    项目上下文: "ProjectContext",
    关键决策: "KeyDecisions",
    经验教训: "LessonsLearned",
    成功方案: "SuccessPatterns",
  }
  return (mapping[category] ?? "ProjectContext") as unknown as NapiMemoryCategory
}

/**
 * Load entire long-term memory file
 *
 * Uses NAPI backend when available.
 */
export async function loadLongTermMemory(): Promise<string> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      const content = napiHandle.loadLongTermMemory()
      if (content) return content
    } catch (error) {
      log.warn("NAPI loadLongTermMemory failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
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
 *
 * Uses NAPI backend when available.
 */
export async function loadCategory(category: MemoryCategory): Promise<string> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      const content = napiHandle.loadCategory(mapCategoryToNapi(category))
      if (content) return content
    } catch (error) {
      log.warn("NAPI loadCategory failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
  const content = await loadLongTermMemory()
  const categoryContent = extractCategory(content, category)

  return categoryContent || formatSectionHeader(category) + "\n_No entries yet._\n"
}

/**
 * Update or create a category in long-term memory
 *
 * Uses NAPI backend when available.
 */
export async function updateCategory(category: MemoryCategory, content: string): Promise<void> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      napiHandle.updateCategory(mapCategoryToNapi(category), content)
      log.debug("updated category via NAPI", { category })
      return
    } catch (error) {
      log.warn("NAPI updateCategory failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
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
 *
 * Uses NAPI backend when available.
 */
export async function mergeToCategory(category: MemoryCategory, update: string): Promise<void> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      napiHandle.mergeToCategory(mapCategoryToNapi(category), update)
      log.debug("merged to category via NAPI", { category })
      return
    } catch (error) {
      log.warn("NAPI mergeToCategory failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
  const existing = await loadCategory(category)
  const merged = smartMerge(category, existing, update)

  await updateCategory(category, merged)
}

/**
 * Get all memory sections as typed objects
 *
 * Uses NAPI backend when available.
 */
export async function getMemorySections(): Promise<MemorySection[]> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      const sections = napiHandle.getMemorySections()
      return sections.map((s: { category: string; content: string; lastUpdated?: string }) => ({
        category: s.category as MemoryCategory,
        content: s.content,
        lastUpdated: s.lastUpdated ?? new Date().toISOString(),
      }))
    } catch (error) {
      log.warn("NAPI getMemorySections failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
  const content = await loadLongTermMemory()
  const categories: MemoryCategory[] = ["用户偏好", "项目上下文", "关键决策", "经验教训", "成功方案"]

  return categories.map((cat) => ({
    category: cat,
    content: extractCategory(content, cat) || "",
    lastUpdated: new Date().toISOString(),
  }))
}

/**
 * Add item to a category list
 *
 * Uses NAPI backend when available.
 */
export async function addListItem(category: MemoryCategory, item: string, subtext?: string): Promise<void> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      napiHandle.addListItem(mapCategoryToNapi(category), item, subtext)
      log.debug("added list item via NAPI", { category, item })
      return
    } catch (error) {
      log.warn("NAPI addListItem failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
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
 *
 * Uses NAPI backend when available.
 */
export async function removeListItem(category: MemoryCategory, itemPattern: string): Promise<void> {
  // Try NAPI first
  const napiHandle = getNapiMemoryHandle()
  if (napiHandle) {
    try {
      napiHandle.removeListItem(mapCategoryToNapi(category), itemPattern)
      log.debug("removed list item via NAPI", { category, itemPattern })
      return
    } catch (error) {
      log.warn("NAPI removeListItem failed, falling back to local storage", { error })
    }
  }

  // Fallback to local storage
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
