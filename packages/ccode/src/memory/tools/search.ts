/**
 * Dynamic Tool Search
 *
 * Provides semantic search and discovery for dynamic tools.
 * Uses hybrid retrieval combining vector similarity, keyword matching, and tag filtering.
 *
 * Part of Phase 12: Dynamic Tool Library
 */

import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/storage/storage"
import { ToolTypes } from "./types"
import { ToolRegistry } from "./registry"

const log = Log.create({ service: "memory.tools.search" })

export namespace ToolSearch {
  // ============================================================================
  // Constants
  // ============================================================================

  const EMBEDDING_DIMENSION = 256 // Smaller dimension for tool search
  const STORAGE_PREFIX = ["memory", "tools", "embeddings"]

  // ============================================================================
  // Scoring Weights
  // ============================================================================

  const WEIGHTS = {
    semantic: 0.5, // Vector similarity weight
    keyword: 0.3, // Keyword match weight
    usage: 0.2, // Usage statistics weight
  }

  // ============================================================================
  // Main Search Function
  // ============================================================================

  /**
   * Search for tools matching a query
   */
  export async function search(
    query: string,
    options?: Partial<ToolTypes.SearchOptions>,
  ): Promise<ToolTypes.ScoredTool[]> {
    const opts = ToolTypes.SearchOptions.parse(options ?? {})

    // Get all tools (filtered by language/tags if specified)
    const allTools = await ToolRegistry.list({
      language: opts.language,
      tags: opts.tags,
    })

    if (allTools.length === 0) {
      return []
    }

    // Generate query embedding
    const queryEmbedding = generateEmbedding(query)
    const queryKeywords = extractKeywords(query)

    // Score each tool
    const scored: ToolTypes.ScoredTool[] = []

    for (const tool of allTools) {
      const score = calculateScore(tool, queryEmbedding, queryKeywords)

      if (score >= opts.minScore) {
        scored.push({ tool, score })
      }
    }

    // Sort by score descending
    scored.sort((a, b) => b.score - a.score)

    // Apply limit
    return scored.slice(0, opts.limit)
  }

  /**
   * Find tools by exact tag match
   */
  export async function findByTags(
    tags: string[],
    options?: { limit?: number },
  ): Promise<ToolTypes.DynamicTool[]> {
    return ToolRegistry.list({
      tags,
      limit: options?.limit,
    })
  }

  /**
   * Find tools by language
   */
  export async function findByLanguage(
    language: ToolTypes.DynamicTool["language"],
    options?: { limit?: number },
  ): Promise<ToolTypes.DynamicTool[]> {
    return ToolRegistry.list({
      language,
      limit: options?.limit,
    })
  }

  /**
   * Find similar tools to a given tool
   */
  export async function findSimilar(
    toolId: string,
    options?: { limit?: number; minScore?: number },
  ): Promise<ToolTypes.ScoredTool[]> {
    const tool = await ToolRegistry.get(toolId)
    if (!tool) return []

    // Search using the tool's description
    const results = await search(tool.description, {
      limit: (options?.limit ?? 5) + 1, // +1 to exclude self
      minScore: options?.minScore ?? 0.5,
    })

    // Filter out the original tool
    return results.filter((r) => r.tool.id !== toolId)
  }

  /**
   * Get recommended tools based on recent usage patterns
   */
  export async function getRecommendations(
    options?: { limit?: number },
  ): Promise<ToolTypes.DynamicTool[]> {
    const limit = options?.limit ?? 5

    // Get tools sorted by a composite score of usage and success rate
    const allTools = await ToolRegistry.list()

    const withScore = allTools.map((tool) => {
      const successRate =
        tool.stats.usageCount > 0
          ? tool.stats.successCount / tool.stats.usageCount
          : 0.5

      // Composite score: usage * success rate, with recency boost
      const recencyBoost = tool.stats.lastUsedAt
        ? Math.exp(-(Date.now() - tool.stats.lastUsedAt) / (7 * 24 * 60 * 60 * 1000))
        : 0.5

      const score = tool.stats.usageCount * successRate * recencyBoost

      return { tool, score }
    })

    withScore.sort((a, b) => b.score - a.score)

    return withScore.slice(0, limit).map((x) => x.tool)
  }

