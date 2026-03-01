/**
 * Volume Commands
 *
 * Commands for managing document volumes (logical groupings of chapters).
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { bootstrap } from "../../bootstrap"
import { Volume } from "../../../document"

// ============================================================================
// Create Command
// ============================================================================

export const VolumeCreateCommand = cmd({
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

// ============================================================================
// List Command
// ============================================================================

export const VolumeListCommand = cmd({
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

// ============================================================================
// Summary Command
// ============================================================================

export const VolumeSummaryCommand = cmd({
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

// ============================================================================
// Auto Command
// ============================================================================

export const VolumeAutoCommand = cmd({
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

// ============================================================================
// Main Volume Command (groups all subcommands)
// ============================================================================

export const VolumeCommand = cmd({
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
