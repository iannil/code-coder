/**
 * Markdown Document Chunker
 *
 * Splits Markdown documents into semantic chunks suitable for embedding and retrieval.
 * Uses structural elements (headings, code blocks, lists) to create meaningful chunks.
 *
 * Part of Phase 2: Global Context Hub
 */

import { Log } from "@/util/log"

const log = Log.create({ service: "memory.chunker" })

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
    const lines = content.split("\n")
    const chunks: Chunk[] = []
    const headingStack: string[] = []

    let currentChunk: string[] = []
    let currentType: ChunkMetadata["type"] = "paragraph"
    let currentStartLine = 0
    let inCodeBlock = false
    let codeLanguage: string | undefined

    const flushChunk = (endLine: number) => {
      if (currentChunk.length === 0) return

      const text = currentChunk.join("\n").trim()
      if (!text) return

      const tokenCount = this.estimateTokens(text)

      // Skip chunks that are too small (unless they're code blocks)
      if (tokenCount < this.config.minTokens && currentType !== "code") {
        return
      }

      // Build chunk with heading context
      let finalContent = text
      if (this.config.includeHeadingContext && headingStack.length > 0 && currentType !== "code") {
        const context = headingStack.join(" > ")
        finalContent = `[${context}]\n\n${text}`
      }

      const chunk: Chunk = {
        id: this.generateChunkId(source, currentStartLine),
        content: finalContent,
        metadata: {
          source,
          headings: [...headingStack],
          type: currentType,
          startLine: currentStartLine,
          endLine,
          language: currentType === "code" ? codeLanguage : undefined,
        },
        tokenCount: this.estimateTokens(finalContent),
      }

      // Split large chunks
      if (chunk.tokenCount > this.config.maxTokens) {
        chunks.push(...this.splitLargeChunk(chunk))
      } else {
        chunks.push(chunk)
      }

      currentChunk = []
      currentType = "paragraph"
      currentStartLine = endLine + 1
    }

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i]

      // Handle code blocks
      if (line.startsWith("```")) {
        if (inCodeBlock) {
          // End of code block
          currentChunk.push(line)
          if (this.config.preserveCodeBlocks) {
            flushChunk(i)
          }
          inCodeBlock = false
          codeLanguage = undefined
        } else {
          // Start of code block
          flushChunk(i - 1)
          inCodeBlock = true
          currentType = "code"
          codeLanguage = line.slice(3).trim() || undefined
          currentChunk.push(line)
          currentStartLine = i
        }
        continue
      }

      if (inCodeBlock) {
        currentChunk.push(line)
        continue
      }

      // Handle headings
      const headingMatch = line.match(/^(#{1,6})\s+(.+)$/)
      if (headingMatch) {
        flushChunk(i - 1)

        const level = headingMatch[1].length
        const title = headingMatch[2]

        // Update heading stack
        while (headingStack.length >= level) {
          headingStack.pop()
        }
        headingStack.push(`${"#".repeat(level)} ${title}`)

        currentType = "heading"
        currentChunk.push(line)
        currentStartLine = i
        continue
      }

      // Handle lists
      if (line.match(/^[\s]*[-*+]\s+/) || line.match(/^[\s]*\d+\.\s+/)) {
        if (currentType !== "list") {
          flushChunk(i - 1)
          currentType = "list"
          currentStartLine = i
        }
        currentChunk.push(line)
        continue
      }

      // Handle blockquotes
      if (line.startsWith(">")) {
        if (currentType !== "blockquote") {
          flushChunk(i - 1)
          currentType = "blockquote"
          currentStartLine = i
        }
        currentChunk.push(line)
        continue
      }

      // Handle tables
      if (line.includes("|") && line.trim().startsWith("|")) {
        if (currentType !== "table") {
          flushChunk(i - 1)
          currentType = "table"
          currentStartLine = i
        }
        currentChunk.push(line)
        continue
      }

      // Handle empty lines
      if (line.trim() === "") {
        // End current paragraph on double newline
        if (currentChunk.length > 0 && currentChunk[currentChunk.length - 1].trim() === "") {
          flushChunk(i - 1)
        } else {
          currentChunk.push(line)
        }
        continue
      }

      // Regular paragraph
      if (currentType !== "paragraph" && currentType !== "heading") {
        flushChunk(i - 1)
        currentType = "paragraph"
        currentStartLine = i
      }
      currentChunk.push(line)
    }

    // Flush remaining content
    flushChunk(lines.length - 1)

    log.debug("Chunked document", {
      source,
      totalChunks: chunks.length,
      avgTokens: Math.round(chunks.reduce((sum, c) => sum + c.tokenCount, 0) / chunks.length),
    })

    return chunks
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
    // Rough estimate: ~4 characters per token for English
    return Math.ceil(text.length / 4)
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private generateChunkId(source: string, startLine: number): string {
    const hash = this.simpleHash(`${source}:${startLine}`)
    return `chunk_${hash}`
  }

  private simpleHash(str: string): string {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash = hash & hash
    }
    return Math.abs(hash).toString(36).slice(0, 8)
  }

  private splitLargeChunk(chunk: Chunk): Chunk[] {
    const { content, metadata } = chunk
    const maxTokens = this.config.maxTokens
    const overlapTokens = this.config.overlapTokens

    const sentences = this.splitIntoSentences(content)
    const result: Chunk[] = []
    let currentSentences: string[] = []
    let currentTokens = 0
    let partIndex = 0

    for (const sentence of sentences) {
      const sentenceTokens = this.estimateTokens(sentence)

      if (currentTokens + sentenceTokens > maxTokens && currentSentences.length > 0) {
        // Create chunk from current sentences
        const chunkContent = currentSentences.join(" ")
        result.push({
          id: `${chunk.id}_p${partIndex}`,
          content: chunkContent,
          metadata: {
            ...metadata,
            parentId: chunk.id,
          },
          tokenCount: this.estimateTokens(chunkContent),
        })
        partIndex++

        // Keep overlap sentences
        const overlapSentences: string[] = []
        let overlapCount = 0
        for (let i = currentSentences.length - 1; i >= 0 && overlapCount < overlapTokens; i--) {
          overlapSentences.unshift(currentSentences[i])
          overlapCount += this.estimateTokens(currentSentences[i])
        }

        currentSentences = overlapSentences
        currentTokens = overlapCount
      }

      currentSentences.push(sentence)
      currentTokens += sentenceTokens
    }

    // Add remaining content
    if (currentSentences.length > 0) {
      const chunkContent = currentSentences.join(" ")
      result.push({
        id: `${chunk.id}_p${partIndex}`,
        content: chunkContent,
        metadata: {
          ...metadata,
          parentId: chunk.id,
        },
        tokenCount: this.estimateTokens(chunkContent),
      })
    }

    return result
  }

  private splitIntoSentences(text: string): string[] {
    // Simple sentence splitting
    return text
      .split(/(?<=[.!?])\s+/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0)
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
