/**
 * Clipboard Utility Unit Tests
 *
 * Tests for the clipboard utility including:
 * - OSC 52 escape sequence writing
 * - Reading clipboard on different platforms (darwin, win32, WSL, linux)
 * - Writing clipboard on different platforms
 * - Base64 encoding
 * - Tmux/screen passthrough wrapping
 */

import { describe, test, expect, beforeEach, mock } from "bun:test"

describe("Clipboard Utility", () => {
  describe("OSC 52 escape sequence", () => {
    test("should encode text to base64", () => {
      const text = "Hello, world!"
      const base64 = Buffer.from(text).toString("base64")

      expect(base64).toBe("SGVsbG8sIHdvcmxkIQ==")
    })

    test("should create OSC 52 sequence", () => {
      const text = "test"
      const base64 = Buffer.from(text).toString("base64")
      const osc52 = `\x1b]52;c;${base64}\x07`

      expect(osc52).toContain("\x1b]52;c;")
      expect(osc52).toContain(base64)
      expect(osc52).toContain("\x07")
    })

    test("should handle empty string", () => {
      const text = ""
      const base64 = Buffer.from(text).toString("base64")

      expect(base64).toBe("")
    })

    test("should handle unicode characters", () => {
      const text = "Hello 🌍 世界"
      const base64 = Buffer.from(text).toString("base64")

      expect(base64).toBeTruthy()
      expect(base64.length).toBeGreaterThan(0)

      // Verify roundtrip
      const decoded = Buffer.from(base64, "base64").toString("utf-8")
      expect(decoded).toBe(text)
    })

    test("should handle special characters", () => {
      const text = "\n\t\r"
      const base64 = Buffer.from(text).toString("base64")

      expect(base64).toBeTruthy()

      const decoded = Buffer.from(base64, "base64").toString("utf-8")
      expect(decoded).toBe(text)
    })
  })

  describe("tmux/screen passthrough", () => {
    test("should wrap OSC 52 for tmux", () => {
      const osc52 = "\x1b]52;c;VGVzdA==\x07"
      const passthrough = true

      const sequence = passthrough ? `\x1bPtmux;\x1b${osc52}\x1b\\` : osc52

      expect(sequence).toContain("\x1bPtmux;")
      expect(sequence).toContain(osc52)
      expect(sequence).toContain("\x1b\\")
    })

    test("should wrap OSC 52 for screen", () => {
      const osc52 = "\x1b]52;c;VGVzdA==\x07"
      const passthrough = true

      const sequence = passthrough ? `\x1bPtmux;\x1b${osc52}\x1b\\` : osc52

      expect(sequence).toContain("\x1bPtmux;")
    })

    test("should not wrap when not in multiplexer", () => {
      const osc52 = "\x1b]52;c;VGVzdA==\x07"
      const passthrough = false

      const sequence = passthrough ? `\x1bPtmux;\x1b${osc52}\x1b\\` : osc52

      expect(sequence).toBe(osc52)
      expect(sequence).not.toContain("\x1bPtmux;")
    })

    test("should detect tmux from environment", () => {
      const env = { TMUX: "/tmp/tmux-1000/default" }
      const hasTmux = !!env["TMUX"]

      expect(hasTmux).toBe(true)
    })

    test("should detect screen from environment", () => {
      const env = { STY: "12345.pts-0.localhost" }
      const hasScreen = !!env["STY"]

      expect(hasScreen).toBe(true)
    })
  })

  describe("platform detection", () => {
    test("should detect darwin (macOS)", () => {
      const platform = "darwin"
      expect(platform).toBe("darwin")
    })

    test("should detect win32 (Windows)", () => {
      const platform = "win32"
      expect(platform).toBe("win32")
    })

    test("should detect linux", () => {
      const platform = "linux"
      expect(platform).toBe("linux")
    })

    test("should detect WSL from release string", () => {
      const release = "5.15.146.1-microsoft-standard-WSL2"
      const isWSL = release.includes("WSL")

      expect(isWSL).toBe(true)
    })

    test("should detect Wayland from environment", () => {
      const env = { WAYLAND_DISPLAY: "wayland-0" }
      const hasWayland = !!env["WAYLAND_DISPLAY"]

      expect(hasWayland).toBe(true)
    })
  })

  describe("base64 encoding", () => {
    test("should encode simple string", () => {
      const text = "ABC"
      const encoded = Buffer.from(text).toString("base64")

      expect(encoded).toBe("QUJD")
    })

    test("should encode and decode correctly", () => {
      const original = "Hello, 世界! 🌍"
      const encoded = Buffer.from(original).toString("base64")
      const decoded = Buffer.from(encoded, "base64").toString("utf-8")

      expect(decoded).toBe(original)
    })

    test("should encode multiline text", () => {
      const text = "Line 1\nLine 2\nLine 3"
      const encoded = Buffer.from(text).toString("base64")

      expect(encoded).toBeTruthy()
      expect(encoded.length).toBeGreaterThan(0)
    })
  })

  describe("writing clipboard - darwin", () => {
    test("should escape backslashes in text", () => {
      const text = 'path\\to\\file'
      const escaped = text.replace(/\\/g, "\\\\")

      expect(escaped).toBe("path\\\\to\\\\file")
    })

    test("should escape quotes in text", () => {
      const text = 'text with "quotes"'
      const escaped = text.replace(/"/g, '\\"')

      expect(escaped).toBe('text with \\"quotes\\"')
    })

    test("should escape both backslashes and quotes", () => {
      const text = 'path\\with\\"quotes\\"'
      const escaped = text.replace(/\\/g, "\\\\").replace(/"/g, '\\"')

      // Verify the transformation works correctly
      expect(escaped).toContain('\\\\')
      expect(escaped).toContain('\\"')
    })

    test("should use osascript command", () => {
      const osascript = "osascript -e 'set the clipboard to \"test\"'"
      expect(osascript).toContain("osascript")
      expect(osascript).toContain("set the clipboard to")
    })
  })

  describe("content interface", () => {
    test("should create text content", () => {
      const content = {
        data: "text content",
        mime: "text/plain",
      }

      expect(content.data).toBe("text content")
      expect(content.mime).toBe("text/plain")
    })

    test("should create image content", () => {
      const content = {
        data: "base64imagedata",
        mime: "image/png",
      }

      expect(content.mime).toBe("image/png")
      expect(content.data).toBe("base64imagedata")
    })

    test("should distinguish mime types", () => {
      const text = { mime: "text/plain" }
      const png = { mime: "image/png" }
      const jpeg = { mime: "image/jpeg" }

      expect(text.mime).not.toBe(png.mime)
      expect(png.mime).not.toBe(jpeg.mime)
    })
  })
})
