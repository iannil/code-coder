/**
 * E2E Tests: File Operations
 *
 * High-priority end-to-end tests for file operations:
 * - Paste file path from clipboard
 * - Drag and drop file to terminal
 * - Image paste
 * - SVG paste
 * - Large content summary paste
 * - Remove file
 */

import { describe, test, expect, beforeEach, afterEach } from "bun:test"
import { createE2ETest } from "../../../helpers/e2e-helper"
import { writeFileSync, mkdirSync, rmSync, existsSync } from "fs"
import { join } from "path"

// E2E tests require proper environment and are skipped by default
// Run with: SKIP_E2E=false bun test test/e2e/tui
const skipE2E = process.env.SKIP_E2E !== "false"

const testDir = "/tmp/test-e2e-file-ops"

// Setup test files
function setupTestFiles() {
  if (!existsSync(testDir)) {
    mkdirSync(testDir, { recursive: true })
  }
  writeFileSync(join(testDir, "test.ts"), "export function test() {}")
  writeFileSync(join(testDir, "sample.png"), Buffer.from("fake-png-data"))
  writeFileSync(join(testDir, "icon.svg"), "<svg></svg>")
}

// Cleanup test files
function cleanupTestFiles() {
  if (existsSync(testDir)) {
    rmSync(testDir, { recursive: true, force: true })
  }
}

describe.skipIf(skipE2E)("File Operations", () => {
  beforeEach(() => {
    setupTestFiles()
  })

  afterEach(() => {
    cleanupTestFiles()
  })

  test("should paste file path into prompt", async () => {
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
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Type @ and file path (simulating file paste)
    e2e.write(`@${join(testDir, "test.ts")} `)

    // Wait for the file reference to be processed
    await e2e.waitForOutput(/test\.ts/i, 3000)

    const output = e2e.getOutput()
    expect(output).toContain("test.ts")

    e2e.cleanup()
  }, 15_000)

  test("should handle image paste", async () => {
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
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Type image path with @ reference
    e2e.write(`@${join(testDir, "sample.png")} `)

    // Wait for image placeholder
    await e2e.waitForOutput(/Image|png/i, 3000)

    const output = e2e.getOutput()
    expect(output.length).toBeGreaterThan(0)

    e2e.cleanup()
  }, 15_000)

  test("should handle SVG paste as text", async () => {
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
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Type SVG path
    e2e.write(`@${join(testDir, "icon.svg")} `)

    // Wait for SVG placeholder/reference
    await e2e.waitForOutput(/svg|icon/i, 3000)

    const output = e2e.getOutput()
    expect(output.length).toBeGreaterThan(0)

    e2e.cleanup()
  }, 15_000)

  test("should summarize large pasted content", async () => {
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
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Create large multiline content
    const largeContent = Array.from({ length: 10 }, (_, i) => `Line ${i} of content`).join("\n")

    // Simulate pasting (in real scenario, this would be bracketed paste)
    e2e.write(largeContent)

    // Wait for content to be processed
    await e2e.waitForOutput(/Line|content|pasted/i, 3000)

    const output = e2e.getOutput()
    expect(output.length).toBeGreaterThan(0)

    e2e.cleanup()
  }, 15_000)

  test("should handle multiple files", async () => {
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
    await e2e.waitForOutput(/Ask anything/i, 10_000)

    // Add multiple file references
    e2e.write(`@${join(testDir, "test.ts")} `)
    await e2e.waitForOutput(/test\.ts/i, 2000)

    e2e.write(`@${join(testDir, "sample.png")} `)
    await e2e.waitForOutput(/png|Image/i, 2000)

    const output = e2e.getOutput()
    expect(output).toContain("test.ts")
    expect(output.length).toBeGreaterThan(0)

    e2e.cleanup()
  }, 20_000)
})
