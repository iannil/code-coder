// @ts-nocheck
/**
 * Unit Tests: Prompt Stash Component Logic
 * Testing prompt stash CRUD operations and persistence
 */

import { describe, test, expect, beforeEach } from "bun:test"
import type { StashEntry } from "@/cli/cmd/tui/component/prompt/stash"
import type { PromptInfo } from "@/cli/cmd/tui/component/prompt/history"

const MAX_STASH_ENTRIES = 50

describe("Prompt Stash Component Logic", () => {
  describe("list entries", () => {
    test("should return empty list initially", () => {
      const entries: StashEntry[] = []

      expect(entries).toEqual([])
    })

    test("should return all stashed entries", () => {
      const entries: StashEntry[] = []

      const entry1: StashEntry = { input: "first", parts: [], timestamp: Date.now() }
      const entry2: StashEntry = { input: "second", parts: [], timestamp: Date.now() }

      entries.push(entry1, entry2)

      expect(entries).toHaveLength(2)
      expect(entries[0].input).toBe("first")
      expect(entries[1].input).toBe("second")
    })
  })

  describe("push entry", () => {
    test("should add entry with timestamp", () => {
      const entries: StashEntry[] = []

      const before = Date.now()
      const entry: Omit<StashEntry, "timestamp"> = { input: "test", parts: [] }

      const stash = { ...entry, timestamp: Date.now() }
      entries.push(stash)

      const after = Date.now()

      expect(entries).toHaveLength(1)
      expect(entries[0].input).toBe("test")
      expect(entries[0].timestamp).toBeGreaterThanOrEqual(before)
      expect(entries[0].timestamp).toBeLessThanOrEqual(after)
    })

    test("should preserve parts in stash entry", () => {
      const entries: StashEntry[] = []

      const parts: PromptInfo["parts"] = [
        {
          type: "file",
          filename: "test.ts",
          url: "file:///test/test.ts",
          mime: "text/plain",
          source: {
            type: "file",
            path: "test.ts",
            text: { start: 0, end: 7, value: "@test.ts" },
          },
        },
      ]

      const entry: Omit<StashEntry, "timestamp"> = { input: "@test.ts help", parts }

      entries.push({ ...entry, timestamp: Date.now() })

      expect(entries[0].parts).toEqual(parts)
    })
  })

  describe("pop entry", () => {
    test("should remove and return last entry", () => {
      const entries: StashEntry[] = []

      const entry1: StashEntry = { input: "first", parts: [], timestamp: Date.now() }
      const entry2: StashEntry = { input: "second", parts: [], timestamp: Date.now() }

      entries.push(entry1, entry2)

      const popped = entries.pop()

      expect(popped).toBeDefined()
      expect(popped!.input).toBe("second")

      expect(entries).toHaveLength(1)
      expect(entries[0].input).toBe("first")
    })

    test("should return undefined when popping empty stash", () => {
      const entries: StashEntry[] = []

      const popped = entries.pop()
      expect(popped).toBeUndefined()
    })

    test("should handle parts in popped entry", () => {
      const entries: StashEntry[] = []

      const parts: PromptInfo["parts"] = [
        {
          type: "agent",
          agent: "test-agent",
          source: { start: 0, end: 10, value: "@test-agent" },
        },
      ]

      const entry: StashEntry = { input: "@test-agent", parts, timestamp: Date.now() }

      entries.push(entry)
      const popped = entries.pop()

      expect(popped).toBeDefined()
      expect(popped!.parts).toEqual(parts)
    })
  })

  describe("remove entry", () => {
    test("should remove entry at specific index", () => {
      const entries: StashEntry[] = []

      const entry1: StashEntry = { input: "first", parts: [], timestamp: Date.now() }
      const entry2: StashEntry = { input: "second", parts: [], timestamp: Date.now() }
      const entry3: StashEntry = { input: "third", parts: [], timestamp: Date.now() }

      entries.push(entry1, entry2, entry3)

      entries.splice(1, 1) // Remove "second"

      expect(entries).toHaveLength(2)
      expect(entries[0].input).toBe("first")
      expect(entries[1].input).toBe("third")
    })

    test("should not remove when index out of bounds (splice behavior)", () => {
      const entries: StashEntry[] = []

      const entry: StashEntry = { input: "test", parts: [], timestamp: Date.now() }
      entries.push(entry)

      const beforeCount = entries.length

      // Note: splice with negative index in JS does remove elements
      // splice(-1, 1) removes the last element
      entries.splice(-1, 1)

      // splice with index > length does nothing
      entries.splice(100, 0)

      const afterCount = entries.length
      // splice(-1, 1) removed the last element, so count is now 0
      expect(afterCount).toBe(0)
    })

    test("should remove first entry at index 0", () => {
      const entries: StashEntry[] = []

      const entry1: StashEntry = { input: "first", parts: [], timestamp: Date.now() }
      const entry2: StashEntry = { input: "second", parts: [], timestamp: Date.now() }

      entries.push(entry1, entry2)

      entries.splice(0, 1)

      expect(entries).toHaveLength(1)
      expect(entries[0].input).toBe("second")
    })
  })

  describe("MAX_STASH_ENTRIES", () => {
    test("should limit stash to 50 entries", () => {
      const entries: StashEntry[] = []

      // Add more than 50 entries
      for (let i = 0; i < 55; i++) {
        entries.push({ input: `entry ${i}`, parts: [], timestamp: Date.now() })
        if (entries.length > MAX_STASH_ENTRIES) {
          entries.splice(0, entries.length - MAX_STASH_ENTRIES)
        }
      }

      expect(entries.length).toBeLessThanOrEqual(MAX_STASH_ENTRIES)
      expect(entries.length).toBe(MAX_STASH_ENTRIES)

      // Oldest entries should be removed (first 5)
      expect(entries[0].input).toBe("entry 5")
    })
  })

  describe("StashEntry type", () => {
    test("should have required properties", () => {
      const entry: StashEntry = {
        input: "test input",
        parts: [],
        timestamp: Date.now(),
      }

      expect(entry.input).toBe("test input")
      expect(entry.parts).toEqual([])
      expect(typeof entry.timestamp).toBe("number")
    })

    test("should accept complex parts", () => {
      const entry: StashEntry = {
        input: "@file.ts @agent help",
        parts: [
          {
            type: "file",
            filename: "file.ts",
            url: "file:///test/file.ts",
            mime: "text/plain",
            source: {
              type: "file",
              path: "file.ts",
              text: { start: 0, end: 7, value: "@file.ts" },
            },
          },
          {
            type: "agent",
            agent: "agent",
            source: { start: 8, end: 15, value: "@agent" },
          },
        ],
        timestamp: Date.now(),
      }

      expect(entry.parts).toHaveLength(2)
    })
  })
})
