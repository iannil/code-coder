import { estimateTokens as napiEstimateTokens } from "@codecoder-ai/core"

// Fail fast if NAPI binding is not available (no fallback strategy)
if (typeof napiEstimateTokens !== "function") {
  throw new Error("NAPI binding 'estimateTokens' not available. Ensure @codecoder-ai/core is properly built.")
}

// Store verified function reference (type-narrowed after check)
const estimateTokens: (text: string) => number = napiEstimateTokens

export namespace Token {
  /**
   * Estimate token count for text using native Rust BPE-based estimation.
   */
  export function estimate(input: string): number {
    const text = input || ""
    if (text.length === 0) return 0
    return estimateTokens(text)
  }
}
