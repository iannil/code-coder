import { cmd } from "@/cli/cmd/cmd"
import { tui } from "./app"
import path from "path"
import { UI } from "@/cli/ui"
import { iife } from "@/util/iife"
import { Log } from "@/util/log"
import { setRpcClient } from "./context/sdk"
import { createWorkerBackend, createIpcBackend, type TuiBackend, type BackendMode } from "./backend"

/**
 * TUI Thread Command
 *
 * Starts the CodeCoder TUI with configurable backend mode:
 * - **worker** (default): In-process Web Worker running LocalAPI
 * - **ipc**: Communication with zero-cli via Unix socket
 *
 * ## Usage
 *
 * ```bash
 * # Default worker mode
 * ccode
 *
 * # IPC mode (requires zero-cli)
 * ccode --backend ipc
 *
 * # With project path
 * ccode ./my-project --backend worker
 * ```
 */
export const TuiThreadCommand = cmd({
  command: "$0 [project]",
  describe: "start CodeCoder tui",
  builder: (yargs) =>
    yargs
      .positional("project", {
        type: "string",
        describe: "path to start codecoder in",
      })
      .option("model", {
        type: "string",
        alias: ["m"],
        describe: "model to use in the format of provider/model",
      })
      .option("continue", {
        alias: ["c"],
        describe: "continue the last session",
        type: "boolean",
      })
      .option("session", {
        alias: ["s"],
        type: "string",
        describe: "session id to continue",
      })
      .option("prompt", {
        type: "string",
        describe: "prompt to use",
      })
      .option("agent", {
        type: "string",
        describe: "agent to use",
      })
      .option("mode", {
        type: "string",
        choices: ["build", "writer", "decision"] as const,
        default: "build",
        describe: "agent mode: build (dev), writer (content), decision (advisory)",
      })
      .option("backend", {
        type: "string",
        choices: ["worker", "ipc"] as const,
        default: "worker" as BackendMode,
        describe: "backend mode: worker (in-process) or ipc (zero-cli)",
      })
      .option("socket", {
        type: "string",
        describe: "path to IPC socket (only used with --backend ipc)",
      }),
  handler: async (args) => {
    // Resolve relative paths against PWD to preserve behavior when using --cwd flag
    const baseCwd = process.env.PWD ?? process.cwd()
    const cwd = args.project ? path.resolve(baseCwd, args.project) : process.cwd()

    try {
      process.chdir(cwd)
    } catch {
      UI.error("Failed to change directory to " + cwd)
      return
    }

    // Create backend based on mode
    const backendMode = args.backend as BackendMode
    let backend: TuiBackend

    try {
      if (backendMode === "ipc") {
        Log.Default.info("Starting TUI in IPC mode")
        backend = await createIpcBackend({
          socketPath: args.socket,
          autoStart: true,
        })
        // Initialize session for IPC backend
        await (backend as import("./backend/ipc").IpcBackend).initializeSession({
          cwd: process.cwd(),
          sessionId: args.session,
        })
      } else {
        Log.Default.info("Starting TUI in Worker mode")
        backend = await createWorkerBackend()
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err)
      UI.error(`Failed to initialize ${backendMode} backend: ${errorMsg}`)
      return
    }

    // Set up process event handlers
    process.on("uncaughtException", (e) => {
      Log.Default.error(e)
    })
    process.on("unhandledRejection", (e) => {
      Log.Default.error(e)
    })
    process.on("SIGUSR2", async () => {
      await backend.reload()
    })

    // Read piped input
    const prompt = await iife(async () => {
      // Only read from stdin if it's actually piped (not a TTY and has data available)
      // Using a short timeout to avoid blocking indefinitely when stdin.isTTY is incorrectly false
      let piped: string | undefined
      if (!process.stdin.isTTY) {
        const timeoutPromise = new Promise<undefined>((resolve) => setTimeout(() => resolve(undefined), 100))
        const stdinPromise = Bun.stdin.text()
        piped = await Promise.race([stdinPromise, timeoutPromise])
      }
      if (!args.prompt) return piped
      return piped ? piped + "\n" + args.prompt : args.prompt
    })

    // Set up the RPC client for SDK calls
    setRpcClient(backend.rpc)

    // Use local API directly (no HTTP server needed)
    const url = "http://codecoder.internal"
    const events = backend.events

    const tuiPromise = tui({
      url,
      events,
      args: {
        continue: args.continue,
        sessionID: args.session,
        agent: args.agent,
        mode: args.mode,
        model: args.model,
        prompt,
      },
      onExit: async () => {
        await backend.shutdown()
      },
    })

    await tuiPromise
  },
})
