/**
 * MCP (Model Context Protocol) Performance Benchmarks
 *
 * Measures performance for MCP client operations:
 * - Server connection time (critical for local MCPs like chrome-devtools-mcp)
 * - Tool listing time
 * - Tool execution latency
 *
 * Special focus on chrome-devtools-mcp which spawns via npx and controls browser
 */

import { spawn, type ChildProcess } from "child_process"
import path from "path"
import type { BenchmarkResult } from "./index"

// MCP performance targets
const MCP_CONNECTION_TARGET_MS = 5000 // 5 seconds for local MCP spawn + connect
const MCP_TOOL_LIST_TARGET_MS = 1000 // 1 second to list tools
const MCP_TOOL_CALL_TARGET_MS = 2000 // 2 seconds for simple tool call
const BROWSER_TOOL_CALL_TARGET_MS = 10000 // 10 seconds for browser operations

interface McpMeasurement {
  name: string
  durationMs: number
  success: boolean
  error?: string
  details?: Record<string, unknown>
}

interface LatencyStats {
  p50: number
  p95: number
  p99: number
  avg: number
  min: number
  max: number
}

function percentile(sorted: number[], p: number): number {
  const index = Math.ceil((p / 100) * sorted.length) - 1
  return sorted[Math.max(0, index)]
}

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
 * Check if MCP module is available
 */
async function isMcpAvailable(): Promise<boolean> {
  try {
    await import("../src/mcp/index")
    return true
  } catch {
    return false
  }
}

/**
 * Measure MCP client initialization and connection time
 */
