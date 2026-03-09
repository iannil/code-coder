/**
 * Memory Client for Observer Network
 *
 * Provides persistent storage for observer history using memory-markdown.
 * Stores observation events, patterns, anomalies, and decisions.
 *
 * @module observer/integration/memory-client
 */

import path from "path"
import { Log } from "@/util/log"
import {
  getStorage,
  appendDailyNote,
  createEntry,
  type DailyEntryType,
} from "@/memory-markdown"

const log = Log.create({ service: "observer.integration.memory" })

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface ObserverHistoryEntry {
  id: string
  type: "observation" | "pattern" | "anomaly" | "opportunity" | "decision" | "escalation" | "execution"
  timestamp: Date
  data: unknown
  tags: string[]
  sessionId?: string
}

export interface ObserverMemoryConfig {
  /** Base path for observer memory (default: {memoryPath}/observer) */
  basePath?: string
  /** Enable daily notes (default: true) */
  enableDailyNotes: boolean
  /** Enable file storage (default: true) */
  enableFileStorage: boolean
  /** Session ID for tagging */
  sessionId?: string
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Configuration
// ─────────────────────────────────────────────────────────────────────────────

const DEFAULT_CONFIG: ObserverMemoryConfig = {
  enableDailyNotes: true,
  enableFileStorage: true,
}

// ─────────────────────────────────────────────────────────────────────────────
// Memory Client
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Memory client for persisting observer history.
 */
export class MemoryClient {
  private config: ObserverMemoryConfig
  private observerPath: string
  private initialized = false

  constructor(config: Partial<ObserverMemoryConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
    const storage = getStorage()
    this.observerPath = config.basePath ?? path.join(storage.basePath, "observer")
  }

  /**
   * Initialize the memory client.
   */
  async initialize(): Promise<void> {
    if (this.initialized) return

    // Ensure observer directory exists
    if (this.config.enableFileStorage) {
      await this.ensureDir(this.observerPath)
      await this.ensureDir(path.join(this.observerPath, "events"))
      await this.ensureDir(path.join(this.observerPath, "patterns"))
      await this.ensureDir(path.join(this.observerPath, "decisions"))
    }

    this.initialized = true
    log.debug("Memory client initialized", { path: this.observerPath })
  }

  /**
   * Record an observation event.
   */
  async recordObservation(entry: ObserverHistoryEntry): Promise<void> {
    await this.initialize()

    // Write to daily notes
    if (this.config.enableDailyNotes) {
      await this.writeToDailyNotes(entry)
    }

    // Write to observer-specific storage
    if (this.config.enableFileStorage) {
      await this.writeToStorage(entry)
    }
  }

  /**
   * Record multiple observations in batch.
   */
  async recordBatch(entries: ObserverHistoryEntry[]): Promise<void> {
    await this.initialize()

    // Write all to daily notes
    if (this.config.enableDailyNotes) {
      for (const entry of entries) {
        await this.writeToDailyNotes(entry)
      }
    }

    // Write to storage - group by directory for consistency with recordObservation
    if (this.config.enableFileStorage && entries.length > 0) {
      const byDirectory = new Map<string, ObserverHistoryEntry[]>()

      for (const entry of entries) {
        const subdir = entry.type === "pattern" ? "patterns"
          : entry.type === "decision" || entry.type === "escalation" ? "decisions"
          : "events"

        const existing = byDirectory.get(subdir) || []
        existing.push(entry)
        byDirectory.set(subdir, existing)
      }

      const timestamp = Date.now()
      for (const [subdir, dirEntries] of byDirectory) {
        const filename = `batch-${timestamp}.jsonl`
        const filePath = path.join(this.observerPath, subdir, filename)
        const content = dirEntries.map((e) => JSON.stringify(e)).join("\n") + "\n"
        await Bun.write(filePath, content)
      }
    }
  }

