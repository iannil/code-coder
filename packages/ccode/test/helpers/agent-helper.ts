/**
 * Agent Test Helper Utilities
 *
 * Provides mock implementations and utilities for testing Agent system components.
 * This includes:
 * - Agent factory mocks for creating test agents
 * - Agent execution context mocks
 * - Tool chain simulation utilities
 * - Permission testing helpers
 */

import { vi, type Mock } from "bun:test"
import type { Agent } from "../../src/agent/agent"
import { PermissionNext } from "../../src/permission/next"

// ===== Agent Factory Mocks =====

/**
 * Default permissions for test agents
 */
export function createDefaultPermissions(): PermissionNext.Ruleset {
  return PermissionNext.fromConfig({
    "*": "allow",
    doom_loop: "ask",
    question: "deny",
    plan_enter: "deny",
    plan_exit: "deny",
  })
}

/**
 * Create a mock Agent.Info with sensible defaults
 */
export function createMockAgent(overrides: Partial<Agent.Info> = {}): Agent.Info {
  return {
    name: "test-agent",
    description: "Test agent for unit testing",
    mode: "primary",
    native: true,
    hidden: false,
    topP: undefined,
    temperature: undefined,
    color: undefined,
    permission: createDefaultPermissions(),
    model: undefined,
    prompt: "You are a test agent.",
    options: {},
    steps: undefined,
    autoApprove: undefined,
    ...overrides,
  }
}

/**
 * Create a subagent configuration
 */
export function createMockSubagent(overrides: Partial<Agent.Info> = {}): Agent.Info {
  return createMockAgent({
    name: "test-subagent",
    description: "Test subagent",
    mode: "subagent",
    permission: PermissionNext.merge(
      createDefaultPermissions(),
      PermissionNext.fromConfig({
        "*": "deny",
        grep: "allow",
        glob: "allow",
        read: "allow",
      }),
    ),
    autoApprove: {
      enabled: true,
      allowedTools: ["Read", "Glob", "Grep"],
      riskThreshold: "safe",
    },
    ...overrides,
  })
}

/**
 * Create a primary agent configuration with planning permissions
 */
export function createMockPrimaryAgent(overrides: Partial<Agent.Info> = {}): Agent.Info {
  return createMockAgent({
    name: "test-primary",
    description: "Test primary agent with planning",
    mode: "primary",
    permission: PermissionNext.merge(
      createDefaultPermissions(),
      PermissionNext.fromConfig({
        question: "allow",
        plan_enter: "allow",
        plan_exit: "allow",
      }),
    ),
    ...overrides,
  })
}

/**
 * Create a hidden system agent
 */
export function createMockSystemAgent(overrides: Partial<Agent.Info> = {}): Agent.Info {
  return createMockAgent({
    name: "test-system",
    description: "Hidden system agent",
    mode: "primary",
    hidden: true,
    permission: PermissionNext.merge(
      createDefaultPermissions(),
      PermissionNext.fromConfig({
        "*": "deny",
      }),
    ),
    ...overrides,
  })
}

/**
 * Create a set of standard test agents
 */
export function createMockAgentSet(): Record<string, Agent.Info> {
  return {
    build: createMockPrimaryAgent({ name: "build", prompt: "Build mode prompt" }),
    plan: createMockPrimaryAgent({ name: "plan", prompt: "Plan mode prompt" }),
    explore: createMockSubagent({
      name: "explore",
      description: "Explore codebase",
      autoApprove: {
        enabled: true,
        allowedTools: ["Read", "Glob", "Grep", "LS"],
        riskThreshold: "low",
      },
    }),
    general: createMockSubagent({
      name: "general",
      description: "General purpose",
    }),
    "code-reviewer": createMockSubagent({
      name: "code-reviewer",
      description: "Code review",
    }),
    compaction: createMockSystemAgent({ name: "compaction" }),
    title: createMockSystemAgent({ name: "title" }),
    summary: createMockSystemAgent({ name: "summary" }),
  }
}

// ===== Agent Execution Context Mocks =====

/**
 * Mock agent execution context
 */
export interface MockAgentContext {
  agent: Agent.Info
  sessionId: string
  messageId: string
  toolCalls: MockToolCall[]
  responses: string[]
  errors: Error[]
  onToolCall: Mock<(call: MockToolCall) => void>
  onResponse: Mock<(response: string) => void>
  onError: Mock<(error: Error) => void>
}

export interface MockToolCall {
  name: string
  args: Record<string, unknown>
  result?: unknown
  error?: Error
  timestamp: number
}

/**
 * Create a mock agent execution context
 */
