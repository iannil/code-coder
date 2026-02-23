/**
 * Sandbox Code Executor
 *
 * Provides secure code execution for the autonomous evolution loop.
 * Supports Python, Node.js, and Shell with resource limits and safety constraints.
 *
 * Execution Backends:
 * - Process: Direct process execution with validation (development/testing)
 * - Docker: Full container isolation (production)
 *
 * Part of Phase 3: Autonomous Problem-Solving Loop
 */

import { spawn } from "child_process"
import path from "path"
import fs from "fs/promises"
import os from "os"
import {
  type DockerSandboxExecutor,
  createDockerSandboxExecutor,
  type DockerExecutionRequest,
  type DockerLanguage,
} from "./docker-sandbox"
import {
  type WasmSandboxExecutor,
  createWasmSandboxExecutor,
  type WasmLanguage,
  recommendSandboxBackend,
} from "./wasm-sandbox"

// ============================================================================
// Types
// ============================================================================

/** Supported execution languages */
export type SandboxLanguage = "python" | "nodejs" | "shell"

/** Execution backend type */
export type SandboxBackend = "process" | "docker" | "wasm" | "auto"

/** Sandbox execution request */
export interface SandboxRequest {
  /** Code language */
  language: SandboxLanguage
  /** Code to execute */
  code: string
  /** Timeout in milliseconds */
  timeoutMs?: number
  /** Working directory */
  workingDir?: string
  /** Environment variables */
  env?: Record<string, string>
  /** Resource limits */
  limits?: ResourceLimits
}

/** Resource limits for execution */
export interface ResourceLimits {
  /** Maximum memory in MB */
  maxMemoryMb?: number
  /** Maximum execution time in ms */
  maxTimeMs?: number
  /** Allow network access */
  allowNetwork?: boolean
  /** Allow file system writes */
  allowFileWrite?: boolean
}

/** Sandbox execution result */
export interface SandboxResult {
  /** Exit code (0 = success) */
  exitCode: number
  /** Standard output */
  stdout: string
  /** Standard error */
  stderr: string
  /** Execution duration in ms */
  durationMs: number
  /** Whether execution was killed due to timeout */
  timedOut: boolean
  /** Any error that occurred */
  error?: string
}

/** Reflection result after analyzing execution */
export interface ReflectionResult {
  /** Whether the execution succeeded */
  success: boolean
  /** Analysis of the output/error */
  analysis: string
  /** Suggested corrections if failed */
  suggestedFix?: string
  /** Should retry with correction */
  shouldRetry: boolean
  /** Confidence in the analysis (0-1) */
  confidence: number
}

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_LIMITS: Required<ResourceLimits> = {
  maxMemoryMb: 256,
  maxTimeMs: 30000,
  allowNetwork: false,
  allowFileWrite: false,
}

const LANGUAGE_CONFIG: Record<SandboxLanguage, { command: string; fileExt: string; args: string[] }> = {
  python: { command: "python3", fileExt: ".py", args: [] },
  nodejs: { command: "bun", fileExt: ".js", args: ["run"] }, // Use Bun for Node.js
  shell: { command: "bash", fileExt: ".sh", args: [] },
}

// ============================================================================
// Sandbox Executor
// ============================================================================

/**
 * Secure code execution sandbox for autonomous problem solving
 *
 * Supports three backends:
 * - Process: Direct execution with code validation (fast, less isolated)
 * - Docker: Container-based execution (slower, production-grade isolation)
 * - WASM: WebAssembly-based execution (fastest, JavaScript only)
 * - Auto: Automatically selects best backend based on code characteristics
 */
export class SandboxExecutor {
  private tempDir: string
  private executionCount = 0
  private backend: SandboxBackend
  private dockerExecutor: DockerSandboxExecutor | null = null
  private wasmExecutor: WasmSandboxExecutor | null = null

  constructor(backend: SandboxBackend = "process") {
    this.tempDir = path.join(os.tmpdir(), "codecoder-sandbox")
    this.backend = backend
  }

