/**
 * Markdown Memory Backend
 *
 * Transparent dual-layer memory architecture using markdown files:
 * - Flow layer (daily): Chronological log in ./memory/daily/{YYYY-MM-DD}.md
 * - Sediment layer (long-term): Consolidated knowledge in ./memory/MEMORY.md
 *
 * Human-readable and Git-friendly storage format.
 *
 * @module memory/backends/markdown
 */

import path from "path"
import type { MarkdownConfig, MemoryCategory, MemoryEntry, UnifiedMemory } from "../types"
import { DEFAULT_CONFIG } from "../types"

/**
 * Daily entry types for categorization
 */
type DailyEntryType = "decision" | "action" | "output" | "error"

/**
 * Map unified memory categories to markdown categories
 */
const CATEGORY_MAP: Record<string, string> = {
  core: "项目上下文",
  preference: "用户偏好",
  decision: "关键决策",
  lesson: "经验教训",
  daily: "daily",
  conversation: "项目上下文",
}

/**
 * Reverse map from markdown categories to unified categories
 */
const REVERSE_CATEGORY_MAP: Record<string, MemoryCategory> = {
  用户偏好: "preference",
  项目上下文: "core",
  关键决策: "decision",
  经验教训: "lesson",
}

/**
 * Markdown Memory Backend
 *
 * Stores memories in human-readable markdown format.
 */
export class MarkdownMemory implements UnifiedMemory {
  readonly name = "markdown"
  readonly basePath: string
  readonly longTermFile: string
  readonly dailyDir: string
  readonly projectId: string
  private initialized = false

  constructor(config: MarkdownConfig = {}) {
    const defaults = DEFAULT_CONFIG.markdown
    this.basePath = this.resolvePath(config.basePath ?? defaults.basePath)
    this.longTermFile = config.longTermFile ?? defaults.longTermFile
    this.dailyDir = config.dailyDir ?? defaults.dailyDir
    this.projectId = config.projectId ?? this.detectProjectId()
  }

  /**
   * Resolve path with ~ expansion
   */
  private resolvePath(p: string): string {
    if (p.startsWith("~/")) {
      const home = process.env.HOME ?? process.env.USERPROFILE ?? ""
      return path.join(home, p.slice(2))
    }
    if (path.isAbsolute(p)) {
      return p
    }
    return path.join(process.cwd(), p)
  }

  /**
   * Detect project ID from git or directory name
   */
  private detectProjectId(): string {
    try {
      const result = Bun.spawnSync(["git", "rev-parse", "--show-toplevel"])
      if (result.success) {
        const gitRoot = result.stdout.toString().trim()
        return path.basename(gitRoot)
      }
    } catch {
      // Not a git repo
    }
    return path.basename(process.cwd())
  }

  /**
   * Get path to long-term memory file
   */
  private get longTermPath(): string {
    return path.join(this.basePath, this.longTermFile)
  }

  /**
   * Get path to daily notes directory
   */
  private get dailyPath(): string {
    return path.join(this.basePath, this.dailyDir)
  }

  /**
   * Ensure base directories exist
   */
  private async ensureInit(): Promise<void> {
    if (this.initialized) return

    try {
      Bun.spawnSync(["mkdir", "-p", this.basePath])
      Bun.spawnSync(["mkdir", "-p", this.dailyPath])

      // Create default MEMORY.md if it doesn't exist
      const file = Bun.file(this.longTermPath)
      if (!file.size) {
        await Bun.write(this.longTermPath, this.getDefaultMemoryContent())
      }

      this.initialized = true
    } catch {
      // Directories may already exist
      this.initialized = true
    }
  }

  /**
   * Format date as YYYY-MM-DD
   */
  private formatDate(date: Date): string {
    const year = date.getFullYear()
    const month = String(date.getMonth() + 1).padStart(2, "0")
    const day = String(date.getDate()).padStart(2, "0")
    return `${year}-${month}-${day}`
  }

  /**
   * Generate unique ID for memory entry
   */
  private generateId(): string {
    return crypto.randomUUID()
  }

  /**
   * Get current ISO timestamp
   */
  private now(): string {
    return new Date().toISOString()
  }

  /**
   * Map unified category to markdown category
   */
  private mapCategory(category: MemoryCategory): string {
    return CATEGORY_MAP[category] ?? category
  }

  async store(key: string, content: string, category: MemoryCategory): Promise<void> {
    await this.ensureInit()

    const mdCategory = this.mapCategory(category)

    if (mdCategory === "daily" || category === "daily") {
      await this.appendDailyNote(key, content)
    } else {
      await this.mergeToCategory(mdCategory, key, content)
    }
  }

