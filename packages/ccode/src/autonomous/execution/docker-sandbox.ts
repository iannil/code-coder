/**
 * Docker Sandbox Executor
 *
 * Provides secure code execution using Docker containers for production-grade isolation.
 * Features:
 * - Network isolation
 * - Resource limits (CPU, memory, time)
 * - Read-only filesystem with tmpfs for temp files
 * - No privileged access
 *
 * Part of Phase 2: Strengthen Sandbox Isolation
 */

import { spawn, type ChildProcess } from "child_process"
import path from "path"
import fs from "fs/promises"
import os from "os"

// ============================================================================
// Types
// ============================================================================

/** Docker sandbox configuration */
export interface DockerSandboxConfig {
  /** Docker image to use (default: based on language) */
  image?: string
  /** CPU limit (e.g., "0.5" for 50% of one CPU) */
  cpuLimit?: string
  /** Memory limit (e.g., "256m") */
  memoryLimit?: string
  /** Maximum execution time in seconds */
  timeoutSecs?: number
  /** Allow network access */
  allowNetwork?: boolean
  /** Additional environment variables */
  env?: Record<string, string>
  /** Mount points (host:container) */
  mounts?: string[]
  /** Working directory inside container */
  workDir?: string
}

/** Supported languages for Docker execution */
export type DockerLanguage = "python" | "nodejs" | "shell" | "rust" | "go"

/** Docker execution request */
export interface DockerExecutionRequest {
  /** Code language */
  language: DockerLanguage
  /** Code to execute */
  code: string
  /** Additional configuration */
  config?: DockerSandboxConfig
}

/** Docker execution result */
export interface DockerExecutionResult {
  /** Exit code (0 = success) */
  exitCode: number
  /** Standard output */
  stdout: string
  /** Standard error */
  stderr: string
  /** Execution duration in milliseconds */
  durationMs: number
  /** Whether execution was killed due to timeout */
  timedOut: boolean
  /** Container ID (for debugging) */
  containerId?: string
  /** Any error that occurred */
  error?: string
}

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_IMAGES: Record<DockerLanguage, string> = {
  python: "python:3.12-slim",
  nodejs: "node:20-alpine",
  shell: "alpine:3.19",
  rust: "rust:1.75-slim",
  go: "golang:1.22-alpine",
}

const LANGUAGE_CONFIG: Record<DockerLanguage, { command: string; fileExt: string; args: string[] }> = {
  python: { command: "python3", fileExt: ".py", args: [] },
  nodejs: { command: "node", fileExt: ".js", args: [] },
  shell: { command: "sh", fileExt: ".sh", args: [] },
  rust: { command: "rustc", fileExt: ".rs", args: ["--edition", "2021", "-o", "/tmp/a.out"] },
  go: { command: "go", fileExt: ".go", args: ["run"] },
}

const DEFAULT_CONFIG: Required<DockerSandboxConfig> = {
  image: "",
  cpuLimit: "0.5",
  memoryLimit: "256m",
  timeoutSecs: 30,
  allowNetwork: false,
  env: {},
  mounts: [],
  workDir: "/sandbox",
}

// ============================================================================
// Docker Sandbox Executor
// ============================================================================

/**
 * Docker-based secure code execution sandbox
 *
 * Uses Docker containers for process and filesystem isolation.
 * Each execution runs in a fresh container that is automatically removed.
 */
export class DockerSandboxExecutor {
  private tempDir: string
  private executionCount = 0
  private dockerAvailable: boolean | null = null

  constructor() {
    this.tempDir = path.join(os.tmpdir(), "codecoder-docker-sandbox")
  }

  /**
   * Initialize the sandbox and check Docker availability
   */
  async initialize(): Promise<void> {
    await fs.mkdir(this.tempDir, { recursive: true })
    this.dockerAvailable = await this.checkDockerAvailable()

    if (!this.dockerAvailable) {
      console.warn("Docker not available. DockerSandboxExecutor will not work.")
    }
  }

