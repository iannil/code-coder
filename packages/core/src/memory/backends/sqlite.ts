/**
 * SQLite Memory Backend
 *
 * Full-featured SQLite-backed memory implementation using:
 * - FTS5 virtual table for full-text search with BM25 scoring
 * - BLOB embeddings storage for future vector similarity search
 * - LRU-evicted embedding cache
 * - WAL mode for concurrent access
 *
 * Compatible with ZeroBot's SQLite schema (Rust implementation).
 *
 * @module memory/backends/sqlite
 */

import { Database } from "bun:sqlite"
import path from "path"
import os from "os"
import type { MemoryCategory, MemoryEntry, SqliteConfig, UnifiedMemory } from "../types"
import { DEFAULT_CONFIG } from "../types"

/**
 * SQLite Memory Backend
 *
 * Provides persistent memory storage with full-text search capabilities.
 * Schema-compatible with ZeroBot's Rust SQLite implementation.
 */
export class SqliteMemory implements UnifiedMemory {
  readonly name = "sqlite"
  private db: Database | null = null
  private readonly dbPath: string
  private readonly readOnly: boolean
  private readonly vectorWeight: number
  private readonly keywordWeight: number
  private readonly cacheMax: number

  constructor(config: SqliteConfig = {}) {
    const defaults = DEFAULT_CONFIG.sqlite
    this.dbPath = this.resolveDbPath(config.dbPath ?? defaults.dbPath)
    this.readOnly = config.readOnly ?? defaults.readOnly
    this.vectorWeight = config.vectorWeight ?? defaults.vectorWeight
    this.keywordWeight = config.keywordWeight ?? defaults.keywordWeight
    this.cacheMax = config.embeddingCacheSize ?? defaults.embeddingCacheSize
  }

  /**
   * Resolve database path with ~ expansion
   */
  private resolveDbPath(dbPath: string): string {
    if (dbPath.startsWith("~/")) {
      const home = os.homedir()
      return path.join(home, dbPath.slice(2))
    }
    return dbPath
  }

  /**
   * Ensure database directory exists
   */
  private async ensureDir(dirPath: string): Promise<void> {
    const dir = path.dirname(dirPath)
    await Bun.write(path.join(dir, ".keep"), "")
  }

  /**
   * Initialize or get existing database connection
   */
  private getConnection(): Database {
    if (this.db) return this.db

    // Ensure directory exists
    const dir = path.dirname(this.dbPath)
    try {
      Bun.spawnSync(["mkdir", "-p", dir])
    } catch {
      // Directory may already exist
    }

    this.db = new Database(this.dbPath, {
      readonly: this.readOnly,
      create: !this.readOnly,
    })

    if (!this.readOnly) {
      this.initSchema()
    }

    return this.db
  }

  /**
   * Initialize database schema (mirrors ZeroBot's schema)
   */
  private initSchema(): void {
    const db = this.db
    if (!db) return

    db.exec(`
      -- Enable WAL mode for concurrent access
      PRAGMA journal_mode = WAL;
      PRAGMA busy_timeout = 5000;

      -- Core memories table
      CREATE TABLE IF NOT EXISTS memories (
        id          TEXT PRIMARY KEY,
        key         TEXT NOT NULL UNIQUE,
        content     TEXT NOT NULL,
        category    TEXT NOT NULL DEFAULT 'core',
        embedding   BLOB,
        created_at  TEXT NOT NULL,
        updated_at  TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
      CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);

      -- FTS5 full-text search (BM25 scoring)
      CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
        key, content, content=memories, content_rowid=rowid
      );

      -- FTS5 triggers: keep in sync with memories table
      CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
        INSERT INTO memories_fts(rowid, key, content)
        VALUES (new.rowid, new.key, new.content);
      END;
      CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
        INSERT INTO memories_fts(memories_fts, rowid, key, content)
        VALUES ('delete', old.rowid, old.key, old.content);
      END;
      CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
        INSERT INTO memories_fts(memories_fts, rowid, key, content)
        VALUES ('delete', old.rowid, old.key, old.content);
        INSERT INTO memories_fts(rowid, key, content)
        VALUES (new.rowid, new.key, new.content);
      END;

      -- Embedding cache with LRU eviction
      CREATE TABLE IF NOT EXISTS embedding_cache (
        content_hash TEXT PRIMARY KEY,
        embedding    BLOB NOT NULL,
        created_at   TEXT NOT NULL,
        accessed_at  TEXT NOT NULL
      );
      CREATE INDEX IF NOT EXISTS idx_cache_accessed ON embedding_cache(accessed_at);
    `)
  }