  /**
   * Append entry to today's daily notes
   */
  private async appendDailyNote(key: string, content: string): Promise<void> {
    const today = new Date()
    const filename = `${this.formatDate(today)}.md`
    const filePath = path.join(this.dailyPath, filename)

    const timestamp = this.now()
    const entry = `### ${timestamp}\n**${key}**\n\n${content}\n`

    try {
      const file = Bun.file(filePath)
      if (file.size > 0) {
        const existing = await file.text()
        await Bun.write(filePath, existing.trimEnd() + "\n\n" + entry)
      } else {
        const header = `# Daily Notes - ${this.formatDate(today)}\n\n`
        await Bun.write(filePath, header + entry)
      }
    } catch {
      const header = `# Daily Notes - ${this.formatDate(today)}\n\n`
      await Bun.write(filePath, header + entry)
    }
  }

  /**
   * Merge content into long-term memory category
   */
  private async mergeToCategory(category: string, key: string, content: string): Promise<void> {
    const filePath = this.longTermPath
    const timestamp = this.now()
    const newEntry = `- **${key}**: ${content}\n  _[${timestamp}]_\n`

    try {
      const file = Bun.file(filePath)
      let fileContent = file.size > 0 ? await file.text() : this.getDefaultMemoryContent()

      // Find and update the category section
      const categoryHeader = `## ${category}`
      const categoryIndex = fileContent.indexOf(categoryHeader)

      if (categoryIndex === -1) {
        // Append new category
        fileContent = fileContent.trimEnd() + `\n\n${categoryHeader}\n${this.getSeparator()}\n\n${newEntry}`
      } else {
        // Find the end of this category section (next ## or end of file)
        const afterHeader = categoryIndex + categoryHeader.length
        const nextCategoryIndex = fileContent.indexOf("\n## ", afterHeader)
        const insertPoint = nextCategoryIndex === -1 ? fileContent.length : nextCategoryIndex

        // Check if key already exists in this section
        const sectionContent = fileContent.slice(categoryIndex, insertPoint)
        const keyPattern = new RegExp(`- \\*\\*${this.escapeRegex(key)}\\*\\*:`)

        if (keyPattern.test(sectionContent)) {
          // Replace existing entry
          const lines = sectionContent.split("\n")
          const updatedLines: string[] = []
          let skipNext = false

          for (const line of lines) {
            if (skipNext && line.startsWith("  _[")) {
              skipNext = false
              continue
            }
            if (keyPattern.test(line)) {
              updatedLines.push(`- **${key}**: ${content}`)
              updatedLines.push(`  _[${timestamp}]_`)
              skipNext = true
              continue
            }
            updatedLines.push(line)
          }

          fileContent = fileContent.slice(0, categoryIndex) + updatedLines.join("\n") + fileContent.slice(insertPoint)
        } else {
          // Find where to insert (after separator line or after header)
          const separatorIndex = sectionContent.indexOf(this.getSeparator())
          const insertOffset =
            separatorIndex !== -1 ? separatorIndex + this.getSeparator().length + 1 : categoryHeader.length + 1

          const actualInsertPoint = categoryIndex + insertOffset
          const before = fileContent.slice(0, actualInsertPoint)
          const after = fileContent.slice(actualInsertPoint)

          fileContent = before + "\n" + newEntry + after
        }
      }

      await Bun.write(filePath, fileContent)
    } catch {
      await Bun.write(filePath, this.getDefaultMemoryContent())
      await this.mergeToCategory(category, key, content)
    }
  }

  /**
   * Escape regex special characters
   */
  private escapeRegex(str: string): string {
    return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
  }

  /**
   * Get section separator
   */
  private getSeparator(): string {
    return "────────────────────────────────────────────────"
  }

  async recall(query: string, limit: number = 10): Promise<MemoryEntry[]> {
    await this.ensureInit()

    const results: MemoryEntry[] = []
    const keywords = query.toLowerCase().split(/\s+/).filter(Boolean)

    if (keywords.length === 0) return []

    // Search long-term memory
    const longTermEntries = await this.searchLongTerm(keywords, limit)
    results.push(...longTermEntries)

    // Search recent daily notes
    const dailyEntries = await this.searchDaily(keywords, Math.max(1, limit - results.length))
    results.push(...dailyEntries)

    // Sort by score and limit
    return results.sort((a, b) => (b.score ?? 0) - (a.score ?? 0)).slice(0, limit)
  }