  /**
   * Check if Docker is available and running
   */
  async checkDockerAvailable(): Promise<boolean> {
    try {
      const result = await this.runCommand("docker", ["version", "--format", "{{.Server.Version}}"])
      return result.exitCode === 0
    } catch {
      return false
    }
  }

  /**
   * Execute code in a Docker container
   */
  async execute(request: DockerExecutionRequest): Promise<DockerExecutionResult> {
    if (!this.dockerAvailable) {
      return {
        exitCode: 1,
        stdout: "",
        stderr: "Docker is not available on this system",
        durationMs: 0,
        timedOut: false,
        error: "Docker not available",
      }
    }

    const startTime = Date.now()
    const config = { ...DEFAULT_CONFIG, ...request.config }
    const langConfig = LANGUAGE_CONFIG[request.language]

    // Determine Docker image
    const image = config.image || DEFAULT_IMAGES[request.language]

    // Create temporary file with code
    const fileId = `sandbox_${Date.now()}_${++this.executionCount}`
    const fileName = `${fileId}${langConfig.fileExt}`
    const hostFilePath = path.join(this.tempDir, fileName)
    const containerFilePath = path.join(config.workDir, fileName)

    try {
      // Write code to temp file
      await fs.writeFile(hostFilePath, request.code, "utf-8")

      // Build Docker run command
      const dockerArgs = this.buildDockerArgs({
        image,
        config,
        hostFilePath,
        containerFilePath,
        langConfig,
        request,
      })

      // Execute with timeout
      const result = await this.runDockerWithTimeout(dockerArgs, config.timeoutSecs * 1000)

      return {
        ...result,
        durationMs: Date.now() - startTime,
      }
    } finally {
      // Cleanup temp file
      await fs.unlink(hostFilePath).catch(() => {})
    }
  }

  /**
   * Build Docker run arguments
   */
  private buildDockerArgs(params: {
    image: string
    config: Required<DockerSandboxConfig>
    hostFilePath: string
    containerFilePath: string
    langConfig: { command: string; args: string[] }
    request: DockerExecutionRequest
  }): string[] {
    const { image, config, hostFilePath, containerFilePath, langConfig, request } = params

    const args: string[] = [
      "run",
      "--rm", // Remove container after execution
      "--init", // Use init process
      "--read-only", // Read-only filesystem
      "--tmpfs", "/tmp:rw,noexec,nosuid,size=64m", // Temp directory
      "--tmpfs", `${config.workDir}:rw,noexec,nosuid,size=64m`, // Work directory
      "--cpu-quota", String(Math.floor(parseFloat(config.cpuLimit) * 100000)),
      "--memory", config.memoryLimit,
      "--memory-swap", config.memoryLimit, // No swap
      "--pids-limit", "50", // Limit number of processes
      "--ulimit", "nofile=256:256", // Limit open files
      "--cap-drop", "ALL", // Drop all capabilities
      "--security-opt", "no-new-privileges", // Prevent privilege escalation
    ]

    // Network isolation
    if (!config.allowNetwork) {
      args.push("--network", "none")
    }

    // Mount the code file
    args.push("-v", `${hostFilePath}:${containerFilePath}:ro`)

    // Additional mounts
    for (const mount of config.mounts) {
      args.push("-v", mount)
    }

    // Environment variables
    for (const [key, value] of Object.entries(config.env)) {
      args.push("-e", `${key}=${value}`)
    }

    // Working directory
    args.push("-w", config.workDir)

    // Image
    args.push(image)

    // Command
    args.push(langConfig.command)
    args.push(...langConfig.args)

    // Special handling for Rust (compile then run)
    if (request.language === "rust") {
      // For Rust, we need to compile and run
      args.push(containerFilePath)
      args.push("&&", "/tmp/a.out")
    } else {
      args.push(containerFilePath)
    }

    return args
  }

