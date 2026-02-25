/**
 * Global Context Hub
 *
 * Unified API for cross-agent context retrieval. Aggregates all memory sources:
 * - Vector embeddings for semantic search
 * - Knowledge sedimentation from autonomous evolution
 * - Markdown memory (daily notes + MEMORY.md)
 * - Code index and patterns
 * - Tool registry
 *
 * Implements the "全局上下文枢纽" (Global Context Hub) from goals.md.
 *
 * Part of Phase 2: Global Context Hub
 */

import { Log } from "@/util/log"
import { Vector } from "./vector"
import { Knowledge } from "./knowledge/index"
import { Patterns } from "./knowledge/patterns"
import { DynamicToolRegistry } from "./tools/index"
import { EmbeddingProvider, getEmbeddingProvider, type EmbeddingResult } from "./embedding-provider"
import { MarkdownChunker, getChunker, type Chunk } from "./chunker"
import { loadMarkdownMemoryContext } from "@/memory-markdown"
import { getKnowledgeSedimentation } from "@/autonomous/execution/knowledge-sedimentation"
import { Storage } from "@/storage/storage"
import { Instance } from "@/project/instance"

const log = Log.create({ service: "memory.context-hub" })

// ============================================================================
// Types
// ============================================================================

/** Source of context */
export type ContextSource =
  | "vector"
  | "knowledge"
  | "markdown"
  | "tool"
  | "sedimentation"
  | "pattern"

/** Single context item */
export interface ContextItem {
  /** Unique item ID */
  id: string
  /** Content of the context */
  content: string
  /** Source of the context */
  source: ContextSource
  /** Relevance score (0-1) */
  score: number
  /** Additional metadata */
  metadata: {
    file?: string
    heading?: string
    type?: string
    timestamp?: number
    [key: string]: unknown
  }
}

/** Context retrieval options */
export interface RetrievalOptions {
  /** Maximum number of items to retrieve */
  limit?: number
  /** Minimum relevance score (0-1) */
  threshold?: number
  /** Sources to include (default: all) */
  sources?: ContextSource[]
  /** Maximum tokens for combined context */
  maxTokens?: number
  /** Include heading context in markdown chunks */
  includeHeadingContext?: boolean
  /** Boost recent items */
  recencyBoost?: boolean
}

/** Aggregated context result */
export interface ContextResult {
  /** Retrieved context items, sorted by relevance */
  items: ContextItem[]
  /** Total estimated tokens */
  totalTokens: number
  /** Sources that contributed to the result */
  contributingSources: ContextSource[]
  /** Query embedding (for debugging) */
  queryEmbedding?: number[]
}

/** Index entry for vector store */
interface IndexEntry {
  id: string
  source: ContextSource
  content: string
  vector: number[]
  metadata: Record<string, unknown>
  timestamp: number
}

// ============================================================================
// Global Context Hub
// ============================================================================

export class GlobalContextHub {
  private embeddingProvider: EmbeddingProvider | null = null
  private chunker: MarkdownChunker
  private indexCache: Map<string, IndexEntry> = new Map()
  private initialized = false

  constructor() {
    this.chunker = getChunker()
  }

  /**
   * Initialize the context hub
   */
  async initialize(): Promise<void> {
    if (this.initialized) return

    log.info("Initializing Global Context Hub")

    // Get embedding provider
    this.embeddingProvider = await getEmbeddingProvider()

    // Load existing index
    await this.loadIndex()

    this.initialized = true
    log.info("Global Context Hub initialized", {
      provider: this.embeddingProvider.getProviderType(),
      indexSize: this.indexCache.size,
    })
  }

