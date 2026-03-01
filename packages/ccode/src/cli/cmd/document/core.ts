/**
 * Core Document Commands
 *
 * CRUD operations for documents: create, list, show, export, delete, update, set-content.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document } from "../../../document"
import { validatePath } from "../../../util/security"
import { z } from "zod"

// Schema for validating outline JSON
const OutlineSchema = z.object({
  title: z.string(),
  description: z.string().optional(),
  chapters: z.array(z.object({
    id: z.string(),
    title: z.string(),
    description: z.string(),
    estimatedWords: z.number().int().positive(),
    subsections: z.array(z.string()).optional(),
  })),
})

// ============================================================================
// Document Create Command
// ============================================================================

export const DocumentCreateCommand = cmd({
  command: "create",
  describe: "create a new long-form document",
  builder: (yargs: Argv) => {
    return yargs
      .option("title", {
        type: "string",
        describe: "document title",
      })
      .option("description", {
        type: "string",
        describe: "document description",
      })
      .option("words", {
        type: "number",
        describe: "target word count",
        default: 100000,
      })
      .option("tone", {
        type: "string",
        describe: "writing tone (e.g., formal, casual, academic)",
      })
      .option("audience", {
        type: "string",
        describe: "target audience",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      prompts.intro("Create Long Document")

      let title: string
      if (args.title) {
        title = args.title as string
      } else {
        const result = await prompts.text({
          message: "Document title",
          placeholder: "e.g., The Future of AI",
          validate: (x) => (x && x.length > 0 ? undefined : "Required"),
        })
        if (prompts.isCancel(result)) throw new UI.CancelledError()
        title = result
      }

      let description: string | undefined
      if (args.description) {
        description = args.description as string
      } else {
        const result = await prompts.text({
          message: "Description (optional)",
          placeholder: "What is this document about?",
        })
        if (prompts.isCancel(result)) throw new UI.CancelledError()
        description = result || undefined
      }

      let targetWords: number
      if (args.words) {
        targetWords = args.words as number
      } else {
        const result = await prompts.text({
          message: "Target word count",
          placeholder: "100000",
          validate: (x) => {
            const num = parseInt(x as string)
            return num > 0 ? undefined : "Must be positive"
          },
        })
        if (prompts.isCancel(result)) throw new UI.CancelledError()
        targetWords = parseInt(result as string) || 100000
      }

      let tone: string | undefined
      if (args.tone) {
        tone = args.tone as string
      } else {
        const result = await prompts.text({
          message: "Writing tone (optional)",
          placeholder: "e.g., formal, casual, academic",
        })
        if (prompts.isCancel(result)) throw new UI.CancelledError()
        tone = result || undefined
      }

      let audience: string | undefined
      if (args.audience) {
        audience = args.audience as string
      } else {
        const result = await prompts.text({
          message: "Target audience (optional)",
          placeholder: "e.g., general public, developers, researchers",
        })
        if (prompts.isCancel(result)) throw new UI.CancelledError()
        audience = result || undefined
      }

      const spinner = prompts.spinner()
      spinner.start("Creating document...")
      const doc = await Document.create({
        title,
        description,
        targetWords,
        styleGuide: {
          tone,
          audience,
          format: "markdown",
        },
      })
      spinner.stop(`Document created: ${doc.id}`)

      prompts.outro(`Next: Use "codecoder document template list" to see templates or "codecoder document update ${doc.id} --file outline.json" to create outline`)
    })
  },
})

// ============================================================================
// Document List Command
// ============================================================================

export const DocumentListCommand = cmd({
  command: "list",
  describe: "list all documents",
  handler: async () => {
    await bootstrap(process.cwd(), async () => {
      const docs = await Document.list()

      if (docs.length === 0) {
        console.log("No documents found.")
        return
      }

      console.log()
      for (const doc of docs) {
        const progress = doc.targetWords > 0 ? Math.round((doc.currentWords / doc.targetWords) * 100) : 0
        console.log(`${doc.id}`)
        console.log(`  Title: ${doc.title}`)
        console.log(`  Status: ${doc.status}`)
        console.log(`  Progress: ${doc.currentWords}/${doc.targetWords} words (${progress}%)`)
        console.log()
      }
    })
  },
})

// ============================================================================
// Document Show Command
// ============================================================================

export const DocumentShowCommand = cmd({
  command: "show <documentID>",
  describe: "show document details and progress",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
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
      console.log(`Title: ${doc.title}`)
      if (doc.description) console.log(`Description: ${doc.description}`)
      console.log(`Status: ${doc.status}`)
      const progress = doc.targetWords > 0 ? Math.round((doc.currentWords / doc.targetWords) * 100) : 0
      console.log(`Progress: ${doc.currentWords}/${doc.targetWords} words (${progress}%)`)
      console.log(`Chapters: ${chapters.length}`)
      if (doc.styleGuide?.tone) console.log(`Tone: ${doc.styleGuide.tone}`)
      if (doc.styleGuide?.audience) console.log(`Audience: ${doc.styleGuide.audience}`)
      console.log()

      console.log("Chapters:")
      for (let i = 0; i < chapters.length; i++) {
        const chapter = chapters[i]!
        const statusIcon = {
          pending: "○",
          outlining: "◐",
          drafting: "◑",
          reviewing: "◕",
          completed: "●",
        }[chapter.status]
        console.log(`  ${statusIcon} ${i + 1}. ${chapter.title} (${chapter.wordCount} words)`)
        const outline = doc.outline.chapters.find((c) => c.id === chapter.outlineID)
        if (outline?.estimatedWords) {
          const chapterProgress = outline.estimatedWords > 0
            ? Math.round((chapter.wordCount / outline.estimatedWords) * 100)
            : 0
          console.log(`     Est: ${outline.estimatedWords} words (${chapterProgress}%)`)
        }
      }
      console.log()
    })
  },
})

// ============================================================================
// Document Export Command
// ============================================================================

export const DocumentExportCommand = cmd({
  command: "export <documentID>",
  describe: "export document to file",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("output", {
        type: "string",
        alias: "o",
        describe: "output file path",
      })
      .option("format", {
        type: "string",
        choices: ["markdown", "html"],
        default: "markdown",
        describe: "export format",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const doc = await Document.get(args.documentID as string)
      if (!doc) {
        UI.error("Document not found")
        process.exit(1)
      }

      const spinner = prompts.spinner()
      spinner.start("Exporting document...")

      const content = await Document.exportDocument({
        documentID: args.documentID as string,
        format: args.format as "markdown" | "html",
      })

      const outputPath = validatePath(
        args.output ?? `${doc.title.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, "_")}.md`
      )
      await Bun.write(outputPath, content)

      spinner.stop(`Exported to: ${outputPath}`)
    })
  },
})

// ============================================================================
// Document Delete Command
// ============================================================================

export const DocumentDeleteCommand = cmd({
  command: "delete <documentID>",
  describe: "delete a document and all its chapters",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("confirm", {
        type: "boolean",
        alias: "y",
        describe: "skip confirmation prompt",
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

      if (!args.confirm) {
        const confirmed = await prompts.confirm({
          message: `Delete document "${doc.title}" and all its chapters?`,
        })
        if (prompts.isCancel(confirmed)) throw new UI.CancelledError()
        if (!confirmed) {
          console.log("Cancelled")
          return
        }
      }

      const spinner = prompts.spinner()
      spinner.start("Deleting document...")

      const chapters = await Document.Chapter.list(args.documentID as string)
      for (const chapter of chapters) {
        await Document.Chapter.remove(args.documentID as string, chapter.id)
      }
      await Document.remove(args.documentID as string)

      spinner.stop(`Document "${doc.title}" deleted`)
    })
  },
})

// ============================================================================
// Document Update Command
// ============================================================================

export const DocumentUpdateCommand = cmd({
  command: "update <documentID>",
  describe: "update document outline from JSON",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("file", {
        type: "string",
        alias: "f",
        describe: "JSON file containing outline",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const doc = await Document.get(args.documentID as string)
      if (!doc) {
        UI.error("Document not found")
        process.exit(1)
      }

      if (!args.file) {
        UI.error("Please provide --file with outline JSON")
        process.exit(1)
      }

      const spinner = prompts.spinner()
      spinner.start("Loading outline...")

      const validatedPath = validatePath(args.file as string)
      const content = await Bun.file(validatedPath).text()
      const outline = OutlineSchema.parse(JSON.parse(content))

      spinner.stop("Creating chapters...")

      await Document.updateOutline({
        documentID: args.documentID as string,
        outline,
      })

      const chapters = await Document.Chapter.list(args.documentID as string)
      prompts.outro(`Created ${chapters.length} chapters. Use "codecoder document write ${args.documentID}" to start writing.`)
    })
  },
})

// ============================================================================
// Document Set Content Command
// ============================================================================

export const DocumentSetContentCommand = cmd({
  command: "set-content <documentID>",
  describe: "manually set chapter content",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "chapter ID",
      })
      .option("file", {
        type: "string",
        alias: "f",
        describe: "file containing chapter content (markdown)",
      })
      .option("summary", {
        type: "string",
        alias: "s",
        describe: "chapter summary (optional)",
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

      const targetChapter = args.chapter
        ? chapters.find((c) => c.id === args.chapter)
        : chapters.find((c) => c.status === "pending" || c.status === "drafting")

      if (!targetChapter && args.chapter) {
        UI.error("Chapter not found")
        process.exit(1)
      }
      if (!targetChapter) {
        UI.error("No pending chapter found")
        process.exit(1)
      }

      let content: string
      if (args.file) {
        const validatedPath = validatePath(args.file as string)
        content = await Bun.file(validatedPath).text()
      } else {
        const result = await prompts.text({
          message: "Paste or type chapter content (Ctrl+D to finish):",
          placeholder: "Enter content...",
        })
        if (prompts.isCancel(result)) throw new UI.CancelledError()
        content = result as string
      }

      const spinner = prompts.spinner()
      spinner.start("Saving chapter...")

      await Document.Chapter.update({
        documentID: args.documentID as string,
        chapterID: targetChapter.id,
        content,
        summary: args.summary as string | undefined,
        status: "completed",
      })

      spinner.stop(`Chapter "${targetChapter.title}" saved (${content.length} chars)`)

      const remaining = chapters.filter((c) => c.status !== "completed").length
      if (remaining > 0) {
        console.log()
        console.log(`${remaining} chapters remaining. Use "codecoder document write ${args.documentID}" to continue.`)
      } else {
        console.log()
        console.log(`All chapters completed! Use "codecoder document export ${args.documentID}" to export.`)
      }
    })
  },
})
