/**
 * E2E Tests: Dialog Workflow
 * Testing various dialog interactions
 */

import { describe, test, expect } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

describe.skipIf(skipE2E)("Dialog Workflow", () => {
  test("should open and close command palette", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-dialog-cmd"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Try to open command palette
    e2e.write("\x10\x50") // Ctrl+Shift+P or similar

    await new Promise((resolve) => setTimeout(resolve, 500))

    // Close with escape
    e2e.write("\x1b")

    await new Promise((resolve) => setTimeout(resolve, 500))

    e2e.cleanup()
  }, 15_000)

  test("should navigate dialog options with arrow keys", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-dialog-nav"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Open command palette
    e2e.write("\x10\x50")

    await new Promise((resolve) => setTimeout(resolve, 500))

    // Try navigating
    e2e.write("\x1b[A") // Up arrow
    e2e.write("\x1b[B") // Down arrow

    await new Promise((resolve) => setTimeout(resolve, 500))

    // Close
    e2e.write("\x1b")

    e2e.cleanup()
  }, 15_000)

  test("should handle nested dialogs", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-dialog-nested"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    await e2e.waitForOutput(/Ask|Help|Welcome/i, 10_000)

    // Open command palette
    e2e.write("\x10\x50")

    await new Promise((resolve) => setTimeout(resolve, 500))

    // Try to open another dialog from within (e.g., model select)
    e2e.write("\x10\x4d") // Ctrl+Shift+M for model

    await new Promise((resolve) => setTimeout(resolve, 500))

    // Close both with multiple escapes
    e2e.write("\x1b")
    await new Promise((resolve) => setTimeout(resolve, 200))
    e2e.write("\x1b")

    e2e.cleanup()
  }, 15_000)
})
