/**
 * Observer Test Setup
 *
 * This file must be imported FIRST (before any @/observer imports)
 * to properly mock dependencies before observer modules try to use them.
 *
 * IMPORTANT: The order of mock.module() calls matters! Mock dependencies
 * of dependencies first (e.g., mock @/util/log before @/config/config).
 *
 * @module test/observer/setup
 */

import { mock } from "bun:test"

// Create a mock logger factory
const createLogger = (tags?: Record<string, unknown>) => ({
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
  tag: () => createLogger(tags),
  clone: () => createLogger(tags),
  time: () => ({ stop: () => {}, [Symbol.dispose]: () => {} }),
  structured: {
    configureObservability: () => {},
    initObservability: async () => {},
    createSpan: () => ({ end: () => {} }),
    point: () => {},
  },
})

// Create a mock Zod-like schema for Log.Level
const mockZodSchema = {
  parse: (v: string) => v,
  safeParse: (v: string) => ({ success: true, data: v }),
  optional: () => mockZodSchema,
  default: () => mockZodSchema,
  describe: () => mockZodSchema,
}

// Mock the Log module before any observer modules import it
mock.module("@/util/log", () => ({
  Log: {
    Level: mockZodSchema,
    Default: createLogger({ service: "default" }),
    init: async () => {},
    create: createLogger,
    file: () => "",
  },
}))

// Mock Instance module - must be before Config which uses Instance.state
mock.module("@/project/instance", () => ({
  Instance: {
    directory: "/tmp/test-project",
    worktree: "/tmp/test-project",
    project: {
      id: "test_project",
      name: "Test Project",
      worktree: "/tmp/test-project",
      sandboxes: [],
      time: { created: Date.now(), updated: Date.now() },
    },
    reset: () => {},
    provide: async <R>(input: { directory: string; init?: () => Promise<unknown>; fn: () => R }): Promise<R> => {
      return input.fn()
    },
    containsPath: () => true,
    containsPathSafe: async () => true,
    state: <S>(init: () => S) => {
      // Return a function that returns the initialized value (lazy evaluation)
      let cached: S | undefined
      return () => {
        if (cached === undefined) {
          cached = init()
        }
        return cached
      }
    },
    dispose: async () => {},
  },
  getProjectIDForStorage: () => "test_project",
}))

// Mock Context module
mock.module("@/util/context", () => ({
  Context: {
    create: () => ({
      use: () => ({
        directory: "/tmp/test-project",
        worktree: "/tmp/test-project",
        project: { id: "test_project" },
      }),
      provide: async <T, R>(value: T, fn: () => R) => fn(),
    }),
  },
}))

// Mock Config module
mock.module("@/config/config", () => ({
  Config: {
    info: () => ({ agent: {}, mode: {} }),
    global: async () => ({}),
    project: async () => ({}),
    loadFile: async () => ({}),
  },
}))

// Mock the Bus module
mock.module("@/bus", () => ({
  Bus: {
    publish: async () => {},
    subscribe: () => () => {},
    unsubscribe: () => {},
  },
  BusEvent: {
    define: () => ({}),
    create: () => ({}),
    parse: () => ({}),
    extend: () => ({}),
  },
}))

// Mock bus-event separately
mock.module("@/bus/bus-event", () => ({
  BusEvent: {
    define: () => ({}),
    create: () => ({}),
    parse: () => ({}),
    extend: () => ({}),
  },
}))

// Mock observability
mock.module("@/observability", () => ({
  configureObservability: () => {},
  initObservability: async () => {},
  createSpan: () => ({ end: () => {} }),
  point: () => {},
  apiCallPoint: () => ({ done: () => {}, error: () => {} }),
  StructuredLog: {
    configure: () => {},
    init: async () => {},
    create: () => ({
      debug: () => {},
      info: () => {},
      warn: () => {},
      error: () => {},
    }),
  },
}))

// Mock Storage module to avoid SQLite dependency in tests
mock.module("@/infrastructure/storage/storage", () => ({
  Storage: {
    NotFoundError: {
      create: () => ({}),
    },
    get: async () => null,
    set: async () => {},
    delete: async () => {},
    list: async () => [],
  },
}))

// Mock the Hands bridge for executor tests
mock.module("@/autonomous/hands/bridge", () => ({
  getBridge: () => ({
    health: async () => false,
    trigger: async () => ({ success: false, error: "Not available in tests" }),
    list: async () => [],
  }),
}))

// Mock memory-markdown module to avoid complex initialization chains
mock.module("@/memory-markdown", () => ({
  getStorage: () => ({
    basePath: "/tmp/test-memory",
    projectId: "test_project",
    dailyPath: "/tmp/test-memory/daily",
    longTermPath: "/tmp/test-memory/MEMORY.md",
    readDailyNote: async () => null,
    writeDailyNote: async () => {},
    listDailyNotes: async () => [],
    readLongTermMemory: async () => "",
    writeLongTermMemory: async () => {},
  }),
  appendDailyNote: async () => {},
  createEntry: (type: string, content: string) => ({
    type,
    content,
    timestamp: new Date(),
  }),
  DailyEntryType: {
    action: "action",
    decision: "decision",
    output: "output",
    error: "error",
  },
  loadMarkdownMemoryContext: async () => ({
    longTerm: "",
    daily: [],
    summary: { totalFiles: 0, categories: [] },
  }),
  loadCategoryContext: async () => "",
  loadRecentContext: async () => "",
  getMemorySummary: async () => ({ totalFiles: 0, categories: [] }),
  loadStorageConfig: () => ({}),
  resetConfigCache: () => {},
  consolidateMemory: async () => {},
  readDailyNote: async () => null,
  writeDailyNote: async () => {},
  listDailyNotes: async () => [],
  readLongTermMemory: async () => "",
  writeLongTermMemory: async () => {},
  detectProjectId: async () => "test_project",
  detectProjectIdSync: () => "test_project",
}))
