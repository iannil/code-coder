/**
 * Snapshot Commands
 *
 * Commands for document version control - creating, listing, and rolling back snapshots.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Version } from "../../../document"

// ============================================================================
// Create Command
// ============================================================================

export const SnapshotCreateCommand = cmd({
  command: "create <documentID>",
  describe: "create a snapshot",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("message", {
        type: "string",
        alias: "m",
        describe: "snapshot message",
        demandOption: true,
      })
      .option("full", {
        type: "boolean",
        alias: "f",
        describe: "create full snapshot",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const spinner = prompts.spinner()
      spinner.start("Creating snapshot...")

      const snapshot = await Version.createSnapshot({
        documentID: args.documentID as string,
        message: args.message as string,
        createFull: args.full as boolean,
      })

      spinner.stop(`Snapshot created: ${snapshot.id}`)
      console.log()
      console.log(`  Chapters: ${snapshot.chapterCount}`)
      console.log(`  Words: ${snapshot.totalWords}`)
      console.log(`  Deltas: ${snapshot.chapterDeltas.length}`)
      console.log()
    })
  },
})

// ============================================================================
// List Command
// ============================================================================

export const SnapshotListCommand = cmd({
  command: "list <documentID>",
  describe: "list all snapshots",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const snapshots = await Version.list(args.documentID as string)

      if (snapshots.length === 0) {
        console.log("No snapshots found.")
        return
      }

      console.log()
      console.log(`Snapshots (${snapshots.length}):`)
      console.log()

      for (const snap of snapshots) {
        const date = new Date(snap.timestamp).toLocaleString()
        const type = snap.baselineSnapshotID ? "incremental" : "full"
        console.log(`${snap.id}`)
        console.log(`  ${date}`)
        console.log(`  ${snap.message}`)
        console.log(`  Type: ${type} | Chapters: ${snap.chapterCount} | Words: ${snap.totalWords}`)
        console.log()
      }
    })
  },
})

// ============================================================================
// Rollback Command
// ============================================================================

export const SnapshotRollbackCommand = cmd({
  command: "rollback <documentID>",
  describe: "rollback to a snapshot",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("snapshot", {
        type: "string",
        alias: "s",
        describe: "snapshot ID",
        demandOption: true,
      })
      .option("chapters", {
        type: "boolean",
        describe: "rollback chapters",
        default: true,
      })
      .option("entities", {
        type: "boolean",
        describe: "rollback entities",
        default: true,
      })
      .option("summary", {
        type: "boolean",
        describe: "rollback global summary",
        default: true,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const snapshot = await Version.get(args.documentID as string, args.snapshot as string)
      if (!snapshot) {
        UI.error("Snapshot not found")
        process.exit(1)
      }

      const confirmed = await prompts.confirm({
        message: `Rollback to "${snapshot.message}"?`,
      })
      if (prompts.isCancel(confirmed) || !confirmed) {
        console.log("Cancelled")
        return
      }

      const spinner = prompts.spinner()
      spinner.start("Rolling back...")

      await Version.rollback({
        documentID: args.documentID as string,
        snapshotID: args.snapshot as string,
        options: {
          chapters: args.chapters as boolean,
          entities: args.entities as boolean,
          globalSummary: args.summary as boolean,
        },
      })

      spinner.stop("Rollback complete")
      console.log()
    })
  },
})

// ============================================================================
// Diff Command
// ============================================================================

export const SnapshotDiffCommand = cmd({
  command: "diff <documentID>",
  describe: "show diff between snapshots",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("from", {
        type: "string",
        alias: "f",
        describe: "from snapshot ID",
        demandOption: true,
      })
      .option("to", {
        type: "string",
        alias: "t",
        describe: "to snapshot ID (default: current)",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const diff = await Version.diff({
        documentID: args.documentID as string,
        fromSnapshotID: args.from as string,
        toSnapshotID: args.to as string,
      })

      console.log()
      console.log("Chapter Changes:")
      for (const change of diff.chaptersChanged) {
        console.log(`  [${change.action}] ${change.title}`)
        if (change.wordCountDiff !== undefined) {
          const diffStr = change.wordCountDiff > 0 ? `+${change.wordCountDiff}` : `${change.wordCountDiff}`
          console.log(`    Words: ${diffStr}`)
        }
      }
      console.log()

      if (diff.entitiesChanged.length > 0) {
        console.log("Entity Changes:")
        for (const change of diff.entitiesChanged) {
          console.log(`  [${change.action}] ${change.name}`)
        }
        console.log()
      }

      console.log(`Global Summary Changed: ${diff.globalSummaryChanged}`)
      console.log(`Time Difference: ${Math.round(diff.timeDifference / 60000)} minutes`)
      console.log()
    })
  },
})

// ============================================================================
// Main Snapshot Command (groups all subcommands)
// ============================================================================

export const SnapshotCommand = cmd({
  command: "snapshot",
  describe: "manage document snapshots",
  builder: (yargs) =>
    yargs
      .command(SnapshotCreateCommand)
      .command(SnapshotListCommand)
      .command(SnapshotRollbackCommand)
      .command(SnapshotDiffCommand)
      .demandCommand(),
  async handler() {},
})
