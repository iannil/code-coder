/**
 * Storage Migration Tool
 *
 * Migrates existing file-based JSON storage to SQLite KV store.
 * This is a one-time migration for users with existing data.
 *
 * Usage:
 *   bun run packages/ccode/src/storage/migrate.ts
 *   bun run packages/ccode/src/storage/migrate.ts --dry-run
 */

import { Log } from "@/util/log"
import path from "path"
import { Global } from "@/util/global"

const log = Log.create({ service: "storage.migrate" })

interface MigrationStats {
  total: number
  migrated: number
  failed: number
  skipped: number
  errors: Array<{ key: string; error: string }>
}

interface NativeKVStoreHandle {
  set(key: string[], value: string): Promise<void>
  get(key: string[]): Promise<string | null>
  exists(key: string[]): Promise<boolean>
  stats(): Promise<{ totalEntries: number; totalSizeBytes: number }>
}

interface NativeBindings {
  openKvStore: (path: string) => Promise<NativeKVStoreHandle>
}

async function loadNativeBindings(): Promise<NativeBindings | null> {
  try {
    const bindings = (await import("@codecoder-ai/core")) as unknown as Record<string, unknown>
    if (typeof bindings.openKvStore === "function") {
      return bindings as unknown as NativeBindings
    }
  } catch {
    // Native bindings not available
  }
  return null
}

/**
 * Migrate file-based storage to SQLite KV store
 */
export async function migrateToSqlite(options: { dryRun?: boolean } = {}): Promise<MigrationStats> {
  const stats: MigrationStats = {
    total: 0,
    migrated: 0,
    failed: 0,
    skipped: 0,
    errors: [],
  }

  const storageDir = path.join(Global.Path.data, "storage")
  const dbPath = path.join(Global.Path.data, "storage.db")

  log.info("Starting storage migration", {
    from: storageDir,
    to: dbPath,
    dryRun: options.dryRun ?? false,
  })

  // Load native KV store
  const bindings = await loadNativeBindings()
  if (!bindings) {
    throw new Error("Native KV store is required for migration - @codecoder-ai/core not available")
  }

  let kvStore: NativeKVStoreHandle
  try {
    kvStore = await bindings.openKvStore(dbPath)
  } catch (e) {
    log.error("Failed to open native KV store", { error: e })
    throw new Error("Native KV store is required for migration")
  }

  // Find all JSON files in storage directory
  const glob = new Bun.Glob("**/*.json")

  for await (const filePath of glob.scan({ cwd: storageDir, absolute: true })) {
    // Skip backup and corrupted directories
    if (filePath.includes("/_backup/") || filePath.includes("/_corrupted/")) {
      continue
    }

    // Skip migration marker file
    if (filePath.endsWith("/migration")) {
      continue
    }

    stats.total++

    // Parse key from file path
    const relativePath = path.relative(storageDir, filePath)
    const keyParts = relativePath.replace(/\.json$/, "").split(path.sep)

    try {
      // Read file content
      const content = await Bun.file(filePath).text()

      // Validate JSON
      try {
        JSON.parse(content)
      } catch {
        log.warn("Skipping invalid JSON file", { path: filePath })
        stats.skipped++
        continue
      }

      // Check if already exists in KV store
      const exists = await kvStore.exists(keyParts)
      if (exists) {
        log.debug("Key already exists in KV store, skipping", { key: keyParts.join("/") })
        stats.skipped++
        continue
      }

      if (options.dryRun) {
        log.info("[DRY RUN] Would migrate", { key: keyParts.join("/"), size: content.length })
      } else {
        // Write to KV store
        await kvStore.set(keyParts, content)
        log.debug("Migrated", { key: keyParts.join("/") })
      }

      stats.migrated++
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : String(e)
      log.error("Failed to migrate file", { path: filePath, error: errorMsg })
      stats.failed++
      stats.errors.push({ key: keyParts.join("/"), error: errorMsg })
    }
  }

  // Log summary
  log.info("Migration complete", {
    total: stats.total,
    migrated: stats.migrated,
    failed: stats.failed,
    skipped: stats.skipped,
    dryRun: options.dryRun ?? false,
  })

  if (stats.errors.length > 0) {
    log.warn("Migration errors", { errors: stats.errors })
  }

  // Get final stats
  const kvStats = await kvStore.stats()
  log.info("SQLite KV store stats", {
    entries: kvStats.totalEntries,
    sizeBytes: kvStats.totalSizeBytes,
  })

  return stats
}

