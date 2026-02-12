/**
 * Visual Regression Tests: Dialogs
 *
 * Visual tests for dialog rendering:
 * - Model selection dialog
 * - Theme selection dialog
 * - MCP configuration dialog
 * - Command palette
 */

import { describe, test, expect } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"
import { assertVisual, updateVisualBaseline } from "../../../helpers/visual-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

// Set UPDATE_BASELINE=true to update baselines instead of testing
const UPDATE_BASELINE = process.env.UPDATE_BASELINE === "true"

describe.skipIf(skipE2E)("Visual: Dialogs", () => {
  test("should render model selection dialog", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-model-dialog"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
        TERM: "xterm-256color",
        COLUMNS: "120",
        LINES: "40",
      },
      rows: 40,
      cols: 120,
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open model selection dialog
    e2e.sendKeybind("ctrl+shift+m")
    await e2e.waitForOutput(/model|claude|gpt|anthropic/i, 3000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "dialog-model-select", {
        description: "Model selection dialog",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "dialog-model-select", {
        description: "Model selection dialog",
        rows: 40,
        cols: 120,
      })
    }

    e2e.sendEscape() // Close dialog
    e2e.cleanup()
  }, 15_000)

  test("should render theme selection dialog", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-theme-dialog"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
        TERM: "xterm-256color",
        COLUMNS: "120",
        LINES: "40",
      },
      rows: 40,
      cols: 120,
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open theme dialog via command palette
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command/i, 2000)

    e2e.write("theme\r")
    await e2e.waitForOutput(/theme|dark|light|nord/i, 3000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "dialog-theme-select", {
        description: "Theme selection dialog",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "dialog-theme-select", {
        description: "Theme selection dialog",
        rows: 40,
        cols: 120,
      })
    }

    e2e.sendEscape() // Close dialog
    e2e.cleanup()
  }, 15_000)

  test("should render MCP configuration dialog", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-mcp-dialog"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
        TERM: "xterm-256color",
        COLUMNS: "120",
        LINES: "40",
      },
      rows: 40,
      cols: 120,
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open MCP dialog via command palette
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command/i, 2000)

    e2e.write("mcp\r")
    await e2e.waitForOutput(/mcp|server|connect/i, 3000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "dialog-mcp-config", {
        description: "MCP configuration dialog",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "dialog-mcp-config", {
        description: "MCP configuration dialog",
        rows: 40,
        cols: 120,
      })
    }

    e2e.sendEscape() // Close dialog
    e2e.cleanup()
  }, 15_000)

  test("should render command palette", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-command-palette"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
        TERM: "xterm-256color",
        COLUMNS: "120",
        LINES: "40",
      },
      rows: 40,
      cols: 120,
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open command palette
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command|palette|new|theme/i, 2000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "dialog-command-palette", {
        description: "Command palette with suggestions",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "dialog-command-palette", {
        description: "Command palette with suggestions",
        rows: 40,
        cols: 120,
      })
    }

    e2e.sendEscape() // Close palette
    e2e.cleanup()
  }, 15_000)

  test("should render command palette with filter", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-palette-filter"],
      cwd: "/Users/iannil/Code/zproducts/code-coder/packages/ccode",
      env: {
        ANTHROPIC_API_KEY: "sk-test-key-for-testing",
        NODE_ENV: "test",
        TERM: "xterm-256color",
        COLUMNS: "120",
        LINES: "40",
      },
      rows: 40,
      cols: 120,
    })

    // Wait for TUI to initialize
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Open command palette
    e2e.sendKeybind("ctrl+shift+p")
    await e2e.waitForOutput(/command/i, 2000)

    // Type filter text
    e2e.write("session")
    await e2e.waitForOutput(/session/i, 1000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "dialog-palette-filtered", {
        description: "Command palette with filtered results",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "dialog-palette-filtered", {
        description: "Command palette with filtered results",
        rows: 40,
        cols: 120,
      })
    }

    e2e.sendEscape() // Close palette
    e2e.cleanup()
  }, 15_000)
})
