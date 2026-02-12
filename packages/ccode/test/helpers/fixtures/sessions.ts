/**
 * Session Test Fixtures
 *
 * Provides mock session data for testing TUI components.
 */

export interface Message {
  id: string
  role: "user" | "assistant" | "system" | "tool"
  content: string
  timestamp: number
  status: "streaming" | "complete" | "error"
  error?: {
    type: string
    message: string
    code?: number
  }
  toolCalls?: Array<{
    id: string
    type: string
    params: Record<string, unknown>
  }>
  toolCallId?: string
}

export interface Session {
  id: string
  title: string
  createdAt: number
  updatedAt: number
  messages: Message[]
  model?: string
  provider?: string
  tags?: string[]
  cwd?: string
  forkedFrom?: string
}

/**
 * Create a mock message
 */
export function createMockMessage(overrides: Partial<Message> = {}): Message {
  return {
    id: `msg-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
    role: overrides.role ?? "user",
    content: overrides.content ?? "",
    timestamp: overrides.timestamp ?? Date.now(),
    status: overrides.status ?? "complete",
    ...overrides,
  }
}

/**
 * Create a mock user message
 */
export function createUserMessage(content: string): Message {
  return createMockMessage({ role: "user", content })
}

/**
 * Create a mock assistant message
 */
export function createAssistantMessage(content: string): Message {
  return createMockMessage({ role: "assistant", content })
}

/**
 * Create a mock system message
 */
export function createSystemMessage(content: string): Message {
  return createMockMessage({ role: "system", content })
}

/**
 * Create a mock session
 */
export function createMockSession(overrides: Partial<Session> = {}): Session {
  const id = overrides.id ?? `sess-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
  return {
    id,
    title: overrides.title ?? "Test Session",
    createdAt: overrides.createdAt ?? Date.now(),
    updatedAt: overrides.updatedAt ?? Date.now(),
    messages: overrides.messages ?? [],
    model: overrides.model ?? "claude-sonnet-4-20250514",
    provider: overrides.provider ?? "anthropic",
    tags: overrides.tags ?? [],
    cwd: overrides.cwd ?? "/test/project",
    ...overrides,
  }
}

/**
 * Create a mock session with messages
 */
export function createSessionWithMessages(
  messages: Array<{ role: "user" | "assistant" | "system"; content: string }>,
  overrides: Partial<Session> = {},
): Session {
  const mockMessages = messages.map((m) =>
    createMockMessage({ role: m.role, content: m.content }),
  )
  return createMockSession({ ...overrides, messages: mockMessages })
}

/**
 * Create a mock coding session
 */
export function createCodingSession(): Session {
  return createSessionWithMessages(
    [
      { role: "user", content: "Help me write a function to calculate fibonacci numbers" },
      {
        role: "assistant",
        content: "Here's a recursive implementation of the Fibonacci function:",
      },
      { role: "user", content: "Can you make it more efficient?" },
      { role: "assistant", content: "Sure! Here's an iterative version with O(n) complexity:" },
    ],
    { title: "Fonacci Function" },
  )
}

/**
 * Create a mock debugging session
 */
export function createDebuggingSession(): Session {
  return createSessionWithMessages(
    [
      { role: "user", content: "I'm getting an error when running my tests" },
      { role: "assistant", content: "Let me help you debug that. What's the error message?" },
      { role: "user", content: 'TypeError: Cannot read property "map" of undefined' },
      { role: "assistant", content: "This suggests you\'re trying to map over an undefined value..." },
    ],
    { title: "Debugging TypeError" },
  )
}

/**
 * Create multiple mock sessions
 */
export function createMockSessions(count: number): Session[] {
  return Array.from({ length: count }, (_, i) =>
    createMockSession({
      id: `sess-test-${i}`,
      title: `Test Session ${i + 1}`,
      createdAt: Date.now() - count * 60_000 + i * 60_000,
    }),
  )
}

/**
 * Create sessions with different states
 */
export function createSessionStates(): {
  active: Session
  completed: Session
  archived: Session
} {
  return {
    active: createMockSession({
      id: "sess-active",
      title: "Active Session",
      tags: ["active"],
    }),
    completed: createMockSession({
      id: "sess-completed",
      title: "Completed Task",
      tags: ["completed"],
      createdAt: Date.now() - 3600000,
      updatedAt: Date.now() - 3600000,
    }),
    archived: createMockSession({
      id: "sess-archived",
      title: "Old Project",
      tags: ["archived"],
      createdAt: Date.now() - 86400000,
      updatedAt: Date.now() - 86400000,
    }),
  }
}

/**
 * Create a session with tool calls
 */
export function createSessionWithToolCalls(): Session {
  return createMockSession({
    title: "File Operations Session",
    messages: [
      createUserMessage("Read the package.json file"),
      createAssistantMessage("I'll read the package.json file for you."),
      createMockMessage({
        role: "assistant",
        content: "",
        toolCalls: [
          {
            id: "call-1",
            type: "read",
            params: { filePath: "package.json" },
          },
        ],
      }),
      createMockMessage({
        role: "tool",
        content: '{\n  "name": "test-project",\n  "version": "1.0.0"\n}',
        toolCallId: "call-1",
      }),
    ],
  })
}

/**
 * Create a session with edits
 */
export function createSessionWithEdits(): Session {
  return createMockSession({
    title: "Edit Session",
    messages: [
      createUserMessage("Change the greeting to 'Hello World'"),
      createMockMessage({
        role: "assistant",
        content: "I'll update the greeting in the index.ts file.",
        toolCalls: [
          {
            id: "call-2",
            type: "edit",
            params: {
              filePath: "src/index.ts",
              oldText: 'console.log("Hi")',
              newText: 'console.log("Hello World")',
            },
          },
        ],
      }),
      createMockMessage({
        role: "tool",
        content: "Successfully updated src/index.ts",
        toolCallId: "call-2",
      }),
    ],
  })
}

/**
 * Create a forked session (for testing timeline forking)
 */
export function createForkedSessions(): { original: Session; forked: Session } {
  const baseMessages = [
    createUserMessage("Create a function to add two numbers"),
    createAssistantMessage("Here's a simple add function:\n\n```js\nfunction add(a, b) { return a + b; }\n```"),
  ]

  const original = createMockSession({
    id: "sess-original",
    title: "Add Function",
    messages: [
      ...baseMessages,
      createUserMessage("Now create a subtract function"),
      createAssistantMessage("Here's a subtract function:\n\n```js\nfunction subtract(a, b) { return a - b; }\n```"),
    ],
  })

  const forked = createMockSession({
    id: "sess-forked",
    title: "Add Function (forked)",
    forkedFrom: original.id,
    messages: [
      ...baseMessages,
      createUserMessage("Now create a multiply function"),
      createAssistantMessage("Here's a multiply function:\n\n```js\nfunction multiply(a, b) { return a * b; }\n```"),
    ],
  })

  return { original, forked }
}