  /**
   * Search long-term memory for keywords
   */
  private async searchLongTerm(keywords: string[], limit: number): Promise<MemoryEntry[]> {
    const results: MemoryEntry[] = []

    try {
      const file = Bun.file(this.longTermPath)
      if (!file.size) return []

      const content = await file.text()
      const lines = content.split("\n")
      let currentCategory = "core"
      let currentEntry: { key: string; content: string; timestamp?: string } | null = null

      for (const line of lines) {
        // Track current category
        if (line.startsWith("## ")) {
          currentCategory = REVERSE_CATEGORY_MAP[line.slice(3).trim()] ?? "core"
          continue
        }

        // Parse entry
        const entryMatch = line.match(/^- \*\*(.+?)\*\*: (.+)$/)
        if (entryMatch) {
          if (currentEntry) {
            const score = this.calculateScore(currentEntry.key + " " + currentEntry.content, keywords)
            if (score > 0) {
              results.push({
                id: this.generateId(),
                key: currentEntry.key,
                content: currentEntry.content,
                category: currentCategory,
                timestamp: currentEntry.timestamp ?? this.now(),
                score,
                source: "markdown",
              })
            }
          }
          currentEntry = { key: entryMatch[1], content: entryMatch[2] }
          continue
        }

        // Parse timestamp
        const timestampMatch = line.match(/^\s+_\[(.+?)\]_$/)
        if (timestampMatch && currentEntry) {
          currentEntry.timestamp = timestampMatch[1]
        }
      }

      // Don't forget the last entry
      if (currentEntry) {
        const score = this.calculateScore(currentEntry.key + " " + currentEntry.content, keywords)
        if (score > 0) {
          results.push({
            id: this.generateId(),
            key: currentEntry.key,
            content: currentEntry.content,
            category: currentCategory,
            timestamp: currentEntry.timestamp ?? this.now(),
            score,
            source: "markdown",
          })
        }
      }
    } catch {
      // File doesn't exist or can't be read
    }

    return results.slice(0, limit)
  }