  // ============================================================================
  // Embedding Management
  // ============================================================================

  /**
   * Update embedding for a tool (called after tool creation/update)
   */
  export async function updateEmbedding(toolId: string): Promise<void> {
    const tool = await ToolRegistry.get(toolId)
    if (!tool) return

    const text = `${tool.name} ${tool.description} ${tool.tags.join(" ")}`
    const embedding = generateEmbedding(text)

    const projectID = Instance.project.id
    await Storage.write([...STORAGE_PREFIX, projectID, toolId], {
      toolId,
      embedding,
      updatedAt: Date.now(),
    })
  }

  /**
   * Get cached embedding for a tool
   */
  async function getCachedEmbedding(toolId: string): Promise<number[] | undefined> {
    const projectID = Instance.project.id

    try {
      const cached = await Storage.read<{ embedding: number[] }>([
        ...STORAGE_PREFIX,
        projectID,
        toolId,
      ])
      return cached.embedding
    } catch {
      return undefined
    }
  }

  /**
   * Rebuild all embeddings
   */
  export async function rebuildEmbeddings(): Promise<number> {
    const tools = await ToolRegistry.list()
    let updated = 0

    for (const tool of tools) {
      await updateEmbedding(tool.id)
      updated++
    }

    log.info("Embeddings rebuilt", { count: updated })
    return updated
  }

  // ============================================================================
  // Scoring Functions
  // ============================================================================

  function calculateScore(
    tool: ToolTypes.DynamicTool,
    queryEmbedding: number[],
    queryKeywords: string[],
  ): number {
    // Semantic similarity
    const toolText = `${tool.name} ${tool.description} ${tool.tags.join(" ")}`
    const toolEmbedding = generateEmbedding(toolText)
    const semanticScore = cosineSimilarity(queryEmbedding, toolEmbedding)

    // Keyword matching
    const keywordScore = calculateKeywordScore(tool, queryKeywords)

    // Usage score (normalized)
    const usageScore = calculateUsageScore(tool)

    // Weighted combination
    const finalScore =
      WEIGHTS.semantic * semanticScore +
      WEIGHTS.keyword * keywordScore +
      WEIGHTS.usage * usageScore

    return Math.min(1, Math.max(0, finalScore))
  }

  function calculateKeywordScore(
    tool: ToolTypes.DynamicTool,
    keywords: string[],
  ): number {
    if (keywords.length === 0) return 0

    const toolText = `${tool.name} ${tool.description} ${tool.tags.join(" ")}`.toLowerCase()
    let matches = 0

    for (const keyword of keywords) {
      if (toolText.includes(keyword.toLowerCase())) {
        matches++

        // Bonus for exact name match
        if (tool.name.toLowerCase().includes(keyword.toLowerCase())) {
          matches += 0.5
        }
      }
    }

    return Math.min(1, matches / keywords.length)
  }

  function calculateUsageScore(tool: ToolTypes.DynamicTool): number {
    const { usageCount, successCount, lastUsedAt } = tool.stats

    if (usageCount === 0) return 0.5 // Neutral for unused tools

    // Success rate component
    const successRate = successCount / usageCount

    // Recency component (decay over 30 days)
    const recencyDecay = lastUsedAt
      ? Math.exp(-(Date.now() - lastUsedAt) / (30 * 24 * 60 * 60 * 1000))
      : 0.5

    // Usage frequency component (log scale, capped at 100)
    const frequencyScore = Math.min(1, Math.log10(usageCount + 1) / 2)

    return (successRate + recencyDecay + frequencyScore) / 3
  }

  // ============================================================================
  // Embedding Generation
  // ============================================================================

