/**
 * NAPI Bindings SDK
 *
 * Provides a clean TypeScript interface to the native Rust NAPI bindings.
 * This module wraps @codecoder-ai/core bindings for direct TUI usage.
 *
 * ## Design
 *
 * This SDK provides NAPI bindings for:
 * - **Memory**: Markdown dual-layer memory (daily notes + long-term)
 * - **Config**: Configuration loading and management
 * - **Tools**: File operations, text processing, etc.
 * - **Observability**: Tracing, metrics, and cost tracking
 *
 * For WebSocket/HTTP based operations, use the `./client` and `./websocket` modules instead.
 *
 * @module sdk/napi
 */

import {
  // Memory System
  MemorySystemHandle,
  createMemorySystem,
  MarkdownMemoryHandle,
  createMarkdownMemory,
  // Chunking and tokenization
  chunkText,
  chunkTextWithConfig,
  estimateTokens,
  // Vector operations
  cosineSimilarity,
  normalizeVector,
  vectorDistance,
  vectorToBytes,
  bytesToVector,
  hybridMergeResults,
  // File operations
  readFile,
  editFile,
  grep,
  glob,
  // Config
  ConfigLoaderHandle,
  createConfigLoader,
  // Safety
  assessBashRisk,
  assessFileRisk,
  // Git operations
  openGitRepo,
  isGitRepo,
  // Hash embeddings
  generateHashEmbedding,
  generateHashEmbeddingsBatch,
  hashEmbeddingSimilarity,
  // isNative check
  isNative,
} from "@codecoder-ai/core"

// ============================================================================
// Types
// ============================================================================

/** Daily entry type */
export type DailyEntryType = "Decision" | "Action" | "Output" | "Error" | "Solution"

/** Memory category */
export type MemoryCategory =
  | "UserPreferences"
  | "ProjectContext"
  | "KeyDecisions"
  | "LessonsLearned"
  | "SuccessfulSolutions"

/** Memory section */
export interface MemorySection {
  category: string
  content: string
  lastUpdated: string
}

/** Memory context */
export interface MemoryContext {
  longTerm: string
  daily: string[]
  combined: string
}

/** Markdown memory configuration */
export interface MarkdownMemoryConfig {
  basePath: string
  projectId: string
  dailyPath: string
  longTermPath: string
}

// ============================================================================
// NAPI Module
// ============================================================================

/**
 * NAPI bindings module for direct Rust function calls.
 *
 * Use this module when you need high-performance operations that bypass
 * the daemon HTTP/WebSocket layer.
 *
 * @example
 * ```typescript
 * import { NAPI } from "./sdk/napi"
 *
 * // Check if native bindings are available
 * if (!NAPI.isAvailable) {
 *   throw new Error("NAPI bindings required")
 * }
 *
 * // Use memory operations
 * const memory = NAPI.memory.createMarkdown("./memory", "my-project")
 * await memory.appendDailyNote("Action", "Started implementation")
 *
 * // Use token estimation
 * const tokens = NAPI.text.estimateTokens("Hello, world!")
 * ```
 */
