import yargs from "yargs"
import { hideBin } from "yargs/helpers"
import { RunCommand } from "./cli/cmd/run"
import { Log } from "./util/log"
import { AgentCommand } from "./cli/cmd/agent"
import { ModelsCommand } from "./cli/cmd/models"
import { UI } from "./cli/ui"
import { VERSION, isLocal } from "./version"
import { NamedError } from "@codecoder-ai/util/error"
import { FormatError } from "./cli/error"
import { DebugCommand } from "./cli/cmd/debug"
import { McpCommand } from "./cli/cmd/mcp"
import { TuiThreadCommand } from "./cli/cmd/tui/thread"
import { EOL } from "os"
import { SessionCommand } from "./cli/cmd/session"
import { DocumentCommand, ChapterCommand } from "./cli/cmd/document"
import { BookExpandCommand } from "./cli/cmd/book-writer"
import { ReverseCommands } from "./cli/cmd/reverse"
import { JarReverseCommands } from "./cli/cmd/jar-reverse"
import { MemoryCommand } from "./cli/cmd/memory"
import { GetStartedCommand } from "./cli/cmd/get-started"
import { AutonomousCommand } from "./cli/cmd/autonomous"
import { ServeCommand } from "./cli/cmd/serve"
import { WebCommand } from "./cli/cmd/web"
import { GlobalErrorHandler } from "./util/global-error-handler"

// Initialize global error handler early (writes to project dev.log)
GlobalErrorHandler.init()

process.on("unhandledRejection", (e) => {
  Log.Default.error("rejection", {
    e: e instanceof Error ? e.message : e,
  })
  GlobalErrorHandler.logError("Unhandled Rejection", e)
})

process.on("uncaughtException", (e) => {
  Log.Default.error("exception", {
    e: e instanceof Error ? e.message : e,
  })
  GlobalErrorHandler.logError("Uncaught Exception", e)
})

const cli = yargs(hideBin(process.argv))
  .parserConfiguration({ "populate--": true })
  .scriptName("codecoder")
  .wrap(100)
  .help("help", "show help")
  .alias("help", "h")
  .version("version", "show version number", VERSION)
  .alias("version", "v")
  .option("print-logs", {
    describe: "print logs to stderr",
    type: "boolean",
  })
  .option("log-level", {
    describe: "log level",
    type: "string",
    choices: ["DEBUG", "INFO", "WARN", "ERROR"],
  })
  .middleware(async (opts) => {
    await Log.init({
      print: process.argv.includes("--print-logs"),
      dev: isLocal(),
      level: (() => {
        if (opts.logLevel) return opts.logLevel as Log.Level
        if (isLocal()) return "DEBUG"
        return "INFO"
      })(),
    })

    process.env.AGENT = "1"
    process.env.CODECODER = "1"

    Log.Default.info("codecoder", {
      version: VERSION,
      args: process.argv.slice(2),
    })
  })
  .usage("\n" + UI.logo())
  .completion("completion", "generate shell completion script")
  .command(McpCommand)
  .command(TuiThreadCommand)
  .command(RunCommand)
  .command(DebugCommand)
  .command(AgentCommand)
  .command(ModelsCommand)
  .command(SessionCommand)
  .command(DocumentCommand)
  .command(BookExpandCommand)
  .command(ReverseCommands)
  .command(JarReverseCommands)
  .command(MemoryCommand)
  .command(GetStartedCommand)
  .command(AutonomousCommand)
  .command(ServeCommand as any)
  .command(WebCommand as any)
  .command(ChapterCommand)
  .fail((msg, err) => {
    if (
      msg?.startsWith("Unknown argument") ||
      msg?.startsWith("Not enough non-option arguments") ||
      msg?.startsWith("Invalid values:")
    ) {
      if (err) throw err
      cli.showHelp("log")
    }
    if (err) throw err
    process.exit(1)
  })
  .strict()

try {
  await cli.parse()
} catch (e) {
  let data: Record<string, any> = {}
  if (e instanceof NamedError) {
    const obj = e.toObject()
    Object.assign(data, {
      ...obj.data,
    })
  }

  if (e instanceof Error) {
    Object.assign(data, {
      name: e.name,
      message: e.message,
      cause: e.cause?.toString(),
      stack: e.stack,
    })
  }

  if (e instanceof ResolveMessage) {
    Object.assign(data, {
      name: e.name,
      message: e.message,
      code: e.code,
      specifier: e.specifier,
      referrer: e.referrer,
      position: e.position,
      importKind: e.importKind,
    })
  }
  Log.Default.error("fatal", data)
  GlobalErrorHandler.logError("CLI Fatal Error", e)
  const formatted = FormatError(e)
  if (formatted) UI.error(formatted)
  if (formatted === undefined) {
    UI.error("Unexpected error, check log file at " + Log.file() + " for more details" + EOL)
    console.error(e instanceof Error ? e.message : String(e))
  }
  process.exitCode = 1
} finally {
  // Some subprocesses don't react properly to SIGTERM and similar signals.
  // Most notably, some docker-container-based MCP servers don't handle such signals unless
  // run using `docker run --init`.
  // Explicitly exit to avoid any hanging subprocesses.
  process.exit()
}