  /**
   * Generate a simple hash-based embedding for text
   * (Can be upgraded to use real embeddings via LLM API)
   */
  function generateEmbedding(text: string): number[] {
    const vector = new Float32Array(EMBEDDING_DIMENSION)

    // Initialize with hash-based values
    let hash = 0
    for (let i = 0; i < text.length; i++) {
      const char = text.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash = hash & hash
    }

    const seed = Math.abs(hash)
    for (let i = 0; i < EMBEDDING_DIMENSION; i++) {
      const x = Math.sin(seed + i) * 10000
      vector[i] = x - Math.floor(x)
    }

    // Add word-level features
    const words = text.toLowerCase().split(/\s+/)
    for (const word of words) {
      if (word.length < 2) continue

      let wordHash = 0
      for (let i = 0; i < word.length; i++) {
        wordHash = (wordHash << 5) - wordHash + word.charCodeAt(i)
        wordHash = wordHash & wordHash
      }

      const idx = Math.abs(wordHash) % EMBEDDING_DIMENSION
      vector[idx] += 0.1
    }

    // Add character n-gram features for better precision
    const ngrams = extractNgrams(text, 3)
    for (const ngram of ngrams) {
      let ngramHash = 0
      for (let i = 0; i < ngram.length; i++) {
        ngramHash = (ngramHash << 5) - ngramHash + ngram.charCodeAt(i)
        ngramHash = ngramHash & ngramHash
      }
      const idx = Math.abs(ngramHash) % EMBEDDING_DIMENSION
      vector[idx] += 0.05
    }

    // Normalize to unit vector
    const norm = Math.sqrt(vector.reduce((sum, val) => sum + val * val, 0))
    for (let i = 0; i < EMBEDDING_DIMENSION; i++) {
      vector[i] /= norm || 1
    }

    return Array.from(vector)
  }

  // ============================================================================
  // Vector Operations
  // ============================================================================

  function cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) return 0

    let dotProduct = 0
    let normA = 0
    let normB = 0

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i]
      normA += a[i] * a[i]
      normB += b[i] * b[i]
    }

    const denominator = Math.sqrt(normA) * Math.sqrt(normB)
    return denominator > 0 ? dotProduct / denominator : 0
  }

  // ============================================================================
  // Text Processing Helpers
  // ============================================================================

  function extractKeywords(text: string): string[] {
    // Simple keyword extraction
    const stopwords = new Set([
      "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
      "have", "has", "had", "do", "does", "did", "will", "would", "could",
      "should", "may", "might", "must", "shall", "can", "need", "dare",
      "to", "of", "in", "for", "on", "with", "at", "by", "from", "as",
      "into", "through", "during", "before", "after", "above", "below",
      "between", "under", "again", "further", "then", "once", "here",
      "there", "when", "where", "why", "how", "all", "each", "few",
      "more", "most", "other", "some", "such", "no", "nor", "not",
      "only", "own", "same", "so", "than", "too", "very", "just",
      "i", "me", "my", "myself", "we", "our", "ours", "ourselves",
      "you", "your", "yours", "yourself", "yourselves", "he", "him",
      "his", "himself", "she", "her", "hers", "herself", "it", "its",
      "itself", "they", "them", "their", "theirs", "themselves",
      "what", "which", "who", "whom", "this", "that", "these", "those",
      "and", "but", "if", "or", "because", "until", "while",
    ])

    const words = text
      .toLowerCase()
      .replace(/[^a-z0-9\s]/g, " ")
      .split(/\s+/)
      .filter((w) => w.length > 2 && !stopwords.has(w))

    // Return unique keywords
    return [...new Set(words)]
  }

  function extractNgrams(text: string, n: number): string[] {
    const normalized = text.toLowerCase().replace(/[^a-z0-9]/g, "")
    const ngrams: string[] = []

    for (let i = 0; i <= normalized.length - n; i++) {
      ngrams.push(normalized.slice(i, i + n))
    }

    return ngrams
  }

  // ============================================================================
  // Cleanup
  // ============================================================================

  /**
   * Clear all cached embeddings
   */
  export async function clearEmbeddings(): Promise<void> {
    const projectID = Instance.project.id

    try {
      const keys = await Storage.list([...STORAGE_PREFIX, projectID])
      for (const key of keys) {
        await Storage.remove(key)
      }
    } catch {
      // Ignore errors
    }
  }

  /**
   * Invalidate search cache
   */
  export async function invalidate(): Promise<void> {
    await clearEmbeddings()
  }
}
