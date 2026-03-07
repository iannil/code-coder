/**
 * Markdown Document Chunker
 *
 * Thin wrapper around @codecoder-ai/core native Rust implementation.
 * Splits Markdown documents into semantic chunks suitable for embedding and retrieval.
 * Uses structural elements (headings, code blocks, lists) to create meaningful chunks.
 *
 * Part of Phase 2: Global Context Hub
 */

import {
  chunkText as coreChunkText,
  estimateTokens as coreEstimateTokens,
  type NapiChunk,
} from "@codecoder-ai/core"

// ============================================================================
// Types
// ============================================================================

export interface ChunkMetadata {
  /** Source file path */
  source: string
  /** Section heading hierarchy (e.g., ["# Main", "## Sub"]) */
  headings: string[]
  /** Chunk type */
  type: "heading" | "paragraph" | "code" | "list" | "table" | "blockquote"
  /** Start line in original document */
  startLine: number
  /** End line in original document */
  endLine: number
  /** Language for code blocks */
  language?: string
  /** Parent chunk ID for hierarchical retrieval */
  parentId?: string
}

export interface Chunk {
  /** Unique chunk ID */
  id: string
  /** Chunk content */
  content: string
  /** Chunk metadata */
  metadata: ChunkMetadata
  /** Estimated token count */
  tokenCount: number
}

export interface ChunkerConfig {
  /** Maximum tokens per chunk (default: 512) */
  maxTokens: number
  /** Minimum tokens per chunk (default: 50) */
  minTokens: number
  /** Overlap tokens between chunks (default: 50) */
  overlapTokens: number
  /** Include heading context in chunks (default: true) */
  includeHeadingContext: boolean
  /** Preserve code blocks as single chunks (default: true) */
  preserveCodeBlocks: boolean
}

const DEFAULT_CONFIG: ChunkerConfig = {
  maxTokens: 512,
  minTokens: 50,
  overlapTokens: 50,
  includeHeadingContext: true,
  preserveCodeBlocks: true,
}

// ============================================================================
// Conversion Utilities
// ============================================================================

function convertChunkString(chunk: NapiChunk, source: string, index: number): Chunk {
  // Determine chunk type from content
  const type = detectChunkType(chunk.content)

  return {
    id: generateChunkId(source, index),
    content: chunk.content,
    metadata: {
      source,
      headings: chunk.heading ? [chunk.heading] : [], // Use heading from NapiChunk if available
      type,
      startLine: 1, // Not tracked in simple chunking
      endLine: 1,
      language: type === "code" ? extractCodeLanguage(chunk.content) : undefined,
    },
    tokenCount: estimateTokens(chunk.content),
  }
}

function detectChunkType(content: string): ChunkMetadata["type"] {
  const trimmed = content.trim()
  if (trimmed.startsWith("```")) return "code"
  if (trimmed.startsWith("#")) return "heading"
  if (trimmed.startsWith(">")) return "blockquote"
  if (trimmed.startsWith("|") || trimmed.includes("\n|")) return "table"
  if (/^[\s]*[-*+]\s+/.test(trimmed) || /^[\s]*\d+\.\s+/.test(trimmed)) return "list"
  return "paragraph"
}

function extractCodeLanguage(content: string): string | undefined {
  const match = content.match(/^```(\w+)?/)
  return match?.[1] || undefined
}

function generateChunkId(source: string, index: number): string {
  const hash = simpleHash(`${source}:${index}`)
  return `chunk_${hash}`
}

function simpleHash(str: string): string {
  let hash = 0
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i)
    hash = (hash << 5) - hash + char
    hash = hash & hash
  }
  return Math.abs(hash).toString(36).slice(0, 8)
}

// ============================================================================
// Chunker
// ============================================================================

export class MarkdownChunker {
  private config: ChunkerConfig

  constructor(config?: Partial<ChunkerConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Chunk a Markdown document into semantic pieces
   */
  chunk(content: string, source: string): Chunk[] {
    // Use native chunker - returns string[]
    if (!coreChunkText) throw new Error("Native bindings not available")
    const chunkStrings = coreChunkText(content)

    // Convert to our format with metadata
    const chunks = chunkStrings.map((c, i) => convertChunkString(c, source, i))

    // Filter by minimum tokens if configured
    const filtered = chunks.filter((c) => c.tokenCount >= this.config.minTokens || c.metadata.type === "code")

    return filtered
  }

  /**
   * Chunk multiple documents
   */
  chunkMany(documents: Array<{ content: string; source: string }>): Chunk[] {
    const allChunks: Chunk[] = []
    for (const doc of documents) {
      allChunks.push(...this.chunk(doc.content, doc.source))
    }
    return allChunks
  }

  /**
   * Estimate token count for text (rough approximation)
   */
  estimateTokens(text: string): number {
    return coreEstimateTokens?.(text) ?? Math.ceil(text.length / 4)
  }
}

// ============================================================================
// Factory
// ============================================================================

let defaultChunker: MarkdownChunker | null = null

/**
 * Get the default chunker instance
 */
export function getChunker(): MarkdownChunker {
  if (!defaultChunker) {
    defaultChunker = new MarkdownChunker()
  }
  return defaultChunker
}

/**
 * Create a chunker with custom config
 */
export function createChunker(config?: Partial<ChunkerConfig>): MarkdownChunker {
  return new MarkdownChunker(config)
}

/**
 * Convenience function to chunk a single document
 */
export function chunkMarkdown(content: string, source: string, config?: Partial<ChunkerConfig>): Chunk[] {
  const chunker = config ? new MarkdownChunker(config) : getChunker()
  return chunker.chunk(content, source)
}

/**
 * Estimate token count for text
 */
export function estimateTokens(text: string): number {
  return coreEstimateTokens?.(text) ?? Math.ceil(text.length / 4)
}
