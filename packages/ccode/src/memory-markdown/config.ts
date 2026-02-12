/**
 * Memory Storage Configuration Loader
 *
 * Configuration priority (highest to lowest):
 * 1. Environment variable: CCODE_MEMORY_DIR
 * 2. Environment variable: CCODE_MEMORY_PROJECT_ID
 * 3. Codecoder config file: memory.storage section
 * 4. Default: {process.cwd()}/memory
 */

import path from "path"
import fs from "fs"
import { Log } from "@/util/log"
import type { MemoryStorageConfig } from "./types"

const log = Log.create({ service: "memory-markdown.config" })

/** Environment variable for memory directory */
const ENV_MEMORY_DIR = "CCODE_MEMORY_DIR"

/** Environment variable for project ID */
const ENV_PROJECT_ID = "CCODE_MEMORY_PROJECT_ID"

// Cached configuration
let cachedConfig: MemoryStorageConfig | null = null

/**
 * Load storage configuration from environment and config files
 *
 * Priority: env > config file > default
 */
export function loadStorageConfig(): MemoryStorageConfig {
  if (cachedConfig) {
    return cachedConfig
  }

  const config: MemoryStorageConfig = {}

  // 1. Check environment variables first
  const envPath = process.env[ENV_MEMORY_DIR]
  if (envPath) {
    config.basePath = envPath
    log.debug("using memory path from environment", { path: envPath })
  }

  const envProjectId = process.env[ENV_PROJECT_ID]
  if (envProjectId) {
    config.projectId = envProjectId
    log.debug("using project ID from environment", { projectId: envProjectId })
  }

  // 2. Check codecoder config file (sync fallback)
  const fileConfig = loadFromConfigFileSync()
  if (fileConfig.basePath && !config.basePath) {
    config.basePath = fileConfig.basePath
    log.debug("using memory path from config file", { path: fileConfig.basePath })
  }
  if (fileConfig.projectId && !config.projectId) {
    config.projectId = fileConfig.projectId
    log.debug("using project ID from config file", { projectId: fileConfig.projectId })
  }
  if (fileConfig.provider) {
    config.provider = fileConfig.provider
  }

  cachedConfig = config
  return config
}

/**
 * Load configuration from codecoder config file (synchronous)
 *
 * Reads the memory.storage section from codecoder.json files.
 */
function loadFromConfigFileSync(): MemoryStorageConfig {
  const config: MemoryStorageConfig = {}

  try {
    // Try to read codecoder.json from current directory
    const configPaths = [
      path.join(process.cwd(), "codecoder.json"),
      path.join(process.cwd(), "codecoder.jsonc"),
      path.join(process.cwd(), ".codecoder.json"),
    ]

    for (const configPath of configPaths) {
      try {
        // Use Node.js fs.readFileSync for sync file reading
        if (!fs.existsSync(configPath)) continue

        const text = fs.readFileSync(configPath, "utf-8")
        const parsed = JSON.parse(text) as {
          memory?: { storage?: MemoryStorageConfig }
        }

        if (parsed.memory?.storage) {
          const storage = parsed.memory.storage
          if (storage.basePath) {
            config.basePath = expandPath(storage.basePath)
          }
          if (storage.projectId) {
            config.projectId = storage.projectId
          }
          if (storage.provider) {
            config.provider = storage.provider
          }
          break
        }
      } catch {
        // File not found or invalid, continue to next
        continue
      }
    }
  } catch (error) {
    log.debug("could not load config file sync", { error })
  }

  return config
}

/**
 * Expand path with ~ support
 *
 * Converts ~/path to /home/user/path
 */
function expandPath(inputPath: string): string {
  if (inputPath.startsWith("~/")) {
    const home = process.env.HOME ?? process.env.USERPROFILE ?? ""
    return path.join(home, inputPath.slice(2))
  }
  return inputPath
}

/**
 * Reset configuration cache
 *
 * Forces reload of configuration on next access.
 */
export function resetConfigCache(): void {
  cachedConfig = null
}

/**
 * Preload configuration asynchronously
 *
 * This can be called early to load configuration from the Config module.
 */
export async function preloadConfig(): Promise<void> {
  const config: MemoryStorageConfig = {}

  // 1. Environment variables
  const envPath = process.env[ENV_MEMORY_DIR]
  if (envPath) config.basePath = envPath

  const envProjectId = process.env[ENV_PROJECT_ID]
  if (envProjectId) config.projectId = envProjectId

  // 2. Try loading from Config module (may not be available during initial load)
  try {
    // Use relative import to avoid module resolution issues
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    const { Config } = await import("../config/config")
    const loadedConfig = await Config.get() as Record<string, unknown>

    // Check for memory.storage section
    // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
    const memory = loadedConfig.memory as Record<string, unknown> | undefined
    // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
    const storage = memory?.storage as {
      basePath?: string
      projectId?: string
      provider?: string
    } | undefined

    if (storage) {
      if (storage.basePath && !config.basePath) {
        config.basePath = expandPath(storage.basePath)
      }
      if (storage.projectId && !config.projectId) {
        config.projectId = storage.projectId
      }
      if (storage.provider) {
        config.provider = storage.provider as MemoryStorageConfig["provider"]
      }
    }
  } catch {
    // Config module may not be available yet, or there was a circular dependency
    // Fall back to environment variables and config files only
  }

  cachedConfig = config
}
