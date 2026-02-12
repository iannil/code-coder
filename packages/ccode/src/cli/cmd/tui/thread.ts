import { cmd } from "@/cli/cmd/cmd"
import { tui } from "./app"
import { Rpc } from "@/util/rpc"
import { type rpc } from "./worker"
import path from "path"
import { UI } from "@/cli/ui"
import { iife } from "@/util/iife"
import { Log } from "@/util/log"
import type { Event } from "@/types"
import type { EventSource } from "./context/sdk"
import { setRpcClient } from "./context/sdk"

declare global {
  const CCODE_WORKER_PATH: string
}

function createEventSource(client: ReturnType<typeof Rpc.client<typeof rpc>>): EventSource {
  return {
    on: (handler) => {
      const unsub = client.on("event", handler)
      return () => {
        unsub()
      }
    },
  }
}

function createSDKRpcClient(client: ReturnType<typeof Rpc.client<typeof rpc>>): {
  call: (input: { namespace: string; method: string; args: any[] }) => Promise<any>
  on: (event: string, handler: (data: any) => void) => () => void
} {
  return {
    call: async (input) => {
      return await client.call("call", input)
    },
    on: (event, handler) => client.on(event, handler),
  }
}

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
      }),
  handler: async (args) => {
    // Resolve relative paths against PWD to preserve behavior when using --cwd flag
    const baseCwd = process.env.PWD ?? process.cwd()
    const cwd = args.project ? path.resolve(baseCwd, args.project) : process.cwd()
    const localWorker = new URL("./worker.ts", import.meta.url)
    const distWorker = new URL("./cli/cmd/tui/worker.js", import.meta.url)
    const workerPath = await iife(async () => {
      if (typeof CCODE_WORKER_PATH !== "undefined") return CCODE_WORKER_PATH
      if (await Bun.file(distWorker).exists()) return distWorker
      return localWorker
    })
    try {
      process.chdir(cwd)
    } catch (e) {
      UI.error("Failed to change directory to " + cwd)
      return
    }

    const worker = new Worker(workerPath, {
      env: Object.fromEntries(
        Object.entries(process.env).filter((entry): entry is [string, string] => entry[1] !== undefined),
      ),
    })
    worker.onerror = (e) => {
      Log.Default.error(e)
    }
    const client = Rpc.client<typeof rpc>(worker)
    process.on("uncaughtException", (e) => {
      Log.Default.error(e)
    })
    process.on("unhandledRejection", (e) => {
      Log.Default.error(e)
    })
    process.on("SIGUSR2", async () => {
      await client.call("reload", undefined)
    })

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
    const sdkRpcClient = createSDKRpcClient(client)
    setRpcClient(sdkRpcClient)

    // Use local API directly (no HTTP server needed)
    const url = "http://codecoder.internal"
    const events = createEventSource(client)

    const tuiPromise = tui({
      url,
      events,
      args: {
        continue: args.continue,
        sessionID: args.session,
        agent: args.agent,
        model: args.model,
        prompt,
      },
      onExit: async () => {
        await client.call("shutdown", undefined)
      },
    })

    await tuiPromise
  },
})