  /**
   * Retrieve relevant context for a query
   *
   * This is the main API for cross-agent context retrieval.
   */
  async retrieve(query: string, options?: RetrievalOptions): Promise<ContextResult> {
    await this.initialize()

    const opts: Required<RetrievalOptions> = {
      limit: options?.limit ?? 10,
      threshold: options?.threshold ?? 0.3,
      sources: options?.sources ?? ["vector", "knowledge", "markdown", "tool", "sedimentation", "pattern"],
      maxTokens: options?.maxTokens ?? 4000,
      includeHeadingContext: options?.includeHeadingContext ?? true,
      recencyBoost: options?.recencyBoost ?? true,
    }

    log.debug("Retrieving context", { query: query.slice(0, 50), options: opts })

    // Generate query embedding
    const queryEmbedding = await this.embeddingProvider!.embed(query)

    // Collect items from all sources
    const items: ContextItem[] = []

    // 1. Vector search
    if (opts.sources.includes("vector")) {
      const vectorItems = await this.searchVector(queryEmbedding.vector, opts)
      items.push(...vectorItems)
    }

    // 2. Knowledge base
    if (opts.sources.includes("knowledge")) {
      const knowledgeItems = await this.searchKnowledge(query, opts)
      items.push(...knowledgeItems)
    }

    // 3. Markdown memory
    if (opts.sources.includes("markdown")) {
      const markdownItems = await this.searchMarkdown(queryEmbedding.vector, opts)
      items.push(...markdownItems)
    }

    // 4. Tool registry
    if (opts.sources.includes("tool")) {
      const toolItems = await this.searchTools(query, opts)
      items.push(...toolItems)
    }

    // 5. Knowledge sedimentation (from autonomous)
    if (opts.sources.includes("sedimentation")) {
      const sedimentItems = await this.searchSedimentation(query, opts)
      items.push(...sedimentItems)
    }

    // 6. Patterns
    if (opts.sources.includes("pattern")) {
      const patternItems = await this.searchPatterns(query, opts)
      items.push(...patternItems)
    }

    // Sort by score with optional recency boost
    const sortedItems = this.sortItems(items, opts.recencyBoost)

    // Deduplicate
    const dedupedItems = this.deduplicateItems(sortedItems)

    // Limit by tokens
    const limitedItems = this.limitByTokens(dedupedItems, opts.maxTokens)

    // Get contributing sources
    const contributingSources = [...new Set(limitedItems.map((i) => i.source))]

    log.debug("Context retrieved", {
      totalItems: limitedItems.length,
      sources: contributingSources,
      totalTokens: this.estimateTokens(limitedItems),
    })

    return {
      items: limitedItems.slice(0, opts.limit),
      totalTokens: this.estimateTokens(limitedItems),
      contributingSources,
      queryEmbedding: queryEmbedding.vector,
    }
  }

  /**
   * Index a document for future retrieval
   */
  async indexDocument(content: string, source: string, metadata?: Record<string, unknown>): Promise<number> {
    await this.initialize()

    // Chunk the document
    const chunks = this.chunker.chunk(content, source)

    // Generate embeddings and store
    let indexed = 0
    for (const chunk of chunks) {
      const embedding = await this.embeddingProvider!.embed(chunk.content)

      const entry: IndexEntry = {
        id: chunk.id,
        source: "vector",
        content: chunk.content,
        vector: embedding.vector,
        metadata: {
          ...chunk.metadata,
          ...metadata,
        },
        timestamp: Date.now(),
      }

      this.indexCache.set(entry.id, entry)
      indexed++
    }

    // Persist index
    await this.saveIndex()

    log.info("Document indexed", { source, chunks: indexed })
    return indexed
  }

  /**
   * Index markdown memory files
   */
  async indexMarkdownMemory(): Promise<number> {
    await this.initialize()

    let indexed = 0

    try {
      const memoryContext = await loadMarkdownMemoryContext({ includeDays: 7 })

      // Index long-term memory
      if (memoryContext.longTerm) {
        indexed += await this.indexDocument(memoryContext.longTerm, "memory/MEMORY.md", {
          type: "long_term_memory",
        })
      }

      // Index daily notes
      for (const daily of memoryContext.daily) {
        indexed += await this.indexDocument(daily, "memory/daily", {
          type: "daily_note",
        })
      }

      log.info("Markdown memory indexed", { indexed })
    } catch (error) {
      log.warn("Failed to index markdown memory", {
        error: error instanceof Error ? error.message : String(error),
      })
    }

    return indexed
  }

  /**
   * Clear the index
   */
  async clearIndex(): Promise<void> {
    this.indexCache.clear()
    await this.saveIndex()
    log.info("Index cleared")
  }

  /**
   * Get index statistics
   */
  getStats(): { totalEntries: number; bySource: Record<ContextSource, number> } {
    const bySource: Record<ContextSource, number> = {
      vector: 0,
      knowledge: 0,
      markdown: 0,
      tool: 0,
      sedimentation: 0,
      pattern: 0,
    }

    for (const entry of this.indexCache.values()) {
      bySource[entry.source]++
    }

    return {
      totalEntries: this.indexCache.size,
      bySource,
    }
  }

