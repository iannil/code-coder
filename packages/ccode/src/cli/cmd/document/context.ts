/**
 * Context Commands
 *
 * Commands for managing document context and global summaries.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document, Context, Summary } from "../../../document"
import { escapeShellArg } from "../../../util/security"
import { $ } from "bun"

// ============================================================================
// Context Command
// ============================================================================

export const ContextCommand = cmd({
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
      const targetChapter = args.chapter
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

// ============================================================================
// Summary Global Command
// ============================================================================

export const SummaryGlobalCommand = cmd({
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
