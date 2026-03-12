import { isLocal } from "@/version"
import { Log } from "@/util/log"
import { Instance } from "@/project/instance"
import { InstanceBootstrap } from "@/project/bootstrap"
import { Rpc } from "@/util/rpc"
import { Bus } from "@/bus"
import type { Event } from "@/types"

// Local API imports (for features not yet migrated to SDK)
import { LocalSession, LocalPermission, LocalConfig, LocalFind } from "@/api"
import { SessionPrompt } from "@/session/prompt"
import { Command } from "@/agent/command"
import { LSP } from "@/lsp"
import { Format } from "@/util/format"
import { Skill } from "@/skill/skill"
import { MCP } from "@/mcp"
import { Vcs } from "@/project/vcs"
import { Question } from "@/agent/question"
import { Global } from "@/util/global"

// SDK imports - primary API access
import { getHttpClient, adaptSessionList, adaptSessionInfo } from "@/sdk"
import { promptViaWebSocket, type WebSocketPromptInput } from "@/sdk"
import type { AgentInfo, ProviderInfo } from "@/sdk"
import { MessageV2 } from "@/session/message-v2"

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

// Feature flag for SDK-based prompt (via WebSocket to Rust daemon)
// Set to true to use the new WebSocket-based agent execution
const USE_SDK_PROMPT = process.env.USE_SDK_PROMPT === "true"

/**
 * SDK-based prompt implementation using WebSocket executeAgent()
 * This replaces LocalSession.prompt when USE_SDK_PROMPT is enabled.
 */
async function promptViaSdk(input: WebSocketPromptInput): Promise<{ messageID: string; content?: string }> {
  // Create publisher that bridges SDK events to Bus
  const publisher = {
    publishPartUpdated: (part: unknown, delta?: string) => {
      Bus.publish(MessageV2.Event.PartUpdated, { part: part as MessageV2.Part, delta })
    },
    publishMessageUpdated: (info: unknown) => {
      Bus.publish(MessageV2.Event.Updated, { info: info as MessageV2.Info })
    },
    publishStepStart: (part: unknown) => {
      Bus.publish(MessageV2.Event.PartUpdated, { part: part as MessageV2.Part })
    },
    publishStepFinish: (part: unknown) => {
      Bus.publish(MessageV2.Event.PartUpdated, { part: part as MessageV2.Part })
    },
    publishError: (error: { code: string; message: string }) => {
      Log.Default.error("SDK agent error", error)
    },
  }
  return promptViaWebSocket(input, publisher)
}

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
    list: async (input?: {
      directory?: string
      roots?: boolean
      start?: number
      search?: string
      limit?: number
    }) => {
      const client = getHttpClient()
      const response = await client.listSessions(input?.limit ?? 50, 0)
      const adapted = adaptSessionList(response.sessions, {
        directory: input?.directory ?? Instance.directory,
      })
      // Apply filters that SDK doesn't support natively
      let filtered = adapted
      if (input?.directory !== undefined) {
        filtered = filtered.filter((s) => s.directory === input.directory)
      }
      if (input?.roots) {
        filtered = filtered.filter((s) => !s.parentID)
      }
      if (input?.start !== undefined) {
        filtered = filtered.filter((s) => s.time.updated >= input.start!)
      }
      if (input?.search !== undefined) {
        const search = input.search.toLowerCase()
        filtered = filtered.filter((s) => s.title.toLowerCase().includes(search))
      }
      return filtered
    },
    get: async (input: { sessionID: string }) => {
      const client = getHttpClient()
      const response = await client.getSession(input.sessionID)
      return adaptSessionInfo(response.session, {
        directory: Instance.directory,
      })
    },
    create: async (input?: { title?: string; agent?: string }) => {
      const client = getHttpClient()
      const response = await client.createSession({
        title: input?.title,
        agent: input?.agent,
      })
      return adaptSessionInfo(response.session, {
        directory: Instance.directory,
      })
    },
    fork: async (input: { sessionID: string; messageID?: string; title?: string }) => {
      const client = getHttpClient()
      const response = await client.forkSession(input.sessionID, {
        message_id: input.messageID,
        title: input.title,
      })
      return adaptSessionInfo(response.session, {
        directory: Instance.directory,
      })
    },
    remove: async (input: { sessionID: string }) => {
      const client = getHttpClient()
      await client.deleteSession(input.sessionID)
      return true
    },
    delete: async (input: { sessionID: string }) => {
      const client = getHttpClient()
      await client.deleteSession(input.sessionID)
      return true
    },
    compact: async (input: { sessionID: string }) => {
      const client = getHttpClient()
      await client.compactSession(input.sessionID)
      return true
    },
    revert: LocalSession.revert,
    status: LocalSession.status,
    summary: LocalSession.summary,
    todo: LocalSession.todo,
    // Use SDK-based prompt via WebSocket when feature flag is enabled
    prompt: USE_SDK_PROMPT ? promptViaSdk : LocalSession.prompt,
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
      const client = getHttpClient()
      const result = await client.listProviders()
      const providers = result.all.map((p: ProviderInfo) => ({
        id: p.id,
        name: p.name,
        models: p.models,
      }))
      return { providers, default: result.default }
    },
  },

  provider: {
    list: async () => {
      const client = getHttpClient()
      return client.listProviders()
    },
    auth: async () => {
      // TODO: Migrate auth methods to SDK when available
      return {}
    },
  },

  command: {
    list: Command.list,
  },

  app: {
    agents: async () => {
      const client = getHttpClient()
      const response = await client.listAgents()
      return response.agents.map((a: AgentInfo) => ({
        name: a.name,
        description: a.description,
        mode: a.mode,
        temperature: a.temperature,
        color: a.color,
        hidden: a.hidden,
      }))
    },
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
