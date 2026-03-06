/**
 * Trace Migration Tool
 *
 * Migrates trace logs from JSONL files to SQLite database.
 * Supports incremental migration and dry-run mode.
 *
 * Usage:
 *   bun run packages/ccode/src/trace/migrate.ts --dry-run
 *   bun run packages/ccode/src/trace/migrate.ts --execute
 *
 * @package trace
 */

import fs from "fs/promises"
import path from "path"
import os from "os"
import { gunzip } from "zlib"
import { promisify } from "util"
import { openTraceStore, toNapiTraceEntry, type TraceStoreHandle } from "./native"
import type { LogEntry } from "../observability"

const gunzipAsync = promisify(gunzip)

// ============================================================================
// Configuration
// ============================================================================

interface MigrateConfig {
  /** Source log directory */
  logDir: string
  /** Target database path */
  dbPath: string
  /** Dry run mode (don't write to database) */
  dryRun: boolean
  /** Batch size for inserts */
  batchSize: number
  /** Keep original files after migration */
  keepOriginals: boolean
}

const defaultConfig: MigrateConfig = {
  logDir: path.join(os.homedir(), ".codecoder", "logs"),
  dbPath: path.join(os.homedir(), ".codecoder", "traces.db"),
  dryRun: true,
  batchSize: 1000,
  keepOriginals: true,
}

// ============================================================================
// Migration Functions
// ============================================================================

/**
 * List all trace log files (both .jsonl and .jsonl.gz)
 */
async function listLogFiles(logDir: string): Promise<string[]> {
  const files: string[] = []

  try {
    const entries = await fs.readdir(logDir)
    for (const entry of entries) {
      if (entry.startsWith("trace-") && (entry.endsWith(".jsonl") || entry.endsWith(".jsonl.gz"))) {
        files.push(path.join(logDir, entry))
      }
    }
  } catch {
    // Directory doesn't exist or not readable
  }

  return files.sort()
}

/**
 * Read and parse a log file (handles both compressed and uncompressed)
 */
async function* parseLogFile(filePath: string): AsyncGenerator<LogEntry> {
  let content: string

  if (filePath.endsWith(".gz")) {
    const compressed = await fs.readFile(filePath)
    const decompressed = await gunzipAsync(compressed)
    content = decompressed.toString("utf-8")
  } else {
    content = await fs.readFile(filePath, "utf-8")
  }

  for (const line of content.split("\n")) {
    if (!line.trim()) continue
    try {
      const entry = JSON.parse(line) as LogEntry
      if (entry.ts && entry.trace_id) {
        yield entry
      }
    } catch {
      // Skip malformed lines
    }
  }
}

/**
 * Extract date from log filename
 */
function extractDateFromFilename(filename: string): string | null {
  const basename = path.basename(filename)
  const match = basename.match(/trace-(\d{4}-\d{2}-\d{2})\.jsonl(\.gz)?/)
  return match ? match[1] : null
}

/**
 * Migrate a single log file to the database
 */
async function migrateFile(
  store: TraceStoreHandle,
  filePath: string,
  config: MigrateConfig,
): Promise<{ migrated: number; errors: number }> {
  let migrated = 0
  let errors = 0
  let batch: ReturnType<typeof toNapiTraceEntry>[] = []

  for await (const entry of parseLogFile(filePath)) {
    try {
      const napiEntry = toNapiTraceEntry({
        ts: entry.ts,
        trace_id: entry.trace_id,
        span_id: entry.span_id,
        parent_span_id: entry.parent_span_id,
        service: entry.service,
        event_type: entry.event_type,
        level: entry.level,
        payload: entry.payload as Record<string, unknown>,
      })
      batch.push(napiEntry)

      if (batch.length >= config.batchSize) {
        if (!config.dryRun) {
          store.appendBatch(batch)
        }
        migrated += batch.length
        batch = []
      }
    } catch {
      errors++
    }
  }

  // Flush remaining batch
  if (batch.length > 0) {
    if (!config.dryRun) {
      store.appendBatch(batch)
    }
    migrated += batch.length
  }

  return { migrated, errors }
}

/**
 * Run the migration
 */
