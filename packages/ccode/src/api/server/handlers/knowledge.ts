/**
 * Knowledge Base API Handler
 *
 * Provides RAG (Retrieval-Augmented Generation) capabilities for ZeroBot:
 * - Document upload with automatic chunking
 * - Semantic search using hybrid BM25 + vector search
 * - Document management (list, delete)
 *
 * Architecture mirrors the Rust zero-memory service:
 * - SQLite with FTS5 for keyword search
 * - OpenAI-compatible embedding API
 * - Hybrid merge scoring
 */

import type { HttpRequest, HttpResponse, RouteParams } from "../types"
import { jsonResponse, errorResponse } from "../middleware"
import z from "zod"
import { Database } from "bun:sqlite"
import { homedir } from "os"
import { join } from "path"
import { existsSync, mkdirSync } from "fs"

// ============================================================================
// Configuration
// ============================================================================

const KNOWLEDGE_DIR = join(homedir(), ".codecoder", "knowledge")
const DB_PATH = join(KNOWLEDGE_DIR, "knowledge.db")

// Embedding configuration
const EMBEDDING_MODEL = "text-embedding-3-small"
const EMBEDDING_DIMS = 1536
const MAX_TOKENS_PER_CHUNK = 512

// Search configuration
const VECTOR_WEIGHT = 0.7
const KEYWORD_WEIGHT = 0.3

// ============================================================================
// Request/Response Types
// ============================================================================

const UploadDocumentRequest = z.object({
  /** Base64-encoded file content or plain text */
  content: z.string(),
  /** Filename for identification */
  filename: z.string(),
  /** MIME type (text/markdown, text/plain, application/pdf) */
  mime_type: z.string().default("text/markdown"),
  /** Optional metadata */
  metadata: z.record(z.string(), z.string()).optional(),
})
type UploadDocumentRequest = z.infer<typeof UploadDocumentRequest>

const SearchRequest = z.object({
  /** Natural language search query */
  query: z.string().min(1),
  /** Maximum number of results */
  limit: z.number().int().min(1).max(50).default(10),
  /** Minimum relevance score (0-1) */
  min_score: z.number().min(0).max(1).optional(),
  /** Filter by document ID */
  document_id: z.string().optional(),
})
type SearchRequest = z.infer<typeof SearchRequest>

export interface KnowledgeDocument {
  id: string
  filename: string
  chunk_count: number
  created_at: string
  size_bytes: number
  metadata?: Record<string, string>
}

export interface KnowledgeChunk {
  id: string
  document_id: string
  content: string
  chunk_index: number
  heading?: string
}

export interface KnowledgeSearchResult {
  content: string
  score: number
  document_id: string
  chunk_index: number
  filename: string
  heading?: string
}

// ============================================================================
// Database Management
// ============================================================================

let db: Database | null = null

function getDatabase(): Database {
  if (db) return db

  // Ensure directory exists
  if (!existsSync(KNOWLEDGE_DIR)) {
    mkdirSync(KNOWLEDGE_DIR, { recursive: true })
  }

  db = new Database(DB_PATH)

  // Initialize schema
  db.exec(`
    -- Documents table
    CREATE TABLE IF NOT EXISTS documents (
      id TEXT PRIMARY KEY,
      filename TEXT NOT NULL,
      content_hash TEXT NOT NULL,
      size_bytes INTEGER NOT NULL,
      chunk_count INTEGER NOT NULL DEFAULT 0,
      metadata TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );

    -- Chunks table
    CREATE TABLE IF NOT EXISTS chunks (
      id TEXT PRIMARY KEY,
      document_id TEXT NOT NULL,
      chunk_index INTEGER NOT NULL,
      content TEXT NOT NULL,
      heading TEXT,
      embedding BLOB,
      created_at TEXT NOT NULL,
      FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
    );

    -- FTS5 virtual table for keyword search
    CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
      id,
      content,
      content='chunks',
      content_rowid='rowid'
    );

    -- Triggers for FTS sync
    CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
      INSERT INTO chunks_fts(rowid, id, content)
      VALUES (new.rowid, new.id, new.content);
    END;

    CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
      INSERT INTO chunks_fts(chunks_fts, rowid, id, content)
      VALUES ('delete', old.rowid, old.id, old.content);
    END;

    CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
      INSERT INTO chunks_fts(chunks_fts, rowid, id, content)
      VALUES ('delete', old.rowid, old.id, old.content);
      INSERT INTO chunks_fts(rowid, id, content)
      VALUES (new.rowid, new.id, new.content);
    END;

    -- Index for document lookups
    CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id);
  `)

  return db
}

