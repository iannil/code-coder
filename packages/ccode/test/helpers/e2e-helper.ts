/**
 * E2E Test Helper
 *
 * Provides utilities for end-to-end testing of the TUI using bun-pty.
 * This helper wraps the bun-pty library to provide a convenient API for TUI testing.
 */

import { spawn } from "bun-pty"
import type { IPty } from "bun-pty"
import { mkdtempSync, rmSync, writeFileSync, readFileSync, existsSync, mkdirSync, readdirSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"

export interface E2ETestOptions {
  cmd: string
  args: string[]
  env?: Record<string, string>
  cwd?: string
  rows?: number
  cols?: number
}

export interface E2ETestContext {
  pty: IPty
  write: (text: string) => void
  waitForOutput: (pattern: string | RegExp, timeout?: number) => Promise<void>
  waitForOutputAbsent: (pattern: string | RegExp, timeout?: number) => Promise<void>
  getOutput: () => string
  screenshot: (name?: string) => string
  sendKeybind: (keybind: string) => void
  pasteFile: (filePath: string) => void
  pasteImage: (imagePath: string) => void
  pasteText: (text: string) => void
  sendEscape: () => void
  sendEnter: () => void
  sendCtrl: (key: string) => void
  cleanup: () => void
}

// Screenshot directory for failed tests
const SCREENSHOT_DIR = join(process.env.TUI_SCREENSHOT_DIR ?? tmpdir(), "tui-screenshots")

// Ensure screenshot directory exists
if (!existsSync(SCREENSHOT_DIR)) {
  mkdirSync(SCREENSHOT_DIR, { recursive: true })
}

/**
 * Screenshot data structure
 */
export interface ScreenshotData {
  timestamp: number
  output: string
  rows: number
  cols: number
}

export async function createE2ETest(options: E2ETestOptions): Promise<E2ETestContext> {
  // Capture output from the PTY
  let outputBuffer = ""

  const pty = spawn(options.cmd, options.args, {
    name: "xterm",
    env: {
      ...process.env,
      // Prevent interactive mode
      CI: "true",
      // Set test API keys
      ANTHROPIC_API_KEY: "sk-test-key-for-testing",
      OPENAI_API_KEY: "sk-test-key-for-testing",
      ...options.env,
    },
    cwd: options.cwd ?? process.cwd(),
    rows: options.rows ?? 40,
    cols: options.cols ?? 120,
  })

  // Subscribe to data events to capture output
  const disposable = pty.onData((data) => {
    outputBuffer += data
  })

  /**
   * Captures terminal output as a screenshot for debugging
   * Returns the path to the saved screenshot file
   */
  function screenshot(name = `screenshot-${Date.now()}`): string {
    const data: ScreenshotData = {
      timestamp: Date.now(),
      output: outputBuffer,
      rows: pty.rows,
      cols: pty.cols,
    }
    const filePath = join(SCREENSHOT_DIR, `${name}.json`)
    writeFileSync(filePath, JSON.stringify(data, null, 2))
    return filePath
  }

  /**
   * Sends a keyboard shortcut/keybind to the terminal
   * Supports formats like: "ctrl+c", "ctrl+shift+p", "escape", "enter", "space"
   */
  function sendKeybind(keybind: string): void {
    const normalized = keybind.toLowerCase().trim()

    // Handle escape
    if (normalized === "escape" || normalized === "esc") {
      pty.write("\x1b")
      return
    }

    // Handle enter/return
    if (normalized === "enter" || normalized === "return") {
      pty.write("\r")
      return
    }

    // Handle space (leader key)
    if (normalized === "space") {
      pty.write(" ")
      return
    }

    // Handle tab
    if (normalized === "tab") {
      pty.write("\t")
      return
    }

    // Handle ctrl+key combinations
    if (normalized.startsWith("ctrl+")) {
      const key = normalized.slice(5)
      pty.write(`\x1b[${ctrlKeyCode(key)}~`)
      return
    }

    // Handle ctrl+shift+key combinations
    if (normalized.startsWith("ctrl+shift+")) {
      const key = normalized.slice(11)
      // Send ctrl + shifted key
      pty.write(`\x1b[${ctrlKeyCode(key)}~`)
      return
    }

    // Default: just write the key
    pty.write(keybind)
  }

  /**
   * Helper for Ctrl key sequences
   */
  function ctrlKeyCode(key: string): number {
    const codes: Record<string, number> = {
      a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7, h: 8, i: 9, j: 10,
      k: 11, l: 12, m: 13, n: 14, o: 15, p: 16, q: 17, r: 18, s: 19,
      t: 20, u: 21, v: 22, w: 23, x: 24, y: 25, z: 26,
    }
    return codes[key.toLowerCase()] ?? 0
  }

  /**
   * Simulates pasting a file path into the terminal
   */
  function pasteFile(filePath: string): void {
    // Files are pasted as @<path> in the TUI
    pty.write(`@${filePath}`)
  }

  /**
   * Simulates pasting an image path into the terminal
   * Images use OSC 1337 escape sequence for iTerm2/Kitty
   */
  function pasteImage(imagePath: string): void {
    // Using iTerm2 image protocol
    const base64 = readFileSync(imagePath, "base64")
    pty.write(`\x1b]1337;File=inline=1:${base64}\x07`)
  }

  /**
   * Simulates pasting text into the terminal
   */
  function pasteText(text: string): void {
    pty.write(text)
  }

  /**
   * Sends Escape key
   */
  function sendEscape(): void {
    pty.write("\x1b")
  }

  /**
   * Sends Enter key
   */
  function sendEnter(): void {
    pty.write("\r")
  }

  /**
   * Sends Ctrl+key combination
   */
  function sendCtrl(key: string): void {
    const code = ctrlKeyCode(key)
    if (code > 0) {
      pty.write(String.fromCharCode(code))
    }
  }

  return {
    pty,

    write(text: string) {
      pty.write(text)
    },

    async waitForOutput(pattern: string | RegExp, timeout = 5000) {
      const start = Date.now()
      const interval = 50

      while (Date.now() - start < timeout) {
        const output = outputBuffer
        if (pattern instanceof RegExp ? pattern.test(output) : output.includes(pattern)) {
          return
        }
        await new Promise((resolve) => setTimeout(resolve, interval))
      }

      throw new Error(
        `Timeout waiting for pattern: ${pattern instanceof RegExp ? pattern.source : pattern}\n` +
          `Current output:\n${outputBuffer.slice(-500)}`,
      )
    },

    async waitForOutputAbsent(pattern: string | RegExp, timeout = 5000) {
      const start = Date.now()
      const interval = 50

      while (Date.now() - start < timeout) {
        const output = outputBuffer
        if (!(pattern instanceof RegExp ? pattern.test(output) : output.includes(pattern))) {
          return
        }
        await new Promise((resolve) => setTimeout(resolve, interval))
      }

      throw new Error(
        `Timeout waiting for pattern to be absent: ${pattern instanceof RegExp ? pattern.source : pattern}`,
      )
    },

    getOutput() {
      return outputBuffer
    },

    screenshot,
    sendKeybind,
    pasteFile,
    pasteImage,
    pasteText,
    sendEscape,
    sendEnter,
    sendCtrl,

    cleanup() {
      disposable.dispose()
      pty.kill()
    },
  }
}

/**
 * Create a mock file system entry for testing
 */
export function createMockFile(path: string, content: string): { path: string; content: string } {
  return { path, content }
}

/**
 * Create a mock session for testing
 */
export function createMockSession(overrides: Partial<SessionData> = {}): SessionData {
  return {
    id: "test-session-1",
    title: "Test Session",
    createdAt: Date.now(),
    updatedAt: Date.now(),
    messages: [],
    ...overrides,
  }
}

export interface SessionData {
  id: string
  title: string
  createdAt: number
  updatedAt: number
  messages: unknown[]
}

/**
 * Wait for a condition to be true
 */
export async function waitFor(
  condition: () => boolean,
  options: { timeout?: number; interval?: number } = {},
): Promise<void> {
  const { timeout = 5000, interval = 50 } = options
  const start = Date.now()

  while (Date.now() - start < timeout) {
    if (condition()) return
    await new Promise((resolve) => setTimeout(resolve, interval))
  }

  throw new Error(`waitFor timed out after ${timeout}ms`)
}

/**
 * Load a screenshot from disk
 */
export function loadScreenshot(filePath: string): ScreenshotData | null {
  try {
    if (!existsSync(filePath)) return null
    const content = readFileSync(filePath, "utf-8")
    return JSON.parse(content) as ScreenshotData
  } catch {
    return null
  }
}

/**
 * Clean up old screenshots
 */
export function cleanupScreenshots(olderThanMs: number = 24 * 60 * 60 * 1000): void {
  const now = Date.now()
  for (const file of readdirSync(SCREENSHOT_DIR)) {
    if (file.endsWith(".json")) {
      const filePath = join(SCREENSHOT_DIR, file)
      const data = loadScreenshot(filePath)
      if (data && now - data.timestamp > olderThanMs) {
        rmSync(filePath)
      }
    }
  }
}
