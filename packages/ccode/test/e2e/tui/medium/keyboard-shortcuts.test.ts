/**
 * E2E Tests: Keyboard Shortcuts
 * Testing keyboard shortcut responsiveness
 */

import { describe, test, expect } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

describe.skipIf(skipE2E)("Keyboard Shortcuts", () => {
  test("should respond to escape key", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-escape-key"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Send escape - should be handled gracefully
    e2e.write("\x1b")

    await new Promise((resolve) => setTimeout(resolve, 500))

    // Should still be running
    const output = e2e.getOutput()
    expect(output).not.toMatch(/error|crash|fatal/i)

    e2e.cleanup()
  }, 15_000)

  test("should respond to Ctrl+C", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-ctrl-c"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Send Ctrl+C - might cancel or prompt for quit
    e2e.write("\x03")

    await new Promise((resolve) => setTimeout(resolve, 500))

    e2e.cleanup()
  }, 15_000)

  test("should handle text input", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-text-input"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Type some text
    e2e.write("test input")

    await new Promise((resolve) => setTimeout(resolve, 500))

    // Clear the input
    e2e.write("\x15") // Ctrl+U to clear line

    await new Promise((resolve) => setTimeout(resolve, 500))

    e2e.cleanup()
  }, 15_000)

  test("should handle backspace", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-backspace"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Type and delete
    e2e.write("hello")
    e2e.write("\x7f") // Backspace
    e2e.write("\x7f")
    e2e.write("\x7f")

    await new Promise((resolve) => setTimeout(resolve, 500))

    e2e.cleanup()
  }, 15_000)
})
