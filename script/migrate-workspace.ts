#!/usr/bin/env bun
/**
 * Workspace Migration Script
 *
 * Migrates all CodeCoder runtime data from scattered locations to unified workspace.
 *
 * Current locations -> New workspace locations:
 * - ~/.codecoder/hands/ -> ~/.codecoder/workspace/hands/
 * - ~/.local/share/ccode/storage/ -> ~/.codecoder/workspace/storage/
 * - ~/.local/share/ccode/log/ -> ~/.codecoder/workspace/log/
 * - ~/.local/share/ccode/tool-output/ -> ~/.codecoder/workspace/tool-output/
 * - ~/.local/share/ccode/bin/ -> ~/.codecoder/workspace/bin/ (skip, contains node_modules)
 * - ~/.local/share/ccode/snapshot/ -> ~/.codecoder/workspace/storage/snapshot/
 * - ~/.local/state/ccode/ -> ~/.codecoder/workspace/knowledge/
 * - ~/.codecoder/*.db -> ~/.codecoder/workspace/storage/
 */

import os from "os"
import path from "path"
import { promises as fs } from "fs"

const HOME = os.homedir()
const CODECODER_DIR = path.join(HOME, ".codecoder")
const WORKSPACE_DIR = path.join(CODECODER_DIR, "workspace")

// Source directories to migrate
const SOURCES = {
  hands: path.join(CODECODER_DIR, "hands"),
  storage: path.join(HOME, ".local", "share", "ccode", "storage"),
  log: path.join(HOME, ".local", "share", "ccode", "log"),
  toolOutput: path.join(HOME, ".local", "share", "ccode", "tool-output"),
  snapshot: path.join(HOME, ".local", "share", "ccode", "snapshot"),
  state: path.join(HOME, ".local", "state", "ccode"),
  bin: path.join(HOME, ".local", "share", "ccode", "bin"),
}

// Destination directories in workspace
const DESTINATIONS = {
  hands: path.join(WORKSPACE_DIR, "hands"),
  storage: path.join(WORKSPACE_DIR, "storage"),
  log: path.join(WORKSPACE_DIR, "log"),
  toolOutput: path.join(WORKSPACE_DIR, "tool-output"),
  knowledge: path.join(WORKSPACE_DIR, "knowledge"),
  snapshot: path.join(WORKSPACE_DIR, "storage", "snapshot"),
  cache: path.join(WORKSPACE_DIR, "cache"),
}

// Database files to migrate
const DB_FILES = [
  "financial.db",
  "financial.db-shm",
  "financial.db-wal",
  "gateway.db",
  "metering.db",
  "hitl.db",
]

async function dirExists(dir: string): Promise<boolean> {
  try {
    const stat = await fs.stat(dir)
    return stat.isDirectory()
  } catch {
    return false
  }
}

async function fileExists(file: string): Promise<boolean> {
  try {
    const stat = await fs.stat(file)
    return stat.isFile()
  } catch {
    return false
  }
}

async function copyDirectory(src: string, dest: string): Promise<void> {
  const srcExists = await dirExists(src)
  if (!srcExists) {
    console.log(`  ⊘ Source does not exist: ${src}`)
    return
  }

  await fs.mkdir(dest, { recursive: true })
  console.log(`  Copying ${src} -> ${dest}`)

  const entries = await fs.readdir(src, { withFileTypes: true })
  for (const entry of entries) {
    const srcPath = path.join(src, entry.name)
    const destPath = path.join(dest, entry.name)

    if (entry.isDirectory()) {
      await copyDirectory(srcPath, destPath)
    } else if (entry.isFile()) {
      await fs.copyFile(srcPath, destPath)
    }
  }
}

async function copyFile(src: string, dest: string): Promise<void> {
  const srcExists = await fileExists(src)
  if (!srcExists) {
    console.log(`  ⊘ Source does not exist: ${src}`)
    return
  }

  await fs.mkdir(path.dirname(dest), { recursive: true })
  console.log(`  Copying ${src} -> ${dest}`)
  await fs.copyFile(src, dest)
}

async function backupPath(srcPath: string, backupName: string): Promise<void> {
  const backupDir = path.join(CODECODER_DIR, "backup", backupName)
  const srcExists = await dirExists(srcPath) || await fileExists(srcPath)

  if (!srcExists) {
    return
  }

  console.log(`  Backing up ${srcPath} -> ${backupDir}`)
  await fs.mkdir(backupDir, { recursive: true })

  const stat = await fs.stat(srcPath)
  if (stat.isDirectory()) {
    await copyDirectory(srcPath, backupDir)
  } else if (stat.isFile()) {
    await fs.mkdir(path.dirname(backupDir), { recursive: true })
    await fs.copyFile(srcPath, backupDir)
  }
}

