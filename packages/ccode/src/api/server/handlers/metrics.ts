/**
 * Metrics Handler
 *
 * Provides /metrics endpoint for observability:
 * - Prometheus-compatible text format
 * - JSON format for dashboard consumption
 * - Request latency percentiles (p50/p95/p99)
 * - Error rates
 * - Memory usage
 */

import type { HttpRequest, HttpResponse, RouteHandler } from "../types"

// ============================================================================
// Types
// ============================================================================

interface HistogramSample {
  value: number
  timestamp: number
}

interface Counter {
  value: number
  labels: Record<string, string>
}

interface RequestMetric {
  method: string
  path: string
  status: number
  duration: number
  timestamp: number
}

// ============================================================================
// Metrics Storage (Module-level singleton)
// ============================================================================

const MAX_SAMPLES = 10000
const WINDOW_MS = 5 * 60 * 1000 // 5 minutes

class MetricsCollector {
  private startTime = Date.now()
  private requests: RequestMetric[] = []
  private errors: Map<string, number> = new Map()
  private activeConnections = 0

  recordRequest(method: string, path: string, status: number, durationMs: number): void {
    const now = Date.now()

    this.requests.push({
      method,
      path,
      status,
      duration: durationMs,
      timestamp: now,
    })

    // Prune old samples
    this.prune()

    // Record errors
    if (status >= 400) {
      const errorType = status >= 500 ? "server_error" : "client_error"
      this.errors.set(errorType, (this.errors.get(errorType) ?? 0) + 1)
    }
  }

  incConnections(): void {
    this.activeConnections++
  }

  decConnections(): void {
    this.activeConnections = Math.max(0, this.activeConnections - 1)
  }

  private prune(): void {
    const cutoff = Date.now() - WINDOW_MS
    this.requests = this.requests.filter((r) => r.timestamp > cutoff)

    // Also bound total samples
    if (this.requests.length > MAX_SAMPLES) {
      this.requests = this.requests.slice(-MAX_SAMPLES)
    }
  }

  private percentile(p: number): number {
    this.prune()
    if (this.requests.length === 0) return 0

    const durations = this.requests.map((r) => r.duration).sort((a, b) => a - b)
    const idx = Math.round((durations.length - 1) * p)
    return durations[Math.min(idx, durations.length - 1)]
  }

  getSnapshot(): MetricsSnapshot {
    this.prune()

    const totalRequests = this.requests.length
    const errorRequests = this.requests.filter((r) => r.status >= 400).length
    const errorRate = totalRequests > 0 ? (errorRequests / totalRequests) * 100 : 0

    return {
      service: "ccode-api",
      total_requests: totalRequests,
      error_requests: errorRequests,
      error_rate: errorRate,
      p50_ms: this.percentile(0.5),
      p95_ms: this.percentile(0.95),
      p99_ms: this.percentile(0.99),
      active_connections: this.activeConnections,
      memory_bytes: process.memoryUsage().heapUsed,
      uptime_secs: Math.floor((Date.now() - this.startTime) / 1000),
    }
  }

  renderPrometheus(): string {
    const snapshot = this.getSnapshot()
    let output = ""

    // HTTP requests by status
    output += "# TYPE http_requests_total counter\n"
    const byStatus = new Map<number, number>()
    for (const r of this.requests) {
      byStatus.set(r.status, (byStatus.get(r.status) ?? 0) + 1)
    }
    for (const [status, count] of byStatus) {
      output += `http_requests_total{service="ccode-api",status="${status}"} ${count}\n`
    }

    // Duration percentiles
    output += "\n# TYPE http_request_duration_ms histogram\n"
    output += `http_request_duration_ms_p50{service="ccode-api"} ${snapshot.p50_ms.toFixed(2)}\n`
    output += `http_request_duration_ms_p95{service="ccode-api"} ${snapshot.p95_ms.toFixed(2)}\n`
    output += `http_request_duration_ms_p99{service="ccode-api"} ${snapshot.p99_ms.toFixed(2)}\n`

    // Errors
    output += "\n# TYPE errors_total counter\n"
    for (const [type, count] of this.errors) {
      output += `errors_total{service="ccode-api",type="${type}"} ${count}\n`
    }

    // Active connections
    output += `\n# TYPE active_connections gauge\nactive_connections{service="ccode-api"} ${snapshot.active_connections}\n`

    // Process memory
    output += `\n# TYPE process_memory_bytes gauge\nprocess_memory_bytes{service="ccode-api"} ${snapshot.memory_bytes}\n`

    // Uptime
    output += `\n# TYPE process_start_time_seconds gauge\nprocess_start_time_seconds{service="ccode-api"} ${Math.floor(this.startTime / 1000)}\n`

    return output
  }
}

// Singleton instance
const collector = new MetricsCollector()

// ============================================================================
// Public API
// ============================================================================

export interface MetricsSnapshot {
  service: string
  total_requests: number
  error_requests: number
  error_rate: number
  p50_ms: number
  p95_ms: number
  p99_ms: number
  active_connections: number
  memory_bytes: number
  uptime_secs: number
}

/**
 * Record an HTTP request for metrics.
 */
export function recordRequest(method: string, path: string, status: number, durationMs: number): void {
  collector.recordRequest(method, path, status, durationMs)
}

/**
 * Increment active connection count.
 */
export function incConnections(): void {
  collector.incConnections()
}

/**
 * Decrement active connection count.
 */
export function decConnections(): void {
  collector.decConnections()
}

/**
 * Get current metrics snapshot.
 */
export function getSnapshot(): MetricsSnapshot {
  return collector.getSnapshot()
}

/**
 * Render metrics in Prometheus text format.
 */
export function renderPrometheus(): string {
  return collector.renderPrometheus()
}

// ============================================================================
// Route Handlers
// ============================================================================

/**
 * GET /metrics - Prometheus text format
 */
export const metricsHandler: RouteHandler = async (_req: HttpRequest): Promise<HttpResponse> => ({
  status: 200,
  headers: { "Content-Type": "text/plain; charset=utf-8" },
  body: collector.renderPrometheus(),
})

/**
 * GET /api/v1/metrics - JSON format
 */
export const metricsJsonHandler: RouteHandler = async (_req: HttpRequest): Promise<HttpResponse> => ({
  status: 200,
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(collector.getSnapshot()),
})
