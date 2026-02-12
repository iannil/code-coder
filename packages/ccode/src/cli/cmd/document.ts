import type { Argv } from "yargs"
import { cmd } from "./cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../ui"
import { bootstrap } from "../bootstrap"
import { Document } from "../../document"
import { DocumentSchema } from "../../document/schema"
import { Writer } from "../../document/writer"
import * as Templates from "../../document/templates"
import { EOL } from "os"
import { $ } from "bun"
import { Context, Entity, Summary, Volume, Version, Editor, Consistency, Proofreader } from "../../document"
import { validatePath, escapeShellArg } from "../../util/security"
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
// Document Commands
// ============================================================================

const DocumentCreateCommand = cmd({
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
// Template Commands
// ============================================================================

const TemplateListCommand = cmd({
  command: "list",
  describe: "list available templates",
  handler: async () => {
    const templates = Templates.listTemplates()
    console.log()
    console.log("Available Templates:")
    console.log()
    for (const t of templates) {
      console.log(`  ${t.id} - ${t.name}`)
      console.log(`    Category: ${t.category}`)
      console.log(`    ${t.description}`)
      const chapterCount = t.defaultOutline.chapters.length
      const totalWords = t.defaultOutline.chapters.reduce((sum, ch) => sum + ch.estimatedWords, 0)
      console.log(`    Chapters: ${chapterCount} | Est. ${totalWords} words`)
      console.log()
    }
    console.log("Usage: codecoder document template use <templateID>")
  },
})

const TemplateUseCommand = cmd({
  command: "use <templateID>",
  describe: "create document from template",
  builder: (yargs: Argv) => {
    return yargs
      .positional("templateID", {
        type: "string",
        describe: "template ID",
        choices: Object.keys(Templates.TEMPLATES),
      })
      .option("title", {
        type: "string",
        describe: "document title",
      })
      .option("words", {
        type: "number",
        describe: "target word count (adjusts chapter estimates)",
        default: 100000,
      })
      .option("chapters", {
        type: "number",
        describe: "number of chapters (splits template evenly)",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const template = Templates.getTemplate(args.templateID as string)
      if (!template) {
        UI.error("Template not found")
        process.exit(1)
      }

      prompts.intro(`Creating from template: ${template.name}`)

      const result = Templates.applyTemplate(args.templateID as string, {
        title: args.title as string,
        targetWords: args.words as number,
        chapterCount: args.chapters as number | undefined,
      })

      const spinner = prompts.spinner()
      spinner.start("Creating document...")

      const doc = await Document.create({
        title: result.outline.title,
        description: result.outline.description,
        targetWords: args.words as number,
        styleGuide: result.styleGuide,
      })

      spinner.stop(`Document created: ${doc.id}`)

      console.log()
      console.log(`Created ${result.outline.chapters.length} chapters from template`)
      console.log()

      await Document.updateOutline({
        documentID: doc.id,
        outline: result.outline,
      })

      prompts.outro(`Use "codecoder document write ${doc.id}" to start writing`)
    })
  },
})

const DocumentTemplateCommand = cmd({
  command: "template",
  describe: "manage document templates",
  builder: (yargs) =>
    yargs.command(TemplateListCommand).command(TemplateUseCommand).demandCommand(),
  async handler() {},
})

// ============================================================================
// Outline Commands
// ============================================================================

const DocumentOutlineCommand = cmd({
  command: "outline <documentID>",
  describe: "generate document outline using writer agent",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapters", {
        type: "number",
        describe: "number of chapters to generate",
        default: 10,
      })
      .option("auto", {
        type: "boolean",
        describe: "automatically run writer agent to generate outline",
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

      prompts.intro(`Generating Outline for: ${doc.title}`)

      const numChapters = args.chapters as number

      const outlinePrompt = `Create a detailed outline for a document titled "${doc.title}".
${doc.description ? `Description: ${doc.description}` : ""}
Target: ${doc.targetWords} words
Generate ${numChapters} chapters with word count estimates.
${doc.styleGuide?.tone ? `Tone: ${doc.styleGuide.tone}` : ""}
${doc.styleGuide?.audience ? `Audience: ${doc.styleGuide.audience}` : ""}

Return the outline in the following JSON format:
{
  "title": "${doc.title}",
  "description": "${doc.description || ""}",
  "chapters": [
    {
      "id": "ch1",
      "title": "Chapter Title",
      "description": "Brief description",
      "estimatedWords": 5000,
      "subsections": ["Section 1", "Section 2"]
    }
  ]
}`

      if (args.auto) {
        const spinner = prompts.spinner()
        spinner.start("Running writer agent...")

        // Use temp file to avoid command injection
        const tempFile = `/tmp/prompt_${Date.now()}.txt`
        await Bun.write(tempFile, outlinePrompt)
        const result = await $`bun run dev run --agent writer "$(cat ${escapeShellArg(tempFile)})"`.quiet().text()
        await Bun.file(tempFile).delete()

        spinner.stop("Outline generated")

        console.log()
        console.log(result)
        console.log()
        console.log("Copy the JSON output and save it to a file, then run:")
        console.log(`  codecoder document update ${args.documentID} --file outline.json`)
      } else {
        console.log()
        console.log("To generate an outline, run:")
        console.log("  codecoder run --agent writer")
        console.log()
        console.log("Then use this prompt:")
        console.log("```")
        console.log(outlinePrompt)
        console.log("```")
        console.log()
        console.log("After getting the outline, save it to outline.json and run:")
        console.log(`  codecoder document update ${args.documentID} --file outline.json`)
      }
    })
  },
})

