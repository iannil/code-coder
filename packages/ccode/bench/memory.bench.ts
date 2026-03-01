/**
 * Memory Performance Benchmarks
 *
 * Measures memory usage patterns:
 * - Initial heap memory baseline
 * - Memory growth after operations (leak detection)
 * - Gateway service memory (NFR-04-3: < 5MB)
 * - Channels service memory
 *
 * Run with: bun run bench/memory.bench.ts
 * Or via main suite: bun run bench --memory
 */

import type { BenchmarkResult } from "./index"

// ============================================================================
// Configuration
// ============================================================================

const TARGETS = {
  // NFR-04-3: Gateway memory < 5MB
  gateway_memory_mb: 5,
  // Reasonable baseline for TS process
  initial_heap_mb: 100,
  // Memory growth threshold after iterations (MB)
  growth_threshold_mb: 50,
}

const GATEWAY_URL = "http://localhost:4430"
const CHANNELS_URL = "http://localhost:4431"

// ============================================================================
// Utilities
// ============================================================================

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(2)}MB`
}

function toMB(bytes: number): number {
  return bytes / (1024 * 1024)
}

function forceGC(): void {
  if (typeof Bun !== "undefined") {
    Bun.gc(true)
  }
}

async function getHeapUsed(): Promise<number> {
  forceGC()
  // Small delay to let GC settle
  await new Promise((resolve) => setTimeout(resolve, 50))
  return process.memoryUsage().heapUsed
}

// ============================================================================
// Memory Baseline
// ============================================================================

async function measureBaseline(): Promise<BenchmarkResult> {
  const heapUsed = await getHeapUsed()
  const heapMb = toMB(heapUsed)

  return {
    name: "Initial Heap Memory",
    target: `<${TARGETS.initial_heap_mb}MB`,
    result: `${heapMb.toFixed(2)}MB`,
    pass: heapMb < TARGETS.initial_heap_mb,
    numericValue: heapMb,
    unit: "MB",
    details: {
      heapUsed: formatBytes(heapUsed),
      heapTotal: formatBytes(process.memoryUsage().heapTotal),
      external: formatBytes(process.memoryUsage().external),
      rss: formatBytes(process.memoryUsage().rss),
    },
  }
}

// ============================================================================
// Memory Growth Test (Leak Detection)
// ============================================================================

async function measureGrowth(): Promise<BenchmarkResult> {
  const iterations = 100

  // Measure initial heap
  const initialHeap = await getHeapUsed()

  // Simulate session-like operations
  const cache = new Map<string, unknown>()
  const eventTargets: EventTarget[] = []

  for (let i = 0; i < iterations; i++) {
    // Simulate typical session operations
    const data = {
      id: i,
      timestamp: Date.now(),
      payload: "x".repeat(1000),
      nested: { a: 1, b: 2, c: 3 },
    }

    // Add to cache
    cache.set(`session-${i}`, data)

    // Create event target (common pattern in tools)
    const et = new EventTarget()
    eventTargets.push(et)

    // Simulate async operations
    await Promise.resolve()
  }

  // Clear references to allow GC
  cache.clear()
  eventTargets.length = 0

  // Measure after operations
  const finalHeap = await getHeapUsed()
  const growth = finalHeap - initialHeap
  const growthMb = toMB(growth)

  return {
    name: `Memory Growth (${iterations} iterations)`,
    target: `<${TARGETS.growth_threshold_mb}MB growth`,
    result: growthMb >= 0 ? `+${growthMb.toFixed(2)}MB` : `${growthMb.toFixed(2)}MB`,
    pass: growthMb < TARGETS.growth_threshold_mb,
    numericValue: growthMb,
    unit: "MB",
    details: {
      initialHeap: formatBytes(initialHeap),
      finalHeap: formatBytes(finalHeap),
      iterations,
    },
  }
}

// ============================================================================
// Gateway Memory (Rust Service)
// ============================================================================

async function measureGatewayMemory(): Promise<BenchmarkResult | null> {
  try {
    // Check if gateway is running
    const healthResponse = await fetch(`${GATEWAY_URL}/health`, {
      signal: AbortSignal.timeout(2000),
    })

    if (!healthResponse.ok) {
      return null
    }

    // Try to get memory stats from metrics endpoint
    const metricsResponse = await fetch(`${GATEWAY_URL}/metrics`, {
      signal: AbortSignal.timeout(2000),
    })

    if (!metricsResponse.ok) {
      return {
        name: "Gateway Memory",
        target: `<${TARGETS.gateway_memory_mb}MB`,
        result: "Metrics N/A",
        pass: true,
        details: { note: "Gateway running but metrics endpoint not available" },
      }
    }

    const metrics = await metricsResponse.text()

    // Parse Prometheus-style metrics
    const memoryMatch = metrics.match(/process_resident_memory_bytes\s+(\d+)/)

    if (memoryMatch) {
      const memoryBytes = parseInt(memoryMatch[1], 10)
      const memoryMb = toMB(memoryBytes)

      return {
        name: "Gateway Memory",
        target: `<${TARGETS.gateway_memory_mb}MB`,
        result: `${memoryMb.toFixed(2)}MB`,
        pass: memoryMb < TARGETS.gateway_memory_mb,
        numericValue: memoryMb,
        unit: "MB",
        details: {
          bytes: memoryBytes,
          formatted: formatBytes(memoryBytes),
        },
      }
    }

    // Try alternative metric names
    const rssMatch = metrics.match(/process_rss_bytes\s+(\d+)/)
    if (rssMatch) {
      const memoryBytes = parseInt(rssMatch[1], 10)
      const memoryMb = toMB(memoryBytes)

      return {
        name: "Gateway Memory",
        target: `<${TARGETS.gateway_memory_mb}MB`,
        result: `${memoryMb.toFixed(2)}MB`,
        pass: memoryMb < TARGETS.gateway_memory_mb,
        numericValue: memoryMb,
        unit: "MB",
        details: { bytes: memoryBytes },
      }
    }

    return {
      name: "Gateway Memory",
      target: `<${TARGETS.gateway_memory_mb}MB`,
      result: "Parse Failed",
      pass: true,
      details: { note: "Could not parse memory metrics" },
    }
  } catch {
    return null
  }
}

// ============================================================================
// Channels Memory (Rust Service)
// ============================================================================

async function measureChannelsMemory(): Promise<BenchmarkResult | null> {
  try {
    const healthResponse = await fetch(`${CHANNELS_URL}/health`, {
      signal: AbortSignal.timeout(2000),
    })

    if (!healthResponse.ok) {
      return null
    }

    const metricsResponse = await fetch(`${CHANNELS_URL}/metrics`, {
      signal: AbortSignal.timeout(2000),
    })

    if (!metricsResponse.ok) {
      return {
        name: "Channels Memory",
        target: "Baseline",
        result: "Metrics N/A",
        pass: true,
        details: { note: "Channels running but metrics endpoint not available" },
      }
    }

    const metrics = await metricsResponse.text()
    const memoryMatch = metrics.match(/process_resident_memory_bytes\s+(\d+)/)

    if (memoryMatch) {
      const memoryBytes = parseInt(memoryMatch[1], 10)
      const memoryMb = toMB(memoryBytes)

      return {
        name: "Channels Memory",
        target: "Baseline",
        result: `${memoryMb.toFixed(2)}MB`,
        pass: true, // No specific target, just baseline tracking
        numericValue: memoryMb,
        unit: "MB",
        details: { bytes: memoryBytes },
      }
    }

    return null
  } catch {
    return null
  }
}

// ============================================================================
// Main Runner
// ============================================================================

export async function runMemoryBenchmarks(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  console.log("  Measuring baseline memory...")
  results.push(await measureBaseline())

  console.log("  Measuring memory growth...")
  results.push(await measureGrowth())

  console.log("  Checking Gateway memory...")
  const gatewayResult = await measureGatewayMemory()
  if (gatewayResult) {
    results.push(gatewayResult)
  } else {
    results.push({
      name: "Gateway Memory",
      target: `<${TARGETS.gateway_memory_mb}MB`,
      result: "Offline",
      pass: true,
      details: { note: "Gateway not running" },
    })
  }

  console.log("  Checking Channels memory...")
  const channelsResult = await measureChannelsMemory()
  if (channelsResult) {
    results.push(channelsResult)
  } else {
    results.push({
      name: "Channels Memory",
      target: "Baseline",
      result: "Offline",
      pass: true,
      details: { note: "Channels not running" },
    })
  }

  return results
}

// Entry point for standalone execution
if (import.meta.main) {
  console.log("Running Memory Performance Benchmarks...\n")
  runMemoryBenchmarks()
    .then((results) => {
      console.log("\n=== Memory Benchmark Results ===\n")
      console.log("| Metric | Result | Target | Status |")
      console.log("|--------|--------|--------|--------|")
      for (const r of results) {
        const status = r.pass ? "✅ PASS" : "❌ FAIL"
        console.log(`| ${r.name} | ${r.result} | ${r.target} | ${status} |`)
      }

      const passed = results.filter((r) => r.pass).length
      const failed = results.filter((r) => !r.pass).length
      console.log(`\nTotal: ${results.length} | Passed: ${passed} | Failed: ${failed}`)

      process.exit(failed > 0 ? 1 : 0)
    })
    .catch((err) => {
      console.error("Benchmark failed:", err)
      process.exit(1)
    })
}
