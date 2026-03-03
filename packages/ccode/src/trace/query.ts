/**
 * Trace Query Engine
 * Provides functions for querying and filtering trace logs
 */

import fs from "fs/promises"
import path from "path"
import readline from "readline"
import { gunzip } from "zlib"
import { promisify } from "util"
import type { LogEntry } from "../observability"

const gunzipAsync = promisify(gunzip)

// ============================================================================
// Types
// ============================================================================

export interface WatchOptions {
  service?: string
  level?: string
  follow?: boolean
}

export interface ErrorGroup {
  key: string
  count: number
  samples: Array<{
    error: string
    timestamp: string
    traceId: string
  }>
}

export interface ErrorSummary {
  total: number
  groups: ErrorGroup[]
}

// ============================================================================
// File Utilities
// ============================================================================

/**
 * Get all trace log files sorted by date (newest first)
 * Includes both .jsonl and .jsonl.gz files
 */
async function getLogFiles(logDir: string): Promise<string[]> {
  const files = await fs.readdir(logDir).catch(() => [])
  return files
    .filter((f) => f.startsWith("trace-") && (f.endsWith(".jsonl") || f.endsWith(".jsonl.gz")))
    .sort()
    .reverse()
    .map((f) => path.join(logDir, f))
}

/**
 * Get Rust service log files (zero-*.log)
 * These contain structured JSON trace entries mixed with unstructured tracing output
 */
async function getServiceLogFiles(logDir: string): Promise<string[]> {
  const files = await fs.readdir(logDir).catch(() => [])
  return files
    .filter((f) => f.startsWith("zero-") && f.endsWith(".log"))
    .map((f) => path.join(logDir, f))
}

/**
 * Normalize a Rust log entry to match TypeScript LogEntry format.
 * Handles the timestamp -> ts field mapping.
 */
function normalizeLogEntry(raw: Record<string, unknown>): LogEntry | null {
  // Handle Rust format (uses "timestamp" instead of "ts")
  const ts = (raw.ts as string) || (raw.timestamp as string)
  if (!ts) return null

  const trace_id = raw.trace_id as string
  if (!trace_id) return null

  return {
    ts,
    trace_id,
    span_id: (raw.span_id as string) || "",
    parent_span_id: raw.parent_span_id as string | undefined,
    service: (raw.service as string) || "unknown",
    event_type: (raw.event_type as string) || "unknown",
    level: (raw.level as string) || "info",
    payload: (raw.payload as Record<string, unknown>) || {},
  } as LogEntry
}

/**
 * Get the current day's log file
 */
function getCurrentLogFile(logDir: string): string {
  const date = new Date().toISOString().split("T")[0]
  return path.join(logDir, `trace-${date}.jsonl`)
}

/**
 * Parse a JSONL file and yield entries
 * Handles both compressed (.jsonl.gz) and uncompressed (.jsonl) files
 */
async function* parseLogFile(filePath: string): AsyncGenerator<LogEntry> {
  let content: string

  if (filePath.endsWith(".gz")) {
    // Handle compressed file
    const compressed = await fs.readFile(filePath).catch(() => Buffer.from(""))
    if (compressed.length === 0) return
    try {
      const decompressed = await gunzipAsync(compressed)
      content = decompressed.toString("utf-8")
    } catch {
      return // Skip corrupted files
    }
  } else {
    // Handle uncompressed file
    content = await fs.readFile(filePath, "utf-8").catch(() => "")
  }

  for (const line of content.split("\n")) {
    if (!line.trim()) continue
    try {
      yield JSON.parse(line) as LogEntry
    } catch {
      // Skip malformed lines
    }
  }
}

/**
 * Parse a service log file (zero-*.log) and yield entries.
 * Service logs contain mixed content: structured JSON and unstructured tracing output.
 * Only JSON lines starting with '{' are parsed; others are skipped.
 */
async function* parseServiceLogFile(filePath: string): AsyncGenerator<LogEntry> {
  const content = await fs.readFile(filePath, "utf-8").catch(() => "")

  for (const line of content.split("\n")) {
    const trimmed = line.trim()
    // Only parse lines that look like JSON (start with {)
    if (!trimmed.startsWith("{")) continue
    try {
      const raw = JSON.parse(trimmed) as Record<string, unknown>
      const entry = normalizeLogEntry(raw)
      if (entry) yield entry
    } catch {
      // Skip malformed lines
    }
  }
}

/**
 * Parse log entries from a file within a time range
 */
async function* parseLogFileInRange(
  filePath: string,
  fromDate: Date,
): AsyncGenerator<LogEntry> {
  for await (const entry of parseLogFile(filePath)) {
    const entryDate = new Date(entry.ts)
    if (entryDate >= fromDate) {
      yield entry
    }
  }
}

// ============================================================================
// Query Functions
// ============================================================================

/**
 * Query all entries for a specific trace ID
 * Searches both trace-*.jsonl files and zero-*.log service files
 */