  /**
   * Search daily notes for keywords
   */
  private async searchDaily(keywords: string[], limit: number): Promise<MemoryEntry[]> {
    const results: MemoryEntry[] = []

    try {
      const globber = new Bun.Glob("*.md")
      const files: string[] = []

      for await (const file of globber.scan({ cwd: this.dailyPath })) {
        files.push(file)
      }

      // Sort by date descending (most recent first)
      files.sort().reverse()

      for (const filename of files.slice(0, 7)) {
        // Check last 7 days
        const filePath = path.join(this.dailyPath, filename)
        const content = await Bun.file(filePath).text()

        // Parse daily entries
        const entries = content.split(/^### /m).slice(1)

        for (const entry of entries) {
          const lines = entry.split("\n")
          const timestamp = lines[0]?.trim() ?? ""
          const keyLine = lines[1]?.trim() ?? ""
          const keyMatch = keyLine.match(/^\*\*(.+?)\*\*$/)
          const key = keyMatch ? keyMatch[1] : keyLine
          const entryContent = lines.slice(2).join("\n").trim()

          const score = this.calculateScore(key + " " + entryContent, keywords)
          if (score > 0) {
            results.push({
              id: this.generateId(),
              key,
              content: entryContent,
              category: "daily",
              timestamp,
              score,
              source: "markdown",
            })
          }
        }

        if (results.length >= limit) break
      }
    } catch {
      // Directory doesn't exist or can't be read
    }

    return results.slice(0, limit)
  }

  /**
   * Calculate relevance score based on keyword matches
   * Returns 1.0 when keywords is empty (for listing all entries)
   */
  private calculateScore(text: string, keywords: string[]): number {
    // When no keywords, return 1.0 for listing purposes
    if (keywords.length === 0) return 1.0

    const lowerText = text.toLowerCase()
    let matches = 0

    for (const keyword of keywords) {
      if (lowerText.includes(keyword)) {
        matches++
      }
    }

    return matches / keywords.length
  }

  async get(key: string): Promise<MemoryEntry | null> {
    await this.ensureInit()

    // Search long-term memory first
    try {
      const file = Bun.file(this.longTermPath)
      if (file.size > 0) {
        const content = await file.text()
        const keyPattern = new RegExp(`^- \\*\\*${this.escapeRegex(key)}\\*\\*: (.+)$`, "m")
        const match = content.match(keyPattern)

        if (match) {
          // Find category by looking backwards from match
          const beforeMatch = content.slice(0, match.index)
          const categoryHeaders = beforeMatch.match(/^## (.+)$/gm)
          const lastCategory = categoryHeaders ? categoryHeaders[categoryHeaders.length - 1] : null
          const category = lastCategory ? REVERSE_CATEGORY_MAP[lastCategory.slice(3)] ?? "core" : "core"

          // Find timestamp
          const afterMatch = content.slice((match.index ?? 0) + match[0].length)
          const timestampMatch = afterMatch.match(/^\s+_\[(.+?)\]_/)

          return {
            id: this.generateId(),
            key,
            content: match[1],
            category,
            timestamp: timestampMatch ? timestampMatch[1] : this.now(),
            source: "markdown",
          }
        }
      }
    } catch {
      // File doesn't exist
    }

    // Search daily notes
    try {
      const globber = new Bun.Glob("*.md")

      for await (const filename of globber.scan({ cwd: this.dailyPath })) {
        const filePath = path.join(this.dailyPath, filename)
        const content = await Bun.file(filePath).text()

        const keyPattern = new RegExp(`\\*\\*${this.escapeRegex(key)}\\*\\*`, "m")
        if (keyPattern.test(content)) {
          const entries = content.split(/^### /m).slice(1)

          for (const entry of entries) {
            if (keyPattern.test(entry)) {
              const lines = entry.split("\n")
              const timestamp = lines[0]?.trim() ?? ""
              const entryContent = lines.slice(2).join("\n").trim()

              return {
                id: this.generateId(),
                key,
                content: entryContent,
                category: "daily",
                timestamp,
                source: "markdown",
              }
            }
          }
        }
      }
    } catch {
      // Directory doesn't exist
    }

    return null
  }

  async list(category?: MemoryCategory): Promise<MemoryEntry[]> {
    await this.ensureInit()

    const results: MemoryEntry[] = []

    // List from long-term memory
    if (!category || category !== "daily") {
      const longTermEntries = await this.searchLongTerm([], 1000)
      results.push(...(category ? longTermEntries.filter((e) => e.category === category) : longTermEntries))
    }

    // List from daily notes
    if (!category || category === "daily") {
      const dailyEntries = await this.searchDaily([], 1000)
      results.push(...dailyEntries)
    }

    return results
  }

  async forget(key: string): Promise<boolean> {
    await this.ensureInit()

    let found = false

    // Remove from long-term memory
    try {
      const file = Bun.file(this.longTermPath)
      if (file.size > 0) {
        const content = await file.text()
        const keyPattern = new RegExp(`^- \\*\\*${this.escapeRegex(key)}\\*\\*: .+\\n(\\s+_\\[.+?\\]_\\n)?`, "gm")

        if (keyPattern.test(content)) {
          const updated = content.replace(keyPattern, "")
          await Bun.write(this.longTermPath, updated)
          found = true
        }
      }
    } catch {
      // File doesn't exist
    }

    return found
  }

  async count(): Promise<number> {
    await this.ensureInit()

    let count = 0

    // Count long-term entries
    try {
      const file = Bun.file(this.longTermPath)
      if (file.size > 0) {
        const content = await file.text()
        const matches = content.match(/^- \*\*.+?\*\*:/gm)
        count += matches ? matches.length : 0
      }
    } catch {
      // File doesn't exist
    }

    // Count daily entries
    try {
      const globber = new Bun.Glob("*.md")

      for await (const filename of globber.scan({ cwd: this.dailyPath })) {
        const filePath = path.join(this.dailyPath, filename)
        const content = await Bun.file(filePath).text()
        const matches = content.match(/^### /gm)
        count += matches ? matches.length : 0
      }
    } catch {
      // Directory doesn't exist
    }

    return count
  }

  async healthCheck(): Promise<boolean> {
    try {
      await this.ensureInit()
      const file = Bun.file(this.longTermPath)
      return file.size >= 0
    } catch {
      return false
    }
  }

  async close(): Promise<void> {
    // No resources to release for markdown backend
    this.initialized = false
  }

  /**
   * Get default memory file content
   */
  private getDefaultMemoryContent(): string {
    return `# Long-term Memory

Transparent markdown-based memory storage. Last updated: ${this.now()}
Project ID: ${this.projectId}

## 用户偏好
${this.getSeparator()}

_No preferences recorded yet._

## 项目上下文
${this.getSeparator()}

_No project context recorded yet._

## 关键决策
${this.getSeparator()}

_No decisions recorded yet._

## 经验教训
${this.getSeparator()}

_No lessons learned yet._
`
  }

  /**
   * Get base path for diagnostics
   */
  getBasePath(): string {
    return this.basePath
  }
}

/**
 * Create a Markdown memory backend with default configuration
 */
export function createMarkdownMemory(config?: MarkdownConfig): MarkdownMemory {
  return new MarkdownMemory(config)
}
