/**
 * Template Commands
 *
 * Commands for managing document templates.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document } from "../../../document"
import * as Templates from "../../../document/templates"

// ============================================================================
// Template List Command
// ============================================================================

export const TemplateListCommand = cmd({
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

// ============================================================================
// Template Use Command
// ============================================================================

export const TemplateUseCommand = cmd({
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

// ============================================================================
// Document Template Command (groups all subcommands)
// ============================================================================

export const DocumentTemplateCommand = cmd({
  command: "template",
  describe: "manage document templates",
  builder: (yargs) =>
    yargs.command(TemplateListCommand).command(TemplateUseCommand).demandCommand(),
  async handler() {},
})