// ============================================================================
// Write Commands
// ============================================================================

const DocumentWriteCommand = cmd({
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

const DocumentWriteAllCommand = cmd({
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
          console.log(`✓ Chapter "${pendingChapter.title}" completed (${content.length} chars)`)

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
          console.log(`✗ Chapter "${pendingChapter.title}" failed: ${error}`)
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

// ============================================================================
// List/Show/Export Commands
// ============================================================================

const DocumentListCommand = cmd({
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

const DocumentShowCommand = cmd({
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
        const chapter = chapters[i]
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

const DocumentExportCommand = cmd({
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
// Update/Delete/Set Content Commands
// ============================================================================

const DocumentSetContentCommand = cmd({
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

      let targetChapter = args.chapter
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

const DocumentDeleteCommand = cmd({
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

const DocumentUpdateCommand = cmd({
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
// Chapter Commands
// ============================================================================

const ChapterListCommand = cmd({
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
        const chapter = chapters[i]
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

const ChapterShowCommand = cmd({
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

const ChapterResetCommand = cmd({
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

const ChapterEditCommand = cmd({
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
      let newStatus = args.status as DocumentSchema.ChapterStatus | undefined

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

const ChapterStatsCommand = cmd({
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
// Context Commands
// ============================================================================

const ContextCommand = cmd({
  command: "context <documentID>",
  describe: "show intelligent context for a chapter",
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
      .option("show", {
        type: "boolean",
        describe: "show full context preview",
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
      let targetChapter = args.chapter
        ? chapters.find((c) => c.id === args.chapter)
        : chapters.find((c) => c.status === "pending" || c.status === "drafting") || chapters[0]

      if (!targetChapter) {
        UI.error("No chapter found")
        process.exit(1)
      }

      if (args.show) {
        const context = await Context.selectContextForChapter({
          documentID: args.documentID as string,
          chapterID: targetChapter.id,
        })

        console.log()
        console.log("--- CONTEXT PREVIEW ---")
        console.log()
        console.log(Context.formatContextForPrompt(context, doc.title))
        console.log()
      }

      const stats = await Context.getContextStats({
        documentID: args.documentID as string,
        chapterID: targetChapter.id,
      })

      console.log()
      console.log("Context Budget:")
      console.log(`  Total: ${stats.budget.totalTokens} tokens`)
      console.log(`  System: ${stats.budget.systemPromptTokens}`)
      console.log(`  Global Summary: ${stats.estimatedTokens.globalSummary}`)
      console.log(`  Entities: ${stats.estimatedTokens.entities}`)
      console.log(`  Volumes: ${stats.estimatedTokens.volumes}`)
      console.log(`  Chapter Summaries: ${stats.estimatedTokens.chapters}`)
      console.log(`  Recent Content: ${stats.estimatedTokens.recentContent}`)
      console.log(`  Used: ${Object.values(stats.estimatedTokens).reduce((a, b) => a + b, 0)}/${stats.budget.totalTokens} tokens`)
      console.log()
    })
  },
})

const SummaryGlobalCommand = cmd({
  command: "summary-global <documentID>",
  describe: "generate or update global summary",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("regenerate", {
        type: "boolean",
        alias: "r",
        describe: "regenerate from scratch",
        default: false,
      })
      .option("auto", {
        type: "boolean",
        describe: "automatically run AI to generate",
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

      const spinner = prompts.spinner()
      spinner.start("Generating global summary prompt...")

      const prompt = args.regenerate
        ? await Summary.generateGlobalSummaryPrompt(args.documentID as string)
        : await Summary.updateGlobalSummaryPrompt(args.documentID as string, [])

      spinner.stop("Prompt ready")

      if (args.auto) {
        spinner.start("Running AI...")
        try {
          // Use temp file to avoid command injection
          const tempFile = `/tmp/prompt_${Date.now()}.txt`
          await Bun.write(tempFile, prompt)
          const result = await $`bun run dev run --agent writer "$(cat ${escapeShellArg(tempFile)})"`.quiet().text()
          await Bun.file(tempFile).delete()

          spinner.stop("Processing response...")

          const summary = await Summary.saveGlobalSummary(args.documentID as string, result)

          console.log()
          console.log("Global Summary Generated:")
          console.log(`  Plot: ${summary.overallPlot.slice(0, 100)}...`)
          console.log(`  Themes: ${summary.mainThemes.join(", ")}`)
          console.log(`  Arcs: ${summary.keyArcs.length}`)
          console.log()
        } catch (error) {
          spinner.stop(`Error: ${error}`)
          throw error
        }
      } else {
        console.log()
        console.log("Use this prompt with the writer agent:")
        console.log("```")
        console.log(prompt.slice(0, 2000))
        if (prompt.length > 2000) console.log("...")
        console.log("```")
        console.log()
        console.log("Or use --auto to automatically generate")
      }
    })
  },
})

// ============================================================================
// Entity Commands
// ============================================================================

const EntityExtractCommand = cmd({
  command: "extract <documentID>",
  describe: "extract entities from chapter content",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "chapter ID (default: first pending)",
      })
      .option("auto", {
        type: "boolean",
        describe: "automatically run AI to extract",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const chapters = await Document.Chapter.list(args.documentID as string)
      let targetChapter = args.chapter
        ? chapters.find((c) => c.id === args.chapter)
        : chapters.find((c) => c.status === "completed") || chapters[0]

      if (!targetChapter) {
        UI.error("No chapter found")
        process.exit(1)
      }

      const spinner = prompts.spinner()
      spinner.start("Generating entity extraction prompt...")

      const prompt = await Entity.extractEntitiesPrompt(args.documentID as string, targetChapter.id)

      spinner.stop("Prompt ready")

      if (args.auto) {
        spinner.start("Running AI...")
        try {
          // Use temp file to avoid command injection
          const tempFile = `/tmp/prompt_${Date.now()}.txt`
          await Bun.write(tempFile, prompt)
          const result = await $`bun run dev run --agent writer "$(cat ${escapeShellArg(tempFile)})"`.quiet().text()
          await Bun.file(tempFile).delete()

          spinner.stop("Processing entities...")

          const processed = await Entity.processExtractionResponse(
            args.documentID as string,
            targetChapter.id,
            result,
          )

          console.log()
          console.log(`Entities processed: ${processed.created} created, ${processed.updated} updated`)
          console.log(`  Mentioned: ${processed.mentioned.length} entities`)
          console.log()
        } catch (error) {
          spinner.stop(`Error: ${error}`)
          throw error
        }
      } else {
        console.log()
        console.log("Use this prompt with the writer agent:")
        console.log("```")
        console.log(prompt.slice(0, 2000))
        if (prompt.length > 2000) console.log("...")
        console.log("```")
        console.log()
        console.log("Or use --auto to automatically extract")
      }
    })
  },
})

const EntityListCommand = cmd({
  command: "list <documentID>",
  describe: "list all entities in document",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("type", {
        type: "string",
        choices: ["character", "location", "concept", "item", "event"],
        describe: "filter by entity type",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const entities = args.type
        ? await Entity.listByType(args.documentID as string, args.type as DocumentSchema.EntityType)
        : await Entity.list(args.documentID as string)

      if (entities.length === 0) {
        console.log("No entities found.")
        return
      }

      console.log()
      console.log(`Entities (${entities.length}):`)
      console.log()

      for (const entity of entities) {
        const emojiMap: Record<DocumentSchema.EntityType, string> = {
          character: "👤",
          location: "📍",
          concept: "💡",
          item: "🔮",
          event: "📅",
        }
        const emoji = emojiMap[entity.type]

        console.log(`${emoji} ${entity.name} (${entity.type})`)
        console.log(`   ID: ${entity.id}`)
        console.log(`   ${entity.description}`)
        if (entity.aliases.length > 0) {
          console.log(`   Aliases: ${entity.aliases.join(", ")}`)
        }
        if (entity.relationships.length > 0) {
          console.log(`   Relationships: ${entity.relationships.length}`)
        }
        console.log()
      }
    })
  },
})

const EntityShowCommand = cmd({
  command: "show <documentID> <entityID>",
  describe: "show entity details",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .positional("entityID", {
        type: "string",
        describe: "entity ID",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const entity = await Entity.get(args.documentID as string, args.entityID as string)
      if (!entity) {
        UI.error("Entity not found")
        process.exit(1)
      }

      const emojiMap: Record<DocumentSchema.EntityType, string> = {
        character: "👤",
        location: "📍",
        concept: "💡",
        item: "🔮",
        event: "📅",
      }
      const emoji = emojiMap[entity.type]

      console.log()
      console.log(`${emoji} ${entity.name}`)
      console.log(`Type: ${entity.type}`)
      console.log(`ID: ${entity.id}`)
      console.log()
      console.log(`Description: ${entity.description}`)
      console.log()

      if (entity.aliases.length > 0) {
        console.log(`Aliases: ${entity.aliases.join(", ")}`)
        console.log()
      }

      if (Object.keys(entity.attributes).length > 0) {
        console.log("Attributes:")
        for (const [key, value] of Object.entries(entity.attributes)) {
          console.log(`  ${key}: ${value}`)
        }
        console.log()
      }

      if (entity.relationships.length > 0) {
        console.log("Relationships:")
        for (const rel of entity.relationships) {
          console.log(`  - ${rel.type}: ${rel.description}`)
        }
        console.log()
      }
    })
  },
})

const EntityDuplicatesCommand = cmd({
  command: "duplicates <documentID>",
  describe: "find potential duplicate entities",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const duplicates = await Entity.findDuplicates(args.documentID as string)

      if (duplicates.length === 0) {
        console.log("No duplicate entities found.")
        return
      }

      console.log()
      console.log(`Found ${duplicates.length} potential duplicates:`)
      console.log()

      for (const dup of duplicates) {
        console.log(`(${Math.round(dup.confidence * 100)}% confidence) ${dup.reason}`)
        console.log(`  Entities: ${dup.entities.map((e: { name: string }) => e.name).join(", ")}`)
        console.log(`  IDs: ${dup.entities.map((e: { id: string }) => e.id).join(", ")}`)
        console.log()
      }
    })
  },
})

const EntityCommand = cmd({
  command: "entity",
  describe: "manage document entities",
  builder: (yargs) =>
    yargs
      .command(EntityExtractCommand)
      .command(EntityListCommand)
      .command(EntityShowCommand)
      .command(EntityDuplicatesCommand)
      .demandCommand(),
  async handler() {},
})

// ============================================================================
// Consistency Commands
// ============================================================================

const CheckCommand = cmd({
  command: "check <documentID>",
  describe: "check document consistency",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "check specific chapter",
      })
      .option("fix", {
        type: "boolean",
        describe: "attempt to auto-fix issues",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const spinner = prompts.spinner()
      spinner.start("Generating consistency check prompt...")

      const prompt = await Consistency.checkPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
      })

      spinner.stop("Prompt ready")

      console.log()
      console.log("Use this prompt with the writer agent:")
      console.log("```")
      console.log(prompt.slice(0, 2000))
      if (prompt.length > 2000) console.log("...")
      console.log("```")
      console.log()
    })
  },
})

const CheckStyleCommand = cmd({
  command: "check-style <documentID>",
  describe: "check writing style consistency",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const result = await Consistency.checkStyleConsistency(args.documentID as string)

      console.log()
      console.log(`Style Consistency Score: ${Math.round(result.overallConsistency * 100)}%`)
      console.log()

      if (result.issues.length > 0) {
        console.log("Issues found:")
        for (const issue of result.issues) {
          console.log(`  - ${issue.issue}`)
          if (issue.chapterID !== "multiple") {
            console.log(`    Chapter: ${issue.chapterTitle}`)
          }
        }
        console.log()
      } else {
        console.log("No style issues detected.")
        console.log()
      }
    })
  },
})

// ============================================================================
// Editor Commands
// ============================================================================

const SearchReplaceCommand = cmd({
  command: "search-replace <documentID>",
  describe: "search and replace text across chapters",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("search", {
        type: "string",
        alias: "s",
        describe: "text to search for",
        demandOption: true,
      })
      .option("replace", {
        type: "string",
        alias: "r",
        describe: "replacement text",
        demandOption: true,
      })
      .option("scope", {
        type: "string",
        choices: ["global", "chapter", "volume"],
        describe: "search scope",
        default: "global",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "chapter ID (for chapter scope)",
      })
      .option("volume", {
        type: "string",
        alias: "v",
        describe: "volume ID (for volume scope)",
      })
      .option("preview", {
        type: "boolean",
        alias: "p",
        describe: "preview changes without applying",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const result = await Editor.searchAndReplace({
        documentID: args.documentID as string,
        search: args.search as string,
        replace: args.replace as string,
        scope: args.scope as "global" | "chapter" | "volume",
        chapterID: args.chapter as string,
        volumeID: args.volume as string,
        previewOnly: args.preview as boolean,
      })

      console.log()
      console.log(`Found ${result.totalReplacements} occurrences`)
      console.log()

      if (result.chaptersModified.length > 0) {
        console.log("Chapters affected:")
        for (const mod of result.chaptersModified) {
          console.log(`  ${mod.chapterTitle}: ${mod.replacementCount} replacements`)
        }
        console.log()

        if (args.preview) {
          console.log("(Preview mode - no changes applied)")
        } else {
          console.log("Changes applied successfully")
        }
      } else {
        console.log("No matches found")
      }
      console.log()
    })
  },
})

const PolishCommand = cmd({
  command: "polish <documentID>",
  describe: "generate AI prompt for polishing chapter",
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
        demandOption: true,
      })
      .option("aspect", {
        type: "string",
        choices: ["fluency", "clarity", "style", "tone", "all"],
        describe: "polishing aspect",
        default: "all",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const prompt = await Editor.polishPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
        aspect: args.aspect as "fluency" | "clarity" | "style" | "tone" | "all",
      })

      console.log()
      console.log("--- POLISH PROMPT ---")
      console.log()
      console.log(prompt)
      console.log()
    })
  },
})

const ExpandCommand = cmd({
  command: "expand <documentID>",
  describe: "generate AI prompt for expanding chapter",
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
        demandOption: true,
      })
      .option("target-words", {
        type: "number",
        alias: "t",
        describe: "target word count",
        demandOption: true,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const prompt = await Editor.expandPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
        targetWords: args.targetWords as number,
      })

      console.log()
      console.log("--- EXPAND PROMPT ---")
      console.log()
      console.log(prompt)
      console.log()
    })
  },
})

