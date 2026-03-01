/**
 * Trace Analysis CLI Command
 * Provides commands for analyzing trace logs
 */

import type { Argv, CommandModule } from "yargs"
import { cmd } from "./cmd"
import os from "os"
import path from "path"

// ============================================================================
// Types
// ============================================================================

interface TraceShowOptions {
  traceId: string
  format?: "json" | "tree" | "text"
}

interface TraceWatchOptions {
  service?: string
  level?: string
  follow?: boolean
}

interface TraceProfileOptions {
  from?: string
  top?: number
}

interface TraceErrorsOptions {
  from?: string
  groupBy?: "service" | "function" | "error"
}

interface TraceTreeOptions {
  traceId: string
}

interface TraceCompressOptions {
  days?: number
}

interface TraceCleanupOptions {
  retentionDays?: number
}

interface TraceStatsOptions {
  // No options currently
}

interface TraceMaintainOptions {
  compressAfterDays?: number
  retentionDays?: number
}

interface TraceStreamOptions {
  service?: string
  level?: string
  port?: number
}

// ============================================================================
// Helper Functions
// ============================================================================

function getLogDir(): string {
  return path.join(os.homedir(), ".codecoder", "logs")
}

function parseRelativeTime(input: string): Date {
  const now = new Date()
  const match = input.match(/^(\d+)\s*(minute|hour|day|min|hr|d|m|h)s?\s*ago$/i)

  if (!match) {
    // Try ISO date
    const parsed = new Date(input)
    if (!isNaN(parsed.getTime())) return parsed
    throw new Error(`Invalid time format: ${input}`)
  }

  const [, amount, unit] = match
  const n = parseInt(amount, 10)

  switch (unit.toLowerCase()) {
    case "minute":
    case "minutes":
    case "min":
    case "m":
      return new Date(now.getTime() - n * 60 * 1000)
    case "hour":
    case "hours":
    case "hr":
    case "h":
      return new Date(now.getTime() - n * 60 * 60 * 1000)
    case "day":
    case "days":
    case "d":
      return new Date(now.getTime() - n * 24 * 60 * 60 * 1000)
    default:
      throw new Error(`Unknown time unit: ${unit}`)
  }
}

// ============================================================================
// Subcommands
// ============================================================================

const showCommand: CommandModule<object, TraceShowOptions> = {
  command: "show <traceId>",
  describe: "Show complete trace chain for a trace ID",
  builder: (yargs) =>
    yargs
      .positional("traceId", {
        type: "string",
        describe: "Trace ID to look up",
        demandOption: true,
      })
      .option("format", {
        type: "string",
        choices: ["json", "tree", "text"] as const,
        default: "text" as const,
        describe: "Output format",
      }),
  handler: async (args) => {
    const { queryTrace } = await import("../../trace/query")
    const { formatAsTree, formatAsText } = await import("../../trace/visualizer")

    const entries = await queryTrace(args.traceId, getLogDir())

    if (entries.length === 0) {
      console.log(`No entries found for trace ID: ${args.traceId}`)
      return
    }

    switch (args.format) {
      case "json":
        console.log(JSON.stringify(entries, null, 2))
        break
      case "tree":
        console.log(formatAsTree(entries))
        break
      case "text":
      default:
        console.log(formatAsText(entries))
    }
  },
}

const watchCommand: CommandModule<object, TraceWatchOptions> = {
  command: "watch",
  describe: "Watch trace logs in real-time",
  builder: (yargs) =>
    yargs
      .option("service", {
        type: "string",
        alias: "s",
        describe: "Filter by service name",
      })
      .option("level", {
        type: "string",
        alias: "l",
        choices: ["debug", "info", "warn", "error"],
        default: "info",
        describe: "Minimum log level",
      })
      .option("follow", {
        type: "boolean",
        alias: "f",
        default: true,
        describe: "Follow log output",
      }),
  handler: async (args) => {
    const { watchLogs } = await import("../../trace/query")

    console.log(`Watching trace logs (level: ${args.level}, service: ${args.service || "all"})...`)
    console.log("Press Ctrl+C to stop\n")

    await watchLogs(getLogDir(), {
      service: args.service,
      level: args.level,
      follow: args.follow,
    })
  },
}

