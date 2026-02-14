import z from "zod"

export const EventType = z.enum([
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

export const LogEntry = z.object({
  timestamp: z.string(),
  trace_id: z.string(),
  span_id: z.string(),
  parent_span_id: z.string().optional(),
  event_type: EventType,
  service: z.string(),
  function_name: z.string().optional(),
  payload: z.record(z.string(), z.unknown()),
  duration_ms: z.number().optional(),
  stack_trace: z.string().optional(),
})
export type LogEntry = z.infer<typeof LogEntry>

export interface TraceContext {
  traceId: string
  spanId: string
  parentSpanId?: string
  service: string
  entries: LogEntry[]
  startTime: number
}

export interface TrackerOptions {
  logArgs?: boolean
  logResult?: boolean
  service?: string
}

export const ObservabilityConfig = z.object({
  enabled: z.boolean().default(true),
  level: z.enum(["debug", "info", "warn", "error"]).default("info"),
  sampling: z.number().min(0).max(1).default(1.0),
})
export type ObservabilityConfig = z.infer<typeof ObservabilityConfig>
