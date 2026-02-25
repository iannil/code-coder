/**
 * Embedding Generation Service
 *
 * Provides real embedding generation using various providers:
 * - OpenAI (text-embedding-3-small/large)
 * - Ollama (local models like nomic-embed-text)
 * - Fallback to hash-based embedding for offline use
 *
 * Part of Phase 2: Global Context Hub
 */

import { Log } from "@/util/log"
import { Provider } from "@/provider/provider"
import { Config } from "@/config/config"
import { Env } from "@/env"

const log = Log.create({ service: "memory.embedding-provider" })

// ============================================================================
// Types
// ============================================================================

export interface EmbeddingResult {
  vector: number[]
  dimension: number
  model: string
  tokens?: number
}

export interface EmbeddingProviderConfig {
  /** Preferred provider: openai, ollama, or hash (fallback) */
  provider: "openai" | "ollama" | "hash"
  /** Model name for the provider */
  model?: string
  /** Dimension for the embedding (only for some models) */
  dimension?: number
  /** Ollama base URL */
  ollamaBaseUrl?: string
  /** Cache embeddings in memory */
  enableCache?: boolean
  /** Maximum cache size */
  maxCacheSize?: number
}

const DEFAULT_CONFIG: EmbeddingProviderConfig = {
  provider: "hash",
  model: "text-embedding-3-small",
  dimension: 1536,
  enableCache: true,
  maxCacheSize: 1000,
}

// ============================================================================
// Embedding Provider
// ============================================================================

export class EmbeddingProvider {
  private config: EmbeddingProviderConfig
  private cache: Map<string, EmbeddingResult> = new Map()

  constructor(config?: Partial<EmbeddingProviderConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Generate embedding for a single text
   */
  async embed(text: string): Promise<EmbeddingResult> {
    // Check cache first
    if (this.config.enableCache) {
      const cacheKey = this.getCacheKey(text)
      const cached = this.cache.get(cacheKey)
      if (cached) {
        return cached
      }
    }

    // Generate embedding based on provider
    let result: EmbeddingResult

    switch (this.config.provider) {
      case "openai":
        result = await this.embedWithOpenAI(text)
        break
      case "ollama":
        result = await this.embedWithOllama(text)
        break
      case "hash":
      default:
        result = this.embedWithHash(text)
    }

    // Cache result
    if (this.config.enableCache) {
      this.addToCache(text, result)
    }

    return result
  }

  /**
   * Generate embeddings for multiple texts (batched)
   */
  async embedBatch(texts: string[]): Promise<EmbeddingResult[]> {
    // For OpenAI, we can batch requests
    if (this.config.provider === "openai") {
      return this.embedBatchWithOpenAI(texts)
    }

    // For other providers, process sequentially
    const results: EmbeddingResult[] = []
    for (const text of texts) {
      results.push(await this.embed(text))
    }
    return results
  }

  /**
   * Calculate cosine similarity between two vectors
   */
  cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) return 0