// ============================================================================
// Text Chunking (port from Rust chunker)
// ============================================================================

interface Chunk {
  index: number
  content: string
  heading?: string
}

/**
 * Split markdown text into semantic chunks.
 *
 * Strategy:
 * 1. Split on ## and # headings
 * 2. If section exceeds max_tokens, split on paragraphs
 * 3. If paragraph still exceeds, split on lines
 */
function chunkMarkdown(text: string, maxTokens: number): Chunk[] {
  if (!text.trim()) return []

  const maxChars = maxTokens * 4 // ~4 chars per token
  const sections = splitOnHeadings(text)
  const chunks: Chunk[] = []

  for (const { heading, body } of sections) {
    const full = heading ? `${heading}\n${body}` : body

    if (full.length <= maxChars) {
      const trimmed = full.trim()
      if (trimmed) {
        chunks.push({
          index: chunks.length,
          content: trimmed,
          heading: heading?.trim(),
        })
      }
      continue
    }

    // Split on paragraphs
    const paragraphs = splitOnBlankLines(body)
    let current = heading ? `${heading}\n` : ""

    for (const para of paragraphs) {
      if (current.length + para.length > maxChars && current.trim()) {
        chunks.push({
          index: chunks.length,
          content: current.trim(),
          heading: heading?.trim(),
        })
        current = heading ? `${heading}\n` : ""
      }

      if (para.length > maxChars) {
        // Paragraph too big - split on lines
        if (current.trim()) {
          chunks.push({
            index: chunks.length,
            content: current.trim(),
            heading: heading?.trim(),
          })
          current = heading ? `${heading}\n` : ""
        }

        for (const lineChunk of splitOnLines(para, maxChars)) {
          const trimmed = lineChunk.trim()
          if (trimmed) {
            chunks.push({
              index: chunks.length,
              content: trimmed,
              heading: heading?.trim(),
            })
          }
        }
      } else {
        current += para + "\n"
      }
    }

    if (current.trim()) {
      chunks.push({
        index: chunks.length,
        content: current.trim(),
        heading: heading?.trim(),
      })
    }
  }

  // Re-index
  return chunks.map((c, i) => ({ ...c, index: i }))
}

function splitOnHeadings(text: string): Array<{ heading?: string; body: string }> {
  const sections: Array<{ heading?: string; body: string }> = []
  let currentHeading: string | undefined
  let currentBody = ""

  for (const line of text.split("\n")) {
    if (line.startsWith("# ") || line.startsWith("## ") || line.startsWith("### ")) {
      if (currentBody.trim() || currentHeading) {
        sections.push({ heading: currentHeading, body: currentBody })
        currentBody = ""
      }
      currentHeading = line
    } else {
      currentBody += line + "\n"
    }
  }

  if (currentBody.trim() || currentHeading) {
    sections.push({ heading: currentHeading, body: currentBody })
  }

  return sections
}

function splitOnBlankLines(text: string): string[] {
  const paragraphs: string[] = []
  let current = ""

  for (const line of text.split("\n")) {
    if (!line.trim()) {
      if (current.trim()) {
        paragraphs.push(current)
        current = ""
      }
    } else {
      current += line + "\n"
    }
  }

  if (current.trim()) {
    paragraphs.push(current)
  }

  return paragraphs
}

function splitOnLines(text: string, maxChars: number): string[] {
  const chunks: string[] = []
  let current = ""

  for (const line of text.split("\n")) {
    if (current.length + line.length + 1 > maxChars && current) {
      chunks.push(current)
      current = ""
    }
    current += line + "\n"
  }

  if (current) {
    chunks.push(current)
  }

  return chunks
}

// ============================================================================
// Embedding Service
// ============================================================================

/**
 * Get embeddings from OpenAI-compatible API.
 */