export function createMockAgentContext(
  agentOverrides: Partial<Agent.Info> = {},
): MockAgentContext {
  return {
    agent: createMockAgent(agentOverrides),
    sessionId: `session-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    messageId: `msg-${Date.now()}-${Math.random().toString(36).slice(2)}`,
    toolCalls: [],
    responses: [],
    errors: [],
    onToolCall: vi.fn(),
    onResponse: vi.fn(),
    onError: vi.fn(),
  }
}

/**
 * Record a tool call in the context
 */
export function recordToolCall(
  ctx: MockAgentContext,
  name: string,
  args: Record<string, unknown>,
  result?: unknown,
  error?: Error,
): MockToolCall {
  const call: MockToolCall = {
    name,
    args,
    result,
    error,
    timestamp: Date.now(),
  }
  ctx.toolCalls.push(call)
  ctx.onToolCall(call)
  return call
}

/**
 * Record a response in the context
 */
export function recordResponse(ctx: MockAgentContext, response: string): void {
  ctx.responses.push(response)
  ctx.onResponse(response)
}

/**
 * Record an error in the context
 */
export function recordError(ctx: MockAgentContext, error: Error): void {
  ctx.errors.push(error)
  ctx.onError(error)
}

// ===== Tool Chain Simulation =====

/**
 * Mock tool definition for simulation
 */
export interface MockToolDef {
  name: string
  description: string
  parameters: Record<string, { type: string; description?: string; required?: boolean }>
  execute: (args: Record<string, unknown>) => Promise<unknown>
}

/**
 * Create a mock tool definition
 */
export function createMockTool(
  name: string,
  execute: (args: Record<string, unknown>) => Promise<unknown>,
  parameters: MockToolDef["parameters"] = {},
): MockToolDef {
  return {
    name,
    description: `Mock tool: ${name}`,
    parameters,
    execute,
  }
}

/**
 * Standard mock tools for testing
 */
export function createMockToolSet(): Record<string, MockToolDef> {
  return {
    Read: createMockTool(
      "Read",
      async (args) => ({
        content: `Mock content for ${args.file_path}`,
        lines: 10,
      }),
      {
        file_path: { type: "string", description: "Path to file", required: true },
      },
    ),
    Glob: createMockTool(
      "Glob",
      async (args) => ({
        files: [`mock/file1.ts`, `mock/file2.ts`],
        pattern: args.pattern,
      }),
      {
        pattern: { type: "string", description: "Glob pattern", required: true },
      },
    ),
    Grep: createMockTool(
      "Grep",
      async (args) => ({
        matches: [
          { file: "mock/file.ts", line: 10, content: `Found: ${args.pattern}` },
        ],
      }),
      {
        pattern: { type: "string", description: "Search pattern", required: true },
      },
    ),
    Bash: createMockTool(
      "Bash",
      async (args) => ({
        stdout: `Executed: ${args.command}`,
        stderr: "",
        exitCode: 0,
      }),
      {
        command: { type: "string", description: "Command to execute", required: true },
      },
    ),
    Edit: createMockTool(
      "Edit",
      async (args) => ({
        success: true,
        file_path: args.file_path,
        changes: 1,
      }),
      {
        file_path: { type: "string", required: true },
        old_string: { type: "string", required: true },
        new_string: { type: "string", required: true },
      },
    ),
    Write: createMockTool(
      "Write",
      async (args) => ({
        success: true,
        file_path: args.file_path,
        bytes: (args.content as string)?.length ?? 0,
      }),
      {
        file_path: { type: "string", required: true },
        content: { type: "string", required: true },
      },
    ),
  }
}

/**
 * Tool chain executor for simulating sequential tool calls
 */
export class MockToolChain {
  private tools: Record<string, MockToolDef>
  private calls: MockToolCall[] = []
  private ctx: MockAgentContext

  constructor(ctx: MockAgentContext, tools: Record<string, MockToolDef> = createMockToolSet()) {
    this.ctx = ctx
    this.tools = tools
  }

  async execute(name: string, args: Record<string, unknown>): Promise<unknown> {
    const tool = this.tools[name]
    if (!tool) {
      const error = new Error(`Unknown tool: ${name}`)
      recordToolCall(this.ctx, name, args, undefined, error)
      throw error
    }

    try {
      const result = await tool.execute(args)
      recordToolCall(this.ctx, name, args, result)
      return result
    } catch (error) {
      recordToolCall(this.ctx, name, args, undefined, error as Error)
      throw error
    }
  }

  async executeSequence(
    calls: Array<{ name: string; args: Record<string, unknown> }>,
  ): Promise<unknown[]> {
    const results: unknown[] = []
    for (const call of calls) {
      results.push(await this.execute(call.name, call.args))
    }
    return results
  }

  getCalls(): MockToolCall[] {
    return [...this.ctx.toolCalls]
  }

  getCallsByTool(name: string): MockToolCall[] {
    return this.ctx.toolCalls.filter((c) => c.name === name)
  }
}

// ===== Permission Testing Helpers =====

/**
 * Test permission configuration result
 */
export interface PermissionTestResult {
  allowed: boolean
  reason: string
  matchedRule?: PermissionNext.Rule
}

/**
 * Check if a permission is allowed for a given agent and operation
 */
export function checkPermission(
  agent: Agent.Info,
  permission: string,
  pattern?: string,
): PermissionTestResult {
  const rules = agent.permission
  let matchedRule: PermissionNext.Rule | undefined

  // Find matching rule (last match wins)
  for (const rule of rules) {
    if (rule.permission === permission) {
      if (pattern === undefined || rule.pattern === "*" || rule.pattern === pattern) {
        matchedRule = rule
      }
    }
  }

  if (!matchedRule) {
    return {
      allowed: false,
      reason: `No rule found for permission: ${permission}`,
    }
  }

  return {
    allowed: matchedRule.action === "allow",
    reason: `Matched rule: ${matchedRule.permission}:${matchedRule.pattern} = ${matchedRule.action}`,
    matchedRule,
  }
}

/**
 * Assert that a permission is allowed
 */
export function assertPermissionAllowed(
  agent: Agent.Info,
  permission: string,
  pattern?: string,
): void {
  const result = checkPermission(agent, permission, pattern)
  if (!result.allowed) {
    throw new Error(`Expected permission ${permission}${pattern ? `:${pattern}` : ""} to be allowed. ${result.reason}`)
  }
}

/**
 * Assert that a permission is denied
 */
export function assertPermissionDenied(
  agent: Agent.Info,
  permission: string,
  pattern?: string,
): void {
  const result = checkPermission(agent, permission, pattern)
  if (result.allowed) {
    throw new Error(`Expected permission ${permission}${pattern ? `:${pattern}` : ""} to be denied. ${result.reason}`)
  }
}

// ===== Agent Response Testing =====

/**
 * Mock LLM response for testing agent behavior
 */
export interface MockLLMResponse {
  content: string
  toolCalls?: Array<{
    name: string
    args: Record<string, unknown>
  }>
  finishReason: "stop" | "tool_calls" | "length" | "content_filter"
}

/**
 * Create a mock LLM response
 */
export function createMockLLMResponse(overrides: Partial<MockLLMResponse> = {}): MockLLMResponse {
  return {
    content: "Mock LLM response",
    toolCalls: undefined,
    finishReason: "stop",
    ...overrides,
  }
}

/**
 * Create a mock LLM response with tool calls
 */
export function createMockLLMToolResponse(
  toolCalls: MockLLMResponse["toolCalls"],
  content = "",
): MockLLMResponse {
  return {
    content,
    toolCalls,
    finishReason: "tool_calls",
  }
}

/**
 * Mock streaming chunk for testing streaming responses
 */
export interface MockStreamChunk {
  type: "text" | "tool_call" | "tool_result" | "done"
  content?: string
  toolCall?: { name: string; args: Record<string, unknown> }
  toolResult?: unknown
}

/**
 * Create mock stream chunks for testing streaming
 */
export function createMockStreamChunks(
  text: string,
  chunkSize = 10,
): MockStreamChunk[] {
  const chunks: MockStreamChunk[] = []
  for (let i = 0; i < text.length; i += chunkSize) {
    chunks.push({
      type: "text",
      content: text.slice(i, i + chunkSize),
    })
  }
  chunks.push({ type: "done" })
  return chunks
}

// ===== Agent State Testing =====

/**
 * Create a mock Instance.state for testing
 */
export function createMockAgentState(agents: Record<string, Agent.Info> = createMockAgentSet()) {
  return vi.fn(async () => agents)
}

/**
 * Reset agent state mock
 */
export function resetAgentStateMock(): void {
  // This would be called to reset any mocked state
  // In practice, the test preload handles this via State.reset() and Instance.reset()
}

// ===== Export all =====
export {
  createDefaultPermissions,
  createMockAgent,
  createMockSubagent,
  createMockPrimaryAgent,
  createMockSystemAgent,
  createMockAgentSet,
  createMockAgentContext,
  recordToolCall,
  recordResponse,
  recordError,
  createMockTool,
  createMockToolSet,
  MockToolChain,
  checkPermission,
  assertPermissionAllowed,
  assertPermissionDenied,
  createMockLLMResponse,
  createMockLLMToolResponse,
  createMockStreamChunks,
  createMockAgentState,
  resetAgentStateMock,
}