    let dotProduct = 0
    let normA = 0
    let normB = 0

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i]
      normA += a[i] * a[i]
      normB += b[i] * b[i]
    }

    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB) || 1)
  }

  /**
   * Get the current provider type
   */
  getProviderType(): string {
    return this.config.provider
  }

  /**
   * Clear the embedding cache
   */
  clearCache(): void {
    this.cache.clear()
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async embedWithOpenAI(text: string): Promise<EmbeddingResult> {
    try {
      const apiKey = Env.get("OPENAI_API_KEY")
      if (!apiKey) {
        log.warn("OpenAI API key not found, falling back to hash embedding")
        return this.embedWithHash(text)
      }

      const model = this.config.model || "text-embedding-3-small"
      const response = await fetch("https://api.openai.com/v1/embeddings", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          input: text.slice(0, 8191), // Max input length
          model,
          dimensions: this.config.dimension,
        }),
      })

      if (!response.ok) {
        const error = await response.text()
        log.warn("OpenAI embedding failed, falling back to hash", { error })
        return this.embedWithHash(text)
      }

      const data = (await response.json()) as {
        data: Array<{ embedding: number[] }>
        usage: { total_tokens: number }
      }

      return {
        vector: data.data[0].embedding,
        dimension: data.data[0].embedding.length,
        model,
        tokens: data.usage.total_tokens,
      }
    } catch (error) {
      log.warn("OpenAI embedding error, falling back to hash", {
        error: error instanceof Error ? error.message : String(error),
      })
      return this.embedWithHash(text)
    }
  }

  private async embedBatchWithOpenAI(texts: string[]): Promise<EmbeddingResult[]> {
    try {
      const apiKey = Env.get("OPENAI_API_KEY")
      if (!apiKey) {
        return texts.map((t) => this.embedWithHash(t))
      }

      const model = this.config.model || "text-embedding-3-small"
      const truncatedTexts = texts.map((t) => t.slice(0, 8191))

      const response = await fetch("https://api.openai.com/v1/embeddings", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          input: truncatedTexts,
          model,
          dimensions: this.config.dimension,
        }),
      })

      if (!response.ok) {
        return texts.map((t) => this.embedWithHash(t))
      }

      const data = (await response.json()) as {
        data: Array<{ embedding: number[]; index: number }>
        usage: { total_tokens: number }
      }

      // Sort by index to maintain order
      const sorted = data.data.sort((a, b) => a.index - b.index)

      return sorted.map((item) => ({
        vector: item.embedding,
        dimension: item.embedding.length,
        model,
        tokens: Math.floor(data.usage.total_tokens / texts.length),
      }))
    } catch {
      return texts.map((t) => this.embedWithHash(t))
    }
  }

  private async embedWithOllama(text: string): Promise<EmbeddingResult> {
    try {
      const baseUrl = this.config.ollamaBaseUrl || "http://localhost:11434"
      const model = this.config.model || "nomic-embed-text"

      const response = await fetch(`${baseUrl}/api/embeddings`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          model,
          prompt: text.slice(0, 8191),
        }),
      })

      if (!response.ok) {
        log.warn("Ollama embedding failed, falling back to hash")
        return this.embedWithHash(text)
      }

      const data = (await response.json()) as { embedding: number[] }

      return {
        vector: data.embedding,
        dimension: data.embedding.length,
        model,
      }
    } catch (error) {
      log.warn("Ollama embedding error, falling back to hash", {
        error: error instanceof Error ? error.message : String(error),
      })
      return this.embedWithHash(text)
    }
  }

  private embedWithHash(text: string): EmbeddingResult {
    const dimension = this.config.dimension || 1536
    const vector = new Float32Array(dimension)

    // Deterministic hash for reproducibility
    let hash = 0
    for (let i = 0; i < text.length; i++) {
      const char = text.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash = hash & hash
    }

    const seed = Math.abs(hash)

    // Generate pseudo-random vector from hash
    for (let i = 0; i < dimension; i++) {
      const x = Math.sin(seed + i) * 10000
      vector[i] = x - Math.floor(x)
    }

    // Add word-level features
    const words = text.toLowerCase().split(/\s+/)
    for (const word of words) {
      let wordHash = 0
      for (let i = 0; i < word.length; i++) {
        wordHash = (wordHash << 5) - wordHash + word.charCodeAt(i)
        wordHash = wordHash & wordHash
      }
      const idx = Math.abs(wordHash) % dimension
      vector[idx] += 0.1
    }

    // Normalize
    const norm = Math.sqrt(vector.reduce((sum, val) => sum + val * val, 0))
    for (let i = 0; i < dimension; i++) {
      vector[i] /= norm || 1
    }

    return {
      vector: Array.from(vector),
      dimension,
      model: "hash",
    }
  }

  private getCacheKey(text: string): string {
    // Simple hash for cache key
    let hash = 0
    for (let i = 0; i < text.length; i++) {
      const char = text.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash = hash & hash
    }
    return `${this.config.provider}:${this.config.model}:${hash}`
  }

  private addToCache(text: string, result: EmbeddingResult): void {
    const key = this.getCacheKey(text)

    // Evict oldest entries if cache is full
    if (this.cache.size >= (this.config.maxCacheSize || 1000)) {
      const firstKey = this.cache.keys().next().value
      if (firstKey) {
        this.cache.delete(firstKey)
      }
    }

    this.cache.set(key, result)
  }
}

// ============================================================================
// Factory
// ============================================================================

let defaultProvider: EmbeddingProvider | null = null

/**
 * Get the default embedding provider
 *
 * Auto-detects the best available provider:
 * 1. OpenAI if OPENAI_API_KEY is set
 * 2. Ollama if running locally
 * 3. Hash-based fallback
 */
export async function getEmbeddingProvider(): Promise<EmbeddingProvider> {
  if (defaultProvider) {
    return defaultProvider
  }

  // Check for OpenAI key
  const openaiKey = Env.get("OPENAI_API_KEY")
  if (openaiKey) {
    log.info("Using OpenAI embedding provider")
    defaultProvider = new EmbeddingProvider({
      provider: "openai",
      model: "text-embedding-3-small",
      dimension: 1536,
    })
    return defaultProvider
  }

  // Check for Ollama
  try {
    const response = await fetch("http://localhost:11434/api/tags", {
      method: "GET",
      signal: AbortSignal.timeout(1000),
    })
    if (response.ok) {
      log.info("Using Ollama embedding provider")
      defaultProvider = new EmbeddingProvider({
        provider: "ollama",
        model: "nomic-embed-text",
      })
      return defaultProvider
    }
  } catch {
    // Ollama not available
  }

  // Fallback to hash
  log.info("Using hash-based embedding provider (no API key or Ollama)")
  defaultProvider = new EmbeddingProvider({
    provider: "hash",
    dimension: 1536,
  })
  return defaultProvider
}

/**
 * Create a new embedding provider with custom config
 */
export function createEmbeddingProvider(config: Partial<EmbeddingProviderConfig>): EmbeddingProvider {
  return new EmbeddingProvider(config)
}

/**
 * Reset the default provider (useful for testing)
 */
export function resetEmbeddingProvider(): void {
  defaultProvider = null
}
