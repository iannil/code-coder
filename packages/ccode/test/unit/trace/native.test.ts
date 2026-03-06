/**
 * Tests for native trace module integration
 */

import { describe, it, expect, beforeAll, afterAll } from "bun:test"
import {
  isNativeAvailable,
  createMemoryTraceStore,
  toNapiTraceEntry,
  fromNapiTraceEntry,
  type TraceStoreHandle,
  type NapiTraceEntry,
} from "@/trace/native"

describe("trace/native", () => {
  describe("toNapiTraceEntry", () => {
    it("should convert LogEntry to NapiTraceEntry", () => {
      const entry = {
        ts: "2026-03-04T12:00:00Z",
        trace_id: "trace-001",
        span_id: "span-001",
        parent_span_id: undefined,
        service: "test-service",
        event_type: "function_end",
        level: "info",
        payload: { function: "testFunc", duration_ms: 100 },
      }

      const result = toNapiTraceEntry(entry)

      expect(result.ts).toBe("2026-03-04T12:00:00Z")
      expect(result.traceId).toBe("trace-001")
      expect(result.spanId).toBe("span-001")
      expect(result.service).toBe("test-service")
      expect(result.eventType).toBe("function_end")
      expect(result.level).toBe("info")
      expect(JSON.parse(result.payload)).toEqual({ function: "testFunc", duration_ms: 100 })
    })
  })

  describe("fromNapiTraceEntry", () => {
    it("should convert NapiTraceEntry back to LogEntry-like object", () => {
      const napiEntry: NapiTraceEntry = {
        ts: "2026-03-04T12:00:00Z",
        traceId: "trace-001",
        spanId: "span-001",
        parentSpanId: undefined,
        service: "test-service",
        eventType: "function_end",
        level: "info",
        payload: JSON.stringify({ function: "testFunc", duration_ms: 100 }),
      }

      const result = fromNapiTraceEntry(napiEntry)

      expect(result.ts).toBe("2026-03-04T12:00:00Z")
      expect(result.trace_id).toBe("trace-001")
      expect(result.span_id).toBe("span-001")
      expect(result.service).toBe("test-service")
      expect(result.event_type).toBe("function_end")
      expect(result.level).toBe("info")
      expect(result.payload).toEqual({ function: "testFunc", duration_ms: 100 })
    })
  })
})

// These tests require native bindings to be available
describe("trace/native (native bindings)", () => {
  let store: TraceStoreHandle | null = null
  let nativeAvailable = false

  beforeAll(async () => {
    nativeAvailable = await isNativeAvailable()
    if (nativeAvailable) {
      store = await createMemoryTraceStore()
    }
  })

  afterAll(() => {
    store = null
  })

  it("should check native availability", async () => {
    const available = await isNativeAvailable()
    // This can be true or false depending on build
    expect(typeof available).toBe("boolean")
  })

  describe("when native available", () => {
    it("should create in-memory store", async () => {
      if (!nativeAvailable) {
        console.log("Skipping: native bindings not available")
        return
      }

      expect(store).not.toBeNull()
      expect(store!.healthCheck()).toBe(true)
    })

    it("should append and query trace entries", async () => {
      if (!nativeAvailable || !store) {
        console.log("Skipping: native bindings not available")
        return
      }

      const entry = toNapiTraceEntry({
        ts: new Date().toISOString(),
        trace_id: "test-trace-001",
        span_id: "span-001",
        parent_span_id: undefined,
        service: "test-service",
        event_type: "function_end",
        level: "info",
        payload: { function: "testFunc", duration_ms: 100 },
      })

      store.append(entry)

      const results = store.queryByTraceId("test-trace-001")
      expect(results.length).toBeGreaterThanOrEqual(1)
      expect(results[0].traceId).toBe("test-trace-001")
    })

    it("should batch append entries", async () => {
      if (!nativeAvailable || !store) {
        console.log("Skipping: native bindings not available")
        return
      }

      const entries = [
        toNapiTraceEntry({
          ts: new Date().toISOString(),
          trace_id: "batch-trace-001",
          span_id: "span-001",
          service: "svc-a",
          event_type: "function_start",
          level: "info",
          payload: {},
        }),
        toNapiTraceEntry({
          ts: new Date().toISOString(),
          trace_id: "batch-trace-001",
          span_id: "span-002",
          service: "svc-a",
          event_type: "function_end",
          level: "info",
          payload: { duration_ms: 50 },
        }),
      ]

      const inserted = store.appendBatch(entries)
      expect(inserted).toBe(2)
    })

    it("should get services", async () => {
      if (!nativeAvailable || !store) {
        console.log("Skipping: native bindings not available")
        return
      }

      const services = store.getServices()
      expect(Array.isArray(services)).toBe(true)
    })

    it("should get stats", async () => {
      if (!nativeAvailable || !store) {
        console.log("Skipping: native bindings not available")
        return
      }

      const stats = store.stats()
      expect(stats.totalEntries).toBeGreaterThanOrEqual(0)
      expect(typeof stats.totalSizeBytes).toBe("number")
    })

    it("should profile traces", async () => {
      if (!nativeAvailable || !store) {
        console.log("Skipping: native bindings not available")
        return
      }

      const fromTs = new Date(Date.now() - 86400000).toISOString() // 24h ago
      const profile = store.profile(fromTs, 10)

      expect(typeof profile.totalTraces).toBe("number")
      expect(typeof profile.totalEvents).toBe("number")
      expect(Array.isArray(profile.slowest)).toBe(true)
      expect(Array.isArray(profile.byService)).toBe(true)
      expect(Array.isArray(profile.byFunction)).toBe(true)
    })

    it("should aggregate errors", async () => {
      if (!nativeAvailable || !store) {
        console.log("Skipping: native bindings not available")
        return
      }

      // Add an error entry
      store.append(
        toNapiTraceEntry({
          ts: new Date().toISOString(),
          trace_id: "error-trace-001",
          span_id: "span-001",
          service: "test-service",
          event_type: "error",
          level: "error",
          payload: { error: "Test error", function: "failingFunc" },
        }),
      )

      const fromTs = new Date(Date.now() - 86400000).toISOString()
      const summary = store.aggregateErrors(fromTs, "service")

      expect(typeof summary.total).toBe("number")
      expect(Array.isArray(summary.groups)).toBe(true)
    })

    it("should cleanup old entries", async () => {
      if (!nativeAvailable || !store) {
        console.log("Skipping: native bindings not available")
        return
      }

      // Cleanup entries older than 365 days (shouldn't delete anything in this test)
      const deleted = store.cleanup(365)
      expect(typeof deleted).toBe("number")
    })
  })
})
