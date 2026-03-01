/**
 * Write Commands
 *
 * Commands for writing document chapters.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document } from "../../../document"
import { Writer } from "../../../document/writer"
import { escapeShellArg } from "../../../util/security"
import { $ } from "bun"

// ============================================================================
// Document Write Command
// ============================================================================

export const DocumentWriteCommand = cmd({
  command: "write <documentID>",
  describe: "write document chapter by chapter",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        describe: "specific chapter ID to write",
      })
      .option("prompt", {
        type: "boolean",
        describe: "output the prepared prompt instead of running",
        default: false,
      })
      .option("auto", {
        type: "boolean",
        describe: "automatically run writer agent with prepared context",
        default: false,
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

      if (chapters.length === 0) {
        UI.error("No chapters found. Create an outline first.")
        process.exit(1)
      }

      const pendingChapter = args.chapter
        ? chapters.find((c) => c.id === args.chapter)
        : chapters.find((c) => c.status === "pending" || c.status === "drafting")

      if (!pendingChapter) {
        const completed = chapters.filter((c) => c.status === "completed").length
        if (completed === chapters.length) {
          console.log(`All ${chapters.length} chapters are completed!`)
          console.log()
          console.log(`Use "codecoder document export ${args.documentID}" to export the document.`)
        } else {
          UI.error("No pending chapter found")
        }
        process.exit(1)
      }

      prompts.intro(`Writing: ${pendingChapter.title}`)

      const completed = chapters.filter((c) => c.status === "completed").length
      const total = chapters.length
      console.log(`Progress: ${completed}/${total} chapters completed`)
      console.log(`Current: ${pendingChapter.title}`)
      console.log()

      const outline = doc.outline.chapters.find((c) => c.id === pendingChapter.outlineID)
      if (outline) {
        console.log(`Chapter description: ${outline.description}`)
        console.log(`Estimated words: ${outline.estimatedWords}`)
      }
      console.log()

      const writerPrompt = await Writer.generatePrompt({
        documentID: args.documentID as string,
        chapterID: pendingChapter.id,
      })

      if (args.prompt) {
        console.log("---")
        console.log("PREPARED PROMPT:")
        console.log("---")
        console.log()
        console.log(writerPrompt)
        console.log()
        console.log("---")
        console.log()
        console.log("Use this prompt with: codecoder run --agent writer")
        return
      }

      if (args.auto) {
        const spinner = prompts.spinner()
        spinner.start("Running writer agent...")

        try {
          // Use temp file to avoid command injection
          const tempFile = `/tmp/prompt_${Date.now()}.txt`
          await Bun.write(tempFile, writerPrompt)
          const result = await $`bun run dev run --agent writer "$(cat ${escapeShellArg(tempFile)})"`.quiet()
            .text()
          await Bun.file(tempFile).delete()

          spinner.stop("Writer agent finished")

          console.log()
          console.log(result)
          console.log()

          const content = Writer.extractContentFromResponse(result)
          const summary = Writer.parseSummaryFromResponse(result)

          await Document.Chapter.update({
            documentID: args.documentID as string,
            chapterID: pendingChapter.id,
            content,
            summary,
            status: "completed",
          })

          console.log(`Chapter updated: ${pendingChapter.title}`)
          console.log(`Words: ${content.length}`)

          // Check for truncation
          const outline = doc.outline.chapters.find((c) => c.id === pendingChapter.outlineID)
          if (outline) {
            const truncationWarning = Writer.detectTruncation(content, outline.estimatedWords)
            if (truncationWarning) {
              console.log()
              console.log(truncationWarning)
            }
          }

          const nextChapter = chapters.find((c) =>
            c.status === "pending" && c.id !== pendingChapter.id,
          )
          if (nextChapter) {
            console.log()
            console.log(`Next: "codecoder document write ${args.documentID}"`)
          } else {
            console.log()
            console.log(`All chapters completed! Use "codecoder document export ${args.documentID}" to export.`)
          }
        } catch (error) {
          spinner.stop(`Error: ${error}`)
          throw error
        }
      } else {
        console.log("To write this chapter, run:")
        console.log("  codecoder run --agent writer")
        console.log()
        console.log("Or use --auto to automatically run:")
        console.log(`  codecoder document write ${args.documentID} --auto`)
        console.log()
        console.log("Or use --prompt to see the prepared prompt:")
        console.log(`  codecoder document write ${args.documentID} --prompt`)
      }
    })
  },
})

// ============================================================================
// Document Write All Command
// ============================================================================

export const DocumentWriteAllCommand = cmd({
  command: "write-all <documentID>",
  describe: "write all pending chapters automatically",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("count", {
        type: "number",
        alias: "n",
        describe: "number of chapters to write",
        default: 1,
      })
      .option("continue", {
        type: "boolean",
        alias: "c",
        describe: "continue writing until all chapters complete",
        default: false,
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

      const pendingChapters = chapters.filter((c) => c.status === "pending" || c.status === "drafting")

      if (pendingChapters.length === 0) {
        const completed = chapters.filter((c) => c.status === "completed").length
        if (completed === chapters.length) {
          console.log(`All ${chapters.length} chapters are already completed!`)
          console.log()
          console.log(`Use "codecoder document export ${args.documentID}" to export the document.`)
        } else {
          console.log("No pending chapters found.")
        }
        process.exit(1)
      }

      prompts.intro(`Writing ${pendingChapters.length} chapters`)

      const countToWrite = args.continue ? pendingChapters.length : Math.min(args.count as number, pendingChapters.length)
      const total = chapters.length
      const alreadyCompleted = chapters.filter((c) => c.status === "completed").length

      console.log(`Plan: Write ${countToWrite} chapter(s)`)
      console.log(`Total: ${total} chapters | Completed: ${alreadyCompleted} | Pending: ${pendingChapters.length}`)
      console.log()

      let writtenCount = 0
      let failedCount = 0

      for (let i = 0; i < countToWrite; i++) {
        const currentChapters = await Document.Chapter.list(args.documentID as string)
        const pendingChapter = currentChapters.find((c) => c.status === "pending" || c.status === "drafting")

        if (!pendingChapter) break

        const currentCompleted = currentChapters.filter((c) => c.status === "completed").length

        console.log()
        console.log(`[${i + 1}/${countToWrite}] Writing: ${pendingChapter.title}`)
        console.log(`Progress: ${currentCompleted}/${total} chapters completed`)
        console.log()

        try {
          const writerPrompt = await Writer.generatePrompt({
            documentID: args.documentID as string,
            chapterID: pendingChapter.id,
          })

          const spinner = prompts.spinner()
          spinner.start("Running writer agent...")

          // Use temp file to avoid command injection
          const tempFile = `/tmp/prompt_${Date.now()}.txt`
          await Bun.write(tempFile, writerPrompt)
          const result = await $`bun run dev run --agent writer "$(cat ${escapeShellArg(tempFile)})"`.quiet()
            .text()
          await Bun.file(tempFile).delete()

          spinner.stop("Writer agent finished")

          const content = Writer.extractContentFromResponse(result)
          const summary = Writer.parseSummaryFromResponse(result)

          await Document.Chapter.update({
            documentID: args.documentID as string,
            chapterID: pendingChapter.id,
            content,
            summary,
            status: "completed",
          })

          writtenCount++
          console.log(`Chapter "${pendingChapter.title}" completed (${content.length} chars)`)

          // Check for truncation
          const outline = doc.outline.chapters.find((c) => c.id === pendingChapter.outlineID)
          if (outline) {
            const truncationWarning = Writer.detectTruncation(content, outline.estimatedWords)
            if (truncationWarning) {
              console.log(`  ${truncationWarning}`)
            }
          }
        } catch (error) {
          failedCount++
          console.log(`Chapter "${pendingChapter.title}" failed: ${error}`)
        }
      }

      console.log()
      console.log("---")
      console.log(`Summary: ${writtenCount} written, ${failedCount} failed`)

      const finalChapters = await Document.Chapter.list(args.documentID as string)
      const finalCompleted = finalChapters.filter((c) => c.status === "completed").length
      const finalPending = finalChapters.filter((c) => c.status === "pending" || c.status === "drafting").length

      console.log(`Progress: ${finalCompleted}/${total} chapters completed`)

      if (finalPending > 0) {
        console.log()
        console.log(`Remaining: ${finalPending} chapters`)
        console.log(`Continue with: "codecoder document write-all ${args.documentID}"`)
      } else {
        console.log()
        console.log(`All chapters completed! Use "codecoder document export ${args.documentID}" to export.`)
      }
    })
  },
})
