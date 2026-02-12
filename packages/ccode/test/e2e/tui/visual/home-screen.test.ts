/**
 * Visual Regression Tests: Home Screen
 *
 * Visual tests for home screen rendering:
 * - Home screen layout
 * - MCP connection status indicator
 * - First visit tips display
 * - Prompt placeholder display
 */

import { describe, test, expect, beforeAll } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"
import { assertVisual, updateVisualBaseline } from "../../../helpers/visual-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

// Set UPDATE_BASELINE=true to update baselines instead of testing
const UPDATE_BASELINE = process.env.UPDATE_BASELINE === "true"

describe.skipIf(skipE2E)("Visual: Home Screen", () => {
  beforeAll(() => {
    // Ensure clean environment
  })

  test("should render home screen correctly", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-home"],
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

    // Wait for TUI to fully initialize
    await e2e.waitForOutput(/Ask anything|How can I help/i, 10_000)

    // Get the terminal output
    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "home-screen", {
        description: "Home screen with prompt and tips",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "home-screen", {
        description: "Home screen with prompt and tips",
        rows: 40,
        cols: 120,
      })
    }

    e2e.cleanup()
  }, 15_000)

  test("should show MCP connection status", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-mcp"],
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

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "home-screen-mcp-status", {
        description: "Home screen showing MCP connection status",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "home-screen-mcp-status", {
        description: "Home screen showing MCP connection status",
        rows: 40,
        cols: 120,
      })
    }

    e2e.cleanup()
  }, 15_000)

  test("should show placeholder on first visit", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-first-visit"],
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
    await e2e.waitForOutput(/Ask anything|TODO|tech stack/i, 10_000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "home-screen-placeholder", {
        description: "Home screen with placeholder suggestions",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "home-screen-placeholder", {
        description: "Home screen with placeholder suggestions",
        rows: 40,
        cols: 120,
      })
    }

    e2e.cleanup()
  }, 15_000)
})