const CompressCommand = cmd({
  command: "compress <documentID>",
  describe: "generate AI prompt for compressing chapter",
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
        demandOption: true,
      })
      .option("target-words", {
        type: "number",
        alias: "t",
        describe: "target word count",
        demandOption: true,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const prompt = await Editor.compressPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
        targetWords: args.targetWords as number,
      })

      console.log()
      console.log("--- COMPRESS PROMPT ---")
      console.log()
      console.log(prompt)
      console.log()
    })
  },
})

// ============================================================================
// Version Commands
// ============================================================================

const SnapshotCreateCommand = cmd({
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

const SnapshotListCommand = cmd({
  command: "list <documentID>",
  describe: "list all snapshots",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
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

const SnapshotRollbackCommand = cmd({
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

const SnapshotDiffCommand = cmd({
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

const SnapshotCommand = cmd({
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

// ============================================================================
// Volume Commands
// ============================================================================

const VolumeCreateCommand = cmd({
  command: "create <documentID>",
  describe: "create a volume",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("title", {
        type: "string",
        alias: "t",
        describe: "volume title",
        demandOption: true,
      })
      .option("start", {
        type: "string",
        alias: "s",
        describe: "start chapter ID",
        demandOption: true,
      })
      .option("end", {
        type: "string",
        alias: "e",
        describe: "end chapter ID",
        demandOption: true,
      })
      .option("description", {
        type: "string",
        alias: "d",
        describe: "volume description",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const spinner = prompts.spinner()
      spinner.start("Creating volume...")

      const volume = await Volume.create({
        documentID: args.documentID as string,
        title: args.title as string,
        description: args.description as string,
        startChapterID: args.start as string,
        endChapterID: args.end as string,
      })

      spinner.stop(`Volume created: ${volume.id}`)
      console.log()
      console.log(`  Title: ${volume.title}`)
      console.log(`  Order: ${volume.order}`)
      console.log()
    })
  },
})

const VolumeListCommand = cmd({
  command: "list <documentID>",
  describe: "list all volumes",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const volumes = await Volume.list(args.documentID as string)

      if (volumes.length === 0) {
        console.log("No volumes found.")
        return
      }

      // Get progress for all volumes
      const progresses = await Volume.getAllVolumeProgress(args.documentID as string)

      console.log()
      console.log(`Volumes (${volumes.length}):`)
      console.log()

      for (const { volume, progress } of progresses) {
        console.log(`${volume.order + 1}. ${volume.title}`)
        console.log(`   ID: ${volume.id}`)
        if (volume.description) console.log(`   ${volume.description}`)
        console.log(`   Progress: ${progress.completedChapters}/${progress.totalChapters} chapters (${progress.completionPercentage}%)`)
        console.log(`   Words: ${progress.totalWords}`)
        console.log()
      }
    })
  },
})

const VolumeSummaryCommand = cmd({
  command: "summary <documentID>",
  describe: "generate volume summary",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("volume", {
        type: "string",
        alias: "v",
        describe: "volume ID",
        demandOption: true,
      })
      .option("regenerate", {
        type: "boolean",
        alias: "r",
        describe: "regenerate summary",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const prompt = await Volume.generateSummaryPrompt(
        args.documentID as string,
        args.volume as string,
      )

      console.log()
      console.log("--- VOLUME SUMMARY PROMPT ---")
      console.log()
      console.log(prompt)
      console.log()
    })
  },
})

const VolumeAutoCommand = cmd({
  command: "auto <documentID>",
  describe: "auto-create volumes",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapters", {
        type: "number",
        alias: "c",
        describe: "chapters per volume",
        default: 10,
      })
      .option("naming", {
        type: "string",
        choices: ["roman", "number", "custom"],
        describe: "naming pattern",
        default: "number",
      })
      .option("prefix", {
        type: "string",
        describe: "custom prefix",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const spinner = prompts.spinner()
      spinner.start("Creating volumes...")

      const volumes = await Volume.autoCreate({
        documentID: args.documentID as string,
        chaptersPerVolume: args.chapters as number,
        namingPattern: args.naming as "roman" | "number" | "custom",
        customPrefix: args.prefix as string,
      })

      spinner.stop(`Created ${volumes.length} volumes`)
      console.log()

      for (const volume of volumes) {
        console.log(`  ${volume.order + 1}. ${volume.title}`)
      }
      console.log()
    })
  },
})

const VolumeCommand = cmd({
  command: "volume",
  describe: "manage document volumes",
  builder: (yargs) =>
    yargs
      .command(VolumeCreateCommand)
      .command(VolumeListCommand)
      .command(VolumeSummaryCommand)
      .command(VolumeAutoCommand)
      .demandCommand(),
  async handler() {},
})

// ============================================================================
// Proofreader Commands
// ============================================================================

const ProofreadCheckCommand = cmd({
  command: "check <documentID>",
  describe: "proofread document for grammar, spelling, and style issues",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "check specific chapter",
      })
      .option("depth", {
        type: "string",
        alias: "d",
        choices: ["quick", "standard", "deep"],
        describe: "checking depth",
        default: "standard",
      })
      .option("auto", {
        type: "boolean",
        describe: "automatically run proofreader agent",
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

      prompts.intro(`Proofreading: ${doc.title}`)

      const spinner = prompts.spinner()
      spinner.start("Generating proofreading prompt...")

      const prompt = await Proofreader.checkPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
        depth: args.depth as "quick" | "standard" | "deep",
      })

      spinner.stop("Prompt ready")

      if (args.auto) {
        spinner.start("Running proofreader agent...")

        try {
          const tempFile = `/tmp/prompt_${Date.now()}.txt`
          await Bun.write(tempFile, prompt)
          const result = await $`bun run dev run --agent proofreader "$(cat ${escapeShellArg(tempFile)})"`.quiet()
            .text()
          await Bun.file(tempFile).delete()

          spinner.stop("Processing results...")

          const scope = args.chapter ? "chapter" : "document"
          const report = await Proofreader.saveReport(
            args.documentID as string,
            result,
            scope,
            args.chapter as string,
          )

          console.log()
          console.log(`Report generated: ${report.id}`)
          console.log(`Issues found: ${report.issues.length}`)
          console.log(`  Critical: ${report.summary.bySeverity.critical ?? 0}`)
          console.log(`  High: ${report.summary.bySeverity.high ?? 0}`)
          console.log(`  Medium: ${report.summary.bySeverity.medium ?? 0}`)
          console.log(`  Low: ${report.summary.bySeverity.low ?? 0}`)
          console.log()

          if (report.readabilityScore !== undefined) {
            console.log(`Readability Score: ${report.readabilityScore}/100`)
            console.log()
          }

          console.log(`View report: codecoder document proofread show ${args.documentID} ${report.id}`)
        } catch (error) {
          spinner.stop(`Error: ${error}`)
          throw error
        }
      } else {
        console.log()
        console.log("Use this prompt with the proofreader agent:")
        console.log("```")
        console.log(prompt.slice(0, 2000))
        if (prompt.length > 2000) console.log("...")
        console.log("```")
        console.log()
        console.log("Or use --auto to automatically run the proofreader agent")
      }
    })
  },
})

