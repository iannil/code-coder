/**
 * Tool Integration Test Helper Utilities
 *
 * Provides mock implementations and utilities for testing Tool components.
 * This includes:
 * - Tool context mocks
 * - File system mock extensions
 * - Tool result validation utilities
 * - Tool chain testing
 */

import { vi, type Mock } from "bun:test"
import z from "zod"
import type { Tool } from "../../src/tool/tool"
import type { MessageV2 } from "../../src/session/message-v2"
import { mkdtempSync, writeFileSync, mkdirSync, existsSync, readFileSync, rmSync } from "fs"
import { join } from "path"
import { tmpdir } from "os"

// ===== Tool Context Mocks =====

/**
 * Create a mock Tool.Context
 */
export function createMockToolContext(overrides: Partial<MockToolContextOptions> = {}): Tool.Context {
  const options: MockToolContextOptions = {
    sessionID: `session-${Date.now()}`,
    messageID: `msg-${Date.now()}`,
    agent: "build",
    callID: `call-${Date.now()}`,
    ...overrides,
  }

  const abortController = new AbortController()
  const metadataCalls: Array<{ title?: string; metadata?: unknown }> = []
  const askCalls: Array<unknown> = []

  const ctx: Tool.Context & MockToolContextExtras = {
    sessionID: options.sessionID,
    messageID: options.messageID,
    agent: options.agent,
    callID: options.callID,
    abort: abortController.signal,
    extra: options.extra,
    metadata: vi.fn((input) => {
      metadataCalls.push(input)
    }),
    ask: vi.fn(async (input) => {
      askCalls.push(input)
    }),
    // Extra testing utilities
    _metadataCalls: metadataCalls,
    _askCalls: askCalls,
    _abortController: abortController,
  }

  return ctx
}

export interface MockToolContextOptions {
  sessionID: string
  messageID: string
  agent: string
  callID?: string
  extra?: Record<string, unknown>
}

export interface MockToolContextExtras {
  _metadataCalls: Array<{ title?: string; metadata?: unknown }>
  _askCalls: Array<unknown>
  _abortController: AbortController
}

/**
 * Trigger abort on a mock context
 */
export function abortToolContext(ctx: Tool.Context): void {
  const extras = ctx as Tool.Context & MockToolContextExtras
  extras._abortController.abort()
}

/**
 * Get metadata calls from a mock context
 */
export function getMetadataCalls(ctx: Tool.Context): Array<{ title?: string; metadata?: unknown }> {
  const extras = ctx as Tool.Context & MockToolContextExtras
  return extras._metadataCalls
}

/**
 * Get ask calls from a mock context
 */
export function getAskCalls(ctx: Tool.Context): Array<unknown> {
  const extras = ctx as Tool.Context & MockToolContextExtras
  return extras._askCalls
}

// ===== File System Mock Extensions =====

/**
 * Create a temporary test directory with files
 */
export interface TestDirectory {
  path: string
  files: Map<string, string>
  writeFile: (relativePath: string, content: string) => string
  readFile: (relativePath: string) => string
  exists: (relativePath: string) => boolean
  mkdir: (relativePath: string) => string
  cleanup: () => void
}

export function createTestDirectory(prefix = "tool-test"): TestDirectory {
  const dir = mkdtempSync(join(tmpdir(), `${prefix}-`))
  const files = new Map<string, string>()

  return {
    path: dir,
    files,

    writeFile(relativePath: string, content: string): string {
      const fullPath = join(dir, relativePath)
      const parentDir = join(fullPath, "..")
      if (!existsSync(parentDir)) {
        mkdirSync(parentDir, { recursive: true })
      }
      writeFileSync(fullPath, content)
      files.set(relativePath, content)
      return fullPath
    },

    readFile(relativePath: string): string {
      const fullPath = join(dir, relativePath)
      return readFileSync(fullPath, "utf-8")
    },

    exists(relativePath: string): boolean {
      return existsSync(join(dir, relativePath))
    },

    mkdir(relativePath: string): string {
      const fullPath = join(dir, relativePath)
      mkdirSync(fullPath, { recursive: true })
      return fullPath
    },

    cleanup(): void {
      rmSync(dir, { recursive: true, force: true })
    },
  }
}

