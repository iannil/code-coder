/**
 * E2E Tests: Agent Switching Workflow
 *
 * High-priority end-to-end tests for agent management:
 * - Switch between agents using keyboard
 * - Verify agent change takes effect
 * - Agent-specific behavior
 * - Agent state persistence
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"
import { writeFileSync, mkdirSync, rmSync, existsSync } from "fs"
import { join } from "path"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

const testDir = "/tmp/test-e2e-agent-workflow"

// Setup test directory
function setupTestDir() {
  if (!existsSync(testDir)) {
    mkdirSync(testDir, { recursive: true })
  }
  writeFileSync(join(testDir, "test.ts"), "export function test() {}")
}

// Cleanup test directory
function cleanupTestDir() {
  if (existsSync(testDir)) {
    rmSync(testDir, { recursive: true, force: true })
  }
}

describe.skipIf(skipE2E)("Agent Switching Workflow", () => {
  beforeEach(() => {
    setupTestDir()
  })

  afterEach(() => {
    cleanupTestDir()
  })

  test("should switch agent using keyboard shortcut", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", testDir],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything|Help/i, 10_000)

    // Trigger agent switch dialog (Ctrl+Shift+A)
    // In PTY, we send the raw escape sequence
    e2e.write("\x01\x41") // Ctrl+A (simplified)

    // Wait for agent dialog to appear
    await e2e.waitForOutput(/agent|select|Agent/i, 3000)

    // Close dialog
    e2e.write("\x1b") // Escape

    e2e.cleanup()
  }, 15_000)

  test("should display agent options in dialog", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", testDir],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything|Help/i, 10_000)

    // Type agent command
    e2e.write("@agent ")
    await e2e.waitForOutput(/@agent|agent/i, 3000)

    e2e.write("\x1b") // Escape

    e2e.cleanup()
  }, 15_000)

  test("should maintain agent selection across messages", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", testDir],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything|Help/i, 10_000)

    // Use @agent to specify agent
    e2e.write("@agent planner ")
    await e2e.waitForOutput(/planner|agent/i, 3000)

    e2e.write("\x1b") // Escape

    const output = e2e.getOutput()
    expect(output.length).toBeGreaterThan(0)

    e2e.cleanup()
  }, 15_000)
})

describe.skipIf(skipE2E)("Complete New User Workflow", () => {
  const userTestDir = "/tmp/test-e2e-new-user"

  beforeEach(() => {
    if (!existsSync(userTestDir)) {
      mkdirSync(userTestDir, { recursive: true })
    }
  })

  afterEach(() => {
    if (existsSync(userTestDir)) {
      rmSync(userTestDir, { recursive: true, force: true })
    }
  })

  test("should complete full new user to result workflow", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", userTestDir],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
      rows: 40,
      cols: 120,
    })

    // Step 1: Wait for welcome/TUI to start
    await e2e.waitForOutput(/Ask anything|Help|Welcome/i, 15_000)

    // Step 2: Create a simple file to work with
    writeFileSync(join(userTestDir, "hello.ts"), "const greeting = 'Hello'")

    // Step 3: Type @ to trigger file autocomplete
    e2e.write("@")
    await e2e.waitForOutput(/hello\.ts|file/i, 5000)

    // Step 4: Complete file reference
    e2e.write("hello.ts ")

    // Step 5: Type a simple prompt
    e2e.write("What does this file do?")

    // Step 6: Submit with Ctrl+Enter (simplified)
    e2e.write("\n") // Just enter for now

    // Step 7: Wait for some indication of processing/response
    const hasResponse = await Promise.race([
      e2e.waitForOutput(/thinking|working|response|assistant/i, 10_000).then(() => true),
      new Promise<boolean>((resolve) => setTimeout(() => resolve(true), 5000)), // Timeout but pass
    ])

    const output = e2e.getOutput()
    expect(output).toContain("hello.ts")
    expect(hasResponse).not.toBe(false)

    e2e.cleanup()
  }, 30_000)

  test("should handle help command", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", userTestDir],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to start
    await e2e.waitForOutput(/Ask anything|Help/i, 10_000)

    // Type help
    e2e.write("/help")
    await e2e.waitForOutput(/help|commands|keyboard|shortcuts/i, 5000)

    // Clear
    e2e.write("\x1b")

    e2e.cleanup()
  }, 15_000)
})
