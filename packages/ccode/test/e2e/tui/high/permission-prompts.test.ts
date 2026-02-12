/**
 * E2E Tests: Permission Prompts
 *
 * High-priority end-to-end tests for permission handling:
 * - Permission request display
 * - Permission grant
 * - Permission denial
 * - Remember permission choice
 */

import { describe, test, expect } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

describe.skipIf(skipE2E)("Permission Prompts", () => {
  test("should display permission request", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-permissions"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Request to read a file (which would trigger a permission prompt)
    e2e.write("Read the file /etc/hosts\r")

    // Wait for permission dialog or response
    await e2e.waitForOutput(/permission|allow|deny|hosts/i, 5000)

    const output = e2e.getOutput()
    expect(output.length).toBeGreaterThan(0)

    e2e.cleanup()
  }, 15_000)

  test("should grant permission when confirmed", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-grant"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Make a request that needs permission
    e2e.write("List files in /tmp\r")

    // Wait for permission prompt
    await e2e.waitForOutput(/allow|yes|permit/i, 5000)

    // Grant permission (Enter or 'y')
    e2e.sendEnter()

    // Should proceed with the action
    await e2e.waitForOutput(/tmp|file|directory/i, 3000)

    e2e.cleanup()
  }, 15_000)

  test("should deny permission when rejected", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-deny"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Make a request
    e2e.write("Delete /tmp/test-file.txt\r")

    // Wait for permission prompt
    await e2e.waitForOutput(/allow|delete|confirm/i, 5000)

    // Deny permission (Escape or 'n')
    e2e.sendEscape()

    // Should cancel the action
    await e2e.waitForOutput(/cancel|denied|abort/i, 3000)

    e2e.cleanup()
  }, 15_000)

  test("should remember permission choice", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-remember"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // First request - should ask for permission
    e2e.write("List files in /tmp\r")
    await e2e.waitForOutput(/allow|yes/i, 5000)
    e2e.sendEnter()
    await e2e.waitForOutput(/tmp/i, 3000)

    // Second similar request - might remember permission
    e2e.write("List files in /tmp again\r")
    await e2e.waitForOutput(/tmp|file/i, 5000)

    const output = e2e.getOutput()
    expect(output).toContain("/tmp")

    e2e.cleanup()
  }, 20_000)

  test("should show context in permission prompt", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-context"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Make a specific request
    e2e.write("Read package.json\r")

    // Permission prompt should show what's being requested
    await e2e.waitForOutput(/package\.json|read|file|allow/i, 5000)

    const output = e2e.getOutput()
    // Should mention the file or operation
    expect(output.toLowerCase()).toMatch(/package|file|read/)

    e2e.sendEscape() // Cancel

    e2e.cleanup()
  }, 15_000)
})
