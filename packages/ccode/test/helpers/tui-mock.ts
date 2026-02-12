/**
 * TUI Mock Helpers
 *
 * Provides mock implementations for OpenTUI primitives and related types
 * to enable testing of TUI components without requiring a terminal.
 */

import { vi } from "bun:test"
import type { ParsedKey } from "@opentui/core"

/**
 * Mock renderer with core OpenTUI renderer methods
 */
export function createMockRenderer() {
  const mock = {
    dimensions: { width: 120, height: 30 },
    requestRender: vi.fn(),
    focus: vi.fn(),
    blur: vi.fn(),
    currentFocusedRenderable: null as RenderableMock | null,
    getSelection: vi.fn(() => null),
    clearSelection: vi.fn(),
    root: {
      getChildren: () => [],
    },
  }
  return mock
}

/**
 * Mock renderable with common properties
 */
export interface RenderableMock {
  focus: () => void
  blur: () => void
  isDestroyed: boolean
  getChildren: () => RenderableMock[]
}

export function createMockRenderable(): RenderableMock {
  const mock: RenderableMock = {
    focus: vi.fn(),
    blur: vi.fn(),
    isDestroyed: false,
    getChildren: () => [],
  }
  return mock
}

/**
 * Mock keyboard event with all ParsedKey properties
 */
export function createMockKeyboardEvent(partial: Partial<ParsedKey> = {}): ParsedKey {
  return {
    name: "a",
    ctrl: false,
    meta: false,
    shift: false,
    option: false,
    sequence: "",
    number: false,
    raw: "",
    eventType: "press",
    source: "raw",
    ...partial,
  }
}

export { createMockKeyboardEvent as createMockKey }

/**
 * Mock terminal dimensions
 */
export function createMockDimensions(width = 120, height = 30) {
  return { width, height }
}

/**
 * Mock RGBA color for testing
 */
export function createMockRGBA(r = 0, g = 0, b = 0, a = 255) {
  return { r, g, b, a }
}

/**
 * Mock theme data
 */
export function createMockTheme() {
  return {
    name: "dark",
    background: createMockRGBA(18, 18, 18),
    foreground: createMockRGBA(241, 241, 241),
    backgroundPanel: createMockRGBA(26, 26, 26),
    border: createMockRGBA(59, 59, 59),
    primary: createMockRGBA(97, 175, 239),
    secondary: createMockRGBA(207, 146, 120),
    success: createMockRGBA(86, 182, 91),
    warning: createMockRGBA(227, 184, 76),
    error: createMockRGBA(214, 79, 79),
    muted: createMockRGBA(119, 119, 119),
  }
}

/**
 * Mock keybind config
 */
export function createMockKeybindConfig() {
  return {
    leader: "space" as const,
    new_session: "ctrl+n" as const,
    previous_session: "ctrl+p" as const,
    next_session: "ctrl+tab" as const,
    close_session: "ctrl+w" as const,
    command_palette: "ctrl+shift+p" as const,
    model_select: "ctrl+shift+m" as const,
    session_list: "ctrl+shift+s" as const,
    submit: "ctrl+return" as const,
    cancel: "escape" as const,
  }
}

/**
 * Create a mock sync context data object
 */
export function createMockSyncData() {
  return {
    config: {
      keybinds: createMockKeybindConfig(),
      theme: "dark",
      editor: "code",
    },
    session: [],
    cwd: "/test/project",
    root: "/test/project",
  }
}

/**
 * Spy on console methods during tests
 */
export function mockConsole() {
  const spies = {
    log: vi.spyOn(console, "log").mockImplementation(() => {}),
    error: vi.spyOn(console, "error").mockImplementation(() => {}),
    warn: vi.spyOn(console, "warn").mockImplementation(() => {}),
    debug: vi.spyOn(console, "debug").mockImplementation(() => {}),
  }

  return {
    restore: () => {
      for (const spy of Object.values(spies)) {
        spy.mockRestore()
      }
    },
  }
}

/**
 * Create a mock function that tracks calls
 */
export function createMockTracker<T extends (...args: any[]) => any>(
  fn?: T,
): T & { calls: any[]; lastCall: any[] | null } {
  const calls: any[][] = []
  const mockFn = ((...args: any[]) => {
    calls.push(args)
    return fn?.(...args)
  }) as T & { calls: any[]; lastCall: any[] | null }

  mockFn.calls = calls
  Object.defineProperty(mockFn, "lastCall", {
    get: () => calls.at(-1) ?? null,
  })

  return mockFn
}

/**
 * Wait for async operations to complete
 */
export async function flushPromises(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0))
}

/**
 * Create a mock stdin/stdout for terminal testing
 */
