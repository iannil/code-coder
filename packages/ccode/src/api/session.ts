import { Session } from "@/session"
import { SessionPrompt } from "@/session/prompt"
import { Command } from "@/command"
import type { Provider } from "@/provider/provider"
import { SessionStatus } from "@/session/status"
import { Todo } from "@/session/todo"
import { SessionCompaction } from "@/session/compaction"
import { SessionRevert } from "@/session/revert"
import { fn } from "@/util/fn"
import z from "zod"

const ListInput = z
  .object({
    directory: z.string().optional(),
    roots: z.boolean().optional(),
    start: z.number().optional(),
    search: z.string().optional(),
    limit: z.number().optional(),
  })
  .optional()

export namespace LocalSession {
  export const list = fn(
    ListInput,
    async (input) => {
      const sessions: Session.Info[] = []
      for await (const session of Session.list()) {
        if (input?.directory !== undefined && session.directory !== input.directory) continue
        if (input?.roots && session.parentID) continue
        if (input?.start !== undefined && session.time.updated < input.start) continue
        if (input?.search !== undefined && !session.title.toLowerCase().includes(input.search.toLowerCase()))
          continue
        sessions.push(session)
        if (input?.limit !== undefined && sessions.length >= input.limit) break
      }
      return sessions
    },
  )

  export const get = fn(Session.get.schema, Session.get)

  export const create = fn(
    z
      .object({
        parentID: Session.Info.shape.id.optional(),
        title: Session.Info.shape.title.optional(),
        permission: Session.Info.shape.permission.optional(),
      })
      .optional(),
    Session.create,
  )

  export const children = fn(Session.children.schema, Session.children)

  export const status = async () => SessionStatus.list()

  export const summary = async (sessionID: string) => {
    const session = await Session.get(sessionID)
    return session.summary
  }

  export const todo = async (sessionID: string) => Todo.get(sessionID)

  export const fork = fn(
    z.object({
      sessionID: Session.Info.shape.id,
      messageID: Session.Info.shape.id.optional(),
    }),
    Session.fork,
  )

  export const remove = fn(Session.remove.schema, Session.remove)

  export const compact = async (sessionID: string) => {
    await SessionCompaction.prune({ sessionID })
    return true
  }

  export const revert = async (sessionID: string, messageID: string) => {
    await SessionRevert.revert({ sessionID, messageID })
    return true
  }

  export const messages = fn(
    z.object({
      sessionID: Session.Info.shape.id,
      limit: z.number().optional(),
    }),
    Session.messages,
  )

  export type PromptInput = {
    sessionID: string
    agent?: string
    model?: string
    variant?: string
    parts: Array<{ type: string; text?: string; url?: string; filename?: string; mime?: string }>
  }

  export const prompt = async (input: PromptInput) => {
    const result = await SessionPrompt.prompt({
      sessionID: input.sessionID,
      agent: input.agent,
      model: input.model as any,
      variant: input.variant,
      parts: input.parts as any,
    })
    // result is either a message ID or a MessageV2.WithParts object
    const messageID = typeof result === "string" ? result : result.info.id
    return { messageID }
  }

  export type CommandInput = {
    sessionID: string
    agent?: string
    model?: string
    command: string
    arguments: string
    variant?: string
  }

  export const command = async (input: CommandInput) => {
    const exists = await Command.get(input.command)
    if (!exists) throw new Error(`Command "${input.command}" not found`)

    const messageID = await SessionPrompt.command({
      sessionID: input.sessionID,
      agent: input.agent,
      model: input.model as any,
      command: input.command,
      arguments: input.arguments,
      variant: input.variant,
    })
    return { messageID: messageID }
  }
}