const profileCommand: CommandModule<object, TraceProfileOptions> = {
  command: "profile",
  describe: "Generate performance profile from traces",
  builder: (yargs) =>
    yargs
      .option("from", {
        type: "string",
        default: "10 minutes ago",
        describe: 'Start time (e.g., "10 minutes ago", "1 hour ago")',
      })
      .option("top", {
        type: "number",
        alias: "n",
        default: 10,
        describe: "Number of slowest operations to show",
      }),
  handler: async (args) => {
    const { profileTraces } = await import("../../trace/profiler")

    const fromDate = parseRelativeTime(args.from || "10 minutes ago")
    const profile = await profileTraces(getLogDir(), fromDate, args.top || 10)

    console.log("\n=== Performance Profile ===\n")
    console.log(`Time range: ${fromDate.toISOString()} - now`)
    console.log(`Total traces analyzed: ${profile.totalTraces}`)
    console.log(`Total events: ${profile.totalEvents}`)
    console.log(`Avg duration: ${profile.avgDurationMs.toFixed(2)}ms\n`)

    console.log("Top slowest operations:")
    console.log("-".repeat(80))

    profile.slowest.forEach((op, i) => {
      console.log(`${i + 1}. ${op.function} (${op.service})`)
      console.log(`   Duration: ${op.durationMs}ms | Trace: ${op.traceId}`)
    })
  },
}

const errorsCommand: CommandModule<object, TraceErrorsOptions> = {
  command: "errors",
  describe: "Aggregate and display errors from traces",
  builder: (yargs) =>
    yargs
      .option("from", {
        type: "string",
        default: "1 hour ago",
        describe: 'Start time (e.g., "1 hour ago")',
      })
      .option("group-by", {
        type: "string",
        choices: ["service", "function", "error"] as const,
        default: "service" as const,
        describe: "Group errors by field",
      }),
  handler: async (args) => {
    const { aggregateErrors } = await import("../../trace/query")

    const fromDate = parseRelativeTime(args.from || "1 hour ago")
    const errors = await aggregateErrors(getLogDir(), fromDate, args.groupBy || "service")

    console.log("\n=== Error Summary ===\n")
    console.log(`Time range: ${fromDate.toISOString()} - now`)
    console.log(`Total errors: ${errors.total}\n`)

    if (errors.total === 0) {
      console.log("No errors found in the specified time range.")
      return
    }

    console.log(`Grouped by: ${args.groupBy}`)
    console.log("-".repeat(60))

    errors.groups.forEach((group) => {
      console.log(`\n${group.key}: ${group.count} errors`)
      group.samples.slice(0, 3).forEach((sample) => {
        console.log(`  - ${sample.error} (${sample.timestamp})`)
      })
    })
  },
}

const treeCommand: CommandModule<object, TraceTreeOptions> = {
  command: "tree <traceId>",
  describe: "Visualize trace as ASCII tree",
  builder: (yargs) =>
    yargs.positional("traceId", {
      type: "string",
      describe: "Trace ID to visualize",
      demandOption: true,
    }),
  handler: async (args) => {
    const { queryTrace } = await import("../../trace/query")
    const { formatAsTree } = await import("../../trace/visualizer")

    const entries = await queryTrace(args.traceId, getLogDir())

    if (entries.length === 0) {
      console.log(`No entries found for trace ID: ${args.traceId}`)
      return
    }

    console.log(`\nTrace: ${args.traceId}`)
    console.log("=".repeat(60))
    console.log(formatAsTree(entries))
  },
}

const compressCommand: CommandModule<object, TraceCompressOptions> = {
  command: "compress",
  describe: "Compress old log files to save space",
  builder: (yargs) =>
    yargs.option("days", {
      type: "number",
      alias: "d",
      default: 1,
      describe: "Compress logs older than this many days",
    }),
  handler: async (args) => {
    const { compressOldLogs } = await import("../../trace/storage")

    console.log(`Compressing logs older than ${args.days} day(s)...`)

    const result = await compressOldLogs({
      logDir: getLogDir(),
      compressAfterDays: args.days ?? 1,
      retentionDays: 7,
    })

    if (result.compressed.length === 0) {
      console.log("No files to compress.")
    } else {
      console.log(`\nCompressed ${result.compressed.length} file(s):`)
      result.compressed.forEach((file) => console.log(`  ✓ ${path.basename(file)}`))
    }

    if (result.errors.length > 0) {
      console.log(`\nErrors (${result.errors.length}):`)
      result.errors.forEach((e) => console.log(`  ✗ ${e.file}: ${e.error}`))
    }
  },
}