async function getEmbeddings(texts: string[]): Promise<number[][] | null> {
  if (!texts.length) return []

  const apiKey = process.env.OPENAI_API_KEY
  if (!apiKey) {
    console.warn("[knowledge] No OPENAI_API_KEY found, falling back to keyword-only search")
    return null
  }

  const baseUrl = process.env.OPENAI_BASE_URL || "https://api.openai.com"

  const response = await fetch(`${baseUrl}/v1/embeddings`, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: EMBEDDING_MODEL,
      input: texts,
    }),
  })

  if (!response.ok) {
    const errorText = await response.text()
    console.error(`[knowledge] Embedding API error: ${response.status} ${errorText}`)
    return null
  }

  const data = await response.json()
  return data.data.map((item: { embedding: number[] }) => item.embedding)
}

/**
 * Convert embedding array to binary format (little-endian float32).
 */
function embeddingToBytes(embedding: number[]): Uint8Array {
  const buffer = new ArrayBuffer(embedding.length * 4)
  const view = new DataView(buffer)
  embedding.forEach((val, i) => view.setFloat32(i * 4, val, true))
  return new Uint8Array(buffer)
}

/**
 * Convert binary format back to embedding array.
 */
function bytesToEmbedding(bytes: Uint8Array): number[] {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const embedding: number[] = []
  for (let i = 0; i < bytes.length; i += 4) {
    embedding.push(view.getFloat32(i, true))
  }
  return embedding
}

/**
 * Compute cosine similarity between two vectors.
 */
function cosineSimilarity(a: number[], b: number[]): number {
  if (a.length !== b.length || !a.length) return 0

  let dot = 0
  let normA = 0
  let normB = 0

  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i]
    normA += a[i] * a[i]
    normB += b[i] * b[i]
  }

  const denom = Math.sqrt(normA) * Math.sqrt(normB)
  if (!isFinite(denom) || denom < Number.EPSILON) return 0

  const sim = dot / denom
  return Math.max(0, Math.min(1, sim))
}

// ============================================================================
// Search Functions
// ============================================================================

interface ScoredResult {
  id: string
  vectorScore?: number
  keywordScore?: number
  finalScore: number
}

/**
 * Vector similarity search using stored embeddings.
 */
function vectorSearch(database: Database, queryEmbedding: number[], limit: number): Array<[string, number]> {
  const stmt = database.query<{ id: string; embedding: Uint8Array }, []>(
    "SELECT id, embedding FROM chunks WHERE embedding IS NOT NULL"
  )

  const results: Array<[string, number]> = []

  for (const row of stmt.all()) {
    const embedding = bytesToEmbedding(row.embedding)
    const score = cosineSimilarity(queryEmbedding, embedding)
    if (score > 0) {
      results.push([row.id, score])
    }
  }

  results.sort((a, b) => b[1] - a[1])
  return results.slice(0, limit)
}

/**
 * Keyword search using FTS5.
 */
function keywordSearch(database: Database, query: string, limit: number): Array<[string, number]> {
  if (!query.trim()) return []

  // Escape special FTS5 characters
  const escaped = query
    .split("")
    .map((c) => (/[a-zA-Z0-9\u4e00-\u9fff\s]/.test(c) ? c : " "))
    .join("")
    .split(/\s+/)
    .filter(Boolean)
    .join(" OR ")

  if (!escaped) return []

  const stmt = database.query<{ id: string; score: number }, [string]>(
    `SELECT id, -bm25(chunks_fts) as score
     FROM chunks_fts
     WHERE chunks_fts MATCH ?1
     ORDER BY score DESC
     LIMIT ${limit}`
  )

  return stmt.all(escaped).map((row) => [row.id, row.score])
}

/**
 * Hybrid merge: combine vector and keyword results with weighted fusion.
 */
function hybridMerge(
  vectorResults: Array<[string, number]>,
  keywordResults: Array<[string, number]>,
  vectorWeight: number,
  keywordWeight: number,
  limit: number
): ScoredResult[] {
  const map = new Map<string, ScoredResult>()

  // Add vector results (already normalized 0-1)
  for (const [id, score] of vectorResults) {
    map.set(id, { id, vectorScore: score, finalScore: 0 })
  }

  // Normalize keyword results (BM25 can be any positive number)
  const maxKw = Math.max(...keywordResults.map(([, s]) => s), 0.001)

  for (const [id, score] of keywordResults) {
    const normalized = score / maxKw
    const existing = map.get(id)
    if (existing) {
      existing.keywordScore = normalized
    } else {
      map.set(id, { id, keywordScore: normalized, finalScore: 0 })
    }
  }

  // Compute final scores
  const results = Array.from(map.values()).map((r) => ({
    ...r,
    finalScore: vectorWeight * (r.vectorScore ?? 0) + keywordWeight * (r.keywordScore ?? 0),
  }))

  results.sort((a, b) => b.finalScore - a.finalScore)
  return results.slice(0, limit)
}

