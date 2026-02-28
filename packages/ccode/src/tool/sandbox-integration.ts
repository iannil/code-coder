/**
 * Sandbox Tool Execution Integration
 *
 * Provides configuration and utilities for sandboxed tool execution.
 * Integrates with existing SandboxExecutor for secure code execution.
 *
 * @package tool
 */

import { Log } from "@/util/log"
import {
  createSandboxExecutor,
  createAutoSandboxExecutor,
  type SandboxExecutor,
  type SandboxBackend,
  type SandboxResult,
  type ResourceLimits,
} from "@/autonomous/execution/sandbox"

const log = Log.create({ service: "tool.sandbox-integration" })

// ============================================================================
// Types
// ============================================================================

/**
 * File system access levels
 */
export type FileSystemAccess = "none" | "readonly" | "restricted" | "full"

/**
 * Sandbox policy for a tool
 */
export interface ToolSandboxPolicy {
  /** Sandbox backend to use */
  backend: SandboxBackend

  /** Resource limits */
  limits: {
    /** Maximum memory in MB */
    memoryMB: number

    /** Maximum CPU time in ms */
    cpuTimeMs: number

    /** Allow network access */
    networkAccess: boolean

    /** File system access level */
    fileSystemAccess: FileSystemAccess
  }

  /** Reason for this policy */
  reason: string
}

/**
 * Tool execution configuration
 */
export interface ToolExecutionConfig {
  /** Default sandbox backend */
  defaultBackend: SandboxBackend

  /** Default resource limits */
  defaultLimits: ResourceLimits

  /** Per-tool sandbox policies */
  policies: Record<string, ToolSandboxPolicy>

  /** Tools that should never be sandboxed */
  bypassTools: string[]

  /** Enable sandbox execution globally */
  enabled: boolean
}

/**
 * Sandboxed execution result
 */
export interface SandboxedToolResult {
  /** Whether execution was sandboxed */
  sandboxed: boolean

  /** Sandbox backend used */
  backend?: SandboxBackend

  /** Execution result */
  result: SandboxResult

  /** Whether rollback is available */
  canRollback: boolean

  /** Policy that was applied */
  policy?: ToolSandboxPolicy
}

// ============================================================================
// Default Configuration
// ============================================================================

/**
 * Default sandbox policies for tools
 *
 * Policy rationale:
 * - Read-only tools (Read, Glob, Grep, LS): No sandbox needed
 * - Network tools (WebFetch, WebSearch): WASM sandbox for isolation
 * - File modification (Write, Edit): Docker sandbox for filesystem isolation
 * - Shell execution (Bash): Docker sandbox for full isolation
 * - Subprocess tools (Task): Docker sandbox for process isolation
 */