  /**
   * Query history from storage.
   */
  async query(options: {
    type?: ObserverHistoryEntry["type"] | ObserverHistoryEntry["type"][]
    startTime?: Date
    endTime?: Date
    limit?: number
  } = {}): Promise<ObserverHistoryEntry[]> {
    await this.initialize()

    if (!this.config.enableFileStorage) {
      return []
    }

    const results: ObserverHistoryEntry[] = []

    // Determine which directories to scan based on type filter
    const dirsToScan = this.getQueryDirectories(options.type)

    try {
      for (const subdir of dirsToScan) {
        const dir = path.join(this.observerPath, subdir)
        const globber = new Bun.Glob("**/*.jsonl")

        try {
          for await (const file of globber.scan({ cwd: dir })) {
            const content = await Bun.file(path.join(dir, file)).text()
            const lines = content.trim().split("\n")

            for (const line of lines) {
              if (!line.trim()) continue
              try {
                const entry = JSON.parse(line) as ObserverHistoryEntry
                entry.timestamp = new Date(entry.timestamp)

                // Filter by type
                if (options.type) {
                  const types = Array.isArray(options.type) ? options.type : [options.type]
                  if (!types.includes(entry.type)) continue
                }

                // Filter by time
                if (options.startTime && entry.timestamp < options.startTime) continue
                if (options.endTime && entry.timestamp > options.endTime) continue

                results.push(entry)

                if (options.limit && results.length >= options.limit) {
                  return results.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
                }
              } catch {
                // Skip malformed lines
              }
            }
          }
        } catch {
          // Directory may not exist, continue to next
        }
      }
    } catch (error) {
      log.warn("Failed to query history", { error })
    }

    return results.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
  }

  /**
   * Determine which directories to scan based on type filter.
   */
  private getQueryDirectories(type?: ObserverHistoryEntry["type"] | ObserverHistoryEntry["type"][]): string[] {
    if (!type) {
      // No filter - scan all directories
      return ["events", "patterns", "decisions"]
    }

    const types = Array.isArray(type) ? type : [type]
    const dirs = new Set<string>()

    for (const t of types) {
      if (t === "pattern") {
        dirs.add("patterns")
      } else if (t === "decision" || t === "escalation") {
        dirs.add("decisions")
      } else {
        dirs.add("events")
      }
    }

    return Array.from(dirs)
  }

  /**
   * Get the observer storage path.
   */
  getPath(): string {
    return this.observerPath
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────────

  private async writeToDailyNotes(entry: ObserverHistoryEntry): Promise<void> {
    try {
      const entryType = this.mapToEntryType(entry.type)
      const content = this.formatEntryContent(entry)
      const dailyEntry = createEntry(entryType, content)
      await appendDailyNote(dailyEntry)
    } catch (error) {
      log.warn("Failed to write to daily notes", { error })
    }
  }

  private async writeToStorage(entry: ObserverHistoryEntry): Promise<void> {
    try {
      // Determine subdirectory based on type
      const subdir = entry.type === "pattern" ? "patterns"
        : entry.type === "decision" || entry.type === "escalation" ? "decisions"
        : "events"

      // Write as JSONL (append to today's file)
      const today = this.formatDate(new Date())
      const filename = `${today}.jsonl`
      const filePath = path.join(this.observerPath, subdir, filename)

      const line = JSON.stringify(entry) + "\n"

      // Append to file
      const existingContent = await this.readFile(filePath)
      const newContent = existingContent ? existingContent + line : line
      await Bun.write(filePath, newContent)
    } catch (error) {
      log.warn("Failed to write to storage", { error })
    }
  }

  private mapToEntryType(type: ObserverHistoryEntry["type"]): DailyEntryType {
    switch (type) {
      case "observation":
        return "output"
      case "pattern":
      case "opportunity":
        return "action"
      case "anomaly":
        return "error"
      case "decision":
      case "escalation":
        return "decision"
      case "execution":
        return "action"
      default:
        return "output"
    }
  }

  private formatEntryContent(entry: ObserverHistoryEntry): string {
    const data = entry.data as Record<string, unknown>
    const summary = data?.description ?? data?.summary ?? data?.type ?? entry.type
    return `[Observer/${entry.type}] ${summary}`
  }

  private formatDate(date: Date): string {
    const year = date.getFullYear()
    const month = String(date.getMonth() + 1).padStart(2, "0")
    const day = String(date.getDate()).padStart(2, "0")
    return `${year}-${month}-${day}`
  }

  private async readFile(filePath: string): Promise<string | null> {
    try {
      const file = Bun.file(filePath)
      if (!file.exists()) return null
      return await file.text()
    } catch {
      return null
    }
  }

  private async ensureDir(dirPath: string): Promise<void> {
    try {
      await Bun.write(path.join(dirPath, ".keep"), "")
    } catch {
      // Directory may already exist
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Singleton Instance
// ─────────────────────────────────────────────────────────────────────────────

let clientInstance: MemoryClient | null = null

/**
 * Get or create the memory client instance.
 */
export function getMemoryClient(config?: Partial<ObserverMemoryConfig>): MemoryClient {
  if (!clientInstance) {
    clientInstance = new MemoryClient(config)
  }
  return clientInstance
}

/**
 * Reset the memory client instance.
 */
export function resetMemoryClient(): void {
  clientInstance = null
}

/**
 * Create a new memory client instance.
 */
export function createMemoryClient(config?: Partial<ObserverMemoryConfig>): MemoryClient {
  return new MemoryClient(config)
}
