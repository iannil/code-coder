/**
 * Configuration Migration Utility
 *
 * Helps users consolidate multiple config files into a single unified config.json.
 *
 * Currently supported migrations:
 * - secrets.json → config.json (secrets field)
 * - providers.json → config.json (provider field)
 * - channels.json → config.json (zerobot.channels field)
 * - trading.json → config.json (trading field)
 *
 * Usage:
 *   bun dev config migrate          # Execute migration
 *   bun dev config migrate --dry-run # Preview changes
 */

import { Global } from "../global"
import path from "path"
import { mergeDeep } from "remeda"
import { Log } from "@/util/log"

const log = Log.create({ service: "config-migrate" })

interface MigrationResult {
  success: boolean
  filesProcessed: string[]
  filesSkipped: string[]
  errors: string[]
  preview?: Record<string, unknown>
}

/**
 * Read a JSON file safely
 */
async function readJsonFile<T>(filepath: string): Promise<T | null> {
  try {
    const file = Bun.file(filepath)
    if (!(await file.exists())) return null
    const text = await file.text()
    return JSON.parse(text) as T
  } catch (error) {
    log.warn("failed to read file", { path: filepath, error })
    return null
  }
}

/**
 * Backup a file before migration
 */
async function backupFile(filepath: string): Promise<string | null> {
  try {
    const file = Bun.file(filepath)
    if (!(await file.exists())) return null

    const timestamp = new Date().toISOString().replace(/[:.]/g, "-")
    const ext = path.extname(filepath)
    const base = path.basename(filepath, ext)
    const dir = path.dirname(filepath)
    const backupPath = path.join(dir, `${base}.${timestamp}.backup${ext}`)

    await Bun.write(backupPath, await file.text())
    return backupPath
  } catch (error) {
    log.error("failed to backup file", { path: filepath, error })
    return null
  }
}

/**
 * Migration configuration
 */
const MIGRATION_MAP: Record<string, { target: string[]; description: string }> = {
  "secrets.json": {
    target: ["secrets"],
    description: "API keys and credentials",
  },
  "providers.json": {
    target: ["provider"],
    description: "LLM provider configurations",
  },
  "channels.json": {
    target: ["zerobot", "channels"],
    description: "IM channel configurations (Telegram, Discord, etc.)",
  },
  "trading.json": {
    target: ["trading"],
    description: "Trading module configuration",
  },
}

/**
 * Set a nested value in an object
 */
function setNestedValue(obj: Record<string, unknown>, path: string[], value: unknown): void {
  let current = obj
  for (let i = 0; i < path.length - 1; i++) {
    const key = path[i]
    if (!(key in current) || typeof current[key] !== "object") {
      current[key] = {}
    }
    current = current[key] as Record<string, unknown>
  }
  const lastKey = path[path.length - 1]
  current[lastKey] = value
}

/**
 * Migrate configuration files
 */
export async function migrateConfig(options: { dryRun?: boolean } = {}): Promise<MigrationResult> {
  const configDir = Global.Path.config
  const result: MigrationResult = {
    success: true,
    filesProcessed: [],
    filesSkipped: [],
    errors: [],
  }

  // Read existing config.json
  const configPath = path.join(configDir, "config.json")
  let config = (await readJsonFile<Record<string, unknown>>(configPath)) ?? {}

  // Track merged content for preview
  const preview: Record<string, unknown> = { ...config }

  // Process each migratable file
  for (const [filename, { target, description }] of Object.entries(MIGRATION_MAP)) {
    const filepath = path.join(configDir, filename)
    const content = await readJsonFile<Record<string, unknown>>(filepath)

    if (!content) {
      result.filesSkipped.push(filename)
      continue
    }

    // Check if already migrated (content exists in config.json at target path)
    let existingValue: unknown = config
    for (const key of target) {
      if (typeof existingValue === "object" && existingValue !== null) {
        existingValue = (existingValue as Record<string, unknown>)[key]
      } else {
        existingValue = undefined
        break
      }
    }

    if (existingValue && Object.keys(existingValue as Record<string, unknown>).length > 0) {
      log.info("merging into existing config", { file: filename, target: target.join(".") })
      // Merge with existing (existing takes precedence for safety)
      setNestedValue(preview, target, mergeDeep(content, existingValue as Record<string, unknown>))
    } else {
      setNestedValue(preview, target, content)
    }

    result.filesProcessed.push(filename)
    log.info("will migrate", { file: filename, target: target.join("."), description })
  }

  // Set preview for dry-run
  result.preview = preview

  if (options.dryRun) {
    log.info("dry-run complete", { processed: result.filesProcessed, skipped: result.filesSkipped })
    return result
  }

  // Execute migration
  if (result.filesProcessed.length === 0) {
    log.info("no files to migrate")
    return result
  }

  try {
    // Backup existing config.json
    const backupPath = await backupFile(configPath)
    if (backupPath) {
      log.info("backed up config.json", { backup: backupPath })
    }

    // Write merged config
    await Bun.write(configPath, JSON.stringify(preview, null, 2))
    log.info("wrote merged config.json")

    // Backup and remove migrated files
    for (const filename of result.filesProcessed) {
      const filepath = path.join(configDir, filename)
      const backupPath = await backupFile(filepath)
      if (backupPath) {
        // Don't delete, just rename to .migrated
        const migratedPath = filepath.replace(/\.json$/, ".migrated.json")
        await Bun.write(migratedPath, await Bun.file(filepath).text())
        // We keep the original for now - user can delete manually
        log.info("migrated file backed up", { original: filename, backup: backupPath })
      }
    }
  } catch (error) {
    result.success = false
    result.errors.push(`Failed to write config: ${error}`)
    log.error("migration failed", { error })
  }

  return result
}

/**
 * Validate current configuration
 */
export async function validateConfig(): Promise<{ valid: boolean; issues: string[] }> {
  const configDir = Global.Path.config
  const issues: string[] = []

  // Check if config.json exists
  const configPath = path.join(configDir, "config.json")
  const config = await readJsonFile<Record<string, unknown>>(configPath)

  if (!config) {
    issues.push("config.json does not exist or is invalid")
  }

  // Check for deprecated separate files that should be migrated
  for (const filename of Object.keys(MIGRATION_MAP)) {
    const filepath = path.join(configDir, filename)
    const content = await readJsonFile<Record<string, unknown>>(filepath)
    if (content && Object.keys(content).length > 0) {
      issues.push(`${filename} exists and should be migrated into config.json`)
    }
  }

  return {
    valid: issues.length === 0,
    issues,
  }
}

/**
 * Show current configuration
 */
export async function showConfig(): Promise<Record<string, unknown>> {
  const configDir = Global.Path.config
  const configPath = path.join(configDir, "config.json")
  return (await readJsonFile<Record<string, unknown>>(configPath)) ?? {}
}
