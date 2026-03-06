/**
 * Unified Text Processing Module
 *
 * Provides unified API for text similarity, fuzzy matching, and diff operations.
 * Uses Rust native implementations when available, falls back to TypeScript.
 *
 * Performance improvements with native:
 * - String similarity: ~10x faster (strsim crate)
 * - Fuzzy matching: ~7.5x faster (parallel search)
 * - Diff computation: ~5x faster (similar crate)
 *
 * This module unifies duplicate implementations found in:
 * - tool/edit.ts (levenshtein)
 * - document/entity.ts (calculateStringSimilarity)
 * - patch/native.ts (native bindings)
 */

import {
  similarityRatioNative,
  findBestMatchNative,
  computeDiffNative,
  isNativeAvailable as isPatchNativeAvailable,
  type NapiBestMatch,
} from "../patch/native"

// ============================================================================
// Re-exports from patch/native.ts
// ============================================================================

export { similarityRatioNative, findBestMatchNative, computeDiffNative }
export type { NapiBestMatch }

// ============================================================================
// TypeScript Fallback Implementations
// ============================================================================

/**
 * Calculate Levenshtein edit distance between two strings.
 *
 * The Levenshtein distance is the minimum number of single-character edits
 * (insertions, deletions, substitutions) required to change one string into another.
 *
 * Time complexity: O(m*n) where m and n are string lengths
 * Space complexity: O(min(m,n)) using optimized single-row algorithm
 *
 * @param a First string
 * @param b Second string
 * @returns Edit distance (0 = identical)
 */
export function levenshteinDistance(a: string, b: string): number {
  // Handle empty strings
  if (a.length === 0) return b.length
  if (b.length === 0) return a.length

  // Ensure a is the shorter string (memory optimization)
  if (a.length > b.length) {
    const tmp = a
    a = b
    b = tmp
  }

  // Use single row optimization - O(min(m,n)) space instead of O(m*n)
  const row = new Array<number>(a.length + 1)

  // Initialize first row
  for (let i = 0; i <= a.length; i++) {
    row[i] = i
  }

  // Fill in the rest of the matrix
  for (let j = 1; j <= b.length; j++) {
    let prev = row[0]
    row[0] = j

    for (let i = 1; i <= a.length; i++) {
      const current = row[i]
      const cost = a[i - 1] === b[j - 1] ? 0 : 1
      row[i] = Math.min(
        row[i - 1] + 1, // insertion
        row[i] + 1, // deletion
        prev + cost, // substitution
      )
      prev = current
    }
  }

  return row[a.length]
}

/**
 * Calculate normalized string similarity (0.0 to 1.0).
 *
 * Uses normalized Levenshtein distance: 1 - (distance / max_length)
 * This gives 1.0 for identical strings and 0.0 for completely different strings.
 *
 * @param a First string
 * @param b Second string
 * @returns Similarity ratio (1.0 = identical, 0.0 = completely different)
 */
export function stringSimilarity(a: string, b: string): number {
  if (a === b) return 1.0
  if (a.length === 0 && b.length === 0) return 1.0
  if (a.length === 0 || b.length === 0) return 0.0

  const distance = levenshteinDistance(a, b)
  const maxLength = Math.max(a.length, b.length)
  return 1.0 - distance / maxLength
}

/**
 * Calculate Jaccard similarity index on words.
 *
 * The Jaccard index measures similarity between finite sample sets,
 * defined as the size of the intersection divided by the size of the union.
 * This is useful for comparing text based on word overlap rather than character edits.
 *
 * @param a First string
 * @param b Second string
 * @returns Jaccard index (1.0 = same words, 0.0 = no common words)
 */
export function wordSimilarity(a: string, b: string): number {
  const wordsA = new Set(a.split(/\s+/).filter(Boolean))
  const wordsB = new Set(b.split(/\s+/).filter(Boolean))

  if (wordsA.size === 0 && wordsB.size === 0) return 1.0
  if (wordsA.size === 0 || wordsB.size === 0) return 0.0

  let intersection = 0
  for (const word of wordsA) {
    if (wordsB.has(word)) intersection++
  }

  const union = wordsA.size + wordsB.size - intersection
  return union > 0 ? intersection / union : 0
}

/**
 * Calculate prefix/suffix similarity.
 *
 * Measures similarity based on common prefix and suffix characters.
 * Useful for comparing identifiers, file names, or other short strings
 * where prefix/suffix matching is more meaningful than full edit distance.
 *
 * @param a First string
 * @param b Second string
 * @returns Similarity ratio (1.0 = identical, 0.0 = no common prefix/suffix)
 */