const ProofreadReportsCommand = cmd({
  command: "reports <documentID>",
  describe: "list all proofreading reports",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const reports = await Proofreader.listReports(args.documentID as string)

      if (reports.length === 0) {
        console.log("No proofreading reports found.")
        return
      }

      console.log()
      console.log(`Proofreading Reports (${reports.length}):`)
      console.log()

      for (const report of reports) {
        const date = new Date(report.timestamp).toLocaleString()
        const critical = report.summary.bySeverity.critical ?? 0
        const high = report.summary.bySeverity.high ?? 0
        const medium = report.summary.bySeverity.medium ?? 0
        const low = report.summary.bySeverity.low ?? 0

        console.log(`${report.id}`)
        console.log(`  ${date}`)
        console.log(`  Scope: ${report.scope}`)
        if (report.chapterID) console.log(`  Chapter: ${report.chapterID}`)
        console.log(`  Issues: ${report.issues.length} total (🔴${critical} 🟠${high} 🟡${medium} 🟢${low})`)
        if (report.readabilityScore !== undefined) {
          console.log(`  Readability: ${report.readabilityScore}/100`)
        }
        console.log()
      }
    })
  },
})

const ProofreadShowCommand = cmd({
  command: "show <documentID> <reportID>",
  describe: "show a specific proofreading report",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .positional("reportID", {
        type: "string",
        describe: "report ID",
      })
      .option("output", {
        type: "string",
        alias: "o",
        describe: "save report to file",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const report = await Proofreader.getReport(args.documentID as string, args.reportID as string)

      if (!report) {
        UI.error("Report not found")
        process.exit(1)
      }

      const markdown = Proofreader.formatReportAsMarkdown(report)

      if (args.output) {
        await Bun.write(args.output as string, markdown)
        console.log(`Report saved to: ${args.output}`)
      } else {
        console.log()
        console.log(markdown)
      }
    })
  },
})

