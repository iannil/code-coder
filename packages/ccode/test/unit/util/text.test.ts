/**
 * Tests for util/text module - unified text processing API
 */

import { describe, it, expect } from "bun:test"
import {
  levenshteinDistance,
  stringSimilarity,
  wordSimilarity,
  prefixSuffixSimilarity,
  findBestMatch,
  isNativeAvailable,
  similarity,
  similaritySync,
  bestMatch,
  isSimilar,
  isSimilarSync,
} from "@/util/text"

describe("levenshteinDistance", () => {
  it("should return 0 for identical strings", () => {
    expect(levenshteinDistance("hello", "hello")).toBe(0)
    expect(levenshteinDistance("", "")).toBe(0)
    expect(levenshteinDistance("test string", "test string")).toBe(0)
  })

  it("should return length of non-empty string when other is empty", () => {
    expect(levenshteinDistance("hello", "")).toBe(5)
    expect(levenshteinDistance("", "hello")).toBe(5)
  })

  it("should handle single character differences", () => {
    expect(levenshteinDistance("cat", "car")).toBe(1) // substitution
    expect(levenshteinDistance("cat", "cats")).toBe(1) // insertion
    expect(levenshteinDistance("cats", "cat")).toBe(1) // deletion
  })

  it("should calculate correct distance for different strings", () => {
    expect(levenshteinDistance("kitten", "sitting")).toBe(3)
    expect(levenshteinDistance("flaw", "lawn")).toBe(2)
    expect(levenshteinDistance("saturday", "sunday")).toBe(3)
  })

  it("should be symmetric", () => {
    expect(levenshteinDistance("abc", "xyz")).toBe(levenshteinDistance("xyz", "abc"))
    expect(levenshteinDistance("test", "testing")).toBe(levenshteinDistance("testing", "test"))
  })

  it("should handle unicode characters", () => {
    expect(levenshteinDistance("café", "cafe")).toBe(1)
    expect(levenshteinDistance("日本語", "日本人")).toBe(1)
  })
})

describe("stringSimilarity", () => {
  it("should return 1.0 for identical strings", () => {
    expect(stringSimilarity("hello", "hello")).toBe(1.0)
    expect(stringSimilarity("", "")).toBe(1.0)
  })

  it("should return 0.0 when one string is empty and other is not", () => {
    expect(stringSimilarity("hello", "")).toBe(0.0)
    expect(stringSimilarity("", "hello")).toBe(0.0)
  })

  it("should return value between 0 and 1", () => {
    const sim = stringSimilarity("testing", "resting")
    expect(sim).toBeGreaterThan(0)
    expect(sim).toBeLessThan(1)
  })

  it("should return higher similarity for more similar strings", () => {
    const highSim = stringSimilarity("hello", "hallo")
    const lowSim = stringSimilarity("hello", "world")
    expect(highSim).toBeGreaterThan(lowSim)
  })

  it("should be symmetric", () => {
    expect(stringSimilarity("test", "tset")).toBe(stringSimilarity("tset", "test"))
  })
})

describe("wordSimilarity", () => {
  it("should return 1.0 for identical word sets", () => {
    expect(wordSimilarity("hello world", "hello world")).toBe(1.0)
    expect(wordSimilarity("a b c", "c b a")).toBe(1.0) // order doesn't matter
  })

  it("should return 1.0 for empty strings", () => {
    expect(wordSimilarity("", "")).toBe(1.0)
  })

  it("should return 0.0 when one is empty and other is not", () => {
    expect(wordSimilarity("hello", "")).toBe(0.0)
    expect(wordSimilarity("", "hello")).toBe(0.0)
  })

  it("should return 0.0 for completely different word sets", () => {
    expect(wordSimilarity("hello world", "foo bar")).toBe(0.0)
  })

  it("should calculate Jaccard index correctly", () => {
    // "a b" and "b c" -> intersection: {b}, union: {a, b, c}
    // Jaccard = 1/3
    expect(wordSimilarity("a b", "b c")).toBeCloseTo(1 / 3)
  })
})

