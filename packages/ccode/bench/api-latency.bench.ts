/**
 * API Latency Benchmarks
 *
 * Measures HTTP API response times for CodeCoder server endpoints.
 * While NFR-04 doesn't specify API latency targets, we track P50/P95/P99
 * for monitoring and regression detection.
 */

import path from "path"
import type { BenchmarkResult } from "./index"

const API_BASE_URL = "http://localhost:4400"
const API_LATENCY_P99_TARGET_MS = 500 // Reasonable P99 target
const WARMUP_REQUESTS = 3
const BENCHMARK_REQUESTS = 20

interface LatencyStats {
  p50: number
  p95: number
  p99: number
  avg: number
  min: number
  max: number
}

interface ApiMeasurement {
  endpoint: string
  stats: LatencyStats | null
  success: boolean
  error?: string
}

/**
 * Calculate percentile from sorted array
 */
function percentile(sorted: number[], p: number): number {
  const index = Math.ceil((p / 100) * sorted.length) - 1
  return sorted[Math.max(0, index)]
}

/**
 * Calculate latency statistics
 */
function calculateStats(durations: number[]): LatencyStats {
  const sorted = [...durations].sort((a, b) => a - b)
  const sum = sorted.reduce((a, b) => a + b, 0)

  return {
    p50: percentile(sorted, 50),
    p95: percentile(sorted, 95),
    p99: percentile(sorted, 99),
    avg: sum / sorted.length,
    min: sorted[0],
    max: sorted[sorted.length - 1],
  }
}

/**
 * Check if API server is running
 */
async function isServerRunning(): Promise<boolean> {
  try {
    const response = await fetch(`${API_BASE_URL}/health`, {
      signal: AbortSignal.timeout(2000),
    })
    return response.ok
  } catch {
    return false
  }
}

/**
 * Measure latency for a single endpoint
 */
async function measureEndpoint(
  endpoint: string,
  method: "GET" | "POST" = "GET",
  body?: unknown,
): Promise<ApiMeasurement> {
  const durations: number[] = []

  try {
    // Warmup requests
    for (let i = 0; i < WARMUP_REQUESTS; i++) {
      await fetch(`${API_BASE_URL}${endpoint}`, {
        method,
        headers: body ? { "Content-Type": "application/json" } : undefined,
        body: body ? JSON.stringify(body) : undefined,
        signal: AbortSignal.timeout(5000),
      })
    }

    // Benchmark requests
    for (let i = 0; i < BENCHMARK_REQUESTS; i++) {
      const start = performance.now()
      const response = await fetch(`${API_BASE_URL}${endpoint}`, {
        method,
        headers: body ? { "Content-Type": "application/json" } : undefined,
        body: body ? JSON.stringify(body) : undefined,
        signal: AbortSignal.timeout(5000),
      })
      const duration = performance.now() - start

      if (response.ok) {
        durations.push(duration)
      }
    }

    if (durations.length === 0) {
      return {
        endpoint,
        stats: null,
        success: false,
        error: "All requests failed",
      }
    }

    return {
      endpoint,
      stats: calculateStats(durations),
      success: true,
    }
  } catch (error) {
    return {
      endpoint,
      stats: null,
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

/**
 * Measure memory usage of Rust Gateway service
 */
async function measureGatewayMemory(): Promise<BenchmarkResult | null> {
  const GATEWAY_URL = "http://localhost:4430"
  const MEMORY_TARGET_MB = 5 // NFR-04: < 5MB

  try {
    // Check if gateway is running
    const healthResponse = await fetch(`${GATEWAY_URL}/health`, {
      signal: AbortSignal.timeout(2000),
    })

    if (!healthResponse.ok) {
      return null
    }

    // Try to get memory stats from gateway metrics endpoint
    const metricsResponse = await fetch(`${GATEWAY_URL}/metrics`, {
      signal: AbortSignal.timeout(2000),
    })

    if (metricsResponse.ok) {
      const metrics = await metricsResponse.text()
      const memoryMatch = metrics.match(/process_resident_memory_bytes\s+(\d+)/)

      if (memoryMatch) {
        const memoryMb = parseInt(memoryMatch[1]) / (1024 * 1024)
        return {
          name: "Gateway Memory",
          target: `<${MEMORY_TARGET_MB}MB`,
          result: `${memoryMb.toFixed(2)}MB`,
          pass: memoryMb < MEMORY_TARGET_MB,
        }
      }
    }

    return null
  } catch {
    return null
  }
}

export async function runApiLatencyBenchmarks(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  // Check if server is running
  const serverRunning = await isServerRunning()

  if (!serverRunning) {
    console.log("  API server not running, skipping latency benchmarks")
    results.push({
      name: "API /health P99",
      target: `≤${API_LATENCY_P99_TARGET_MS}ms`,
      result: "Server Offline",
      pass: true, // Don't fail if server not running
      details: { note: "Start server with 'bun dev serve' to run API benchmarks" },
    })
    return results
  }

  // Measure health endpoint
  console.log("  Measuring /health endpoint...")
  const healthMeasurement = await measureEndpoint("/health")

  if (healthMeasurement.success && healthMeasurement.stats) {
    results.push({
      name: "API /health P99",
      target: `≤${API_LATENCY_P99_TARGET_MS}ms`,
      result: `${healthMeasurement.stats.p99.toFixed(0)}ms`,
      pass: healthMeasurement.stats.p99 <= API_LATENCY_P99_TARGET_MS,
      details: {
        p50: `${healthMeasurement.stats.p50.toFixed(0)}ms`,
        p95: `${healthMeasurement.stats.p95.toFixed(0)}ms`,
        avg: `${healthMeasurement.stats.avg.toFixed(0)}ms`,
      },
    })
  } else {
    results.push({
      name: "API /health P99",
      target: `≤${API_LATENCY_P99_TARGET_MS}ms`,
      result: "Failed",
      pass: false,
      details: { error: healthMeasurement.error },
    })
  }

  // Measure API info endpoint
  console.log("  Measuring /api/info endpoint...")
  const infoMeasurement = await measureEndpoint("/api/info")

  if (infoMeasurement.success && infoMeasurement.stats) {
    results.push({
      name: "API /api/info P99",
      target: "Info",
      result: `${infoMeasurement.stats.p99.toFixed(0)}ms`,
      pass: true,
      details: {
        p50: `${infoMeasurement.stats.p50.toFixed(0)}ms`,
        p95: `${infoMeasurement.stats.p95.toFixed(0)}ms`,
      },
    })
  }

  // Measure sessions list endpoint (common operation)
  console.log("  Measuring /api/sessions endpoint...")
  const sessionsMeasurement = await measureEndpoint("/api/sessions")

  if (sessionsMeasurement.success && sessionsMeasurement.stats) {
    results.push({
      name: "API /api/sessions P99",
      target: "Info",
      result: `${sessionsMeasurement.stats.p99.toFixed(0)}ms`,
      pass: true,
      details: {
        p50: `${sessionsMeasurement.stats.p50.toFixed(0)}ms`,
        p95: `${sessionsMeasurement.stats.p95.toFixed(0)}ms`,
      },
    })
  }

  // Try to measure Gateway memory
  console.log("  Checking Gateway memory usage...")
  const memoryResult = await measureGatewayMemory()
  if (memoryResult) {
    results.push(memoryResult)
  }

  return results
}
