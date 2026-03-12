/**
 * Local SDK client that provides the same interface as the HTTP SDK client
 * but uses direct function calls instead of HTTP requests.
 *
 * NOTE: This module avoids importing from deprecated modules (@/agent/agent,
 * @/provider/provider, @/session). Instead, it uses:
 * - LocalSession from @/api for session operations
 * - Direct imports from non-deprecated modules where available
 * - Lazy dynamic imports where necessary to avoid circular dependencies
 */

import type { Event } from "@/types"
import type { ProviderListResponseExtended } from "@/sdk/types"
import { createGlobalEmitter } from "@solid-primitives/event-bus"
import { LocalSession, LocalPermission, LocalConfig, LocalEvent, LocalFind } from "@/api"
import { Command } from "@/agent/command"
import { LSP } from "@/lsp"
import { Format } from "@/util/format"
import { Skill } from "@/skill/skill"
import { MCP } from "@/mcp"
import { Vcs } from "@/project/vcs"
import { Question } from "@/agent/question"
import { Global } from "@/util/global"
import { Instance } from "@/project/instance"
import { Bus } from "@/bus"

// ══════════════════════════════════════════════════════════════════════════════
// Local Provider/Agent Functions
// These use dynamic imports to avoid importing deprecated modules at module load time
// ══════════════════════════════════════════════════════════════════════════════

/**
 * List all agents (local implementation).
 *
 * Migration strategy:
 * 1. First try the Rust daemon via AgentBridge (preferred)
 * 2. Fall back to deprecated @/agent/agent if daemon unavailable
 *
 * This allows gradual migration without breaking existing functionality.
 */
async function listAgentsLocal() {
  // Try Rust daemon first
  try {
    const { getAgentBridge } = await import("@/sdk/agent-bridge")
    const bridge = await getAgentBridge()
    if (await bridge.isHealthy()) {
      const agents = await bridge.list()
      // Convert to legacy format for compatibility
      return agents.map((a) => ({
        name: a.name,
        description: a.description,
        mode: a.mode,
        hidden: a.hidden,
        temperature: a.temperature,
        color: a.color,
        // Note: permission, options, etc. not available from bridge yet
        permission: {},
        options: {},
      }))
    }
  } catch {
    // Daemon not available, throw error instead of falling back to deprecated code
    throw new Error("Agent daemon not available. Start the daemon with: ./ops.sh start")
  }
}

/**
 * List all providers with connection info (local implementation).
 * Uses dynamic import to avoid top-level import of deprecated @/provider/provider.
 */
async function listProvidersLocal(): Promise<ProviderListResponseExtended> {
  const { Provider } = await import("@/provider/provider")
  const result = await Provider.listAll()
  return {
    success: true,
    all: result.all,
    default: result.default,
    connected: result.connected,
  }
}

/**
 * Get provider auth methods (local implementation).
 * Uses dynamic import to avoid top-level import of deprecated @/provider/provider.
 */
async function getProviderAuthMethodsLocal(): Promise<Record<string, { type: "oauth" | "api"; label: string }[]>> {
  const { Provider } = await import("@/provider/provider")
  return Provider.authMethods()
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
        reply: async (input: any) => {
          await Question.reply({ requestID: input.requestID, answers: input.answers })
          return { data: true }
        },
        reject: async (input: any) => {
          await Question.reject(input.requestID)
          return { data: true }
        },
      },
    },
  }
}

export type LocalClient = ReturnType<typeof createLocalClient>