describe("prefixSuffixSimilarity", () => {
  it("should return 1.0 for identical strings", () => {
    expect(prefixSuffixSimilarity("hello", "hello")).toBe(1.0)
  })

  it("should return 0.0 when one string is empty", () => {
    expect(prefixSuffixSimilarity("hello", "")).toBe(0.0)
    expect(prefixSuffixSimilarity("", "hello")).toBe(0.0)
  })

  it("should detect common prefix", () => {
    const sim = prefixSuffixSimilarity("testing", "test")
    expect(sim).toBeGreaterThan(0)
  })

  it("should detect common suffix", () => {
    const sim = prefixSuffixSimilarity("testing", "resting")
    expect(sim).toBeGreaterThan(0)
  })
})

describe("findBestMatch", () => {
  it("should return null for empty haystack", () => {
    expect(findBestMatch("test", [])).toBe(null)
  })

  it("should find exact match", () => {
    const result = findBestMatch("hello", ["world", "hello", "foo"])
    expect(result).not.toBe(null)
    expect(result!.text).toBe("hello")
    expect(result!.ratio).toBe(1.0)
  })

  it("should find best approximate match", () => {
    const result = findBestMatch("hello", ["hallo", "world", "foo"])
    expect(result).not.toBe(null)
    expect(result!.text).toBe("hallo")
  })

  it("should respect threshold", () => {
    const result = findBestMatch("abc", ["xyz"], 0.5) // very different strings
    expect(result).toBe(null)
  })
})

describe("isNativeAvailable", () => {
  it("should return boolean", async () => {
    const available = await isNativeAvailable()
    expect(typeof available).toBe("boolean")
  })
})

describe("similarity (hybrid)", () => {
  it("should return 1.0 for identical strings", async () => {
    const sim = await similarity("hello", "hello")
    expect(sim).toBe(1.0)
  })

  it("should return value between 0 and 1", async () => {
    const sim = await similarity("testing", "resting")
    expect(sim).toBeGreaterThan(0)
    expect(sim).toBeLessThanOrEqual(1)
  })

  it("should match TypeScript implementation for empty strings", async () => {
    const sim = await similarity("", "")
    expect(sim).toBe(1.0)
  })
})

describe("similaritySync", () => {
  it("should match stringSimilarity exactly", () => {
    const pairs = [
      ["hello", "hello"],
      ["test", "testing"],
      ["abc", "xyz"],
    ]
    for (const [a, b] of pairs) {
      expect(similaritySync(a, b)).toBe(stringSimilarity(a, b))
    }
  })
})

describe("bestMatch (hybrid)", () => {
  it("should find exact match", async () => {
    const result = await bestMatch("hello", ["world", "hello", "foo"])
    expect(result).not.toBe(null)
    expect(result!.text).toBe("hello")
  })

  it("should return null for empty haystack", async () => {
    const result = await bestMatch("test", [])
    expect(result).toBe(null)
  })
})

describe("isSimilar", () => {
  it("should return true for identical strings", async () => {
    expect(await isSimilar("hello", "hello")).toBe(true)
  })

  it("should return false for very different strings", async () => {
    expect(await isSimilar("hello", "xyz", 0.8)).toBe(false)
  })

  it("should respect threshold parameter", async () => {
    const a = "test"
    const b = "best"
    // With very low threshold
    expect(await isSimilar(a, b, 0.1)).toBe(true)
    // With very high threshold
    expect(await isSimilar(a, b, 0.99)).toBe(false)
  })
})

describe("isSimilarSync", () => {
  it("should match isSimilar for same inputs", async () => {
    const pairs = [
      { a: "hello", b: "hello", threshold: 0.8 },
      { a: "test", b: "best", threshold: 0.5 },
      { a: "abc", b: "xyz", threshold: 0.8 },
    ]
    for (const { a, b, threshold } of pairs) {
      expect(isSimilarSync(a, b, threshold)).toBe(await isSimilar(a, b, threshold))
    }
  })
})

describe("consistency between native and TypeScript", () => {
  it("should produce similar results for similarity", async () => {
    const isNative = await isNativeAvailable()
    if (!isNative) {
      // Skip native consistency test if not available
      return
    }

    // If native is available, results should be very close
    const testCases = [
      ["hello", "hello"],
      ["testing", "resting"],
      ["algorithm", "altruistic"],
    ]

    for (const [a, b] of testCases) {
      const nativeSim = await similarity(a, b)
      const tsSim = stringSimilarity(a, b)
      // Allow small floating point differences
      expect(Math.abs(nativeSim - tsSim)).toBeLessThan(0.01)
    }
  })
})
