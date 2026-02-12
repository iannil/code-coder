// @ts-nocheck
/**
 * Memory Markdown Consolidation Unit Tests
 *
 * Tests for the memory consolidation functionality including:
 * - Importance scoring
 * - Entry categorization
 * - Deduplication
 * - Consolidation extraction
 */

import { describe, test, expect } from "bun:test"
import type { DailyEntry } from "@/memory-markdown"

describe("Memory Markdown - Consolidation", () => {
  describe("importance calculation", () => {
    test("should score decision entries high", () => {
      const entry: DailyEntry = {
        type: "decision",
        content: "Decided to use TypeScript",
        timestamp: Date.now(),
      }

      const score = calculateImportance(entry)
      expect(score).toBeGreaterThanOrEqual(0.8)
    })

    test("should score lesson entries highest", () => {
      const entry: DailyEntry = {
        type: "lesson",
        content: "Learned that testing is important",
        timestamp: Date.now(),
      }

      const score = calculateImportance(entry)
      expect(score).toBeGreaterThanOrEqual(0.9)
    })

    test("should score simple notes low", () => {
      const entry: DailyEntry = {
        type: "note",
        content: "Simple note",
        timestamp: Date.now(),
      }

      const score = calculateImportance(entry)
      expect(score).toBeLessThan(0.5)
    })

    test("should boost score for important keywords", () => {
      const entry1: DailyEntry = {
        type: "note",
        content: "This is critical for the system",
        timestamp: Date.now(),
      }

      const entry2: DailyEntry = {
        type: "note",
        content: "This is a simple note",
        timestamp: Date.now(),
      }

      const score1 = calculateImportance(entry1)
      const score2 = calculateImportance(entry2)

      expect(score1).toBeGreaterThan(score2)
    })

    test("should cap score at 1.0", () => {
      const entry: DailyEntry = {
        type: "lesson",
        content: "Critical important key best practice architecture design decision learned",
        timestamp: Date.now(),
      }

      const score = calculateImportance(entry)
      expect(score).toBeLessThanOrEqual(1.0)
    })
  })

  describe("entry categorization", () => {
    test("should categorize preference type entries", () => {
      const entry: DailyEntry = {
        type: "preference",
        content: "User prefers dark mode",
        timestamp: Date.now(),
      }

      const category = categorizeEntry(entry)
      expect(category).toBe("用户偏好")
    })

    test("should categorize decision type entries", () => {
      const entry: DailyEntry = {
        type: "decision",
        content: "Chose to use PostgreSQL",
        timestamp: Date.now(),
      }

      const category = categorizeEntry(entry)
      expect(category).toBe("关键决策")
    })

    test("should categorize lesson type entries", () => {
      const entry: DailyEntry = {
        type: "lesson",
        content: "Learned to always write tests first",
        timestamp: Date.now(),
      }

      const category = categorizeEntry(entry)
      expect(category).toBe("经验教训")
    })

    test("should detect preferences from content", () => {
      const entry: DailyEntry = {
        type: "note",
        content: "I prefer using spaces over tabs",
        timestamp: Date.now(),
      }

      const category = categorizeEntry(entry)
      expect(category).toBe("用户偏好")
    })

    test("should detect decisions from content", () => {
      const entry: DailyEntry = {
        type: "note",
        content: "We decided to migrate to React",
        timestamp: Date.now(),
      }

      const category = categorizeEntry(entry)
      expect(category).toBe("关键决策")
    })

    test("should return null for unrecognizable entries", () => {
      const entry: DailyEntry = {
        type: "error",
        content: "Some error occurred",
        timestamp: Date.now(),
      }

      const category = categorizeEntry(entry)
      expect(category).toBeNull()
    })
  })

  describe("entry deduplication", () => {
    test("should remove duplicate entries", () => {
      const entries = [
        { content: "- Test entry", source: "2026-02-05", importance: 0.8, timestamp: Date.now() },
        { content: "- Test entry", source: "2026-02-05", importance: 0.8, timestamp: Date.now() },
      ]

      const deduplicated = deduplicateEntries(entries)
      expect(deduplicated.length).toBe(1)
    })

    test("should preserve unique entries", () => {
      const entries = [
        { content: "- First entry", source: "2026-02-05", importance: 0.8, timestamp: Date.now() },
        { content: "- Second entry", source: "2026-02-05", importance: 0.7, timestamp: Date.now() },
      ]

      const deduplicated = deduplicateEntries(entries)
      expect(deduplicated.length).toBe(2)
    })

    test("should sort by importance", () => {
      const entries = [
        { content: "- Low priority", source: "2026-02-05", importance: 0.3, timestamp: Date.now() },
        { content: "- High priority", source: "2026-02-05", importance: 0.9, timestamp: Date.now() },
        { content: "- Medium priority", source: "2026-02-05", importance: 0.6, timestamp: Date.now() },
      ]

      const deduplicated = deduplicateEntries(entries)
      expect(deduplicated[0].importance).toBeGreaterThanOrEqual(deduplicated[1].importance)
      expect(deduplicated[1].importance).toBeGreaterThanOrEqual(deduplicated[2].importance)
    })
  })
})

// Helper functions redeclared for testing
function calculateImportance(entry: DailyEntry): number {
  let score = 0

  const typeScores: Record<string, number> = {
    decision: 0.8,
    preference: 0.7,
    lesson: 0.9,
    task: 0.3,
    note: 0.2,
    error: 0.6,
  }

  score += typeScores[entry.type ?? "note"] ?? 0.5

  const content = entry.content.toLowerCase()

  const importantKeywords = [
    "critical",
    "important",
    "key",
    "must",
    "always",
    "never",
    "best practice",
    "architecture",
    "design",
  ]

  for (const keyword of importantKeywords) {
    if (content.includes(keyword)) {
      score += 0.1
    }
  }

  if (entry.content.length > 100) {
    score += 0.1
  }
  if (entry.content.length > 300) {
    score += 0.1
  }

  return Math.min(score, 1)
}

function categorizeEntry(entry: DailyEntry): string | null {
  const content = entry.content.toLowerCase()
  const type = entry.type

  if (type === "preference") return "用户偏好"
  if (type === "decision") return "关键决策"
  if (type === "lesson") return "经验教训"

  if (content.includes("prefer")) {
    return "用户偏好"
  }

  if (content.includes("decided") || content.includes("decision")) {
    return "关键决策"
  }

  if (content.includes("learned") || content.includes("lesson")) {
    return "经验教训"
  }

  if (content.includes("project") || content.includes("architecture")) {
    return "项目上下文"
  }

  if (type === "note" || type === "task") {
    return "项目上下文"
  }

  return null
}

function deduplicateEntries(entries: Array<{ content: string; source: string; importance: number; timestamp: number }>): Array<{ content: string; source: string; importance: number; timestamp: number }> {
  const seen = new Set<string>()
  const unique: typeof entries = []

  for (const entry of entries) {
    const normalized = entry.content
      .toLowerCase()
      .replace(/\[\w+\]\s*/, "")
      .replace(/\s*\(\d{4}-\d{2}-\d{2}\)\s*$/, "")
      .trim()

    if (!seen.has(normalized)) {
      seen.add(normalized)
      unique.push(entry)
    }
  }

  return unique.sort((a, b) => b.importance - a.importance)
}