export function createMockStdio() {
  let written: Buffer[] = []

  return {
    stdin: {
      isTTY: true,
      setRawMode: vi.fn(),
      on: vi.fn(),
      removeListener: vi.fn(),
    },
    stdout: {
      isTTY: true,
      write: vi.fn((data: string | Buffer) => {
        written.push(Buffer.isBuffer(data) ? data : Buffer.from(data))
        return true
      }),
      getWritten: () => written,
      clearWritten: () => {
        written = []
      },
    },
  }
}

// ===== Additional Mock Helpers for TUI Testing =====

/**
 * Mock agent info for agent selection and display
 */
export interface MockAgentInfo {
  name: string
  native?: boolean
  description?: string
  label?: string
}

export function createMockAgent(overrides: Partial<MockAgentInfo> = {}): MockAgentInfo {
  return {
    name: "editor",
    native: true,
    description: "Edit files",
    label: "Editor",
    ...overrides,
  }
}

export function createMockAgents(count = 3): MockAgentInfo[] {
  const agents: MockAgentInfo[] = [
    { name: "editor", native: true, description: "Edit files", label: "Editor" },
    { name: "planner", native: true, description: "Plan changes", label: "Planner" },
    { name: "reviewer", native: true, description: "Review code", label: "Reviewer" },
  ]
  return agents.slice(0, count)
}

/**
 * Mock model info for model selection dialogs
 */
export interface MockModelInfo {
  id: string
  name: string
  provider: string
  description?: string
  variants?: string[]
}

export function createMockModel(overrides: Partial<MockModelInfo> = {}): MockModelInfo {
  return {
    id: "claude-sonnet-4-5",
    name: "Claude Sonnet 4.5",
    provider: "anthropic",
    description: "Balanced model for coding",
    variants: ["sonnet", "opus"],
    ...overrides,
  }
}

export function createMockModels(count = 3): MockModelInfo[] {
  const models: MockModelInfo[] = [
    { id: "claude-sonnet-4-5", name: "Claude Sonnet 4.5", provider: "anthropic" },
    { id: "claude-opus-4-5", name: "Claude Opus 4.5", provider: "anthropic" },
    { id: "gpt-4o", name: "GPT-4o", provider: "openai" },
  ]
  return models.slice(0, count)
}

/**
 * Mock message for session testing
 */
export interface MockMessage {
  id: string
  role: "user" | "assistant" | "system"
  content: string
  timestamp?: number
  parts?: MockMessagePart[]
}

export interface MockMessagePart {
  type: "text" | "file" | "image" | "tool" | "diff"
  content: string
  [key: string]: any
}

export function createMockMessage(overrides: Partial<MockMessage> = {}): MockMessage {
  return {
    id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
    role: "user",
    content: "Test message",
    timestamp: Date.now(),
    parts: [{ type: "text", content: "Test message" }],
    ...overrides,
  }
}

export function createMockMessages(count = 10): MockMessage[] {
  return Array.from({ length: count }, (_, i) =>
    createMockMessage({
      id: `msg-${i}`,
      role: i % 2 === 0 ? "user" : "assistant",
      content: `Message ${i + 1}`,
    }),
  )
}

/**
 * Mock session for session management testing
 */
export interface MockSession {
  id: string
  title: string
  createdAt: number
  updatedAt: number
  messages: MockMessage[]
  agent?: string
  model?: string
  tags?: string[]
}

export function createMockSession(overrides: Partial<MockSession> = {}): MockSession {
  const now = Date.now()
  return {
    id: `session-${now}-${Math.random().toString(36).slice(2, 9)}`,
    title: "New Session",
    createdAt: now,
    updatedAt: now,
    messages: [],
    agent: "editor",
    model: "claude-sonnet-4-5",
    tags: [],
    ...overrides,
  }
}

export function createMockSessions(count = 5): MockSession[] {
  const baseTime = Date.now()
  return Array.from({ length: count }, (_, i) =>
    createMockSession({
      id: `session-${i}`,
      title: `Session ${i + 1}`,
      createdAt: baseTime - (count - i) * 60_000, // Stagger by 1 minute
      updatedAt: baseTime - (count - i) * 60_000,
      messages: i > 0 ? createMockMessages(i * 2) : [],
    }),
  )
}

/**
 * Mock SDK context for testing SDK-dependent components
 */
export interface MockSDKContext {
  agents: MockAgentInfo[]
  models: MockModelInfo[]
  currentAgent: string
  currentModel: string
  currentProvider: string
}

export function createMockSDK(overrides: Partial<MockSDKContext> = {}): MockSDKContext {
  return {
    agents: createMockAgents(),
    models: createMockModels(),
    currentAgent: "editor",
    currentModel: "claude-sonnet-4-5",
    currentProvider: "anthropic",
    ...overrides,
  }
}

/**
 * Mock route for navigation testing
 */
