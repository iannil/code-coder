/**
 * Log storage management with compression support.
 *
 * Handles:
 * - Compressing old log files (gzip)
 * - Reading compressed and uncompressed files
 * - Log retention and cleanup
 */

import path from "path"
import fs from "fs/promises"
import os from "os"
import { gzip, gunzip } from "zlib"
import { promisify } from "util"

const gzipAsync = promisify(gzip)
const gunzipAsync = promisify(gunzip)

// ============================================================================
// Configuration
// ============================================================================

export interface StorageConfig {
  /** Log directory path */
  logDir: string
  /** Number of days to keep uncompressed logs */
  compressAfterDays: number
  /** Number of days to retain logs (including compressed) */
  retentionDays: number
}

const defaultConfig: StorageConfig = {
  logDir: path.join(os.homedir(), ".codecoder", "logs"),
  compressAfterDays: 1,
  retentionDays: 7,
}

// ============================================================================
// File Operations
// ============================================================================

/**
 * Get list of all log files (both .jsonl and .jsonl.gz)
 */
export async function listLogFiles(config: Partial<StorageConfig> = {}): Promise<string[]> {
  const { logDir } = { ...defaultConfig, ...config }

  const files: string[] = []

  // Find uncompressed files
  const jsonlGlob = new Bun.Glob("trace-*.jsonl")
  for await (const file of jsonlGlob.scan({ cwd: logDir, absolute: true })) {
    files.push(file)
  }

  // Find compressed files
  const gzGlob = new Bun.Glob("trace-*.jsonl.gz")
  for await (const file of gzGlob.scan({ cwd: logDir, absolute: true })) {
    files.push(file)
  }

  return files.sort()
}

/**
 * Extract date from log filename
 */
export function extractDateFromFilename(filename: string): string | null {
  const basename = path.basename(filename)
  const match = basename.match(/trace-(\d{4}-\d{2}-\d{2})\.jsonl(\.gz)?/)
  return match ? match[1] : null
}

/**
 * Read log file content (handles both compressed and uncompressed)
 */
export async function readLogFile(filepath: string): Promise<string> {
  const isCompressed = filepath.endsWith(".gz")

  if (isCompressed) {
    const compressed = await fs.readFile(filepath)
    const decompressed = await gunzipAsync(compressed)
    return decompressed.toString("utf-8")
  }

  return await fs.readFile(filepath, "utf-8")
}

/**
 * Read log entries from a file
 */
export async function readLogEntries(filepath: string): Promise<unknown[]> {
  const content = await readLogFile(filepath)
  const lines = content.trim().split("\n").filter(Boolean)

  return lines.map((line) => {
    try {
      return JSON.parse(line)
    } catch {
      return null
    }
  }).filter(Boolean)
}

/**
 * Compress a log file (creates .gz version and removes original)
 */
export async function compressLogFile(filepath: string): Promise<string> {
  if (filepath.endsWith(".gz")) {
    return filepath // Already compressed
  }

  const content = await fs.readFile(filepath)
  const compressed = await gzipAsync(content)
  const gzPath = filepath + ".gz"

  await fs.writeFile(gzPath, compressed)
  await fs.unlink(filepath)

  return gzPath
}

/**
 * Decompress a log file (for debugging/inspection)
 */
export async function decompressLogFile(filepath: string): Promise<string> {
  if (!filepath.endsWith(".gz")) {
    return filepath // Not compressed
  }

  const compressed = await fs.readFile(filepath)
  const decompressed = await gunzipAsync(compressed)
  const jsonlPath = filepath.slice(0, -3) // Remove .gz

  await fs.writeFile(jsonlPath, decompressed)

  return jsonlPath
}

// ============================================================================
// Maintenance Operations
// ============================================================================

/**
 * Compress old log files based on configuration
 */
