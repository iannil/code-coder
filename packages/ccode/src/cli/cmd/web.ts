/**
 * Web Development Server CLI Command
 * Starts the CodeCoder Web UI development server
 */

import type { Argv } from "yargs"
import type { CommandModule } from "yargs"
import { cmd } from "./cmd"
import { join } from "path"
import { spawn } from "child_process"

interface WebOptions {
  port: number
  host: string
  open: boolean
}

const webCommandImpl: CommandModule<{}, WebOptions> = {
  command: "web",
  describe: "Start Web UI development server",
  builder: (yargs: Argv<{}>) =>
    yargs
      .option("port", {
        type: "number",
        describe: "port to listen on",
        default: 3000,
        alias: "p",
      })
      .option("host", {
        type: "string",
        describe: "hostname to listen on",
        default: "localhost",
      })
      .option("open", {
        type: "boolean",
        describe: "open browser on startup",
        default: false,
        alias: "o",
      }),
  handler: async (args) => {
    // Find the web package directory relative to the ccode package
    const webDir = join(import.meta.dir, "../../../../web")

    console.log(`Starting Web UI development server...`)
    console.log(`  Directory: ${webDir}`)
    console.log(`  URL: http://${args.host}:${args.port}`)

    const viteArgs = ["run", "vite", "--port", String(args.port), "--host", args.host]

    if (args.open) {
      viteArgs.push("--open")
    }

    const child = spawn("bun", viteArgs, {
      cwd: webDir,
      stdio: "inherit",
      env: {
        ...process.env,
        FORCE_COLOR: "1",
      },
    })

    child.on("error", (err) => {
      console.error("Failed to start web server:", err.message)
      process.exit(1)
    })

    child.on("exit", (code) => {
      process.exit(code ?? 0)
    })

    // Handle termination signals
    const cleanup = () => {
      child.kill()
    }

    process.on("SIGINT", cleanup)
    process.on("SIGTERM", cleanup)

    // Keep process alive
    await new Promise(() => {})
  },
}

export const WebCommand = cmd(webCommandImpl)
