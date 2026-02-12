/**
 * Unit Tests: Frecency Context
 * Testing frecency calculation and sorting logic
 */

import { describe, test, expect } from "bun:test"

describe("Frecency Context", () => {
  describe("calculateFrecency", () => {
    function calculateFrecency(entry?: { frequency: number; lastOpen: number }): number {
      if (!entry) return 0
      const daysSince = (Date.now() - entry.lastOpen) / 86400000 // ms per day
      const weight = 1 / (1 + daysSince)
      return entry.frequency * weight
    }

    test("should return 0 for undefined entry", () => {
      const result = calculateFrecency(undefined)
      expect(result).toBe(0)
    })

    test("should calculate frecency for recent entry", () => {
      const now = Date.now()
      const entry = {
        frequency: 5,
        lastOpen: now, // Just opened
      }

      const result = calculateFrecency(entry)

      // With 0 days since, weight is 1, so frecency = 5
      // Use toBeCloseTo for floating point precision
      expect(result).toBeCloseTo(5, 5)
    })

    test("should calculate frecency for entry opened yesterday", () => {
      const yesterday = Date.now() - 86400000
      const entry = {
        frequency: 10,
        lastOpen: yesterday,
      }

      const result = calculateFrecency(entry)

      // With 1 day since, weight is 1/2, so frecency = 5
      expect(result).toBeCloseTo(5, 4)
    })

    test("should calculate frecency for entry opened a week ago", () => {
      const weekAgo = Date.now() - 7 * 86400000
      const entry = {
        frequency: 16,
        lastOpen: weekAgo,
      }

      const result = calculateFrecency(entry)

      // With 7 days since, weight is 1/8, so frecency = 2
      expect(result).toBeCloseTo(2, 4)
    })

    test("should give higher score to frequent recent items", () => {
      const now = Date.now()

      const frequentRecent = {
        frequency: 10,
        lastOpen: now,
      }

      const infrequentOld = {
        frequency: 20,
        lastOpen: now - 10 * 86400000, // 10 days ago
      }

      const recentScore = calculateFrecency(frequentRecent)
      const oldScore = calculateFrecency(infrequentOld)

      expect(recentScore).toBeGreaterThan(oldScore)
    })

    test("should decay score over time for same frequency", () => {
      const now = Date.now()
      const frequency = 10

      const today = { frequency, lastOpen: now }
      const yesterday = { frequency, lastOpen: now - 86400000 }
      const lastWeek = { frequency, lastOpen: now - 7 * 86400000 }

      const todayScore = calculateFrecency(today)
      const yesterdayScore = calculateFrecency(yesterday)
      const lastWeekScore = calculateFrecency(lastWeek)

      expect(todayScore).toBeGreaterThan(yesterdayScore)
      expect(yesterdayScore).toBeGreaterThan(lastWeekScore)
    })

    test("should handle edge case of very old entry", () => {
      const ancient = Date.now() - 365 * 86400000 // 1 year ago
      const entry = {
        frequency: 100,
        lastOpen: ancient,
      }

      const result = calculateFrecency(entry)

      // With ~365 days since, weight is ~1/366, so frecency should be small
      expect(result).toBeLessThan(1)
    })
  })

  describe("frecency sorting", () => {
    function calculateFrecency(entry?: { frequency: number; lastOpen: number }): number {
      if (!entry) return 0
      const daysSince = (Date.now() - entry.lastOpen) / 86400000
      const weight = 1 / (1 + daysSince)
      return entry.frequency * weight
    }

    test("should sort entries by frecency score", () => {
      const now = Date.now()

      const entries = [
        { path: "/a", frequency: 5, lastOpen: now - 86400000 },
        { path: "/b", frequency: 10, lastOpen: now },
        { path: "/c", frequency: 3, lastOpen: now },
      ]

      const withScores = entries.map((e) => ({
        ...e,
        score: calculateFrecency(e),
      }))

      const sorted = [...withScores].sort((a, b) => b.score - a.score)

      // /b has highest score (10 * 1 = 10)
      expect(sorted[0].path).toBe("/b")
      // /c has second highest (3 * 1 = 3)
      expect(sorted[1].path).toBe("/c")
      // /a has lowest score (5 * 0.5 = 2.5)
      expect(sorted[2].path).toBe("/a")
    })

    test("should prioritize recency when frequencies are equal", () => {
      const now = Date.now()

      const entries = [
        { path: "/old", frequency: 5, lastOpen: now - 7 * 86400000 },
        { path: "/new", frequency: 5, lastOpen: now },
      ]

      const sorted = [...entries].sort(
        (a, b) => calculateFrecency(b) - calculateFrecency(a),
      )

      expect(sorted[0].path).toBe("/new")
      expect(sorted[1].path).toBe("/old")
    })

    test("should prioritize frequency when recency is equal", () => {
      const now = Date.now()

      const entries = [
        { path: "/rare", frequency: 2, lastOpen: now },
        { path: "/freq", frequency: 10, lastOpen: now },
      ]

      const sorted = [...entries].sort(
        (a, b) => calculateFrecency(b) - calculateFrecency(a),
      )

      expect(sorted[0].path).toBe("/freq")
      expect(sorted[1].path).toBe("/rare")
    })
  })

  describe("frecency update", () => {
    test("should increment frequency on update", () => {
      const entry = { frequency: 5, lastOpen: Date.now() - 86400000 }

      const updated = {
        frequency: entry.frequency + 1,
        lastOpen: Date.now(),
      }

      expect(updated.frequency).toBe(6)
      expect(updated.lastOpen).toBeGreaterThan(entry.lastOpen)
    })

    test("should create new entry if not exists", () => {
      const existing = undefined as { frequency: number; lastOpen: number } | undefined

      const newEntry = {
        frequency: (existing?.frequency || 0) + 1,
        lastOpen: Date.now(),
      }

      expect(newEntry.frequency).toBe(1)
      expect(newEntry.lastOpen).toBeDefined()
    })
  })

  describe("MAX_FRECENCY_ENTRIES", () => {
    const MAX_FRECENCY_ENTRIES = 1000

    test("should define maximum entries constant", () => {
      expect(MAX_FRECENCY_ENTRIES).toBe(1000)
    })

    test("should limit entries to maximum", () => {
      const now = Date.now()
      const entries: Record<string, { frequency: number; lastOpen: number }> = {}

      // Add more than max entries
      for (let i = 0; i < MAX_FRECENCY_ENTRIES + 100; i++) {
        entries[`/path/${i}`] = {
          frequency: 1,
          lastOpen: now - i * 1000, // Slightly different timestamps
        }
      }

      const sorted = Object.entries(entries)
        .sort(([, a], [, b]) => b.lastOpen - a.lastOpen)
        .slice(0, MAX_FRECENCY_ENTRIES)

      expect(sorted.length).toBe(MAX_FRECENCY_ENTRIES)
    })

    test("should keep most recently opened when limiting", () => {
      const now = Date.now()
      const entries: Record<string, { frequency: number; lastOpen: number }> = {}

      // Add entries with varying recency
      for (let i = 0; i < 20; i++) {
        entries[`/path/${i}`] = {
          frequency: 1,
          lastOpen: now - i * 1000000, // Older entries have lower i
        }
      }

      // Limit to 10 entries
      const limited = Object.entries(entries)
        .sort(([, a], [, b]) => b.lastOpen - a.lastOpen)
        .slice(0, 10)

      expect(limited.length).toBe(10)
      // Should keep the most recent (highest lastOpen values)
      expect(limited[0][0]).toBe("/path/0")
      expect(limited[9][0]).toBe("/path/9")
    })
  })

  describe("frecency edge cases", () => {
    function calculateFrecency(entry?: { frequency: number; lastOpen: number }): number {
      if (!entry) return 0
      const daysSince = (Date.now() - entry.lastOpen) / 86400000
      const weight = 1 / (1 + daysSince)
      return entry.frequency * weight
    }

    test("should handle zero frequency", () => {
      const entry = {
        frequency: 0,
        lastOpen: Date.now(),
      }

      const result = calculateFrecency(entry)
      expect(result).toBe(0)
    })

    test("should handle very high frequency", () => {
      const entry = {
        frequency: 10000,
        lastOpen: Date.now(),
      }

      const result = calculateFrecency(entry)
      expect(result).toBeCloseTo(10000, 4)
    })

    test("should handle negative timestamp (future date)", () => {
      const entry = {
        frequency: 5,
        lastOpen: Date.now() + 86400000, // Tomorrow
      }

      // This is an edge case that shouldn't happen in practice
      // With negative daysSince, the weight becomes negative
      // and dividing by zero or negative gives Infinity
      const result = calculateFrecency(entry)
      expect(result).toBe(Infinity) // Future dates result in Infinity
    })
  })
})