/**
 * Create a test directory with a standard project structure
 */
export function createTestProjectDirectory(): TestDirectory {
  const dir = createTestDirectory("project-test")

  // Create standard project structure
  dir.writeFile("package.json", JSON.stringify({
    name: "test-project",
    version: "1.0.0",
  }, null, 2))

  dir.writeFile("src/index.ts", `export function main() {
  console.log("Hello, world!")
}`)

  dir.writeFile("src/utils.ts", `export function add(a: number, b: number): number {
  return a + b
}

export function multiply(a: number, b: number): number {
  return a * b
}`)

  dir.writeFile("test/index.test.ts", `import { describe, it, expect } from "bun:test"
import { main } from "../src/index"

describe("main", () => {
  it("should run", () => {
    expect(main).toBeDefined()
  })
})`)

  dir.mkdir(".git")
  dir.writeFile(".gitignore", "node_modules/\ndist/\n.env")

  return dir
}

// ===== Tool Result Validation =====

/**
 * Tool execution result
 */
export interface ToolResult<M = unknown> {
  title: string
  metadata: M
  output: string
  attachments?: MessageV2.FilePart[]
}

/**
 * Assert tool result matches expected values
 */
export function assertToolResult<M>(
  result: ToolResult<M>,
  expected: Partial<ToolResult<M>>,
): void {
  if (expected.title !== undefined && result.title !== expected.title) {
    throw new Error(`Expected title "${expected.title}", got "${result.title}"`)
  }

  if (expected.output !== undefined && !result.output.includes(expected.output)) {
    throw new Error(`Expected output to contain "${expected.output}", got "${result.output}"`)
  }

  if (expected.attachments !== undefined) {
    if (result.attachments?.length !== expected.attachments.length) {
      throw new Error(
        `Expected ${expected.attachments.length} attachments, got ${result.attachments?.length ?? 0}`,
      )
    }
  }
}

/**
 * Assert tool output contains specific text
 */
export function assertOutputContains(result: ToolResult, text: string): void {
  if (!result.output.includes(text)) {
    throw new Error(`Expected output to contain "${text}"`)
  }
}

/**
 * Assert tool output does NOT contain specific text
 */
export function assertOutputNotContains(result: ToolResult, text: string): void {
  if (result.output.includes(text)) {
    throw new Error(`Expected output to NOT contain "${text}"`)
  }
}

/**
 * Assert tool result has metadata
 */
export function assertHasMetadata<M>(
  result: ToolResult<M>,
  key: keyof M,
): void {
  if (!(key in (result.metadata as object))) {
    throw new Error(`Expected metadata to have key "${String(key)}"`)
  }
}

// ===== Tool Chain Testing =====

/**
 * Tool chain step definition
 */
export interface ToolChainStep<Args = unknown, M = unknown> {
  tool: string
  args: Args
  expectedResult?: Partial<ToolResult<M>>
  onResult?: (result: ToolResult<M>) => void
}

/**
 * Tool chain executor for testing sequences
 */
export class ToolChainExecutor {
  private tools: Map<string, Tool.Info>
  private results: Array<{ step: ToolChainStep; result: ToolResult }>
  private testDir: TestDirectory

  constructor(testDir: TestDirectory) {
    this.tools = new Map()
    this.results = []
    this.testDir = testDir
  }

  /**
   * Register a tool for the chain
   */
  registerTool(tool: Tool.Info): void {
    this.tools.set(tool.id, tool)
  }

  /**
   * Execute a chain of tool calls
   */
  async execute(
    steps: ToolChainStep[],
    ctx: Tool.Context = createMockToolContext(),
  ): Promise<ToolResult[]> {
    const results: ToolResult[] = []

    for (const step of steps) {
      const tool = this.tools.get(step.tool)
      if (!tool) {
        throw new Error(`Unknown tool: ${step.tool}`)
      }

      const toolInfo = await tool.init()
      const result = await toolInfo.execute(step.args as z.infer<typeof toolInfo.parameters>, ctx)

      results.push(result)
      this.results.push({ step, result })

      if (step.expectedResult) {
        assertToolResult(result, step.expectedResult)
      }

      if (step.onResult) {
        step.onResult(result)
      }
    }

    return results
  }

