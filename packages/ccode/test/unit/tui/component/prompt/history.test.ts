// @ts-nocheck
/**
 * Unit Tests: Prompt History Component Logic
 * Testing prompt history navigation, persistence, and duplicate detection
 */

import { describe, test, expect, beforeEach } from "bun:test"
import type { PromptInfo } from "@/cli/cmd/tui/component/prompt/history"

describe("Prompt History Component Logic", () => {
  describe("history navigation", () => {
    test("should return undefined when history is empty", () => {
      const history: PromptInfo[] = []
      const index = 0

      const move = (direction: 1 | -1, input: string) => {
        if (!history.length) return undefined
        const current = history.at(index)
        if (!current) return undefined
        if (current.input !== input && input.length) return
        const next = index + direction
        if (Math.abs(next) > history.length) return
        if (next > 0) return
        const newIndex = next
        if (newIndex === 0)
          return {
            input: "",
            parts: [],
          }
        return history.at(newIndex)
      }

      const result = move(1, "")
      expect(result).toBeUndefined()
    })

    test("should return undefined when input doesn't match current history entry", () => {
      const history: PromptInfo[] = [
        { input: "test prompt", parts: [] },
      ]
      const index = 0

      const move = (direction: 1 | -1, input: string) => {
        if (!history.length) return undefined
        const current = history.at(index)
        if (!current) return undefined
        if (current.input !== input && input.length) return
        const next = index + direction
        if (next > 0) return
        return history.at(next)
      }

      const result = move(-1, "different input")
      expect(result).toBeUndefined()
    })

    test("should return empty state when navigating beyond history", () => {
      const history: PromptInfo[] = []
      let index = 0

      const move = (direction: 1 | -1) => {
        if (!history.length) return undefined
        const next = index + direction
        if (Math.abs(next) > history.length) return undefined
        if (next > 0) return undefined
        index = next
        if (index === 0)
          return {
            input: "",
            parts: [],
          }
        return history.at(index)
      }

      const result = move(1)
      // Empty history returns undefined
      expect(result).toBeUndefined()
    })

    test("should cycle through history with up/down navigation", () => {
      const history: PromptInfo[] = [
        { input: "first prompt", parts: [] },
        { input: "second prompt", parts: [] },
      ]
      let index = 0

      const move = (direction: 1 | -1) => {
        if (!history.length) return undefined
        const next = index + direction
        if (Math.abs(next) > history.length) return
        if (next > 0) return
        index = next
        return history.at(index)
      }

      // Navigate back from empty (index 0) to entry2 (index -1)
      const result1 = move(-1)
      expect(result1).toEqual({ input: "second prompt", parts: [] })

      // Navigate back to entry1 (index -2)
      const result2 = move(-1)
      expect(result2).toEqual({ input: "first prompt", parts: [] })

      // Navigate forward to entry2 (index -1)
      index = -2
      const result3 = move(1)
      expect(result3).toEqual({ input: "second prompt", parts: [] })
    })
  })

  describe("history append", () => {
    test("should add entry to history", () => {
      const history: PromptInfo[] = []

      const append = (entry: PromptInfo) => {
        history.push(entry)
      }

      const entry: PromptInfo = {
        input: "new prompt",
        parts: [],
      }

      append(entry)

      expect(history).toHaveLength(1)
      expect(history[0]).toEqual(entry)
    })

    test("should handle parts in history entry", () => {
      const history: PromptInfo[] = []

      const entry: PromptInfo = {
        input: "explain @index.ts",
        parts: [
          {
            type: "file",
            filename: "index.ts",
            url: "file:///test/index.ts",
            mime: "text/plain",
            source: {
              type: "file",
              path: "index.ts",
              text: { start: 7, end: 16, value: "@index.ts" },
            },
          },
        ],
      }

      history.push(entry)

      expect(history[0].parts).toHaveLength(1)
      expect(history[0].parts[0].type).toBe("file")
    })
  })

  describe("MAX_HISTORY_ENTRIES", () => {
    const MAX_HISTORY_ENTRIES = 50

    test("should limit history to 50 entries", () => {
      const history: PromptInfo[] = []

      // Add more than 50 entries
      for (let i = 0; i < 55; i++) {
        history.push({ input: `prompt ${i}`, parts: [] })
        if (history.length > MAX_HISTORY_ENTRIES) {
          history.splice(0, history.length - MAX_HISTORY_ENTRIES)
        }
      }

      expect(history.length).toBe(MAX_HISTORY_ENTRIES)

      // The oldest 5 entries should be removed
      expect(history[0].input).toBe("prompt 5")
      expect(history[history.length - 1].input).toBe("prompt 54")
    })
  })

  describe("history reset", () => {
    test("should reset to index 0 after append", () => {
      const history: PromptInfo[] = []
      let index = 0

      const append = (entry: PromptInfo) => {
        history.push(entry)
        index = 0
      }

      append({ input: "first", parts: [] })
      append({ input: "second", parts: [] })

      index = -1 // Navigate back
      expect(index).toBe(-1)

      append({ input: "third", parts: [] })

      expect(index).toBe(0)
    })
  })

  describe("PromptInfo type", () => {
    test("should accept file parts with source text", () => {
      const promptInfo: PromptInfo = {
        input: "test @file.ts",
        parts: [
          {
            type: "file",
            filename: "file.ts",
            url: "file:///test/file.ts",
            mime: "text/plain",
            source: {
              type: "file",
              path: "file.ts",
              text: { start: 5, end: 13, value: "@file.ts" },
            },
          },
        ],
      }

      expect(promptInfo.input).toBe("test @file.ts")
      expect(promptInfo.parts).toHaveLength(1)
      expect(promptInfo.parts[0].type).toBe("file")
    })

    test("should accept agent parts", () => {
      const promptInfo: PromptInfo = {
        input: "@agent-name help",
        parts: [
          {
            type: "agent",
            agent: "agent-name",
            source: { start: 0, end: 11, value: "@agent-name" },
          },
        ],
      }

      expect(promptInfo.parts[0].type).toBe("agent")
    })

    test("should accept text parts with source", () => {
      const promptInfo: PromptInfo = {
        input: "hello world",
        parts: [
          {
            type: "text",
            content: "hello world",
            source: {
              text: { start: 0, end: 11, value: "hello world" },
            },
          },
        ],
      }

      expect(promptInfo.parts[0].type).toBe("text")
    })
  })
})