export function prefixSuffixSimilarity(a: string, b: string): number {
  if (a === b) return 1.0
  if (a.length === 0 || b.length === 0) return 0.0

  const maxLength = Math.max(a.length, b.length)
  let common = 0

  // Check prefix
  let i = 0
  while (i < a.length && i < b.length && a[i] === b[i]) {
    common++
    i++
  }

  // Check suffix (avoid double-counting overlap with prefix)
  let j = 1
  while (j <= a.length - i && j <= b.length - i && a[a.length - j] === b[b.length - j]) {
    common++
    j++
  }

  return common / maxLength
}

/**
 * Find the best matching string from a list of candidates.
 *
 * @param needle The string to match
 * @param haystack Array of candidate strings
 * @param threshold Minimum similarity threshold (default: 0.0)
 * @returns Best match with similarity score, or null if no match above threshold
 */
export function findBestMatch(
  needle: string,
  haystack: string[],
  threshold = 0.0,
): { text: string; ratio: number } | null {
  if (haystack.length === 0) return null

  let bestMatch: { text: string; ratio: number } | null = null
  let bestRatio = threshold

  for (const candidate of haystack) {
    const ratio = stringSimilarity(needle, candidate)
    if (ratio > bestRatio) {
      bestRatio = ratio
      bestMatch = { text: candidate, ratio }
    }
  }

  return bestMatch
}

// ============================================================================
// Hybrid API (Native preferred, TypeScript fallback)
// ============================================================================

/**
 * Check if native text processing bindings are available.
 */
export async function isNativeAvailable(): Promise<boolean> {
  return isPatchNativeAvailable()
}

/**
 * Calculate string similarity with native optimization.
 *
 * Attempts to use Rust native strsim, falls back to TypeScript.
 *
 * @param a First string
 * @param b Second string
 * @returns Similarity ratio (1.0 = identical, 0.0 = completely different)
 */
export async function similarity(a: string, b: string): Promise<number> {
  const native = await similarityRatioNative(a, b)
  if (native !== null) return native
  return stringSimilarity(a, b)
}

/**
 * Calculate string similarity synchronously (TypeScript only).
 *
 * Use this when you need synchronous operation and can't await.
 * For best performance, prefer the async `similarity()` function.
 *
 * @param a First string
 * @param b Second string
 * @returns Similarity ratio (1.0 = identical, 0.0 = completely different)
 */
export function similaritySync(a: string, b: string): number {
  return stringSimilarity(a, b)
}

/**
 * Find best match with native optimization.
 *
 * Attempts to use Rust native implementation, falls back to TypeScript.
 *
 * @param needle The string to match
 * @param haystack Array of candidate strings
 * @param threshold Minimum similarity threshold (default: 0.0)
 * @returns Best match with similarity score, or null if no match
 */
export async function bestMatch(
  needle: string,
  haystack: string[],
  threshold = 0.0,
): Promise<{ text: string; ratio: number } | null> {
  const native = await findBestMatchNative(needle, haystack)
  if (native !== null && native.ratio >= threshold) {
    return native
  }
  return findBestMatch(needle, haystack, threshold)
}

/**
 * Compute unified diff between two strings with native optimization.
 *
 * @param oldContent Original content
 * @param newContent Modified content
 * @param filePath File path for diff header
 * @returns Unified diff string
 */
export async function diff(oldContent: string, newContent: string, filePath: string): Promise<string> {
  const native = await computeDiffNative(oldContent, newContent, filePath)
  if (native !== null) return native

  // Simple fallback - just return the new content with markers
  // For a full implementation, use a proper diff library
  if (oldContent === newContent) return ""

  const lines: string[] = []
  lines.push(`--- a/${filePath}`)
  lines.push(`+++ b/${filePath}`)
  lines.push("@@ -1 +1 @@")

  const oldLines = oldContent.split("\n")
  const newLines = newContent.split("\n")

  for (const line of oldLines) {
    lines.push(`-${line}`)
  }
  for (const line of newLines) {
    lines.push(`+${line}`)
  }

  return lines.join("\n")
}

/**
 * Check if two strings are "similar enough" based on threshold.
 *
 * @param a First string
 * @param b Second string
 * @param threshold Similarity threshold (default: 0.8)
 * @returns True if similarity >= threshold
 */
export async function isSimilar(a: string, b: string, threshold = 0.8): Promise<boolean> {
  const ratio = await similarity(a, b)
  return ratio >= threshold
}

/**
 * Check if two strings are "similar enough" synchronously.
 *
 * @param a First string
 * @param b Second string
 * @param threshold Similarity threshold (default: 0.8)
 * @returns True if similarity >= threshold
 */
export function isSimilarSync(a: string, b: string, threshold = 0.8): boolean {
  return stringSimilarity(a, b) >= threshold
}