export const NAPI = {
  /** Whether native bindings are available */
  isAvailable: isNative,

  // ==========================================================================
  // Memory Operations
  // ==========================================================================

  memory: {
    /**
     * Create a unified memory system
     */
    createSystem: (dataDir: string, projectId: string) => {
      if (!createMemorySystem) {
        throw new Error("NAPI binding required: createMemorySystem")
      }
      return createMemorySystem(dataDir, projectId)
    },

    /**
     * Create a markdown memory store (dual-layer)
     */
    createMarkdown: (basePath: string, projectId: string) => {
      if (!createMarkdownMemory) {
        throw new Error("NAPI binding required: createMarkdownMemory")
      }
      return createMarkdownMemory(basePath, projectId)
    },

    /**
     * Get MemorySystemHandle class (if available)
     */
    get MemorySystemHandle() {
      return MemorySystemHandle
    },

    /**
     * Get MarkdownMemoryHandle class (if available)
     */
    get MarkdownMemoryHandle() {
      return MarkdownMemoryHandle
    },
  },

  // ==========================================================================
  // Text Operations
  // ==========================================================================

  text: {
    /**
     * Chunk text into semantic chunks
     */
    chunk: (text: string, maxTokens?: number) => {
      if (!chunkText) {
        throw new Error("NAPI binding required: chunkText")
      }
      return chunkText(text, maxTokens)
    },

    /**
     * Chunk text with custom configuration
     */
    chunkWithConfig: (
      text: string,
      config: { maxTokens: number; overlapTokens: number; preserveHeadings: boolean }
    ) => {
      if (!chunkTextWithConfig) {
        throw new Error("NAPI binding required: chunkTextWithConfig")
      }
      return chunkTextWithConfig(text, config)
    },

    /**
     * Estimate token count for text
     */
    estimateTokens: (text: string) => {
      if (!estimateTokens) {
        throw new Error("NAPI binding required: estimateTokens")
      }
      return estimateTokens(text)
    },
  },

  // ==========================================================================
  // Vector Operations
  // ==========================================================================

  vector: {
    /**
     * Calculate cosine similarity between two vectors
     */
    cosineSimilarity: (a: number[], b: number[]) => {
      if (!cosineSimilarity) {
        throw new Error("NAPI binding required: cosineSimilarity")
      }
      return cosineSimilarity(a, b)
    },

    /**
     * Normalize a vector to unit length
     */
    normalize: (v: number[]) => {
      if (!normalizeVector) {
        throw new Error("NAPI binding required: normalizeVector")
      }
      return normalizeVector(v)
    },

    /**
     * Calculate Euclidean distance between two vectors
     */
    distance: (a: number[], b: number[]) => {
      if (!vectorDistance) {
        throw new Error("NAPI binding required: vectorDistance")
      }
      return vectorDistance(a, b)
    },

    /**
     * Serialize vector to bytes
     */
    toBytes: (v: number[]) => {
      if (!vectorToBytes) {
        throw new Error("NAPI binding required: vectorToBytes")
      }
      return vectorToBytes(v)
    },

    /**
     * Deserialize bytes to vector
     */
    fromBytes: (bytes: Buffer) => {
      if (!bytesToVector) {
        throw new Error("NAPI binding required: bytesToVector")
      }
      return bytesToVector(bytes)
    },

    /**
     * Hybrid merge of vector and keyword results
     */
    hybridMerge: (
      vectorResults: Array<{ id: string; score: number }>,
      keywordResults: Array<{ id: string; score: number }>,
      vectorWeight: number,
      keywordWeight: number,
      limit: number
    ) => {
      if (!hybridMergeResults) {
        throw new Error("NAPI binding required: hybridMergeResults")
      }
      return hybridMergeResults(vectorResults, keywordResults, vectorWeight, keywordWeight, limit)
    },
  },

  // ==========================================================================
  // Hash Embedding Operations
  // ==========================================================================

  embedding: {
    /**
     * Generate hash-based embedding for text
     */
    generateHash: (text: string, dimension?: number) => {
      if (!generateHashEmbedding) {
        throw new Error("NAPI binding required: generateHashEmbedding")
      }
      return generateHashEmbedding(text, dimension)
    },

    /**
     * Generate hash embeddings for multiple texts
     */
    generateHashBatch: (texts: string[], dimension?: number) => {
      if (!generateHashEmbeddingsBatch) {
        throw new Error("NAPI binding required: generateHashEmbeddingsBatch")
      }
      return generateHashEmbeddingsBatch(texts, dimension)
    },

    /**
     * Calculate similarity between two hash embeddings
     */
    similarity: (a: number[], b: number[]) => {
      if (!hashEmbeddingSimilarity) {
        throw new Error("NAPI binding required: hashEmbeddingSimilarity")
      }
      return hashEmbeddingSimilarity(a, b)
    },
  },

  // ==========================================================================
  // File Operations
  // ==========================================================================

  file: {
    /**
     * Read a file
     */
    read: (filePath: string, options?: { offset?: number; limit?: number }) => {
      if (!readFile) {
        throw new Error("NAPI binding required: readFile")
      }
      return readFile(filePath, options)
    },

    /**
     * Edit a file (string replacement)
     */
    edit: (filePath: string, options: { oldString: string; newString: string; replaceAll?: boolean }) => {
      if (!editFile) {
        throw new Error("NAPI binding required: editFile")
      }
      return editFile(filePath, options)
    },

    /**
     * Search files with ripgrep
     */
    grep: (options: { pattern: string; path?: string; glob?: string }) => {
      if (!grep) {
        throw new Error("NAPI binding required: grep")
      }
      return grep(options)
    },

    /**
     * Find files matching a glob pattern
     */
    glob: (options: { pattern: string; path?: string }) => {
      if (!glob) {
        throw new Error("NAPI binding required: glob")
      }
      return glob(options)
    },
  },

  // ==========================================================================
  // Config Operations
  // ==========================================================================

  config: {
    /**
     * Create a config loader
     */
    createLoader: () => {
      if (!createConfigLoader) {
        throw new Error("NAPI binding required: createConfigLoader")
      }
      return createConfigLoader()
    },

    /**
     * Get ConfigLoaderHandle class (if available)
     */
    get ConfigLoaderHandle() {
      return ConfigLoaderHandle
    },
  },

  // ==========================================================================
  // Safety Operations
  // ==========================================================================

  safety: {
    /**
     * Assess risk of a bash command
     */
    assessBashRisk: (command: string) => {
      if (!assessBashRisk) {
        throw new Error("NAPI binding required: assessBashRisk")
      }
      return assessBashRisk(command)
    },

    /**
     * Assess risk of a file operation
     */
    assessFileRisk: (filePath: string) => {
      if (!assessFileRisk) {
        throw new Error("NAPI binding required: assessFileRisk")
      }
      return assessFileRisk(filePath)
    },
  },

  // ==========================================================================
  // Git Operations
  // ==========================================================================

  git: {
    /**
     * Open a git repository
     */
    openRepo: (path: string) => {
      if (!openGitRepo) {
        throw new Error("NAPI binding required: openGitRepo")
      }
      return openGitRepo(path)
    },

    /**
     * Check if a path is a git repository
     */
    isRepo: (path: string) => {
      if (!isGitRepo) {
        throw new Error("NAPI binding required: isGitRepo")
      }
      return isGitRepo(path)
    },
  },
}

// ============================================================================
// Re-exports for backward compatibility
// ============================================================================

export {
  // Memory
  MemorySystemHandle,
  createMemorySystem,
  MarkdownMemoryHandle,
  createMarkdownMemory,
  // Text
  chunkText,
  chunkTextWithConfig,
  estimateTokens,
  // Vector
  cosineSimilarity,
  normalizeVector,
  vectorDistance,
  vectorToBytes,
  bytesToVector,
  hybridMergeResults,
  // File
  readFile,
  editFile,
  grep,
  glob,
  // Config
  ConfigLoaderHandle,
  createConfigLoader,
  // Safety
  assessBashRisk,
  assessFileRisk,
  // Git
  openGitRepo,
  isGitRepo,
  // Embedding
  generateHashEmbedding,
  generateHashEmbeddingsBatch,
  hashEmbeddingSimilarity,
  // Check
  isNative,
}

export default NAPI