async function migrate(): Promise<void> {
  console.log("🔄 CodeCoder Workspace Migration")
  console.log("=" .repeat(50))
  console.log(`Workspace: ${WORKSPACE_DIR}`)
  console.log("")

  // Create backup directory with timestamp
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19)
  const backupDir = path.join(CODECODER_DIR, "backup", `before-workspace-migration-${timestamp}`)
  await fs.mkdir(backupDir, { recursive: true })
  console.log(`📦 Backup directory: ${backupDir}`)
  console.log("")

  // Step 1: Create workspace directories
  console.log("📁 Creating workspace directories...")
  for (const dest of Object.values(DESTINATIONS)) {
    await fs.mkdir(dest, { recursive: true })
  }
  console.log("  ✓ Workspace directories created")
  console.log("")

  // Step 2: Backup and migrate hands
  console.log("🔄 Migrating hands...")
  await backupPath(SOURCES.hands, "hands")
  await copyDirectory(SOURCES.hands, DESTINATIONS.hands)
  console.log("  ✓ Hands migrated")
  console.log("")

  // Step 3: Backup and migrate storage
  console.log("🔄 Migrating storage...")
  await backupPath(SOURCES.storage, "storage")
  await copyDirectory(SOURCES.storage, DESTINATIONS.storage)
  console.log("  ✓ Storage migrated")
  console.log("")

  // Step 4: Backup and migrate log
  console.log("🔄 Migrating log...")
  await backupPath(SOURCES.log, "log")
  await copyDirectory(SOURCES.log, DESTINATIONS.log)
  console.log("  ✓ Log migrated")
  console.log("")

  // Step 5: Backup and migrate tool-output
  console.log("🔄 Migrating tool-output...")
  await backupPath(SOURCES.toolOutput, "tool-output")
  await copyDirectory(SOURCES.toolOutput, DESTINATIONS.toolOutput)
  console.log("  ✓ Tool-output migrated")
  console.log("")

  // Step 6: Backup and migrate snapshot (as part of storage)
  console.log("🔄 Migrating snapshot...")
  await backupPath(SOURCES.snapshot, "snapshot")
  await copyDirectory(SOURCES.snapshot, DESTINATIONS.snapshot)
  console.log("  ✓ Snapshot migrated")
  console.log("")

  // Step 7: Backup and migrate state files (as knowledge)
  console.log("🔄 Migrating state files (knowledge)...")
  await backupPath(SOURCES.state, "state")
  if (await dirExists(SOURCES.state)) {
    await copyDirectory(SOURCES.state, DESTINATIONS.knowledge)
    console.log("  ✓ State files migrated")
  }
  console.log("")

  // Step 8: Migrate database files
  console.log("🔄 Migrating database files...")
  for (const db of DB_FILES) {
    const src = path.join(CODECODER_DIR, db)
    const dest = path.join(DESTINATIONS.storage, db)

    if (await fileExists(src)) {
      // Backup
      const backupPath = path.join(backupDir, db)
      await fs.mkdir(path.dirname(backupPath), { recursive: true })
      await fs.copyFile(src, backupPath)
      console.log(`  Backed up ${db}`)

      // Copy to workspace
      await copyFile(src, dest)
    }
  }
  console.log("  ✓ Databases migrated")
  console.log("")

  // Step 9: Copy daemon_state.json
  console.log("🔄 Migrating daemon_state.json...")
  const daemonStateSrc = path.join(CODECODER_DIR, "daemon_state.json")
  const daemonStateDest = path.join(WORKSPACE_DIR, "daemon_state.json")
  if (await fileExists(daemonStateSrc)) {
    await backupPath(daemonStateSrc, "daemon_state.json")
    await copyFile(daemonStateSrc, daemonStateDest)
    console.log("  ✓ daemon_state.json migrated")
  }
  console.log("")

  // Step 10: Copy auth.json to knowledge
  console.log("🔄 Migrating auth.json...")
  const authSrc = path.join(SOURCES.storage, "..", "auth.json")
  const authDest = path.join(DESTINATIONS.knowledge, "auth.json")
  if (await fileExists(authSrc)) {
    await backupPath(authSrc, "auth.json")
    await copyFile(authSrc, authDest)
    console.log("  ✓ auth.json migrated")
  }
  console.log("")

  // Generate migration report
  const reportPath = path.join(CODECODER_DIR, "backup", "migration-report.md")
  const report = `# Workspace Migration Report

**Date**: ${new Date().toISOString()}

## Migration Summary

- **Source locations**:
  - \`~/.codecoder/hands/\`
  - \`~/.local/share/ccode/\`
  - \`~/.local/state/ccode/\`

- **Destination**: \`${WORKSPACE_DIR}\`

## Backup Location

\`${backupDir}\`

## Next Steps

1. Verify that all data has been correctly migrated
2. Test that CodeCoder works with the new workspace configuration
3. If everything works, you can remove the backup directory (not recommended immediately)

## Rollback

If you need to rollback:
1. Remove \`workspace\` field from ~/.codecoder/config.json
2. Restore data from backup: \`cp -r ${backupDir}/* ~/.local/share/ccode/\` and \`cp -r ${backupDir}/hands ~/.codecoder/\`
`

  await fs.writeFile(reportPath, report)
  console.log(`📄 Migration report saved to: ${reportPath}`)
  console.log("")

  console.log("✅ Migration complete!")
  console.log("")
  console.log("📋 Next steps:")
  console.log("   1. Update ~/.codecoder/config.json with workspace configuration")
  console.log("   2. Restart CodeCoder services")
  console.log("   3. Verify everything works correctly")
  console.log("   4. If satisfied, you can remove the old data directories")
}

// Run migration
migrate().catch((err) => {
  console.error("❌ Migration failed:", err)
  process.exit(1)
})