const ProofreadFixCommand = cmd({
  command: "fix <documentID>",
  describe: "generate fix prompt for proofreading issues",
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
        demandOption: true,
      })
      .option("severity", {
        type: "string",
        choices: ["critical", "high", "medium", "low"],
        describe: "minimum severity to fix",
        default: "medium",
      })
      .option("type", {
        type: "string",
        choices: ["grammar", "spelling", "punctuation", "terminology", "style", "flow", "readability", "structure"],
        describe: "only fix issues of this type",
      })
      .option("dry-run", {
        type: "boolean",
        describe: "show issues without generating fix prompt",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const reports = await Proofreader.listReports(args.documentID as string)

      if (reports.length === 0) {
        console.log("No proofreading reports found. Run 'proofread check' first.")
        return
      }

      const latestReport = reports[0]

      // Filter issues by severity
      const severityOrder = { critical: 4, high: 3, medium: 2, low: 1 }
      const minSeverity = args.severity as "critical" | "high" | "medium" | "low"
      const minLevel = severityOrder[minSeverity] || 2

      let issuesToFix = latestReport.issues.filter((issue) => {
        return (severityOrder[issue.severity] || 0) >= minLevel
      })

      // Filter by type if specified
      if (args.type) {
        issuesToFix = issuesToFix.filter((issue) => issue.type === args.type)
      }

      // Only include issues for the specified chapter
      issuesToFix = issuesToFix.filter((issue) => {
        return issue.location?.chapterID === args.chapter
      })

      if (issuesToFix.length === 0) {
        console.log()
        console.log("No issues to fix with the given filters.")
        return
      }

      console.log()
      console.log(`Issues to fix: ${issuesToFix.length}`)
      console.log()

      if (args.dryRun) {
        for (const issue of issuesToFix) {
          const icon = { critical: "🔴", high: "🟠", medium: "🟡", low: "🟢" }[issue.severity] || "⚪"
          console.log(`${icon} [${issue.severity}] ${issue.type}`)
          console.log(`  ${issue.description}`)
          if (issue.suggestion) console.log(`  Fix: ${issue.suggestion}`)
          console.log()
        }
        return
      }

      const spinner = prompts.spinner()
      spinner.start("Generating fix prompt...")

      const prompt = await Proofreader.generateFixPrompt({
        documentID: args.documentID as string,
        chapterID: args.chapter as string,
        issues: issuesToFix,
      })

      spinner.stop("Prompt ready")

      console.log()
      console.log("--- FIX PROMPT ---")
      console.log()
      console.log(prompt)
      console.log()
    })
  },
})

