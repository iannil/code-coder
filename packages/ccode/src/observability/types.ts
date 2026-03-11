import z from "zod"

// ============================================================================
// Event Types - Unified across TypeScript and Rust
// ============================================================================

/**
 * Unified event types matching zero-common/src/logging.rs LifecycleEventType
 */
export const EventType = z.enum([
  "function_start",
  "function_end",
  "branch",
  "error",
  "api_call",
  "http_request",
  "http_response",
  // Legacy types (mapped for backwards compatibility)
  "Function_Start",
  "Function_End",
  "Branch",
  "Loop",
  "API_Call_Start",
  "API_Call_End",
  "Error",
  "Point",
])
export type EventType = z.infer<typeof EventType>

/**
 * Log level enum matching zero-common
 */
export const LogLevel = z.enum(["debug", "info", "warn", "error"])
export type LogLevel = z.infer<typeof LogLevel>

// ============================================================================
// Unified Log Entry - Cross-language compatible
// ============================================================================

/**
 * Unified LogEntry format for both TypeScript (ccode) and Rust (zero-*) services.
 * Designed for easy analysis with jq and cross-service trace correlation.
 *
 * Format aligns with: services/zero-common/src/logging.rs LifecycleEvent
 */
export const LogEntry = z.object({
  /** ISO 8601 timestamp (e.g., "2026-02-28T10:30:00.123Z") */
  ts: z.string(),
  /** Unique trace ID for the request chain (UUID format) */
  trace_id: z.string(),
  /** Current span ID (8-char hex) */
  span_id: z.string(),
  /** Parent span ID if this is a child span */
  parent_span_id: z.string().optional(),
  /** Service name (e.g., "ccode-api", "zero-channels") */
  service: z.string(),
  /** Event type (snake_case for cross-language compatibility) */
  event_type: z.string(),
  /** Log level */
  level: LogLevel,
  /** Structured payload with event-specific data */
  payload: z.object({
    /** Function name for function_start/function_end events */
    function: z.string().optional(),
    /** Duration in milliseconds for function_end events */
    duration_ms: z.number().optional(),
    /** Arguments for function_start events */
    args: z.unknown().optional(),
    /** Result for function_end events */
    result: z.unknown().optional(),
    /** Error message for error events */
    error: z.string().optional(),
    /** Stack trace for error events */
    stack_trace: z.string().optional(),
  }).passthrough(),
})
export type LogEntry = z.infer<typeof LogEntry>

// ============================================================================
// Trace Context
// ============================================================================

export interface TraceContext {
  traceId: string
  spanId: string
  parentSpanId?: string
  service: string
  entries: LogEntry[]
  startTime: number
  /** User ID if authenticated */
  userId?: string
  /** Additional baggage for context propagation */
  baggage?: Record<string, string>
}

export interface TrackerOptions {
  logArgs?: boolean
  logResult?: boolean
  service?: string
}

// ============================================================================
// HTTP Headers for Trace Propagation
// ============================================================================

/**
 * Standard HTTP headers for distributed tracing.
 * Must match zero-common/src/logging.rs RequestContext::from_headers
 */
export const TraceHeaders = {
  TRACE_ID: "X-Trace-Id",
  SPAN_ID: "X-Span-Id",
  PARENT_SPAN_ID: "X-Parent-Span-Id",
  USER_ID: "X-User-Id",
} as const

// ============================================================================
// Configuration
// ============================================================================

export const ObservabilityConfig = z.object({
  /** Enable/disable observability */
  enabled: z.boolean().default(true),
  /** Minimum log level */
  level: LogLevel.default("info"),
  /** Sampling rate (0.0 to 1.0) */
  sampling: z.number().min(0).max(1).default(1.0),
  /** Log file directory (defaults to ~/.codecoder/logs/trace-YYYY-MM-DD.jsonl) */
  logDir: z.string().optional(),
  /** Log file rotation: days to keep */
  retentionDays: z.number().default(7),
})
export type ObservabilityConfig = z.infer<typeof ObservabilityConfig>
