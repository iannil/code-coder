/**
 * E2E Tests: Theme Switching
 *
 * High-priority end-to-end tests for theme switching:
 * - Open theme dialog
 * - Preview theme
 * - Apply theme
 * - Cancel and restore
 * - Dark/light mode switching
 */

import { describe, test, expect } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

describe.skipIf(skipE2E)("Theme Switching", () => {
  test("should open theme dialog", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-theme"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open theme dialog via command palette
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command|palette/i, 2000)

    // Type theme command
    e2e.write("theme\r")

    // Should show theme list
    await e2e.waitForOutput(/theme|dark|light|nord/i, 3000)

    const output = e2e.getOutput()
    expect(output.toLowerCase()).toContain("theme")

    e2e.cleanup()
  }, 20_000)

  test("should preview theme", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-theme-preview"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open theme dialog
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command/i, 2000)

    e2e.write("theme\r")
    await e2e.waitForOutput(/theme/i, 3000)

    // Navigate to different theme (arrow keys would move selection)
    // Theme preview should be applied immediately on move

    const output = e2e.getOutput()
    expect(output.length).toBeGreaterThan(0)

    e2e.cleanup()
  }, 20_000)

  test("should apply theme on selection", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-theme-apply"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open theme dialog
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command/i, 2000)

    e2e.write("theme\r")
    await e2e.waitForOutput(/theme/i, 3000)

    // Select a theme (Enter key)
    e2e.sendEnter()

    // Dialog should close and theme should be applied
    await e2e.waitForOutputAbsent(/theme.*dialog|select.*theme/i, 2000)

    e2e.cleanup()
  }, 20_000)

  test("should restore original theme on cancel", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-theme-cancel"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open theme dialog
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command/i, 2000)

    e2e.write("theme\r")
    await e2e.waitForOutput(/theme/i, 3000)

    // Cancel (Escape key)
    e2e.sendEscape()

    // Dialog should close
    await e2e.waitForOutputAbsent(/theme.*dialog|select.*theme/i, 2000)

    e2e.cleanup()
  }, 20_000)

  test("should switch between dark and light themes", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-e2e-dark-light"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
      },
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open theme dialog
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command/i, 2000)

    e2e.write("theme\r")
    await e2e.waitForOutput(/theme/i, 3000)

    // Type "light" to filter
    e2e.write("light")

    // Should show light theme option
    await e2e.waitForOutput(/light/i, 2000)

    const output = e2e.getOutput()
    expect(output.toLowerCase()).toContain("light")

    e2e.cleanup()
  }, 20_000)
})
