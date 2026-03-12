/**
 * Storage Provider Abstraction for Markdown Memory Layer
 *
 * Abstracts storage backend operations to support:
 * - Local filesystem (current default)
 * - NAPI-backed Rust implementation (high performance)
 * - Remote storage (future: HTTP, database)
 * - Multi-project shared memory with project isolation
 */

import path from "path"
import { Log } from "@/util/log"
import type { MemoryStorageConfig } from "./types"
import { loadStorageConfig } from "./config"
import { detectProjectIdSync } from "./project"
import {
  createMarkdownMemory,
  isNative,
} from "@codecoder-ai/core"

// Instance type for MarkdownMemoryHandle (return type of createMarkdownMemory)
type MarkdownMemoryHandleInstance = NonNullable<ReturnType<NonNullable<typeof createMarkdownMemory>>>

const log = Log.create({ service: "memory-markdown.storage" })

/**
 * Storage Provider Interface
 *
 * All storage backends must implement this interface.
 */
export interface MarkdownStorageProvider {
  /** Base storage path */
  readonly basePath: string

  /** Project identifier for multi-project scenarios */
  readonly projectId: string

  /** Daily notes directory path */
  readonly dailyPath: string

  /** Long-term memory file path */
  readonly longTermPath: string

  /** Read a daily note file */
  readDailyNote(date: Date): Promise<string | null>

  /** Write a daily note file */
  writeDailyNote(date: Date, content: string): Promise<void>

  /** List all daily note files */
  listDailyNotes(): Promise<string[]>

  /** Read long-term memory file */
  readLongTermMemory(): Promise<string>

  /** Write long-term memory file */
  writeLongTermMemory(content: string): Promise<void>

  /** Check if a file exists */
  fileExists(filePath: string): Promise<boolean>

  /** Ensure a directory exists */
  ensureDir(dirPath: string): Promise<void>
}

/**
 * Local Filesystem Storage Provider
 *
 * Default implementation using local filesystem via Bun.file API.
 */
export class LocalMarkdownStorage implements MarkdownStorageProvider {
  readonly basePath: string
  readonly projectId: string
  readonly dailyPath: string
  readonly longTermPath: string

  constructor(config: MemoryStorageConfig = {}) {
    this.basePath = this.resolveBasePath(config.basePath)
    this.projectId = config.projectId ?? detectProjectIdSync()
    this.dailyPath = path.join(this.basePath, "daily")
    this.longTermPath = path.join(this.basePath, "MEMORY.md")

    log.debug("initialized local storage", {
      basePath: this.basePath,
      projectId: this.projectId,
    })
  }

  /** Resolve base path with ~ expansion and default fallback */
  private resolveBasePath(configPath?: string): string {
    if (configPath) {
      // Expand ~ to home directory
      if (configPath.startsWith("~/")) {
        const home = process.env.HOME ?? process.env.USERPROFILE ?? ""
        return path.join(home, configPath.slice(2))
      }
      return configPath
    }
    // Default: process.cwd()/memory
    return path.join(process.cwd(), "memory")
  }

  async readDailyNote(date: Date): Promise<string | null> {
    const filePath = this.getDailyPath(date)
    return this.readFile(filePath)
  }

  async writeDailyNote(date: Date, content: string): Promise<void> {
    const filePath = this.getDailyPath(date)
    await this.ensureDir(this.dailyPath)
    await Bun.write(filePath, content)
  }

  async listDailyNotes(): Promise<string[]> {
    try {
      await this.ensureDir(this.dailyPath)

      const globber = new Bun.Glob("**/*.md")
      const files: string[] = []

      for await (const file of globber.scan({ cwd: this.dailyPath })) {
        if (file.endsWith(".md")) {
          files.push(file.replace(".md", ""))
        }
      }

      return files.sort().reverse()
    } catch (error) {
      log.warn("failed to list daily notes", { error })
      return []
    }
  }

  async readLongTermMemory(): Promise<string> {
    try {
      await this.ensureDir(this.basePath)
      return await Bun.file(this.longTermPath).text()
    } catch {
      return ""
    }
  }

  async writeLongTermMemory(content: string): Promise<void> {
    await this.ensureDir(this.basePath)
    await Bun.write(this.longTermPath, content)
  }

  async fileExists(filePath: string): Promise<boolean> {
    try {
      const file = Bun.file(filePath)
      const exists = file.exists()
      if (!exists) return false
      await file.text() // Verify readability
      return true
    } catch {
      return false
    }
  }