  /**
   * Initialize the sandbox environment
   */
  async initialize(): Promise<void> {
    await fs.mkdir(this.tempDir, { recursive: true })

    // Initialize Docker executor if using Docker or auto backend
    if (this.backend === "docker" || this.backend === "auto") {
      this.dockerExecutor = await createDockerSandboxExecutor()
      if (!this.dockerExecutor.isAvailable() && this.backend === "docker") {
        console.warn("Docker not available, falling back to process backend")
        this.backend = "process"
        this.dockerExecutor = null
      }
    }

    // Initialize WASM executor if using WASM or auto backend
    if (this.backend === "wasm" || this.backend === "auto") {
      this.wasmExecutor = await createWasmSandboxExecutor()
      if (!this.wasmExecutor.isAvailable() && this.backend === "wasm") {
        console.warn("WASM sandbox not available, falling back to process backend")
        this.backend = "process"
        this.wasmExecutor = null
      }
    }
  }

  /**
   * Get the current execution backend
   */
  getBackend(): SandboxBackend {
    return this.backend
  }

  /**
   * Check if Docker backend is available
   */
  isDockerAvailable(): boolean {
    return this.dockerExecutor?.isAvailable() ?? false
  }

  /**
   * Check if WASM backend is available
   */
  isWasmAvailable(): boolean {
    return this.wasmExecutor?.isAvailable() ?? false
  }

  /**
   * Execute code in a sandboxed environment
   *
   * Backend selection:
   * - "docker": Uses Docker container
   * - "wasm": Uses WASM sandbox (JavaScript only)
   * - "process": Direct process execution with validation
   * - "auto": Automatically selects best backend based on code
   */
  async execute(request: SandboxRequest): Promise<SandboxResult> {
    // Auto backend: select based on code characteristics
    if (this.backend === "auto") {
      const recommended = recommendSandboxBackend(request.code, request.language)

      if (recommended === "wasm" && this.wasmExecutor?.isAvailable()) {
        return this.executeWithWasm(request)
      }

      if (recommended === "docker" && this.dockerExecutor?.isAvailable()) {
        return this.executeWithDocker(request)
      }

      return this.executeWithProcess(request)
    }

    // WASM backend
    if (this.backend === "wasm" && this.wasmExecutor) {
      return this.executeWithWasm(request)
    }

    // Docker backend
    if (this.backend === "docker" && this.dockerExecutor) {
      return this.executeWithDocker(request)
    }

    return this.executeWithProcess(request)
  }

  /**
   * Execute code using WASM sandbox (JavaScript only)
   */
  private async executeWithWasm(request: SandboxRequest): Promise<SandboxResult> {
    const startTime = Date.now()

    if (!this.wasmExecutor) {
      return {
        exitCode: 1,
        stdout: "",
        stderr: "WASM executor not initialized",
        durationMs: 0,
        timedOut: false,
        error: "WASM executor not initialized",
      }
    }

    // Only JavaScript is supported in WASM
    if (request.language !== "nodejs") {
      // Fall back to process execution for non-JS
      return this.executeWithProcess(request)
    }

    const result = await this.wasmExecutor.execute({
      language: "javascript" as WasmLanguage,
      code: request.code,
      config: {
        maxMemoryBytes: (request.limits?.maxMemoryMb ?? 256) * 1024 * 1024,
        maxTimeMs: request.limits?.maxTimeMs ?? 30000,
      },
    })

    return {
      exitCode: result.exitCode,
      stdout: result.stdout,
      stderr: result.stderr,
      durationMs: Date.now() - startTime,
      timedOut: result.timedOut,
      error: result.error,
    }
  }

  /**
   * Execute code using Docker container isolation
   */
  private async executeWithDocker(request: SandboxRequest): Promise<SandboxResult> {
    const startTime = Date.now()

    if (!this.dockerExecutor) {
      return {
        exitCode: 1,
        stdout: "",
        stderr: "Docker executor not initialized",
        durationMs: 0,
        timedOut: false,
        error: "Docker executor not initialized",
      }
    }

    // Map language to Docker language
    const dockerLanguage: DockerLanguage = request.language === "nodejs" ? "nodejs" : request.language

    const dockerRequest: DockerExecutionRequest = {
      language: dockerLanguage,
      code: request.code,
      config: {
        memoryLimit: `${request.limits?.maxMemoryMb ?? 256}m`,
        timeoutSecs: Math.floor((request.limits?.maxTimeMs ?? 30000) / 1000),
        allowNetwork: request.limits?.allowNetwork ?? false,
        env: request.env,
      },
    }

    const result = await this.dockerExecutor.execute(dockerRequest)

    return {
      exitCode: result.exitCode,
      stdout: result.stdout,
      stderr: result.stderr,
      durationMs: Date.now() - startTime,
      timedOut: result.timedOut,
      error: result.error,
    }
  }