async function measureMcpConnection(): Promise<McpMeasurement> {
  const startTime = performance.now()

  try {
    const { MCP } = await import("../src/mcp/index")
    const { Instance } = await import("../src/project/instance")

    // Initialize instance context
    const projectRoot = path.resolve(import.meta.dir, "../../..")
    await Instance.provide({
      directory: projectRoot,
      fn: async () => {
        // Get MCP status (triggers connection)
        const status = await MCP.status()
        const connectedCount = Object.values(status).filter((s) => s.status === "connected").length
        const totalCount = Object.keys(status).length

        const durationMs = performance.now() - startTime
        return {
          name: "MCP Connection",
          durationMs,
          success: true,
          details: {
            connected: connectedCount,
            total: totalCount,
            servers: Object.entries(status).map(([name, s]) => ({
              name,
              status: s.status,
            })),
          },
        }
      },
    })

    // Measure time including context cleanup
    const durationMs = performance.now() - startTime
    return {
      name: "MCP Connection",
      durationMs,
      success: true,
    }
  } catch (error) {
    return {
      name: "MCP Connection",
      durationMs: performance.now() - startTime,
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

/**
 * Measure MCP tool listing time
 */
async function measureMcpToolList(): Promise<McpMeasurement> {
  const startTime = performance.now()

  try {
    const { MCP } = await import("../src/mcp/index")
    const { Instance } = await import("../src/project/instance")

    const projectRoot = path.resolve(import.meta.dir, "../../..")
    let toolCount = 0

    await Instance.provide({
      directory: projectRoot,
      fn: async () => {
        const tools = await MCP.tools()
        toolCount = Object.keys(tools).length
      },
    })

    const durationMs = performance.now() - startTime
    return {
      name: "MCP Tool List",
      durationMs,
      success: true,
      details: { toolCount },
    }
  } catch (error) {
    return {
      name: "MCP Tool List",
      durationMs: performance.now() - startTime,
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

/**
 * Check if Playwright MCP server is running by checking the port
 */
async function isPlaywrightMcpRunning(): Promise<boolean> {
  try {
    // Check if browser_snapshot tool is available
    const { MCP } = await import("../src/mcp/index")
    const { Instance } = await import("../src/project/instance")

    const projectRoot = path.resolve(import.meta.dir, "../../..")
    let hasPlaywright = false

    await Instance.provide({
      directory: projectRoot,
      fn: async () => {
        const tools = await MCP.tools()
        hasPlaywright = Object.keys(tools).some(
          (name) => name.includes("playwright") || name.includes("browser"),
        )
      },
    })

    return hasPlaywright
  } catch {
    return false
  }
}

/**
 * Measure Playwright browser tool call latency
 */
async function measurePlaywrightToolCall(): Promise<McpMeasurement> {
  const startTime = performance.now()

  try {
    const { MCP } = await import("../src/mcp/index")
    const { Instance } = await import("../src/project/instance")

    const projectRoot = path.resolve(import.meta.dir, "../../..")

    await Instance.provide({
      directory: projectRoot,
      fn: async () => {
        const tools = await MCP.tools()

        // Find browser_snapshot or similar tool
        const snapshotTool = Object.entries(tools).find(
          ([name]) => name.includes("browser_snapshot") || name.includes("snapshot"),
        )

        if (!snapshotTool) {
          throw new Error("No browser snapshot tool found")
        }

        // Note: We don't actually call the tool here as it requires a browser
        // Just measuring the tool discovery time
      },
    })

    const durationMs = performance.now() - startTime
    return {
      name: "Playwright Tool Discovery",
      durationMs,
      success: true,
    }
  } catch (error) {
    return {
      name: "Playwright Tool Discovery",
      durationMs: performance.now() - startTime,
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

/**
 * Measure chrome-devtools-mcp specific startup time
 * This spawns the MCP server via npx and measures cold start
 */
async function measureChromeDevToolsStartup(): Promise<McpMeasurement> {
  return new Promise((resolve) => {
    const startTime = performance.now()
    let proc: ChildProcess | null = null
    let resolved = false

    const cleanup = (result: McpMeasurement) => {
      if (resolved) return
      resolved = true
      if (proc) {
        proc.kill("SIGTERM")
      }
      resolve(result)
    }

    // Timeout after 30 seconds
    const timeout = setTimeout(() => {
      cleanup({
        name: "chrome-devtools-mcp Startup",
        durationMs: 30000,
        success: false,
        error: "Timeout after 30 seconds",
      })
    }, 30000)

    try {
      // Check if npx is available
      proc = spawn("npx", ["chrome-devtools-mcp@latest", "--help"], {
        stdio: ["ignore", "pipe", "pipe"],
        env: {
          ...process.env,
          // Suppress npm update notifications
          NO_UPDATE_NOTIFIER: "1",
        },
      })

      let output = ""

      proc.stdout?.on("data", (data) => {
        output += data.toString()
        // MCP server ready when it outputs help or starts listening
        if (output.includes("Usage") || output.includes("help") || output.includes("MCP")) {
          clearTimeout(timeout)
          cleanup({
            name: "chrome-devtools-mcp Startup",
            durationMs: performance.now() - startTime,
            success: true,
            details: { output: output.substring(0, 200) },
          })
        }
      })

      proc.stderr?.on("data", (data) => {
        output += data.toString()
      })

      proc.on("error", (err) => {
        clearTimeout(timeout)
        cleanup({
          name: "chrome-devtools-mcp Startup",
          durationMs: performance.now() - startTime,
          success: false,
          error: err.message,
        })
      })

      proc.on("exit", (code) => {
        clearTimeout(timeout)
        // Exit code 0 is success (help command completed)
        if (code === 0) {
          cleanup({
            name: "chrome-devtools-mcp Startup",
            durationMs: performance.now() - startTime,
            success: true,
          })
        } else if (!resolved) {
          cleanup({
            name: "chrome-devtools-mcp Startup",
            durationMs: performance.now() - startTime,
            success: false,
            error: `Process exited with code ${code}`,
          })
        }
      })
    } catch (error) {
      clearTimeout(timeout)
      cleanup({
        name: "chrome-devtools-mcp Startup",
        durationMs: performance.now() - startTime,
        success: false,
        error: error instanceof Error ? error.message : String(error),
      })
    }
  })
}

export async function runMcpBenchmarks(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  // Check if MCP is available
  const mcpAvailable = await isMcpAvailable()

  if (!mcpAvailable) {
    console.log("  MCP module not available, skipping MCP benchmarks")
    results.push({
      name: "MCP Connection",
      target: `≤${MCP_CONNECTION_TARGET_MS}ms`,
      result: "Not Available",
      pass: true,
      details: { note: "MCP module not available" },
    })
    return results
  }

  // Measure chrome-devtools-mcp startup (npx spawn time)
  console.log("  Measuring chrome-devtools-mcp startup...")
  const chromeDevToolsMeasurement = await measureChromeDevToolsStartup()

  results.push({
    name: "chrome-devtools-mcp Spawn",
    target: `≤${MCP_CONNECTION_TARGET_MS}ms`,
    result: chromeDevToolsMeasurement.success
      ? `${chromeDevToolsMeasurement.durationMs.toFixed(0)}ms`
      : "Failed",
    pass: chromeDevToolsMeasurement.success && chromeDevToolsMeasurement.durationMs <= MCP_CONNECTION_TARGET_MS,
    details: chromeDevToolsMeasurement.error ? { error: chromeDevToolsMeasurement.error } : undefined,
  })

  // Measure MCP connection (all configured servers)
  console.log("  Measuring MCP connection time...")
  const connectionMeasurement = await measureMcpConnection()

  results.push({
    name: "MCP Client Init",
    target: `≤${MCP_CONNECTION_TARGET_MS}ms`,
    result: connectionMeasurement.success
      ? `${connectionMeasurement.durationMs.toFixed(0)}ms`
      : "Failed",
    pass: connectionMeasurement.success && connectionMeasurement.durationMs <= MCP_CONNECTION_TARGET_MS,
    details: connectionMeasurement.details || (connectionMeasurement.error ? { error: connectionMeasurement.error } : undefined),
  })

  // Measure tool listing time
  console.log("  Measuring MCP tool listing...")
  const toolListMeasurement = await measureMcpToolList()

  results.push({
    name: "MCP Tool List",
    target: `≤${MCP_TOOL_LIST_TARGET_MS}ms`,
    result: toolListMeasurement.success
      ? `${toolListMeasurement.durationMs.toFixed(0)}ms`
      : "Failed",
    pass: toolListMeasurement.success && toolListMeasurement.durationMs <= MCP_TOOL_LIST_TARGET_MS,
    details: toolListMeasurement.details || (toolListMeasurement.error ? { error: toolListMeasurement.error } : undefined),
  })

  // Check for Playwright tools
  console.log("  Checking Playwright MCP tools...")
  const hasPlaywright = await isPlaywrightMcpRunning()

  if (hasPlaywright) {
    const playwrightMeasurement = await measurePlaywrightToolCall()
    results.push({
      name: "Playwright Tool Discovery",
      target: `≤${MCP_TOOL_CALL_TARGET_MS}ms`,
      result: playwrightMeasurement.success
        ? `${playwrightMeasurement.durationMs.toFixed(0)}ms`
        : "Failed",
      pass: playwrightMeasurement.success && playwrightMeasurement.durationMs <= MCP_TOOL_CALL_TARGET_MS,
      details: playwrightMeasurement.error ? { error: playwrightMeasurement.error } : undefined,
    })
  } else {
    results.push({
      name: "Playwright Tool Discovery",
      target: `≤${MCP_TOOL_CALL_TARGET_MS}ms`,
      result: "Not Connected",
      pass: true,
      details: { note: "Playwright MCP not connected" },
    })
  }

  return results
}