  // ============================================================================
  // Private Methods - Search
  // ============================================================================

  private async searchVector(queryVector: number[], opts: Required<RetrievalOptions>): Promise<ContextItem[]> {
    const items: ContextItem[] = []

    for (const entry of this.indexCache.values()) {
      if (entry.source !== "vector") continue

      const score = this.embeddingProvider!.cosineSimilarity(queryVector, entry.vector)
      if (score < opts.threshold) continue

      items.push({
        id: entry.id,
        content: entry.content,
        source: "vector",
        score,
        metadata: entry.metadata as ContextItem["metadata"],
      })
    }

    return items.sort((a, b) => b.score - a.score).slice(0, opts.limit * 2)
  }

  private async searchKnowledge(query: string, opts: Required<RetrievalOptions>): Promise<ContextItem[]> {
    const items: ContextItem[] = []

    try {
      const knowledge = await Knowledge.get()
      if (!knowledge) return items

      // Search API endpoints
      for (const endpoint of knowledge.apiEndpoints || []) {
        const score = this.textSimilarity(query, `${endpoint.method} ${endpoint.path} ${endpoint.description || ""}`)
        if (score > opts.threshold) {
          items.push({
            id: `api_${endpoint.path}`,
            content: `API: ${endpoint.method} ${endpoint.path}\n${endpoint.description || ""}`,
            source: "knowledge",
            score,
            metadata: { type: "api_endpoint", file: endpoint.file },
          })
        }
      }

      // Search data models
      for (const model of knowledge.dataModels || []) {
        const searchText = `${model.name} ${model.type} ${model.properties?.map((p) => p.name).join(" ") || ""}`
        const score = this.textSimilarity(query, searchText)
        if (score > opts.threshold) {
          items.push({
            id: `model_${model.name}`,
            content: `Model: ${model.name} (${model.type})\nProperties: ${model.properties?.map((p) => `${p.name}: ${p.type}`).join(", ") || "none"}`,
            source: "knowledge",
            score,
            metadata: { type: "data_model", file: model.file },
          })
        }
      }
    } catch {
      // Knowledge not available
    }

    return items.slice(0, opts.limit)
  }

  private async searchMarkdown(queryVector: number[], opts: Required<RetrievalOptions>): Promise<ContextItem[]> {
    const items: ContextItem[] = []

    for (const entry of this.indexCache.values()) {
      if (entry.source !== "vector") continue
      if (!entry.metadata.type?.toString().includes("memory")) continue

      const score = this.embeddingProvider!.cosineSimilarity(queryVector, entry.vector)
      if (score < opts.threshold) continue

      items.push({
        id: entry.id,
        content: entry.content,
        source: "markdown",
        score,
        metadata: entry.metadata as ContextItem["metadata"],
      })
    }

    return items.sort((a, b) => b.score - a.score).slice(0, opts.limit)
  }

  private async searchTools(query: string, opts: Required<RetrievalOptions>): Promise<ContextItem[]> {
    const items: ContextItem[] = []

    try {
      const results = await DynamicToolRegistry.search(query, {
        limit: opts.limit,
        minScore: opts.threshold,
      })

      for (const result of results) {
        items.push({
          id: result.tool.id,
          content: `Tool: ${result.tool.name}\n${result.tool.description}\n\`\`\`${result.tool.language}\n${result.tool.code.slice(0, 500)}\n\`\`\``,
          source: "tool",
          score: result.score,
          metadata: {
            type: "tool",
            language: result.tool.language,
            tags: result.tool.tags,
          },
        })
      }
    } catch {
      // Tool registry not available
    }

    return items
  }

  private async searchSedimentation(query: string, opts: Required<RetrievalOptions>): Promise<ContextItem[]> {
    const items: ContextItem[] = []

    try {
      const sedimentation = await getKnowledgeSedimentation()
      const results = await sedimentation.search(query, opts.limit)

      for (const result of results) {
        items.push({
          id: result.entry.id,
          content: `${result.entry.title}\n${result.entry.content}\n${result.entry.codeExamples?.map((c) => c.code).join("\n") || ""}`,
          source: "sedimentation",
          score: result.relevanceScore,
          metadata: {
            type: "sedimented_knowledge",
            category: result.entry.category,
            tags: result.entry.tags,
          },
        })
      }
    } catch {
      // Sedimentation not available
    }

    return items
  }

