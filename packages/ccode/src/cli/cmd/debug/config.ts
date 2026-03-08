import { EOL } from "os"
import { Config } from "../../../config/config"
import { migrateConfig, validateConfig, showConfig } from "../../../config/migrate"
import { bootstrap } from "../../bootstrap"
import { cmd } from "../cmd"

export const ConfigCommand = cmd({
  command: "config",
  describe: "configuration management commands",
  builder: (yargs) =>
    yargs
      .command(ShowCommand)
      .command(MigrateCommand)
      .command(ValidateCommand)
      .demandCommand(),
  async handler() {},
})

const ShowCommand = cmd({
  command: "show",
  aliases: ["$0"],
  describe: "show resolved configuration",
  builder: (yargs) => yargs,
  async handler() {
    await bootstrap(process.cwd(), async () => {
      const config = await Config.get()
      process.stdout.write(JSON.stringify(config, null, 2) + EOL)
    })
  },
})

const MigrateCommand = cmd({
  command: "migrate",
  describe: "migrate separate config files into unified config.json",
  builder: (yargs) =>
    yargs.option("dry-run", {
      alias: "d",
      type: "boolean",
      default: false,
      describe: "preview changes without modifying files",
    }),
  async handler(args) {
    await bootstrap(process.cwd(), async () => {
      const result = await migrateConfig({ dryRun: args.dryRun as boolean })

      if (args.dryRun) {
        console.log("=== DRY RUN - No files modified ===\n")
      }

      if (result.filesProcessed.length === 0) {
        console.log("No files to migrate. Configuration is already unified.")
        return
      }

      console.log("Files to migrate:")
      for (const file of result.filesProcessed) {
        console.log(`  ✓ ${file}`)
      }

      if (result.filesSkipped.length > 0) {
        console.log("\nFiles skipped (not found):")
        for (const file of result.filesSkipped) {
          console.log(`  - ${file}`)
        }
      }

      if (args.dryRun && result.preview) {
        console.log("\n=== Preview of merged config.json ===")
        console.log(JSON.stringify(result.preview, null, 2))
      }

      if (!args.dryRun) {
        if (result.success) {
          console.log("\n✓ Migration completed successfully")
          console.log("  Original files backed up with .backup extension")
          console.log("  Run 'ccode debug config show' to verify")
        } else {
          console.error("\n✗ Migration failed:")
          for (const error of result.errors) {
            console.error(`  - ${error}`)
          }
          process.exit(1)
        }
      }
    })
  },
})

const ValidateCommand = cmd({
  command: "validate",
  describe: "validate configuration and check for issues",
  builder: (yargs) => yargs,
  async handler() {
    await bootstrap(process.cwd(), async () => {
      const result = await validateConfig()

      if (result.valid) {
        console.log("✓ Configuration is valid")
      } else {
        console.log("✗ Configuration issues found:")
        for (const issue of result.issues) {
          console.log(`  - ${issue}`)
        }
        process.exit(1)
      }
    })
  },
})
