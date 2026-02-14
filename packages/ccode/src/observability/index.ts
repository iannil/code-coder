// Types
export type { EventType, LogEntry, TraceContext, TrackerOptions, ObservabilityConfig } from "./types"

// Trace Context Management
export {
  getContext,
  getTraceId,
  getSpanId,
  getEntries,
  createContext,
  runWithContext,
  runWithNewContext,
  runWithChildSpan,
  runWithContextAsync,
  runWithNewContextAsync,
  runWithChildSpanAsync,
} from "./trace-context"

// Structured Logging
export {
  init as initObservability,
  configure as configureObservability,
  getConfig as getObservabilityConfig,
  isEnabled,
  getLogPath,
  log,
  functionStart,
  functionEnd,
  functionError,
} from "./structured-log"

// Lifecycle Tracking
export { tracked, trackedAsync, createTracker } from "./lifecycle-tracker"

// Point (Instrumentation)
export { branch, loop, apiCall, point } from "./point"

// Report Generation
export {
  generateReport,
  formatReportAsText,
  formatReportAsJson,
  type ExecutionReport,
  type TimelineEntry,
  type ErrorEntry,
  type ApiCallEntry,
} from "./report"

// Convenience namespace for import
import * as TraceContext from "./trace-context"
import * as StructuredLog from "./structured-log"
import * as LifecycleTracker from "./lifecycle-tracker"
import * as Point from "./point"
import * as Report from "./report"

export const Observability = {
  TraceContext,
  StructuredLog,
  LifecycleTracker,
  Point,
  Report,
}
