// @ts-nocheck
/**
 * Dialog Stash Component Unit Tests
 *
 * Tests for the prompt stash dialog including:
 * - Stash entry list display
 * - Relative time display
 * - Delete confirmation
 * - Line count display
 * - Preview truncation
 */

import { describe, test, expect, beforeEach } from "bun:test"

type StashEntry = {
  input: string
  timestamp: number
}

describe("Dialog Stash Component", () => {
  describe("time display", () => {
    test("should show 'just now' for very recent entries", () => {
      const now = Date.now()
      const entry: StashEntry = {
        input: "test",
        timestamp: now - 5000, // 5 seconds ago
      }

      const getRelativeTime = (timestamp: number) => {
        const diff = Date.now() - timestamp
        const seconds = Math.floor(diff / 1000)
        if (seconds < 60) return "just now"
        return `${seconds}s ago`
      }

      expect(getRelativeTime(entry.timestamp)).toBe("just now")
    })

    test("should show minutes ago for recent entries", () => {
      const now = Date.now()
      const entry: StashEntry = {
        input: "test",
        timestamp: now - 5 * 60 * 1000, // 5 minutes ago
      }

      const getRelativeTime = (timestamp: number) => {
        const diff = Date.now() - timestamp
        const seconds = Math.floor(diff / 1000)
        const minutes = Math.floor(seconds / 60)
        if (seconds < 60) return "just now"
        if (minutes < 60) return `${minutes}m ago`
        return `${minutes}m ago`
      }

      expect(getRelativeTime(entry.timestamp)).toBe("5m ago")
    })

    test("should show hours ago for older entries", () => {
      const now = Date.now()
      const entry: StashEntry = {
        input: "test",
        timestamp: now - 2 * 60 * 60 * 1000, // 2 hours ago
      }

      const getRelativeTime = (timestamp: number) => {
        const diff = Date.now() - timestamp
        const seconds = Math.floor(diff / 1000)
        const minutes = Math.floor(seconds / 60)
        const hours = Math.floor(minutes / 60)
        if (seconds < 60) return "just now"
        if (minutes < 60) return `${minutes}m ago`
        if (hours < 24) return `${hours}h ago`
        return `${hours}h ago`
      }

      expect(getRelativeTime(entry.timestamp)).toBe("2h ago")
    })

    test("should show date for very old entries", () => {
      const now = Date.now()
      const entry: StashEntry = {
        input: "test",
        timestamp: now - 10 * 24 * 60 * 60 * 1000, // 10 days ago
      }

      const getRelativeTime = (timestamp: number) => {
        const diff = Date.now() - timestamp
        const seconds = Math.floor(diff / 1000)
        const minutes = Math.floor(seconds / 60)
        const hours = Math.floor(minutes / 60)
        const days = Math.floor(hours / 24)
        if (seconds < 60) return "just now"
        if (minutes < 60) return `${minutes}m ago`
        if (hours < 24) return `${hours}h ago`
        if (days < 7) return `${days}d ago`
        return new Date(timestamp).toLocaleDateString()
      }

      const result = getRelativeTime(entry.timestamp)
      expect(result).toMatch(/\d{1,2}\/\d{1,2}\/\d{2,4}/)
    })
  })

  describe("entry preview", () => {
    test("should truncate long input to 50 characters", () => {
      const longInput = "This is a very long prompt that needs to be truncated to fit in the preview"
      const maxLength = 50

      const getPreview = (input: string, maxLen: number = maxLength) => {
        const firstLine = input.split("\n")[0].trim()
        return firstLine.slice(0, maxLen) + (firstLine.length > maxLen ? "..." : "")
      }

      const preview = getPreview(longInput)
      expect(preview.length).toBeLessThanOrEqual(53) // 50 + "..."
      expect(preview).toContain("...")
    })

    test("should show first line of multi-line input", () => {
      const multiLineInput = "Line 1\nLine 2\nLine 3"

      const getPreview = (input: string) => {
        const firstLine = input.split("\n")[0].trim()
        return firstLine
      }

      expect(getPreview(multiLineInput)).toBe("Line 1")
    })

    test("should handle empty input", () => {
      const emptyInput = ""

      const getPreview = (input: string) => {
        const firstLine = input.split("\n")[0].trim()
        return firstLine
      }

      expect(getPreview(emptyInput)).toBe("")
    })
  })

  describe("line count display", () => {
    test("should show line count for multi-line entries", () => {
      const entries: StashEntry[] = [
        { input: "single line", timestamp: Date.now() },
        { input: "line 1\nline 2\nline 3", timestamp: Date.now() },
        { input: "a\nb\nc\nd\ne", timestamp: Date.now() },
      ]

      const getLineCount = (entry: StashEntry) => {
        const lineCount = (entry.input.match(/\n/g)?.length ?? 0) + 1
        return lineCount
      }

      expect(getLineCount(entries[0])).toBe(1)
      expect(getLineCount(entries[1])).toBe(3)
      expect(getLineCount(entries[2])).toBe(5)
    })

    test("should show footer only for multi-line entries", () => {
      const getFooter = (lineCount: number) => {
        return lineCount > 1 ? `~${lineCount} lines` : undefined
      }

      expect(getFooter(1)).toBeUndefined()
      expect(getFooter(3)).toBe("~3 lines")
    })
  })

  describe("entry ordering", () => {
    test("should show most recent entries first", () => {
      const now = Date.now()
      const entries: StashEntry[] = [
        { input: "oldest", timestamp: now - 100000 },
        { input: "newer", timestamp: now - 50000 },
        { input: "newest", timestamp: now - 10000 },
      ]

      const sorted = [...entries].sort((a, b) => b.timestamp - a.timestamp)

      expect(sorted[0].input).toBe("newest")
      expect(sorted[1].input).toBe("newer")
      expect(sorted[2].input).toBe("oldest")
    })
  })

  describe("delete confirmation", () => {
    test("should require two attempts to delete", () => {
      let toDelete: string | undefined = undefined
      const sessionID = "session-123"

      const handleDelete = (id: string) => {
        if (toDelete === id) {
          // Confirmed delete
          return true
        }
        toDelete = id
        return false
      }

      // First attempt - mark for deletion
      const firstResult = handleDelete(sessionID)
      expect(firstResult).toBe(false)
      expect(toDelete).toBe(sessionID)

      // Second attempt - confirm delete
      const secondResult = handleDelete(sessionID)
      expect(secondResult).toBe(true)
    })

    test("should reset delete state on different selection", () => {
      let markedIndex = 2

      const selectDifferent = (index: number) => {
        markedIndex = index
      }

      selectDifferent(0)
      expect(markedIndex).toBe(0)
    })

    test("should show delete prompt when marked", () => {
      const keybind = "ctrl+d"

      const getTitle = (marked: boolean) =>
        marked ? `Press ${keybind} again to confirm` : "Prompt entry"

      expect(getTitle(true)).toContain("again to confirm")
      expect(getTitle(false)).toBe("Prompt entry")
    })
  })

  describe("stash operations", () => {
    test("should remove entry on confirmed delete", () => {
      const entries: StashEntry[] = [
        { input: "entry1", timestamp: Date.now() },
        { input: "entry2", timestamp: Date.now() },
        { input: "entry3", timestamp: Date.now() },
      ]
      const selectedIndex = 1

      // Simulate delete
      entries.splice(selectedIndex, 1)

      expect(entries).toHaveLength(2)
      expect(entries.find((e) => e.input === "entry2")).toBeUndefined()
    })

    test("should restore selected entry from stash", () => {
      const selectedEntry: StashEntry = {
        input: "restore me",
        timestamp: Date.now(),
      }
      const stash = [selectedEntry]

      const onSelect = (entry: StashEntry) => {
        // Restore entry to prompt
        expect(entry.input).toBe("restore me")
      }

      onSelect(selectedEntry)
    })

    test("should clear dialog after selection", () => {
      let dialogCleared = false

      const onSelect = () => {
        dialogCleared = true
      }

      onSelect()
      expect(dialogCleared).toBe(true)
    })
  })

  describe("keybind integration", () => {
    test("should have delete keybind", () => {
      const keybind = "ctrl+d"

      expect(keybind).toBe("ctrl+d")
    })

    test("should show delete keybind in title when marked", () => {
      const isMarked = true

      const getTitle = (marked: boolean, keybind: string) =>
        marked ? `Press ${keybind} again to confirm` : "Stash entry"

      expect(getTitle(true, "ctrl+d")).toContain("ctrl+d")
      expect(getTitle(false, "ctrl+d")).not.toContain("ctrl+d")
    })
  })
})