  /**
   * Run Docker with timeout
   */
  private async runDockerWithTimeout(
    args: string[],
    timeoutMs: number,
  ): Promise<Omit<DockerExecutionResult, "durationMs">> {
    return new Promise((resolve) => {
      let stdout = ""
      let stderr = ""
      let timedOut = false
      let containerId: string | undefined

      const proc = spawn("docker", args, {
        stdio: ["pipe", "pipe", "pipe"],
      })

      // Timeout handler
      const timeout = setTimeout(() => {
        timedOut = true
        proc.kill("SIGKILL")
      }, timeoutMs)

      proc.stdout.on("data", (data) => {
        stdout += data.toString()
        // Extract container ID from output if available
        if (!containerId && stdout.length === 64) {
          containerId = stdout.trim()
        }
        // Limit output size
        if (stdout.length > 1000000) {
          proc.kill("SIGKILL")
        }
      })

      proc.stderr.on("data", (data) => {
        stderr += data.toString()
        if (stderr.length > 1000000) {
          proc.kill("SIGKILL")
        }
      })

      proc.on("close", (code) => {
        clearTimeout(timeout)
        resolve({
          exitCode: code ?? 1,
          stdout: stdout.slice(0, 100000),
          stderr: stderr.slice(0, 100000),
          timedOut,
          containerId,
        })
      })

      proc.on("error", (error) => {
        clearTimeout(timeout)
        resolve({
          exitCode: 1,
          stdout,
          stderr: error.message,
          timedOut: false,
          error: error.message,
        })
      })
    })
  }

  /**
   * Run a simple command and return result
   */
  private async runCommand(command: string, args: string[]): Promise<{ exitCode: number; stdout: string }> {
    return new Promise((resolve) => {
      let stdout = ""

      const proc = spawn(command, args, {
        stdio: ["pipe", "pipe", "pipe"],
      })

      proc.stdout.on("data", (data) => {
        stdout += data.toString()
      })

      proc.on("close", (code) => {
        resolve({ exitCode: code ?? 1, stdout: stdout.trim() })
      })

      proc.on("error", () => {
        resolve({ exitCode: 1, stdout: "" })
      })
    })
  }

  /**
   * Pull a Docker image if not present
   */
  async pullImage(image: string): Promise<boolean> {
    try {
      const result = await this.runCommand("docker", ["pull", image])
      return result.exitCode === 0
    } catch {
      return false
    }
  }

  /**
   * List available images for sandbox
   */
  async listAvailableImages(): Promise<string[]> {
    try {
      const result = await this.runCommand("docker", [
        "images",
        "--format",
        "{{.Repository}}:{{.Tag}}",
      ])

      if (result.exitCode !== 0) return []

      const images = result.stdout.split("\n").filter(Boolean)
      const sandboxImages = Object.values(DEFAULT_IMAGES)

      return images.filter((img) => sandboxImages.some((si) => img.startsWith(si.split(":")[0])))
    } catch {
      return []
    }
  }

  /**
   * Cleanup sandbox resources
   */
  async cleanup(): Promise<void> {
    try {
      await fs.rm(this.tempDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  }

  /**
   * Check if Docker is available
   */
  isAvailable(): boolean {
    return this.dockerAvailable === true
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create a Docker sandbox executor instance
 */
export async function createDockerSandboxExecutor(): Promise<DockerSandboxExecutor> {
  const executor = new DockerSandboxExecutor()
  await executor.initialize()
  return executor
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Validate code for obvious dangerous patterns before Docker execution
 *
 * This is a defense-in-depth measure. Docker isolation is the primary security layer.
 */
export function validateCodeForDocker(code: string, language: DockerLanguage): { safe: boolean; reason: string } {
  // These patterns are blocked even in Docker for defense in depth
  const universalBlocked = [
    { pattern: /rm\s+-rf\s+\/\s*$/, reason: "Recursive root deletion" },
    { pattern: /dd\s+if=.*of=\/dev/, reason: "Direct device write" },
    { pattern: /:(){ :|:& };:/, reason: "Fork bomb" },
  ]

  for (const { pattern, reason } of universalBlocked) {
    if (pattern.test(code)) {
      return { safe: false, reason }
    }
  }

  return { safe: true, reason: "" }
}