const ProofreadReadabilityCommand = cmd({
  command: "readability <documentID>",
  describe: "analyze document readability",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "analyze specific chapter",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const result = await Proofreader.analyzeReadability(
        args.documentID as string,
        args.chapter as string,
      )

      console.log()
      console.log("## Readability Analysis")
      console.log()
      console.log(`**Score:** ${result.readabilityScore}/100 (${result.assessment})`)
      console.log()
      console.log("### Metrics")
      console.log()
      console.log(`- Average Sentence Length: ${result.metrics.avgSentenceLength.toFixed(1)} words`)
      console.log(`- Average Word Length: ${result.metrics.avgWordLength.toFixed(1)} characters`)
      console.log(`- Complex Words: ${result.metrics.complexWords}`)
      console.log(`- Total Sentences: ${result.metrics.totalSentences}`)
      console.log(`- Total Words: ${result.metrics.totalWords}`)
      console.log()

      // Assessment
      if (result.readabilityScore >= 80) {
        console.log("✓ Very readable content")
      } else if (result.readabilityScore >= 60) {
        console.log("✓ Standard readability")
      } else if (result.readabilityScore >= 40) {
        console.log("⚠ Content may be difficult for some readers")
      } else {
        console.log("⚠ Content may be very difficult to read")
      }
      console.log()
    })
  },
})

