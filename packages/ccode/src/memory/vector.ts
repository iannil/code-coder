import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { Storage } from "@/infrastructure/storage/storage"
import {
  cosineSimilarity as nativeCosineSimilarity,
  normalizeVector,
  EmbeddingIndexHandle,
  createEmbeddingIndex,
  type EmbeddingIndexHandleType,
} from "@codecoder-ai/core"
import z from "zod"

const log = Log.create({ service: "memory.vector" })

export namespace Vector {
  export const Embedding = z.object({
    id: z.string(),
    text: z.string(),
    vector: z.array(z.number()),
    metadata: z.object({
      file: z.string(),
      type: z.enum(["function", "class", "interface", "comment", "code"]),
      line: z.number().optional(),
      language: z.string().optional(),
      created: z.number(),
    }),
  })
  export type Embedding = z.infer<typeof Embedding>

  export const SearchResult = z.object({
    embedding: Embedding,
    score: z.number(),
  })
  export type SearchResult = z.infer<typeof SearchResult>

  export const IndexStats = z.object({
    totalEmbeddings: z.number(),
    dimension: z.number(),
    lastUpdated: z.number(),
  })
  export type IndexStats = z.infer<typeof IndexStats>

  const DEFAULT_DIMENSION = 1536

  let embeddingCache: Map<string, number[]> | undefined

  // Native embedding index for SIMD-accelerated search
  let nativeIndex: EmbeddingIndexHandleType | undefined
  let indexInitialized = false

  /**
   * Initialize the native embedding index for SIMD-accelerated search.
   * This is required for search functionality.
   */
  export async function initializeNativeIndex(): Promise<void> {
    if (indexInitialized) {
      return
    }

    if (!createEmbeddingIndex) {
      throw new Error("Native binding required: @codecoder-ai/core createEmbeddingIndex not available")
    }

    const stats = await getStats()
    nativeIndex = createEmbeddingIndex(stats.dimension || DEFAULT_DIMENSION)

    // Load existing embeddings into the native index
    const projectID = Instance.project.id
    const keys = await Storage.list(["memory", "vector", "embeddings", projectID])

    for (const key of keys) {
      const embedding = await Storage.read<Embedding>(key)
      nativeIndex.add(embedding.id, embedding.vector)
    }

    indexInitialized = true
    log.info("Native embedding index initialized", { count: keys.length })
  }

  /**
   * Check if native index is available and initialized.
   */
  export function isNativeIndexAvailable(): boolean {
    return indexInitialized && nativeIndex !== undefined
  }

  /**
   * Get native index stats if available.
   */
  export function getNativeIndexStats(): { count: number; dimension: number; memoryBytes: number } | undefined {
    return nativeIndex?.stats()
  }