const cleanupCommand: CommandModule<object, TraceCleanupOptions> = {
  command: "cleanup",
  describe: "Delete expired log files beyond retention period",
  builder: (yargs) =>
    yargs.option("retention-days", {
      type: "number",
      alias: "r",
      default: 7,
      describe: "Delete logs older than this many days",
    }),
  handler: async (args) => {
    const { cleanupOldLogs } = await import("../../trace/storage")

    console.log(`Cleaning up logs older than ${args.retentionDays} days...`)

    const result = await cleanupOldLogs({
      logDir: getLogDir(),
      compressAfterDays: 1,
      retentionDays: args.retentionDays ?? 7,
    })

    if (result.deleted.length === 0) {
      console.log("No expired files to delete.")
    } else {
      console.log(`\nDeleted ${result.deleted.length} file(s):`)
      result.deleted.forEach((file) => console.log(`  ✓ ${path.basename(file)}`))
    }

    if (result.errors.length > 0) {
      console.log(`\nErrors (${result.errors.length}):`)
      result.errors.forEach((e) => console.log(`  ✗ ${e.file}: ${e.error}`))
    }
  },
}

const statsCommand: CommandModule<object, TraceStatsOptions> = {
  command: "stats",
  describe: "Show trace log storage statistics",
  builder: (yargs) => yargs,
  handler: async () => {
    const { getStorageStats, formatBytes } = await import("../../trace/storage")

    const stats = await getStorageStats({ logDir: getLogDir() })

    console.log("\n=== Trace Log Storage ===\n")
    console.log(`Total files:        ${stats.totalFiles}`)
    console.log(`  Uncompressed:     ${stats.uncompressedFiles}`)
    console.log(`  Compressed (.gz): ${stats.compressedFiles}`)
    console.log()
    console.log(`Total size:         ${formatBytes(stats.totalSizeBytes)}`)
    console.log(`  Uncompressed:     ${formatBytes(stats.uncompressedSizeBytes)}`)
    console.log(`  Compressed:       ${formatBytes(stats.compressedSizeBytes)}`)
    console.log()
    console.log(`Date range:         ${stats.oldestDate ?? "N/A"} → ${stats.newestDate ?? "N/A"}`)
    console.log(`\nLog directory:      ${getLogDir()}`)
  },
}

const maintainCommand: CommandModule<object, TraceMaintainOptions> = {
  command: "maintain",
  describe: "Run full maintenance (compress + cleanup)",
  builder: (yargs) =>
    yargs
      .option("compress-after-days", {
        type: "number",
        alias: "c",
        default: 1,
        describe: "Compress logs older than this many days",
      })
      .option("retention-days", {
        type: "number",
        alias: "r",
        default: 7,
        describe: "Delete logs older than this many days",
      }),
  handler: async (args) => {
    const { runMaintenance, formatBytes, getStorageStats } = await import("../../trace/storage")

    const beforeStats = await getStorageStats({ logDir: getLogDir() })

    console.log("Running trace log maintenance...")
    console.log(`  Compress after: ${args.compressAfterDays} day(s)`)
    console.log(`  Retention:      ${args.retentionDays} days\n`)

    const result = await runMaintenance({
      logDir: getLogDir(),
      compressAfterDays: args.compressAfterDays ?? 1,
      retentionDays: args.retentionDays ?? 7,
    })

    if (result.compressed.length > 0) {
      console.log(`Compressed ${result.compressed.length} file(s)`)
    }

    if (result.deleted.length > 0) {
      console.log(`Deleted ${result.deleted.length} file(s)`)
    }

    if (result.errors.length > 0) {
      console.log(`\nErrors (${result.errors.length}):`)
      result.errors.forEach((e) => console.log(`  ✗ ${e.file}: ${e.error}`))
    }

    const afterStats = await getStorageStats({ logDir: getLogDir() })
    const savedBytes = beforeStats.totalSizeBytes - afterStats.totalSizeBytes

    console.log(`\nSpace saved: ${formatBytes(savedBytes > 0 ? savedBytes : 0)}`)
    console.log(`Current size: ${formatBytes(afterStats.totalSizeBytes)}`)
  },
}