const DEFAULT_POLICIES: Record<string, ToolSandboxPolicy> = {
  // Safe tools - no sandbox
  Read: {
    backend: "process",
    limits: { memoryMB: 256, cpuTimeMs: 30000, networkAccess: false, fileSystemAccess: "readonly" },
    reason: "Read-only file access, low risk",
  },
  Glob: {
    backend: "process",
    limits: { memoryMB: 128, cpuTimeMs: 10000, networkAccess: false, fileSystemAccess: "readonly" },
    reason: "File pattern matching, low risk",
  },
  Grep: {
    backend: "process",
    limits: { memoryMB: 256, cpuTimeMs: 30000, networkAccess: false, fileSystemAccess: "readonly" },
    reason: "Content search, low risk",
  },
  LS: {
    backend: "process",
    limits: { memoryMB: 64, cpuTimeMs: 5000, networkAccess: false, fileSystemAccess: "readonly" },
    reason: "Directory listing, low risk",
  },

  // Network tools - WASM sandbox
  WebFetch: {
    backend: "wasm",
    limits: { memoryMB: 128, cpuTimeMs: 60000, networkAccess: true, fileSystemAccess: "none" },
    reason: "Network access requires isolation",
  },
  WebSearch: {
    backend: "wasm",
    limits: { memoryMB: 128, cpuTimeMs: 60000, networkAccess: true, fileSystemAccess: "none" },
    reason: "Network access requires isolation",
  },

  // File modification tools - Docker sandbox
  Write: {
    backend: "docker",
    limits: { memoryMB: 256, cpuTimeMs: 30000, networkAccess: false, fileSystemAccess: "restricted" },
    reason: "File creation/modification requires isolation",
  },
  Edit: {
    backend: "docker",
    limits: { memoryMB: 256, cpuTimeMs: 30000, networkAccess: false, fileSystemAccess: "restricted" },
    reason: "File modification requires isolation",
  },
  NotebookEdit: {
    backend: "docker",
    limits: { memoryMB: 256, cpuTimeMs: 30000, networkAccess: false, fileSystemAccess: "restricted" },
    reason: "Notebook modification requires isolation",
  },

  // Shell execution - Docker sandbox (highest isolation)
  Bash: {
    backend: "docker",
    limits: { memoryMB: 512, cpuTimeMs: 120000, networkAccess: false, fileSystemAccess: "restricted" },
    reason: "Arbitrary command execution requires full isolation",
  },

  // Subprocess tools - Docker sandbox
  Task: {
    backend: "docker",
    limits: { memoryMB: 1024, cpuTimeMs: 600000, networkAccess: false, fileSystemAccess: "restricted" },
    reason: "Subprocess execution requires isolation",
  },
}

/**
 * Tools that bypass sandbox entirely
 * These are internal/safe tools that don't need sandboxing
 */
const BYPASS_TOOLS = [
  "TodoRead",
  "TodoWrite",
  "AskUserQuestion",
  "EnterPlanMode",
  "ExitPlanMode",
  "Skill",
  "TaskCreate",
  "TaskGet",
  "TaskUpdate",
  "TaskList",
]

/**
 * Default execution configuration
 */
const DEFAULT_CONFIG: ToolExecutionConfig = {
  defaultBackend: "auto",
  defaultLimits: {
    maxMemoryMb: 256,
    maxTimeMs: 30000,
    allowNetwork: false,
    allowFileWrite: false,
  },
  policies: DEFAULT_POLICIES,
  bypassTools: BYPASS_TOOLS,
  enabled: false, // Disabled by default, opt-in
}

// ============================================================================
// Sandbox Integration Manager
// ============================================================================

/**
 * Sandbox Integration Manager
 *
 * Manages sandbox execution for tools based on configured policies.
 */
export class SandboxIntegrationManager {
  private config: ToolExecutionConfig
  private executor: SandboxExecutor | null = null
  private initialized = false