export async function queryTrace(traceId: string, logDir: string): Promise<LogEntry[]> {
  const results: LogEntry[] = []

  // Search trace JSONL files first
  const traceFiles = await getLogFiles(logDir)
  for (const file of traceFiles) {
    for await (const entry of parseLogFile(file)) {
      if (entry.trace_id === traceId) {
        results.push(entry)
      }
    }
    // Stop searching older files if we found entries
    if (results.length > 0 && traceFiles.indexOf(file) > 0) {
      break
    }
  }

  // Also search Rust service logs (zero-*.log) for cross-service trace correlation
  const serviceFiles = await getServiceLogFiles(logDir)
  for (const file of serviceFiles) {
    for await (const entry of parseServiceLogFile(file)) {
      if (entry.trace_id === traceId) {
        // Avoid duplicates
        const isDuplicate = results.some(
          (r) => r.ts === entry.ts && r.span_id === entry.span_id && r.event_type === entry.event_type,
        )
        if (!isDuplicate) {
          results.push(entry)
        }
      }
    }
  }

  // Sort by timestamp
  return results.sort((a, b) => new Date(a.ts).getTime() - new Date(b.ts).getTime())
}

/**
 * Watch log files in real-time
 */
export async function watchLogs(logDir: string, options: WatchOptions): Promise<void> {
  const levelPriority: Record<string, number> = {
    debug: 0,
    info: 1,
    warn: 2,
    error: 3,
  }

  const minLevel = levelPriority[options.level ?? "info"] ?? 1

  const shouldShow = (entry: LogEntry): boolean => {
    if (options.service && entry.service !== options.service) return false
    if (levelPriority[entry.level] < minLevel) return false
    return true
  }

  const formatEntry = (entry: LogEntry): string => {
    const time = new Date(entry.ts).toLocaleTimeString()
    const level = entry.level.toUpperCase().padEnd(5)
    const service = entry.service.padEnd(15)
    const event = entry.event_type.padEnd(15)
    const func = entry.payload?.function ?? ""
    const duration = entry.payload?.duration_ms ? `${entry.payload.duration_ms}ms` : ""

    return `${time} ${level} ${service} ${event} ${func} ${duration}`
  }

  const logFile = getCurrentLogFile(logDir)

  // Print existing entries first
  for await (const entry of parseLogFile(logFile)) {
    if (shouldShow(entry)) {
      console.log(formatEntry(entry))
    }
  }

  if (!options.follow) return

  // Watch for new entries using file system watcher
  const { watch } = await import("fs")

  let lastSize = 0
  const file = Bun.file(logFile)
  lastSize = (await file.exists()) ? file.size : 0

  // Poll for changes (more reliable than fs.watch for log files)
  const interval = setInterval(async () => {
    const currentFile = Bun.file(logFile)
    if (!(await currentFile.exists())) return

    const currentSize = currentFile.size
    if (currentSize <= lastSize) return

    const content = await currentFile.text()
    const lines = content.split("\n")

    // Process new lines
    for (const line of lines.slice(-100)) {
      if (!line.trim()) continue
      try {
        const entry = JSON.parse(line) as LogEntry
        if (shouldShow(entry)) {
          console.log(formatEntry(entry))
        }
      } catch {
        // Skip malformed
      }
    }

    lastSize = currentSize
  }, 500)

  // Handle exit
  process.on("SIGINT", () => {
    clearInterval(interval)
    process.exit(0)
  })

  // Keep running
  await new Promise(() => {})
}

/**
 * Aggregate errors from trace logs
 */
export async function aggregateErrors(
  logDir: string,
  fromDate: Date,
  groupBy: "service" | "function" | "error" = "service",
): Promise<ErrorSummary> {
  const files = await getLogFiles(logDir)
  const groups = new Map<string, ErrorGroup>()
  let total = 0

  for (const file of files) {
    // Check if file date is before our range
    const fileMatch = path.basename(file).match(/trace-(\d{4}-\d{2}-\d{2})\.jsonl(\.gz)?/)
    if (fileMatch) {
      const fileDate = new Date(fileMatch[1])
      if (fileDate < new Date(fromDate.toISOString().split("T")[0])) {
        continue
      }
    }

    for await (const entry of parseLogFileInRange(file, fromDate)) {
      if (entry.event_type !== "error") continue

      total++

      let key: string
      switch (groupBy) {
        case "service":
          key = entry.service
          break
        case "function":
          key = entry.payload?.function as string ?? "unknown"
          break
        case "error":
          key = entry.payload?.error as string ?? "unknown"
          break
      }

      const existing = groups.get(key)
      if (existing) {
        existing.count++
        if (existing.samples.length < 5) {
          existing.samples.push({
            error: entry.payload?.error as string ?? "unknown",
            timestamp: entry.ts,
            traceId: entry.trace_id,
          })
        }
      } else {
        groups.set(key, {
          key,
          count: 1,
          samples: [
            {
              error: entry.payload?.error as string ?? "unknown",
              timestamp: entry.ts,
              traceId: entry.trace_id,
            },
          ],
        })
      }
    }
  }

  // Sort by count descending
  const sortedGroups = Array.from(groups.values()).sort((a, b) => b.count - a.count)

  return {
    total,
    groups: sortedGroups,
  }
}
