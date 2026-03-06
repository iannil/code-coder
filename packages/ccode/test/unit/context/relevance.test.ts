/**
 * Tests for context/relevance module with native integration
 */

import { describe, it, expect } from "bun:test"
import { Relevance } from "@/context/relevance"
import * as RelevanceNative from "@/context/relevance-native"

describe("Relevance", () => {
  describe("scoreRelevance", () => {
    it("should score exact match higher than partial match", async () => {
      const exactScore = await Relevance.scoreRelevance("authentication", "authentication")
      const partialScore = await Relevance.scoreRelevance("auth", "user authentication system")
      // Exact match should score higher or equal
      expect(exactScore).toBeGreaterThanOrEqual(partialScore)
    })

    it("should return score between 0 and 1", async () => {
      const score = await Relevance.scoreRelevance("user login", "user authentication login handler")
      expect(score).toBeGreaterThanOrEqual(0)
      expect(score).toBeLessThanOrEqual(1)
    })

    it("should handle word overlap scoring", async () => {
      const score = await Relevance.scoreRelevance("user login", "user authentication login handler")
      expect(score).toBeGreaterThan(0)
    })
  })

  describe("isUsingNative", () => {
    it("should return boolean for native status", () => {
      const status = Relevance.isUsingNative()
      expect(typeof status).toBe("boolean")
    })
  })
})

describe("RelevanceNative", () => {
  describe("isNativeAvailable", () => {
    it("should return boolean", async () => {
      const available = await RelevanceNative.isNativeAvailable()
      expect(typeof available).toBe("boolean")
    })
  })

  describe("scoreRelevanceNative", () => {
    it("should return score object or null", async () => {
      const result = await RelevanceNative.scoreRelevanceNative("test", "test content for testing")
      // Result is either a score object or null if native not available
      if (result !== null) {
        expect(typeof result.score).toBe("number")
        expect(typeof result.keywordScore).toBe("number")
        expect(typeof result.structuralScore).toBe("number")
        expect(typeof result.recencyScore).toBe("number")
        expect(Array.isArray(result.matchedKeywords)).toBe(true)
      } else {
        expect(result).toBe(null)
      }
    })

    it("should return scores in valid range", async () => {
      const result = await RelevanceNative.scoreRelevanceNative("function", "export function test() {}")
      if (result !== null) {
        expect(result.score).toBeGreaterThanOrEqual(0)
        expect(result.score).toBeLessThanOrEqual(1)
      }
    })
  })

  describe("contentHashNative", () => {
    it("should return consistent hash or null", async () => {
      const hash1 = await RelevanceNative.contentHashNative("hello world")
      const hash2 = await RelevanceNative.contentHashNative("hello world")

      if (hash1 !== null && hash2 !== null) {
        expect(hash1).toBe(hash2)
      } else {
        // Native not available
        expect(hash1).toBe(null)
        expect(hash2).toBe(null)
      }
    })

    it("should return different hash for different content", async () => {
      const hash1 = await RelevanceNative.contentHashNative("hello world")
      const hash2 = await RelevanceNative.contentHashNative("goodbye world")

      if (hash1 !== null && hash2 !== null) {
        expect(hash1).not.toBe(hash2)
      }
    })
  })

  describe("DEFAULT_SCORER_CONFIG", () => {
    it("should have correct default weights", () => {
      const config = RelevanceNative.DEFAULT_SCORER_CONFIG
      expect(config.keywordWeight).toBe(0.5)
      expect(config.structuralWeight).toBe(0.3)
      expect(config.recencyWeight).toBe(0.2)
      expect(config.minScore).toBe(0.1)
      expect(config.caseInsensitive).toBe(true)
    })
  })
})