  export async function getStats(): Promise<IndexStats> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<IndexStats>(["memory", "vector", "stats", projectID])
    } catch {
      return {
        totalEmbeddings: 0,
        dimension: DEFAULT_DIMENSION,
        lastUpdated: Date.now(),
      }
    }
  }

  export async function updateStats(updates: Partial<IndexStats>): Promise<void> {
    const stats = await getStats()
    const updated = { ...stats, ...updates, lastUpdated: Date.now() }
    const projectID = Instance.project.id
    await Storage.write(["memory", "vector", "stats", projectID], updated)
  }

  export async function store(text: string, metadata: Embedding["metadata"], vector?: number[]): Promise<Embedding> {
    const projectID = Instance.project.id
    const now = Date.now()

    const embeddingVector = vector || (await generateEmbedding(text))

    const embedding: Embedding = {
      id: `emb_${now}_${Math.random().toString(36).slice(2, 9)}`,
      text,
      vector: embeddingVector,
      metadata: {
        ...metadata,
        created: now,
      },
    }

    await Storage.write(["memory", "vector", "embeddings", projectID, embedding.id], embedding)

    // Add to native index if initialized
    if (nativeIndex) {
      nativeIndex.add(embedding.id, embeddingVector)
    }

    const stats = await getStats()
    await updateStats({
      totalEmbeddings: stats.totalEmbeddings + 1,
      dimension: embeddingVector.length,
    })

    return embedding
  }

  export async function storeBatch(
    items: Array<{ text: string; metadata: Embedding["metadata"] }>,
  ): Promise<Embedding[]> {
    const embeddings: Embedding[] = []

    for (const item of items) {
      const embedding = await store(item.text, item.metadata)
      embeddings.push(embedding)
    }

    return embeddings
  }

  export async function get(id: string): Promise<Embedding | undefined> {
    const projectID = Instance.project.id
    try {
      return await Storage.read<Embedding>(["memory", "vector", "embeddings", projectID, id])
    } catch {
      return undefined
    }
  }

  export async function search(
    query: string,
    options?: {
      limit?: number
      threshold?: number
      fileType?: string
      type?: Embedding["metadata"]["type"]
    },
  ): Promise<SearchResult[]> {
    if (!nativeIndex) {
      throw new Error("Native binding required: call initializeNativeIndex() before search")
    }

    const projectID = Instance.project.id
    const queryVector = await generateEmbedding(query)
    const limit = options?.limit || 10
    const threshold = options?.threshold || 0

    // Fast path: use SIMD-accelerated native search
    const nativeResults = nativeIndex.search(queryVector, limit * 2, threshold) // fetch extra for filtering

    // Load embeddings from Storage to apply metadata filters and return full data
    const results: SearchResult[] = []
    for (const { id, score } of nativeResults) {
      if (results.length >= limit) break

      const embedding = await Storage.read<Embedding>(["memory", "vector", "embeddings", projectID, id])
      if (!embedding) continue

      // Apply metadata filters if needed
      if (options?.fileType && embedding.metadata.file !== options.fileType) continue
      if (options?.type && embedding.metadata.type !== options.type) continue

      results.push({ embedding, score })
    }

    return results
  }

  export async function searchByFile(filePath: string, limit = 10): Promise<Embedding[]> {
    const projectID = Instance.project.id

    try {
      const keys = await Storage.list(["memory", "vector", "embeddings", projectID])
      const results: Embedding[] = []

      for (const key of keys) {
        const embedding = await Storage.read<Embedding>(key)
        if (embedding.metadata.file === filePath) {
          results.push(embedding)
        }
      }

      return results.slice(0, limit)
    } catch {
      return []
    }
  }

  async function generateEmbedding(text: string): Promise<number[]> {
    if (embeddingCache?.has(text)) {
      return embeddingCache.get(text)!
    }

    const vector = await simpleHashEmbedding(text)

    if (!embeddingCache) {
      embeddingCache = new Map()
    }
    embeddingCache.set(text, vector)

    return vector
  }

  function simpleHashEmbedding(text: string): number[] {
    const dimension = DEFAULT_DIMENSION
    const vector = new Float32Array(dimension)

    let hash = 0
    for (let i = 0; i < text.length; i++) {
      const char = text.charCodeAt(i)
      hash = (hash << 5) - hash + char
      hash = hash & hash
    }

    const seed = Math.abs(hash)

    for (let i = 0; i < dimension; i++) {
      const x = sin(seed + i) * 10000
      vector[i] = x - Math.floor(x)
    }

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

    // Use native SIMD-accelerated normalization
    if (!normalizeVector) {
      throw new Error("Native bindings required: @codecoder-ai/core normalizeVector not available")
    }
    return normalizeVector(Array.from(vector))
  }

  function sin(x: number): number {
    return Math.sin(x)
  }

  /**
   * Cosine similarity using native SIMD-accelerated Rust implementation.
   * Requires native bindings from @codecoder-ai/core.
   */
  function cosineSimilarity(a: number[], b: number[]): number {
    if (!nativeCosineSimilarity) {
      throw new Error("Native bindings required: @codecoder-ai/core cosineSimilarity not available")
    }
    return nativeCosineSimilarity(a, b)
  }

  export async function findSimilar(
    text: string,
    options?: {
      limit?: number
      threshold?: number
      excludeFile?: string
    },
  ): Promise<Array<{ text: string; file: string; score: number }>> {
    const results = await search(text, {
      limit: options?.limit || 5,
      threshold: options?.threshold || 0.5,
    })

    return results
      .filter((r) => !options?.excludeFile || r.embedding.metadata.file !== options.excludeFile)
      .map((r) => ({
        text: r.embedding.text,
        file: r.embedding.metadata.file,
        score: r.score,
      }))
  }

  export async function remove(id: string): Promise<boolean> {
    const projectID = Instance.project.id
    try {
      await Storage.remove(["memory", "vector", "embeddings", projectID, id])

      // Remove from native index if initialized
      if (nativeIndex) {
        nativeIndex.remove(id)
      }

      const stats = await getStats()
      await updateStats({
        totalEmbeddings: Math.max(0, stats.totalEmbeddings - 1),
      })

      return true
    } catch {
      return false
    }
  }

  export async function removeByFile(filePath: string): Promise<number> {
    const projectID = Instance.project.id
    let removed = 0

    try {
      const keys = await Storage.list(["memory", "vector", "embeddings", projectID])
      for (const key of keys) {
        const embedding = await Storage.read<Embedding>(key)
        if (embedding.metadata.file === filePath) {
          await Storage.remove(key)
          removed++
        }
      }

      if (removed > 0) {
        const stats = await getStats()
        await updateStats({
          totalEmbeddings: Math.max(0, stats.totalEmbeddings - removed),
        })
      }
    } catch {}

    return removed
  }

  export async function clear(): Promise<void> {
    const projectID = Instance.project.id

    try {
      const keys = await Storage.list(["memory", "vector", "embeddings", projectID])
      for (const key of keys) {
        await Storage.remove(key)
      }

      // Clear native index if initialized
      if (nativeIndex) {
        nativeIndex.clear()
      }

      await updateStats({ totalEmbeddings: 0 })
    } catch {}
  }

  export async function cleanup(beforeDate: number): Promise<number> {
    const projectID = Instance.project.id
    let removed = 0

    try {
      const keys = await Storage.list(["memory", "vector", "embeddings", projectID])
      for (const key of keys) {
        const embedding = await Storage.read<Embedding>(key)
        if (embedding.metadata.created < beforeDate) {
          await Storage.remove(key)
          removed++
        }
      }

      if (removed > 0) {
        const stats = await getStats()
        await updateStats({
          totalEmbeddings: Math.max(0, stats.totalEmbeddings - removed),
        })
      }
    } catch {}

    return removed
  }

  export async function getContextForQuery(
    query: string,
    maxTokens = 4000,
  ): Promise<Array<{ file: string; content: string; score: number }>> {
    const results = await search(query, { limit: 20, threshold: 0.3 })

    const context: Array<{ file: string; content: string; score: number }> = []
    let totalChars = 0
    const charsPerToken = 4

    for (const result of results) {
      const content = result.embedding.text
      const chars = content.length

      if (totalChars + chars > maxTokens * charsPerToken) {
        break
      }

      context.push({
        file: result.embedding.metadata.file,
        content,
        score: result.score,
      })

      totalChars += chars
    }

    return context
  }

  export async function invalidate(): Promise<void> {
    const projectID = Instance.project.id
    await Storage.remove(["memory", "vector", "stats", projectID])
    await clear()

    // Reset native index
    if (nativeIndex) {
      nativeIndex.clear()
    }
    indexInitialized = false
  }
}
