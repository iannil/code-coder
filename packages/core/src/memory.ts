/**
 * Memory module - Text chunking, vector operations, embeddings
 *
 * This module provides TypeScript types and wrappers for:
 * - Text chunking (markdown-aware splitting)
 * - Vector operations (similarity, normalization)
 * - Embedding provider abstraction
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

// ============================================================================
// Fallback Implementations
// ============================================================================

/** Chunk markdown text (fallback implementation) */
export function chunkTextFallback(text: string, maxTokens: number = 512): Chunk[] {
  if (!text.trim()) return []

  const maxChars = maxTokens * 4
  const sections = splitOnHeadings(text)
  const chunks: Chunk[] = []
  let offset = 0

  for (const [heading, body] of sections) {
    const full = heading ? `${heading}\n${body}` : body
    const sectionLen = full.length

    if (full.length <= maxChars) {
      if (full.trim()) {
        chunks.push({
          index: chunks.length,
          content: full.trim(),
          heading: heading ?? undefined,
          startOffset: offset,
          endOffset: offset + sectionLen,
        })
      }
    } else {
      // Split on paragraphs
      const paragraphs = splitOnBlankLines(body)
      let current = heading ? `${heading}\n` : ''
      let currentStart = offset

      for (const para of paragraphs) {
        if (current.length + para.length > maxChars && current.trim()) {
          chunks.push({
            index: chunks.length,
            content: current.trim(),
            heading: heading ?? undefined,
            startOffset: currentStart,
            endOffset: currentStart + current.length,
          })
          currentStart += current.length
          current = heading ? `${heading}\n` : ''
        }
        current += para + '\n'
      }

      if (current.trim()) {
        chunks.push({
          index: chunks.length,
          content: current.trim(),
          heading: heading ?? undefined,
          startOffset: currentStart,
          endOffset: currentStart + current.length,
        })
      }
    }

    offset += sectionLen
  }

  // Re-index
  chunks.forEach((c, i) => (c.index = i))
  return chunks
}

function splitOnHeadings(text: string): [string | null, string][] {
  const sections: [string | null, string][] = []
  let currentHeading: string | null = null
  let currentBody = ''

  for (const line of text.split('\n')) {
    if (line.startsWith('# ') || line.startsWith('## ') || line.startsWith('### ')) {
      if (currentBody.trim() || currentHeading) {
        sections.push([currentHeading, currentBody])
        currentBody = ''
      }
      currentHeading = line
    } else {
      currentBody += line + '\n'
    }
  }

  if (currentBody.trim() || currentHeading) {
    sections.push([currentHeading, currentBody])
  }

  return sections
}

function splitOnBlankLines(text: string): string[] {
  const paragraphs: string[] = []
  let current = ''

  for (const line of text.split('\n')) {
    if (!line.trim()) {
      if (current.trim()) {
        paragraphs.push(current)
        current = ''
      }
    } else {
      current += line + '\n'
    }
  }

  if (current.trim()) {
    paragraphs.push(current)
  }

  return paragraphs
}

/** Estimate token count (~4 chars per token) */
export function estimateChunkTokens(text: string): number {
  return Math.ceil(text.length / 4)
}

/** Calculate cosine similarity (fallback implementation) */
export function cosineSimilarityFallback(a: number[], b: number[]): number {
  if (a.length !== b.length || a.length === 0) return 0

  let dot = 0
  let normA = 0
  let normB = 0

  for (let i = 0; i < a.length; i++) {
    const ai = a[i]!
    const bi = b[i]!
    dot += ai * bi
    normA += ai * ai
    normB += bi * bi
  }

  const denom = Math.sqrt(normA) * Math.sqrt(normB)
  if (denom < 1e-10) return 0

  return Math.max(0, Math.min(1, dot / denom))
}

/** Calculate Euclidean distance (fallback implementation) */
export function vectorDistanceFallback(a: number[], b: number[]): number {
  if (a.length !== b.length || a.length === 0) return Infinity

  let sum = 0
  for (let i = 0; i < a.length; i++) {
    const diff = a[i]! - b[i]!
    sum += diff * diff
  }

  return Math.sqrt(sum)
}