const ProofreadTerminologyCommand = cmd({
  command: "terminology <documentID>",
  describe: "check terminology consistency",
  builder: (yargs: Argv) => {
    return yargs.positional("documentID", {
      type: "string",
      describe: "document ID",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const issues = await Proofreader.checkTerminology(args.documentID as string)

      if (issues.length === 0) {
        console.log()
        console.log("No terminology issues found.")
        return
      }

      console.log()
      console.log(`Found ${issues.length} terminology issue(s):`)
      console.log()

      for (const issue of issues) {
        const icon = { low: "🟢", medium: "🟡", high: "🟠" }[issue.severity] || "⚪"
        console.log(`${icon} [${issue.severity.toUpperCase()}] ${issue.description}`)
        console.log()

        for (const occ of issue.occurrences.slice(0, 5)) {
          console.log(`  - "${occ.term}" in ${occ.chapter}`)
        }
        if (issue.occurrences.length > 5) {
          console.log(`  ... and ${issue.occurrences.length - 5} more`)
        }
        console.log()
      }
    })
  },
})

const ProofreadBatchCommand = cmd({
  command: "batch <documentID>",
  describe: "batch proofread multiple chapters",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("parallel", {
        type: "number",
        alias: "p",
        describe: "number of parallel checks",
        default: 3,
      })
      .option("depth", {
        type: "string",
        alias: "d",
        choices: ["quick", "standard", "deep"],
        describe: "checking depth",
        default: "standard",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const chapters = await Document.Chapter.list(args.documentID as string)
      const completed = chapters.filter((c) => c.status === "completed")

      if (completed.length === 0) {
        console.log("No completed chapters to check.")
        return
      }

      prompts.intro(`Batch Proofreading ${completed.length} chapters`)

      const spinner = prompts.spinner()
      spinner.start("Generating prompts...")

      const promptsList = await Proofreader.batchCheck({
        documentID: args.documentID as string,
        chapterIDs: completed.map((c) => c.id),
        depth: args.depth as "quick" | "standard" | "deep",
      })

      spinner.stop(`${promptsList.length} prompts generated`)

      console.log()
      console.log("To proofread chapters, run:")
      console.log()
      for (const item of promptsList.slice(0, 3)) {
        const chapter = completed.find((c) => c.id === item.chapterID)
        console.log(`  codecoder document proofread check ${args.documentID} --chapter ${item.chapterID}`)
      }
      if (promptsList.length > 3) {
        console.log(`  ... and ${promptsList.length - 3} more`)
      }
      console.log()
    })
  },
})

