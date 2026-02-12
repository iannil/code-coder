import type { Argv } from "yargs"
import { cmd } from "./cmd"
import { bootstrap } from "../bootstrap"
import { UI } from "../ui"
import path from "path"
import { Log } from "@/util/log"
import {
  loadLongTermMemory,
  loadCategory,
  listDailyNoteDates,
  loadDailyNotes,
  consolidateMemory,
  getConsolidationStats,
  type MemoryCategory,
} from "@/memory-markdown"

const log = Log.create({ service: "cli.memory" })

const MEMORY_CATEGORIES: MemoryCategory[] = ["用户偏好", "项目上下文", "关键决策", "经验教训"]

function pagerCmd(): string[] {
  const lessOptions = ["-R", "-S"]
  if (process.platform !== "win32") {
    return ["less", ...lessOptions]
  }

  const lessOnPath = Bun.which("less")
  if (lessOnPath) {
    if (Bun.file(lessOnPath).size) return [lessOnPath, ...lessOptions]
  }

  return ["cmd", "/c", "more"]
}

export const MemoryCommand = cmd({
  command: "memory",
  describe: "manage markdown-based memory",
  builder: (yargs: Argv) =>
    yargs
      .command(ViewCommand)
      .command(EditCommand)
      .command(ListCommand)
      .command(ConsolidateCommand)
      .command(StatsCommand)
      .demandCommand(),
  async handler() {},
})

export const ViewCommand = cmd({
  command: "view [category]",
  describe: "view memory content",
  builder: (yargs: Argv) => {
    return yargs
      .positional("category", {
        describe: "memory category to view",
        type: "string",
        choices: [...MEMORY_CATEGORIES, "all", "daily"],
        default: "all",
      })
      .option("days", {
        alias: "d",
        describe: "number of recent days to show (for daily)",
        type: "number",
        default: 7,
      })
      .option("output", {
        alias: "o",
        describe: "output file path",
        type: "string",
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      try {
        let content: string

        if (args.category === "daily") {
          const dates = await listDailyNoteDates()
          const recentDates = dates.slice(-args.days)
          const dailyContents: string[] = []

          for (const date of recentDates) {
            const notes = await loadDailyNotes(new Date(date))
            const notesArray = Array.isArray(notes) ? notes : []
            if (notesArray.length > 0) {
              dailyContents.push(`# ${date}\n${notes.join("\n")}`)
            }
          }

          content = dailyContents.length > 0 ? dailyContents.join("\n\n") : "_No daily notes found._"
        } else if (args.category === "all") {
          content = await loadLongTermMemory()
        } else {
          content = await loadCategory(args.category as MemoryCategory)
        }

        if (args.output) {
          await Bun.write(args.output, content)
          log.info("memory exported", { path: args.output })
        } else if (process.stdout.isTTY) {
          const proc = Bun.spawn({
            cmd: pagerCmd(),
            stdin: "pipe",
            stdout: "inherit",
            stderr: "inherit",
          })
          proc.stdin.write(content)
          proc.stdin.end()
          await proc.exited
        } else {
          console.log(content)
        }
      } catch (error) {
        UI.error(`Failed to view memory: ${error instanceof Error ? error.message : String(error)}`)
        process.exit(1)
      }
    })
  },
})

export const EditCommand = cmd({
  command: "edit [category]",
  describe: "edit memory file in external editor",
  builder: (yargs: Argv) => {
    return yargs.positional("category", {
      describe: "memory category to edit or 'daily' for today's notes",
      type: "string",
      choices: [...MEMORY_CATEGORIES, "all", "daily"],
      default: "daily",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      try {
        const memoryDir = path.join(process.cwd(), "memory")
        const editor = process.env.EDITOR ?? process.env.VISUAL ?? "vi"

        let filePath: string

        if (args.category === "daily") {
          const today = new Date().toISOString().split("T")[0]
          filePath = path.join(memoryDir, "daily", `${today}.md`)
        } else if (args.category === "all") {
          filePath = path.join(memoryDir, "MEMORY.md")
        } else {
          filePath = path.join(memoryDir, "MEMORY.md")
        }

        // Ensure file exists
        if (args.category === "daily") {
          await Bun.write(filePath, `# Daily Notes - ${new Date().toISOString().split("T")[0]}\n\n`)
        } else if (args.category !== "all") {
          const existing = await loadLongTermMemory()
          await Bun.write(path.join(memoryDir, "MEMORY.md"), existing)
        }

        // Open in editor
        const proc = Bun.spawn({
          cmd: [editor, filePath],
          stdin: "inherit",
          stdout: "inherit",
          stderr: "inherit",
        })

        const exitCode = await proc.exited
        if (exitCode !== 0) {
          UI.error(`Editor exited with code ${exitCode}`)
          process.exit(1)
        }

        log.info("memory file edited", { path: filePath })
      } catch (error) {
        UI.error(`Failed to edit memory: ${error instanceof Error ? error.message : String(error)}`)
        process.exit(1)
      }
    })
  },
})