export async function migrate(config: Partial<MigrateConfig> = {}): Promise<MigrateResult> {
  const cfg = { ...defaultConfig, ...config }

  console.log("=".repeat(60))
  console.log("  Trace Log Migration: JSONL → SQLite")
  console.log("=".repeat(60))
  console.log()
  console.log(`Source: ${cfg.logDir}`)
  console.log(`Target: ${cfg.dbPath}`)
  console.log(`Mode:   ${cfg.dryRun ? "DRY RUN (no changes)" : "EXECUTE"}`)
  console.log()

  // List source files
  const files = await listLogFiles(cfg.logDir)
  if (files.length === 0) {
    console.log("No log files found to migrate.")
    return { totalFiles: 0, totalEntries: 0, totalErrors: 0, dryRun: cfg.dryRun }
  }

  console.log(`Found ${files.length} log file(s):`)
  for (const file of files) {
    const date = extractDateFromFilename(file)
    const isCompressed = file.endsWith(".gz")
    console.log(`  - ${path.basename(file)} (${date ?? "unknown"}) ${isCompressed ? "[compressed]" : ""}`)
  }
  console.log()

  // Open database (or simulate for dry run)
  let store: TraceStoreHandle | null = null
  if (!cfg.dryRun) {
    store = await openTraceStore(cfg.dbPath)
    if (!store) {
      console.error("ERROR: Failed to open trace store. Native bindings not available?")
      return { totalFiles: 0, totalEntries: 0, totalErrors: 0, dryRun: cfg.dryRun }
    }
  }

  // Migrate each file
  let totalEntries = 0
  let totalErrors = 0
  const fileResults: Array<{ file: string; migrated: number; errors: number }> = []

  for (const file of files) {
    process.stdout.write(`Migrating ${path.basename(file)}... `)

    if (cfg.dryRun) {
      // Dry run: just count entries
      let count = 0
      let errCount = 0
      for await (const entry of parseLogFile(file)) {
        if (entry.ts && entry.trace_id) {
          count++
        } else {
          errCount++
        }
      }
      console.log(`${count} entries (${errCount} errors)`)
      totalEntries += count
      totalErrors += errCount
      fileResults.push({ file, migrated: count, errors: errCount })
    } else if (store) {
      const { migrated, errors } = await migrateFile(store, file, cfg)
      console.log(`${migrated} entries (${errors} errors)`)
      totalEntries += migrated
      totalErrors += errors
      fileResults.push({ file, migrated, errors })
    }
  }

  console.log()
  console.log("-".repeat(40))
  console.log(`Total: ${totalEntries} entries migrated, ${totalErrors} errors`)

  if (!cfg.dryRun && store) {
    // Get stats
    const stats = store.stats()
    console.log()
    console.log("Database Stats:")
    console.log(`  Total entries: ${stats.totalEntries}`)
    console.log(`  Total size:    ${formatBytes(stats.totalSizeBytes)}`)
    console.log(`  Oldest:        ${stats.oldestTs ?? "N/A"}`)
    console.log(`  Newest:        ${stats.newestTs ?? "N/A"}`)

    // Compact
    console.log()
    console.log("Compacting database...")
    store.compact()
    console.log("Done.")
  }

  console.log()
  if (cfg.dryRun) {
    console.log("This was a DRY RUN. No changes were made.")
    console.log("Run with --execute to perform the actual migration.")
  } else {
    console.log("Migration complete!")
    if (cfg.keepOriginals) {
      console.log("Original files were preserved.")
    }
  }

  return {
    totalFiles: files.length,
    totalEntries,
    totalErrors,
    dryRun: cfg.dryRun,
    fileResults,
  }
}

interface MigrateResult {
  totalFiles: number
  totalEntries: number
  totalErrors: number
  dryRun: boolean
  fileResults?: Array<{ file: string; migrated: number; errors: number }>
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

// ============================================================================
// CLI Entry Point
// ============================================================================

if (require.main === module || process.argv[1]?.endsWith("migrate.ts")) {
  const args = process.argv.slice(2)
  const dryRun = !args.includes("--execute")

  migrate({ dryRun })
    .then((result) => {
      process.exit(result.totalErrors > 0 ? 1 : 0)
    })
    .catch((err) => {
      console.error("Migration failed:", err)
      process.exit(1)
    })
}

export type { MigrateConfig, MigrateResult }
