/**
 * Outline Commands
 *
 * Commands for generating and managing document outlines.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document } from "../../../document"
import { escapeShellArg } from "../../../util/security"
import { $ } from "bun"

// ============================================================================
// Document Outline Command
// ============================================================================

export const DocumentOutlineCommand = cmd({
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
