import { estimateTokens } from "@codecoder-ai/core"

export namespace Token {
  // Fallback constant for when native is unavailable
  const CHARS_PER_TOKEN = 4

  /**
   * Estimate token count for text.
   * Uses native Rust implementation when available, falls back to char/4 heuristic.
   */
  export function estimate(input: string): number {
    const text = input || ""
    if (text.length === 0) return 0

    // Use native implementation if available (more accurate BPE-based estimation)
    if (estimateTokens) {
      return estimateTokens(text)
    }

    // Fallback: simple character-based estimation
    return Math.max(0, Math.round(text.length / CHARS_PER_TOKEN))
  }
}
