import { AsyncLocalStorage } from "async_hooks"
import type { LogEntry, TraceContext } from "./types"

const storage = new AsyncLocalStorage<TraceContext>()

let traceCounter = 0
let spanCounter = 0

function generateTraceId(): string {
  const timestamp = Date.now().toString(36)
  const random = Math.random().toString(36).substring(2, 10)
  const counter = (traceCounter++).toString(36).padStart(4, "0")
  return `tr_${timestamp}_${random}_${counter}`
}

function generateSpanId(): string {
  const timestamp = Date.now().toString(36)
  const counter = (spanCounter++).toString(36).padStart(4, "0")
  return `sp_${timestamp}_${counter}`
}

export function getContext(): TraceContext | undefined {
  return storage.getStore()
}

export function getTraceId(): string | undefined {
  return storage.getStore()?.traceId
}

export function getSpanId(): string | undefined {
  return storage.getStore()?.spanId
}

export function getEntries(): LogEntry[] {
  return storage.getStore()?.entries ?? []
}

export function addEntry(entry: LogEntry): void {
  const ctx = storage.getStore()
  if (ctx) {
    ctx.entries.push(entry)
  }
}

export function createContext(service: string): TraceContext {
  return {
    traceId: generateTraceId(),
    spanId: generateSpanId(),
    service,
    entries: [],
    startTime: Date.now(),
  }
}

export function runWithContext<T>(context: TraceContext, fn: () => T): T {
  return storage.run(context, fn)
}

export function runWithNewContext<T>(service: string, fn: () => T): T {
  const context = createContext(service)
  return storage.run(context, fn)
}

export function runWithChildSpan<T>(fn: () => T): T {
  const parent = storage.getStore()
  if (!parent) {
    return fn()
  }

  const child: TraceContext = {
    traceId: parent.traceId,
    spanId: generateSpanId(),
    parentSpanId: parent.spanId,
    service: parent.service,
    entries: parent.entries,
    startTime: Date.now(),
  }

  return storage.run(child, fn)
}

export async function runWithContextAsync<T>(context: TraceContext, fn: () => Promise<T>): Promise<T> {
  return storage.run(context, fn)
}

export async function runWithNewContextAsync<T>(service: string, fn: () => Promise<T>): Promise<T> {
  const context = createContext(service)
  return storage.run(context, fn)
}

export async function runWithChildSpanAsync<T>(fn: () => Promise<T>): Promise<T> {
  const parent = storage.getStore()
  if (!parent) {
    return fn()
  }

  const child: TraceContext = {
    traceId: parent.traceId,
    spanId: generateSpanId(),
    parentSpanId: parent.spanId,
    service: parent.service,
    entries: parent.entries,
    startTime: Date.now(),
  }

  return storage.run(child, fn)
}