  async ensureDir(dirPath: string): Promise<void> {
    try {
      await Bun.write(dirPath + "/.keep", "")
    } catch {
      // Directory may already exist
    }
  }

  /** Read file content */
  private async readFile(filePath: string): Promise<string | null> {
    try {
      const file = Bun.file(filePath)
      const exists = file.exists()
      if (!exists) return null
      return await file.text()
    } catch {
      return null
    }
  }

  /** Get daily note file path for a given date */
  private getDailyPath(date: Date): string {
    const filename = `${this.formatDate(date)}.md`
    return path.join(this.dailyPath, filename)
  }

  /** Format date as YYYY-MM-DD */
  private formatDate(date: Date): string {
    const year = date.getFullYear()
    const month = String(date.getMonth() + 1).padStart(2, "0")
    const day = String(date.getDate()).padStart(2, "0")
    return `${year}-${month}-${day}`
  }
}

/** Global storage instance */
let storageInstance: MarkdownStorageProvider | null = null

/**
 * Get the current storage provider instance
 *
 * Creates a new instance on first call or after resetStorage() is called.
 * Configuration is loaded from environment variables or config files.
 */
export function getStorage(): MarkdownStorageProvider {
  if (!storageInstance) {
    const config = loadStorageConfig()
    storageInstance = new LocalMarkdownStorage(config)
  }
  return storageInstance
}

/**
 * Reset the storage instance
 *
 * Allows reconfiguring storage with new settings.
 * Use this for testing or when configuration changes.
 */
export function resetStorage(): void {
  storageInstance = null
}

/**
 * Configure memory storage programmatically
 *
 * @param config - Storage configuration
 */
export function configureMemory(config: MemoryStorageConfig): void {
  storageInstance = new LocalMarkdownStorage(config)
}

/**
 * Get current storage configuration
 *
 * Returns the resolved configuration being used.
 */
export function getMemoryConfig(): {
  basePath: string
  projectId: string
  dailyPath: string
  longTermPath: string
} {
  const storage = getStorage()
  return {
    basePath: storage.basePath,
    projectId: storage.projectId,
    dailyPath: storage.dailyPath,
    longTermPath: storage.longTermPath,
  }
}

// ============================================================================
// NAPI Memory Handle (High-Performance Rust Backend)
// ============================================================================

/** Global NAPI memory handle instance */
let napiMemoryHandle: MarkdownMemoryHandleInstance | null = null

/**
 * Check if NAPI memory is available
 */
export function isNapiMemoryAvailable(): boolean {
  return Boolean(isNative && createMarkdownMemory)
}

/**
 * Get NAPI markdown memory handle
 *
 * Returns the high-performance Rust-backed memory handle when available.
 * Returns null if NAPI bindings are not available.
 *
 * Usage:
 * ```typescript
 * const handle = getNapiMemoryHandle()
 * if (handle) {
 *   handle.appendDailyNote("Action", "completed task", null)
 * } else {
 *   // Fall back to storage provider
 * }
 * ```
 */
export function getNapiMemoryHandle(): MarkdownMemoryHandleInstance | null {
  if (!isNapiMemoryAvailable()) {
    return null
  }

  if (!napiMemoryHandle) {
    try {
      const config = loadStorageConfig()
      const basePath = config.basePath ?? path.join(process.cwd(), "memory")
      const projectId = config.projectId ?? detectProjectIdSync()

      napiMemoryHandle = createMarkdownMemory!(basePath, projectId)
      log.debug("created NAPI memory handle", { basePath, projectId })
    } catch (error) {
      log.warn("failed to create NAPI memory handle", { error })
      return null
    }
  }

  return napiMemoryHandle
}

/**
 * Reset NAPI memory handle
 *
 * Call this to force recreation with new configuration.
 */
export function resetNapiMemoryHandle(): void {
  napiMemoryHandle = null
}

/**
 * Configure NAPI memory with custom settings
 */
export function configureNapiMemory(basePath: string, projectId: string): MarkdownMemoryHandleInstance | null {
  if (!isNapiMemoryAvailable()) {
    log.warn("NAPI memory not available, using local storage")
    return null
  }

  try {
    napiMemoryHandle = createMarkdownMemory!(basePath, projectId)
    log.debug("configured NAPI memory handle", { basePath, projectId })
    return napiMemoryHandle
  } catch (error) {
    log.error("failed to configure NAPI memory", { error })
    return null
  }
}

