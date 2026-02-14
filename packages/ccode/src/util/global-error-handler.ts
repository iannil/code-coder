import * as path from "path"
import * as fs from "fs"

export namespace GlobalErrorHandler {
  let logFile: string | null = null
  let initialized = false

  // Store recent context for debugging
  const recentContext: Array<{ timestamp: number; type: string; data: unknown }> = []
  const MAX_CONTEXT_ITEMS = 20

  function formatTimestamp(): string {
    return new Date().toISOString()
  }

  // Safe stringify that handles circular refs and limits depth
  function safeStringify(value: unknown, maxDepth = 4): string {
    const seen = new WeakSet()
    const stringify = (val: unknown, depth: number): string => {
      if (depth > maxDepth) return "[MAX_DEPTH]"
      if (val === null) return "null"
      if (val === undefined) return "undefined"
      if (typeof val === "string") {
        const truncated = val.length > 500 ? val.slice(0, 500) + `... [+${val.length - 500} chars]` : val
        return `"${truncated}"`
      }
      if (typeof val === "number" || typeof val === "boolean") return String(val)
      if (typeof val === "bigint") return `${val}n`
      if (typeof val === "function") return `[Function: ${val.name || "anonymous"}]`
      if (typeof val === "symbol") return val.toString()
      if (val instanceof Error) {
        return `[Error: ${val.name}] ${val.message}`
      }
      if (Array.isArray(val)) {
        if (seen.has(val)) return "[Circular]"
        seen.add(val)
        const items = val.slice(0, 10).map((v) => stringify(v, depth + 1))
        return `[${items.join(", ")}${val.length > 10 ? `, ... +${val.length - 10} more` : ""}]`
      }
      if (typeof val === "object") {
        if (seen.has(val)) return "[Circular]"
        seen.add(val)
        try {
          const entries = Object.entries(val).slice(0, 20)
          const items = entries.map(([k, v]) => `${k}: ${stringify(v, depth + 1)}`)
          const extra = Object.keys(val).length > 20 ? `, ... +${Object.keys(val).length - 20} more` : ""
          return `{${items.join(", ")}${extra}}`
        } catch {
          return "[Object]"
        }
      }
      return String(val)
    }
    try {
      return stringify(value, 0)
    } catch {
      return "[STRINGIFY_ERROR]"
    }
  }

  function formatError(error: unknown): string {
    if (error instanceof Error) {
      const lines = [
        `  Name: ${error.name}`,
        `  Message: ${error.message}`,
        `  Stack: ${error.stack ?? "N/A"}`,
      ]
      if (error.cause) {
        lines.push(`  Cause: ${formatError(error.cause)}`)
      }
      return lines.join("\n")
    }
    return `  Value: ${safeStringify(error)}`
  }

  function writeLog(level: string, type: string, error: unknown, context?: Record<string, unknown>): void {
    if (!logFile) return
    const timestamp = formatTimestamp()
    const formatted = formatError(error)

    const lines = [`[${timestamp}] [${level}] ${type}`, formatted]

    // Add context if provided
    if (context && Object.keys(context).length > 0) {
      lines.push(`  Context:`)
      for (const [key, value] of Object.entries(context)) {
        lines.push(`    ${key}: ${safeStringify(value)}`)
      }
    }

    // Add recent context history for debugging
    if (recentContext.length > 0) {
      lines.push(`  Recent Activity (last ${recentContext.length} items):`)
      for (const item of recentContext.slice(-5)) {
        const relativeTime = Date.now() - item.timestamp
        lines.push(`    [${relativeTime}ms ago] ${item.type}: ${safeStringify(item.data)}`)
      }
    }

    lines.push("")
    const entry = lines.join("\n")

    try {
      fs.appendFileSync(logFile, entry)
    } catch {
      // Fallback to stderr if file write fails
      process.stderr.write(entry)
    }
  }

  export function init(projectDir?: string): void {
    if (initialized) return
    initialized = true

    // Determine log file path
    const dir = projectDir ?? process.cwd()
    logFile = path.join(dir, "dev.log")

    // Ensure log file exists and add init marker
    try {
      fs.appendFileSync(logFile, `[${formatTimestamp()}] [INFO] Global error handler initialized\n`)
    } catch {
      logFile = null
      return
    }

    // Handle unhandled promise rejections
    process.on("unhandledRejection", (reason, promise) => {
      writeLog("ERROR", "Unhandled Promise Rejection", reason, {
        promise: String(promise),
      })
    })

    // Handle uncaught exceptions
    process.on("uncaughtException", (error, origin) => {
      writeLog("FATAL", `Uncaught Exception (${origin})`, error)
    })

    // Handle uncaught exception monitor (non-terminating)
    process.on("uncaughtExceptionMonitor", (error, origin) => {
      writeLog("ERROR", `Exception Monitor (${origin})`, error)
    })

    // Handle warnings
    process.on("warning", (warning) => {
      writeLog("WARN", "Process Warning", warning)
    })
  }

  /**
   * Log an error with optional context
   */
  export function logError(type: string, error: unknown, context?: Record<string, unknown>): void {
    writeLog("ERROR", type, error, context)
  }

  /**
   * Log info message
   */
  export function logInfo(message: string, data?: Record<string, unknown>): void {
    if (!logFile) return
    const timestamp = formatTimestamp()
    const extra = data ? `\n  ${safeStringify(data)}` : ""
    const entry = `[${timestamp}] [INFO] ${message}${extra}\n`
    try {
      fs.appendFileSync(logFile, entry)
    } catch {
      // Ignore
    }
  }

  /**
   * Add context for debugging (kept in memory, logged when error occurs)
   */
  export function addContext(type: string, data: unknown): void {
    recentContext.push({
      timestamp: Date.now(),
      type,
      data,
    })
    // Keep only recent items
    while (recentContext.length > MAX_CONTEXT_ITEMS) {
      recentContext.shift()
    }
  }

  /**
   * Log debug information (verbose logging)
   */
  export function logDebug(type: string, data: unknown): void {
    if (!logFile) return
    const timestamp = formatTimestamp()
    const entry = `[${timestamp}] [DEBUG] ${type}\n  ${safeStringify(data)}\n`
    try {
      fs.appendFileSync(logFile, entry)
    } catch {
      // Ignore
    }
  }

  export function getLogPath(): string | null {
    return logFile
  }
}
