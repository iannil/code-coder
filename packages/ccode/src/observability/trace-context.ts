import { AsyncLocalStorage } from "async_hooks"
import type { LogEntry, TraceContext } from "./types"
import { TraceHeaders } from "./types"

const storage = new AsyncLocalStorage<TraceContext>()

let traceCounter = 0
let spanCounter = 0

/**
 * Generate a UUID-format trace ID for cross-service compatibility.
 * Format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
 */
function generateTraceId(): string {
  // Use crypto.randomUUID if available (Node 19+, Bun), otherwise fallback
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID()
  }
  // Fallback for older environments
  const timestamp = Date.now().toString(36)
  const random = Math.random().toString(36).substring(2, 10)
  const counter = (traceCounter++).toString(36).padStart(4, "0")
  return `${timestamp}-${random}-${counter}-${Math.random().toString(36).substring(2, 6)}`
}

/**
 * Generate a short span ID (8 hex characters).
 * Matches zero-common/src/logging.rs generate_span_id()
 */
function generateSpanId(): string {
  const hex = Math.random().toString(16).substring(2, 10).padStart(8, "0")
  return hex
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

// ============================================================================
// HTTP Header Helpers - For cross-service trace propagation
// ============================================================================

/**
 * HTTP headers type (compatible with both native Headers and plain objects)
 */
type HeadersLike = Headers | Record<string, string | undefined> | { get(name: string): string | null }

/**
 * Extract trace context from HTTP request headers.
 * Creates a new context with the extracted trace_id or generates a new one.
 *
 * Matches: zero-common/src/logging.rs RequestContext::from_headers
 *
 * @param headers - HTTP headers (Request.headers, plain object, or Headers instance)
 * @param service - Service name to use for the context
 * @returns TraceContext with extracted or generated IDs
 */
export function fromHeaders(headers: HeadersLike, service: string): TraceContext {
  const getHeader = (name: string): string | undefined => {
    if (headers instanceof Headers) {
      return headers.get(name) ?? undefined
    }
    if (typeof (headers as any).get === "function") {
      return (headers as { get(name: string): string | null }).get(name) ?? undefined
    }
    return (headers as Record<string, string | undefined>)[name]
  }

  const traceId = getHeader(TraceHeaders.TRACE_ID) || generateTraceId()
  const parentSpanId = getHeader(TraceHeaders.SPAN_ID) // incoming span becomes our parent
  const userId = getHeader(TraceHeaders.USER_ID)

  return {
    traceId,
    spanId: generateSpanId(),
    parentSpanId,
    service,
    entries: [],
    startTime: Date.now(),
    userId,
  }
}

/**
 * Inject current trace context into HTTP request headers for propagation.
 *
 * Matches: zero-common/src/logging.rs RequestContext::to_headers
 *
 * @param headers - Mutable Headers object to inject into
 * @param ctx - Optional context (uses current context if not provided)
 */
export function toHeaders(headers: Headers, ctx?: TraceContext): void {
  const context = ctx ?? storage.getStore()
  if (!context) return

  headers.set(TraceHeaders.TRACE_ID, context.traceId)
  headers.set(TraceHeaders.SPAN_ID, context.spanId)
  if (context.parentSpanId) {
    headers.set(TraceHeaders.PARENT_SPAN_ID, context.parentSpanId)
  }
  if (context.userId) {
    headers.set(TraceHeaders.USER_ID, context.userId)
  }
}

/**
 * Create context from headers and run a function within it.
 * Useful for HTTP request handlers.
 *
 * @param headers - HTTP request headers
 * @param service - Service name
 * @param fn - Function to run within the context
 */
export function runWithHeaderContext<T>(headers: HeadersLike, service: string, fn: () => T): T {
  const context = fromHeaders(headers, service)
  return storage.run(context, fn)
}

/**
 * Async version of runWithHeaderContext
 */
export async function runWithHeaderContextAsync<T>(
  headers: HeadersLike,
  service: string,
  fn: () => Promise<T>,
): Promise<T> {
  const context = fromHeaders(headers, service)
  return storage.run(context, fn)
}
