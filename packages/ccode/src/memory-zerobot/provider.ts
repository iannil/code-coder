/**
 * ZeroBot Memory Provider
 *
 * Provides CodeCoder access to ZeroBot's SQLite memory database.
 * This enables shared memory between ZeroBot (Rust) and CodeCoder (TypeScript).
 *
 * Database schema (from ZeroBot):
 * - memories: id, key, content, category, embedding, created_at, updated_at
 * - memories_fts: FTS5 virtual table for full-text search
 * - embedding_cache: LRU cache for vector embeddings
 *
 * @module memory-zerobot/provider
 */

import { Database } from "bun:sqlite"
import path from "path"
import os from "os"
import { Log } from "@/util/log"
import type { MemoryEntry, ZeroBotMemoryConfig } from "./types"

const log = Log.create({ service: "memory-zerobot" })

/** Default ZeroBot workspace path */
const DEFAULT_WORKSPACE = path.join(os.homedir(), ".codecoder", "workspace")

/** Database filename within workspace */
const DEFAULT_DB_FILE = path.join("memory", "brain.db")

/**
 * ZeroBot Memory Provider
 *
 * Reads and writes to ZeroBot's SQLite memory database.
 */
export class ZeroBotMemoryProvider {
  private db: Database | null = null
  private readonly dbPath: string
  private readonly readOnly: boolean

  constructor(config: ZeroBotMemoryConfig = {}) {
    const workspace = config.workspacePath ?? DEFAULT_WORKSPACE
    const dbFile = config.dbFilename ?? DEFAULT_DB_FILE
    this.dbPath = path.join(workspace, dbFile)
    // Default to read-only to prevent accidental writes to ZeroBot's database
    this.readOnly = config.readOnly ?? true
  }

  /**
   * Initialize connection to ZeroBot's database
   */
  private ensureConnection(): Database {
    if (this.db) return this.db

    try {
      this.db = new Database(this.dbPath, {
        readonly: this.readOnly,
        create: false, // Don't create if doesn't exist
      })
      log.info("connected to ZeroBot memory", { path: this.dbPath })
    } catch (error) {
      log.error("failed to connect to ZeroBot memory", { error, path: this.dbPath })
      throw new Error(`ZeroBot memory not found at ${this.dbPath}. Ensure ZeroBot is configured.`)
    }

    return this.db
  }

  /**
   * Check if ZeroBot memory database exists
   */
  isAvailable(): boolean {
    try {
      const file = Bun.file(this.dbPath)
      return file.size > 0
    } catch {
      return false
    }
  }

  /**
   * Store a memory entry (upsert by key)
   */
  store(key: string, content: string, category: string = "core"): void {
    if (this.readOnly) {
      throw new Error("Cannot store in read-only mode")
    }

    const db = this.ensureConnection()
    const id = crypto.randomUUID()
    const now = new Date().toISOString()

    db.run(
      `INSERT INTO memories (id, key, content, category, created_at, updated_at)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)
       ON CONFLICT(key) DO UPDATE SET
          content = excluded.content,
          category = excluded.category,
          updated_at = excluded.updated_at`,
      [id, key, content, category, now, now],
    )

    log.debug("stored memory", { key, category })
  }

  /**
   * Check if this provider is in writable mode
   */
  isWritable(): boolean {
    return !this.readOnly
  }

  /**
   * Attempt to store a memory, returning false if in read-only mode
   *
   * This is a safe alternative to store() that won't throw on read-only mode.
   */
  tryStore(key: string, content: string, category: string = "core"): boolean {
    if (this.readOnly) {
      log.warn("attempted write in read-only mode", { key })
      return false
    }
    this.store(key, content, category)
    return true
  }

