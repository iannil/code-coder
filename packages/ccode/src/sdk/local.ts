/**
 * Local SDK client that provides the same interface as the HTTP SDK client
 * but uses direct function calls instead of HTTP requests.
 */

import type { Event } from "@/types"
import { createGlobalEmitter } from "@solid-primitives/event-bus"
import { LocalSession, LocalPermission, LocalConfig, LocalEvent, LocalFind } from "@/api"
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
import { Instance } from "@/project/instance"
import { Bus } from "@/bus"

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
        get: async (input: { sessionID: string }) => ({ data: await Session.get(input.sessionID) }),
        create: async (input?: any) => ({ data: await LocalSession.create(input) }),
        fork: async (input: { sessionID: string; messageID?: string }) => ({
          data: await LocalSession.fork(input),
        }),
        remove: async (input: { sessionID: string }) => {
          await Session.remove(input.sessionID)
          return { data: true }
        },
        delete: async (input: { sessionID: string }) => {
          await Session.remove(input.sessionID)
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
        list: async () => ({ data: await Provider.listAll() }),
        auth: async () => ({ data: await Provider.authMethods() }),
      },

      command: {
        list: async () => ({ data: await Command.list() }),
      },

      app: {
        agents: async () => ({ data: await Agent.list() }),
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