  private async searchPatterns(query: string, opts: Required<RetrievalOptions>): Promise<ContextItem[]> {
    const items: ContextItem[] = []

    try {
      const patternStore = await Patterns.get()
      if (!patternStore || !patternStore.patterns) return items

      for (const pattern of patternStore.patterns.slice(0, opts.limit)) {
        const score = this.textSimilarity(query, `${pattern.name} ${pattern.description || ""} ${pattern.category}`)
        if (score > opts.threshold) {
          items.push({
            id: `pattern_${pattern.id}`,
            content: `Pattern: ${pattern.name}\nCategory: ${pattern.category}\n${pattern.description || ""}\nTemplate: ${pattern.template}`,
            source: "pattern",
            score,
            metadata: { type: "pattern", category: pattern.category, files: pattern.files },
          })
        }
      }
    } catch {
      // Patterns not available
    }

    return items
  }

  // ============================================================================
  // Private Methods - Utilities
  // ============================================================================

  private sortItems(items: ContextItem[], recencyBoost: boolean): ContextItem[] {
    return items.sort((a, b) => {
      let scoreA = a.score
      let scoreB = b.score

      if (recencyBoost) {
        const now = Date.now()
        const ageA = now - (a.metadata.timestamp as number || 0)
        const ageB = now - (b.metadata.timestamp as number || 0)
        const dayMs = 24 * 60 * 60 * 1000

        // Boost items from last 7 days
        if (ageA < 7 * dayMs) scoreA *= 1.2
        if (ageB < 7 * dayMs) scoreB *= 1.2
      }

      return scoreB - scoreA
    })
  }

  private deduplicateItems(items: ContextItem[]): ContextItem[] {
    const seen = new Set<string>()
    const result: ContextItem[] = []

    for (const item of items) {
      // Create content fingerprint
      const fingerprint = item.content.slice(0, 100).toLowerCase().replace(/\s+/g, " ")
      if (seen.has(fingerprint)) continue

      seen.add(fingerprint)
      result.push(item)
    }

    return result
  }

  private limitByTokens(items: ContextItem[], maxTokens: number): ContextItem[] {
    const result: ContextItem[] = []
    let totalTokens = 0

    for (const item of items) {
      const itemTokens = Math.ceil(item.content.length / 4)
      if (totalTokens + itemTokens > maxTokens) break

      result.push(item)
      totalTokens += itemTokens
    }

    return result
  }

  private estimateTokens(items: ContextItem[]): number {
    return items.reduce((sum, item) => sum + Math.ceil(item.content.length / 4), 0)
  }

  private textSimilarity(a: string, b: string): number {
    const wordsA = new Set(a.toLowerCase().split(/\s+/))
    const wordsB = new Set(b.toLowerCase().split(/\s+/))

    const intersection = new Set([...wordsA].filter((x) => wordsB.has(x)))
    const union = new Set([...wordsA, ...wordsB])

    return intersection.size / union.size
  }

  private async loadIndex(): Promise<void> {
    try {
      const projectID = Instance.project.id
      const entries = await Storage.read<IndexEntry[]>(["memory", "context-hub", "index", projectID])
      if (entries) {
        for (const entry of entries) {
          this.indexCache.set(entry.id, entry)
        }
      }
    } catch {
      // No existing index
    }
  }

  private async saveIndex(): Promise<void> {
    const projectID = Instance.project.id
    const entries = Array.from(this.indexCache.values())
    await Storage.write(["memory", "context-hub", "index", projectID], entries)
  }
}

// ============================================================================
// Factory
// ============================================================================

let defaultHub: GlobalContextHub | null = null

/**
 * Get the global context hub singleton
 */
export async function getContextHub(): Promise<GlobalContextHub> {
  if (!defaultHub) {
    defaultHub = new GlobalContextHub()
    await defaultHub.initialize()
  }
  return defaultHub
}

/**
 * Create a new context hub instance
 */
export function createContextHub(): GlobalContextHub {
  return new GlobalContextHub()
}

/**
 * Convenience function for quick context retrieval
 */
export async function retrieveContext(query: string, options?: RetrievalOptions): Promise<ContextResult> {
  const hub = await getContextHub()
  return hub.retrieve(query, options)
}