  /**
   * Recall memories matching a query using FTS5 search
   */
  recall(query: string, limit: number = 5): MemoryEntry[] {
    if (!query.trim()) return []

    const db = this.ensureConnection()

    // Build FTS5 query (quote each word)
    const ftsQuery = query
      .split(/\s+/)
      .filter(Boolean)
      .map((w) => `"${w}"`)
      .join(" OR ")

    if (!ftsQuery) return []

    try {
      // Try FTS5 search first
      const stmt = db.prepare(`
        SELECT m.id, m.key, m.content, m.category, m.created_at, bm25(memories_fts) as score
        FROM memories_fts f
        JOIN memories m ON m.rowid = f.rowid
        WHERE memories_fts MATCH ?1
        ORDER BY score
        LIMIT ?2
      `)

      const rows = stmt.all(ftsQuery, limit) as Array<{
        id: string
        key: string
        content: string
        category: string
        created_at: string
        score: number
      }>

      return rows.map((row) => ({
        id: row.id,
        key: row.key,
        content: row.content,
        category: row.category,
        timestamp: row.created_at,
        score: -row.score, // BM25 returns negative scores
      }))
    } catch {
      // Fall back to LIKE search
      log.debug("FTS5 search failed, falling back to LIKE", { query })
      return this.recallWithLike(query, limit)
    }
  }

  /**
   * Fallback LIKE-based search
   */
  private recallWithLike(query: string, limit: number): MemoryEntry[] {
    const db = this.ensureConnection()
    const pattern = `%${query}%`

    const stmt = db.prepare(`
      SELECT id, key, content, category, created_at
      FROM memories
      WHERE content LIKE ?1 OR key LIKE ?1
      ORDER BY updated_at DESC
      LIMIT ?2
    `)

    const rows = stmt.all(pattern, limit) as Array<{
      id: string
      key: string
      content: string
      category: string
      created_at: string
    }>

    return rows.map((row) => ({
      id: row.id,
      key: row.key,
      content: row.content,
      category: row.category,
      timestamp: row.created_at,
      score: 1.0,
    }))
  }

  /**
   * Get a specific memory by key
   */
  get(key: string): MemoryEntry | null {
    const db = this.ensureConnection()

    const stmt = db.prepare(`
      SELECT id, key, content, category, created_at
      FROM memories WHERE key = ?1
    `)

    const row = stmt.get(key) as {
      id: string
      key: string
      content: string
      category: string
      created_at: string
    } | null

    if (!row) return null

    return {
      id: row.id,
      key: row.key,
      content: row.content,
      category: row.category,
      timestamp: row.created_at,
    }
  }

  /**
   * List all memories, optionally filtered by category
   */
  list(category?: string): MemoryEntry[] {
    const db = this.ensureConnection()

    const sql = category
      ? `SELECT id, key, content, category, created_at FROM memories WHERE category = ?1 ORDER BY updated_at DESC`
      : `SELECT id, key, content, category, created_at FROM memories ORDER BY updated_at DESC`

    const stmt = db.prepare(sql)
    const rows = (category ? stmt.all(category) : stmt.all()) as Array<{
      id: string
      key: string
      content: string
      category: string
      created_at: string
    }>

    return rows.map((row) => ({
      id: row.id,
      key: row.key,
      content: row.content,
      category: row.category,
      timestamp: row.created_at,
    }))
  }

  /**
   * Remove a memory by key
   */
  forget(key: string): boolean {
    if (this.readOnly) {
      throw new Error("Cannot forget in read-only mode")
    }

    const db = this.ensureConnection()
    const result = db.run("DELETE FROM memories WHERE key = ?1", [key])
    return result.changes > 0
  }

  /**
   * Count total memories
   */
  count(): number {
    const db = this.ensureConnection()
    const row = db.prepare("SELECT COUNT(*) as count FROM memories").get() as { count: number }
    return row.count
  }

  /**
   * Close database connection
   */
  close(): void {
    if (this.db) {
      this.db.close()
      this.db = null
      log.debug("closed ZeroBot memory connection")
    }
  }
}

/**
 * Create a ZeroBot memory provider with default configuration
 */
export function createZeroBotMemory(config?: ZeroBotMemoryConfig): ZeroBotMemoryProvider {
  return new ZeroBotMemoryProvider(config)
}