export interface MockRoute {
  type: "home" | "session"
  sessionId?: string
  params?: Record<string, string>
}

export function createMockRoute(overrides: Partial<MockRoute> = {}): MockRoute {
  return {
    type: "home",
    ...overrides,
  }
}

/**
 * Mock file info for file attachment testing
 */
export interface MockFileInfo {
  path: string
  filename: string
  mime: string
  content?: string
  size?: number
}

export function createMockFile(overrides: Partial<MockFileInfo> = {}): MockFileInfo {
  return {
    path: "/test/project/src/test.ts",
    filename: "test.ts",
    mime: "text/typescript",
    content: "export function test() {}",
    size: 32,
    ...overrides,
  }
}

/**
 * Mock MCP server info for MCP integration testing
 */
export interface MockMCPServer {
  name: string
  status: "connected" | "disconnected" | "error"
  tools?: string[]
  error?: string
}

export function createMockMCPServer(overrides: Partial<MockMCPServer> = {}): MockMCPServer {
  return {
    name: "test-server",
    status: "connected",
    tools: ["tool1", "tool2"],
    ...overrides,
  }
}

export function createMockMCPServers(count = 3): MockMCPServer[] {
  return Array.from({ length: count }, (_, i) =>
    createMockMCPServer({
      name: `server-${i + 1}`,
      status: i === 1 ? "disconnected" : "connected",
      tools: [`server-${i + 1}-tool1`, `server-${i + 1}-tool2`],
    }),
  )
}

/**
 * Mock keybind info for keybind testing
 */
export interface MockKeybindInfo {
  key: string
  ctrl: boolean
  shift: boolean
  meta: boolean
  name: string
}

export function createMockKeybindInfo(
  keybind: string,
): MockKeybindInfo {
  const parts = keybind.toLowerCase().split("+")
  const key = parts.at(-1) ?? ""
  const ctrl = parts.includes("ctrl")
  const shift = parts.includes("shift")
  const meta = parts.includes("meta") || parts.includes("cmd")

  return { key, ctrl, shift, meta, name: keybind }
}

/**
 * Mock prompt info for prompt component testing
 */
export interface MockPromptInfo {
  input: string
  parts: Array<{
    type: "file" | "image" | "text" | "agent"
    [key: string]: any
  }>
  mode?: "normal" | "shell"
  cursor?: number
}

export function createMockPrompt(overrides: Partial<MockPromptInfo> = {}): MockPromptInfo {
  return {
    input: "",
    parts: [],
    mode: "normal",
    cursor: 0,
    ...overrides,
  }
}

/**
 * Mock dialog option for select dialogs
 */
export interface MockDialogOption {
  value: string
  title: string
  description?: string
  category?: string
  disabled?: boolean
}

export function createMockDialogOption(
  overrides: Partial<MockDialogOption> = {},
): MockDialogOption {
  return {
    value: "option-1",
    title: "Option 1",
    ...overrides,
  }
}

export function createMockDialogOptions(count = 5): MockDialogOption[] {
  return Array.from({ length: count }, (_, i) =>
    createMockDialogOption({
      value: `option-${i + 1}`,
      title: `Option ${i + 1}`,
    }),
  )
}

/**
 * Create a mock provider configuration
 */
export interface MockProviderConfig {
  id: string
  name: string
  baseUrl?: string
  apiKey?: string
  models: string[]
}

export function createMockProvider(overrides: Partial<MockProviderConfig> = {}): MockProviderConfig {
  return {
    id: "anthropic",
    name: "Anthropic",
    models: ["claude-sonnet-4-5", "claude-opus-4-5"],
    ...overrides,
  }
}

export function createMockProviders(count = 3): MockProviderConfig[] {
  const providers: MockProviderConfig[] = [
    { id: "anthropic", name: "Anthropic", models: ["claude-sonnet-4-5", "claude-opus-4-5"] },
    { id: "openai", name: "OpenAI", models: ["gpt-4o", "gpt-4o-mini"] },
    { id: "google", name: "Google", models: ["gemini-2.0-flash-exp"] },
  ]
  return providers.slice(0, count)
}

/**
 * Mock theme info for theme selection
 */
export interface MockThemeInfo {
  id: string
  name: string
  isDark: boolean
  preview: string
}

export function createMockThemeInfo(overrides: Partial<MockThemeInfo> = {}): MockThemeInfo {
  return {
    id: "dark",
    name: "Dark Theme",
    isDark: true,
    preview: "███",
    ...overrides,
  }
}

export function createMockThemes(): MockThemeInfo[] {
  return [
    { id: "dark", name: "Dark Theme", isDark: true, preview: "███" },
    { id: "light", name: "Light Theme", isDark: false, preview: "░░░" },
    { id: "nord", name: "Nord", isDark: true, preview: "▓▓▓" },
  ]
}