  /**
   * Get all results
   */
  getResults(): Array<{ step: ToolChainStep; result: ToolResult }> {
    return [...this.results]
  }

  /**
   * Clear results
   */
  clearResults(): void {
    this.results = []
  }

  /**
   * Get the test directory
   */
  getTestDir(): TestDirectory {
    return this.testDir
  }
}

// ===== Mock Tool Factory =====

/**
 * Create a mock tool for testing
 */
export function createMockTool<P extends z.ZodType, M>(
  id: string,
  options: MockToolOptions<P, M>,
): Tool.Info<P, M> {
  const executeImpl = options.execute ?? (async () => ({
    title: `Mock ${id} result`,
    metadata: {} as M,
    output: `Mock output from ${id}`,
  }))

  return {
    id,
    init: async () => ({
      description: options.description ?? `Mock tool: ${id}`,
      parameters: options.parameters,
      execute: executeImpl,
    }),
  }
}

export interface MockToolOptions<P extends z.ZodType, M> {
  description?: string
  parameters: P
  execute?: (args: z.infer<P>, ctx: Tool.Context) => Promise<ToolResult<M>>
}

/**
 * Create mock versions of standard tools
 */
export function createMockStandardTools(testDir: TestDirectory): Record<string, Tool.Info> {
  return {
    Read: createMockTool("Read", {
      parameters: z.object({
        file_path: z.string(),
        offset: z.number().optional(),
        limit: z.number().optional(),
      }),
      execute: async (args) => {
        const relativePath = args.file_path.startsWith(testDir.path)
          ? args.file_path.slice(testDir.path.length + 1)
          : args.file_path

        if (!testDir.exists(relativePath)) {
          return {
            title: `Error reading ${args.file_path}`,
            metadata: { error: true },
            output: `File not found: ${args.file_path}`,
          }
        }

        const content = testDir.readFile(relativePath)
        const lines = content.split("\n")
        const start = args.offset ?? 0
        const end = args.limit ? start + args.limit : lines.length
        const selectedLines = lines.slice(start, end)

        return {
          title: `Read ${args.file_path}`,
          metadata: { lineCount: selectedLines.length },
          output: selectedLines.map((line, i) => `${start + i + 1}→${line}`).join("\n"),
        }
      },
    }),

    Glob: createMockTool("Glob", {
      parameters: z.object({
        pattern: z.string(),
        path: z.string().optional(),
      }),
      execute: async (args) => {
        // Simple mock - return files from test directory
        const files: string[] = []
        testDir.files.forEach((_, key) => {
          if (key.includes(args.pattern.replace("**/*", "").replace("*", ""))) {
            files.push(join(testDir.path, key))
          }
        })

        return {
          title: `Glob ${args.pattern}`,
          metadata: { count: files.length },
          output: files.length > 0 ? files.join("\n") : "No files found",
        }
      },
    }),

    Grep: createMockTool("Grep", {
      parameters: z.object({
        pattern: z.string(),
        path: z.string().optional(),
        glob: z.string().optional(),
      }),
      execute: async (args) => {
        const matches: string[] = []
        testDir.files.forEach((content, key) => {
          const lines = content.split("\n")
          lines.forEach((line, i) => {
            if (line.includes(args.pattern)) {
              matches.push(`${join(testDir.path, key)}:${i + 1}:${line}`)
            }
          })
        })

        return {
          title: `Grep ${args.pattern}`,
          metadata: { matchCount: matches.length },
          output: matches.length > 0 ? matches.join("\n") : "No matches found",
        }
      },
    }),

    Write: createMockTool("Write", {
      parameters: z.object({
        file_path: z.string(),
        content: z.string(),
      }),
      execute: async (args) => {
        const relativePath = args.file_path.startsWith(testDir.path)
          ? args.file_path.slice(testDir.path.length + 1)
          : args.file_path

        testDir.writeFile(relativePath, args.content)

        return {
          title: `Wrote ${args.file_path}`,
          metadata: { bytes: args.content.length },
          output: `Successfully wrote ${args.content.length} bytes to ${args.file_path}`,
        }
      },
    }),

    Edit: createMockTool("Edit", {
      parameters: z.object({
        file_path: z.string(),
        old_string: z.string(),
        new_string: z.string(),
        replace_all: z.boolean().optional(),
      }),
      execute: async (args) => {
        const relativePath = args.file_path.startsWith(testDir.path)
          ? args.file_path.slice(testDir.path.length + 1)
          : args.file_path

        if (!testDir.exists(relativePath)) {
          return {
            title: `Error editing ${args.file_path}`,
            metadata: { error: true },
            output: `File not found: ${args.file_path}`,
          }
        }

        let content = testDir.readFile(relativePath)
        const replaceCount = args.replace_all
          ? (content.match(new RegExp(args.old_string, "g")) ?? []).length
          : content.includes(args.old_string) ? 1 : 0

        if (replaceCount === 0) {
          return {
            title: `No changes to ${args.file_path}`,
            metadata: { changes: 0 },
            output: `String not found: "${args.old_string}"`,
          }
        }

        content = args.replace_all
          ? content.replaceAll(args.old_string, args.new_string)
          : content.replace(args.old_string, args.new_string)

        testDir.writeFile(relativePath, content)

        return {
          title: `Edited ${args.file_path}`,
          metadata: { changes: replaceCount },
          output: `Made ${replaceCount} replacement(s) in ${args.file_path}`,
        }
      },
    }),

    Bash: createMockTool("Bash", {
      parameters: z.object({
        command: z.string(),
        description: z.string().optional(),
        timeout: z.number().optional(),
      }),
      execute: async (args) => {
        // Mock bash - just return command echo for safety
        return {
          title: `Executed command`,
          metadata: { exitCode: 0 },
          output: `[Mock] Would execute: ${args.command}`,
        }
      },
    }),
  }
}

