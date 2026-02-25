/**
 * Context loader for markdown memory layer
 *
 * Combines flow layer (daily) and sediment layer (long-term)
 * to provide comprehensive context for agents
 */

import { Log } from "@/util/log"
import type { LoadOptions, MemoryContext } from "./types"
import { loadDailyNotes } from "./daily"
import { loadLongTermMemory, loadCategory } from "./long-term"
import type { MemoryCategory } from "./types"

const log = Log.create({ service: "memory-markdown.loader" })

/**
 * Load complete markdown memory context
 */
export async function loadMarkdownMemoryContext(options: LoadOptions = {}): Promise<MemoryContext> {
  const { includeDays = 3, categories } = options

  try {
    const [longTermContent, dailyNotes] = await Promise.all([
      loadLongTermMemory(),
      loadDailyNotes(new Date(), includeDays),
    ])

    const filteredDaily = categories ? filterByCategories(dailyNotes, categories) : dailyNotes
    const combined = formatCombinedContext(longTermContent, filteredDaily)

    log.debug("loaded markdown memory context", {
      longTermLength: longTermContent.length,
      dailyCount: dailyNotes.length,
    })

    return {
      longTerm: longTermContent,
      daily: filteredDaily,
      combined,
    }
  } catch (error) {
    log.warn("failed to load complete context, returning minimal", { error })
    return {
      longTerm: "# Long-term Memory\n\n_No content yet._\n",
      daily: [],
      combined: "# Memory Context\n\n_No content available._\n",
    }
  }
}

/**
 * Load context for specific categories only
 */
export async function loadCategoryContext(categories: MemoryCategory[]): Promise<string> {
  const sections = await Promise.all(
    categories.map(async (cat) => {
      const content = await loadCategory(cat)
      return content
    }),
  )

  return sections.join("\n\n")
}

/**
 * Load recent context for agent prompt injection
 */
export async function loadRecentContext(days = 1): Promise<string> {
  const { includeDays = days } = { includeDays: days }
  const { combined } = await loadMarkdownMemoryContext({ includeDays })

  return truncateForPrompt(combined, 4000)
}

/**
 * Get formatted summary of memory state
 */
export async function getMemorySummary(): Promise<{
  longTermSize: number
  dailyNoteCount: number
  lastUpdated: string
}> {
  try {
    const longTerm = await loadLongTermMemory()
    const dailyNotes = await loadDailyNotes(new Date(), 30)

    return {
      longTermSize: longTerm.length,
      dailyNoteCount: dailyNotes.length,
      lastUpdated: new Date().toISOString(),
    }
  } catch {
    return {
      longTermSize: 0,
      dailyNoteCount: 0,
      lastUpdated: new Date().toISOString(),
    }
  }
}

/**
 * Format combined context for agent consumption
 */
function formatCombinedContext(longTerm: string, daily: string[]): string {
  const parts: string[] = []

  parts.push("# Memory Context")
  parts.push("")
  parts.push("## Long-term Memory (Sediment Layer)")
  parts.push("")
  parts.push(longTerm)
  parts.push("")
  parts.push("## Recent Activity (Flow Layer)")

  if (daily.length > 0) {
    for (const note of daily) {
      parts.push("")
      parts.push(note)
    }
  } else {
    parts.push("")
    parts.push("_No recent activity._")
  }

  return parts.join("\n")
}

/**
 * Filter daily notes by category relevance
 */
function filterByCategories(notes: string[], categories: MemoryCategory[]): string[] {
  if (categories.length === 0) return notes

  const keywords = new Set<string>()
  const categoryKeywords: Record<MemoryCategory, string[]> = {
    用户偏好: ["prefer", "config", "setting", "style", "format"],
    项目上下文: ["project", "context", "structure", "architecture"],
    关键决策: ["decision", "chose", "selected", "decided"],
    经验教训: ["learned", "lesson", "mistake", "fix", "issue"],
    成功方案: ["solution", "solved", "success", "resolved", "pattern"],
  }

  for (const cat of categories) {
    const words = categoryKeywords[cat] || []
    for (const word of words) {
      keywords.add(word.toLowerCase())
    }
  }

  return notes.filter((note) => {
    const lower = note.toLowerCase()
    for (const keyword of keywords) {
      if (lower.includes(keyword)) return true
    }
    return false
  })
}

/**
 * Truncate content for prompt injection
 */
function truncateForPrompt(content: string, maxLength: number): string {
  if (content.length <= maxLength) return content

  const lines = content.split("\n")
  let length = 0
  const result: string[] = []

  for (const line of lines) {
    if (length + line.length > maxLength) {
      result.push("\n_... (truncated for context length) _")
      break
    }
    result.push(line)
    length += line.length + 1
  }

  return result.join("\n")
}