  /**
   * Convert category enum to string
   */
  private categoryToStr(category: MemoryCategory): string {
    return category
  }

  /**
   * Generate UUID for new entries
   */
  private generateId(): string {
    return crypto.randomUUID()
  }

  /**
   * Get current ISO timestamp
   */
  private now(): string {
    return new Date().toISOString()
  }

  async store(key: string, content: string, category: MemoryCategory): Promise<void> {
    if (this.readOnly) {
      throw new Error("Cannot store in read-only mode")
    }

    const db = this.getConnection()
    const id = this.generateId()
    const now = this.now()
    const cat = this.categoryToStr(category)

    db.run(
      `INSERT INTO memories (id, key, content, category, created_at, updated_at)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6)
       ON CONFLICT(key) DO UPDATE SET
          content = excluded.content,
          category = excluded.category,
          updated_at = excluded.updated_at`,
      [id, key, content, cat, now, now],
    )
  }

  async recall(query: string, limit: number = 10): Promise<MemoryEntry[]> {
    if (!query.trim()) return []

    const db = this.getConnection()

    // Build FTS5 query (quote each word for exact matching)
    const ftsQuery = query
      .split(/\s+/)
      .filter(Boolean)
      .map((w) => `"${w.replace(/"/g, '""')}"`)
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

      if (rows.length > 0) {
        return rows.map((row) => ({
          id: row.id,
          key: row.key,
          content: row.content,
          category: row.category,
          timestamp: row.created_at,
          score: -row.score, // BM25 returns negative scores (lower = better)
          source: "sqlite" as const,
        }))
      }
    } catch {
      // FTS5 query failed, fall through to LIKE search
    }

    // Fall back to LIKE search
    return this.recallWithLike(query, limit)
  }

  /**
   * Fallback LIKE-based search when FTS5 fails
   */
  private recallWithLike(query: string, limit: number): MemoryEntry[] {
    const db = this.getConnection()
    const keywords = query.split(/\s+/).filter(Boolean)

    if (keywords.length === 0) return []

    // Build OR conditions for each keyword
    const conditions = keywords.map((_, i) => `(content LIKE ?${i * 2 + 1} OR key LIKE ?${i * 2 + 2})`).join(" OR ")

    const sql = `
      SELECT id, key, content, category, created_at
      FROM memories
      WHERE ${conditions}
      ORDER BY updated_at DESC
      LIMIT ?${keywords.length * 2 + 1}
    `

    const params: (string | number)[] = []
    for (const kw of keywords) {
      const pattern = `%${kw}%`
      params.push(pattern, pattern)
    }
    params.push(limit)

    const stmt = db.prepare(sql)
    const rows = stmt.all(...params) as Array<{
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
      source: "sqlite" as const,
    }))
  }

  async get(key: string): Promise<MemoryEntry | null> {
    const db = this.getConnection()

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
      source: "sqlite",
    }
  }

  async list(category?: MemoryCategory): Promise<MemoryEntry[]> {
    const db = this.getConnection()

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
      source: "sqlite" as const,
    }))
  }

  async forget(key: string): Promise<boolean> {
    if (this.readOnly) {
      throw new Error("Cannot forget in read-only mode")
    }

    const db = this.getConnection()
    const result = db.run("DELETE FROM memories WHERE key = ?1", [key])
    return result.changes > 0
  }

  async count(): Promise<number> {
    const db = this.getConnection()
    const row = db.prepare("SELECT COUNT(*) as count FROM memories").get() as { count: number }
    return row.count
  }

  async healthCheck(): Promise<boolean> {
    try {
      const db = this.getConnection()
      db.prepare("SELECT 1").get()
      return true
    } catch {
      return false
    }
  }

  async close(): Promise<void> {
    if (this.db) {
      this.db.close()
      this.db = null
    }
  }

  /**
   * Check if the database exists and is accessible
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
   * Check if this provider is in writable mode
   */
  isWritable(): boolean {
    return !this.readOnly
  }

  /**
   * Rebuild FTS5 index
   */
  async reindex(): Promise<number> {
    if (this.readOnly) {
      throw new Error("Cannot reindex in read-only mode")
    }

    const db = this.getConnection()
    db.exec("INSERT INTO memories_fts(memories_fts) VALUES('rebuild');")
    return 0
  }

  /**
   * Get database path for diagnostics
   */
  getDbPath(): string {
    return this.dbPath
  }
}

/**
 * Create a SQLite memory backend with default configuration
 */
export function createSqliteMemory(config?: SqliteConfig): SqliteMemory {
  return new SqliteMemory(config)
}