export const ListCommand = cmd({
  command: "list",
  describe: "list daily note dates",
  builder: (yargs: Argv) => {
    return yargs.option("format", {
      alias: "f",
      describe: "output format",
      type: "string",
      choices: ["list", "json", "table"],
      default: "list",
    })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      try {
        const dates = await listDailyNoteDates()

        if (args.format === "json") {
          console.log(JSON.stringify({ dates, count: dates.length }, null, 2))
        } else if (args.format === "table") {
          console.log(UI.header("Daily Notes"))
          console.log("")
          if (dates.length === 0) {
            console.log("_No daily notes found._")
          } else {
            for (const date of dates) {
              console.log(`  ${date}`)
            }
          }
          console.log("")
          console.log(`Total: ${dates.length} days`)
        } else {
          for (const date of dates) {
            console.log(date)
          }
        }
      } catch (error) {
        UI.error(`Failed to list daily notes: ${error instanceof Error ? error.message : String(error)}`)
        process.exit(1)
      }
    })
  },
})

export const ConsolidateCommand = cmd({
  command: "consolidate",
  describe: "consolidate daily notes into long-term memory",
  builder: (yargs: Argv) => {
    return yargs
      .option("days", {
        alias: "d",
        describe: "number of recent days to process",
        type: "number",
        default: 7,
      })
      .option("min-importance", {
        alias: "m",
        describe: "minimum importance score (0-1)",
        type: "number",
        default: 0.5,
      })
      .option("dry-run", {
        alias: "n",
        describe: "show what would be consolidated without making changes",
        type: "boolean",
        default: false,
      })
  },
  handler: async (args) => {
    await bootstrap(process.cwd(), async () => {
      try {
        if (args.dryRun) {
          console.log(UI.header("Dry Run: Consolidation Preview"))
          console.log("")
          console.log(`Would process last ${args.days} days of daily notes`)
          console.log(`Minimum importance: ${args.minImportance}`)
          console.log("")
          console.log("Run without --dry-run to perform consolidation.")
          return
        }

        console.log(UI.header("Consolidating Memory"))
        console.log("")

        const results = await consolidateMemory({
          days: args.days,
          minImportance: args.minImportance,
        })

        if (results.length === 0) {
          console.log("_No important entries found for consolidation._")
        } else {
          for (const result of results) {
            console.log(`**${result.category}**: ${result.entries.length} entries`)
          }
        }

        console.log("")
        console.log("Consolidation complete.")
      } catch (error) {
        UI.error(`Failed to consolidate memory: ${error instanceof Error ? error.message : String(error)}`)
        process.exit(1)
      }
    })
  },
})

export const StatsCommand = cmd({
  command: "stats",
  describe: "show memory statistics",
  builder: (yargs: Argv) => yargs,
  handler: async () => {
    await bootstrap(process.cwd(), async () => {
      try {
        const stats = await getConsolidationStats()
        const dates = await listDailyNoteDates()

        console.log(UI.header("Memory Statistics"))
        console.log("")
        console.log(`Daily Notes: ${stats.totalDailyNotes} days`)
        console.log(`Last Daily Note: ${stats.lastConsolidated ?? "None"}`)
        console.log("")

        // Show recent activity
        if (dates.length > 0) {
          console.log("Recent Activity:")
          const recentDates = dates.slice(-5).reverse()
          for (const date of recentDates) {
            const notes = await loadDailyNotes(new Date(date))
            const notesArray = Array.isArray(notes) ? notes : []
            console.log(`  ${date}: ${notesArray.length} entries`)
          }
        }

        console.log("")
      } catch (error) {
        UI.error(`Failed to get memory stats: ${error instanceof Error ? error.message : String(error)}`)
        process.exit(1)
      }
    })
  },
})
