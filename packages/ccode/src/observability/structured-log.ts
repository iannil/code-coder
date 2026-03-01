import path from "path"
import fs from "fs/promises"
import os from "os"
import type { LogEntry, LogLevel, ObservabilityConfig } from "./types"
import { getContext, addEntry } from "./trace-context"

// ============================================================================
// Configuration
// ============================================================================

function getEnvConfig(): Partial<ObservabilityConfig> {
  const envEnabled = process.env.CCODE_OBSERVABILITY_ENABLED
  const envLevel = process.env.CCODE_OBSERVABILITY_LEVEL
  const envSampling = process.env.CCODE_OBSERVABILITY_TRACE_SAMPLING

  const result: Partial<ObservabilityConfig> = {}

  if (envEnabled !== undefined) {
    result.enabled = envEnabled.toLowerCase() === "true"
  }
  if (envLevel && ["debug", "info", "warn", "error"].includes(envLevel.toLowerCase())) {
    result.level = envLevel.toLowerCase() as LogLevel
  }
  if (envSampling !== undefined) {
    const parsed = parseFloat(envSampling)
    if (!isNaN(parsed) && parsed >= 0 && parsed <= 1) {
      result.sampling = parsed
    }
  }

  return result
}

const envConfig = getEnvConfig()

let config: ObservabilityConfig = {
  enabled: envConfig.enabled ?? true,
  level: envConfig.level ?? "info",
  sampling: envConfig.sampling ?? 1.0,
  retentionDays: 7,
}

let logPath = ""
let writeToFile: ((data: string) => Promise<void>) | undefined

const levelPriority: Record<string, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
}

function shouldLog(level: string): boolean {
  return levelPriority[level] >= levelPriority[config.level]
}

function shouldSample(): boolean {
  return config.sampling >= 1.0 || Math.random() < config.sampling
}

export function isEnabled(): boolean {
  return config.enabled
}

export function configure(newConfig: Partial<ObservabilityConfig>): void {
  config = { ...config, ...newConfig }
}

export function getConfig(): ObservabilityConfig {
  return { ...config }
}

// ============================================================================
// Initialization
// ============================================================================

/**
 * Get the default log directory (~/.codecoder/logs)
 */
function getDefaultLogDir(): string {
  return config.logDir ?? path.join(os.homedir(), ".codecoder", "logs")
}

export async function init(): Promise<void> {
  if (!config.enabled) return

  const logDir = getDefaultLogDir()
  await fs.mkdir(logDir, { recursive: true })

  await cleanupLogs(logDir)

  // Use date-based filename for daily rotation: trace-YYYY-MM-DD.jsonl
  const date = new Date().toISOString().split("T")[0]
  logPath = path.join(logDir, `trace-${date}.jsonl`)

  const logFile = Bun.file(logPath)
  const writer = logFile.writer()

  writeToFile = async (data: string) => {
    writer.write(data + "\n")
    writer.flush()
  }
}

async function cleanupLogs(dir: string): Promise<void> {
  const glob = new Bun.Glob("trace-*.jsonl")
  const files = await Array.fromAsync(
    glob.scan({
      cwd: dir,
      absolute: true,
    }),
  )

  // Keep files from the last N days
  const cutoffDate = new Date()
  cutoffDate.setDate(cutoffDate.getDate() - config.retentionDays)
  const cutoffStr = cutoffDate.toISOString().split("T")[0]

  const filesToDelete = files.filter((f) => {
    const match = path.basename(f).match(/trace-(\d{4}-\d{2}-\d{2})\.jsonl/)
    return match && match[1] < cutoffStr
  })

  await Promise.all(filesToDelete.map((file) => fs.unlink(file).catch(() => {})))
}

export function getLogPath(): string {
  return logPath
}

// ============================================================================
// Logging Functions - Unified Format
// ============================================================================

/**
 * Map legacy event types to unified snake_case format
 */
function normalizeEventType(eventType: string): string {
  const mapping: Record<string, string> = {
    Function_Start: "function_start",
    Function_End: "function_end",
    Branch: "branch",
    Loop: "branch",
    API_Call_Start: "api_call",
    API_Call_End: "api_call",
    Error: "error",
    Point: "branch",
  }
  return mapping[eventType] ?? eventType
}

