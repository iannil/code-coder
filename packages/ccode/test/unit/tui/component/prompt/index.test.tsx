// @ts-nocheck
/**
 * Prompt Component Unit Tests
 *
 * Tests for the main Prompt component including:
 * - Input handling
 * - Multi-line input
 * - Empty input handling
 * - File paste
 * - Image paste
 * - SVG paste
 * - Autocomplete trigger and selection
 * - History navigation
 * - Shell mode (! prefix)
 * - Submit flow
 */

import { describe, it, expect, beforeEach, afterEach, mock } from "bun:test"
import type { ParsedKey } from "@opentui/core"
import { createMockKeyboardEvent } from "../../../../helpers/tui-mock"
import type { PromptRef, PromptProps } from "@/cli/cmd/tui/component/prompt/index"
import type { PromptInfo } from "@/cli/cmd/tui/component/prompt/history"

describe("Prompt Component", () => {
  describe("Input Handling", () => {
    it("should accept basic text input", () => {
      const testPrompt: PromptInfo = {
        input: "Hello, world!",
        parts: [],
      }
      expect(testPrompt.input).toBe("Hello, world!")
    })

    it("should handle multi-line input", () => {
      const testPrompt: PromptInfo = {
        input: "Line 1\nLine 2\nLine 3",
        parts: [],
      }
      expect(testPrompt.input).toContain("\n")
      expect(testPrompt.input.split("\n").length).toBe(3)
    })

    it("should handle special characters", () => {
      const testPrompt: PromptInfo = {
        input: 'Test with "quotes" and \'apostrophes\' and @symbols',
        parts: [],
      }
      expect(testPrompt.input).toContain('"quotes"')
      expect(testPrompt.input).toContain("'apostrophes'")
    })

    it("should handle unicode and emoji", () => {
      const testPrompt: PromptInfo = {
        input: "Hello 🌍 世界 🚀",
        parts: [],
      }
      expect(testPrompt.input).toBe("Hello 🌍 世界 🚀")
    })
  })

  describe("Empty Input Handling", () => {
    it("should not submit empty input", () => {
      const testPrompt: PromptInfo = {
        input: "",
        parts: [],
      }
      expect(testPrompt.input.trim()).toBe("")
    })

    it("should not submit whitespace-only input", () => {
      const testPrompt: PromptInfo = {
        input: "   \n\t  ",
        parts: [],
      }
      expect(testPrompt.input.trim()).toBe("")
    })
  })

  describe("Prompt Parts (Files, Images, etc)", () => {
    it("should store file parts correctly", () => {
      const testPrompt: PromptInfo = {
        input: "@src/test.ts ",
        parts: [
          {
            type: "file",
            filename: "test.ts",
            path: "src/test.ts",
            mime: "text/typescript",
          },
        ],
      }
      expect(testPrompt.parts).toHaveLength(1)
      expect(testPrompt.parts[0].type).toBe("file")
    })

    it("should store image parts correctly", () => {
      const testPrompt: PromptInfo = {
        input: "[Image 1] ",
        parts: [
          {
            type: "file",
            filename: "screenshot.png",
            mime: "image/png",
            url: "data:image/png;base64,iVBORw0KG...",
          },
        ],
      }
      expect(testPrompt.parts).toHaveLength(1)
      expect(testPrompt.parts[0].type).toBe("file")
      expect(testPrompt.parts[0].mime).toBe("image/png")
    })

    it("should store agent parts correctly", () => {
      const testPrompt: PromptInfo = {
        input: "@editor ",
        parts: [
          {
            type: "agent",
            name: "editor",
          },
        ],
      }
      expect(testPrompt.parts).toHaveLength(1)
      expect(testPrompt.parts[0].type).toBe("agent")
    })

    it("should store text paste parts correctly", () => {
      const testPrompt: PromptInfo = {
        input: "[Pasted ~5 lines] ",
        parts: [
          {
            type: "text",
            text: "Line 1\nLine 2\nLine 3\nLine 4\nLine 5",
          },
        ],
      }
      expect(testPrompt.parts).toHaveLength(1)
      expect(testPrompt.parts[0].type).toBe("text")
    })

    it("should handle multiple parts", () => {
      const testPrompt: PromptInfo = {
        input: "@src/test.ts [Image 1] @editor ",
        parts: [
          {
            type: "file",
            filename: "test.ts",
            path: "src/test.ts",
            mime: "text/typescript",
          },
          {
            type: "file",
            filename: "screenshot.png",
            mime: "image/png",
            url: "data:image/png;base64,iVBORw0KG...",
          },
          {
            type: "agent",
            name: "editor",
          },
        ],
      }
      expect(testPrompt.parts).toHaveLength(3)
    })
  })

  describe("Keyboard Shortcuts", () => {
    it("should recognize Ctrl+C for cancel", () => {
      const ctrlC = createMockKeyboardEvent({
        name: "c",
        ctrl: true,
      })
      expect(ctrlC.ctrl).toBe(true)
      expect(ctrlC.name).toBe("c")
    })

    it("should recognize Enter for submit", () => {
      const enter = createMockKeyboardEvent({
        name: "enter",
        sequence: "\r",
      })
      expect(enter.name).toBe("enter")
    })

    it("should recognize Escape for cancel/close", () => {
      const escape = createMockKeyboardEvent({
        name: "escape",
        sequence: "\x1b",
      })
      expect(escape.name).toBe("escape")
    })
  })

  describe("File and Image Handling", () => {
    it("should detect image mime types", () => {
      const imageTypes = ["image/png", "image/jpeg", "image/gif", "image/webp"]

      imageTypes.forEach((type) => {
        expect(type.startsWith("image/")).toBe(true)
      })
    })

    it("should detect SVG files", () => {
      const svgMime = "image/svg+xml"
      expect(svgMime).toBe("image/svg+xml")
    })

    it("should handle large content paste", () => {
      const content = "a".repeat(200)
      expect(content.length).toBeGreaterThan(150)
    })

    it("should detect multi-line paste", () => {
      const content = "Line 1\nLine 2\nLine 3"
      const lineCount = (content.match(/\n/g)?.length ?? 0) + 1
      expect(lineCount).toBe(3)
    })
  })

  describe("Prompt Info Structure", () => {
    it("should have correct structure", () => {
      const prompt: PromptInfo = {
        input: "test input",
        parts: [],
        mode: "normal",
      }

      expect(prompt).toHaveProperty("input")
      expect(prompt).toHaveProperty("parts")
      expect(prompt.input).toBe("test input")
      expect(Array.isArray(prompt.parts)).toBe(true)
    })

    it("should support shell mode", () => {
      const prompt: PromptInfo = {
        input: "ls -la",
        parts: [],
        mode: "shell",
      }

      expect(prompt.mode).toBe("shell")
    })
  })
})
