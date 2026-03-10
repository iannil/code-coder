/**
 * Memory module - Text chunking, vector operations, embeddings
 *
 * This module provides TypeScript types and wrappers for:
 * - Text chunking (markdown-aware splitting)
 * - Vector operations (similarity, normalization)
 * - Embedding provider abstraction
 *
 * NOTE: Native bindings are REQUIRED. No JavaScript fallbacks.
 *
 * @example
 * ```typescript
 * import { chunkText, cosineSimilarity, normalizeVector } from '@codecoder-ai/core/memory'
 *
 * // Chunk markdown text
 * const chunks = chunkText('# Title\nContent here...', { maxTokens: 512 })
 *
 * // Calculate similarity
 * const sim = cosineSimilarity([1, 0, 0], [0.9, 0.1, 0])
 *
 * // Normalize vector
 * const normalized = normalizeVector([3, 4])
 * ```
 */

// ============================================================================
// Chunker Types
// ============================================================================

/** Configuration for text chunking */
export interface ChunkerConfig {
  /** Maximum tokens per chunk (~4 chars per token) */
  maxTokens: number
  /** Overlap tokens between chunks */
  overlapTokens: number
  /** Preserve heading context in split chunks */
  preserveHeadings: boolean
}

/** Default chunker configuration */
export const DEFAULT_CHUNKER_CONFIG: ChunkerConfig = {
  maxTokens: 512,
  overlapTokens: 0,
  preserveHeadings: true,
}

/** A chunk of text with metadata */
export interface Chunk {
  /** Chunk index (0-based) */
  index: number
  /** Chunk content */
  content: string
  /** Heading context (if any) */
  heading?: string
  /** Start offset in original text */
  startOffset: number
  /** End offset in original text */
  endOffset: number
}

// ============================================================================
// Vector Types
// ============================================================================

/** Scored result from hybrid search */
export interface ScoredResult {
  /** Result identifier */
  id: string
  /** Vector similarity score */
  vectorScore?: number
  /** Keyword/BM25 score */
  keywordScore?: number
  /** Final combined score */
  finalScore: number
}

/** Vector result for hybrid merge */
export interface VectorResult {
  id: string
  score: number
}

// ============================================================================
// Embedding Types
// ============================================================================

/** Embedding provider configuration */
export interface EmbeddingConfig {
  /** Provider type: "openai", "custom:<url>", "none" */
  provider: string
  /** API key (optional for some providers) */
  apiKey?: string
  /** Model name */
  model: string
  /** Embedding dimensions */
  dimensions: number
}

/** Default embedding configuration */
export const DEFAULT_EMBEDDING_CONFIG: EmbeddingConfig = {
  provider: 'none',
  model: 'text-embedding-3-small',
  dimensions: 1536,
}

/** Embedding provider interface */
export interface EmbeddingProvider {
  /** Provider name */
  name(): string
  /** Embedding dimensions */
  dimensions(): number
  /** Embed a batch of texts */
  embed(texts: string[]): Promise<number[][]>
  /** Embed a single text */
  embedOne(text: string): Promise<number[]>
}

/** No-op embedding provider (for when embeddings are disabled) */
export class NoopEmbeddingProvider implements EmbeddingProvider {
  name(): string {
    return 'none'
  }
  dimensions(): number {
    return 0
  }
  async embed(_texts: string[]): Promise<number[][]> {
    return []
  }
  async embedOne(_text: string): Promise<number[]> {
    throw new Error('No embedding provider configured')
  }
}

/** Estimate token count (~4 chars per token) - pure TypeScript utility */
export function estimateChunkTokens(text: string): number {
  return Math.ceil(text.length / 4)
}

// ============================================================================
// Native Bindings (Required)
// ============================================================================

let nativeBindings: {
  chunkText?: (text: string, maxTokens?: number) => Chunk[]
  chunkTextWithConfig?: (text: string, config: ChunkerConfig) => Chunk[]
  estimateTokens?: (text: string) => number
  cosineSimilarity?: (a: number[], b: number[]) => number
  vectorDistance?: (a: number[], b: number[]) => number
  normalizeVector?: (v: number[]) => number[]
  vectorToBytes?: (v: number[]) => Uint8Array
  bytesToVector?: (bytes: Uint8Array) => number[]
  hybridMergeResults?: (
    vectorResults: VectorResult[],
    keywordResults: VectorResult[],
    vectorWeight: number,
    keywordWeight: number,
    limit: number,
  ) => ScoredResult[]
} | null = null

try {
  nativeBindings = require('../binding.js')
} catch {
  // Native bindings not available - will throw on use
}

// Helper to create a function that throws if native binding is missing
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function requireNative<T>(name: string, fn: T | undefined): T {
  if (fn) return fn
  // Return a function that throws - type assertion needed for flexibility
  const throwFn = (): never => {
    throw new Error(`Native binding required: ${name}. Build native modules with \`cargo build\` in services/zero-core.`)
  }
  return throwFn as unknown as T
}

/** Chunk markdown text */
export const chunkText = requireNative('chunkText', nativeBindings?.chunkText)

/** Chunk markdown text with custom configuration */
export const chunkTextWithConfig = requireNative('chunkTextWithConfig', nativeBindings?.chunkTextWithConfig)

/** Estimate token count for text (native implementation) */
export const estimateChunkTokensNative = requireNative('estimateTokens', nativeBindings?.estimateTokens)

/** Calculate cosine similarity between two vectors */
export const cosineSimilarity = requireNative('cosineSimilarity', nativeBindings?.cosineSimilarity)

/** Calculate Euclidean distance between two vectors */
export const vectorDistance = requireNative('vectorDistance', nativeBindings?.vectorDistance)

/** Normalize a vector to unit length */
export const normalizeVector = requireNative('normalizeVector', nativeBindings?.normalizeVector)

/** Serialize vector to bytes */
export const vectorToBytes = requireNative('vectorToBytes', nativeBindings?.vectorToBytes)

/** Deserialize bytes to vector */
export const bytesToVector = requireNative('bytesToVector', nativeBindings?.bytesToVector)

/** Hybrid merge: combine vector and keyword results */
export const hybridMergeResults = requireNative('hybridMergeResults', nativeBindings?.hybridMergeResults)

/** Create an embedding provider */
export function createEmbeddingProvider(_config: EmbeddingConfig): EmbeddingProvider {
  // For now, always return noop - actual implementations would need async setup
  return new NoopEmbeddingProvider()
}