export async function compressOldLogs(config: Partial<StorageConfig> = {}): Promise<{
  compressed: string[]
  errors: Array<{ file: string; error: string }>
}> {
  const cfg = { ...defaultConfig, ...config }
  const today = new Date().toISOString().split("T")[0]

  const cutoffDate = new Date()
  cutoffDate.setDate(cutoffDate.getDate() - cfg.compressAfterDays)
  const cutoffStr = cutoffDate.toISOString().split("T")[0]

  const files = await listLogFiles(cfg)
  const compressed: string[] = []
  const errors: Array<{ file: string; error: string }> = []

  for (const file of files) {
    // Skip already compressed files
    if (file.endsWith(".gz")) continue

    const fileDate = extractDateFromFilename(file)
    if (!fileDate) continue

    // Skip today's log (still being written)
    if (fileDate === today) continue

    // Compress if older than cutoff
    if (fileDate < cutoffStr) {
      try {
        const gzPath = await compressLogFile(file)
        compressed.push(gzPath)
      } catch (err) {
        errors.push({
          file,
          error: err instanceof Error ? err.message : String(err),
        })
      }
    }
  }

  return { compressed, errors }
}

/**
 * Delete old log files beyond retention period
 */
export async function cleanupOldLogs(config: Partial<StorageConfig> = {}): Promise<{
  deleted: string[]
  errors: Array<{ file: string; error: string }>
}> {
  const cfg = { ...defaultConfig, ...config }

  const cutoffDate = new Date()
  cutoffDate.setDate(cutoffDate.getDate() - cfg.retentionDays)
  const cutoffStr = cutoffDate.toISOString().split("T")[0]

  const files = await listLogFiles(cfg)
  const deleted: string[] = []
  const errors: Array<{ file: string; error: string }> = []

  for (const file of files) {
    const fileDate = extractDateFromFilename(file)
    if (!fileDate) continue

    if (fileDate < cutoffStr) {
      try {
        await fs.unlink(file)
        deleted.push(file)
      } catch (err) {
        errors.push({
          file,
          error: err instanceof Error ? err.message : String(err),
        })
      }
    }
  }

  return { deleted, errors }
}

/**
 * Run full maintenance: compress old logs and cleanup expired ones
 */
export async function runMaintenance(config: Partial<StorageConfig> = {}): Promise<{
  compressed: string[]
  deleted: string[]
  errors: Array<{ file: string; error: string }>
}> {
  const compressResult = await compressOldLogs(config)
  const cleanupResult = await cleanupOldLogs(config)

  return {
    compressed: compressResult.compressed,
    deleted: cleanupResult.deleted,
    errors: [...compressResult.errors, ...cleanupResult.errors],
  }
}

// ============================================================================
// Statistics
// ============================================================================

export interface StorageStats {
  totalFiles: number
  uncompressedFiles: number
  compressedFiles: number
  totalSizeBytes: number
  compressedSizeBytes: number
  uncompressedSizeBytes: number
  oldestDate: string | null
  newestDate: string | null
}

/**
 * Get storage statistics
 */
export async function getStorageStats(config: Partial<StorageConfig> = {}): Promise<StorageStats> {
  const cfg = { ...defaultConfig, ...config }
  const files = await listLogFiles(cfg)

  let totalSizeBytes = 0
  let compressedSizeBytes = 0
  let uncompressedSizeBytes = 0
  let compressedFiles = 0
  let uncompressedFiles = 0
  const dates: string[] = []

  for (const file of files) {
    try {
      const stat = await fs.stat(file)
      totalSizeBytes += stat.size

      if (file.endsWith(".gz")) {
        compressedFiles++
        compressedSizeBytes += stat.size
      } else {
        uncompressedFiles++
        uncompressedSizeBytes += stat.size
      }

      const date = extractDateFromFilename(file)
      if (date) dates.push(date)
    } catch {
      // Skip files that can't be stat'd
    }
  }

  dates.sort()

  return {
    totalFiles: files.length,
    uncompressedFiles,
    compressedFiles,
    totalSizeBytes,
    compressedSizeBytes,
    uncompressedSizeBytes,
    oldestDate: dates[0] ?? null,
    newestDate: dates[dates.length - 1] ?? null,
  }
}

/**
 * Format bytes to human readable string
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}
