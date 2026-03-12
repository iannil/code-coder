/**
 * Local SDK client that provides the same interface as the HTTP SDK client
 * but uses direct function calls instead of HTTP requests.
 *
 * Uses Rust API via @/api for all session and command operations.
 */

import type { Event } from "@/types"
import { createGlobalEmitter } from "@solid-primitives/event-bus"
import { LocalSession, LocalPermission, LocalConfig, LocalEvent, LocalFind, Command } from "@/api"
import { LSP } from "@/lsp"
import { Format } from "@/util/format"
import { Skill } from "@/skill/skill"
import { MCP } from "@/mcp"
import { Vcs } from "@/project/vcs"
import { Global } from "@/util/global"
import { Instance } from "@/project/instance"

// ══════════════════════════════════════════════════════════════════════════════
// Local Provider/Agent Functions
// ══════════════════════════════════════════════════════════════════════════════

/**
 * List all agents (local implementation).
 * Uses Rust daemon via AgentBridge.
 */
async function listAgentsLocal() {
  try {
    const { getAgentBridge } = await import("@/sdk/agent-bridge")
    const bridge = await getAgentBridge()
    if (await bridge.isHealthy()) {
      const agents = await bridge.list()
      return agents.map((a) => ({
        name: a.name,
        description: a.description,
        mode: a.mode,
        hidden: a.hidden,
        temperature: a.temperature,
        color: a.color,
        permission: {},
        options: {},
      }))
    }
  } catch {
    // Daemon not available
  }
  throw new Error("Agent daemon not available. Start the daemon with: zero-cli serve")
}

/**
 * List all providers with connection info (local implementation).
 */
async function listProvidersLocal() {
  try {
    const { getRustClient } = await import("@/api/rust-client")
    const client = getRustClient()
    const response = await client.listProviders()
    if (response.success && response.data) {
      return {
        success: true,
        all: response.data.providers.map((p) => ({
          id: p.id,
          name: p.name,
          source: "rust" as const,
          env: {},
          options: {},
          models: p.models,
        })),
        default: response.data.providers.find((p) => p.is_default)?.id ?? {},
        connected: response.data.providers.filter((p) => p.models.length > 0).map((p) => p.id),
      }
    }
  } catch {
    // Fall back to empty
  }
  return {
    success: false,
    all: [],
    default: {},
    connected: [],
  }
}

/**
 * Get provider auth methods (stub - returns empty object).
 */
async function getProviderAuthMethodsLocal(): Promise<Record<string, { type: "oauth" | "api"; label: string }[]>> {
  return {}
}

export function createLocalClient() {
  const eventSource = LocalEvent.subscribe()
  const emitter = createGlobalEmitter<{
    [key in Event["type"]]: Extract<Event, { type: key }>
  }>()

  // Subscribe to local events and emit to the SolidJS event bus
  ;(async () => {
    const reader = eventSource.stream.getReader()
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      emitter.emit(value.type, value)
    }
  })()

  return {
    url: "http://codecoder.internal",

    event: {
      ...emitter,
      subscribe: async () => {
        const reader = eventSource.stream.getReader()
        return {
          stream: eventSource.stream,
          async *[Symbol.asyncIterator]() {
            while (true) {
              const { done, value } = await reader.read()
              if (done) break
              yield value
            }
          },
        }
      },
    },

    client: {
      session: {
        list: async (input?: any) => ({ data: await LocalSession.list(input) }),
        get: async (input: { sessionID: string }) => ({ data: await LocalSession.get(input.sessionID) }),
        create: async (input?: any) => ({ data: await LocalSession.create(input) }),
        fork: async (input: { sessionID: string; messageID?: string }) => ({
          data: await LocalSession.fork(input),
        }),
        remove: async (input: { sessionID: string }) => {
          await LocalSession.remove(input.sessionID)
          return { data: true }
        },
        delete: async (input: { sessionID: string }) => {
          await LocalSession.remove(input.sessionID)
          return { data: true }
        },
        compact: async (input: { sessionID: string }) => ({ data: await LocalSession.compact(input.sessionID) }),
        revert: async (input: { sessionID: string; messageID: string }) => ({
          data: await LocalSession.revert(input.sessionID, input.messageID),
        }),
        status: async () => ({ data: await LocalSession.status() }),
        summary: async (input: { sessionID: string }) => ({ data: await LocalSession.summary(input.sessionID) }),
        todo: async (input: { sessionID: string }) => ({ data: await LocalSession.todo(input.sessionID) }),
        prompt: async (input: any) => ({ data: await LocalSession.prompt(input) }),
        command: async (input: any) => ({ data: await LocalSession.command(input) }),
        children: async (input: { sessionID: string }) => ({ data: await LocalSession.children(input.sessionID) }),
      },

      permission: {
        list: async () => ({ data: LocalPermission.list() }),
        reply: async (input: any) => {
          await LocalPermission.reply(input)
          return { data: true }
        },
      },

      config: {
        get: async () => ({ data: await LocalConfig.get() }),
        update: async (input: any) => ({ data: await LocalConfig.update(input) }),
      },

      provider: {
        list: async () => ({ data: await listProvidersLocal() }),
        auth: async () => ({ data: await getProviderAuthMethodsLocal() }),
      },

      command: {
        list: async () => ({ data: await Command.list() }),
      },

      app: {
        agents: async () => ({ data: await listAgentsLocal() }),
        skills: async () => ({ data: await Skill.all() }),
      },

      lsp: {
        status: async () => ({ data: await LSP.status() }),
      },

      formatter: {
        status: async () => ({ data: await Format.status() }),
      },

      mcp: {
        status: async () => ({ data: await MCP.status() }),
        resources: {
          list: async () => ({ data: await MCP.resources() }),
        },
      },

      experimental: {
        resources: {
          list: async () => ({ data: await MCP.resources() }),
        },
      },

      find: {
        files: async (input: any) => ({ data: await LocalFind.files(input) }),
      },

      vcs: {
        get: async () => ({ data: { branch: await Vcs.branch() } }),
      },

      path: {
        get: async () => ({
          data: {
            home: Global.Path.home,
            state: Global.Path.state,
            config: Global.Path.config,
            worktree: Instance.worktree,
            directory: Instance.directory,
          },
        }),
      },

      question: {
        reply: async (_input: { requestID: string; answers: string[][] }) => {
          // Question reply is now handled via Rust API
          console.warn("question.reply is deprecated")
          return { data: true }
        },
        reject: async (_input: { requestID: string }) => {
          // Question reject is now handled via Rust API
          console.warn("question.reject is deprecated")
          return { data: true }
        },
      },
    },
  }
}

export type LocalClient = ReturnType<typeof createLocalClient>
