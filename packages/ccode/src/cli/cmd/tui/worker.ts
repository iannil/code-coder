import { isLocal } from "@/version"
import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { InstanceBootstrap } from "@/project/bootstrap"
import { Rpc } from "@/util/rpc"
import { Bus } from "@/bus"
import type { Event } from "@/types"

// Local API imports
import { LocalSession, LocalPermission, LocalConfig, LocalFind } from "@/api"
import { SessionPrompt } from "@/session/prompt"
import { Session } from "@/session"
import { Provider } from "@/provider/provider"
import { Command } from "@/command"
import { LSP } from "@/lsp"
import { Format } from "@/format"
import { Agent } from "@/agent/agent"
import { Skill } from "@/skill/skill"
import { MCP } from "@/mcp"
import { Vcs } from "@/project/vcs"
import { Question } from "@/question"
import { Global } from "@/global"

await Log.init({
  print: process.argv.includes("--print-logs"),
  dev: isLocal(),
  level: (() => {
    if (isLocal()) return "DEBUG"
    return "INFO"
  })(),
})

process.on("unhandledRejection", (e) => {
  Log.Default.error("rejection", {
    e: e instanceof Error ? e.message : e,
  })
})

process.on("uncaughtException", (e) => {
  Log.Default.error("exception", {
    e: e instanceof Error ? e.message : e,
  })
})

// Worker state
let eventUnsub = () => {}
let initPromise: Promise<void> | null = null

// One-time initialization of instance and bus
async function initialize() {
  if (!initPromise) {
    initPromise = (async () => {
      const directory = process.cwd()
      Log.Default.info("worker initializing", { directory })

      // Establish instance context and initialize subsystems
      await Instance.provide({
        directory,
        init: InstanceBootstrap,
        fn: async () => {
          // Subscribe to Bus events and forward them via RPC
          eventUnsub = Bus.subscribeAll(async (event) => {
            Rpc.emit("event", event as Event)
          })
          Log.Default.info("worker initialized")
        },
      })
    })()
  }
  return initPromise
}

// Local API wrapper that mimics the SDK client interface
const localApi = {
  session: {
    list: LocalSession.list,
    get: Session.get,
    create: LocalSession.create,
    fork: LocalSession.fork,
    remove: async (input: { sessionID: string }) => {
      await Session.remove(input.sessionID)
      return true
    },
    delete: async (input: { sessionID: string }) => {
      await Session.remove(input.sessionID)
      return true
    },
    compact: LocalSession.compact,
    revert: LocalSession.revert,
    status: LocalSession.status,
    summary: LocalSession.summary,
    todo: LocalSession.todo,
    prompt: LocalSession.prompt,
    command: LocalSession.command,
    children: LocalSession.children,
    messages: LocalSession.messages,
    diff: async () => [],
    abort: (input: { sessionID: string }) => {
      SessionPrompt.cancel(input.sessionID)
      return true
    },
  },

  permission: {
    list: LocalPermission.list,
    reply: LocalPermission.reply,
    respond: LocalPermission.respond,
  },

  config: {
    get: LocalConfig.get,
    update: LocalConfig.update,
    providers: async () => {
      const result = await Provider.listAll()
      const providers = result.all.map((p) => ({
        id: p.id,
        name: p.name,
        models: p.models,
      }))
      return { providers, default: result.default }
    },
  },

  provider: {
    list: Provider.listAll,
    auth: Provider.authMethods,
  },

  command: {
    list: Command.list,
  },

  app: {
    agents: Agent.list,
    skills: Skill.all,
  },

  lsp: {
    status: LSP.status,
  },

  formatter: {
    status: Format.status,
  },

  mcp: {
    status: MCP.status,
    resources: MCP.resources,
  },

  experimental: {
    resources: MCP.resources,
    resource: {
      list: MCP.resources,
    },
  },

  find: {
    files: LocalFind.files,
  },

  vcs: {
    get: async () => ({ branch: await Vcs.branch() }),
  },

  path: {
    get: () => ({
      home: Global.Path.home,
      state: Global.Path.state,
      config: Global.Path.config,
      worktree: Instance.worktree,
      directory: Instance.directory,
    }),
  },

  question: {
    reply: (input: { requestID: string; answers: string[][] }) =>
      Question.reply({ requestID: input.requestID, answers: input.answers }),
    reject: (input: { requestID: string }) => Question.reject(input.requestID),
  },
}

type ApiCall = {
  namespace: string
  method: string
  args: any[]
}

export const rpc = {
  async call(input: ApiCall): Promise<any> {
    // Ensure worker is initialized before handling API calls
    await initialize()

    // Execute API call within instance context
    const directory = process.cwd()
    return Instance.provide({
      directory,
      fn: async () => {
        const ns = input.namespace as keyof typeof localApi
        const methodPath = input.method.split(".")
        let api: any = localApi[ns]
        if (!api) {
          throw new Error(`Unknown API namespace: ${ns}`)
        }
        // Navigate nested methods like "resource.list"
        for (const part of methodPath) {
          api = api[part]
          if (!api) {
            throw new Error(`Unknown API: ${ns}.${input.method}`)
          }
        }
        if (typeof api !== "function") {
          throw new Error(`API ${ns}.${input.method} is not a function`)
        }
        return await api(...input.args)
      },
    })
  },

  async reload() {
    await Instance.disposeAll()
  },

  async shutdown() {
    Log.Default.info("worker shutting down")
    eventUnsub()
    await Instance.disposeAll()
  },
}

Rpc.listen(rpc)
