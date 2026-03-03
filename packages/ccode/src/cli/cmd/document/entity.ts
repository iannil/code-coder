/**
 * Entity Commands
 *
 * Commands for managing document entities (characters, locations, items, etc.).
 */
import type { Argv } from "yargs"
import { cmd } from "../cmd"
import * as prompts from "@clack/prompts"
import { UI } from "../../ui"
import { bootstrap } from "../../bootstrap"
import { Document, Entity } from "../../../document"
import { DocumentSchema } from "../../../document/schema"
import { escapeShellArg } from "@/util/security"
import { $ } from "bun"

// ============================================================================
// Extract Command
// ============================================================================

export const EntityExtractCommand = cmd({
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
      const targetChapter = args.chapter
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

// ============================================================================
// List Command
// ============================================================================

export const EntityListCommand = cmd({
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

// ============================================================================
// Show Command
// ============================================================================

export const EntityShowCommand = cmd({
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

// ============================================================================
// Duplicates Command
// ============================================================================

export const EntityDuplicatesCommand = cmd({
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

// ============================================================================
// Main Entity Command (groups all subcommands)
// ============================================================================

export const EntityCommand = cmd({
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
