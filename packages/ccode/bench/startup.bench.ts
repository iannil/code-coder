/**
 * Startup Time Benchmarks
 *
 * Validates NFR-04 requirement: ZeroBot startup time ≤ 0.5s
 *
 * This measures cold-start time for TypeScript services (CodeCoder)
 * and Rust services (Gateway, Channels, Workflow).
 */

import { spawn } from "child_process"
import path from "path"
import type { BenchmarkResult } from "./index"

const STARTUP_TARGET_MS = 500 // 0.5 seconds per NFR-04
const PROJECT_ROOT = path.resolve(import.meta.dir, "../../..")

interface StartupMeasurement {
  name: string
  durationMs: number
  success: boolean
  error?: string
}

/**
 * Measure cold-start time for a Rust service by executing it and
 * waiting for the startup log message.
 */
async function measureRustServiceStartup(
  serviceName: string,
  binaryPath: string,
  startupPattern: RegExp,
  timeoutMs = 10000,
): Promise<StartupMeasurement> {
  return new Promise((resolve) => {
    const startTime = performance.now()
    let output = ""
    let resolved = false

    const cleanup = (result: StartupMeasurement) => {
      if (resolved) return
      resolved = true
      proc.kill("SIGTERM")
      resolve(result)
    }

    const proc = spawn(binaryPath, [], {
      cwd: PROJECT_ROOT,
      env: {
        ...process.env,
        RUST_LOG: "info",
        // Use test config to avoid real connections
        CODECODER_CONFIG_PATH: path.join(PROJECT_ROOT, "config/test.json"),
      },
      stdio: ["ignore", "pipe", "pipe"],
    })

    const timeout = setTimeout(() => {
      cleanup({
        name: serviceName,
        durationMs: timeoutMs,
        success: false,
        error: `Timeout after ${timeoutMs}ms`,
      })
    }, timeoutMs)

    const handleOutput = (data: Buffer) => {
      output += data.toString()

      // Look for startup complete message
      if (startupPattern.test(output)) {
        const durationMs = performance.now() - startTime
        clearTimeout(timeout)
        cleanup({
          name: serviceName,
          durationMs,
          success: true,
        })
      }
    }

    proc.stdout.on("data", handleOutput)
    proc.stderr.on("data", handleOutput)

    proc.on("error", (err) => {
      clearTimeout(timeout)
      cleanup({
        name: serviceName,
        durationMs: performance.now() - startTime,
        success: false,
        error: err.message,
      })
    })

    proc.on("exit", (code) => {
      if (!resolved) {
        clearTimeout(timeout)
        cleanup({
          name: serviceName,
          durationMs: performance.now() - startTime,
          success: false,
          error: `Process exited with code ${code}`,
        })
      }
    })
  })
}

/**
 * Measure cold-start time for TypeScript service by importing the module
 * and measuring time to first initialization.
 */
async function measureTypeScriptStartup(): Promise<StartupMeasurement> {
  const startTime = performance.now()

  try {
    // Dynamically import to measure cold-start
    const { Instance } = await import("../src/project/instance")
    const { Config } = await import("../src/config/config")

    // Simulate minimal initialization
    const tmpDir = path.join(PROJECT_ROOT, "packages/ccode")
    await Instance.provide({
      directory: tmpDir,
      fn: async () => {
        await Config.get()
      },
    })

    const durationMs = performance.now() - startTime
    return {
      name: "TypeScript Core",
      durationMs,
      success: true,
    }
  } catch (error) {
    return {
      name: "TypeScript Core",
      durationMs: performance.now() - startTime,
      success: false,
      error: error instanceof Error ? error.message : String(error),
    }
  }
}

/**
 * Check if Rust binaries are available
 */
async function checkRustBinaries(): Promise<{ gateway: string | null; channels: string | null; workflow: string | null }> {
  const releaseDir = path.join(PROJECT_ROOT, "release/rust")
  const debugDir = path.join(PROJECT_ROOT, "target/release")

  const binaries = {
    gateway: null as string | null,
    channels: null as string | null,
    workflow: null as string | null,
  }

  for (const [name, key] of [
    ["zero-gateway", "gateway"],
    ["zero-channels", "channels"],
    ["zero-workflow", "workflow"],
  ] as const) {
    // Check release directory first
    let binPath = path.join(releaseDir, name)
    if (await Bun.file(binPath).exists()) {
      binaries[key] = binPath
      continue
    }

    // Check target/release
    binPath = path.join(debugDir, name)
    if (await Bun.file(binPath).exists()) {
      binaries[key] = binPath
    }
  }

  return binaries
}

export async function runStartupBenchmarks(): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  // Measure TypeScript startup
  console.log("  Measuring TypeScript core startup...")
  const tsResult = await measureTypeScriptStartup()
  results.push({
    name: "TS Core Startup",
    target: `≤${STARTUP_TARGET_MS}ms`,
    result: `${tsResult.durationMs.toFixed(0)}ms`,
    pass: tsResult.success && tsResult.durationMs <= STARTUP_TARGET_MS,
    details: tsResult.error ? { error: tsResult.error } : undefined,
  })

  // Check for Rust binaries
  const binaries = await checkRustBinaries()

  // Measure Rust services if available
  const rustServices = [
    { name: "Gateway", key: "gateway", pattern: /Service started|listening|Zero Gateway/ },
    { name: "Channels", key: "channels", pattern: /Service started|listening|Zero Channels/ },
    { name: "Workflow", key: "workflow", pattern: /Service started|listening|Zero Workflow/ },
  ] as const

  for (const service of rustServices) {
    const binPath = binaries[service.key]

    if (binPath) {
      console.log(`  Measuring ${service.name} startup...`)
      const measurement = await measureRustServiceStartup(service.name, binPath, service.pattern)

      results.push({
        name: `${service.name} Startup`,
        target: `≤${STARTUP_TARGET_MS}ms`,
        result: measurement.success ? `${measurement.durationMs.toFixed(0)}ms` : "N/A",
        pass: measurement.success && measurement.durationMs <= STARTUP_TARGET_MS,
        details: measurement.error ? { error: measurement.error } : undefined,
      })
    } else {
      console.log(`  Skipping ${service.name} (binary not found)`)
      results.push({
        name: `${service.name} Startup`,
        target: `≤${STARTUP_TARGET_MS}ms`,
        result: "Not Built",
        pass: true, // Don't fail if binary not available
        details: { note: "Binary not found, skipped" },
      })
    }
  }

  return results
}