  /**
   * Execute code using process-level isolation (original implementation)
   */
  private async executeWithProcess(request: SandboxRequest): Promise<SandboxResult> {
    const startTime = Date.now()
    const limits = { ...DEFAULT_LIMITS, ...request.limits }

    // Validate code for dangerous patterns
    const validation = this.validateCode(request.code, request.language)
    if (!validation.safe) {
      return {
        exitCode: 1,
        stdout: "",
        stderr: validation.reason,
        durationMs: 0,
        timedOut: false,
        error: `Security validation failed: ${validation.reason}`,
      }
    }

    // Create temporary file
    const fileId = `sandbox_${Date.now()}_${++this.executionCount}`
    const config = LANGUAGE_CONFIG[request.language]
    const filePath = path.join(this.tempDir, `${fileId}${config.fileExt}`)

    try {
      // Write code to temp file
      await fs.writeFile(filePath, request.code, "utf-8")

      // Build environment
      const env: Record<string, string> = {
        ...process.env,
        ...request.env,
        PYTHONDONTWRITEBYTECODE: "1",
        NODE_OPTIONS: `--max-old-space-size=${limits.maxMemoryMb}`,
      }

      // Remove sensitive env vars
      delete env.ANTHROPIC_API_KEY
      delete env.OPENAI_API_KEY
      delete env.AWS_SECRET_ACCESS_KEY
      delete env.GITHUB_TOKEN

      // Execute
      const result = await this.spawnProcess({
        command: config.command,
        args: [...config.args, filePath],
        cwd: request.workingDir ?? this.tempDir,
        env,
        timeoutMs: limits.maxTimeMs,
      })

      return {
        ...result,
        durationMs: Date.now() - startTime,
      }
    } finally {
      // Cleanup temp file
      await fs.unlink(filePath).catch(() => {})
    }
  }

  /**
   * Execute code with automatic reflection and retry
   */
  async executeWithReflection(
    request: SandboxRequest,
    maxRetries = 3,
    onReflection?: (result: SandboxResult, reflection: ReflectionResult, attempt: number) => void,
  ): Promise<{ finalResult: SandboxResult; attempts: number; reflections: ReflectionResult[] }> {
    let currentCode = request.code
    const reflections: ReflectionResult[] = []
    let attempts = 0

    while (attempts < maxRetries) {
      attempts++

      // Execute
      const result = await this.execute({ ...request, code: currentCode })

      // Reflect on result
      const reflection = this.reflect(result, currentCode, request.language)
      reflections.push(reflection)

      // Notify callback
      onReflection?.(result, reflection, attempts)

      // Success or no retry needed
      if (reflection.success || !reflection.shouldRetry || !reflection.suggestedFix) {
        return { finalResult: result, attempts, reflections }
      }

      // Apply fix and retry
      currentCode = reflection.suggestedFix
    }

    // Max retries reached
    const finalResult = await this.execute({ ...request, code: currentCode })
    return { finalResult, attempts, reflections }
  }

