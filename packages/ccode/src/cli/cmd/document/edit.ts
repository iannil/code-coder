/**
 * Editor Commands
 *
 * Commands for editing document content (search-replace, polish, expand, compress).
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import { bootstrap } from "../../bootstrap"
import { Editor } from "../../../document"

// ============================================================================
// Search Replace Command
// ============================================================================

export const SearchReplaceCommand = cmd({
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

// ============================================================================
// Polish Command
// ============================================================================

export const PolishCommand = cmd({
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

// ============================================================================
// Expand Command
// ============================================================================

export const ExpandCommand = cmd({
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

// ============================================================================
// Compress Command
// ============================================================================

export const CompressCommand = cmd({
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
