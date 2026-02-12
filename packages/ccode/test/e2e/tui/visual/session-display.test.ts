/**
 * Visual Regression Tests: Session Display
 *
 * Visual tests for session message display:
 * - User message format
 * - Assistant response format
 * - Code block syntax highlighting
 * - Diff view rendering
 * - Tool call output display
 */

import { describe, test, expect } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"
import { assertVisual, updateVisualBaseline } from "../../../helpers/visual-helper"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

// Set UPDATE_BASELINE=true to update baselines instead of testing
const UPDATE_BASELINE = process.env.UPDATE_BASELINE === "true"

describe.skipIf(skipE2E)("Visual: Session Display", () => {
  test("should display user message correctly", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-user-msg"],
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

    // Send a message
    e2e.write("Hello, this is a test message\r")

    // Wait for message to be displayed
    await e2e.waitForOutput(/Hello|test|message/i, 5000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "session-user-message", {
        description: "User message in session",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "session-user-message", {
        description: "User message in session",
        rows: 40,
        cols: 120,
      })
    }

    e2e.cleanup()
  }, 15_000)

  test("should display code blocks with syntax highlighting", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-codeblock"],
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

    // Send a message asking for code
    e2e.write("Write a hello world function in TypeScript\r")

    // Wait for response
    await e2e.waitForOutput(/function|typescript|hello/i, 8000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "session-code-block", {
        description: "Code block with syntax highlighting",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "session-code-block", {
        description: "Code block with syntax highlighting",
        rows: 40,
        cols: 120,
      })
    }

    e2e.cleanup()
  }, 20_000)

  test("should display diff view correctly", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-diff"],
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

    // Request a diff (this may not work without actual files, but we test the pattern)
    e2e.write("Show the diff for changing hello to goodbye\r")

    // Wait for any response
    await e2e.waitForOutput(/hello|goodbye|diff|\+|-/i, 8000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "session-diff-view", {
        description: "Diff view with +/- indicators",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "session-diff-view", {
        description: "Diff view with +/- indicators",
        rows: 40,
        cols: 120,
      })
    }

    e2e.cleanup()
  }, 20_000)

  test("should display tool call output", async () => {
    const e2e = await createE2ETest({
      cmd: process.execPath,
      args: ["run", "./src/index.ts", "dev", "/tmp/test-visual-tool"],
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

    // Request a file operation
    e2e.write("List the files in the current directory\r")

    // Wait for tool output
    await e2e.waitForOutput(/file|directory|tool|read/i, 8000)

    const output = e2e.getOutput()

    if (UPDATE_BASELINE) {
      updateVisualBaseline(output, "session-tool-output", {
        description: "Tool call output display",
        rows: 40,
        cols: 120,
      })
    } else {
      await assertVisual(output, "session-tool-output", {
        description: "Tool call output display",
        rows: 40,
        cols: 120,
      })
    }

    e2e.cleanup()
  }, 20_000)
})