  /**
   * Validate code for dangerous patterns
   */
  private validateCode(code: string, language: SandboxLanguage): { safe: boolean; reason: string } {
    const dangerousPatterns: Record<SandboxLanguage, Array<{ pattern: RegExp; reason: string }>> = {
      python: [
        { pattern: /import\s+subprocess/, reason: "subprocess import not allowed" },
        { pattern: /import\s+os\s*$|from\s+os\s+import/, reason: "os module restricted" },
        { pattern: /open\s*\([^)]*['"][wa]['"]/, reason: "file write operations not allowed" },
        { pattern: /eval\s*\(|exec\s*\(/, reason: "eval/exec not allowed" },
        { pattern: /__import__/, reason: "dynamic imports not allowed" },
        { pattern: /socket\./, reason: "socket operations not allowed" },
      ],
      nodejs: [
        { pattern: /child_process/, reason: "child_process not allowed" },
        { pattern: /require\s*\(\s*['"]fs['"]/, reason: "fs module restricted" },
        { pattern: /require\s*\(\s*['"]net['"]/, reason: "net module not allowed" },
        { pattern: /process\.env/, reason: "process.env access not allowed" },
        { pattern: /eval\s*\(/, reason: "eval not allowed" },
        { pattern: /new\s+Function\s*\(/, reason: "Function constructor not allowed" },
      ],
      shell: [
        { pattern: /rm\s+-rf\s+\//, reason: "destructive rm command not allowed" },
        { pattern: /mkfs|dd\s+if=/, reason: "disk operations not allowed" },
        { pattern: /curl|wget/, reason: "network downloads not allowed" },
        { pattern: />>\s*\/|>\s*\//, reason: "writing to system paths not allowed" },
        { pattern: /chmod\s+[0-7]*7/, reason: "world-writable permissions not allowed" },
        { pattern: /\$\(|`/, reason: "command substitution not allowed" },
      ],
    }

    const patterns = dangerousPatterns[language] ?? []

    for (const { pattern, reason } of patterns) {
      if (pattern.test(code)) {
        return { safe: false, reason }
      }
    }

    return { safe: true, reason: "" }
  }

  /**
   * Reflect on execution result and suggest fixes
   */
  private reflect(result: SandboxResult, code: string, language: SandboxLanguage): ReflectionResult {
    // Success case
    if (result.exitCode === 0 && !result.error) {
      return {
        success: true,
        analysis: "Execution completed successfully",
        shouldRetry: false,
        confidence: 1.0,
      }
    }

    // Timeout
    if (result.timedOut) {
      return {
        success: false,
        analysis: "Execution timed out - code may have infinite loop or is too slow",
        suggestedFix: this.suggestTimeoutFix(code, language),
        shouldRetry: true,
        confidence: 0.5,
      }
    }

    // Analyze error
    const errorAnalysis = this.analyzeError(result.stderr, code, language)

    return {
      success: false,
      analysis: errorAnalysis.analysis,
      suggestedFix: errorAnalysis.fix,
      shouldRetry: errorAnalysis.fix !== undefined,
      confidence: errorAnalysis.confidence,
    }
  }

  /**
   * Analyze error and suggest fix
   */
  private analyzeError(
    stderr: string,
    code: string,
    language: SandboxLanguage,
  ): { analysis: string; fix?: string; confidence: number } {
    const lowerStderr = stderr.toLowerCase()

    // Common error patterns
    if (language === "python") {
      if (lowerStderr.includes("modulenotfounderror") || lowerStderr.includes("no module named")) {
        const moduleMatch = stderr.match(/No module named ['"]?(\w+)['"]?/i)
        const moduleName = moduleMatch?.[1]
        return {
          analysis: `Missing Python module: ${moduleName}`,
          fix: moduleName ? `# Install: pip install ${moduleName}\n${code}` : undefined,
          confidence: 0.8,
        }
      }

      if (lowerStderr.includes("syntaxerror")) {
        const lineMatch = stderr.match(/line (\d+)/i)
        return {
          analysis: `Syntax error${lineMatch ? ` at line ${lineMatch[1]}` : ""}`,
          confidence: 0.6,
        }
      }

      if (lowerStderr.includes("indentationerror")) {
        return {
          analysis: "Indentation error - check whitespace consistency",
          fix: code.replace(/\t/g, "    "), // Convert tabs to spaces
          confidence: 0.7,
        }
      }
    }

    if (language === "nodejs") {
      if (lowerStderr.includes("cannot find module")) {
        const moduleMatch = stderr.match(/Cannot find module ['"]([^'"]+)['"]/i)
        const moduleName = moduleMatch?.[1]
        return {
          analysis: `Missing Node module: ${moduleName}`,
          fix: moduleName ? `// Install: bun add ${moduleName}\n${code}` : undefined,
          confidence: 0.8,
        }
      }

      if (lowerStderr.includes("syntaxerror") || lowerStderr.includes("unexpected token")) {
        return {
          analysis: "JavaScript syntax error",
          confidence: 0.6,
        }
      }
    }

    if (language === "shell") {
      if (lowerStderr.includes("command not found")) {
        const cmdMatch = stderr.match(/(\w+):\s*command not found/i)
        return {
          analysis: `Command not found: ${cmdMatch?.[1] ?? "unknown"}`,
          confidence: 0.9,
        }
      }

      if (lowerStderr.includes("permission denied")) {
        return {
          analysis: "Permission denied - operation not allowed in sandbox",
          confidence: 0.9,
        }
      }
    }

    // Generic error
    return {
      analysis: `Execution failed: ${stderr.slice(0, 200)}`,
      confidence: 0.3,
    }
  }

  /**
   * Suggest fix for timeout
   */
  private suggestTimeoutFix(code: string, _language: SandboxLanguage): string | undefined {
    // Check for obvious infinite loops
    if (code.includes("while True") || code.includes("while (true)") || code.includes("for (;;)")) {
      return undefined // Can't automatically fix infinite loops
    }

    // Add timeout handling for Python
    if (_language === "python" && !code.includes("signal.alarm")) {
      return `import signal\n\ndef timeout_handler(signum, frame):\n    raise TimeoutError("Execution timeout")\n\nsignal.signal(signal.SIGALRM, timeout_handler)\nsignal.alarm(25)  # 25 second timeout\n\n${code}\n\nsignal.alarm(0)  # Cancel alarm`
    }

    return undefined
  }

  /**
   * Spawn a process with timeout
   */
  private async spawnProcess(options: {
    command: string
    args: string[]
    cwd: string
    env: Record<string, string>
    timeoutMs: number
  }): Promise<Omit<SandboxResult, "durationMs">> {
    return new Promise((resolve) => {
      let stdout = ""
      let stderr = ""
      let timedOut = false

      const proc = spawn(options.command, options.args, {
        cwd: options.cwd,
        env: options.env as NodeJS.ProcessEnv,
        stdio: ["pipe", "pipe", "pipe"],
      })

      // Timeout handler
      const timeout = setTimeout(() => {
        timedOut = true
        proc.kill("SIGKILL")
      }, options.timeoutMs)

      proc.stdout.on("data", (data) => {
        stdout += data.toString()
        if (stdout.length > 1000000) {
          // 1MB limit
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
          stdout: stdout.slice(0, 100000), // Limit output size
          stderr: stderr.slice(0, 100000),
          timedOut,
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
   * Cleanup sandbox directory
   */
  async cleanup(): Promise<void> {
    try {
      await fs.rm(this.tempDir, { recursive: true, force: true })
    } catch {
      // Ignore cleanup errors
    }
  }
}

// ============================================================================
// Factory
// ============================================================================

/**
 * Create a sandbox executor instance
 *
 * @param backend - Execution backend:
 *   - "process": Direct execution with validation (development)
 *   - "docker": Container isolation (production)
 *   - "wasm": WebAssembly sandbox (fastest, JS only)
 *   - "auto": Automatically select best backend
 */
export async function createSandboxExecutor(backend: SandboxBackend = "process"): Promise<SandboxExecutor> {
  const executor = new SandboxExecutor(backend)
  await executor.initialize()
  return executor
}

/**
 * Create a sandbox executor with Docker backend if available, otherwise process
 */
export async function createSandboxExecutorWithFallback(): Promise<SandboxExecutor> {
  // Try Docker first
  const dockerExecutor = new SandboxExecutor("docker")
  await dockerExecutor.initialize()

  if (dockerExecutor.isDockerAvailable()) {
    return dockerExecutor
  }

  // Fall back to process
  const processExecutor = new SandboxExecutor("process")
  await processExecutor.initialize()
  return processExecutor
}

/**
 * Create a sandbox executor with automatic backend selection.
 *
 * Uses WASM for simple JavaScript, Docker for complex scripts,
 * and process as fallback.
 */
export async function createAutoSandboxExecutor(): Promise<SandboxExecutor> {
  const executor = new SandboxExecutor("auto")
  await executor.initialize()
  return executor
}

/**
 * Create a WASM sandbox executor for fast JavaScript execution.
 *
 * Falls back to process execution if WASM is not available.
 */
export async function createWasmSandboxExecutorWithFallback(): Promise<SandboxExecutor> {
  // Try WASM first
  const wasmExecutor = new SandboxExecutor("wasm")
  await wasmExecutor.initialize()

  if (wasmExecutor.isWasmAvailable()) {
    return wasmExecutor
  }

  // Fall back to process
  console.warn("WASM sandbox not available, using process execution")
  const processExecutor = new SandboxExecutor("process")
  await processExecutor.initialize()
  return processExecutor
}
