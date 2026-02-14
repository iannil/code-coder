import path from "path"
import fs from "fs/promises"
import { Global } from "../global"
import type { EventType, LogEntry, ObservabilityConfig } from "./types"
import { getContext, addEntry } from "./trace-context"

function getEnvConfig(): Partial<ObservabilityConfig> {
  const envEnabled = process.env.CCODE_OBSERVABILITY_ENABLED
  const envLevel = process.env.CCODE_OBSERVABILITY_LEVEL
  const envSampling = process.env.CCODE_OBSERVABILITY_TRACE_SAMPLING

  const result: Partial<ObservabilityConfig> = {}

  if (envEnabled !== undefined) {
    result.enabled = envEnabled.toLowerCase() === "true"
  }
  if (envLevel && ["debug", "info", "warn", "error"].includes(envLevel.toLowerCase())) {
    result.level = envLevel.toLowerCase() as ObservabilityConfig["level"]
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

export async function init(): Promise<void> {
  if (!config.enabled) return

  const logDir = path.join(Global.Path.log, "observability")
  await fs.mkdir(logDir, { recursive: true })

  await cleanupLogs(logDir)

  const timestamp = new Date().toISOString().split(".")[0].replace(/:/g, "")
  logPath = path.join(logDir, `trace-${timestamp}.jsonl`)

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

  if (files.length <= 10) return

  const sorted = files.sort()
  const filesToDelete = sorted.slice(0, -10)
  await Promise.all(filesToDelete.map((file) => fs.unlink(file).catch(() => {})))
}

export function getLogPath(): string {
  return logPath
}

export interface StructuredLogOptions {
  eventType: EventType
  service?: string
  functionName?: string
  payload?: Record<string, unknown>
  durationMs?: number
  stackTrace?: string
  level?: string
}

export function log(options: StructuredLogOptions): void {
  if (!config.enabled) return
  if (!shouldSample()) return
  if (options.level && !shouldLog(options.level)) return

  const ctx = getContext()

  const entry: LogEntry = {
    timestamp: new Date().toISOString(),
    trace_id: ctx?.traceId ?? "no-trace",
    span_id: ctx?.spanId ?? "no-span",
    parent_span_id: ctx?.parentSpanId,
    event_type: options.eventType,
    service: options.service ?? ctx?.service ?? "unknown",
    function_name: options.functionName,
    payload: options.payload ?? {},
    duration_ms: options.durationMs,
    stack_trace: options.stackTrace,
  }

  addEntry(entry)

  if (writeToFile) {
    writeToFile(JSON.stringify(entry)).catch(() => {})
  }
}

export function functionStart(name: string, args?: Record<string, unknown>, service?: string): void {
  log({
    eventType: "Function_Start",
    functionName: name,
    payload: args ? { args } : {},
    service,
    level: "debug",
  })
}

export function functionEnd(
  name: string,
  result?: unknown,
  durationMs?: number,
  service?: string,
): void {
  log({
    eventType: "Function_End",
    functionName: name,
    payload: result !== undefined ? { result: safeSerialize(result) } : {},
    durationMs,
    service,
    level: "debug",
  })
}

export function functionError(
  name: string,
  error: unknown,
  durationMs?: number,
  service?: string,
): void {
  const errorPayload: Record<string, unknown> = {
    error: error instanceof Error ? error.message : String(error),
  }

  if (error instanceof Error && error.stack) {
    log({
      eventType: "Error",
      functionName: name,
      payload: errorPayload,
      durationMs,
      stackTrace: error.stack,
      service,
      level: "error",
    })
  } else {
    log({
      eventType: "Error",
      functionName: name,
      payload: errorPayload,
      durationMs,
      service,
      level: "error",
    })
  }
}

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
