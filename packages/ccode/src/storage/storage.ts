/**
 * Storage Module
 *
 * Provides ACID-compliant key-value storage backed by SQLite via Rust NAPI bindings.
 * This module requires @codecoder-ai/core native bindings.
 *
 * For migrating existing file-based storage, use: bun run src/storage/migrate.ts
 */

import { Log } from "@/util/log"
import path from "path"
import { Global } from "../global"
import { lazy } from "@/util/lazy"
import { NamedError } from "@codecoder-ai/util/error"
import z from "zod"

export namespace Storage {
  const log = Log.create({ service: "storage" })

  // ============================================================================
  // Error Types
  // ============================================================================

  export const NotFoundError = NamedError.create(
    "NotFoundError",
    z.object({
      message: z.string(),
    }),
  )

  export const CorruptedError = NamedError.create(
    "CorruptedError",
    z.object({
      path: z.string(),
      message: z.string(),
      originalError: z.string().optional(),
      recovered: z.boolean().optional(),
    }),
  )

  export const StorageUnavailableError = NamedError.create(
    "StorageUnavailableError",
    z.object({
      message: z.string(),
    }),
  )

  // ============================================================================
  // Native KV Store Interface
  // ============================================================================

  interface NativeKVStoreHandle {
    set(key: string[], value: string): Promise<void>
    get(key: string[]): Promise<string | null>
    delete(key: string[]): Promise<boolean>
    exists(key: string[]): Promise<boolean>
    list(prefix: string[]): Promise<string[]>
    count(prefix: string[]): Promise<number>
    deletePrefix(prefix: string[]): Promise<number>
    stats(): Promise<{ totalEntries: number; totalSizeBytes: number }>
    healthCheck(): Promise<boolean>
    compact(): Promise<void>
    path(): string
  }

  interface NativeBindings {
    openKvStore: (path: string) => Promise<NativeKVStoreHandle>
  }

  // ============================================================================
  // Native Store Initialization
  // ============================================================================

  let kvStore: NativeKVStoreHandle | null = null

  async function loadNativeBindings(): Promise<NativeBindings> {
    try {
      const bindings = (await import("@codecoder-ai/core")) as unknown as Record<string, unknown>
      if (typeof bindings.openKvStore === "function") {
        return bindings as unknown as NativeBindings
      }
    } catch (e) {
      log.error("Failed to load @codecoder-ai/core native bindings", { error: e })
    }

    throw new StorageUnavailableError({
      message:
        "Native storage bindings not available. " +
        "Ensure @codecoder-ai/core is installed and built correctly. " +
        "Run: cd packages/core && bun run build",
    })
  }

  const state = lazy(async () => {
    const bindings = await loadNativeBindings()
    const dbPath = path.join(Global.Path.data, "storage.db")

    kvStore = await bindings.openKvStore(dbPath)
    log.info("Using native SQLite KV store", { path: dbPath })

    return { dbPath }
  })

  async function getStore(): Promise<NativeKVStoreHandle> {
    await state()
    if (!kvStore) {
      throw new StorageUnavailableError({ message: "KV store not initialized" })
    }
    return kvStore
  }

  // ============================================================================
  // Public API
  // ============================================================================

  /**
   * Remove a value by key
   */
  export async function remove(key: string[]): Promise<void> {
    const store = await getStore()
    await store.delete(key)
  }

  /**
   * Read a value by key
   */
  export async function read<T>(key: string[]): Promise<T> {
    const store = await getStore()
    const value = await store.get(key)

    if (value === null) {
      throw new NotFoundError({ message: `Resource not found: ${key.join("/")}` })
    }

    try {
      return JSON.parse(value) as T
    } catch (e) {
      throw new CorruptedError({
        path: key.join("/"),
        message: "Failed to parse JSON from KV store",
        originalError: e instanceof Error ? e.message : String(e),
        recovered: false,
      })
    }
  }

  /**
   * Update a value by key using a mutation function
   */
  export async function update<T>(key: string[], fn: (draft: T) => void): Promise<T> {
    const store = await getStore()
    const value = await store.get(key)

    if (value === null) {
      throw new NotFoundError({ message: `Resource not found: ${key.join("/")}` })
    }

    const content = JSON.parse(value) as T
    fn(content)
    await store.set(key, JSON.stringify(content))
    return content
  }

  /**
   * Write a value to a key
   */
  export async function write<T>(key: string[], content: T): Promise<void> {
    const store = await getStore()
    await store.set(key, JSON.stringify(content))
  }

  /**
   * List all keys with a prefix
   */
  export async function list(prefix: string[]): Promise<string[][]> {
    const store = await getStore()
    const keys = await store.list(prefix)
    // Convert from "a/b/c" format to ["a", "b", "c"] format
    return keys.map((k) => k.split("/"))
  }

  /**
   * Check if a key exists
   */
  export async function exists(key: string[]): Promise<boolean> {
    const store = await getStore()
    return store.exists(key)
  }

  /**
   * Count entries with a prefix
   */
  export async function count(prefix: string[]): Promise<number> {
    const store = await getStore()
    return store.count(prefix)
  }

  /**
   * Delete all entries with a prefix
   */
  export async function deletePrefix(prefix: string[]): Promise<number> {
    const store = await getStore()
    return store.deletePrefix(prefix)
  }

  // ============================================================================
  // Health & Stats
  // ============================================================================

  export interface HealthReport {
    total: number
    healthy: number
    corrupted: { key: string[]; error: string }[]
    orphaned: string[]
  }

  /**
   * Health check for the storage
   */
  export async function healthCheck(prefix: string[]): Promise<HealthReport> {
    const store = await getStore()
    const isHealthy = await store.healthCheck()
    const stats = await store.stats()

    return {
      total: stats.totalEntries,
      healthy: isHealthy ? stats.totalEntries : 0,
      corrupted: [],
      orphaned: [],
    }
  }

  /**
   * Check if native SQLite storage is being used (always true now)
   */
  export function isUsingNative(): boolean {
    return kvStore !== null
  }

  /**
   * Get storage statistics
   */
  export async function getStats(): Promise<{
    mode: "native"
    entries: number
    sizeBytes: number
    path: string
  }> {
    const store = await getStore()
    const stats = await store.stats()

    return {
      mode: "native",
      entries: stats.totalEntries,
      sizeBytes: stats.totalSizeBytes,
      path: store.path(),
    }
  }

  /**
   * Compact the storage (VACUUM for SQLite)
   */
  export async function compact(): Promise<void> {
    const store = await getStore()
    await store.compact()
    log.info("Storage compacted")
  }

  // ============================================================================
  // Deprecated Functions (kept for backward compatibility)
  // ============================================================================

  /**
   * @deprecated Backup is not needed for SQLite - it has ACID guarantees
   */
  export async function backup(_key: string[]): Promise<string | undefined> {
    log.debug("backup() is deprecated - SQLite provides ACID guarantees")
    return undefined
  }

  /**
   * @deprecated Restore is not needed for SQLite - it has ACID guarantees
   */
  export async function restore(_key: string[]): Promise<boolean> {
    log.debug("restore() is deprecated - SQLite provides ACID guarantees")
    return false
  }

  /**
   * @deprecated Use healthCheck() instead
   */
  export async function listCorrupted(): Promise<string[]> {
    return []
  }

  /**
   * @deprecated Use healthCheck() instead
   */
  export async function clearCorrupted(): Promise<number> {
    return 0
  }
}