  constructor(config: Partial<ToolExecutionConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Initialize the sandbox manager
   */
  async initialize(): Promise<void> {
    if (this.initialized) return

    if (this.config.enabled) {
      this.executor = await createAutoSandboxExecutor()
      log.info("Sandbox integration initialized", {
        backend: this.executor.getBackend(),
        dockerAvailable: this.executor.isDockerAvailable(),
        wasmAvailable: this.executor.isWasmAvailable(),
      })
    }

    this.initialized = true
  }

  /**
   * Get sandbox policy for a tool
   */
  getPolicy(toolName: string): ToolSandboxPolicy | null {
    if (this.config.bypassTools.includes(toolName)) {
      return null
    }

    return this.config.policies[toolName] ?? null
  }

  /**
   * Check if a tool should be sandboxed
   */
  shouldSandbox(toolName: string): boolean {
    if (!this.config.enabled) return false
    if (this.config.bypassTools.includes(toolName)) return false

    return this.config.policies[toolName] !== undefined
  }

  /**
   * Get recommended sandbox backend for a tool
   */
  getRecommendedBackend(toolName: string): SandboxBackend {
    const policy = this.getPolicy(toolName)
    return policy?.backend ?? this.config.defaultBackend
  }

  /**
   * Execute code in sandbox (for Bash tool)
   *
   * @param code Code to execute
   * @param toolName Tool requesting execution
   * @returns Sandbox execution result
   */
  async executeSandboxed(
    code: string,
    toolName: string,
    language: "shell" | "python" | "nodejs" = "shell",
  ): Promise<SandboxedToolResult> {
    await this.ensureInitialized()

    const policy = this.getPolicy(toolName)

    // If no policy or sandbox disabled, return non-sandboxed result
    if (!this.config.enabled || !policy || !this.executor) {
      return {
        sandboxed: false,
        result: {
          exitCode: 0,
          stdout: "",
          stderr: "Sandbox not enabled",
          durationMs: 0,
          timedOut: false,
        },
        canRollback: false,
      }
    }

    log.debug("Executing sandboxed", {
      tool: toolName,
      backend: policy.backend,
      language,
    })

    const result = await this.executor.execute({
      language,
      code,
      timeoutMs: policy.limits.cpuTimeMs,
      limits: {
        maxMemoryMb: policy.limits.memoryMB,
        maxTimeMs: policy.limits.cpuTimeMs,
        allowNetwork: policy.limits.networkAccess,
        allowFileWrite: policy.limits.fileSystemAccess !== "none" && policy.limits.fileSystemAccess !== "readonly",
      },
    })

    return {
      sandboxed: true,
      backend: policy.backend,
      result,
      canRollback: policy.backend === "docker",
      policy,
    }
  }

  /**
   * Update configuration
   */
  updateConfig(config: Partial<ToolExecutionConfig>): void {
    this.config = { ...this.config, ...config }
  }

  /**
   * Enable sandbox execution
   */
  async enable(): Promise<void> {
    this.config.enabled = true
    if (!this.executor) {
      this.executor = await createAutoSandboxExecutor()
    }
  }

  /**
   * Disable sandbox execution
   */
  disable(): void {
    this.config.enabled = false
  }

  /**
   * Check if sandbox is enabled
   */
  isEnabled(): boolean {
    return this.config.enabled
  }

  /**
   * Get current configuration
   */
  getConfig(): Readonly<ToolExecutionConfig> {
    return this.config
  }

  /**
   * Cleanup resources
   */
  async cleanup(): Promise<void> {
    if (this.executor) {
      await this.executor.cleanup()
      this.executor = null
    }
    this.initialized = false
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async ensureInitialized(): Promise<void> {
    if (!this.initialized) {
      await this.initialize()
    }
  }
}

// ============================================================================
// Singleton and Convenience Functions
// ============================================================================

let managerInstance: SandboxIntegrationManager | null = null

/**
 * Get the global sandbox integration manager
 */
export function getSandboxManager(): SandboxIntegrationManager {
  if (!managerInstance) {
    managerInstance = new SandboxIntegrationManager()
  }
  return managerInstance
}

/**
 * Create a new sandbox integration manager
 */
export function createSandboxManager(config?: Partial<ToolExecutionConfig>): SandboxIntegrationManager {
  return new SandboxIntegrationManager(config)
}

/**
 * Check if a tool should be sandboxed (convenience function)
 */
export function shouldSandboxTool(toolName: string): boolean {
  return getSandboxManager().shouldSandbox(toolName)
}

/**
 * Get sandbox policy for a tool (convenience function)
 */
export function getToolSandboxPolicy(toolName: string): ToolSandboxPolicy | null {
  return getSandboxManager().getPolicy(toolName)
}

/**
 * Execute code in sandbox (convenience function)
 */
export async function executeSandboxed(
  code: string,
  toolName: string,
  language?: "shell" | "python" | "nodejs",
): Promise<SandboxedToolResult> {
  return getSandboxManager().executeSandboxed(code, toolName, language)
}