/** Normalize vector to unit length (fallback implementation) */
export function normalizeVectorFallback(v: number[]): number[] {
  const norm = Math.sqrt(v.reduce((sum, x) => sum + x * x, 0))
  if (norm < 1e-10) return v.map(() => 0)
  return v.map((x) => x / norm)
}

/** Serialize vector to bytes (fallback implementation) */
export function vectorToBytesFallback(v: number[]): Uint8Array {
  const buffer = new ArrayBuffer(v.length * 4)
  const view = new Float32Array(buffer)
  for (let i = 0; i < v.length; i++) {
    view[i] = v[i]!
  }
  return new Uint8Array(buffer)
}

/** Deserialize bytes to vector (fallback implementation) */
export function bytesToVectorFallback(bytes: Uint8Array): number[] {
  const view = new Float32Array(bytes.buffer, bytes.byteOffset, Math.floor(bytes.length / 4))
  return Array.from(view)
}

/** Hybrid merge: combine vector and keyword results (fallback implementation) */
export function hybridMergeResultsFallback(
  vectorResults: VectorResult[],
  keywordResults: VectorResult[],
  vectorWeight: number,
  keywordWeight: number,
  limit: number,
): ScoredResult[] {
  const map = new Map<string, ScoredResult>()

  // Add vector results
  for (const r of vectorResults) {
    map.set(r.id, {
      id: r.id,
      vectorScore: r.score,
      keywordScore: undefined,
      finalScore: 0,
    })
  }

  // Normalize keyword scores
  const maxKw = Math.max(...keywordResults.map((r) => r.score), 0.001)

  // Add keyword results
  for (const r of keywordResults) {
    const normalized = r.score / maxKw
    const existing = map.get(r.id)
    if (existing) {
      existing.keywordScore = normalized
    } else {
      map.set(r.id, {
        id: r.id,
        vectorScore: undefined,
        keywordScore: normalized,
        finalScore: 0,
      })
    }
  }

  // Compute final scores
  const results: ScoredResult[] = []
  for (const r of map.values()) {
    r.finalScore = vectorWeight * (r.vectorScore ?? 0) + keywordWeight * (r.keywordScore ?? 0)
    results.push(r)
  }

  // Sort and limit
  results.sort((a, b) => b.finalScore - a.finalScore)
  return results.slice(0, limit)
}

/** No-op embedding provider (fallback) */
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

// ============================================================================
// Exports (Native with Fallback)
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
  // Native bindings not available
}

/** Chunk markdown text */
export const chunkText = nativeBindings?.chunkText ?? chunkTextFallback

/** Chunk markdown text with custom configuration */
export const chunkTextWithConfig =
  nativeBindings?.chunkTextWithConfig ?? ((text: string, config: ChunkerConfig) => chunkTextFallback(text, config.maxTokens))

/** Estimate token count for text */
export const estimateChunkTokensNative = nativeBindings?.estimateTokens ?? estimateChunkTokens

/** Calculate cosine similarity between two vectors */
export const cosineSimilarity = nativeBindings?.cosineSimilarity ?? cosineSimilarityFallback

/** Calculate Euclidean distance between two vectors */
export const vectorDistance = nativeBindings?.vectorDistance ?? vectorDistanceFallback

/** Normalize a vector to unit length */
export const normalizeVector = nativeBindings?.normalizeVector ?? normalizeVectorFallback

/** Serialize vector to bytes */
export const vectorToBytes = nativeBindings?.vectorToBytes ?? vectorToBytesFallback

/** Deserialize bytes to vector */
export const bytesToVector = nativeBindings?.bytesToVector ?? bytesToVectorFallback

/** Hybrid merge: combine vector and keyword results */
export const hybridMergeResults = nativeBindings?.hybridMergeResults ?? hybridMergeResultsFallback

/** Create an embedding provider */
export function createEmbeddingProvider(_config: EmbeddingConfig): EmbeddingProvider {
  // For now, always return noop - actual implementations would need async setup
  return new NoopEmbeddingProvider()
}
