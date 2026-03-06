/**
 * Trace Module
 * Provides trace log analysis, querying, and visualization
 */

export * from "./query"
export * from "./visualizer"
export * from "./profiler"
export * from "./storage"
export * as TraceNative from "./native"
export {
  isNativeAvailable as isTraceNativeAvailable,
  isUsingNative as isTraceUsingNative,
  openTraceStore,
  createMemoryTraceStore,
  getGlobalTraceStore,
  type TraceStoreHandle,
  type NapiTraceEntry,
  type NapiTraceFilter,
  type NapiProfileResult,
  type NapiErrorSummary,
} from "./native"
