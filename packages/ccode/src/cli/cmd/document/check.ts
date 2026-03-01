/**
 * Check Commands
 *
 * Commands for document consistency and style checking.
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { bootstrap } from "../../bootstrap"
import { Consistency } from "../../../document"

// ============================================================================
// Check Command
// ============================================================================

export const CheckCommand = cmd({
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

// ============================================================================
// Check Style Command
// ============================================================================

export const CheckStyleCommand = cmd({
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
