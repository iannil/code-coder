import fs from "fs/promises"
import path from "path"
import os from "os"

const homeDir = os.homedir()
const newBase = path.join(homeDir, ".codecoder")

interface MigrationPath {
  from: string
  to: string
  description: string
}

const MIGRATIONS: MigrationPath[] = [
  {
    from: path.join(homeDir, ".local", "share", "ccode", "storage"),
    to: path.join(newBase, "data", "storage"),
    description: "Session and message storage",
  },
  {
    from: path.join(homeDir, ".local", "share", "ccode", "memory"),
    to: path.join(newBase, "data", "memory"),
    description: "Memory database",
  },
  {
    from: path.join(homeDir, ".local", "share", "ccode", "snapshot"),
    to: path.join(newBase, "data", "snapshot"),
    description: "Snapshots",
  },
  {
    from: path.join(homeDir, ".local", "share", "ccode", "log"),
    to: path.join(newBase, "logs"),
    description: "Trace logs (XDG)",
  },
  {
    from: path.join(newBase, "log"),
    to: path.join(newBase, "logs"),
    description: "Trace logs (internal migration)",
  },
  {
    from: path.join(homeDir, ".local", "share", "ccode", "bin"),
    to: path.join(newBase, "bin"),
    description: "LSP binaries",
  },
  {
    from: path.join(homeDir, ".cache", "ccode"),
    to: path.join(newBase, "cache"),
    description: "Cache files",
  },
  {
    from: path.join(homeDir, ".local", "state", "ccode"),
    to: path.join(newBase, "state"),
    description: "State files",
  },
]

const MIGRATION_MARKER = path.join(newBase, ".migration-v1-complete")

async function exists(p: string): Promise<boolean> {
  return fs
    .access(p)
    .then(() => true)
    .catch(() => false)
}

async function copyDir(src: string, dest: string): Promise<void> {
  await fs.mkdir(dest, { recursive: true })
  const entries = await fs.readdir(src, { withFileTypes: true })

  for (const entry of entries) {
    const srcPath = path.join(src, entry.name)
    const destPath = path.join(dest, entry.name)

    if (entry.isDirectory()) {
      await copyDir(srcPath, destPath)
    } else {
      await fs.copyFile(srcPath, destPath)
    }
  }
}

export interface MigrationResult {
  migrated: string[]
  skipped: string[]
  errors: Array<{ path: string; error: string }>
  alreadyComplete: boolean
}

export async function checkMigrationNeeded(): Promise<boolean> {
  if (await exists(MIGRATION_MARKER)) {
    return false
  }

  for (const migration of MIGRATIONS) {
    if (await exists(migration.from)) {
      return true
    }
  }

  return false
}

export async function runMigration(dryRun = false): Promise<MigrationResult> {
  const result: MigrationResult = {
    migrated: [],
    skipped: [],
    errors: [],
    alreadyComplete: false,
  }

  if (await exists(MIGRATION_MARKER)) {
    result.alreadyComplete = true
    return result
  }

  for (const migration of MIGRATIONS) {
    const sourceExists = await exists(migration.from)
    const destExists = await exists(migration.to)

    if (!sourceExists) {
      result.skipped.push(`${migration.description}: source not found`)
      continue
    }

    if (destExists) {
      result.skipped.push(`${migration.description}: destination already exists`)
      continue
    }

    if (dryRun) {
      result.migrated.push(`${migration.description}: ${migration.from} -> ${migration.to}`)
      continue
    }

    try {
      await fs.mkdir(path.dirname(migration.to), { recursive: true })
      await copyDir(migration.from, migration.to)
      result.migrated.push(`${migration.description}: migrated successfully`)
    } catch (e) {
      result.errors.push({
        path: migration.from,
        error: e instanceof Error ? e.message : String(e),
      })
    }
  }

  if (!dryRun && result.errors.length === 0) {
    await fs.writeFile(MIGRATION_MARKER, new Date().toISOString())
  }

  return result
}

export async function printMigrationStatus(): Promise<void> {
  const needed = await checkMigrationNeeded()

  if (!needed) {
    const markerExists = await exists(MIGRATION_MARKER)
    if (markerExists) {
      console.log("✓ Storage migration already complete")
    } else {
      console.log("✓ No migration needed (no old data found)")
    }
    return
  }

  console.log("⚠ Storage migration needed")
  console.log("\nOld locations found:")

  for (const migration of MIGRATIONS) {
    if (await exists(migration.from)) {
      console.log(`  • ${migration.description}: ${migration.from}`)
    }
  }

  console.log("\nNew unified location: ~/.codecoder/")
  console.log("\nRun with --migrate to perform migration")
}

if (import.meta.main) {
  const args = process.argv.slice(2)
  const dryRun = args.includes("--dry-run")
  const migrate = args.includes("--migrate")

  if (migrate) {
    console.log(dryRun ? "Dry run mode - no changes will be made\n" : "Running migration...\n")
    const result = await runMigration(dryRun)

    if (result.alreadyComplete) {
      console.log("✓ Migration already complete")
    } else {
      if (result.migrated.length > 0) {
        console.log("Migrated:")
        result.migrated.forEach((m) => console.log(`  ✓ ${m}`))
      }
      if (result.skipped.length > 0) {
        console.log("\nSkipped:")
        result.skipped.forEach((s) => console.log(`  - ${s}`))
      }
      if (result.errors.length > 0) {
        console.log("\nErrors:")
        result.errors.forEach((e) => console.log(`  ✗ ${e.path}: ${e.error}`))
      }
    }
  } else {
    await printMigrationStatus()
  }
}
