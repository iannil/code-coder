// @ts-nocheck
/**
 * Memory Markdown Utility Unit Tests
 *
 * Tests for the memory-markdown utility functions including:
 * - Date formatting
 * - Timestamp formatting
 * - Markdown section formatting
 * - Category extraction
 */

import { describe, test, expect } from "bun:test"
import {
  formatDate,
  formatTimestamp,
  parseDate,
  formatDailyEntry,
  formatSectionHeader,
  extractCategory,
  getLastNDays,
  sanitizeFilename,
} from "@/memory-markdown/util"

describe("Memory Markdown - Utilities", () => {
  describe("formatDate", () => {
    test("should format date as YYYY-MM-DD", () => {
      const date = new Date("2026-02-05T12:00:00Z")
      expect(formatDate(date)).toBe("2026-02-05")
    })

    test("should handle single digit month and day", () => {
      const date = new Date("2026-01-09T12:00:00Z")
      expect(formatDate(date)).toBe("2026-01-09")
    })

    test("should handle leap year date", () => {
      const date = new Date("2024-02-29T12:00:00Z")
      expect(formatDate(date)).toBe("2024-02-29")
    })
  })

  describe("formatTimestamp", () => {
    test("should format timestamp as ISO string", () => {
      const date = new Date("2026-02-05T12:34:56Z")
      expect(formatTimestamp(date)).toBe("2026-02-05T12:34:56.000Z")
    })

    test("should handle epoch timestamp", () => {
      const date = new Date(0)
      expect(formatTimestamp(date)).toBe("1970-01-01T00:00:00.000Z")
    })
  })

  describe("parseDate", () => {
    test("should parse YYYY-MM-DD string", () => {
      const date = parseDate("2026-02-05")
      expect(date.getFullYear()).toBe(2026)
      expect(date.getMonth()).toBe(1) // February is 1
      expect(date.getDate()).toBe(5)
    })

    test("should handle invalid date string", () => {
      const date = parseDate("invalid")
      expect(date.toString()).toBe("Invalid Date")
    })
  })

  describe("formatDailyEntry", () => {
    test("should format entry with timestamp", () => {
      const entry = {
        type: "decision",
        content: "Test note content",
        timestamp: 1738789447000,
      }
      const formatted = formatDailyEntry(entry)

      expect(formatted).toContain("[") // Time wrapper
      expect(formatted).toContain("]") // Time wrapper
      expect(formatted).toContain("💭") // Decision icon
      expect(formatted).toContain("DECISION")
      expect(formatted).toContain("Test note content")
    })

    test("should format task entry", () => {
      const entry = {
        type: "action",
        content: "Test task",
        timestamp: Date.now(),
      }
      const formatted = formatDailyEntry(entry)

      expect(formatted).toContain("⚡") // Action icon
      expect(formatted).toContain("ACTION")
      expect(formatted).toContain("Test task")
    })

    test("should format error entry", () => {
      const entry = {
        type: "error",
        content: "Error occurred",
        timestamp: Date.now(),
      }
      const formatted = formatDailyEntry(entry)

      expect(formatted).toContain("❌") // Error icon
      expect(formatted).toContain("ERROR")
      expect(formatted).toContain("Error occurred")
    })
  })

  describe("formatSectionHeader", () => {
    test("should format category with underline", () => {
      const header = formatSectionHeader("用户偏好")

      expect(header).toContain("## 用户偏好")
      expect(header).toContain("─")
    })

    test("should handle different category lengths", () => {
      const header1 = formatSectionHeader("项目上下文")
      const header2 = formatSectionHeader("关键决策")

      expect(header1).toContain("## 项目上下文")
      expect(header2).toContain("## 关键决策")
    })
  })

  describe("extractCategory", () => {
    test("should extract category content", () => {
      const content = `## 用户偏好
────────────────────────────────────────────────
- Item 1
- Item 2

## 项目上下文
────────────────────────────────────────────────
- Context item`

      const extracted = extractCategory(content, "用户偏好")

      expect(extracted).toContain("Item 1")
      expect(extracted).toContain("Item 2")
      expect(extracted).not.toContain("Context item")
    })

    test("should return null for missing category", () => {
      const content = "## Other Section\nSome content"
      const extracted = extractCategory(content, "用户偏好")

      expect(extracted).toBeNull()
    })

    test("should handle category with special characters", () => {
      const content = `## 经验教训
────────────────────────────────────────────────
- Learned: don't repeat errors`

      const extracted = extractCategory(content, "经验教训")

      expect(extracted).toContain("Learned:")
    })
  })

  describe("getLastNDays", () => {
    test("should return last N days", () => {
      const dates = getLastNDays(3)
      expect(dates.length).toBe(3)
    })

    test("should include today", () => {
      const dates = getLastNDays(1)
      const today = formatDate(new Date())

      expect(formatDate(dates[0])).toBe(today)
    })

    test("should handle zero days", () => {
      const dates = getLastNDays(0)
      expect(dates.length).toBe(0)
    })
  })

  describe("sanitizeFilename", () => {
    test("should remove invalid characters", () => {
      const sanitized = sanitizeFilename("file/name?.txt")
      expect(sanitized).not.toContain("/")
      expect(sanitized).not.toContain("?")
    })

    test("should preserve valid characters", () => {
      const sanitized = sanitizeFilename("2026-02-05")
      expect(sanitized).toBe("2026-02-05")
    })

    test("should replace spaces with underscores", () => {
      const sanitized = sanitizeFilename("my file name")
      expect(sanitized).toBe("my_file_name")
    })

    test("should handle empty string", () => {
      const sanitized = sanitizeFilename("")
      expect(sanitized).toBe("")
    })

    test("should handle multiple consecutive invalid characters", () => {
      const sanitized = sanitizeFilename("file///name\\\\test")
      expect(sanitized).not.toContain("/")
      expect(sanitized).not.toContain("\\")
    })
  })
})