export interface StructuredLogOptions {
  eventType: string
  service?: string
  functionName?: string
  payload?: Record<string, unknown>
  durationMs?: number
  stackTrace?: string
  level?: LogLevel
}

/**
 * Log a structured event in the unified format.
 * Output matches zero-common/src/logging.rs LifecycleEvent
 */
export function log(options: StructuredLogOptions): void {
  if (!config.enabled) return
  if (!shouldSample()) return
  if (options.level && !shouldLog(options.level)) return

  const ctx = getContext()

  // Build payload with function name included (matches Rust format)
  const payload: Record<string, unknown> = { ...options.payload }
  if (options.functionName) {
    payload.function = options.functionName
  }
  if (options.durationMs !== undefined) {
    payload.duration_ms = options.durationMs
  }
  if (options.stackTrace) {
    payload.stack_trace = options.stackTrace
  }

  const entry: LogEntry = {
    ts: new Date().toISOString(),
    trace_id: ctx?.traceId ?? "no-trace",
    span_id: ctx?.spanId ?? "no-span",
    parent_span_id: ctx?.parentSpanId,
    service: options.service ?? ctx?.service ?? "ccode-api",
    event_type: normalizeEventType(options.eventType),
    level: options.level ?? "info",
    payload,
  }

  addEntry(entry)

  if (writeToFile) {
    writeToFile(JSON.stringify(entry)).catch(() => {})
  }
}

/**
 * Log function start event
 */
export function functionStart(name: string, args?: Record<string, unknown>, service?: string): void {
  log({
    eventType: "function_start",
    functionName: name,
    payload: args ? { args: safeSerialize(args) } : {},
    service,
    level: "debug",
  })
}

/**
 * Log function end event
 */
export function functionEnd(name: string, result?: unknown, durationMs?: number, service?: string): void {
  log({
    eventType: "function_end",
    functionName: name,
    payload: result !== undefined ? { result: safeSerialize(result) } : {},
    durationMs,
    service,
    level: "debug",
  })
}

/**
 * Log error event
 */
export function functionError(name: string, error: unknown, durationMs?: number, service?: string): void {
  const errorMessage = error instanceof Error ? error.message : String(error)
  const stackTrace = error instanceof Error ? error.stack : undefined

  log({
    eventType: "error",
    functionName: name,
    payload: { error: errorMessage },
    durationMs,
    stackTrace,
    service,
    level: "error",
  })
}

/**
 * Log HTTP request event
 */
export function httpRequest(
  method: string,
  path: string,
  extra?: Record<string, unknown>,
  service?: string,
): void {
  log({
    eventType: "http_request",
    payload: { method, path, ...extra },
    service,
    level: "info",
  })
}

/**
 * Log HTTP response event
 */
export function httpResponse(
  method: string,
  path: string,
  status: number,
  durationMs: number,
  extra?: Record<string, unknown>,
  service?: string,
): void {
  log({
    eventType: "http_response",
    payload: { method, path, status, ...extra },
    durationMs,
    service,
    level: "info",
  })
}

/**
 * Log API call event (for external service calls)
 */
export function apiCall(
  endpoint: string,
  method: string,
  durationMs?: number,
  success?: boolean,
  service?: string,
): void {
  log({
    eventType: "api_call",
    payload: { endpoint, method, success },
    durationMs,
    service,
    level: "info",
  })
}

// ============================================================================
// Utilities
// ============================================================================

function safeSerialize(value: unknown, depth = 0): unknown {
  if (depth > 3) return "[max depth]"
  if (value === null || value === undefined) return value
  if (typeof value === "string") return value.length > 1000 ? value.slice(0, 1000) + "..." : value
  if (typeof value === "number" || typeof value === "boolean") return value
  if (typeof value === "function") return "[function]"
  if (value instanceof Error) return { message: value.message, name: value.name }
  if (Array.isArray(value)) {
    return value.length > 10
      ? [...value.slice(0, 10).map((v) => safeSerialize(v, depth + 1)), `... ${value.length - 10} more`]
      : value.map((v) => safeSerialize(v, depth + 1))
  }
  if (typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
    const limited = entries.slice(0, 20)
    const result: Record<string, unknown> = {}
    for (const [k, v] of limited) {
      result[k] = safeSerialize(v, depth + 1)
    }
    if (entries.length > 20) {
      result["..."] = `${entries.length - 20} more keys`
    }
    return result
  }
  return String(value)
}
