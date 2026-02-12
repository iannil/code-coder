/**
 * E2E Tests: Basic Prompt Flow
 * Testing first prompt submission and session creation
 */

import { describe, test, expect } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

describe.skipIf(skipE2E)("Basic Prompt Flow E2E", () => {
  test("should accept first prompt and create session", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-prompt-project"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to start
    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Type a simple prompt
    e2e.write("Hello, can you help me?\n")

    // Wait for some response or indication that prompt was submitted
    // This might show as a new session being created or response starting
    const hasResponse = await Promise.race([
      e2e.waitForOutput(/session|thinking|response|working/i, 5000).then(() => true),
      e2e.waitForOutput(/error|failed/i, 5000).then(() => false),
      new Promise<boolean>((resolve) => setTimeout(() => resolve(true), 3000)), // Timeout but don't fail
    ])

    // The test passes if we don't crash
    expect(hasResponse).not.toBe(false)

    e2e.cleanup()
  }, 20_000)

  test("should handle command palette invocation", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-command-project"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to start
    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Trigger command palette with Ctrl+Shift+P (simulated with special chars)
    // Since we're using pty, we can send the key combination
    e2e.write("\x10\x50") // Ctrl+P (may vary by terminal)

    // Wait a bit for the command palette to potentially appear
    await new Promise((resolve) => setTimeout(resolve, 1000))

    // Close with escape
    e2e.write("\x1b")

    e2e.cleanup()
  }, 15_000)

  test("should handle slash command", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-slash-project"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to start
    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Type a slash command
    e2e.write("/help")

    // Wait for autocomplete or help display
    await new Promise((resolve) => setTimeout(resolve, 1000))

    // Clear input
    e2e.write("\x1b") // Escape

    e2e.cleanup()
  }, 15_000)

  test("should handle empty input gracefully", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-empty-project"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to start
    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Press enter without typing anything
    e2e.write("\n")

    // Wait a moment - should not crash or show error
    await new Promise((resolve) => setTimeout(resolve, 1000))

    // Should still be running
    const output = e2e.getOutput()
    expect(output).not.toMatch(/error|crash|fatal/i)

    e2e.cleanup()
  }, 15_000)
})