const streamCommand: CommandModule<object, TraceStreamOptions> = {
  command: "stream",
  describe: "Stream trace logs in real-time via WebSocket (requires running server)",
  builder: (yargs) =>
    yargs
      .option("service", {
        type: "string",
        alias: "s",
        describe: "Filter by service name",
      })
      .option("level", {
        type: "string",
        alias: "l",
        choices: ["debug", "info", "warn", "error"],
        default: "info",
        describe: "Minimum log level",
      })
      .option("port", {
        type: "number",
        alias: "p",
        default: 4400,
        describe: "Server port",
      }),
  handler: async (args) => {
    const wsUrl = `ws://localhost:${args.port}/api/v1/trace/ws`

    console.log(`Connecting to trace stream at ${wsUrl}...`)
    console.log(`Filters: level=${args.level}${args.service ? `, service=${args.service}` : ""}`)
    console.log("Press Ctrl+C to stop\n")

    const levelColors: Record<string, string> = {
      debug: "\x1b[90m", // gray
      info: "\x1b[36m", // cyan
      warn: "\x1b[33m", // yellow
      error: "\x1b[31m", // red
    }
    const reset = "\x1b[0m"

    const formatEntry = (entry: {
      ts: string
      level: string
      service: string
      event_type: string
      payload?: Record<string, unknown>
    }): string => {
      const time = new Date(entry.ts).toLocaleTimeString()
      const level = entry.level.toUpperCase().padEnd(5)
      const service = entry.service.padEnd(15)
      const event = entry.event_type.padEnd(15)
      const color = levelColors[entry.level] ?? ""
      const func = (entry.payload?.function as string) ?? ""
      const duration = entry.payload?.duration_ms ? `${entry.payload.duration_ms}ms` : ""

      return `${color}${time} ${level}${reset} ${service} ${event} ${func} ${duration}`
    }

    try {
      const ws = new WebSocket(wsUrl)

      ws.onopen = () => {
        console.log("Connected. Streaming trace logs...\n")
        // Send filter configuration
        ws.send(
          JSON.stringify({
            type: "filter",
            filters: {
              level: args.level,
              service: args.service,
            },
          }),
        )
      }

      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data as string)
          switch (data.type) {
            case "entry":
              console.log(formatEntry(data.data))
              break
            case "batch":
              if (Array.isArray(data.data)) {
                data.data.forEach((entry: { ts: string; level: string; service: string; event_type: string; payload?: Record<string, unknown> }) => console.log(formatEntry(entry)))
              }
              break
            case "connected":
              // Already handled
              break
            case "filtered":
              console.log(`Filters updated: ${JSON.stringify(data.filters)}`)
              break
            case "error":
              console.error(`Error: ${data.message}`)
              break
          }
        } catch {
          // Skip malformed messages
        }
      }

      ws.onerror = (error) => {
        console.error("WebSocket error:", error)
        console.log("\nMake sure the server is running: bun dev serve")
        process.exit(1)
      }

      ws.onclose = () => {
        console.log("\nConnection closed")
        process.exit(0)
      }

      // Handle Ctrl+C
      process.on("SIGINT", () => {
        ws.close()
        process.exit(0)
      })

      // Keep running
      await new Promise(() => {})
    } catch (error) {
      console.error("Failed to connect:", error)
      console.log("\nMake sure the server is running: bun dev serve")
      process.exit(1)
    }
  },
}

// ============================================================================
// Main Command
// ============================================================================

const traceCommandImpl: CommandModule = {
  command: "trace <command>",
  describe: "Trace analysis commands",
  builder: (yargs: Argv) =>
    yargs
      .command(showCommand)
      .command(watchCommand)
      .command(streamCommand)
      .command(profileCommand)
      .command(errorsCommand)
      .command(treeCommand)
      .command(compressCommand)
      .command(cleanupCommand)
      .command(statsCommand)
      .command(maintainCommand)
      .demandCommand(1, "You must specify a subcommand"),
  handler: () => {},
}

export const TraceCommand = cmd(traceCommandImpl)
