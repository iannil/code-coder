/**
 * Chapter Commands
 *
 * Commands for managing document chapters.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document } from "../../../document"
import { DocumentSchema } from "../../../document/schema"
import { validatePath } from "../../../util/security"

// ============================================================================
// List Command
// ============================================================================

export const ChapterListCommand = cmd({
  command: "list <documentID>",
  describe: "list all chapters in a document",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const doc = await Document.get(args.documentID as string)
      if (!doc) {
        UI.error("Document not found")
        process.exit(1)
      }

      const chapters = await Document.Chapter.list(args.documentID as string)

      console.log()
      console.log(`Document: ${doc.title}`)
      console.log(`Chapters: ${chapters.length}`)
      console.log()

      for (let i = 0; i < chapters.length; i++) {
        const chapter = chapters[i]!
        const statusIcon = {
          pending: "○",
          outlining: "◐",
          drafting: "◑",
          reviewing: "◕",
          completed: "●",
        }[chapter.status]
        console.log(`${statusIcon} ${i + 1}. ${chapter.title}`)
        console.log(`   ID: ${chapter.id}`)
        console.log(`   Status: ${chapter.status}`)
        console.log(`   Words: ${chapter.wordCount}`)
        if (chapter.summary) {
          const preview = chapter.summary.slice(0, 100)
          console.log(`   Summary: ${preview}${chapter.summary.length > 100 ? "..." : ""}`)
        }
        console.log()
      }
    })
  },
})

// ============================================================================
// Show Command
// ============================================================================

export const ChapterShowCommand = cmd({
  command: "show <documentID> <chapterID>",
  describe: "show chapter content and details",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .positional("chapterID", {
        type: "string",
        describe: "chapter ID",
      })
      .option("output", {
        type: "string",
        alias: "o",
        describe: "save content to file",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const chapter = await Document.Chapter.get(args.documentID as string, args.chapterID as string)
      if (!chapter) {
        UI.error("Chapter not found")
        process.exit(1)
      }

      const doc = await Document.get(args.documentID as string)

      console.log()
      console.log(`Chapter: ${chapter.title}`)
      console.log(`Document: ${doc?.title || "Unknown"}`)
      console.log(`Status: ${chapter.status}`)
      console.log(`Words: ${chapter.wordCount}`)
      console.log()

      if (args.output) {
        await Bun.write(args.output as string, chapter.content)
        console.log(`Content saved to: ${args.output}`)
      } else {
        console.log("---")
        console.log()
        console.log(chapter.content)
        console.log()
        console.log("---")
      }

      if (chapter.summary) {
        console.log()
        console.log("Summary:")
        console.log(chapter.summary)
      }
    })
  },
})

// ============================================================================
// Reset Command
// ============================================================================

export const ChapterResetCommand = cmd({
  command: "reset <documentID> <chapterID>",
  describe: "reset chapter status to pending",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .positional("chapterID", {
        type: "string",
        describe: "chapter ID",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const chapter = await Document.Chapter.get(args.documentID as string, args.chapterID as string)
      if (!chapter) {
        UI.error("Chapter not found")
        process.exit(1)
      }

      await Document.Chapter.update({
        documentID: args.documentID as string,
        chapterID: args.chapterID as string,
        content: "",
        summary: "",
        status: "pending",
      })

      console.log(`Chapter "${chapter.title}" reset to pending`)
    })
  },
})

// ============================================================================
// Edit Command
// ============================================================================

export const ChapterEditCommand = cmd({
  command: "edit <documentID> <chapterID>",
  describe: "edit chapter content or summary",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .positional("chapterID", {
        type: "string",
        describe: "chapter ID",
      })
      .option("content", {
        type: "string",
        alias: "c",
        describe: "new content (or use --file)",
      })
      .option("file", {
        type: "string",
        alias: "f",
        describe: "file with new content",
      })
      .option("summary", {
        type: "string",
        alias: "s",
        describe: "new summary",
      })
      .option("status", {
        type: "string",
        alias: "t",
        describe: "new status (pending/drafting/reviewing/completed)",
        choices: ["pending", "drafting", "reviewing", "completed"],
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const chapter = await Document.Chapter.get(args.documentID as string, args.chapterID as string)
      if (!chapter) {
        UI.error("Chapter not found")
        process.exit(1)
      }

      let newContent = chapter.content
      let newSummary = chapter.summary
      const newStatus = args.status as DocumentSchema.ChapterStatus | undefined

      if (args.file) {
        const validatedPath = validatePath(args.file as string)
        newContent = await Bun.file(validatedPath).text()
      }

      if (args.content) {
        newContent = args.content as string
      }

      if (args.summary) {
        newSummary = args.summary as string
      }

      await Document.Chapter.update({
        documentID: args.documentID as string,
        chapterID: args.chapterID as string,
        content: newContent,
        summary: newSummary,
        status: newStatus,
      })

      console.log(`Chapter "${chapter.title}" updated`)
      if (newStatus) console.log(`  Status: ${newStatus}`)
    })
  },
})

// ============================================================================
// Stats Command
// ============================================================================

export const ChapterStatsCommand = cmd({
  command: "stats <documentID>",
  describe: "show document statistics",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("json", {
        type: "boolean",
        describe: "output as JSON",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const stats = await Document.getStats(args.documentID as string)

      if (args.json) {
        console.log(JSON.stringify(stats, null, 2))
        return
      }

      console.log()
      console.log(`Document Statistics:`)
      console.log(`  Total Chapters: ${stats.totalChapters}`)
      console.log(`  Completed: ${stats.completedChapters}`)
      console.log(`  Pending: ${stats.pendingChapters}`)
      console.log()
      console.log(`  Words: ${stats.totalWords}/${stats.targetWords} (${stats.progress}%)`)
      console.log(`  Estimated Remaining: ${stats.estimatedRemaining} words`)
      console.log()

      const barWidth = 40
      const filled = Math.round((stats.progress / 100) * barWidth)
      const empty = barWidth - filled
      const bar = "█".repeat(filled) + "░".repeat(empty)
      console.log(`  [${bar}] ${stats.progress}%`)
      console.log()
    })
  },
})

// ============================================================================
// Main Chapter Command (groups all subcommands)
// ============================================================================

export const ChapterCommand = cmd({
  command: "chapter",
  describe: "manage document chapters",
  builder: (yargs) =>
    yargs
      .command(ChapterListCommand)
      .command(ChapterShowCommand)
      .command(ChapterEditCommand)
      .command(ChapterResetCommand)
      .command(ChapterStatsCommand)
      .demandCommand(),
  async handler() {},
})