/**
 * Verify migration by comparing file and KV store contents
 */
export async function verifyMigration(): Promise<{ matches: number; mismatches: number; details: string[] }> {
  const result = {
    matches: 0,
    mismatches: 0,
    details: [] as string[],
  }

  const storageDir = path.join(Global.Path.data, "storage")
  const dbPath = path.join(Global.Path.data, "storage.db")

  // Load native KV store
  const bindings = await loadNativeBindings()
  if (!bindings) {
    throw new Error("Native KV store is required for verification - @codecoder-ai/core not available")
  }

  let kvStore: NativeKVStoreHandle
  try {
    kvStore = await bindings.openKvStore(dbPath)
  } catch {
    throw new Error("Native KV store is required for verification")
  }

  const glob = new Bun.Glob("**/*.json")

  for await (const filePath of glob.scan({ cwd: storageDir, absolute: true })) {
    if (filePath.includes("/_backup/") || filePath.includes("/_corrupted/")) {
      continue
    }

    const relativePath = path.relative(storageDir, filePath)
    const keyParts = relativePath.replace(/\.json$/, "").split(path.sep)

    try {
      const fileContent = await Bun.file(filePath).text()
      const kvContent = await kvStore.get(keyParts)

      if (kvContent === null) {
        result.mismatches++
        result.details.push(`Missing in KV: ${keyParts.join("/")}`)
      } else {
        // Compare parsed JSON (to ignore formatting differences)
        const fileJson = JSON.parse(fileContent)
        const kvJson = JSON.parse(kvContent)

        if (JSON.stringify(fileJson) === JSON.stringify(kvJson)) {
          result.matches++
        } else {
          result.mismatches++
          result.details.push(`Content mismatch: ${keyParts.join("/")}`)
        }
      }
    } catch (e) {
      result.mismatches++
      result.details.push(`Error comparing ${keyParts.join("/")}: ${e instanceof Error ? e.message : String(e)}`)
    }
  }

  log.info("Verification complete", {
    matches: result.matches,
    mismatches: result.mismatches,
  })

  return result
}

// CLI entry point
if (import.meta.main) {
  const args = process.argv.slice(2)
  const dryRun = args.includes("--dry-run")
  const verify = args.includes("--verify")

  if (verify) {
    verifyMigration()
      .then((result) => {
        console.log("\nVerification Results:")
        console.log(`  Matches: ${result.matches}`)
        console.log(`  Mismatches: ${result.mismatches}`)
        if (result.details.length > 0) {
          console.log("\nDetails:")
          result.details.forEach((d) => console.log(`  - ${d}`))
        }
        process.exit(result.mismatches > 0 ? 1 : 0)
      })
      .catch((e) => {
        console.error("Verification failed:", e)
        process.exit(1)
      })
  } else {
    migrateToSqlite({ dryRun })
      .then((stats) => {
        console.log("\nMigration Results:")
        console.log(`  Total files: ${stats.total}`)
        console.log(`  Migrated: ${stats.migrated}`)
        console.log(`  Skipped: ${stats.skipped}`)
        console.log(`  Failed: ${stats.failed}`)
        process.exit(stats.failed > 0 ? 1 : 0)
      })
      .catch((e) => {
        console.error("Migration failed:", e)
        process.exit(1)
      })
  }
}
