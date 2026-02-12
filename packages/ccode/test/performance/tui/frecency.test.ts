/**
 * Performance Tests: Frecency Algorithm
 *
 * Tests for the frecency (frequency + recency) algorithm performance:
 * - Score calculation speed
 * - Large dataset handling
 * - Ranking performance
 */

import { describe, test, expect, beforeEach } from "bun:test"

describe("Frecency Performance", () => {
  interface FrecencyItem {
    id: string
    frequency: number
    lastAccess: number
    score: number
  }

  describe("score calculation", () => {
    test("should calculate score for 1000 items quickly", () => {
      const items: FrecencyItem[] = Array.from({ length: 1000 }, (_, i) => ({
        id: `item-${i}`,
        frequency: Math.floor(Math.random() * 100),
        lastAccess: Date.now() - Math.floor(Math.random() * 1_000_000 * 60), // Random time in last million minutes
        score: 0,
      }))

      const startTime = performance.now()

      const halfLife = 7 * 24 * 60 * 60 * 1000 // 7 days in ms
      const now = Date.now()

      for (const item of items) {
        const age = now - item.lastAccess
        const decay = Math.exp(-Math.log(2) * (age / halfLife))
        item.score = item.frequency * decay
      }

      const endTime = performance.now()
      const calcTime = endTime - startTime

      expect(items.length).toBe(1000)
      expect(calcTime).toBeLessThan(20)
    })

    test("should rank 500 items by score efficiently", () => {
      const items: FrecencyItem[] = Array.from({ length: 500 }, (_, i) => ({
        id: `item-${i}`,
        frequency: Math.floor(Math.random() * 50) + 1,
        lastAccess: Date.now() - Math.floor(Math.random() * 10_000_000),
        score: 0,
      }))

      // Calculate scores first
      const now = Date.now()
      const halfLife = 7 * 24 * 60 * 60 * 1000

      for (const item of items) {
        const age = now - item.lastAccess
        const decay = Math.exp(-Math.log(2) * (age / halfLife))
        item.score = item.frequency * decay
      }

      const startTime = performance.now()

      const ranked = [...items].sort((a, b) => b.score - a.score)

      const endTime = performance.now()
      const sortTime = endTime - startTime

      expect(ranked.length).toBe(500)
      expect(ranked[0].score).toBeGreaterThanOrEqual(ranked[ranked.length - 1].score)
      expect(sortTime).toBeLessThan(30)
    })
  })

  describe("dataset operations", () => {
    test("should handle insert efficiently", () => {
      const dataset = new Map<string, FrecencyItem>()
      const insertCount = 1000

      const startTime = performance.now()

      for (let i = 0; i < insertCount; i++) {
        dataset.set(`item-${i}`, {
          id: `item-${i}`,
          frequency: 1,
          lastAccess: Date.now(),
          score: 1,
        })
      }

      const endTime = performance.now()
      const insertTime = endTime - startTime

      expect(dataset.size).toBe(insertCount)
      expect(insertTime).toBeLessThan(20)
    })

    test("should handle update efficiently", () => {
      const dataset = new Map<string, FrecencyItem>()
      const itemCount = 500

      // Pre-populate
      for (let i = 0; i < itemCount; i++) {
        dataset.set(`item-${i}`, {
          id: `item-${i}`,
          frequency: 1,
          lastAccess: Date.now(),
          score: 1,
        })
      }

      const startTime = performance.now()

      // Update all items (increment frequency)
      for (const [id, item] of dataset) {
        item.frequency++
        item.lastAccess = Date.now()
        item.score = item.frequency
      }

      const endTime = performance.now()
      const updateTime = endTime - startTime

      expect(dataset.get("item-0")?.frequency).toBe(2)
      expect(updateTime).toBeLessThan(15)
    })

    test("should handle search efficiently", () => {
      const dataset: FrecencyItem[] = Array.from({ length: 1000 }, (_, i) => ({
        id: i % 10 === 0 ? `important-${i}` : `item-${i}`,
        frequency: Math.floor(Math.random() * 100),
        lastAccess: Date.now() - Math.floor(Math.random() * 10_000_000),
        score: 0,
      }))

      // Pre-calculate scores
      for (const item of dataset) {
        item.score = item.frequency
      }

      const startTime = performance.now()

      // Search for items matching prefix
      const results = dataset.filter((item) => item.id.startsWith("important-"))

      const endTime = performance.now()
      const searchTime = endTime - startTime

      expect(results.length).toBe(100)
      expect(searchTime).toBeLessThan(10)
    })
  })

  describe("ranking performance", () => {
    test("should handle top-N selection efficiently", () => {
      const items: FrecencyItem[] = Array.from({ length: 2000 }, (_, i) => ({
        id: `item-${i}`,
        frequency: Math.floor(Math.random() * 100),
        lastAccess: Date.now() - Math.floor(Math.random() * 10_000_000),
        score: Math.random() * 1000,
      }))

      const n = 10
      const startTime = performance.now()

      // Get top N by score
      const topN = [...items]
        .sort((a, b) => b.score - a.score)
        .slice(0, n)

      const endTime = performance.now()
      const selectTime = endTime - startTime

      expect(topN.length).toBe(n)
      expect(topN[0].score).toBeGreaterThanOrEqual(topN[topN.length - 1].score)
      expect(selectTime).toBeLessThan(50)
    })

    test("should handle filtered ranking", () => {
      const items: FrecencyItem[] = Array.from({ length: 500 }, (_, i) => ({
        id: i % 3 === 0 ? `important-${i}` : i % 2 === 0 ? `urgent-${i}` : `item-${i}`,
        frequency: Math.floor(Math.random() * 100),
        lastAccess: Date.now() - Math.floor(Math.random() * 10_000_000),
        score: Math.random() * 1000,
      }))

      const filterPrefix = "important-"
      const startTime = performance.now()

      // Filter then rank
      const filtered = items.filter((item) => item.id.startsWith(filterPrefix))
      const ranked = filtered.sort((a, b) => b.score - a.score)

      const endTime = performance.now()
      const rankTime = endTime - startTime

      expect(ranked.length).toBeGreaterThan(0)
      expect(rankTime).toBeLessThan(20)
    })
  })
})
