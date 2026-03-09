// IMPORTANT: Set env vars BEFORE any imports from src/ directory
// Paths are now derived from CCODE_TEST_HOME, so we must set this first
import os from "os"
import path from "path"
import fs from "fs/promises"
import fsSync from "fs"
import { afterAll, beforeEach, mock } from "bun:test"

const dir = path.join(os.tmpdir(), "codecoder-test-data-" + process.pid)
await fs.mkdir(dir, { recursive: true })
afterAll(() => {
  fsSync.rmSync(dir, { recursive: true, force: true })
})
// Set test home directory to isolate tests from user's actual home directory
// This prevents tests from picking up real user configs/skills from ~/.claude/skills
// All paths (data, cache, state, log, bin) are now derived from ~/.codecoder/
// which is based on CCODE_TEST_HOME when set
const testHome = path.join(dir, "home")
await fs.mkdir(testHome, { recursive: true })
process.env["CCODE_TEST_HOME"] = testHome

// Create the unified ~/.codecoder/ directory structure for tests
const codeCoderDir = path.join(testHome, ".codecoder")
await fs.mkdir(path.join(codeCoderDir, "cache"), { recursive: true })
await fs.mkdir(path.join(codeCoderDir, "data"), { recursive: true })
await fs.mkdir(path.join(codeCoderDir, "state"), { recursive: true })
await fs.mkdir(path.join(codeCoderDir, "log"), { recursive: true })
await fs.mkdir(path.join(codeCoderDir, "bin"), { recursive: true })

// Write the cache version file to prevent global/index.ts from clearing the cache
await fs.writeFile(path.join(codeCoderDir, "cache", "version"), "18")

// Clear provider env vars to ensure clean test state
delete process.env["ANTHROPIC_API_KEY"]
delete process.env["OPENAI_API_KEY"]
delete process.env["GOOGLE_API_KEY"]
delete process.env["GOOGLE_GENERATIVE_AI_API_KEY"]
delete process.env["AZURE_OPENAI_API_KEY"]
delete process.env["AWS_ACCESS_KEY_ID"]
delete process.env["AWS_PROFILE"]
delete process.env["AWS_REGION"]
delete process.env["AWS_BEARER_TOKEN_BEDROCK"]
delete process.env["OPENROUTER_API_KEY"]
delete process.env["GROQ_API_KEY"]
delete process.env["MISTRAL_API_KEY"]
delete process.env["PERPLEXITY_API_KEY"]
delete process.env["TOGETHER_API_KEY"]
delete process.env["XAI_API_KEY"]
delete process.env["DEEPSEEK_API_KEY"]
delete process.env["FIREWORKS_API_KEY"]
delete process.env["CEREBRAS_API_KEY"]
delete process.env["SAMBANOVA_API_KEY"]

// Check if we're running observer tests - they need special mocking
// Observer tests have their own setup.ts that must run first
const isObserverTest = process.argv.some((arg) => arg.includes("test/observer"))

// Set up minimal Log mock FIRST, before any src/ imports
// This prevents initialization errors in modules that use Log.create() at module level
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

// Set up Log mock before any imports
mock.module("@/util/log", () => ({
  Log: {
    Level: mockZodSchema,
    Default: createLogger({ service: "default" }),
    init: async () => {},
    create: createLogger,
    file: () => "",
  },
}))

if (!isObserverTest) {
  // Now safe to import from src/
  const { Log } = await import("../src/util/log")
  const { State } = await import("../src/project/state")
  const { Instance } = await import("../src/project/instance")
  // Note: resetAllLazy() can cause initialization order issues
  // const { resetAllLazy } = await import("@codecoder-ai/util/lazy")

  Log.init({
    print: true,
    dev: true,
    level: "DEBUG",
  })

  // Reset state before each test to ensure test isolation
  // This clears all cached singleton state from Instance.state() calls
  beforeEach(() => {
    State.reset()
    Instance.reset()
    // resetAllLazy() - disabled: can cause re-initialization issues
  })
}
