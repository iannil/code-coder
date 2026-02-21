import { Log } from "@/util/log"
import { Provider } from "@/provider/provider"
import { generateText } from "ai"

const log = Log.create({ service: "bootstrap.compression" })

/**
 * PromptCompression module handles automatic compression of learned patterns
 * to reduce token usage while preserving semantic meaning.
 */
export namespace PromptCompression {
  /**
   * Result of a compression operation
   */
  export interface CompressionResult {
    original: string
    compressed: string
    originalTokens: number
    compressedTokens: number
    compressionRatio: number
    semanticSimilarity: number
  }

  /**
   * Minimum semantic similarity required (90%)
   */
  const MIN_SEMANTIC_SIMILARITY = 0.9

  /**
   * Target compression ratio
   */
  const TARGET_COMPRESSION_RATIO = 0.7

  /**
   * Estimate token count (rough approximation)
   * In production, use the actual tokenizer for the model
   */
  function estimateTokens(text: string): number {
    // Rough approximation: ~4 chars per token on average
    return Math.ceil(text.length / 4)
  }

  /**
   * Compress a skill prompt while preserving semantics
   */
  export async function compress(content: string): Promise<CompressionResult> {
    const originalTokens = estimateTokens(content)

    // Skip compression for short content
    if (originalTokens < 100) {
      return {
        original: content,
        compressed: content,
        originalTokens,
        compressedTokens: originalTokens,
        compressionRatio: 1,
        semanticSimilarity: 1,
      }
    }

    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const prompt = `Compress the following text to be more concise while preserving ALL key information and semantic meaning.

Rules:
- Remove redundant words and phrases
- Use abbreviations where clear
- Combine similar concepts
- Keep all technical terms intact
- Preserve the structure/steps if present
- Keep it actionable and clear

Original text:
${content}

Return ONLY the compressed text, no explanations or formatting.`

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: Math.ceil(originalTokens * 0.8),
        temperature: 0.2,
      })

      const compressed = result.text.trim()
      const compressedTokens = estimateTokens(compressed)
      const compressionRatio = compressedTokens / originalTokens

      // Verify semantic similarity
      const similarity = await verifySemantic(content, compressed)

      // If compression is too aggressive or loses meaning, fall back
      if (similarity < MIN_SEMANTIC_SIMILARITY || compressedTokens >= originalTokens) {
        log.info("compression rejected", {
          reason: similarity < MIN_SEMANTIC_SIMILARITY ? "low similarity" : "no reduction",
          similarity,
          originalTokens,
          compressedTokens,
        })

        return {
          original: content,
          compressed: content,
          originalTokens,
          compressedTokens: originalTokens,
          compressionRatio: 1,
          semanticSimilarity: 1,
        }
      }

      log.info("compressed prompt", {
        originalTokens,
        compressedTokens,
        ratio: compressionRatio,
        similarity,
      })

      return {
        original: content,
        compressed,
        originalTokens,
        compressedTokens,
        compressionRatio,
        semanticSimilarity: similarity,
      }
    } catch (error) {
      log.warn("compression failed", { error })
      return {
        original: content,
        compressed: content,
        originalTokens,
        compressedTokens: originalTokens,
        compressionRatio: 1,
        semanticSimilarity: 1,
      }
    }
  }

  /**
   * Verify semantic similarity between original and compressed
   */
  async function verifySemantic(original: string, compressed: string): Promise<number> {
    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      const prompt = `Compare these two texts and rate their semantic similarity from 0.0 to 1.0.
1.0 = identical meaning, 0.0 = completely different.

Text A:
${original.slice(0, 500)}

Text B:
${compressed.slice(0, 500)}

Return ONLY a decimal number between 0.0 and 1.0, nothing else.`

      const result = await generateText({
        model: languageModel,
        prompt,
        maxOutputTokens: 10,
        temperature: 0.1,
      })

      const similarity = parseFloat(result.text.trim())
      return isNaN(similarity) ? 0.8 : Math.max(0, Math.min(1, similarity))
    } catch {
      // Default to 0.8 if verification fails
      return 0.8
    }
  }

  /**
   * Verify that compressed prompt produces equivalent results
   */
  export async function verifyEquivalence(
    original: string,
    compressed: string,
    testCases: string[],
  ): Promise<boolean> {
    if (testCases.length === 0) {
      // No test cases, rely on semantic similarity
      const similarity = await verifySemantic(original, compressed)
      return similarity >= MIN_SEMANTIC_SIMILARITY
    }

    try {
      const model = await Provider.defaultModel()
      const languageModel = await Provider.getLanguage(
        await Provider.getModel(model.providerID, model.modelID),
      )

      let matches = 0

      for (const testCase of testCases.slice(0, 3)) {
        // Test with original
        const originalResult = await generateText({
          model: languageModel,
          prompt: `${original}\n\nApply to: ${testCase}`,
          maxOutputTokens: 200,
          temperature: 0.1,
        })

        // Test with compressed
        const compressedResult = await generateText({
          model: languageModel,
          prompt: `${compressed}\n\nApply to: ${testCase}`,
          maxOutputTokens: 200,
          temperature: 0.1,
        })

        // Compare results
        const similarity = await verifySemantic(originalResult.text, compressedResult.text)
        if (similarity >= 0.85) {
          matches++
        }
      }

      return matches >= testCases.slice(0, 3).length * 0.66
    } catch {
      return false
    }
  }

  /**
   * Iteratively compress until target token count or cannot compress further
   */
  export async function iterativeCompress(
    content: string,
    targetTokens: number,
  ): Promise<CompressionResult> {
    let current = content
    let bestResult: CompressionResult = {
      original: content,
      compressed: content,
      originalTokens: estimateTokens(content),
      compressedTokens: estimateTokens(content),
      compressionRatio: 1,
      semanticSimilarity: 1,
    }

    const maxIterations = 3
    let iteration = 0

    while (
      estimateTokens(current) > targetTokens &&
      iteration < maxIterations
    ) {
      iteration++

      const result = await compress(current)

      // If no progress, stop
      if (result.compressionRatio >= 0.95) {
        break
      }

      // If semantic similarity drops, stop
      if (result.semanticSimilarity < MIN_SEMANTIC_SIMILARITY) {
        break
      }

      current = result.compressed
      bestResult = {
        original: content,
        compressed: current,
        originalTokens: estimateTokens(content),
        compressedTokens: estimateTokens(current),
        compressionRatio: estimateTokens(current) / estimateTokens(content),
        semanticSimilarity: result.semanticSimilarity,
      }

      log.info("iterative compression", {
        iteration,
        currentTokens: estimateTokens(current),
        targetTokens,
      })
    }

    return bestResult
  }
}