// ============================================================================
// Helper Functions
// ============================================================================

async function readRequestBody(body: ReadableStream | null | undefined): Promise<string> {
  if (!body) throw new Error("Request body is empty")
  return await new Response(body).text()
}

function generateId(): string {
  return crypto.randomUUID()
}

function hashContent(content: string): string {
  return Bun.hash(content).toString(16)
}

// ============================================================================
// Handler Functions
// ============================================================================

/**
 * POST /api/v1/knowledge/upload
 *
 * Upload and index a document.
 */
export async function uploadDocument(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = UploadDocumentRequest.parse(JSON.parse(body))

    const database = getDatabase()

    // Decode content if base64
    let content: string
    if (input.mime_type === "text/plain" || input.mime_type === "text/markdown") {
      // Check if it looks like base64
      if (/^[A-Za-z0-9+/=]+$/.test(input.content) && input.content.length > 100) {
        content = Buffer.from(input.content, "base64").toString("utf-8")
      } else {
        content = input.content
      }
    } else {
      return errorResponse("Unsupported MIME type. Supported: text/plain, text/markdown", 400)
    }

    const contentHash = hashContent(content)
    const sizeBytes = Buffer.byteLength(content, "utf-8")

    // Check for duplicate
    const existing = database
      .query<{ id: string }, [string]>("SELECT id FROM documents WHERE content_hash = ?")
      .get(contentHash)

    if (existing) {
      return jsonResponse({
        success: true,
        data: {
          id: existing.id,
          message: "Document already exists",
          duplicate: true,
        },
      })
    }

    // Generate document ID
    const docId = generateId()
    const now = new Date().toISOString()

    // Chunk the document
    const chunks = chunkMarkdown(content, MAX_TOKENS_PER_CHUNK)

    // Get embeddings for all chunks
    const chunkTexts = chunks.map((c) => c.content)
    const embeddings = await getEmbeddings(chunkTexts)

    // Insert document
    database
      .query(
        `INSERT INTO documents (id, filename, content_hash, size_bytes, chunk_count, metadata, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
      )
      .run(
        docId,
        input.filename,
        contentHash,
        sizeBytes,
        chunks.length,
        input.metadata ? JSON.stringify(input.metadata) : null,
        now,
        now
      )

    // Insert chunks
    const insertChunk = database.prepare(
      `INSERT INTO chunks (id, document_id, chunk_index, content, heading, embedding, created_at)
       VALUES (?, ?, ?, ?, ?, ?, ?)`
    )

    for (let i = 0; i < chunks.length; i++) {
      const chunk = chunks[i]
      const embedding = embeddings?.[i] ? embeddingToBytes(embeddings[i]) : null
      insertChunk.run(generateId(), docId, chunk.index, chunk.content, chunk.heading ?? null, embedding, now)
    }

    return jsonResponse(
      {
        success: true,
        data: {
          id: docId,
          filename: input.filename,
          chunk_count: chunks.length,
          size_bytes: sizeBytes,
          has_embeddings: !!embeddings,
        },
      },
      201
    )
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Invalid request: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/knowledge/documents
 *
 * List all indexed documents.
 */
export async function listDocuments(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const database = getDatabase()

    const docs = database
      .query<
        {
          id: string
          filename: string
          chunk_count: number
          size_bytes: number
          metadata: string | null
          created_at: string
        },
        []
      >("SELECT id, filename, chunk_count, size_bytes, metadata, created_at FROM documents ORDER BY created_at DESC")
      .all()

    const documents: KnowledgeDocument[] = docs.map((doc) => ({
      id: doc.id,
      filename: doc.filename,
      chunk_count: doc.chunk_count,
      size_bytes: doc.size_bytes,
      created_at: doc.created_at,
      metadata: doc.metadata ? JSON.parse(doc.metadata) : undefined,
    }))

    return jsonResponse({
      success: true,
      data: {
        documents,
        total: documents.length,
      },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * DELETE /api/v1/knowledge/documents/:id
 *
 * Delete a document and its chunks.
 */
export async function deleteDocument(_req: HttpRequest, params: RouteParams): Promise<HttpResponse> {
  try {
    const docId = params.id
    if (!docId) {
      return errorResponse("Document ID is required", 400)
    }

    const database = getDatabase()

    // Check if document exists
    const doc = database.query<{ id: string }, [string]>("SELECT id FROM documents WHERE id = ?").get(docId)

    if (!doc) {
      return errorResponse("Document not found", 404)
    }

    // Delete chunks first (due to foreign key)
    database.query("DELETE FROM chunks WHERE document_id = ?").run(docId)

    // Delete document
    database.query("DELETE FROM documents WHERE id = ?").run(docId)

    return jsonResponse({
      success: true,
      data: { deleted: true, id: docId },
    })
  } catch (error) {
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * POST /api/v1/knowledge/search
 *
 * Semantic search across indexed documents.
 */
export async function searchKnowledge(req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const body = await readRequestBody(req.body)
    const input = SearchRequest.parse(JSON.parse(body))

    const database = getDatabase()
    const limit = input.limit * 2 // Over-fetch for hybrid merge

    // Get query embedding
    const queryEmbeddings = await getEmbeddings([input.query])
    const queryEmbedding = queryEmbeddings?.[0]

    // Perform searches
    const vectorResults = queryEmbedding ? vectorSearch(database, queryEmbedding, limit) : []
    const keywordResults = keywordSearch(database, input.query, limit)

    // Hybrid merge
    const merged = hybridMerge(vectorResults, keywordResults, VECTOR_WEIGHT, KEYWORD_WEIGHT, input.limit)

    // Apply minimum score filter
    const filtered = input.min_score ? merged.filter((r) => r.finalScore >= input.min_score!) : merged

    // Fetch chunk details
    const results: KnowledgeSearchResult[] = []

    for (const result of filtered) {
      const chunk = database
        .query<
          {
            content: string
            chunk_index: number
            document_id: string
            heading: string | null
          },
          [string]
        >("SELECT content, chunk_index, document_id, heading FROM chunks WHERE id = ?")
        .get(result.id)

      if (!chunk) continue

      // Apply document filter if specified
      if (input.document_id && chunk.document_id !== input.document_id) continue

      const doc = database
        .query<{ filename: string }, [string]>("SELECT filename FROM documents WHERE id = ?")
        .get(chunk.document_id)

      results.push({
        content: chunk.content,
        score: result.finalScore,
        document_id: chunk.document_id,
        chunk_index: chunk.chunk_index,
        filename: doc?.filename ?? "unknown",
        heading: chunk.heading ?? undefined,
      })
    }

    return jsonResponse({
      success: true,
      data: {
        results,
        total: results.length,
        query: input.query,
        search_mode: queryEmbedding ? "hybrid" : "keyword",
      },
    })
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResponse(`Invalid request: ${error.issues.map((e) => e.message).join(", ")}`, 400)
    }
    return errorResponse(error instanceof Error ? error.message : String(error), 500)
  }
}

/**
 * GET /api/v1/knowledge/health
 *
 * Health check for knowledge base service.
 */
export async function knowledgeHealth(_req: HttpRequest, _params: RouteParams): Promise<HttpResponse> {
  try {
    const database = getDatabase()

    const docCount =
      database.query<{ count: number }, []>("SELECT COUNT(*) as count FROM documents").get()?.count ?? 0

    const chunkCount =
      database.query<{ count: number }, []>("SELECT COUNT(*) as count FROM chunks").get()?.count ?? 0

    const embeddingCount =
      database
        .query<{ count: number }, []>("SELECT COUNT(*) as count FROM chunks WHERE embedding IS NOT NULL")
        .get()?.count ?? 0

    const hasOpenAiKey = !!process.env.OPENAI_API_KEY

    return jsonResponse({
      success: true,
      data: {
        status: "healthy",
        document_count: docCount,
        chunk_count: chunkCount,
        embedding_count: embeddingCount,
        embedding_enabled: hasOpenAiKey,
        search_mode: hasOpenAiKey ? "hybrid" : "keyword",
        db_path: DB_PATH,
      },
    })
  } catch (error) {
    return jsonResponse(
      {
        success: true,
        data: {
          status: "degraded",
          error: error instanceof Error ? error.message : String(error),
        },
      },
      200
    )
  }
}