const ProofreadQuickCommand = cmd({
  command: "quick <documentID>",
  describe: "quick local grammar check (no AI)",
  builder: (yargs: Argv) => {
    return yargs
      .positional("documentID", {
        type: "string",
        describe: "document ID",
      })
      .option("chapter", {
        type: "string",
        alias: "c",
        describe: "check specific chapter",
        demandOption: true,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      const issues = await Proofreader.quickGrammarCheck(
        args.documentID as string,
        args.chapter as string,
      )

      if (issues.length === 0) {
        console.log()
        console.log("No issues found in quick check.")
        return
      }

      console.log()
      console.log(`Found ${issues.length} issue(s):`)
      console.log()

      for (const issue of issues) {
        console.log(`[${issue.type}] ${issue.description}`)
        console.log(`  Location: ${issue.location}`)
        console.log(`  Suggestion: ${issue.suggestion}`)
        console.log()
      }
    })
  },
})

const ProofreadCommand = cmd({
  command: "proofread",
  describe: "proofread document for errors and improvements",
  builder: (yargs) =>
    yargs
      .command(ProofreadCheckCommand)
      .command(ProofreadReportsCommand)
      .command(ProofreadShowCommand)
      .command(ProofreadFixCommand)
      .command(ProofreadReadabilityCommand)
      .command(ProofreadTerminologyCommand)
      .command(ProofreadBatchCommand)
      .command(ProofreadQuickCommand)
      .demandCommand(),
  async handler() {},
})

// ============================================================================
// Command Exports
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

export const DocumentCommand = cmd({
  command: "document",
  describe: "manage long-form documents",
  builder: (yargs) =>
    yargs
      .command(DocumentCreateCommand)
      .command(DocumentTemplateCommand)
      .command(DocumentOutlineCommand)
      .command(DocumentWriteCommand)
      .command(DocumentWriteAllCommand)
      .command(DocumentListCommand)
      .command(DocumentShowCommand)
      .command(DocumentExportCommand)
      .command(DocumentSetContentCommand)
      .command(DocumentDeleteCommand)
      .command(DocumentUpdateCommand)
      // New commands for long document support
      .command(ContextCommand)
      .command(SummaryGlobalCommand)
      .demandCommand(),
  async handler() {},
})

// Export new command groups
export { EntityCommand, VolumeCommand, SnapshotCommand }
export { CheckCommand, CheckStyleCommand }
export { SearchReplaceCommand, PolishCommand, ExpandCommand, CompressCommand }
export { ProofreadCommand }