// ===== Permission Testing =====

/**
 * Mock permission ask result
 */
export interface MockPermissionResult {
  allowed: boolean
  reason?: string
}

/**
 * Create a mock permission handler
 */
export function createMockPermissionHandler(
  defaultResult: MockPermissionResult = { allowed: true },
): {
  handler: Tool.Context["ask"]
  calls: Array<unknown>
  setResult: (result: MockPermissionResult) => void
} {
  const calls: Array<unknown> = []
  let currentResult = defaultResult

  return {
    handler: vi.fn(async (input) => {
      calls.push(input)
      if (!currentResult.allowed) {
        throw new Error(currentResult.reason ?? "Permission denied")
      }
    }),
    calls,
    setResult: (result) => {
      currentResult = result
    },
  }
}

// ===== Hook Testing =====

/**
 * Mock hook result
 */
export interface MockHookResult {
  blocked: boolean
  hookName?: string
  message?: string
}

/**
 * Create a mock hook for testing
 */
export function createMockHook(
  lifecycle: "PreToolUse" | "PostToolUse",
  result: MockHookResult = { blocked: false },
): {
  calls: Array<unknown>
  setResult: (result: MockHookResult) => void
  reset: () => void
} {
  const calls: Array<unknown> = []
  let currentResult = result

  return {
    calls,
    setResult: (newResult) => {
      currentResult = newResult
    },
    reset: () => {
      calls.length = 0
      currentResult = { blocked: false }
    },
  }
}

// ===== Export all =====
export {
  createMockToolContext,
  abortToolContext,
  getMetadataCalls,
  getAskCalls,
  createTestDirectory,
  createTestProjectDirectory,
  assertToolResult,
  assertOutputContains,
  assertOutputNotContains,
  assertHasMetadata,
  ToolChainExecutor,
  createMockTool,
  createMockStandardTools,
  createMockPermissionHandler,
  createMockHook,
}
